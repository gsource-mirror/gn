// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use allocative::Allocative;
use starlark::collections::SmallMap;
use starlark::values::list::UnpackList;
use starlark::values::none::NoneOr;
use starlark::values::Freeze;
use starlark::values::FreezeResult;
use starlark::values::Freezer;
use starlark::values::Heap;
use starlark::values::UnpackValue as _;
use starlark::values::Value;
use std::fmt;
use types::{File, Label, LabelRef, PackageRef, PathResolver};

use crate::allow_files::AllowFiles;
use crate::schema::{AllowFilesSchema, AttrKind, AttrSchema};

/// A Starlark value that can be either a target `Label` or a source `File`.
///
/// We do this because at the time we resolve attributes, the dependency has
/// not yet been resolved, and thus we don't know what files it expands to.
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

/// Represents an actual attribute passed to a target.
///
/// For example, sources = ["foo.cc"] => Attr::LabelList(vec![file for foo.cc])
///
/// Guaranteed to match the type of the corresponding `AttrSchema`'s `AttrKind`.
/// For example, the bool type defaults to false, attr.bool() can never produce `Attr::Label(None)`.
/// On the other hand, `attr.label()` defaults to `Attr::Label(None)`.
#[derive(Clone, Debug, PartialEq, Eq, Allocative)]
pub enum Attr {
    Bool(bool),
    Int(i32),
    String(String),
    IntList(Vec<i32>),
    StringList(Vec<String>),
    StringListDict(SmallMap<String, Vec<String>>),
    Label(Option<LabelOrFile>),
    LabelList(Vec<LabelOrFile>),
    StringDict(SmallMap<String, String>),
    LabelKeyedStringDict(SmallMap<LabelOrFile, String>),
    StringKeyedLabelDict(SmallMap<String, LabelOrFile>),
    LabelListDict(SmallMap<String, Vec<LabelOrFile>>),
}

impl std::fmt::Display for Attr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Freeze for Attr {
    type Frozen = Self;

    fn freeze(self, _freezer: &Freezer) -> FreezeResult<Self::Frozen> {
        Ok(self)
    }
}

impl Attr {
    /// Creates an `Attr` value from an input Starlark `Value`, applying default values from the schema if not provided.
    pub fn create(
        param_name: &str,
        schema: &AttrSchema,
        // Distinguish between an explicit = None and an implicit value not provided.
        value: Option<Value<'_>>,
        package: &PackageRef,
        path_resolver: &PathResolver,
    ) -> starlark::Result<Self> {
        if let Some(value) = value {
            Self::create_without_defaults(param_name, schema, value, package, path_resolver)
        } else if let Some(default) = schema.default.as_ref() {
            // We already validated the default during attr schema creation.
            Ok(default.clone())
        } else {
            Err(starlark::Error::from(crate::Error::MandatoryAttribute {
                param: param_name.to_owned(),
            }))
        }
    }

    /// Coerces a Starlark `Value` into an `Attr` based on the validation rules in the `AttrSchema`.
    pub fn create_without_defaults(
        param_name: &str,
        schema: &AttrSchema,
        value: Value<'_>,
        package: &PackageRef,
        path_resolver: &PathResolver,
    ) -> starlark::Result<Self> {
        let parse_label = |s: &str| -> starlark::Result<LabelOrFile> {
            crate::allow_files::parse_label_like(
                s,
                schema.file_matcher().unwrap_or(&AllowFiles::None),
                package,
                path_resolver,
            )
        };

        match &schema.kind {
            AttrKind::Bool => Ok(Self::Bool(bool::unpack_named_param(value, param_name)?)),
            AttrKind::Int { allowed } => {
                let v = i32::unpack_named_param(value, param_name)?;
                if allowed.as_ref().is_some_and(|a| !a.contains(&v)) {
                    return Err(crate::Error::IntNotAllowed(v).into());
                }
                Ok(Self::Int(v))
            }
            AttrKind::String { allowed } => {
                let v = <String>::unpack_named_param(value, param_name)?;
                if allowed.as_ref().is_some_and(|a| !a.contains(&v)) {
                    return Err(crate::Error::StringNotAllowed(v).into());
                }
                Ok(Self::String(v))
            }
            AttrKind::IntList => {
                let list = UnpackList::<i32>::unpack_named_param(value, param_name)?;
                if schema.disallow_empty && list.items.is_empty() {
                    return Err(crate::Error::EmptyListDisallowed.into());
                }
                Ok(Self::IntList(list.items))
            }
            AttrKind::StringList => {
                let list = UnpackList::<String>::unpack_named_param(value, param_name)?;
                if schema.disallow_empty && list.items.is_empty() {
                    return Err(crate::Error::EmptyListDisallowed.into());
                }
                Ok(Self::StringList(list.items))
            }
            AttrKind::StringListDict => {
                let dict =
                    SmallMap::<&str, UnpackList<String>>::unpack_named_param(value, param_name)?;
                if schema.disallow_empty && dict.is_empty() {
                    return Err(crate::Error::EmptyDictDisallowed.into());
                }
                Ok(Self::StringListDict(
                    dict.into_iter()
                        .map(|(k, v)| (k.to_string(), v.items))
                        .collect(),
                ))
            }
            AttrKind::Label => match NoneOr::<&str>::unpack_named_param(value, param_name)? {
                // An explicit None is disallowed if the attribute is mandatory.
                NoneOr::None => {
                    if schema.default.is_none() {
                        Err(crate::Error::MandatoryAttribute {
                            param: param_name.to_owned(),
                        }
                        .into())
                    } else {
                        Ok(Self::Label(None))
                    }
                }
                NoneOr::Other(s) => Ok(Self::Label(Some(parse_label(s)?))),
            },
            AttrKind::LabelList => {
                let list = UnpackList::<&str>::unpack_named_param(value, param_name)?;
                if schema.disallow_empty && list.items.is_empty() {
                    return Err(crate::Error::EmptyListDisallowed.into());
                }
                Ok(Self::LabelList(
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
                Ok(Self::StringDict(dict))
            }
            AttrKind::LabelKeyedStringDict => {
                let dict = SmallMap::<&str, &str>::unpack_named_param(value, param_name)?;
                if schema.disallow_empty && dict.is_empty() {
                    return Err(crate::Error::EmptyDictDisallowed.into());
                }
                Ok(Self::LabelKeyedStringDict(
                    dict.into_iter()
                        .map(|(k, v)| Ok((parse_label(k)?, v.to_string())))
                        .collect::<starlark::Result<SmallMap<_, _>>>()?,
                ))
            }
            AttrKind::StringKeyedLabelDict => {
                let dict = SmallMap::<&str, &str>::unpack_named_param(value, param_name)?;
                if schema.disallow_empty && dict.is_empty() {
                    return Err(crate::Error::EmptyDictDisallowed.into());
                }
                Ok(Self::StringKeyedLabelDict(
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
                Ok(Self::LabelListDict(
                    dict.into_iter()
                        .map(|(k, v)| {
                            Ok((
                                k.to_string(),
                                v.items
                                    .iter()
                                    .map(|s| parse_label(s))
                                    .collect::<starlark::Result<Vec<_>>>()?,
                            ))
                        })
                        .collect::<starlark::Result<SmallMap<_, _>>>()?,
                ))
            }
        }
    }

    /// Registers target dependencies contained within this attribute value.
    pub fn register_dependencies<S: crate::Session>(
        &self,
        session: &S,
        source: S::TargetRef,
        toolchain: LabelRef<'_>,
    ) {
        match self {
            Self::Label(Some(LabelOrFile::Label(lbl))) => {
                session.register_dependency(source, lbl.as_ref(), toolchain);
            }
            Self::LabelList(list) => {
                for lf in list {
                    if let LabelOrFile::Label(lbl) = lf {
                        session.register_dependency(source.clone(), lbl.as_ref(), toolchain);
                    }
                }
            }
            Self::LabelKeyedStringDict(dict) => {
                for (lf, _) in dict {
                    if let LabelOrFile::Label(lbl) = lf {
                        session.register_dependency(source.clone(), lbl.as_ref(), toolchain);
                    }
                }
            }
            Self::StringKeyedLabelDict(dict) => {
                for (_, lf) in dict {
                    if let LabelOrFile::Label(lbl) = lf {
                        session.register_dependency(source.clone(), lbl.as_ref(), toolchain);
                    }
                }
            }
            Self::LabelListDict(dict) => {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cfg::AttrCfg;
    use starlark::values::FrozenHeap;

    #[test]
    fn test_attr_bool() {
        let pkg = PackageRef::new_for_testing("//foo");
        let path_resolver = types::PathResolver::new_for_testing();
        let heap = FrozenHeap::new();

        let schema = AttrSchema {
            kind: AttrKind::Bool,
            default: Some(Attr::Bool(false)),
            disallow_empty: false,
            allow_files: AllowFilesSchema::None,
            cfg: AttrCfg::CurrentToolchain,
            doc: String::new(),
        };

        // Test explicit true
        assert_eq!(
            Attr::create(
                "my_attr",
                &schema,
                Some(Value::new_frozen(heap.alloc(true))),
                pkg,
                &path_resolver,
            )
            .unwrap(),
            Attr::Bool(true)
        );

        // Test default when value is None
        assert_eq!(
            Attr::create("my_attr", &schema, None, pkg, &path_resolver).unwrap(),
            Attr::Bool(false)
        );

        // Test non-boolean value fails
        assert!(Attr::create(
            "my_attr",
            &schema,
            Some(Value::new_frozen(heap.alloc(42))),
            pkg,
            &path_resolver,
        )
        .is_err());
    }

    #[test]
    fn test_attr_label_no_files() {
        let pkg = PackageRef::new_for_testing("//foo");
        let path_resolver = types::PathResolver::new_for_testing();
        let heap = FrozenHeap::new();

        let schema = AttrSchema {
            kind: AttrKind::Label,
            default: None,
            disallow_empty: false,
            allow_files: AllowFilesSchema::None,
            cfg: AttrCfg::CurrentToolchain,
            doc: String::new(),
        };

        // Test parsing a label ":bar"
        assert_eq!(
            Attr::create(
                "my_attr",
                &schema,
                Some(Value::new_frozen(heap.alloc(":bar"))),
                pkg,
                &path_resolver,
            )
            .unwrap(),
            Attr::Label(Some(LabelOrFile::Label(Label::new(
                PackageRef::new_for_testing("//foo").to_owned(),
                "bar".to_owned(),
            ))))
        );

        // Test that a file string fails because files are not allowed
        assert!(Attr::create(
            "my_attr",
            &schema,
            Some(Value::new_frozen(heap.alloc("file.cc"))),
            pkg,
            &path_resolver,
        )
        .is_err());

        // Test that passing None to a mandatory label fails
        assert!(Attr::create(
            "my_attr",
            &schema,
            Some(Value::new_none()),
            pkg,
            &path_resolver,
        )
        .is_err());
    }

    #[test]
    fn test_attr_label_allow_files() {
        let pkg = PackageRef::new_for_testing("//foo");
        let path_resolver = types::PathResolver::new_for_testing();
        let heap = FrozenHeap::new();

        let schema = AttrSchema {
            kind: AttrKind::Label,
            default: None,
            disallow_empty: false,
            allow_files: AllowFilesSchema::Many(AllowFiles::Some(vec![".cc".to_owned()])),
            cfg: AttrCfg::CurrentToolchain,
            doc: String::new(),
        };

        // Test a valid file "file.cc" (exists in testdata/foo/file.cc)
        assert_eq!(
            Attr::create(
                "my_attr",
                &schema,
                Some(Value::new_frozen(heap.alloc("file.cc"))),
                pkg,
                &path_resolver,
            )
            .unwrap(),
            Attr::Label(Some(LabelOrFile::File(
                path_resolver.source_file(pkg, "file.cc").unwrap()
            )))
        );

        // Test an invalid file "file.h" (extension not in allowed list)
        assert!(Attr::create(
            "my_attr",
            &schema,
            Some(Value::new_frozen(heap.alloc("file.h"))),
            pkg,
            &path_resolver,
        )
        .is_err());

        // Test that a label still resolves
        assert_eq!(
            Attr::create(
                "my_attr",
                &schema,
                Some(Value::new_frozen(heap.alloc(":bar"))),
                pkg,
                &path_resolver,
            )
            .unwrap(),
            Attr::Label(Some(LabelOrFile::Label(Label::new(
                PackageRef::new_for_testing("//foo").to_owned(),
                "bar".to_owned(),
            ))))
        );
    }
}
