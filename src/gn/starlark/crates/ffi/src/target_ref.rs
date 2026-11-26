// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use starlark::values::{AllocValue, Heap, ProvidesStaticType, StarlarkValue, Value};
use starlark_derive::{NoSerialize, starlark_value};
use types::{LabelRef, TargetRef as _};

use crate::target::Target;

#[derive(Clone, Copy, ProvidesStaticType, NoSerialize)]
pub struct TargetRef(&'static crate::bridge::CxxTarget);

impl TargetRef {
    /// Creates a new reference to a C++ target.
    pub fn new(target: &'static crate::bridge::CxxTarget) -> Self {
        Self(target)
    }

    /// Returns the package directory of this target.
    pub fn package(self) -> &'static types::PackageRef {
        self.0.label().package()
    }

    /// Returns the toolchain label for this target.
    pub fn toolchain(self) -> LabelRef<'static> {
        self.0.toolchain()
    }
}

impl allocative::Allocative for TargetRef {
    fn visit<'a, 'b: 'a>(&self, visitor: &'a mut allocative::Visitor<'b>) {
        let visitor = visitor.enter_self_sized::<Self>();
        visitor.exit();
    }
}

// Safety: Target pointers in GN are heap-allocated and thread-safe to transfer
// across evaluation boundaries.
unsafe impl Send for TargetRef {}
// Safety: Target pointers in GN are thread-safe to reference across evaluation
// boundaries.
unsafe impl Sync for TargetRef {}

impl PartialEq for TargetRef {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.0, other.0)
    }
}
impl Eq for TargetRef {}

impl std::ops::Deref for TargetRef {
    type Target = Target;

    fn deref(&self) -> &Self::Target {
        self.0.rust_target()
    }
}

impl std::hash::Hash for TargetRef {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::ptr::hash(self.0, state);
    }
}

impl std::fmt::Debug for TargetRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Delegate to Display
        write!(f, "{self}")
    }
}

impl std::fmt::Display for TargetRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

impl types::IPromiseToImplementStarlarkEqAndHash for TargetRef {}

#[starlark_value(type = "Target")]
impl<'v> StarlarkValue<'v> for TargetRef {
    fn equals(&self, other: Value<'v>) -> starlark::Result<bool> {
        Ok(other
            .downcast_ref::<Self>()
            .is_some_and(|other| other == self))
    }

    fn write_hash(
        &self,
        hasher: &mut starlark::collections::StarlarkHasher,
    ) -> starlark::Result<()> {
        use std::hash::Hash as _;
        self.hash(hasher);
        Ok(())
    }

    fn at(&self, index: Value<'v>, heap: Heap<'v>) -> starlark::Result<Value<'v>> {
        self.evaluated()
            .by_ref_with_reconstructor(|evaluated, reconstructor| {
                match evaluated.providers.get(self, index)? {
                    Some(val) => Ok(reconstructor.edge(heap).rebrand(val)),
                    None => {
                        Err(crate::errors::Error::MissingProvider(*self, index.to_string()).into())
                    },
                }
            })
    }

    fn is_in(&self, index: Value<'v>) -> starlark::Result<bool> {
        self.evaluated()
            .by_ref(|evaluated| evaluated.providers.contains(self, index))
    }

    fn dir_attr(&self) -> Vec<String> {
        vec!["label".to_owned()]
    }

    fn get_attr(&self, attribute: &str, heap: Heap<'v>) -> Option<Value<'v>> {
        match attribute {
            "label" => Some(heap.alloc(self.label().to_owned())),
            _ => None,
        }
    }
}

impl<'v> AllocValue<'v> for TargetRef {
    fn alloc_value(self, heap: Heap<'v>) -> Value<'v> {
        heap.alloc_simple(self)
    }
}

impl types::TargetRef for TargetRef {
    type Rule = rule::Rule<'static, crate::eval_context::EvalContext>;
    type Session = crate::Session;

    fn label(&self) -> LabelRef<'_> {
        self.0.label().as_ref()
    }

    fn toolchain(&self) -> LabelRef<'_> {
        self.0.toolchain()
    }

    fn rule(&self) -> &'static Self::Rule {
        self.rule
    }

    fn outputs(&self) -> Vec<types::File> {
        self.0
            .computed_outputs()
            .iter()
            .map(|f| f.to_rust())
            .collect()
    }

    fn output_type(&self) -> Option<types::OutputType> {
        types::OutputType::from_u8(self.0.output_type())
    }

    fn is_default_toolchain(&self) -> bool {
        self.0.settings().is_default()
    }

    fn builtin_attrs<'v>(&self, heap: &Heap<'v>) -> Vec<Value<'v>> {
        let Some(output_type) = self.output_type() else {
            return vec![];
        };
        let mut attrs = vec![];
        let settings = self.0.settings();
        // The order here needs to match the order in OutputType::attrs
        if output_type.has_sources() {
            attrs.push(heap.alloc(if self.0.all_headers_public() {
                self.0
                    .sources()
                    .iter()
                    .filter(|s| !s.is_header())
                    .map(|s| heap.alloc(s.to_rust(settings)))
                    .collect::<Vec<_>>()
            } else {
                self.0
                    .sources()
                    .iter()
                    .map(|s| heap.alloc(s.to_rust(settings)))
                    .collect::<Vec<_>>()
            }));
        }
        if output_type.has_public() {
            attrs.push(heap.alloc(if self.0.all_headers_public() {
                self.0
                    .sources()
                    .iter()
                    .filter(|s| s.is_header())
                    .map(|s| heap.alloc(s.to_rust(settings)))
                    .collect::<Vec<_>>()
            } else {
                self.0
                    .public_headers()
                    .iter()
                    .map(|s| heap.alloc(s.to_rust(settings)))
                    .collect::<Vec<_>>()
            }));
        }
        if output_type.has_deps() {
            attrs.push(
                heap.alloc(
                    self.0
                        .private_deps()
                        .iter()
                        .map(|target| heap.alloc(target.to_rust()))
                        .collect::<Vec<_>>(),
                ),
            );
        }
        if output_type.has_public_deps() {
            attrs.push(
                heap.alloc(
                    self.0
                        .public_deps()
                        .iter()
                        .map(|target| heap.alloc(target.to_rust()))
                        .collect::<Vec<_>>(),
                ),
            );
        }
        attrs
    }

    fn has_provider(&self, id: types::ProviderId) -> bool {
        self.evaluated
            .get()
            .is_some_and(|evaluated| evaluated.by_ref(|e| e.providers.contains_id(id)))
    }
}

impl attr::TargetAttrExt for TargetRef {
    fn attrs(&self) -> &[attr::Attr] {
        self.attrs.as_slice()
    }

    fn deps(&self) -> impl Iterator<Item = Self> + '_ {
        self.0.starlark_deps().iter().map(|pair| pair.to_rust())
    }
}

impl crate::bridge::LabelTargetPair {
    /// Returns the resolved target.
    ///
    /// Must only be called if the target is already resolved.
    pub fn to_rust(&self) -> TargetRef {
        TargetRef(crate::bridge::label_target_pair_target(self))
    }
}
