// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;

use allocative::Allocative;
use starlark::starlark_simple_value;
use starlark::values::none::NoneOr;
use starlark::values::Freeze;
use starlark::values::FreezeResult;
use starlark::values::Freezer;
use starlark::values::Heap;
use starlark::values::ProvidesStaticType;
use starlark::values::StarlarkValue;
use starlark::values::Trace;
use starlark::values::Value;
use starlark_derive::starlark_value;
use starlark_derive::NoSerialize;
use types::{PackageRef, PathResolver};

use crate::allow_files::AllowFiles;
use crate::cfg::AttrCfg;
use crate::Attr;

use crate::globals::AttrSpecArgs;

/// The underlying data type of a target attribute (e.g. Bool, String, LabelList).
#[derive(Debug, Clone, PartialEq, Eq, Allocative)]
pub enum AttrKind {
    Bool,
    // Allowed is typically *very* small, so we use a Vec.
    Int { allowed: Option<Vec<i32>> },
    IntList,
    Label,
    LabelKeyedStringDict,
    LabelList,
    LabelListDict,
    // Allowed is typically *very* small, so we use a Vec.
    String { allowed: Option<Vec<String>> },
    StringDict,
    StringKeyedLabelDict,
    StringList,
    StringListDict,
}

/// Schema specifying what files (single or multiple) are allowed on a label-like attribute.
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
    pub(crate) kind: AttrKind,
    pub(crate) default: Option<Attr>,
    pub(crate) disallow_empty: bool,
    pub(crate) allow_files: AllowFilesSchema,
    pub(crate) cfg: AttrCfg,
    pub(crate) doc: String,
}

starlark_simple_value!(AttrSchema);

impl Freeze for AttrSchema {
    type Frozen = Self;

    fn freeze(self, _freezer: &Freezer) -> FreezeResult<Self::Frozen> {
        Ok(self)
    }
}

impl Display for AttrSchema {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

#[starlark_value(type = "AttrSchema")]
// Clippy recommends eliding the 'v lifetime, but the #[starlark_value] macro
// requires it to be explicitly declared.
#[allow(clippy::elidable_lifetime_names)]
impl<'v> StarlarkValue<'v> for AttrSchema {
    fn collect_repr(&self, collector: &mut String) {
        use std::fmt::Write as _;
        write!(collector, "{self:?}").unwrap();
    }
}

impl AttrSchema {
    /// Creates an `AttrSchema` from validation attributes and registers it.
    pub fn create<'v>(
        kind: AttrKind,
        args: AttrSpecArgs<'v>,
        package: &PackageRef,
        path_resolver: &PathResolver,
        heap: &Heap<'v>,
    ) -> starlark::Result<Value<'v>> {
        let mut schema = Self {
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
                _ => return Err(crate::Error::AllowFilesMutuallyExclusive.into()),
            },
            cfg: args.cfg.unwrap_or(AttrCfg::CurrentToolchain),
            doc: match args.doc {
                None | Some(NoneOr::None) => String::new(),
                Some(NoneOr::Other(s)) => s,
            },
        };

        let mandatory = args.mandatory.unwrap_or(false);
        if schema.disallow_empty && !mandatory && args.default.is_none() {
            return Err(crate::Error::AllowEmptyRequiresMandatoryOrDefault.into());
        }

        schema.default = match (mandatory, args.default) {
            (true, None) => None,
            (false, None) => Some(match &schema.kind {
                AttrKind::Bool => Attr::Bool(false),
                AttrKind::Int { .. } => Attr::Int(0),
                AttrKind::String { .. } => Attr::String(Default::default()),
                AttrKind::IntList => Attr::IntList(Default::default()),
                AttrKind::StringList => Attr::StringList(Default::default()),
                AttrKind::LabelList => Attr::LabelList(Default::default()),
                AttrKind::StringListDict => Attr::StringListDict(Default::default()),
                AttrKind::StringDict => Attr::StringDict(Default::default()),
                AttrKind::LabelKeyedStringDict => Attr::LabelKeyedStringDict(Default::default()),
                AttrKind::StringKeyedLabelDict => Attr::StringKeyedLabelDict(Default::default()),
                AttrKind::LabelListDict => Attr::LabelListDict(Default::default()),
                AttrKind::Label => Attr::None,
            }),
            (false, Some(v)) => {
                // We provide an empty param name because you can write the following code:
                // p = attr.string(default = 1)
                // rule(..., attrs = {"foo": p})
                // This means that at the time you call attr.string, no parameter name has
                // been set.
                Some(Attr::create_without_defaults(
                    "<unnamed>",
                    &schema,
                    v,
                    package,
                    path_resolver,
                )?)
            }
            (true, Some(_)) => {
                return Err(crate::Error::MandatoryAndDefaultMutuallyExclusive.into());
            }
        };

        Ok(heap.alloc(schema))
    }

    /// Returns the allowed files schema for this attribute.
    pub fn allow_files(&self) -> &AllowFilesSchema {
        &self.allow_files
    }

    /// Returns the default value of this attribute, if any.
    pub fn default(&self) -> Option<&Attr> {
        self.default.as_ref()
    }

    /// Returns the file matcher if this attribute schema allows files.
    pub fn file_matcher(&self) -> Option<&AllowFiles> {
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
    use crate::globals::tests::new_attr_assert;
    use crate::LabelOrFile;
    use types::Label;

    #[track_caller]
    fn assert_eq_schema(
        a: &mut starlark::assert::Assert<'static>,
        code: &str,
        expected: &AttrSchema,
    ) {
        let val = a.pass(code);
        let unpacked: &AttrSchema =
            starlark::values::UnpackValue::unpack_value_err(val.value()).unwrap();
        assert_eq!(unpacked, expected);
    }

    #[test]
    fn test_schema_bool() {
        let mut a = new_attr_assert();

        assert_eq_schema(
            &mut a,
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
        let mut a = new_attr_assert();

        assert_eq_schema(
            &mut a,
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

        a.fail(
            "attr.int(values=[1, 2], default=3)",
            "Value 3 is not in allowed set",
        );
    }

    #[test]
    fn test_schema_string() {
        let mut a = new_attr_assert();

        assert_eq_schema(
            &mut a,
            "attr.string(values=['foo', 'bar'], mandatory=True)",
            &AttrSchema {
                kind: AttrKind::String {
                    allowed: Some(vec!["foo".to_string(), "bar".to_string()]),
                },
                default: None,
                disallow_empty: false,
                allow_files: AllowFilesSchema::None,
                cfg: AttrCfg::CurrentToolchain,
                doc: String::new(),
            },
        );

        a.fail(
            "attr.string(values=['a', 'b'], default='c')",
            "Value \"c\" is not in allowed set",
        );
    }

    #[test]
    fn test_schema_label() {
        let mut a = new_attr_assert();

        assert_eq_schema(
            &mut a,
            "attr.label(allow_files=True, default=':foo')",
            &AttrSchema {
                kind: AttrKind::Label,
                default: Some(Attr::Label(LabelOrFile::Label(Label::new(
                    PackageRef::root().to_owned(),
                    "foo".to_owned(),
                )))),
                disallow_empty: false,
                allow_files: AllowFilesSchema::Many(AllowFiles::All),
                cfg: AttrCfg::CurrentToolchain,
                doc: String::new(),
            },
        );
    }

    #[test]
    fn test_schema_string_list() {
        let mut a = new_attr_assert();

        assert_eq_schema(
            &mut a,
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

        a.fail(
            "attr.string_list(allow_empty=False)",
            "allow_empty = False requires the attribute to be mandatory or have a non-empty default value",
        );
        a.fail(
            "attr.string_list(allow_empty=False, default=[])",
            "Want non-empty list, got []",
        );
    }

    #[test]
    fn test_schema_string_default() {
        let mut a = new_attr_assert();

        assert_eq_schema(
            &mut a,
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
    fn test_schema_string_dict() {
        let a = new_attr_assert();

        a.fail(
            "attr.string_dict(allow_empty=False, default={})",
            "Want non-empty dict, got {}",
        );
    }

    #[test]
    fn test_schema_allow_files_error() {
        let a = new_attr_assert();

        a.fail(
            "attr.label(allow_files=True, allow_single_file=True)",
            "allow_files and allow_single_file are mutually exclusive",
        );
    }

    #[test]
    fn test_schema_mandatory_default_error() {
        let a = new_attr_assert();

        a.fail(
            "attr.bool(mandatory=True, default=True)",
            "mandatory and default are mutually exclusive",
        );
    }
}
