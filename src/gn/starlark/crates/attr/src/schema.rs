// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;

use allocative::Allocative;
use starlark::collections::SmallMap;
use starlark::collections::SmallSet;
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
    type Frozen = AttrSchema;

    fn freeze(self, _freezer: &Freezer) -> FreezeResult<Self::Frozen> {
        Ok(self)
    }
}

impl Display for AttrSchema {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "AttrSchema(...)")
    }
}

#[starlark_value(type = "AttrSchema")]
impl<'v> StarlarkValue<'v> for AttrSchema {
    fn collect_repr(&self, collector: &mut String) {
        use std::fmt::Write;
        write!(collector, "{:?}", self).unwrap();
    }
}

impl AttrSchema {
    /// Creates a new `AttrSchema` for testing.
    pub fn new_for_testing(
        kind: AttrKind,
        default: Option<crate::Attr>,
        disallow_empty: bool,
        allow_files: AllowFilesSchema,
        cfg: AttrCfg,
        doc: String,
    ) -> Self {
        Self {
            kind,
            default,
            disallow_empty,
            allow_files,
            cfg,
            doc,
        }
    }

    /// Creates an `AttrSchema` from validation attributes and registers it.
    pub fn create<'v>(
        kind: AttrKind,
        args: AttrSpecArgs<'v>,
        package: &PackageRef,
        path_resolver: &PathResolver,
        heap: &Heap<'v>,
    ) -> starlark::Result<Value<'v>> {
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
                    AttrKind::StringListDict => Attr::StringListDict(SmallMap::new()),
                    AttrKind::StringDict => Attr::StringDict(SmallMap::new()),
                    AttrKind::LabelKeyedStringDict => Attr::LabelKeyedStringDict(SmallMap::new()),
                    AttrKind::StringKeyedLabelDict => Attr::StringKeyedLabelDict(SmallMap::new()),
                    AttrKind::LabelListDict => Attr::LabelListDict(SmallMap::new()),
                    AttrKind::Label => Attr::None,
                };
                Some(default_attr)
            }
            (false, Some(v)) => {
                // We provide an empty param name because you can write the following code:
                // p = attr.string(default = 1)
                // rule(..., attrs = {"foo": p})
                // This means that at the time you call attr.string, it is unaware of the parameter name.
                Some(Attr::create_without_defaults(
                    "",
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
        schema.default = default_val;

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


