// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;

use allocative::Allocative;
use starlark::environment::Methods;
use starlark::environment::MethodsBuilder;
use starlark::environment::MethodsStatic;
use starlark::starlark_complex_value;
use starlark::typing::Ty;
use starlark::values::type_repr::StarlarkTypeRepr;
use starlark::values::UnpackValue;
use starlark::values::{
    Freeze, FreezeResult, Freezer, Heap, ProvidesStaticType, StarlarkValue, Trace, Value, ValueLike,
};
use starlark_derive::Coerce;
use starlark_derive::{starlark_module, starlark_value, NoSerialize};
use types::File;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Allocative)]
/// All orderings are guarunteed to be deterministic.
pub enum Order {
    /// Our unspecified order is postorder. However, this should not be relied upon.
    Unspecified,
    /// Guarunteed to traverse through direct dependencies in left-to-right order, then transitive in left-to-right order.
    Preorder,
    /// Guarunteed to traverse through transitive dependencies in left-to-right order, then direct in left-to-right order.
    Postorder,
    // Our topological order is reverse postorder. However, this should not be relied upon.
    // Note: Topological order is much slower and less memory efficient
    // as it requires an intermediate Vec to be created and then reversed.
    Topological,
}

impl StarlarkTypeRepr for Order {
    type Canonical = String;

    fn starlark_type_repr() -> Ty {
        String::starlark_type_repr()
    }
}

impl<'v> UnpackValue<'v> for Order {
    type Error = starlark::Error;

    fn unpack_value_impl(value: Value<'v>) -> Result<Option<Self>, Self::Error> {
        match <&'v str>::unpack_value_err(value)? {
            "default" => Ok(Some(Self::Unspecified)),
            "preorder" => Ok(Some(Self::Preorder)),
            "postorder" => Ok(Some(Self::Postorder)),
            "topological" => Ok(Some(Self::Topological)),
            s => Err(crate::Error::InvalidOrder(s.to_owned()).into()),
        }
    }
}

impl Display for Order {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unspecified => write!(f, "unspecified"),
            Self::Preorder => write!(f, "preorder"),
            Self::Postorder => write!(f, "postorder"),
            Self::Topological => write!(f, "topological"),
        }
    }
}

/// The type of elements contained in a depset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, allocative::Allocative)]
pub enum Kind {
    Empty,
    Unknown,
    File,
}

impl Display for Kind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "empty"),
            Self::Unknown => write!(f, "unknown"),
            Self::File => write!(f, "File"),
        }
    }
}

// By implementing coerce, freezing depsets is zero-cost.
// Starlark knows that it can just do a reinterpret cast of the memory.
/// A generic implementation of a Starlark Depset.
#[derive(Debug, Trace, Coerce, ProvidesStaticType, NoSerialize, Allocative)]
#[repr(C)]
pub struct DepsetGen<V> {
    pub(crate) order: Order,
    // De-duped on creation.
    pub(crate) direct: Vec<V>,
    // Transitive depsets. Each entry is guarunteed to be a non-empty depset.
    pub(crate) transitive: Vec<V>,
    // The inner type of the depset.
    pub(crate) kind: Kind,
    // Set For depset[File] only.
    // If the depset only has a single element, it will just be that element.
    pub(crate) phony: Option<File>,
}

impl<'v> Depset<'v> {
    /// Creates a new `Depset` containing `File` elements.
    pub fn new_file_depset(direct: Vec<File>, heap: &Heap<'v>) -> Self {
        let direct_vals: Vec<Value<'v>> = direct.into_iter().map(|f| heap.alloc(f)).collect();
        let phony = if direct_vals.len() == 1 {
            direct_vals[0].downcast_ref::<File>().cloned()
        } else {
            None
        };
        Self {
            order: Order::Unspecified,
            direct: direct_vals,
            transitive: Vec::new(),
            kind: Kind::File,
            phony,
        }
    }
}

impl<V> Default for DepsetGen<V> {
    fn default() -> Self {
        Self {
            order: Order::Unspecified,
            direct: Vec::new(),
            transitive: Vec::new(),
            kind: Kind::Empty,
            phony: None,
        }
    }
}

impl<V> DepsetGen<V> {
    pub(crate) fn order(&self) -> Order {
        self.order
    }

    pub(crate) fn direct(&self) -> &[V] {
        &self.direct
    }

    pub(crate) fn transitive(&self) -> &[V] {
        &self.transitive
    }

    /// Returns the element kind of this depset.
    pub fn kind(&self) -> &Kind {
        &self.kind
    }

    /// Returns the single phony File if this is a file depset containing exactly one element.
    pub fn phony(&self) -> &Option<File> {
        &self.phony
    }

    /// Returns true if this depset has no elements (its kind is Empty).
    pub fn is_empty(&self) -> bool {
        self.kind == Kind::Empty
    }
}

starlark_complex_value!(pub Depset);

impl<'v, V: ValueLike<'v>> Display for DepsetGen<V>
where
    Self: ProvidesStaticType<'v>,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // Explicitly DO NOT flatten a depset implicitly, they can get massive.
        // We may consider printing some fields in the future, but for now
        // we'll leave this empty to be safe.
        if self.is_empty() {
            write!(f, "depset(...)")
        } else {
            write!(f, "depset([])")
        }
    }
}

#[starlark_value(type = "depset")]
impl<'v, V: ValueLike<'v>> StarlarkValue<'v> for DepsetGen<V>
where
    Self: ProvidesStaticType<'v>,
{
    fn get_methods() -> Option<&'static Methods> {
        static RES: MethodsStatic = MethodsStatic::new("depset", depset_methods);
        Some(RES.methods())
    }

    fn to_bool(&self) -> bool {
        !self.is_empty()
    }
}

impl Freeze for Depset<'_> {
    type Frozen = FrozenDepset;
    fn freeze(self, freezer: &Freezer) -> FreezeResult<Self::Frozen> {
        Ok(DepsetGen {
            order: self.order,
            direct: self.direct.freeze(freezer)?,
            transitive: self.transitive.freeze(freezer)?,
            kind: self.kind,
            phony: self.phony,
        })
    }
}

/// Declares the Starlark methods for the `depset` type.
#[starlark_module]
pub fn depset_methods(builder: &mut MethodsBuilder) {
    fn to_list<'v>(this: &Depset<'v>) -> starlark::Result<Vec<Value<'v>>> {
        Ok(this.iter().collect())
    }
}
