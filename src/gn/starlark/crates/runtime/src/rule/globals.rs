// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use starlark::environment::GlobalsBuilder;
use starlark::eval::Evaluator;
use starlark::values::Value;
use starlark_derive::starlark_module;

use attr::AttrSchema;
use attr::EvalContext as _;
use types::EvaluatorContextExt;
use crate::session::EvalContext;
use crate::rule::Rule;

#[starlark_module]
pub fn register_rule(builder: &mut GlobalsBuilder) {
    fn rule<'v>(
        implementation: Value<'v>,
        attrs: Option<starlark::collections::SmallMap<&'v str, &'v AttrSchema>>,
        eval: &mut Evaluator<'v, '_, '_>,
    ) -> starlark::Result<Value<'v>> {
        eval.context::<EvalContext>().require_bzl()?;
        let heap = eval.heap();
        Ok(heap.alloc(Rule::new(
            gn_rule::globals::common_rule(implementation, attrs, false, eval)?,
        )))
    }

    fn rule_extension<'v>(
        implementation: Value<'v>,
        attrs: Option<starlark::collections::SmallMap<&'v str, &'v AttrSchema>>,
        eval: &mut Evaluator<'v, '_, '_>,
    ) -> starlark::Result<Value<'v>> {
        eval.context::<EvalContext>().require_bzl()?;
        let heap = eval.heap();
        Ok(heap.alloc(Rule::new(
            gn_rule::globals::common_rule(implementation, attrs, true, eval)?,
        )))
    }
}
