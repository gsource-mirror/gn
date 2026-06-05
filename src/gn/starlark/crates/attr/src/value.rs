// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use allocative::Allocative;
use starlark::{
    collections::{SmallMap, SmallSet},
    values::{dict::Dict, Heap, Value},
};
use types::{File, LabelRef, TargetRef as _};

use crate::{schema::AttrSchema, Attr, Session};

/// AttrValue is the value that will appear in ctx.(attr|file|files).foo
#[derive(Clone, Debug, Allocative)]
pub struct AttrValue<'v> {
    /// A single resolved `File` value.
    /// Only present if allow_single_file is True.
    /// If allow_single_file is true, is either a file or the *starlark* none.
    pub file: Option<Value<'v>>,
    /// A list of resolved `File` values representing all files resolved for
    /// this attribute. Only present if allow_single_file is True, or
    /// allow_files is True.
    pub files: Option<Value<'v>>,
    /// The coerced value of the attribute itself.
    pub attr: Value<'v>,
}

impl Attr {
    /// Resolves the attribute value against the session to collect any provided
    /// files and outputs an `AttrValue`.
    pub fn to_value<'v, S: Session>(
        &self,
        schema: &AttrSchema,
        session: &S,
        current_toolchain: &LabelRef,
        heap: &Heap<'v>,
    ) -> starlark::Result<AttrValue<'v>> {
        let mut unique_files = SmallSet::new();
        let attr =
            self.resolve_and_collect(schema, session, current_toolchain, heap, &mut unique_files)?;

        let files_list: Vec<Value<'v>> = unique_files.into_iter().map(|f| heap.alloc(f)).collect();

        Ok(AttrValue {
            attr,
            file: match &schema.allow_files {
                crate::AllowFilesSchema::Single(_) => {
                    if files_list.len() == 1 {
                        Some(files_list[0])
                    } else if files_list.is_empty() {
                        Some(Value::new_none())
                    } else {
                        let label = match self {
                            Self::Label(Some(crate::LabelOrFile::Label(lbl))) => lbl.clone(),
                            _ => unreachable!(
                                "files_list.len() > 1 is only possible for Label attributes"
                            ),
                        };
                        return Err(starlark::Error::new_other(
                            crate::Error::MustProduceSingleFile(label),
                        ));
                    }
                },
                _ => None,
            },
            files: if schema.file_matcher().is_some() {
                Some(heap.alloc(files_list))
            } else {
                None
            },
        })
    }

    fn resolve_and_collect<'v, S: Session>(
        &self,
        schema: &AttrSchema,
        session: &S,
        current_toolchain: &LabelRef,
        heap: &Heap<'v>,
        files: &mut SmallSet<File>,
    ) -> starlark::Result<Value<'v>> {
        match self {
            Self::Bool(b) => Ok(Value::new_bool(*b)),
            Self::Int(i) => Ok(heap.alloc(*i)),
            Self::String(s) => Ok(heap.alloc(s.as_str())),
            Self::IntList(l) => Ok(heap.alloc(l.clone())),
            Self::StringList(l) => Ok(heap.alloc(l.clone())),
            Self::StringListDict(d) => {
                let mut res = SmallMap::with_capacity(d.len());
                for (k, v) in d {
                    res.insert_hashed(
                        heap.alloc(k.as_str()).get_hashed().unwrap(),
                        heap.alloc(v.clone()),
                    );
                }
                Ok(heap.alloc(Dict::new(res)))
            },
            Self::Label(None) => Ok(Value::new_none()),
            Self::Label(Some(lf)) => {
                Self::resolve_label_or_file(lf, schema, session, current_toolchain, heap, files)
            },
            Self::LabelList(l) => {
                let resolved_list = l
                    .iter()
                    .map(|lf| {
                        Self::resolve_label_or_file(
                            lf,
                            schema,
                            session,
                            current_toolchain,
                            heap,
                            files,
                        )
                    })
                    .collect::<starlark::Result<Vec<_>>>()?;
                Ok(heap.alloc(resolved_list))
            },
            Self::StringDict(d) => {
                let mut res = SmallMap::with_capacity(d.len());
                for (k, v) in d {
                    res.insert_hashed(
                        heap.alloc(k.as_str()).get_hashed().unwrap(),
                        heap.alloc(v.as_str()),
                    );
                }
                Ok(heap.alloc(Dict::new(res)))
            },
            Self::LabelKeyedStringDict(d) => {
                let mut res = SmallMap::with_capacity(d.len());
                for (k, v) in d {
                    res.insert_hashed(
                        Self::resolve_label_or_file(
                            k,
                            schema,
                            session,
                            current_toolchain,
                            heap,
                            files,
                        )?
                        .get_hashed()?,
                        heap.alloc(v.as_str()),
                    );
                }
                Ok(heap.alloc(Dict::new(res)))
            },
            Self::StringKeyedLabelDict(d) => {
                let mut res = SmallMap::with_capacity(d.len());
                for (k, v) in d {
                    res.insert_hashed(
                        heap.alloc(k.as_str()).get_hashed().unwrap(),
                        Self::resolve_label_or_file(
                            v,
                            schema,
                            session,
                            current_toolchain,
                            heap,
                            files,
                        )?,
                    );
                }
                Ok(heap.alloc(Dict::new(res)))
            },
            Self::LabelListDict(d) => {
                let mut res = SmallMap::with_capacity(d.len());
                for (k, v) in d {
                    let resolved_list = v
                        .iter()
                        .map(|lf| {
                            Self::resolve_label_or_file(
                                lf,
                                schema,
                                session,
                                current_toolchain,
                                heap,
                                files,
                            )
                        })
                        .collect::<starlark::Result<Vec<_>>>()?;
                    res.insert_hashed(
                        heap.alloc(k.as_str()).get_hashed().unwrap(),
                        heap.alloc(resolved_list),
                    );
                }
                Ok(heap.alloc(Dict::new(res)))
            },
        }
    }

    /// Resolves a label or file object:
    /// * If it's a label, resolves it to a target.
    ///   * If allow_single_file / allow_files is set, DefaultInfo.files of the
    ///     target is expanded into files.
    /// * If it's a file, returns itself and collects the file to files.
    fn resolve_label_or_file<'v, S: Session>(
        lf: &crate::LabelOrFile,
        schema: &AttrSchema,
        session: &S,
        current_toolchain: &LabelRef,
        heap: &Heap<'v>,
        files: &mut SmallSet<File>,
    ) -> starlark::Result<Value<'v>> {
        match lf {
            crate::LabelOrFile::Label(lbl) => {
                let target = session.get_target(lbl.as_ref(), *current_toolchain);
                if let Some(matcher) = schema.file_matcher() {
                    for f in target.outputs() {
                        matcher.validate(f.as_str())?;
                        files.insert(f.clone());
                    }
                }
                Ok(heap.alloc(target))
            },
            crate::LabelOrFile::File(f) => {
                if let Some(matcher) = schema.file_matcher() {
                    matcher.validate(f.as_str())?;
                    files.insert(f.clone());
                }
                Ok(heap.alloc(f.clone()))
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use starlark::{
        environment::Module,
        values::{list::UnpackList, UnpackValue as _, ValueLike as _},
    };
    use testutils::{FakeSession, FakeTarget, FakeTargetRef};
    use types::{Label, PackageRef};

    use super::*;
    use crate::{
        allow_files::AllowFiles,
        cfg::AttrCfg,
        schema::{AllowFilesSchema, AttrKind},
    };

    #[test]
    fn test_to_value_basic() {
        let session = FakeSession::new();
        let schema = AttrSchema {
            kind: AttrKind::Bool,
            default: None,
            disallow_empty: false,
            allow_files: AllowFilesSchema::None,
            cfg: AttrCfg::CurrentToolchain,
            doc: String::new(),
        };

        Module::with_temp_heap(|module| {
            let heap = module.heap();
            let AttrValue { attr, file, files } = Attr::Bool(true)
                .to_value(
                    &schema,
                    &session,
                    &session.default_toolchain.as_ref(),
                    &heap,
                )
                .unwrap();

            assert!(attr.unpack_bool().unwrap());
            assert!(file.is_none());
            assert!(files.is_none());
        });
    }

    #[test]
    fn test_to_value_label_no_files() {
        let session = FakeSession::new();
        let target_label = Label::new(
            PackageRef::new("//foo").unwrap().to_owned(),
            "bar".to_owned(),
        );

        let schema = AttrSchema {
            kind: AttrKind::Label,
            default: None,
            disallow_empty: false,
            allow_files: AllowFilesSchema::None,
            cfg: AttrCfg::CurrentToolchain,
            doc: String::new(),
        };

        Module::with_temp_heap(|module| {
            let heap = module.heap();
            let AttrValue { attr, file, files } =
                Attr::Label(Some(crate::LabelOrFile::Label(target_label.clone())))
                    .to_value(
                        &schema,
                        &session,
                        &session.default_toolchain.as_ref(),
                        &heap,
                    )
                    .unwrap();

            // The resolved value should be the Target object
            let resolved_target = attr.downcast_ref::<FakeTargetRef>().unwrap();
            assert!(resolved_target.registered_deps().is_empty());
            assert!(resolved_target.outputs().is_empty());
            assert!(file.is_none());
            assert!(files.is_none());
        });
    }

    #[test]
    fn test_to_value_label_allow_files_many() {
        let session = FakeSession::new();
        let target_label = Label::new(
            PackageRef::new("//foo").unwrap().to_owned(),
            "bar".to_owned(),
        );
        let label_only_file = File::new("label_only.cc");
        let overlap = File::new("overlap.cc");
        let file_only_file = File::new("file_only.cc");

        // Target outputs out.cc and overlap.h
        let dep = FakeTargetRef::new(FakeTarget {
            outputs: vec![label_only_file.clone(), overlap.clone()],
            ..Default::default()
        });
        session.insert_target(target_label.clone(), dep.clone());

        let schema = AttrSchema {
            kind: AttrKind::LabelList,
            default: None,
            disallow_empty: false,
            allow_files: AllowFilesSchema::Many(AllowFiles::All),
            cfg: AttrCfg::CurrentToolchain,
            doc: String::new(),
        };

        Module::with_temp_heap(|module| {
            let heap = module.heap();
            let AttrValue { attr, file, files } = Attr::LabelList(vec![
                crate::LabelOrFile::Label(target_label.clone()),
                crate::LabelOrFile::File(file_only_file.clone()),
                crate::LabelOrFile::File(overlap.clone()),
            ])
            .to_value(
                &schema,
                &session,
                &session.default_toolchain.as_ref(),
                &heap,
            )
            .unwrap();

            let attr_list = UnpackList::<Value>::unpack_value_err(attr).unwrap().items;
            assert_eq!(attr_list.len(), 3);
            assert_eq!(
                <&FakeTargetRef>::unpack_value_err(attr_list[0]).unwrap(),
                &dep
            );
            assert_eq!(
                <&File>::unpack_value_err(attr_list[1]).unwrap(),
                &file_only_file
            );
            assert_eq!(<&File>::unpack_value_err(attr_list[2]).unwrap(), &overlap);

            assert!(file.is_none());
            // Verifying files resolved correctly: target outputs + the direct file (with
            // overlap deduplicated!)
            assert_eq!(
                UnpackList::<&File>::unpack_value_err(files.unwrap())
                    .unwrap()
                    .items,
                vec![&label_only_file, &overlap, &file_only_file]
            );
        });
    }

    #[test]
    fn test_to_value_label_allow_files_single() {
        let session = FakeSession::new();
        let target_label = Label::new(
            PackageRef::new("//foo").unwrap().to_owned(),
            "bar".to_owned(),
        );
        let file1 = File::new("out.cc");

        let schema = AttrSchema {
            kind: AttrKind::Label,
            default: None,
            disallow_empty: false,
            allow_files: AllowFilesSchema::Single(AllowFiles::All),
            cfg: AttrCfg::CurrentToolchain,
            doc: String::new(),
        };

        Module::with_temp_heap(|module| {
            let heap = module.heap();

            // Case 1: Target has exactly 1 output file -> succeeds
            session.insert_target(
                target_label.clone(),
                FakeTargetRef::new(FakeTarget {
                    outputs: vec![file1.clone()],
                    ..Default::default()
                }),
            );

            let path_resolver = types::PathResolver::new_for_testing();
            let starlark_val = heap.alloc(":bar");
            let attr = Attr::create(
                &schema,
                Some(starlark_val),
                PackageRef::new("//foo").unwrap(),
                &path_resolver,
            )
            .unwrap();

            let source_target = FakeTargetRef::default();
            attr.register_dependencies(
                &session,
                source_target.clone(),
                session.default_toolchain.as_ref(),
            );

            assert_eq!(
                source_target.registered_deps(),
                HashSet::from([(target_label.clone(), session.default_toolchain.clone())])
            );

            let AttrValue {
                file,
                files,
                attr: resolved_attr,
            } = attr
                .to_value(
                    &schema,
                    &session,
                    &session.default_toolchain.as_ref(),
                    &heap,
                )
                .unwrap();

            let file_val = file.unwrap();
            let single_file = file_val.downcast_ref::<File>().unwrap();
            assert_eq!(single_file, &file1);

            assert_eq!(
                UnpackList::<&File>::unpack_value_err(files.unwrap())
                    .unwrap()
                    .items,
                vec![&file1]
            );

            let resolved_target = resolved_attr.downcast_ref::<FakeTargetRef>().unwrap();
            assert_eq!(resolved_target.outputs(), vec![file1.clone()]);

            // Case 2: Direct File -> succeeds
            let AttrValue {
                file,
                files,
                attr: _,
            } = Attr::Label(Some(crate::LabelOrFile::File(file1.clone())))
                .to_value(
                    &schema,
                    &session,
                    &session.default_toolchain.as_ref(),
                    &heap,
                )
                .unwrap();

            let file_val = file.unwrap();
            let single_file = file_val.downcast_ref::<File>().unwrap();
            assert_eq!(single_file, &file1);

            assert_eq!(
                UnpackList::<&File>::unpack_value_err(files.unwrap())
                    .unwrap()
                    .items,
                vec![&file1]
            );

            // Case 3: Target has 2 outputs -> fails
            session.insert_target(
                target_label.clone(),
                FakeTargetRef::new(FakeTarget {
                    outputs: vec![file1.clone(), File::new("out.h")],
                    attrs: vec![],
                    ..Default::default()
                }),
            );

            let res = Attr::Label(Some(crate::LabelOrFile::Label(target_label.clone()))).to_value(
                &schema,
                &session,
                &session.default_toolchain.as_ref(),
                &heap,
            );
            assert_eq!(
                res.unwrap_err().to_string(),
                "target `//foo:bar` must produce a single output file"
            );
        });
    }
}
