// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use allocative::Allocative;
use starlark::collections::{SmallMap, SmallSet};
use starlark::values::dict::Dict;
use starlark::values::{Heap, Value};
use types::{File, LabelRef, TargetRef};
use crate::Session;

use crate::schema::AttrSchema;
use crate::Attr;

/// AttrValue is the value that will appear in ctx.(attr|file|files).foo
#[derive(Clone, Debug, Allocative)]
pub struct AttrValue<'v> {
    /// A single resolved `File` value.
    /// Only present if allow_single_file is True.
    /// If allow_single_file is true, is either a file or the *starlark* none.
    pub file: Option<Value<'v>>,
    /// A list of resolved `File` values representing all files resolved for this attribute.
    /// Only present if allow_single_file is True, or allow_files is True.
    pub files: Option<Value<'v>>,
    /// The coerced value of the attribute itself.
    pub attr: Value<'v>,
}

impl Attr {
    /// Resolves the attribute value against the session to collect any provided files and outputs an `AttrValue`.
    pub fn to_value<'v, S: Session>(
        &self,
        schema: &AttrSchema,
        session: &S,
        current_toolchain: &LabelRef,
        target_label: &LabelRef,
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
                        return Err(starlark::Error::new_other(
                            crate::Error::MustProduceSingleFile(target_label.to_string()),
                        ));
                    }
                }
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
            Self::None => Ok(Value::new_none()),
            Self::Bool(b) => Ok(Value::new_bool(*b)),
            Self::Int(i) => Ok(heap.alloc(*i)),
            Self::String(s) => Ok(heap.alloc(s.as_str())),
            Self::IntList(l) => Ok(heap.alloc(l.clone())),
            Self::StringList(l) => Ok(heap.alloc(l.clone())),
            Self::StringListDict(d) => {
                let mut res = SmallMap::with_capacity(d.len());
                for (k, v) in d.iter() {
                    res.insert_hashed(
                        heap.alloc(k.as_str()).get_hashed().unwrap(),
                        heap.alloc(v.clone()),
                    );
                }
                Ok(heap.alloc(Dict::new(res)))
            }
            Self::Label(lf) => {
                Self::resolve_label_or_file(lf, schema, session, current_toolchain, heap, files)
            }
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
            }
            Self::StringDict(d) => {
                let mut res = SmallMap::with_capacity(d.len());
                for (k, v) in d.iter() {
                    res.insert_hashed(
                        heap.alloc(k.as_str()).get_hashed().unwrap(),
                        heap.alloc(v.as_str()),
                    );
                }
                Ok(heap.alloc(Dict::new(res)))
            }
            Self::LabelKeyedStringDict(d) => {
                let mut res = SmallMap::with_capacity(d.len());
                for (k, v) in d.iter() {
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
            }
            Self::StringKeyedLabelDict(d) => {
                let mut res = SmallMap::with_capacity(d.len());
                for (k, v) in d.iter() {
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
            }
            Self::LabelListDict(d) => {
                let mut res = SmallMap::with_capacity(d.len());
                for (k, v) in d.iter() {
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
            }
        }
    }

    /// Resolves a label or file object:
    /// * If it's a label, resolves it to a target.
    ///   * If allow_single_file / allow_files is set, DefaultInfo.files of the target is expanded into files.
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
            }
            crate::LabelOrFile::File(f) => {
                if let Some(matcher) = schema.file_matcher() {
                    matcher.validate(f.as_str())?;
                }
                if schema.file_matcher().is_some() {
                    files.insert(f.clone());
                }
                Ok(heap.alloc(f.clone()))
            }
        }
    }
}


