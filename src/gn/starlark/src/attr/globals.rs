// Copyright 2026 The GN Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::fmt::{self, Display, Formatter};
use allocative::Allocative;
use starlark::environment::{GlobalsBuilder, MethodsBuilder};
use starlark::eval::Evaluator;
use starlark::values::{ProvidesStaticType, StarlarkValue, Value, list::UnpackList, none::NoneOr};
use starlark::starlark_simple_value;
use starlark_derive::{starlark_module, starlark_value, NoSerialize};
use super::allow_files::AllowFiles;
use super::AttrKind;
use super::schema::AttrSchema;
use super::cfg::AttrCfg;

// Helper struct to create the schema because rust doesn't support default arguments.
#[derive(Default)]
pub(crate) struct AttrSpecArgs<'v> {
    pub default: Option<Value<'v>>,
    pub mandatory: Option<bool>,
    pub allow_empty: Option<bool>,
    pub allow_files: Option<AllowFiles>,
    pub allow_single_file: Option<AllowFiles>,
    pub cfg: Option<AttrCfg>,
    pub doc: Option<NoneOr<String>>,
}

#[derive(Debug, ProvidesStaticType, NoSerialize, Allocative)]
pub struct AttrModule;

starlark_simple_value!(AttrModule);

impl Display for AttrModule {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "attr")
    }
}

#[starlark_value(type = "attr")]
impl<'v> StarlarkValue<'v> for AttrModule {
    fn get_methods() -> Option<&'static starlark::environment::Methods> {
        static RES: starlark::environment::MethodsStatic = starlark::environment::MethodsStatic::new("attr", attr_methods);
        Some(RES.methods())
    }
}

pub fn register_attr(builder: &mut GlobalsBuilder) {
    builder.set("attr", AttrModule);
}

#[starlark_module]
pub fn attr_methods(builder: &mut MethodsBuilder) {
    #[allow(unused_variables)]
    fn bool<'v>(
        this: &AttrModule,
        default: Option<Value<'v>>,
        doc: Option<NoneOr<String>>,
        mandatory: Option<bool>,
        eval: &mut Evaluator<'v, '_, '_>,
    ) -> starlark::Result<Value<'v>> {
        AttrSchema::create(
            AttrKind::Bool,
            AttrSpecArgs {
                default,
                doc,
                mandatory,
                ..Default::default()
            },
            eval,
        )
    }

    #[allow(unused_variables)]
    fn int<'v>(
        this: &AttrModule,
        default: Option<Value<'v>>,
        doc: Option<NoneOr<String>>,
        mandatory: Option<bool>,
        values: Option<UnpackList<i32>>,
        eval: &mut Evaluator<'v, '_, '_>,
    ) -> starlark::Result<Value<'v>> {
        AttrSchema::create(
            AttrKind::Int {
                allowed: values.map(|v| v.into_iter().collect()),
            },
            AttrSpecArgs {
                default,
                doc,
                mandatory,
                ..Default::default()
            },
            eval,
        )
    }

    #[allow(unused_variables)]
    fn int_list<'v>(
        this: &AttrModule,
        default: Option<Value<'v>>,
        doc: Option<NoneOr<String>>,
        mandatory: Option<bool>,
        allow_empty: Option<bool>,
        eval: &mut Evaluator<'v, '_, '_>,
    ) -> starlark::Result<Value<'v>> {
        AttrSchema::create(
            AttrKind::IntList,
            AttrSpecArgs {
                default,
                doc,
                mandatory,
                allow_empty,
                ..Default::default()
            },
            eval,
        )
    }

    #[allow(unused_variables)]
    fn label<'v>(
        this: &AttrModule,
        default: Option<Value<'v>>,
        doc: Option<NoneOr<String>>,
        mandatory: Option<bool>,
        allow_files: Option<AllowFiles>,
        allow_single_file: Option<AllowFiles>,
        cfg: Option<AttrCfg>,
        eval: &mut Evaluator<'v, '_, '_>,
    ) -> starlark::Result<Value<'v>> {
        AttrSchema::create(
            AttrKind::Label,
            AttrSpecArgs {
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

    #[allow(unused_variables)]
    fn label_keyed_string_dict<'v>(
        this: &AttrModule,
        default: Option<Value<'v>>,
        doc: Option<NoneOr<String>>,
        mandatory: Option<bool>,
        allow_empty: Option<bool>,
        eval: &mut Evaluator<'v, '_, '_>,
    ) -> starlark::Result<Value<'v>> {
        AttrSchema::create(
            AttrKind::LabelKeyedStringDict,
            AttrSpecArgs {
                default,
                doc,
                mandatory,
                allow_empty,
                ..Default::default()
            },
            eval,
        )
    }

    #[allow(unused_variables)]
    fn label_list<'v>(
        this: &AttrModule,
        default: Option<Value<'v>>,
        doc: Option<NoneOr<String>>,
        mandatory: Option<bool>,
        allow_empty: Option<bool>,
        allow_files: Option<AllowFiles>,
        allow_single_file: Option<AllowFiles>,
        cfg: Option<AttrCfg>,
        eval: &mut Evaluator<'v, '_, '_>,
    ) -> starlark::Result<Value<'v>> {
        AttrSchema::create(
            AttrKind::LabelList,
            AttrSpecArgs {
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

    #[allow(unused_variables)]
    fn label_list_dict<'v>(
        this: &AttrModule,
        default: Option<Value<'v>>,
        doc: Option<NoneOr<String>>,
        mandatory: Option<bool>,
        allow_empty: Option<bool>,
        allow_files: Option<AllowFiles>,
        allow_single_file: Option<AllowFiles>,
        cfg: Option<AttrCfg>,
        eval: &mut Evaluator<'v, '_, '_>,
    ) -> starlark::Result<Value<'v>> {
        AttrSchema::create(
            AttrKind::LabelListDict,
            AttrSpecArgs {
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

    #[allow(unused_variables)]
    fn string<'v>(
        this: &AttrModule,
        default: Option<Value<'v>>,
        doc: Option<NoneOr<String>>,
        mandatory: Option<bool>,
        values: Option<UnpackList<String>>,
        eval: &mut Evaluator<'v, '_, '_>,
    ) -> starlark::Result<Value<'v>> {
        AttrSchema::create(
            AttrKind::String {
                allowed: values.map(|v| v.into_iter().collect()),
            },
            AttrSpecArgs {
                default,
                doc,
                mandatory,
                ..Default::default()
            },
            eval,
        )
    }

    #[allow(unused_variables)]
    fn string_dict<'v>(
        this: &AttrModule,
        default: Option<Value<'v>>,
        doc: Option<NoneOr<String>>,
        mandatory: Option<bool>,
        allow_empty: Option<bool>,
        eval: &mut Evaluator<'v, '_, '_>,
    ) -> starlark::Result<Value<'v>> {
        AttrSchema::create(
            AttrKind::StringDict,
            AttrSpecArgs {
                default,
                doc,
                mandatory,
                allow_empty,
                ..Default::default()
            },
            eval,
        )
    }

    #[allow(unused_variables)]
    fn string_keyed_label_dict<'v>(
        this: &AttrModule,
        default: Option<Value<'v>>,
        doc: Option<NoneOr<String>>,
        mandatory: Option<bool>,
        allow_empty: Option<bool>,
        allow_files: Option<AllowFiles>,
        allow_single_file: Option<AllowFiles>,
        cfg: Option<AttrCfg>,
        eval: &mut Evaluator<'v, '_, '_>,
    ) -> starlark::Result<Value<'v>> {
        AttrSchema::create(
            AttrKind::StringKeyedLabelDict,
            AttrSpecArgs {
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

    #[allow(unused_variables)]
    fn string_list<'v>(
        this: &AttrModule,
        default: Option<Value<'v>>,
        doc: Option<NoneOr<String>>,
        mandatory: Option<bool>,
        allow_empty: Option<bool>,
        eval: &mut Evaluator<'v, '_, '_>,
    ) -> starlark::Result<Value<'v>> {
        AttrSchema::create(
            AttrKind::StringList,
            AttrSpecArgs {
                default,
                doc,
                mandatory,
                allow_empty,
                ..Default::default()
            },
            eval,
        )
    }

    #[allow(unused_variables)]
    fn string_list_dict<'v>(
        this: &AttrModule,
        default: Option<Value<'v>>,
        doc: Option<NoneOr<String>>,
        mandatory: Option<bool>,
        allow_empty: Option<bool>,
        eval: &mut Evaluator<'v, '_, '_>,
    ) -> starlark::Result<Value<'v>> {
        AttrSchema::create(
            AttrKind::StringListDict,
            AttrSpecArgs {
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
