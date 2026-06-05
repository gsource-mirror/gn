// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use providers::provider_fields;
use starlark::collections::SmallMap;
use starlark::eval::ParametersSpec;
use starlark::eval::ParametersSpecParam;
use starlark::values::record::RecordType;
use starlark::values::Value;
use starlark::eval::Evaluator;

use super::rule::RuleGen;
use attr::{AllowFilesSchema, AttrSchema};

pub fn common_rule<'v, 'a, 'e>(
    implementation: Value<'v>,
    attrs: Option<SmallMap<&'v str, &'v AttrSchema>>,
    is_extension: bool,
    eval: &Evaluator<'v, 'a, 'e>,
) -> starlark::Result<RuleGen<Value<'v>>> {
    let heap = eval.heap();
    let attrs = attrs.unwrap_or_default();
    if attrs.contains_key("name") {
        return Err(crate::errors::Error::NameAttrForbidden.into());
    }
    let named_only: Vec<_> = attrs
        .iter()
        .map(|(name, attr)| {
            let spec_val = match attr.default() {
                None => ParametersSpecParam::Required,
                Some(_) => ParametersSpecParam::Optional,
            };
            (*name, spec_val)
        })
        .collect();

    let signature = ParametersSpec::new_parts(
        "rule",
        // Rule extensions should take the target to extend as a positional-only argument. They don't need a name because the target is already named.
        if is_extension {
            vec![("target", ParametersSpecParam::Required)]
        } else {
            vec![]
        },
        // Rules should take the target as a named-only argument. *However*, we allow positional or named to allow the following GN syntax:
        // target("name") { ... }
        if !is_extension {
            vec![("name", ParametersSpecParam::Required)]
        } else {
            vec![]
        },
        false,
        named_only,
        false,
    );

    let attrs_keys: Vec<&str> = attrs.keys().map(|s| s.as_ref()).collect();
    let file_keys: Vec<&str> = attrs
        .iter()
        .filter(|(_, attr)| matches!(attr.allow_files(), AllowFilesSchema::Single(_)))
        .map(|(name, _)| name.as_ref())
        .collect();
    let files_keys: Vec<&str> = attrs
        .iter()
        .filter(|(_, attr)| attr.file_matcher().is_some())
        .map(|(name, _)| name.as_ref())
        .collect();

    let attrs_record_type = heap.alloc(RecordType::new(provider_fields(&attrs_keys)));
    let file_record_type = heap.alloc(RecordType::new(provider_fields(&file_keys)));
    let files_record_type = heap.alloc(RecordType::new(provider_fields(&files_keys)));

    Ok(RuleGen {
        implementation,
        attrs: attrs
            .iter()
            .map(|(k, &v)| (k.to_string(), v.clone()))
            .collect(),
        is_extension,
        signature,
        attrs_record_type,
        file_record_type,
        files_record_type,
    })
}
