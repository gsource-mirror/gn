// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use starlark::{
    typing::Ty,
    values::{
        type_repr::StarlarkTypeRepr, typing::TypeInstanceId, UnpackValue, Value, ValueLike as _,
    },
};

use crate::provider_instance::FrozenProviderInstance;
use crate::provider_instance::ProviderInstance;

/// Represents a unique type identifier for a provider instance in Starlark.
pub type TypeId = TypeInstanceId;

/// Helper utility to unpack a Starlark `Value` representing a provider
/// instance.
#[derive(Debug, Clone, Copy)]
pub struct UnpackProviderInstance<'v> {
    pub type_id: TypeInstanceId,
    pub value: Value<'v>,
}

impl StarlarkTypeRepr for UnpackProviderInstance<'_> {
    type Canonical = Self;

    fn starlark_type_repr() -> Ty {
        Ty::any()
    }
}

impl<'v> UnpackValue<'v> for UnpackProviderInstance<'v> {
    type Error = starlark::Error;

    fn unpack_value_impl(value: Value<'v>) -> Result<Option<Self>, Self::Error> {
        if let Some(instance) = value.downcast_ref::<ProviderInstance<'v>>() {
            Ok(Some(UnpackProviderInstance {
                type_id: instance.ty().id,
                value,
            }))
        } else if let Some(instance) = value.downcast_ref::<FrozenProviderInstance>() {
            Ok(Some(UnpackProviderInstance {
                type_id: instance.ty().id,
                value,
            }))
        } else {
            Ok(None)
        }
    }
}

impl<'v> UnpackProviderInstance<'v> {
    /// Returns the slice of field values (slots) for this provider instance.
    pub fn values(&self) -> &'v [Option<Value<'v>>] {
        if let Some(instance) = self.value.downcast_ref::<ProviderInstance<'v>>() {
            starlark::coerce::coerce(&*instance.values)
        } else if let Some(instance) = self.value.downcast_ref::<FrozenProviderInstance>() {
            starlark::coerce::coerce(&*instance.values)
        } else {
            unreachable!("UnpackProviderInstance guarantees value is a provider instance")
        }
    }
}

#[cfg(test)]
mod tests {
    use starlark::values::{list::UnpackList, UnpackValue as _};

    use crate::{globals::register_providers, UnpackProviderInstance};

    fn new_assert() -> testutils::Assert {
        let mut a = testutils::Assert::default();
        a.modify_globals(|builder| {
            register_providers(builder);
        });
        a
    }

    #[test]
    fn test_unpack_provider_instance() {
        let mut a = new_assert();

        let val = a.pass(r#"1"#);
        let err = <UnpackProviderInstance>::unpack_param(val.value()).unwrap_err();
        assert_eq!(
            err.to_string(),
            "Type of parameters mismatch, expected `typing.Any`, actual `int (repr: 1)`"
        );

        let val = a.pass(
            r#"
p_first = provider(fields = ['a'])
p_second = provider(fields = ['a'])
[p_first(a=1), p_second(a=2), p_first(a=3)]
"#,
        );
        let providers = <UnpackList<UnpackProviderInstance>>::unpack_param(val.value())
            .unwrap()
            .items;
        // Check the type IDs
        assert_eq!(providers[0].type_id, providers[2].type_id);
        assert_ne!(providers[0].type_id, providers[1].type_id);

        let val = a.pass(r#"
p = provider(fields = ["a", "b", "c"])
p(a=1, c=3)
"#);
        let values = <UnpackProviderInstance>::unpack_param(val.value())
            .unwrap()
            .values();
        let expected1 = a.pass(r"1");
        let expected3 = a.pass(r"3");
        assert_eq!(values[0], Some(expected1.value()));
        assert_eq!(values[1], None);
        assert_eq!(values[2], Some(expected3.value()));
    }
}
