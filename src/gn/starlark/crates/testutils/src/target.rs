// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    ops::Deref,
    rc::Rc,
};

use allocative::Allocative;
use attr::Attr;
use starlark::{
    starlark_simple_value,
    values::{FrozenValue, ProvidesStaticType, StarlarkValue, Value, ValueLike},
};
use starlark_derive::{starlark_value, NoSerialize};
use types::{
    File, IPromiseToImplementStarlarkEqAndHash, Label, LabelRef, OutputType, Session, TargetRef,
};

/// A fake target struct for testing.
#[derive(Allocative, Debug)]
pub struct FakeTarget {
    pub label: Label,
    pub toolchain: Label,
    /// A list of fake files returned as outputs of the target.
    pub outputs: Vec<File>,
    /// A list of attributes.
    pub attrs: Vec<Attr>,
    pub output_type: Option<OutputType>,
    pub rule: FrozenValue,
    #[allocative(skip)]
    pub cxx_attrs: HashMap<String, Value<'static>>,
    /// Registered target dependencies.
    #[allocative(skip)]
    pub dependencies: RefCell<HashSet<(Label, Label)>>,
}

impl PartialEq for FakeTarget {
    fn eq(&self, other: &Self) -> bool {
        self.label == other.label
            && self.toolchain == other.toolchain
            && self.outputs == other.outputs
            && self.attrs == other.attrs
            && self.output_type == other.output_type
            && self.rule == other.rule
            && self.cxx_attrs.len() == other.cxx_attrs.len()
            && self.cxx_attrs.iter().all(|(k, v)| {
                other
                    .cxx_attrs
                    .get(k)
                    .is_some_and(|ov| v.equals(*ov).unwrap_or(false))
            })
            && *self.dependencies.borrow() == *other.dependencies.borrow()
    }
}

impl Eq for FakeTarget {}

/// A reference to a fake target.
#[derive(Debug, ProvidesStaticType, NoSerialize, Allocative, Clone)]
pub struct FakeTargetRef(#[allocative(skip)] Rc<FakeTarget>);

// Safety: FakeTargetRef is only used in single-threaded contexts inside unit
// tests.
unsafe impl Send for FakeTargetRef {}
// Safety: FakeTargetRef is only used in single-threaded contexts inside unit
// tests.
unsafe impl Sync for FakeTargetRef {}

impl FakeTargetRef {
    /// Creates a new `FakeTargetRef` containing the given `FakeTarget`.
    pub fn new(target: FakeTarget) -> Self {
        Self(Rc::new(target))
    }

    /// Returns a shared reference to the underlying target.
    pub fn get(&self) -> &FakeTarget {
        &self.0
    }

    /// Returns the registered dependencies of this target.
    pub fn registered_deps(&self) -> HashSet<(Label, Label)> {
        self.dependencies.borrow().clone()
    }
}

impl PartialEq for FakeTargetRef {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for FakeTargetRef {}

impl std::hash::Hash for FakeTargetRef {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        Rc::as_ptr(&self.0).hash(state);
    }
}

starlark_simple_value!(FakeTargetRef);

impl std::fmt::Display for FakeTargetRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FakeTargetRef")
    }
}

impl IPromiseToImplementStarlarkEqAndHash for FakeTargetRef {}

#[starlark_value(type = "Target")]
impl<'v> StarlarkValue<'v> for FakeTargetRef {
    fn equals(&self, other: Value<'v>) -> starlark::Result<bool> {
        if let Some(other) = other.downcast_ref::<Self>() {
            Ok(self == other)
        } else {
            Ok(false)
        }
    }

    fn write_hash(
        &self,
        hasher: &mut starlark::collections::StarlarkHasher,
    ) -> starlark::Result<()> {
        use std::hash::Hash as _;
        self.hash(hasher);
        Ok(())
    }
}

impl Deref for FakeTargetRef {
    type Target = FakeTarget;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl TargetRef for FakeTargetRef {
    fn label(&self) -> LabelRef<'_> {
        self.get().label.as_ref()
    }

    fn toolchain(&self) -> LabelRef<'_> {
        self.get().toolchain.as_ref()
    }

    fn outputs(&self) -> Vec<File> {
        self.get().outputs.clone()
    }

    fn target_out_dir(&self, prefix: &str, suffix: &str, _separator: &str) -> String {
        format!("{prefix}$TOOLCHAIN/{suffix}$LABEL")
    }

    fn register_dependencies<S: Session<TargetRef = Self>>(
        &self,
        session: &S,
        toolchain: LabelRef<'_>,
    ) {
        for attr in &self.get().attrs {
            attr.register_dependencies(session, self.clone(), toolchain);
        }
    }
}
