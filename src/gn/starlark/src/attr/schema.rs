// Copyright 2026 The GN Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use allocative::Allocative;
use starlark::starlark_simple_value;
use starlark::values::{
    ProvidesStaticType, StarlarkValue, Trace, none::NoneOr, Freeze, Freezer, FreezeResult,
};
use starlark_derive::{starlark_value, NoSerialize};
use std::fmt::{self, Display, Formatter};

use starlark::collections::SmallSet;

use super::allow_files::AllowFiles;
use super::cfg::AttrCfg;
use super::attr::Attr;
use super::globals::AttrSpecArgs;
use crate::Error;

#[derive(Debug, Clone, PartialEq, Eq, Allocative)]
pub enum AttrKind {
    Bool,
    Int { allowed: Option<SmallSet<i32>> },
    IntList,
    Label,
    LabelKeyedStringDict,
    LabelList,
    LabelListDict,
    String { allowed: Option<SmallSet<String>> },
    StringDict,
    StringKeyedLabelDict,
    StringList,
    StringListDict,
}

#[derive(Debug, Clone, PartialEq, Eq, Allocative)]
pub enum AllowFilesSchema {
    None,
    Single(AllowFiles),
    Many(AllowFiles),
}

/// Represents an attr.foo(...) parameter.
/// Eg. attr.label(allow_single_file = True)
#[derive(Debug, Clone, PartialEq, Eq, Trace, ProvidesStaticType, NoSerialize, Allocative)]
pub struct AttrSchema {
    pub kind: AttrKind,
    pub default: Option<Attr>,
    pub disallow_empty: bool,
    pub allow_files: AllowFilesSchema,
    pub cfg: AttrCfg,
    pub doc: String,
}

starlark_simple_value!(AttrSchema);

pub type FrozenAttrSchema = AttrSchema;

impl Display for AttrSchema {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "AttrSchema(...)")
    }
}

impl Freeze for AttrSchema {
    type Frozen = AttrSchema;

    fn freeze(self, _freezer: &Freezer) -> FreezeResult<Self::Frozen> {
        Ok(self)
    }
}

#[starlark_value(type = "AttrSchema")]
impl<'v> StarlarkValue<'v> for AttrSchema {}

impl AttrSchema {
    pub(crate) fn create<'v>(
        kind: AttrKind,
        args: AttrSpecArgs<'v>,
        eval: &mut starlark::eval::Evaluator<'v, '_, '_>,
    ) -> starlark::Result<starlark::values::Value<'v>> {
        let heap = eval.heap();

        let mut schema = AttrSchema {
            kind,
            default: None,
            disallow_empty: match args.allow_empty {
                None => false,
                Some(b) => !b,
            },
            allow_files: match (
                    args.allow_single_file.unwrap_or(AllowFiles::None),
                    args.allow_files.unwrap_or(AllowFiles::None),
                ) {
                    (AllowFiles::None, AllowFiles::None) => AllowFilesSchema::None,
                    (af, AllowFiles::None) => AllowFilesSchema::Single(af),
                    (AllowFiles::None, af) => AllowFilesSchema::Many(af),
                    _ => return Err(crate::errors::Error::AllowFilesMutuallyExclusive.into())
                },
            cfg: args.cfg.unwrap_or(AttrCfg::CurrentToolchain),
            doc: match args.doc {
                None | Some(NoneOr::None) => String::new(),
                Some(NoneOr::Other(s)) => s,
            },
        };

        let mandatory = args.mandatory.unwrap_or(false);
        if schema.disallow_empty && !mandatory && args.default.is_none() {
            return Err(Error::AllowEmptyRequiresMandatoryOrDefault.into())
        }

        let default_val = match (mandatory, args.default) {
            (true, None) => None,
            (false, None) => {
                let default_attr = match &schema.kind {
                    AttrKind::Bool => Attr::Bool(false),
                    AttrKind::Int { .. } => Attr::Int(0),
                    AttrKind::String { .. } => Attr::String(String::new()),
                    AttrKind::IntList => Attr::IntList(Vec::new()),
                    AttrKind::StringList => Attr::StringList(Vec::new()),
                    AttrKind::LabelList => Attr::LabelList(Vec::new()),
                    AttrKind::StringListDict => Attr::StringListDict(starlark::collections::SmallMap::new()),
                    AttrKind::StringDict => Attr::StringDict(starlark::collections::SmallMap::new()),
                    AttrKind::LabelKeyedStringDict => Attr::LabelKeyedStringDict(starlark::collections::SmallMap::new()),
                    AttrKind::StringKeyedLabelDict => Attr::StringKeyedLabelDict(starlark::collections::SmallMap::new()),
                    AttrKind::LabelListDict => Attr::LabelListDict(starlark::collections::SmallMap::new()),
                    AttrKind::Label => Attr::None,
                };
                Some(default_attr)
            }
            (false, Some(v)) => {
                // We provide an empty param name because you can write the following code:
                // p = attr.string(default = 1)
                // rule(..., attrs = {"foo": p})
                // This means that at the time you call attr.string, it is unaware of the parameter name.
                Some(Attr::create_without_defaults("", &schema, v, std::ptr::null_mut(), eval.into())?)
            }
            (true, Some(_)) => {
                return Err(crate::errors::Error::MandatoryAndDefaultMutuallyExclusive.into());
            }
        };
        schema.default = default_val;

        Ok(heap.alloc(schema))
    }

    pub(crate) fn file_matcher(&self) -> Option<&AllowFiles> {
        match &self.allow_files {
            AllowFilesSchema::Single(s) => Some(s),
            AllowFilesSchema::Many(s) => Some(s),
            AllowFilesSchema::None => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attr::allow_files::AllowFiles;
    use crate::attr::LabelOrFile;
    use crate::session::EvalKind;
    use crate::label::{Label, Package};

    #[test]
    fn test_schema_bool() {
        let a = crate::Assert::new();
        a.eq(
            "attr.bool()",
            &AttrSchema {
                kind: AttrKind::Bool,
                default: Some(Attr::Bool(false)),
                disallow_empty: false,
                allow_files: AllowFilesSchema::None,
                cfg: AttrCfg::CurrentToolchain,
                doc: String::new(),
            },
        );

        a.fail("attr.bool(default=None)", "expected `bool`");
    }

    #[test]
    fn test_schema_int() {
        let a = crate::Assert::new();
        a.eq(
            "attr.int(default=42, doc='An integer')",
            &AttrSchema {
                kind: AttrKind::Int { allowed: None },
                default: Some(Attr::Int(42)),
                disallow_empty: false,
                allow_files: AllowFilesSchema::None,
                cfg: AttrCfg::CurrentToolchain,
                doc: "An integer".to_string(),
            },
        );
    }

    #[test]
    fn test_schema_string() {
        let a = crate::Assert::new();
        let mut allowed = starlark::collections::SmallSet::new();
        allowed.insert("foo".to_string());
        allowed.insert("bar".to_string());
        a.eq(
            "attr.string(values=['foo', 'bar'], mandatory=True)",
            &AttrSchema {
                kind: AttrKind::String { allowed: Some(allowed) },
                default: None,
                disallow_empty: false,
                allow_files: AllowFilesSchema::None,
                cfg: AttrCfg::CurrentToolchain,
                doc: String::new(),
            },
        );
    }

    #[test]
    fn test_schema_label() {
        let a = crate::Assert::new().configure(EvalKind::BzlFile(Package("//pkg".to_owned())));
        a.eq(
            "attr.label(allow_files=True, default=':foo')",
            &AttrSchema {
                kind: AttrKind::Label,
                // Relative labels should be resolved relative to the bzl file, not the caller.
                default: Some(Attr::Label(LabelOrFile::Label(Label::new(Package("//pkg".to_owned()), "foo".to_owned())))),
                disallow_empty: false,
                allow_files: AllowFilesSchema::Many(AllowFiles::All),
                cfg: AttrCfg::CurrentToolchain,
                doc: String::new(),
            },
        );
    }

    #[test]
    fn test_schema_string_list() {
        let a = crate::Assert::new();
        a.eq(
            "attr.string_list(allow_empty=False, mandatory=True)",
            &AttrSchema {
                kind: AttrKind::StringList,
                default: None,
                disallow_empty: true,
                allow_files: AllowFilesSchema::None,
                cfg: AttrCfg::CurrentToolchain,
                doc: String::new(),
            },
        );
    }

    #[test]
    fn test_schema_string_default() {
        let a = crate::Assert::new();
        a.eq(
            "attr.string(default='hello')",
            &AttrSchema {
                kind: AttrKind::String { allowed: None },
                default: Some(Attr::String("hello".to_string())),
                disallow_empty: false,
                allow_files: AllowFilesSchema::None,
                cfg: AttrCfg::CurrentToolchain,
                doc: String::new(),
            },
        );
    }

    #[test]
    fn test_schema_allow_files_error() {
        let a = crate::Assert::new();
        a.fail(
            "attr.label(allow_files=True, allow_single_file=True)",
            "allow_files and allow_single_file are mutually exclusive",
        );
    }

    #[test]
    fn test_schema_mandatory_default_error() {
        let a = crate::Assert::new();
        a.fail(
            "attr.bool(mandatory=True, default=True)",
            "mandatory and default are mutually exclusive",
        );
    }

    #[test]
    fn test_schema_default_validation_error() {
        let a = crate::Assert::new();
        a.fail(
            "attr.int(values=[1, 2], default=3)",
            "Value 3 is not in allowed set",
        );
        a.fail(
            "attr.string(values=['a', 'b'], default='c')",
            "Value \"c\" is not in allowed set",
        );
        a.fail(
            "attr.string_list(allow_empty=False)",
            "allow_empty = False requires the attribute to be mandatory or have a non-empty default value",
        );
        a.fail(
            "attr.string_list(allow_empty=False, default=[])",
            "Want non-empty list, got []",
        );
        a.fail(
            "attr.string_dict(allow_empty=False, default={})",
            "Want non-empty dict, got {}",
        );
    }
}