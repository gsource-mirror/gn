/*
 * Copyright 2026 The Chromium Authors. All rights reserved.
 * Use of this source code is governed by a BSD-style license that can be
 * found in the LICENSE file.
 */

use starlark::environment::Methods;
use starlark::environment::MethodsBuilder;
use starlark::environment::MethodsStatic;
use starlark::starlark_simple_value;
use starlark::values::record::FrozenRecordType;
use starlark::values::record::RecordType;
use starlark::values::Heap;
use starlark::values::ProvidesStaticType;
use starlark::values::StarlarkValue;
use starlark::values::Trace;
use starlark::values::Tracer;
use starlark::values::Value;
use starlark::values::ValueLike;
use starlark_derive::starlark_module;
use starlark_derive::starlark_value;
use starlark_derive::NoSerialize;
use types::util::extend_lifetime;

use crate::Target;
use crate::Label;

#[derive(Copy, Clone, Debug, ProvidesStaticType, NoSerialize, allocative::Allocative)]

/// A reference to a registered `Target` struct.
pub struct TargetRef(&'static Target);

impl PartialEq for TargetRef {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.0, other.0)
    }
}

impl Eq for TargetRef {}

starlark_simple_value!(TargetRef);

impl std::fmt::Display for TargetRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.0, f)
    }
}

unsafe impl<'v> Trace<'v> for TargetRef {
    fn trace(&mut self, _tracer: &Tracer<'v>) {}
}

impl std::ops::Deref for TargetRef {
    type Target = Target;
    fn deref(&self) -> &'static Self::Target {
        self.0
    }
}

impl From<&Target> for TargetRef {
    fn from(target: &Target) -> Self {
        Self(unsafe { extend_lifetime(target) })
    }
}

impl From<TargetRef> for &'static Target {
    fn from(target_ref: TargetRef) -> Self {
        target_ref.0
    }
}

crate::cannot_freeze!(TargetRef);

impl types::TargetRef for TargetRef {
    fn outputs(&self) -> Vec<types::File> {
        self.0.outputs()
    }

    fn target_out_dir(&self, prefix: &str, suffix: &str, separator: &str) -> String {
        self.0.target_out_dir(prefix, suffix, separator)
    }
}

#[starlark_module]
fn target_methods(methods: &mut MethodsBuilder) {
    #[starlark(attribute)]
    fn label(this: &TargetRef) -> starlark::Result<Label> {
        Ok(this.label().to_owned())
    }
}

#[starlark_value(type = "Target")]
impl<'v> StarlarkValue<'v> for TargetRef {
    fn get_methods() -> Option<&'static Methods>
    where
        Self: Sized,
    {
        static RES: MethodsStatic = MethodsStatic::new("Target", target_methods);
        Some(RES.methods())
    }

    fn at(&self, index: Value<'v>, heap: Heap<'v>) -> starlark::Result<Value<'v>> {
        let type_id = if let Some(rt) = index.downcast_ref::<FrozenRecordType>() {
            rt.type_id()
        } else if let Some(rt) = index.downcast_ref::<RecordType<'v>>() {
            rt.type_id()
        } else {
            return Err(crate::Error::ExpectedProviderType.into());
        };

        if let Some(val) = self.providers().get(&type_id, &heap) {
            return Ok(val);
        }
        Ok(Value::new_none())
    }

    fn is_in(&self, other: Value<'v>) -> starlark::Result<bool> {
        let type_id = if let Some(rt) = other.downcast_ref::<FrozenRecordType>() {
            rt.type_id()
        } else if let Some(rt) = other.downcast_ref::<RecordType<'v>>() {
            rt.type_id()
        } else {
            return Ok(false);
        };
        Ok(self.providers().contains_key(&type_id))
    }
}
