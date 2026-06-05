// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use starlark::values::none::NoneOr;
use starlark::values::Value;

use crate::allow_files::AllowFiles;
use crate::cfg::AttrCfg;

/// The type that all parameters of attr.* get converted to.
/// Mostly used because rust doesn't have default parameters,
/// so we just ..Default::default() the fields that aren't used.
///
/// Solely pub because it's accessed by the macro defined below from another crate.
#[derive(Default)]
#[doc(hidden)]
pub struct AttrSpecArgs<'v> {
    pub default: Option<Value<'v>>,
    pub mandatory: Option<bool>,
    pub allow_empty: Option<bool>,
    pub allow_files: Option<AllowFiles>,
    pub allow_single_file: Option<AllowFiles>,
    pub cfg: Option<AttrCfg>,
    pub doc: Option<NoneOr<String>>,
}

/// Macro to declare the Starlark `attr` module.
/// We use a macro to avoid boilerplate code duplication between tests,
/// which grab FakeEvalContext from the evaluator, and real code,
/// which grabs EvalContext.
///
/// We could use a `dyn` trait cast instead, but this requires dynamic dispatch,
/// Which I'd rather avoid in the hot path.
#[macro_export]
macro_rules! declare_attr_module {
    ($make_attr_schema_fn:path) => {
        use std::fmt;
        use std::fmt::Display;
        use std::fmt::Formatter;

        use allocative::Allocative;
        use starlark::environment::Methods;
        use starlark::environment::MethodsBuilder;
        use starlark::environment::MethodsStatic;
        use starlark::eval::Evaluator;
        use starlark::starlark_simple_value;
        use starlark::values::list::UnpackList;
        use starlark::values::none::NoneOr;
        use starlark::values::ProvidesStaticType;
        use starlark::values::StarlarkValue;
        use starlark::values::Value;
        use starlark_derive::starlark_module;
        use starlark_derive::starlark_value;
        use starlark_derive::NoSerialize;

        /// The Starlark `attr` module containing functions to declare rule attributes.
        #[derive(Debug, ProvidesStaticType, NoSerialize, Allocative)]
        pub struct AttrModule;

        starlark_simple_value!(AttrModule);

        impl Display for AttrModule {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                write!(f, "attr")
            }
        }

        #[starlark_value(type = "attr")]
        // Clippy recommends eliding the 'v lifetime, but the #[starlark_value] macro requires it to be explicitly declared.
        #[allow(clippy::elidable_lifetime_names)]
        impl<'v> StarlarkValue<'v> for AttrModule {
            fn get_methods() -> Option<&'static Methods> {
                static RES: MethodsStatic = MethodsStatic::new("attr", attr_methods);
                Some(RES.methods())
            }
        }

        #[starlark_module]
        pub fn attr_methods(builder: &mut MethodsBuilder) {
            fn bool<'v>(
                #[starlark(this)] _this: &AttrModule,
                default: Option<Value<'v>>,
                doc: Option<NoneOr<String>>,
                mandatory: Option<bool>,
                eval: &mut Evaluator<'v, '_, '_>,
            ) -> starlark::Result<Value<'v>> {
                $make_attr_schema_fn(
                    $crate::AttrKind::Bool,
                    $crate::AttrSpecArgs {
                        default,
                        doc,
                        mandatory,
                        ..Default::default()
                    },
                    eval,
                )
            }

            fn int<'v>(
                #[starlark(this)] _this: &AttrModule,
                default: Option<Value<'v>>,
                doc: Option<NoneOr<String>>,
                mandatory: Option<bool>,
                values: Option<UnpackList<i32>>,
                eval: &mut Evaluator<'v, '_, '_>,
            ) -> starlark::Result<Value<'v>> {
                $make_attr_schema_fn(
                    $crate::AttrKind::Int {
                        allowed: values.map(|v| v.into_iter().collect()),
                    },
                    $crate::AttrSpecArgs {
                        default,
                        doc,
                        mandatory,
                        ..Default::default()
                    },
                    eval,
                )
            }

            fn int_list<'v>(
                #[starlark(this)] _this: &AttrModule,
                default: Option<Value<'v>>,
                doc: Option<NoneOr<String>>,
                mandatory: Option<bool>,
                allow_empty: Option<bool>,
                eval: &mut Evaluator<'v, '_, '_>,
            ) -> starlark::Result<Value<'v>> {
                $make_attr_schema_fn(
                    $crate::AttrKind::IntList,
                    $crate::AttrSpecArgs {
                        default,
                        doc,
                        mandatory,
                        allow_empty,
                        ..Default::default()
                    },
                    eval,
                )
            }

            fn label<'v>(
                #[starlark(this)] _this: &AttrModule,
                default: Option<Value<'v>>,
                doc: Option<NoneOr<String>>,
                mandatory: Option<bool>,
                allow_files: Option<$crate::AllowFiles>,
                allow_single_file: Option<$crate::AllowFiles>,
                cfg: Option<$crate::AttrCfg>,
                eval: &mut Evaluator<'v, '_, '_>,
            ) -> starlark::Result<Value<'v>> {
                $make_attr_schema_fn(
                    $crate::AttrKind::Label,
                    $crate::AttrSpecArgs {
                        default,
                        doc,
                        mandatory,
                        allow_files,
                        allow_single_file,
                        cfg,
                        ..Default::default()
                    },
                    eval,
                )
            }

            fn label_keyed_string_dict<'v>(
                #[starlark(this)] _this: &AttrModule,
                default: Option<Value<'v>>,
                doc: Option<NoneOr<String>>,
                mandatory: Option<bool>,
                allow_empty: Option<bool>,
                eval: &mut Evaluator<'v, '_, '_>,
            ) -> starlark::Result<Value<'v>> {
                $make_attr_schema_fn(
                    $crate::AttrKind::LabelKeyedStringDict,
                    $crate::AttrSpecArgs {
                        default,
                        doc,
                        mandatory,
                        allow_empty,
                        ..Default::default()
                    },
                    eval,
                )
            }

            fn label_list<'v>(
                #[starlark(this)] _this: &AttrModule,
                default: Option<Value<'v>>,
                doc: Option<NoneOr<String>>,
                mandatory: Option<bool>,
                allow_empty: Option<bool>,
                allow_files: Option<$crate::AllowFiles>,
                allow_single_file: Option<$crate::AllowFiles>,
                cfg: Option<$crate::AttrCfg>,
                eval: &mut Evaluator<'v, '_, '_>,
            ) -> starlark::Result<Value<'v>> {
                $make_attr_schema_fn(
                    $crate::AttrKind::LabelList,
                    $crate::AttrSpecArgs {
                        default,
                        doc,
                        mandatory,
                        allow_empty,
                        allow_files,
                        allow_single_file,
                        cfg,
                        ..Default::default()
                    },
                    eval,
                )
            }

            fn label_list_dict<'v>(
                #[starlark(this)] _this: &AttrModule,
                default: Option<Value<'v>>,
                doc: Option<NoneOr<String>>,
                mandatory: Option<bool>,
                allow_empty: Option<bool>,
                allow_files: Option<$crate::AllowFiles>,
                allow_single_file: Option<$crate::AllowFiles>,
                cfg: Option<$crate::AttrCfg>,
                eval: &mut Evaluator<'v, '_, '_>,
            ) -> starlark::Result<Value<'v>> {
                $make_attr_schema_fn(
                    $crate::AttrKind::LabelListDict,
                    $crate::AttrSpecArgs {
                        default,
                        doc,
                        mandatory,
                        allow_empty,
                        allow_files,
                        allow_single_file,
                        cfg,
                        ..Default::default()
                    },
                    eval,
                )
            }

            fn string<'v>(
                #[starlark(this)] _this: &AttrModule,
                default: Option<Value<'v>>,
                doc: Option<NoneOr<String>>,
                mandatory: Option<bool>,
                values: Option<UnpackList<String>>,
                eval: &mut Evaluator<'v, '_, '_>,
            ) -> starlark::Result<Value<'v>> {
                $make_attr_schema_fn(
                    $crate::AttrKind::String {
                        allowed: values.map(|v| v.into_iter().collect()),
                    },
                    $crate::AttrSpecArgs {
                        default,
                        doc,
                        mandatory,
                        ..Default::default()
                    },
                    eval,
                )
            }

            fn string_dict<'v>(
                #[starlark(this)] _this: &AttrModule,
                default: Option<Value<'v>>,
                doc: Option<NoneOr<String>>,
                mandatory: Option<bool>,
                allow_empty: Option<bool>,
                eval: &mut Evaluator<'v, '_, '_>,
            ) -> starlark::Result<Value<'v>> {
                $make_attr_schema_fn(
                    $crate::AttrKind::StringDict,
                    $crate::AttrSpecArgs {
                        default,
                        doc,
                        mandatory,
                        allow_empty,
                        ..Default::default()
                    },
                    eval,
                )
            }

            fn string_keyed_label_dict<'v>(
                #[starlark(this)] _this: &AttrModule,
                default: Option<Value<'v>>,
                doc: Option<NoneOr<String>>,
                mandatory: Option<bool>,
                allow_empty: Option<bool>,
                allow_files: Option<$crate::AllowFiles>,
                allow_single_file: Option<$crate::AllowFiles>,
                cfg: Option<$crate::AttrCfg>,
                eval: &mut Evaluator<'v, '_, '_>,
            ) -> starlark::Result<Value<'v>> {
                $make_attr_schema_fn(
                    $crate::AttrKind::StringKeyedLabelDict,
                    $crate::AttrSpecArgs {
                        default,
                        doc,
                        mandatory,
                        allow_empty,
                        allow_files,
                        allow_single_file,
                        cfg,
                        ..Default::default()
                    },
                    eval,
                )
            }

            fn string_list<'v>(
                #[starlark(this)] _this: &AttrModule,
                default: Option<Value<'v>>,
                doc: Option<NoneOr<String>>,
                mandatory: Option<bool>,
                allow_empty: Option<bool>,
                eval: &mut Evaluator<'v, '_, '_>,
            ) -> starlark::Result<Value<'v>> {
                $make_attr_schema_fn(
                    $crate::AttrKind::StringList,
                    $crate::AttrSpecArgs {
                        default,
                        doc,
                        mandatory,
                        allow_empty,
                        ..Default::default()
                    },
                    eval,
                )
            }

            fn string_list_dict<'v>(
                #[starlark(this)] _this: &AttrModule,
                default: Option<Value<'v>>,
                doc: Option<NoneOr<String>>,
                mandatory: Option<bool>,
                allow_empty: Option<bool>,
                eval: &mut Evaluator<'v, '_, '_>,
            ) -> starlark::Result<Value<'v>> {
                $make_attr_schema_fn(
                    $crate::AttrKind::StringListDict,
                    $crate::AttrSpecArgs {
                        default,
                        doc,
                        mandatory,
                        allow_empty,
                        ..Default::default()
                    },
                    eval,
                )
            }
        }
    };
}

#[cfg(test)]
pub(crate) mod tests {
    fn make_attr_schema<'v>(
        kind: crate::AttrKind,
        args: crate::AttrSpecArgs<'v>,
        eval: &mut starlark::eval::Evaluator<'v, '_, '_>,
    ) -> starlark::Result<starlark::values::Value<'v>> {
        let package = types::PackageRef::root();
        crate::AttrSchema::create(
            kind,
            args,
            package,
            &types::PathResolver::new_for_testing(),
            &eval.heap(),
        )
    }

    crate::declare_attr_module!(make_attr_schema);

    pub fn new_attr_assert() -> starlark::assert::Assert<'static> {
        let mut assert = starlark::assert::Assert::new();
        assert.globals_add(|builder| {
            builder.set("attr", AttrModule);
        });
        assert
    }
}
