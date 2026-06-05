// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::{fmt, fmt::Display, hash::Hasher as _};

use allocative::Allocative;
use starlark::{
    any::ProvidesStaticType,
    coerce::Coerce,
    collections::{Hashed, StarlarkHasher},
    starlark_complex_value,
    values::{
        typing::TypeInstanceId, Freeze, Heap, StarlarkValue, Trace, Value, ValueLifetimeless,
        ValueLike, ValueTyped,
    },
};
use starlark_derive::{starlark_value, NoSerialize};
use types::CoercableOption;

use crate::provider_type::ProviderType;

/// Represents an instance of a provider.
#[derive(Clone, Trace, Coerce, Freeze, ProvidesStaticType, Allocative, NoSerialize)]
#[repr(C)]
pub struct ProviderInstanceGen<V: ValueLifetimeless> {
    pub(crate) provider_type: V, // Must be ProviderType
    pub(crate) values: Box<[CoercableOption<V>]>,
}

starlark_complex_value!(pub ProviderInstance);

impl<'v, V: ValueLike<'v>> ProviderInstanceGen<V> {
    pub(crate) fn ty(&self) -> &'v ProviderType {
        // Safety: The constructor always sets self.provider_type to a Value wrapping a
        // ProviderType.
        unsafe {
            ValueTyped::<'v, ProviderType>::new_unchecked(self.provider_type.to_value()).as_ref()
        }
    }

    pub(crate) fn ty_name(&self) -> &'v str {
        &self.ty().configured().name
    }

    pub fn provider_type_id(&self) -> TypeInstanceId {
        self.ty().id
    }

    pub fn iter<'a>(&'a self) -> impl Iterator<Item = (&'v str, V)> + 'a
    where
        'v: 'a,
    {
        let fields = &self.ty().fields;
        fields
            .iter()
            .filter_map(move |(name, &idx)| self.values[idx].map(|val| (name.as_str(), val)))
    }
}

impl<'v, V: ValueLike<'v>> fmt::Debug for ProviderInstanceGen<V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}(", self.ty_name())?;
        for (i, (name, val)) in self.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{name} = {val}")?;
        }
        write!(f, ")")
    }
}

impl<'v, V: ValueLike<'v>> Display for ProviderInstanceGen<V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

#[starlark_value(type = "provider_instance")]
impl<'v, V: ValueLike<'v>> StarlarkValue<'v> for ProviderInstanceGen<V>
where
    Self: ProvidesStaticType<'v>,
{
    type Canonical = FrozenProviderInstance;

    fn get_type_value_dyn(&self) -> starlark::values::FrozenStringValue {
        self.ty().configured().name
    }

    fn get_attr(&self, attribute: &str, heap: Heap<'v>) -> Option<Value<'v>> {
        self.get_attr_hashed(Hashed::new(attribute), heap)
    }

    fn get_attr_hashed(&self, attribute: Hashed<&str>, _heap: Heap<'v>) -> Option<Value<'v>> {
        let &i = self.ty().fields.get_hashed(attribute)?;
        self.values[i].map(|v| v.to_value())
    }

    fn write_hash(&self, hasher: &mut StarlarkHasher) -> starlark::Result<()> {
        self.provider_type.write_hash(hasher)?;
        for v in &self.values {
            if let Some(val) = **v {
                val.write_hash(hasher)?;
            } else {
                hasher.write_u8(0);
            }
        }
        Ok(())
    }

    fn collect_repr(&self, collector: &mut String) {
        use std::fmt::Write as _;
        write!(collector, "{self:?}").unwrap();
    }

    fn dir_attr(&self) -> Vec<String> {
        let fields = &self.ty().fields;
        fields
            .iter()
            .filter_map(|(name, &idx)| {
                if self.values[idx].is_some() {
                    Some(name.clone())
                } else {
                    None
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::register_providers;

    fn new_assert() -> testutils::Assert {
        let mut a = testutils::Assert::default();
        a.modify_globals(|builder| {
            register_providers(builder);
        });
        a
    }

    #[test]
    fn test_provider_instance() {
        let mut a = new_assert();

        let ty = a.pass("MyInfo = provider(fields = ['first', 'second', 'third']); MyInfo");
        a.modify_globals(move |builder| {
            builder.set("MyInfo", ty.clone());
        });

        let instance = a.pass("MyInfo(first = 1, third = 3)");

        // Register them as globals
        a.modify_globals(move |builder| {
            builder.set("info", instance.clone());
        });

        a.eq("type(info)", "MyInfo".to_string());

        a.eq("str(info)", "MyInfo(first = 1, third = 3)".to_string());
        a.eq("repr(info)", "MyInfo(first = 1, third = 3)".to_string());

        a.eq(
            "dir(info)",
            starlark::values::list::UnpackList {
                items: vec!["first".to_string(), "third".to_string()],
            },
        );

        // 'first' field: declared and set
        a.eq(r#"hasattr(info, "first")"#, true);
        a.eq(r#"getattr(info, "first")"#, 1);
        a.eq(r#"info.first"#, 1);

        // 'second' field: declared but unset
        a.eq(r#"hasattr(info, "second")"#, false);
        a.fail(
            "info.second",
            "Object of type `MyInfo` has no attribute `second`",
        );
        a.fail(
            "getattr(info, 'second')",
            "Operation `.second` not supported on type `MyInfo`",
        );

        // 'nonexistent' field: undeclared and unset
        a.eq(r#"hasattr(info, "nonexistent")"#, false);
        a.fail(
            "info.nonexistent",
            "Object of type `MyInfo` has no attribute `nonexistent`",
        );
        a.fail(
            "getattr(info, 'nonexistent')",
            "Operation `.nonexistent` not supported on type `MyInfo`",
        );
        a.eq(r#"getattr(info, "nonexistent", 99)"#, 99);
    }
}
