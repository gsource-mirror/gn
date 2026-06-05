// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use starlark::collections::SmallMap;
use starlark::environment::GlobalsBuilder;
use starlark::eval::Evaluator;
use starlark::typing::Ty;
use starlark::values::record::FieldGen;
use starlark::values::record::FrozenRecord;
use starlark::values::record::Record;
use starlark::values::record::RecordType;
use starlark::values::type_repr::StarlarkTypeRepr;
use starlark::values::typing::TypeCompiled;
use starlark::values::typing::TypeInstanceId;
use starlark::values::UnpackValue;
use starlark::values::Value;
use starlark::values::ValueLike as _;
use starlark_derive::starlark_module;

use crate::errors::Error;

/// Represents a unique type identifier for a provider instance in Starlark.
pub type TypeId = TypeInstanceId;

/// Helper utility to unpack a Starlark `Value` representing a provider (a `Record` or `FrozenRecord`).
#[derive(Debug)]
pub struct UnpackProvider<'v>(pub TypeId, pub Value<'v>);

impl StarlarkTypeRepr for UnpackProvider<'_> {
    type Canonical = Self;
    fn starlark_type_repr() -> Ty {
        Ty::any()
    }
}

impl<'v> UnpackValue<'v> for UnpackProvider<'v> {
    type Error = starlark::Error;
    fn unpack_value_impl(value: Value<'v>) -> Result<Option<Self>, Self::Error> {
        if let Some(rec) = value.downcast_ref::<Record<'v>>() {
            Ok(Some(UnpackProvider(rec.record_type_id(), value)))
        } else if let Some(rec) = value.downcast_ref::<FrozenRecord>() {
            Ok(Some(UnpackProvider(rec.record_type_id(), value)))
        } else {
            Err(Error::ExpectedProviderType.into())
        }
    }
}

/// Registers the global `provider()` function in Starlark.
#[starlark_module]
pub fn register_provider(builder: &mut GlobalsBuilder) {
    fn provider<'v>(
        fields: Value<'v>,
        eval: &mut Evaluator<'v, '_, '_>,
    ) -> starlark::Result<Value<'v>> {
        let mut kwargs = SmallMap::new();

        let any_type = eval.heap().alloc(TypeCompiled::any());

        for field in fields.iterate(eval.heap())? {
            let s = field.unpack_str().ok_or(Error::FieldsMustBeStrings)?;
            let compiled_type = TypeCompiled::new(any_type, eval.heap())?;
            let field_gen = FieldGen::new(compiled_type, None);
            kwargs.insert(s.to_owned(), field_gen);
        }

        let record_type = RecordType::new(kwargs);
        Ok(eval.heap().alloc(record_type))
    }
}

/// Helper to construct a typed field map for provider records during C++ integration.
pub fn provider_fields<'v>(fields: &[&str]) -> SmallMap<String, FieldGen<Value<'v>>> {
    let mut record_fields = SmallMap::new();
    let any: TypeCompiled<Value<'v>> = starlark::coerce::coerce(TypeCompiled::any());
    for field in fields {
        record_fields.insert(field.to_string(), FieldGen::new(any, None));
    }
    record_fields
}
