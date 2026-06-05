// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::fmt;
use allocative::Allocative;
use attr::{Attr, AttrSchema, EvalContextExt, TargetRefExt};
use types::{EvalContext, EvaluatorContextExt};
use starlark::coerce::Coerce;
use starlark::collections::SmallMap;
use starlark::eval::{Arguments, Evaluator, ParametersSpec};
use starlark::values::Freeze;
use starlark::values::FreezeResult;
use starlark::values::Freezer;
use starlark::values::Value;
use starlark::values::ValueLike;
use starlark_derive::{NoSerialize, Trace};
use starlark::values::UnpackValue;

/// Generic representation of a Starlark-callable rule object (`RuleCallable`).
#[derive(Debug, Trace, NoSerialize, Allocative)]
#[repr(C)]
pub struct RuleGen<V> {
    /// The implementation function of the rule.
    pub implementation: V,
    /// Map of attribute names to their schemas.
    pub attrs: SmallMap<String, AttrSchema>,
    /// True if this rule is a rule extension.
    pub is_extension: bool,
    pub signature: ParametersSpec<V>,
    pub attrs_record_type: V,
    pub file_record_type: V,
    pub files_record_type: V,
}

impl<V> fmt::Display for RuleGen<V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_extension {
            write!(f, "<rule_extension>")
        } else {
            write!(f, "<rule>")
        }
    }
}

unsafe impl<From, To> Coerce<RuleGen<To>> for RuleGen<From> where From: Coerce<To> {}

impl<'v> Freeze for RuleGen<starlark::values::Value<'v>> {
    type Frozen = RuleGen<starlark::values::FrozenValue>;
    fn freeze(self, freezer: &Freezer) -> FreezeResult<Self::Frozen> {
        Ok(RuleGen {
            implementation: self.implementation.freeze(freezer)?,
            attrs: self.attrs.freeze(freezer)?,
            is_extension: self.is_extension,
            signature: self.signature.freeze(freezer)?,
            attrs_record_type: self.attrs_record_type.freeze(freezer)?,
            file_record_type: self.file_record_type.freeze(freezer)?,
            files_record_type: self.files_record_type.freeze(freezer)?,
        })
    }
}

impl<V> RuleGen<V> {
    pub fn invoke<'v, C: EvalContext + EvalContextExt>(
        &self,
        this: Value<'v>,
        args: &Arguments<'v, '_>,
        eval: &mut Evaluator<'v, '_, '_>,
    ) -> starlark::Result<Value<'v>>
    where
        V: ValueLike<'v>,
        for<'v2> &'v2 <C::Session as attr::Session>::TargetRef: starlark::values::UnpackValue<'v2>,
        <C::Session as types::Session>::TargetRef: TargetRefExt,
    {
        let (target_or_name, attrs) = self.signature.parser(args, eval, |parser, eval| {
            let ctx = eval.context::<C>();

            Ok((parser.next()?, 
            self.attrs
                .iter()
                .map(|(name, schema)| {
                    Attr::create(
                        name,
                        schema,
                        parser.next_opt()?,
                        ctx.current_package(),
                        ctx.path_resolver(),
                    )
                })
                .collect::<starlark::Result<Vec<_>>>()?))
        })?;

        let ctx = eval.context::<C>();
        let session = ctx.session();
        let toolchain = ctx.current_toolchain();

        ctx.require_macro()?;

        let attrs_clone = attrs.clone();

        let target_ref = if self.is_extension {
            let target = <&<C::Session as attr::Session>::TargetRef>::unpack_value_err(target_or_name)?.clone();
            target.set_attrs(attrs);
            target
        } else {
            let name = target_or_name.unpack_str().ok_or_else(|| {
                starlark::Error::new_other(crate::errors::Error::CustomRuleNameRequired)
            })?;
            ctx.create_starlark_target(name, this.unpack_frozen().unwrap(), attrs)?
        };

        for attr in &attrs_clone {
            attr.register_dependencies(session, target_ref.clone(), toolchain);
        }

        Ok(Value::new_none())
    }
}
