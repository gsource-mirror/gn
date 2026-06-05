// Copyright 2026 The GN Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use allocative::Allocative;
use starlark::values::{Heap, Value, ValueLike};
use starlark::collections::SmallMap;
use crate::file::File;
use crate::LabelRef;

use super::schema::AttrSchema;
use super::Attr;

#[derive(Clone, Debug, Allocative)]
pub struct AttrValue<'v> {
    pub(crate) file: Option<Value<'v>>,
    pub(crate) files: Option<Value<'v>>,
    pub(crate) attr: Value<'v>,
}

impl Attr {
    pub fn to_coerced_value<'v>(&'static self, heap: &Heap<'v>) -> Value<'v> {
        match self {
            Self::None => Value::new_none(),
            Self::Bool(b) => Value::new_bool(*b),
            Self::Int(i) => heap.alloc(*i),
            Self::String(s) => heap.alloc(s.as_str()),
            Self::IntList(l) => heap.alloc(l.clone()),
            Self::StringList(l) => heap.alloc(l.clone()),
            Self::StringListDict(d) => {
                let mut res = SmallMap::with_capacity(d.len());
                for (k, v) in d.iter() {
                    let k_val = heap.alloc(k.as_str());
                    let v_val = heap.alloc(v.clone());
                    res.insert_hashed(k_val.get_hashed().unwrap(), v_val);
                }
                heap.alloc(starlark::values::dict::Dict::new(res))
            }
            Self::Label(lf) => lf.to_value(heap),
            Self::LabelList(l) => {
                let list: Vec<Value<'v>> = l.iter().map(|lf| lf.to_value(heap)).collect();
                heap.alloc(list)
            }
            Self::StringDict(d) => {
                let mut res = SmallMap::with_capacity(d.len());
                for (k, v) in d.iter() {
                    let k_val = heap.alloc(k.as_str());
                    let v_val = heap.alloc(v.as_str());
                    res.insert_hashed(k_val.get_hashed().unwrap(), v_val);
                }
                heap.alloc(starlark::values::dict::Dict::new(res))
            }
            Self::LabelKeyedStringDict(d) => {
                let mut res = SmallMap::with_capacity(d.len());
                for (k, v) in d.iter() {
                    let k_val = k.to_value(heap);
                    let v_val = heap.alloc(v.as_str());
                    res.insert_hashed(k_val.get_hashed().unwrap(), v_val);
                }
                heap.alloc(starlark::values::dict::Dict::new(res))
            }
            Self::StringKeyedLabelDict(d) => {
                let mut res = SmallMap::with_capacity(d.len());
                for (k, v) in d.iter() {
                    let k_val = heap.alloc(k.as_str());
                    let v_val = v.to_value(heap);
                    res.insert_hashed(k_val.get_hashed().unwrap(), v_val);
                }
                heap.alloc(starlark::values::dict::Dict::new(res))
            }
            Self::LabelListDict(d) => {
                let mut res = SmallMap::with_capacity(d.len());
                for (k, v) in d.iter() {
                    let k_val = heap.alloc(k.as_str());
                    let v_list: Vec<Value<'v>> = v.iter().map(|lf| lf.to_value(heap)).collect();
                    let v_val = heap.alloc(v_list);
                    res.insert_hashed(k_val.get_hashed().unwrap(), v_val);
                }
                heap.alloc(starlark::values::dict::Dict::new(res))
            }
        }
    }

    pub fn to_value<'v>(
        &'static self,
        schema: &AttrSchema,
        rust_session: &crate::session::StarlarkSession,
        current_toolchain: &LabelRef,
        target: *mut crate::ffi::Target,
        heap: &Heap<'v>,
    ) -> starlark::Result<AttrValue<'v>> {
        let coerced = self.to_coerced_value(heap);
        let mut files = Vec::new();
        let resolved = resolve_and_collect(coerced, schema, rust_session, current_toolchain, target, heap, &mut files)?;

        let file_val = match &schema.allow_files {
            crate::attr::AllowFilesSchema::Single(_) => {
                if files.len() == 1 {
                    Some(files[0])
                } else if files.is_empty() {
                    Some(Value::new_none())
                } else {
                    let label = unsafe { &*crate::ffi::GetTargetLabel(&*target) };
                    return Err(starlark::Error::new_other(crate::errors::Error::MustProduceSingleFile(label.GetUserVisibleName(true).to_string())));
                }
            }
            _ => None,
        };

        let files_val = if schema.file_matcher().is_some() {
            Some(heap.alloc(files))
        } else {
            None
        };

        Ok(AttrValue {
            file: file_val,
            files: files_val,
            attr: resolved,
        })
    }
}

fn resolve_and_collect<'v>(
    val: Value<'v>,
    schema: &AttrSchema,
    rust_session: &crate::session::StarlarkSession,
    current_toolchain: &LabelRef,
    target: *mut crate::ffi::Target,
    heap: &Heap<'v>,
    files: &mut Vec<Value<'v>>,
) -> starlark::Result<Value<'v>> {
    if val.is_none() {
        return Ok(Value::new_none());
    }

    if let Some(list) = starlark::values::list::ListRef::from_value(val) {
        let mut resolved_list = Vec::with_capacity(list.len());
        for item in list.iter() {
            let item: Value<'_> = unsafe { std::mem::transmute(item) };
            let resolved = resolve_and_collect(item, schema, rust_session, current_toolchain, target, heap, files)?;
            resolved_list.push(resolved);
        }
        Ok(heap.alloc(resolved_list))
    } else if let Some(dict) = starlark::values::dict::DictRef::from_value(val) {
        let mut resolved_dict = SmallMap::with_capacity(dict.len());
        for (k, v) in dict.iter() {
            let k: Value<'_> = unsafe { std::mem::transmute(k) };
            let v: Value<'_> = unsafe { std::mem::transmute(v) };
            let resolved_k = resolve_and_collect(k, schema, rust_session, current_toolchain, target, heap, files)?;
            let resolved_v = resolve_and_collect(v, schema, rust_session, current_toolchain, target, heap, files)?;
            resolved_dict.insert_hashed(resolved_k.get_hashed()?, resolved_v);
        }
        Ok(heap.alloc(starlark::values::dict::Dict::new(resolved_dict)))
    } else if let Some(dep_lbl) = val.downcast_ref::<crate::label::Label>() {
        let target_ref = rust_session.get_target_by_label(dep_lbl.as_ref(), current_toolchain.clone());
        let target_val = heap.alloc(target_ref);
        if let Some(matcher) = schema.file_matcher() {
            for f in target_ref.outputs() {
                matcher.validate(f.as_path()).map_err(starlark::Error::new_other)?;
                files.push(heap.alloc(f));
            }
        }
        Ok(target_val)
    } else if let Some(f) = val.downcast_ref::<File>() {
        if let Some(matcher) = schema.file_matcher() {
            matcher.validate(f.as_path()).map_err(starlark::Error::new_other)?;
            files.push(val);
        }
        Ok(val)
    } else {
        Ok(val)
    }
}
