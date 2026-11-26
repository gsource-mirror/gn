// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::{collections::BTreeSet, pin::Pin, sync::RwLock};

use loader::FileLoader;
use providers::BuiltinProviders;
use starlark::environment::{FrozenModule, Globals};
use types::{
    Label, LabelRef, PackageRef, PathResolver, Session as TypesSession, util::extend_lifetime,
};

use crate::{errors::Error, target::Target};

/// Represents a Starlark evaluation session exposed to C++ via FFI.
pub struct Session {
    loader: FileLoader,
    pub(crate) path_resolver: PathResolver,
    globals: Globals,
    mnemonics: RwLock<BTreeSet<String>>,
    builtin_rules: Vec<&'static rule::Rule<'static, crate::eval_context::EvalContext>>,
    builtin_providers: BuiltinProviders,
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
        let mut builder = starlark::environment::GlobalsBuilder::extended_by(&[
            starlark::environment::LibraryExtension::StructType,
        ]);
        builder.set("attr", attr::AttrModule { make_attr_schema });
        providers::globals::register_providers(&mut builder);
        depset::depset_globals!(&mut builder, crate::eval_context::EvalContext);
        rule::register_rule_globals!(&mut builder, crate::eval_context::EvalContext);

        let mut loader: FileLoader = Default::default();
        // Safety: The module lives as long as the loader lasts, which is as long as the session
        // lasts, which is 'static.
        let rules_module: &'static FrozenModule = unsafe {
            extend_lifetime(loader.preload(
                Label::parse("//builtins:rules.scl", PackageRef::root()).unwrap(),
                rule::register_builtin_rules::<crate::eval_context::EvalContext>(),
            ))
        };
        loader.preload(
            Label::parse("//builtins:providers.scl", PackageRef::root()).unwrap(),
            providers::builtin_providers_module(),
        );

        assert!(
            std::mem::size_of::<Target>() <= crate::bridge::rust_target_storage_size(),
            "Item::rust_target_storage_ ({} bytes) cannot hold ffi::Target ({} bytes)",
            crate::bridge::rust_target_storage_size(),
            std::mem::size_of::<Target>(),
        );

        let globals = builder.build();

        Self {
            path_resolver,
            mnemonics: Default::default(),
            builtin_rules: rule::extract_builtin_rules(rules_module),
            builtin_providers: providers::BuiltinProviders::new(&globals),
            loader,
            globals,
        }
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

    pub(crate) fn builtin_rule(
        &self,
        output_type: u8,
    ) -> &'static rule::Rule<'static, crate::eval_context::EvalContext> {
        self.builtin_rules[output_type as usize]
    }

    pub(crate) fn record_mnemonics<'v>(&self, actions: &[rule::action::Action<'v>]) {
        if actions.is_empty() {
            return;
        }
        {
            let mnemonics = self.mnemonics.read().unwrap();
            if actions
                .iter()
                .all(|a| mnemonics.contains(a.mnemonic.map(|m| m.as_str()).unwrap_or("ACTION")))
            {
                return;
            }
        }
        let mut mnemonics = self.mnemonics.write().unwrap();
        for action in actions {
            let mnemonic = action.mnemonic.map(|m| m.as_str()).unwrap_or("ACTION");
            if !mnemonics.contains(mnemonic) {
                mnemonics.insert(mnemonic.to_owned());
            }
        }
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

    /// Writes global Ninja rules for all unique action mnemonics across evaluated targets.
    pub fn write_all_rules(&self, buffer: Pin<&mut crate::bridge::StringOutputBuffer>) {
        let mnemonics = self.mnemonics.read().unwrap();

        let mut buf = crate::StringOutputBuffer::new(buffer);
        for mnemonic in mnemonics.iter() {
            buf.append((
                "rule action_",
                mnemonic.as_str(),
                "\n  command = $command\n  restat = 1\n  description = ",
                mnemonic.as_str(),
                " $out\n\n",
            ));
        }
    }
}

impl TypesSession for Session {
    type TargetRef = crate::TargetRef;

    fn run_builtin_rule<'v>(
        &self,
        heap: &starlark::values::Heap<'v>,
        target: &Self::TargetRef,
        state: &mut types::CtxState<Self::TargetRef>,
    ) -> starlark::values::Value<'v> {
        use types::TargetRef as _;
        let outputs = target.outputs();
        let default_info = self.builtin_providers.new_defaultinfo(
            heap.alloc(depset::Depset::new_file_depset(outputs, heap, state)),
            None,
            heap,
        );
        heap.alloc(vec![default_info])
    }
}
