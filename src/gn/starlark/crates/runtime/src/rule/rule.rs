// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::sync::OnceLock;
use std::fmt;

use allocative::Allocative;
use gn_rule::RuleGen;
use starlark::eval::{Arguments, Evaluator};
use starlark::values::{
    AllocFrozenValue, AllocValue, Freeze, FreezeResult, Freezer, FrozenHeap, FrozenValue, Heap,
    ProvidesStaticType, StarlarkValue, Trace, Value, ValueLike,
};
use starlark_derive::{starlark_value, NoSerialize};

use crate::session::EvalContext;

/// A wrapper around crates::rule which actually adds the starlark methods.
/// The crates::rule version depends on the interface, and does not have access
/// to the EvalContext implementation. 
#[derive(Debug, Trace, NoSerialize, Allocative, ProvidesStaticType)]
pub struct Rule<V> {
    rule_gen: RuleGen<V>,
    #[allocative(skip)]
    name: OnceLock<String>,
}

impl<V> Rule<V> {
    pub fn new(rule_gen: RuleGen<V>) -> Self {
        Self {
            rule_gen,
            name: OnceLock::new(),
        }
    }

    pub fn name(&self) -> Option<&str> {
        self.name.get().map(|s: &String| s.as_str())
    }
}

pub type FrozenRule = Rule<FrozenValue>;

impl<V> std::ops::Deref for Rule<V> {
    type Target = RuleGen<V>;
    fn deref(&self) -> &Self::Target {
        &self.rule_gen
    }
}

impl<'v> Freeze for Rule<Value<'v>> {
    type Frozen = Rule<FrozenValue>;
    fn freeze(self, freezer: &Freezer) -> FreezeResult<Self::Frozen> {
        Ok(Rule {
            rule_gen: self.rule_gen.freeze(freezer)?,
            name: self.name.clone(),
        })
    }
}

impl<'v> AllocValue<'v> for Rule<Value<'v>> {
    fn alloc_value(self, heap: Heap<'v>) -> Value<'v> {
        heap.alloc_complex(self)
    }
}

impl AllocFrozenValue for Rule<FrozenValue> {
    fn alloc_frozen_value(self, heap: &FrozenHeap) -> FrozenValue {
        heap.alloc_simple(self)
    }
}

impl<V> fmt::Display for Rule<V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = self.name.get().map(|s: &String| s.as_str()).unwrap_or("anonymous");
        if self.is_extension {
            write!(f, "<rule_extension {}>", name)
        } else {
            write!(f, "<rule {}>", name)
        }
    }
}

#[starlark_value(type = "rule_callable")]
impl<'v, V: ValueLike<'v>> StarlarkValue<'v> for Rule<V>
where
    Self: ProvidesStaticType<'v>,
    for<'v2> &'v2 <<EvalContext as attr::EvalContext>::Session as attr::Session>::TargetRef:
        starlark::values::UnpackValue<'v2>,
{
    type Canonical = Self;

    fn export_as(
        &self,
        variable_name: &str,
        _eval: &mut Evaluator<'v, '_, '_>,
    ) -> starlark::Result<()> {
        let _unused = self.name.set(variable_name.to_owned());
        Ok(())
    }

    fn collect_repr(&self, collector: &mut String) {
        use std::fmt::Write;
        write!(collector, "{}", self).unwrap();
    }

    fn invoke(
        &self,
        this: Value<'v>,
        args: &Arguments<'v, '_>,
        eval: &mut Evaluator<'v, '_, '_>,
    ) -> starlark::Result<Value<'v>> {
        self.rule_gen.invoke::<EvalContext>(this, args, eval)
    }
}
