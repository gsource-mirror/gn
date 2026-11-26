// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.
use std::{
    cell::{RefCell, UnsafeCell},
    collections::{HashMap, HashSet},
    rc::Rc,
};

use attr::{Attr, EvalContext as AttrEvalContext, EvalContextAttrExt};
use starlark::{
    Result,
    values::{FrozenValueTyped, Heap, ProvidesStaticType, Value},
};
use types::{
    CtxState, Label, LabelRef, OutputType, Package, PackageRef, PathResolver, Scope, Session,
    TargetRef as _,
};

use crate::{FakeSession, FakeTarget, FakeTargetRef};

#[derive(Clone, Default, Debug)]
pub struct FakeScope(HashMap<String, Value<'static>>);

impl Scope for FakeScope {
    type Owned = Box<Self>;

    fn copy_with<'a, 'v>(
        self: std::pin::Pin<&mut Self>,
        kv: impl Iterator<Item = (&'a str, Value<'v>)>,
    ) -> Result<Self::Owned> {
        let mut values = self.0.clone();
        for (k, v) in kv {
            // Safety: Transmuting 'v to 'static is safe because this mock scope
            // is only used in tests, where the Starlark heap outlives the evaluation
            // context.
            let static_val = unsafe { std::mem::transmute::<Value<'v>, Value<'static>>(v) };
            values.insert(k.to_owned(), static_val);
        }
        Ok(Box::new(Self(values)))
    }

    fn as_pin_mut(owned: &mut Self::Owned) -> std::pin::Pin<&mut Self> {
        std::pin::Pin::new(owned.as_mut())
    }

    fn get<'v>(&self, key: &str, _heap: &Heap<'v>) -> Option<Value<'v>> {
        self.0.get(key).map(|v| {
            // Safety: Shortening the lifetime is always safe.
            unsafe { std::mem::transmute::<Value<'static>, Value<'v>>(*v) }
        })
    }
}

/// A simple implementation of the evaluation context used in Starlark unit
/// tests.
#[derive(allocative::Allocative)]
pub struct FakeEvalContext {
    /// The current package being processed.
    pub package: Package,
    /// The current toolchain.
    pub current_toolchain: Label,
    /// The fake starlark session.
    #[allocative(skip)]
    pub session: Rc<FakeSession>,
    /// The fake path resolver.
    #[allocative(skip)]
    pub path_resolver: PathResolver,
    /// The fake rule state.
    #[allocative(skip)]
    pub rule_state: RefCell<CtxState<FakeTargetRef>>,
    /// The fake scope.
    #[allocative(skip)]
    pub scope: UnsafeCell<FakeScope>,
    /// Targets defined in this evaluation context.
    #[allocative(skip)]
    pub targets: RefCell<HashSet<FakeTargetRef>>,
}

// Safety: FakeEvalContext is 'static and has no lifetime parameters.
unsafe impl<'v> ProvidesStaticType<'v> for FakeEvalContext {
    type StaticType = Self;
}

impl FakeEvalContext {
    /// Creates a new eval context for a given session.
    pub fn default_rule_impl(session: Rc<FakeSession>) -> Self {
        Self::with_target(session, PackageRef::root(), "default")
    }

    /// Creates a new `FakeEvalContext` for a given package name.
    pub fn new(package: &PackageRef, name: &str) -> Self {
        Self::with_target(Rc::new(FakeSession::default()), package, name)
    }

    fn with_target(session: Rc<FakeSession>, package: &PackageRef, name: &str) -> Self {
        let target = FakeTargetRef::new(FakeTarget::empty(&session, package, name));
        let ctx = Self::rule_impl(session, target.clone());
        ctx.targets.borrow_mut().insert(target);
        ctx
    }

    pub fn rule_impl(session: Rc<FakeSession>, target: FakeTargetRef) -> Self {
        Self {
            package: target.label().package().to_owned(),
            current_toolchain: target.toolchain().to_owned(),
            session,
            path_resolver: PathResolver::new_for_testing(),
            rule_state: RefCell::new(CtxState::new(target)),
            scope: FakeScope::default().into(),
            targets: RefCell::new(HashSet::new()),
        }
    }

    /// Helper to insert a target into this evaluation context.
    pub fn insert_target(&self, target: FakeTarget) -> FakeTargetRef {
        let target_ref = FakeTargetRef::new(target);
        let mut targets = self.targets.borrow_mut();
        assert!(
            targets.insert(target_ref.clone()),
            "Inserting an already existing target into the map"
        );
        target_ref
    }

    /// Helper to create an empty target with the default toolchain.
    pub fn empty_target(&self, package: &PackageRef, name: &str) -> FakeTarget {
        FakeTarget::empty(&self.session, package, name)
    }

    /// Helper to insert an empty target into this evaluation context.
    pub fn insert_empty_target(&self, package: &PackageRef, name: &str) -> FakeTargetRef {
        self.insert_target(self.empty_target(package, name))
    }

    /// Look up a target by its label and toolchain, returning `None` if absent.
    pub fn try_get_target(
        &self,
        label: LabelRef<'_>,
        current_toolchain: LabelRef<'_>,
    ) -> Option<FakeTargetRef> {
        self.targets
            .borrow()
            .iter()
            .find(|target| target.label() == label && target.toolchain() == current_toolchain)
            .cloned()
    }

    /// Look up a target by its label and current toolchain.
    pub fn get_target(
        &self,
        label: LabelRef<'_>,
        current_toolchain: LabelRef<'_>,
    ) -> FakeTargetRef {
        self.try_get_target(label, current_toolchain)
            .unwrap_or_else(|| {
                panic!(
                    "get_target failed to find label: {label:?}, toolchain: {current_toolchain:?}. Available targets: {:?}",
                    self.targets.borrow()
                )
            })
    }
}

impl AttrEvalContext for FakeEvalContext {
    type Scope = FakeScope;
    type Session = FakeSession;

    fn session(&self) -> &Self::Session {
        &self.session
    }

    fn current_package(&self) -> &PackageRef {
        &self.package
    }

    fn path_resolver(&self) -> &PathResolver {
        &self.path_resolver
    }

    fn current_toolchain(&self) -> LabelRef<'_> {
        self.current_toolchain.as_ref()
    }

    #[allow(clippy::mut_from_ref)]
    fn require_macro(&self) -> Result<std::pin::Pin<&mut FakeScope>> {
        // Safety: The Scope pointer is non-null and valid during macro evaluation.
        Ok(std::pin::Pin::new(unsafe { &mut (*self.scope.get()) }))
    }

    fn require_bzl(&self) -> Result<()> {
        Ok(())
    }

    fn require_rule_impl(
        &self,
    ) -> Result<&RefCell<CtxState<<Self::Session as Session>::TargetRef>>> {
        Ok(&self.rule_state)
    }
}

impl EvalContextAttrExt for FakeEvalContext {
    fn create_target(
        &self,
        target_type: Option<OutputType>,
        target_name: &str,
        scope: std::pin::Pin<&mut FakeScope>,
        rule: Value<'static>,
        attrs: Vec<Attr>,
    ) -> Result<()> {
        let typed = FrozenValueTyped::<'static, rule::Rule<'static, Self>>::new_err(rule)?;
        let label = Label::new(self.package.clone(), target_name.to_owned());
        let toolchain = self.current_toolchain().to_owned();
        let mut dependencies = HashSet::new();
        let mut resolved_deps = Vec::new();
        for attr in &attrs {
            for (dep_label, dep_toolchain) in attr.dependencies(toolchain.as_ref()) {
                dependencies.insert((dep_label.to_owned(), dep_toolchain.to_owned()));
                if let Some(dep_target) = self.try_get_target(dep_label, dep_toolchain) {
                    resolved_deps.push(dep_target);
                }
            }
        }
        self.insert_target(FakeTarget {
            label,
            dependencies,
            resolved_deps,
            toolchain,
            output_type: target_type,
            rule: typed.as_ref(),
            cxx_attrs: scope.0.clone(),
            attrs,
            ..self.empty_target(&self.package, target_name)
        });
        Ok(())
    }
}

rule::impl_ctx_methods!(FakeEvalContext);
