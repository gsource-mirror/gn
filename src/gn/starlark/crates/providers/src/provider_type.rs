// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::{
    cell::UnsafeCell,
    fmt,
    fmt::{Debug, Display},
    hash::Hash as _,
};

use allocative::Allocative;
use starlark::{
    any::ProvidesStaticType,
    collections::{SmallMap, StarlarkHasher},
    eval::{Arguments, Evaluator, ParametersSpec, ParametersSpecParam},
    starlark_simple_value,
    values::{
        typing::TypeInstanceId, Freeze, FreezeResult, Freezer, FrozenValue, StarlarkValue, Trace,
        Tracer, Value, ValueLike as _,
    },
};
use starlark_derive::{starlark_value, NoSerialize};

use crate::{errors::Error, provider_instance::ProviderInstance};

#[derive(Debug, Clone, Trace, Allocative)]
// Contains all the information we cannot know about a provider type until we
// actually know the name of it.
pub(crate) struct ProviderTypeData {
    pub(crate) name: starlark::values::FrozenStringValue,
    pub(crate) parameter_spec: ParametersSpec<FrozenValue>,
}

/// Represents the provider type constructor.
#[derive(Debug, ProvidesStaticType, NoSerialize)]
pub struct ProviderType {
    /// The unique type identifier.
    pub(crate) id: TypeInstanceId,
    /// The configured provider fields. This is set when starlark calls
    /// `export_as` when you assign the provider to a variable.
    /// If this is not set, you cannot construct the provider.
    pub(crate) data: UnsafeCell<Option<ProviderTypeData>>,
    /// A mapping from field name to index.
    /// This is akin to python's `__slots__`.
    pub(crate) fields: SmallMap<String, usize>,
}

// Safety: We only write to data during single-threaded evaluation in export_as.
// Once frozen, it is read-only, making concurrent reads safe.
unsafe impl Send for ProviderType {}
unsafe impl Sync for ProviderType {}

starlark_simple_value!(ProviderType);

impl Display for ProviderType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<provider>")
    }
}

#[starlark_value(type = "provider")]
impl<'v> StarlarkValue<'v> for ProviderType {
    fn write_hash(&self, hasher: &mut StarlarkHasher) -> starlark::Result<()> {
        self.id.hash(hasher);
        Ok(())
    }

    fn equals(&self, other: Value<'v>) -> starlark::Result<bool> {
        if let Some(other) = other.downcast_ref::<Self>() {
            Ok(self.id == other.id)
        } else {
            Ok(false)
        }
    }

    fn invoke(
        &self,
        me: Value<'v>,
        args: &Arguments<'v, '_>,
        eval: &mut Evaluator<'v, '_, '_>,
    ) -> starlark::Result<Value<'v>> {
        // Safety: This is safe as long as data is not mutably borrowed.
        // This only happens in export_as, which can only be called while the
        // ProviderType is not frozen. Until it is frozen, evaluation is purely
        // single-threaded.
        let spec = match unsafe { &*self.data.get() } {
            Some(data) => &data.parameter_spec,
            None => return Err(Error::ProviderNotExported.into()),
        };

        let this = me;
        spec.parser(args, eval, |param_parser, eval| {
            let values: Box<[Option<Value<'v>>]> = (0..self.fields.len())
                .map(|_| param_parser.next_opt())
                .collect::<starlark::Result<_>>()?;
            Ok(eval.heap().alloc_complex(ProviderInstance {
                provider_type: this,
                values,
            }))
        })
    }

    fn export_as(&self, name: &str, eval: &mut Evaluator<'v, '_, '_>) -> starlark::Result<()> {
        let name_frozen = eval.frozen_heap().alloc_str(name);
        if !name.ends_with("Info") {
            return Err(Error::InvalidProviderName(name.to_owned()).into());
        }

        let data = ProviderTypeData {
            name: name_frozen,
            parameter_spec: ParametersSpec::new_named_only(
                name,
                self.fields
                    .keys()
                    .map(|f| (f.as_str(), ParametersSpecParam::Optional)),
            ),
        };

        // Safety: This is safe as long as there are no other borrows. This can only
        // happen before freezing, when starlark is still executing single-threaded.
        unsafe {
            *self.data.get() = Some(data);
        }
        Ok(())
    }
}

impl ProviderType {
    /// Creates a new provider type with the provided fields.
    /// This provider is not yet configured, and is unusable until `export_as`
    /// is called.
    pub fn new(fields: Vec<String>) -> Self {
        Self {
            id: TypeInstanceId::r#gen(),
            data: UnsafeCell::new(None),
            fields: fields
                .into_iter()
                .enumerate()
                .map(|(idx, field)| (field, idx))
                .collect(),
        }
    }

    pub(crate) fn configured(&self) -> &ProviderTypeData {
        // Safety: This function is only usable by provider_instance.
        // Successfully creating a provider instance requires export_as to be called.
        unsafe {
            let val = (&*self.data.get()).as_ref();
            debug_assert!(val.is_some());
            val.unwrap_unchecked()
        }
    }
}

unsafe impl<'v> Trace<'v> for ProviderType {
    fn trace(&mut self, _tracer: &Tracer<'v>) {}
}

impl Allocative for ProviderType {
    fn visit<'a, 'b: 'a>(&self, visitor: &'a mut allocative::Visitor<'b>) {
        // ProviderType is a type whose allocation footprint is negligible
        // so visiting its fields is unnecessary (and requires unsafe).
        let _unused = visitor.enter_self_sized::<Self>();
    }
}

impl Freeze for ProviderType {
    type Frozen = Self;

    fn freeze(self, _freezer: &Freezer) -> FreezeResult<Self::Frozen> {
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use crate::globals::register_providers;

    fn new_assert() -> testutils::Assert {
        let mut a = testutils::Assert::default();
        a.modify_globals(|builder| {
            register_providers(builder);
        });
        a
    }

    #[test]
    fn test_provider_type() {
        let mut a = new_assert();
        a.fail(
            "provider()",
            "Missing named-only parameter `fields` for call to `provider`",
        );

        a.fail(
            "provider(fields=['a'])(a=1)",
            "The result of provider() must be assigned to a variable",
        );

        a.fail(
            r#"
PInfo = provider(fields=['a'])
PInfo(a=1, b=2)
"#,
            "Found `b` extra named parameter(s) for call to PInfo",
        );
        a.eq(
            r#"
PInfo = provider(fields=['a'])
PInfo(a=1).a
"#,
            1,
        );
        a.eq(
            r#"
PInfo = provider(fields = {'a': 'desc'})
PInfo(a = 42).a
"#,
            42,
        );
        a.fail(
            "provider(fields = 1)",
            "Provider fields must be an iterable",
        );
        a.fail("provider(fields = ['a', 'a'])", "Duplicate field name: a");
        a.fail(
            r#"
p = provider(fields=['a'])
"#,
            "Provider name must end with 'Info' (got 'p')",
        );
    }
}
