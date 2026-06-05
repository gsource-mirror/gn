// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::{fmt, marker::PhantomData};

use allocative::Allocative;
use attr::{traits::EvalContextAttrExt, Attr, CtxAttrSchema};
use starlark::{
    any::ProvidesStaticType,
    collections::SmallMap,
    eval::{Arguments, Evaluator, ParametersSpec},
    values::{FrozenHeap, FrozenValue, FrozenValueTyped, Heap, StarlarkValue, Value},
};
use starlark_derive::{starlark_value, NoSerialize};
use types::{EvalContext, EvaluatorContextExt, Scope, Session};

use crate::rule::build_signature;

/// A frozen representation of a Starlark rule object.
///
/// Once a rule has been exported from a loaded Starlark module (e.g., from
/// `.bzl` files), it is frozen into a `FrozenRule` and is ready to be invoked
/// in target build files.
#[derive(NoSerialize, Allocative)]
pub struct FrozenRule<C: EvalContext + EvalContextAttrExt> {
    pub(crate) schema: CtxAttrSchema,
    pub(crate) builtin: Option<&'static str>,
    pub(crate) implementation: FrozenValue,
    pub(crate) once_named: Option<(String, ParametersSpec<FrozenValue>)>,
    pub(crate) parent: Option<FrozenValueTyped<'static, FrozenRule<C>>>,
    pub(crate) _phantom: PhantomData<C>,
}

unsafe impl<'v, C: EvalContext + EvalContextAttrExt> ProvidesStaticType<'v> for FrozenRule<C> {
    type StaticType = FrozenRule<C>;
}

unsafe impl<'v, C: EvalContext + EvalContextAttrExt> starlark::values::Trace<'v> for FrozenRule<C> {
    fn trace(&mut self, _tracer: &starlark::values::Tracer<'v>) {}
}

impl<C: EvalContext + EvalContextAttrExt> fmt::Debug for FrozenRule<C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FrozenRule")
            .field("schema", &self.schema)
            .field("builtin", &self.builtin)
            .field("implementation", &self.implementation)
            .field("parent", &self.parent)
            .finish()
    }
}

impl<C: EvalContext + EvalContextAttrExt> FrozenRule<C> {
    /// invoke_native is called when targets are invoked directly from GN.
    pub fn invoke_native<'v>(
        &self,
        me: FrozenValue,
        ctx: &C,
        target_name: &str,
        heap: &'v Heap<'v>,
    ) -> starlark::Result<<C::Session as Session>::TargetRef> {
        let scope = ctx.require_macro().unwrap();
        let attrs = self
            .schema
            .attrs()
            .iter()
            .map(|(name, schema)| {
                let value_opt = scope.get(name, heap);
                attr::Attr::create(
                    schema,
                    value_opt,
                    ctx.current_package(),
                    ctx.path_resolver(),
                )
            })
            .collect::<starlark::Result<Vec<_>>>()?;

        if let Some(builtin) = self.builtin {
            ctx.create_target(builtin, target_name, scope, me, attrs)
        } else {
            ctx.create_target("starlark", target_name, scope, me, attrs)
        }
    }
}

impl<C: EvalContext + EvalContextAttrExt> FrozenRule<C> {
    /// Creates a new `FrozenRule` for a built-in rule.
    pub fn new_builtin(builtin: &'static str, frozen_heap: &FrozenHeap) -> Self {
        let schema = CtxAttrSchema::new(SmallMap::new(), frozen_heap);
        let signature = build_signature(builtin, &schema, true);
        Self {
            schema,
            builtin: Some(builtin),
            implementation: FrozenValue::new_none(),
            once_named: Some((builtin.to_owned(), signature)),
            parent: None,
            _phantom: PhantomData,
        }
    }
}

#[starlark_value(type = "rule")]
impl<'v, C: EvalContext + EvalContextAttrExt> StarlarkValue<'v> for FrozenRule<C>
where
    Self: ProvidesStaticType<'v>,
{
    type Canonical = Self;

    /// Invoking a rule generates a target.
    /// Note: This is only used when calling my_rule(...) from *starlark*.
    /// When calling from GN directly, we use custom logic to handle scoping
    /// correctly.
    fn invoke(
        &self,
        me: Value<'v>,
        args: &Arguments<'v, '_>,
        eval: &mut Evaluator<'v, '_, '_>,
    ) -> starlark::Result<Value<'v>> {
        let me = me.unpack_frozen().unwrap();
        let (_name, signature) = self
            .once_named
            .as_ref()
            .ok_or_else(|| starlark::Error::new_other(crate::errors::Error::RuleMustBeNamed))?;

        signature.parser(args, eval, |param_parser, eval| {
            let target_name: &str = param_parser.next()?;

            let context = eval.context_mut::<C>();
            let scope = context.require_macro()?;
            let package = context.current_package();
            let path_resolver = context.path_resolver();

            let attrs = self
                .schema
                .attrs()
                .iter()
                .map(|(_name, schema)| {
                    let value_opt: Option<Value<'v>> = param_parser.next_opt()?;
                    Attr::create(schema, value_opt, package, path_resolver)
                })
                .collect::<Result<Vec<_>, _>>()?;

            if let Some(builtin) = self.builtin {
                // Collect all the arguments we don't recognise and pass them to the native
                // implementation.
                let kwargs: SmallMap<String, Value<'v>> = param_parser.next()?;
                let child_scope = scope.copy_with(kwargs.iter().map(|(k, v)| (k.as_str(), *v)));
                context.create_target(builtin, target_name, &child_scope, me, attrs)?;
            } else {
                context.create_target("starlark", target_name, scope, me, attrs)?;
            }

            Ok(eval.heap().alloc(FrozenValue::new_none()))
        })
    }
}

impl<C: EvalContext + EvalContextAttrExt> starlark::values::AllocFrozenValue for FrozenRule<C> {
    #[inline]
    fn alloc_frozen_value(
        self,
        heap: &starlark::values::FrozenHeap,
    ) -> starlark::values::FrozenValue {
        heap.alloc_simple(self)
    }
}

impl<C: EvalContext + EvalContextAttrExt> fmt::Display for FrozenRule<C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some((name, _)) = &self.once_named {
            write!(f, "<rule: {name}>")
        } else {
            write!(f, "<anonymous rule>")
        }
    }
}

#[cfg(test)]
mod tests {
    use attr::{Attr, LabelOrFile};
    use types::{Label, PackageRef};

    use crate::globals::tests::new_assert;

    #[test]
    fn test_pure_rule_inheritance() {
        let mut assert = new_assert();
        assert.load_module("//rules:native.scl");
        assert.load_module("//rules:pure.scl");

        assert.pass(
            r#"
load("//builtins:rules.scl", "executable")
load("//rules:pure.scl", "child_rule", "parent_rule")
load("//rules:native.scl", "custom_shared_library", "static_library")

custom_shared_library(
    name = "shared_library",
    mandatory = "mandatory_val",
    optional = "optional_val",
    unknown = "unknown",
)

static_library(
   name = "static_library",
   optional = "optional_val",
   unknown = "unknown"
)

executable(
    name = "executable",
    unknown = "unknown",
)

parent_rule(
    name = "parent_defaulted",
    parent_only = "p",
)

child_rule(
    name = "child_defaulted",
    parent_only = "p",
    child_only = "c",
)

child_rule(
    name = "child_override",
    parent_only = "parent_val",
    child_only = "child_val",
    override = "//:custom_val",
)
"#,
        );

        let context = assert.context();
        let targets_lock = context.session.targets.lock().unwrap();
        let load = |name: &str| {
            let label = Label::new(PackageRef::root().to_owned(), name.to_owned());
            targets_lock
                .get(&(label, context.session.default_toolchain.clone()))
                .unwrap()
                .get()
        };

        assert_eq!(
            load("shared_library").attrs,
            vec![
                Attr::String("optional_val".to_owned()),
                Attr::String("mandatory_val".to_owned()),
            ]
        );

        assert_eq!(
            load("static_library").attrs,
            vec![Attr::String("optional_val".to_owned())]
        );

        assert_eq!(
            load("parent_defaulted").attrs,
            vec![
                Attr::String("p".to_owned()),
                Attr::Label(Some(LabelOrFile::Label(Label::new(
                    PackageRef::root().to_owned(),
                    "parent".to_owned()
                )))),
            ]
        );

        assert_eq!(
            load("child_defaulted").attrs,
            vec![
                Attr::String("c".to_owned()),
                Attr::Label(Some(LabelOrFile::Label(Label::new(
                    PackageRef::root().to_owned(),
                    "child".to_owned()
                )))),
                Attr::String("p".to_owned()),
            ]
        );

        assert_eq!(
            load("child_override").attrs,
            vec![
                Attr::String("child_val".to_owned()),
                Attr::Label(Some(LabelOrFile::Label(Label::new(
                    PackageRef::root().to_owned(),
                    "custom_val".to_owned()
                )))),
                Attr::String("parent_val".to_owned()),
            ]
        );
    }
}
