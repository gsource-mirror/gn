// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::fmt;
use allocative::Allocative;
use starlark::collections::SmallMap;
use starlark::values::list::UnpackList;
use starlark::values::none::NoneOr;
use starlark::values::Freeze;
use starlark::values::FreezeResult;
use starlark::values::Freezer;
use starlark::values::Heap;
use starlark::values::ProvidesStaticType;
use starlark::values::StarlarkValue;
use starlark::values::UnpackValue;
use starlark::values::Value;
use types::{File, Label, PackageRef, PathResolver, LabelRef};

use crate::allow_files::AllowFiles;
use crate::schema::{AttrSchema, AttrKind, AllowFilesSchema};

/// A Starlark value that can be either a target `Label` or a source `File`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Allocative)]
pub enum LabelOrFile {
    Label(Label),
    File(File),
}

impl LabelOrFile {
    /// Converts this `LabelOrFile` into a Starlark `Value` allocated on the heap.
    pub fn to_value<'v>(&self, heap: &Heap<'v>) -> Value<'v> {
        match self {
            Self::Label(l) => heap.alloc(l.clone()),
            Self::File(f) => heap.alloc(f.clone()),
        }
    }
}

/// Represents an actual parameter passed to a target.
/// Guaranteed to match the corresponding AttrSchema,
/// with the exception of allow_[single_]file, since at the time we resolve the
/// parameter, we haven't yet resolved the label to a Target, so don't know
/// what files it provides.
#[derive(
    Clone, Debug, PartialEq, Eq, Allocative, ProvidesStaticType, starlark_derive::NoSerialize,
)]
pub enum Attr {
    None,
    Bool(bool),
    Int(i32),
    String(String),
    IntList(Vec<i32>),
    StringList(Vec<String>),
    StringListDict(SmallMap<String, Vec<String>>),
    Label(LabelOrFile),
    LabelList(Vec<LabelOrFile>),
    StringDict(SmallMap<String, String>),
    LabelKeyedStringDict(SmallMap<LabelOrFile, String>),
    StringKeyedLabelDict(SmallMap<String, LabelOrFile>),
    LabelListDict(SmallMap<String, Vec<LabelOrFile>>),
}

impl std::fmt::Display for Attr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Freeze for Attr {
    type Frozen = Attr;

    fn freeze(self, _freezer: &Freezer) -> FreezeResult<Self::Frozen> {
        Ok(self)
    }
}

impl Attr {
    /// Creates an `Attr` value from an input Starlark `Value`, applying default values from the schema if not provided.
    pub fn create<'v>(
        param_name: &str,
        schema: &AttrSchema,
        // Distinguish between an explicit = None and an implicit value not provided.
        value: Option<NoneOr<Value<'v>>>,
        package: &PackageRef,
        path_resolver: &PathResolver,
    ) -> starlark::Result<Self> {
        match value {
            Some(NoneOr::Other(v)) => Self::create_without_defaults(
                param_name,
                schema,
                v,
                package,
                path_resolver,
            ),
            Some(NoneOr::None) => Self::create_without_defaults(
                param_name,
                schema,
                Value::new_none(),
                package,
                path_resolver,
            ),
            None => match &schema.default {
                None => Err(starlark::Error::from(crate::Error::MandatoryAttribute {
                    param: param_name.to_owned(),
                })),
                Some(default) => Ok(default.clone()),
            },
        }
    }

    /// Coerces a Starlark `Value` into an `Attr` based on the validation rules in the `AttrSchema`.
    pub fn create_without_defaults<'v>(
        param_name: &str,
        schema: &AttrSchema,
        value: Value<'v>,
        package: &PackageRef,
        path_resolver: &PathResolver,
    ) -> starlark::Result<Self> {
        let parse_label = |s: &str| -> starlark::Result<LabelOrFile> {
            let allowed = match &schema.allow_files {
                AllowFilesSchema::None => &AllowFiles::None,
                AllowFilesSchema::Single(a) | AllowFilesSchema::Many(a) => a,
            };
            Ok(crate::allow_files::parse_label_like(s, allowed, package, path_resolver)?)
        };

        match &schema.kind {
            AttrKind::Bool => {
                let b = bool::unpack_named_param(value, param_name)?;
                Ok(Attr::Bool(b))
            }
            AttrKind::Int { allowed } => {
                let v = i32::unpack_named_param(value, param_name)?;
                if allowed.as_ref().is_some_and(|a| !a.contains(&v)) {
                    return Err(crate::Error::IntNotAllowed(v).into());
                }
                Ok(Attr::Int(v))
            }
            AttrKind::String { allowed } => {
                let v = <String>::unpack_named_param(value, param_name)?;
                if allowed.as_ref().is_some_and(|a| !a.contains(v.as_str())) {
                    return Err(crate::Error::StringNotAllowed(v).into());
                }
                Ok(Attr::String(v))
            }
            AttrKind::IntList => {
                let list = UnpackList::<i32>::unpack_named_param(value, param_name)?;
                if schema.disallow_empty && list.items.is_empty() {
                    return Err(crate::Error::EmptyListDisallowed.into());
                }
                Ok(Attr::IntList(list.items))
            }
            AttrKind::StringList => {
                let list = UnpackList::<String>::unpack_named_param(value, param_name)?;
                if schema.disallow_empty && list.items.is_empty() {
                    return Err(crate::Error::EmptyListDisallowed.into());
                }
                Ok(Attr::StringList(list.items))
            }
            AttrKind::StringListDict => {
                let dict =
                    SmallMap::<&str, UnpackList<String>>::unpack_named_param(value, param_name)?;
                if schema.disallow_empty && dict.is_empty() {
                    return Err(crate::Error::EmptyDictDisallowed.into());
                }
                let res = dict
                    .into_iter()
                    .map(|(k, v)| (k.to_string(), v.items))
                    .collect();
                Ok(Attr::StringListDict(res))
            }
            AttrKind::Label => match NoneOr::<&str>::unpack_named_param(value, param_name)? {
                NoneOr::None => Ok(Attr::None),
                NoneOr::Other(s) => {
                    let lf = parse_label(&s)?;
                    Ok(Attr::Label(lf))
                }
            },
            AttrKind::LabelList => {
                let list = UnpackList::<&str>::unpack_named_param(value, param_name)?;
                if schema.disallow_empty && list.items.is_empty() {
                    return Err(crate::Error::EmptyListDisallowed.into());
                }
                Ok(Attr::LabelList(
                    list.items
                        .iter()
                        .map(|s| parse_label(s))
                        .collect::<starlark::Result<Vec<_>>>()?,
                ))
            }
            AttrKind::StringDict => {
                let dict = SmallMap::<String, String>::unpack_named_param(value, param_name)?;
                if schema.disallow_empty && dict.is_empty() {
                    return Err(crate::Error::EmptyDictDisallowed.into());
                }
                Ok(Attr::StringDict(dict))
            }
            AttrKind::LabelKeyedStringDict => {
                let dict = SmallMap::<&str, &str>::unpack_named_param(value, param_name)?;
                if schema.disallow_empty && dict.is_empty() {
                    return Err(crate::Error::EmptyDictDisallowed.into());
                }
                Ok(Attr::LabelKeyedStringDict(
                    dict.into_iter()
                        .map(|(k, v)| {
                            let key = parse_label(k)?;
                            Ok((key, v.to_string()))
                        })
                        .collect::<starlark::Result<SmallMap<_, _>>>()?,
                ))
            }
            AttrKind::StringKeyedLabelDict => {
                let dict = SmallMap::<&str, &str>::unpack_named_param(value, param_name)?;
                if schema.disallow_empty && dict.is_empty() {
                    return Err(crate::Error::EmptyDictDisallowed.into());
                }
                Ok(Attr::StringKeyedLabelDict(
                    dict.into_iter()
                        .map(|(k, v)| Ok((k.to_string(), parse_label(v)?)))
                        .collect::<starlark::Result<SmallMap<_, _>>>()?,
                ))
            }
            AttrKind::LabelListDict => {
                let dict =
                    SmallMap::<&str, UnpackList<&str>>::unpack_named_param(value, param_name)?;
                if schema.disallow_empty && dict.is_empty() {
                    return Err(crate::Error::EmptyDictDisallowed.into());
                }
                Ok(Attr::LabelListDict(
                    dict.into_iter()
                        .map(|(k, v)| {
                            let label_list = v
                                .items
                                .iter()
                                .map(|s| parse_label(s))
                                .collect::<starlark::Result<Vec<_>>>();
                            Ok((k.to_string(), label_list?))
                        })
                        .collect::<starlark::Result<SmallMap<_, _>>>()?,
                ))
            }
        }
    }

    /// Recursively registers target dependencies contained within this attribute value.
    pub fn register_dependencies<S: crate::Session>(
        &self,
        session: &S,
        source: S::TargetRef,
        toolchain: LabelRef<'_>,
    ) {
        match self {
            Attr::Label(LabelOrFile::Label(lbl)) => {
                session.register_dependency(source, lbl.as_ref(), toolchain);
            }
            Attr::LabelList(list) => {
                for lf in list {
                    if let LabelOrFile::Label(lbl) = lf {
                        session.register_dependency(source.clone(), lbl.as_ref(), toolchain);
                    }
                }
            }
            Attr::LabelKeyedStringDict(dict) => {
                for (lf, _) in dict {
                    if let LabelOrFile::Label(lbl) = lf {
                        session.register_dependency(source.clone(), lbl.as_ref(), toolchain);
                    }
                }
            }
            Attr::StringKeyedLabelDict(dict) => {
                for (_, lf) in dict {
                    if let LabelOrFile::Label(lbl) = lf {
                        session.register_dependency(source.clone(), lbl.as_ref(), toolchain);
                    }
                }
            }
            Attr::LabelListDict(dict) => {
                for (_, list) in dict {
                    for lf in list {
                        if let LabelOrFile::Label(lbl) = lf {
                            session.register_dependency(source.clone(), lbl.as_ref(), toolchain);
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

// Allow Attr to be returned by starlark functions during testing.
starlark::starlark_simple_value!(Attr);

#[starlark_derive::starlark_value(type = "Attr")]
impl<'v> StarlarkValue<'v> for Attr {}


