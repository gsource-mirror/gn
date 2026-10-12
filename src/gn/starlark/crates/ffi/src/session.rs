// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::{
    collections::{hash_map::Entry, HashMap},
    pin::Pin,
    sync::RwLock,
};

use loader::FileLoader;
use providers::BuiltinProviders;
use starlark::environment::{FrozenModule, Globals};
use types::{
    util::extend_lifetime, Label, LabelRef, PackageRef, PathResolver, Session as TypesSession,
};

use crate::{errors::Error, target::Target};

type TargetMap = HashMap<(Label, Label), Pin<Box<Target>>>;

/// Represents a Starlark evaluation session exposed to C++ via FFI.
pub struct Session {
    loader: FileLoader,
    pub(crate) path_resolver: PathResolver,
    globals: Globals,
    targets: RwLock<TargetMap>,
    builtin_rules: Vec<&'static rule::Rule<'static, crate::eval_context::EvalContext>>,
    // Never None. Only an Option because this contains references to other fields in session, so
    // we first need to construct a session object and mutate it before returning.
    builtin_providers: Option<BuiltinProviders>,
}

fn make_attr_schema<'v>(
    kind: attr::AttrKind,
    args: attr::AttrSpecArgs<'v>,
    eval: &mut starlark::eval::Evaluator<'v, '_, '_>,
) -> starlark::Result<starlark::values::Value<'v>> {
    use types::{EvalContext as _, EvaluatorContextExt as _};
    let ctx = eval.context::<crate::eval_context::EvalContext>();
    attr::AttrSchema::create(
        kind,
        args,
        ctx.current_package(),
        ctx.path_resolver(),
        &eval.heap(),
    )
}

impl Session {
    /// Creates a new `Session`.
    pub fn from_resolver(path_resolver: PathResolver) -> Self {
        let mut builder = starlark::environment::GlobalsBuilder::standard();
        builder.set("attr", attr::AttrModule { make_attr_schema });
        providers::globals::register_providers(&mut builder);
        depset::depset_globals!(&mut builder, crate::eval_context::EvalContext);
        rule::register_rule_globals!(&mut builder, crate::eval_context::EvalContext);
        let mut s = Self {
            loader: Default::default(),
            path_resolver,
            globals: builder.build(),
            targets: Default::default(),
            builtin_rules: Vec::new(),
            builtin_providers: None,
        };
        s.as_static_mut().builtin_rules =
            rule::extract_builtin_rules(s.as_static_mut().loader.preload(
                Label::parse("//builtins:rules.scl", PackageRef::root()).unwrap(),
                rule::register_builtin_rules::<crate::eval_context::EvalContext>(),
            ));
        s.as_static_mut().loader.preload(
            Label::parse("//builtins:providers.scl", PackageRef::root()).unwrap(),
            providers::builtin_providers_module(),
        );
        s.as_static_mut().builtin_providers =
            Some(providers::BuiltinProviders::new(&s.as_static_mut().globals));
        s
    }

    fn builtin_providers(&self) -> &BuiltinProviders {
        self.builtin_providers.as_ref().unwrap()
    }

    /// Associated function for C++ constructor.
    pub fn new(source_root: &str, build_root: &str, source_root_rel: &str) -> Box<Self> {
        Box::new(Self::from_resolver(PathResolver::new(
            std::path::PathBuf::from(source_root),
            std::path::PathBuf::from(build_root),
            source_root_rel.to_owned(),
        )))
    }

    /// Associated function for C++ constructor.
    pub fn new_for_testing() -> Box<Self> {
        Box::new(Self::from_resolver(PathResolver::new_for_testing()))
    }

    fn as_static_mut(&mut self) -> &'static mut Self {
        // Safety: The session's lifetime is tied to the build session and thus lives forever.
        unsafe { std::mem::transmute(self) }
    }

    pub fn loaded(&self) -> Vec<&'static str> {
        self.loader
            .loaded()
            .into_iter()
            .map(|label| {
                self.path_resolver
                    .source_file(label.package(), label.name())
                    .as_str()
            })
            .collect()
    }

    pub(crate) fn register_target(&self, target: Target) -> crate::TargetRef {
        let label = target.label().as_ref().to_owned();
        let toolchain = target.toolchain().to_owned();

        let mut targets = self.targets.write().unwrap();
        crate::TargetRef(match targets.entry((label, toolchain)) {
            Entry::Vacant(e) => {
                let pinned = Box::pin(target);
                // Safety: A target object in the map is never deleted and lives for the
                // session.
                let static_ref: &'static Target = unsafe { extend_lifetime(&*pinned) };
                e.insert(pinned);
                static_ref.cxx.set_rust_target(static_ref);
                static_ref
            },
            // This should *almost* never occur. The one exception is when two threads try to call
            // rust_target from a C++ target at the same time.
            Entry::Occupied(e) => {
                assert!(
                    target.rule.is_builtin(),
                    "Target already registered, or accessed via to_rust before registration"
                );
                // Safety: A target object in the map is never deleted and lives for the
                // session.
                unsafe { extend_lifetime(&**e.get()) }
            },
        })
    }

    pub(crate) fn register_cxx_target(
        &self,
        cxx: &'static crate::bridge::CxxTarget,
    ) -> &'static Target {
        self.register_target(Target {
            cxx,
            rule: self.builtin_rules[cxx.output_type() as usize],
            attrs: Vec::new(),
            evaluated: Default::default(),
        })
        .0
    }

    pub(crate) fn load(&'static self, label: LabelRef<'_>) -> starlark::Result<FrozenModule> {
        self.loader
            .load(label, &self.path_resolver, &self.globals, &|pkg| {
                // Safety: The package reference is guaranteed to live as long as the
                // eval context.
                crate::eval_context::EvalContext::new_bzl_file(self, unsafe {
                    extend_lifetime(pkg)
                })
            })
    }

    /// Loads a Starlark module and populates a scope with values by key.
    pub fn load_values(
        &'static self,
        label: &str,
        relative_to: &str,
        keys: &[&str],
        mut scope: Pin<&mut crate::bridge::Scope>,
        settings: &crate::Settings,
        origin: crate::bridge::ParseNodePtr,
        mut err: Pin<&mut crate::bridge::Err>,
    ) {
        err.as_mut().handle((|| -> starlark::Result<()> {
            let label = Label::parse(label, PackageRef::new(relative_to)?)?;
            let module = self.load(label.as_ref())?;

            for key in keys {
                let value = module
                    .get(key)
                    .map_err(|_| Error::KeyNotFound(key.to_string(), label.clone()))?;
                let mut cxx_value = crate::bridge::SetValue(scope.as_mut(), key, origin);
                let owner = value.owner().to_owned();
                cxx_value.as_mut().assign(
                    value.as_ref().value(),
                    Some(&owner),
                    settings,
                    origin,
                )?;
            }
            Ok(())
        })());
    }
}

impl TypesSession for Session {
    type TargetRef = crate::TargetRef;

    fn get_target(&self, label: LabelRef<'_>, toolchain: LabelRef<'_>) -> Self::TargetRef {
        let targets = self.targets.read().unwrap();
        let target_pin = targets
            .get(&(label.to_owned(), toolchain.to_owned()))
            .expect("Requested target was not registered in the session");
        // Safety: A target object can never be deleted.
        let static_ref: &'static Target = unsafe { extend_lifetime(&**target_pin) };
        crate::TargetRef(static_ref)
    }

    fn run_builtin_rule<'v>(
        &self,
        heap: &starlark::values::Heap<'v>,
        target: &Self::TargetRef,
        state: &mut types::CtxState<Self::TargetRef>,
    ) -> starlark::values::Value<'v> {
        let outputs = target
            .0
            .cxx
            .computed_outputs()
            .iter()
            .map(|f| f.to_rust())
            .collect();
        let default_info = self.builtin_providers().new_defaultinfo(
            heap.alloc(depset::Depset::new_file_depset(outputs, heap, state)),
            None,
            heap,
        );
        heap.alloc(vec![default_info])
    }
}
