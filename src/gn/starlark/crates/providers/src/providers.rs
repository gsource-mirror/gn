// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use args::FrozenArgsSequence;
use depset::UnpackFileDepset;
use starlark::{
    collections::SmallMap,
    typing::Ty,
    values::{
        list::ListRef, structs::FrozenStructRef, type_repr::StarlarkTypeRepr,
        typing::TypeInstanceId, FrozenValue, UnpackValue, Value, ValueLike,
    },
};
use types::File;

use crate::ProviderInstance;

/// Helper to unpack a frozen list of providers to useful metadata.
#[derive(Debug, PartialEq, Clone, Default)]
pub struct Providers {
    /// Value of DefaultInfo.files
    pub outputs_phony: Option<File>,
    /// Value of GnInputsInfo.files
    pub inputs_phony: Option<File>,
    /// Value of GnSubstitutionInfo.substitutions
    pub substitutions: SmallMap<&'static str, FrozenArgsSequence<'static>>,
    /// All provider instances mapped by their ProviderType's TypeInstanceId.
    pub value: SmallMap<TypeInstanceId, FrozenValue>,
}

impl StarlarkTypeRepr for Providers {
    type Canonical = Value<'static>;

    fn starlark_type_repr() -> Ty {
        Ty::list(Ty::any())
    }
}

impl<'v> UnpackValue<'v> for Providers {
    type Error = starlark::Error;

    fn unpack_value_impl(value: Value<'v>) -> Result<Option<Self>, Self::Error> {
        debug_assert!(value.unpack_frozen().is_some());

        let mut providers = Providers::default();
        for item in <&ListRef>::unpack_value_err(value)?.iter() {
            let instance = <&ProviderInstance>::unpack_value_err(item.to_value())?;
            // Safety: This function can only be called on frozen values.
            // We did a debug_assert at the top of the function to validate that,
            // but it remains the responsibility of the user to ensure that you
            // only call this function on frozen values.
            let item = unsafe { item.unpack_frozen().unwrap_unchecked() };

            let type_id = instance.ty().id;
            let values = &instance.values;

            let frozen_item = item;

            if providers.value.insert(type_id, frozen_item).is_some() {
                return Err(
                    crate::errors::Error::DuplicateProvider(instance.ty_name().to_owned()).into(),
                );
            }

            match type_id {
                crate::builtins::DEFAULT_INFO_ID => {
                    let files = values[0].unwrap();
                    providers.outputs_phony = UnpackFileDepset::unpack_value_err(files)
                        .map_err(|_| {
                            crate::errors::Error::DefaultInfoFilesMustBeFileDepset(files.to_repr())
                        })?
                        .0;
                },
                crate::builtins::INPUTS_INFO_ID => {
                    let files = values[0].unwrap();
                    providers.inputs_phony = UnpackFileDepset::unpack_value_err(files)
                        .map_err(|_| {
                            crate::errors::Error::GnInputsInfoFilesMustBeFileDepset(files.to_repr())
                        })?
                        .0;
                },
                crate::builtins::SUBSTITUTIONS_INFO_ID => {
                    // GnSubstitutionsInfo(substitutions = struct)
                    // Safety: We already checked it was frozen earlier.
                    let substitutions_val = values[0].unwrap();
                    let substitutions_struct = FrozenStructRef::from_value(unsafe {
                        substitutions_val.unpack_frozen().unwrap_unchecked()
                    })
                    .ok_or_else(|| {
                        starlark::Error::from(
                            crate::errors::Error::GnSubstitutionsInfoSubstitutionsMustBeStruct(
                                substitutions_val.to_repr(),
                            ),
                        )
                    })?;

                    providers.substitutions = substitutions_struct
                        .iter()
                        .map(|(k, v)| {
                            Ok((
                                k.as_str(),
                                <FrozenArgsSequence>::unpack_value_err(v.to_value())?,
                            ))
                        })
                        .collect::<Result<_, starlark::Error>>()?;
                },
                _ => {},
            }
        }

        Ok(Some(providers))
    }
}

#[cfg(test)]
mod tests {
    use starlark::values::UnpackValue;
    use types::File;

    use crate::Providers;

    fn new_assert() -> testutils::Assert {
        let mut a = testutils::Assert::default();

        a.modify_globals(|builder| {
            depset::depset_globals!(builder, testutils::eval_context::FakeEvalContext);
            let builtin_providers = crate::globals::register_providers(builder);
            builder.set(
                "GnInputsInfo",
                builtin_providers.module.get("GnInputsInfo").unwrap(),
            );
            builder.set(
                "GnSubstitutionsInfo",
                builtin_providers.module.get("GnSubstitutionsInfo").unwrap(),
            );
        });
        a
    }

    #[track_caller]
    fn assert_unpack_fails(a: &mut testutils::Assert, expr: &str, expected_err: &str) {
        let val = a.pass(expr);
        let err = Providers::unpack_value_err(val.value()).unwrap_err();
        assert_eq!(err.to_string(), expected_err);
    }

    #[test]
    fn test_providers_unpacking() {
        let mut a = new_assert();

        let custom_info_ty = a.pass("CustomInfo = provider(fields = ['foo']); CustomInfo");
        a.modify_globals(move |builder| {
            builder.set("CustomInfo", custom_info_ty.clone());
        });

        // Unpacking all three providers
        let val = a.pass(
            r#"[
    DefaultInfo(files = depset([make_file("a")])),
    GnInputsInfo(files = depset([make_file("b")])),
    GnSubstitutionsInfo(substitutions = struct(key = ["val"])),
    CustomInfo(foo = 1),
]"#,
        );
        let providers = Providers::unpack_value_err(val.value()).unwrap();

        assert_eq!(providers.outputs_phony, Some(File::intern("a")));
        assert_eq!(providers.inputs_phony, Some(File::intern("b")));

        // Check substitutions
        let keys: Vec<&str> = providers.substitutions.keys().copied().collect();
        assert_eq!(keys, vec!["key"]);

        // Check value map
        assert_eq!(providers.value.len(), 4);
        assert!(providers
            .value
            .contains_key(&crate::builtins::DEFAULT_INFO_ID));
        assert!(providers
            .value
            .contains_key(&crate::builtins::INPUTS_INFO_ID));
        assert!(providers
            .value
            .contains_key(&crate::builtins::SUBSTITUTIONS_INFO_ID));

        let val = a.pass("[]");
        let providers = Providers::unpack_value_err(val.value()).unwrap();
        assert_eq!(providers.outputs_phony, None);
        assert_eq!(providers.inputs_phony, None);
        assert!(providers.substitutions.is_empty());
        assert!(providers.value.is_empty());

        // Duplicate provider check
        assert_unpack_fails(
            &mut a,
            "[DefaultInfo(files = depset()), DefaultInfo(files = depset())]",
            "Duplicate provider: DefaultInfo",
        );
    }

    #[test]
    fn test_providers_unpacking_default_info_errors() {
        let mut a = new_assert();

        // DefaultInfo.files must be a depset
        assert_unpack_fails(
            &mut a,
            r#"[DefaultInfo(files = "not-a-depset")]"#,
            "DefaultInfo.files must be a depset of files, got \"not-a-depset\"",
        );

        // DefaultInfo.files must contain only files
        assert_unpack_fails(
            &mut a,
            r#"[DefaultInfo(files = depset(["not-a-file"]))]"#,
            "DefaultInfo.files must be a depset of files, got depset(...)",
        );
    }

    #[test]
    fn test_providers_unpacking_inputs_info_errors() {
        let mut a = new_assert();

        // GnInputsInfo.files must be a depset
        assert_unpack_fails(
            &mut a,
            r#"[GnInputsInfo(files = "not-a-depset")]"#,
            "GnInputsInfo.files must be a depset of files, got \"not-a-depset\"",
        );

        // GnInputsInfo.files must contain only files
        assert_unpack_fails(
            &mut a,
            r#"[GnInputsInfo(files = depset(["not-a-file"]))]"#,
            "GnInputsInfo.files must be a depset of files, got depset(...)",
        );
    }

    #[test]
    fn test_providers_unpacking_substitutions_info_errors() {
        let mut a = new_assert();

        // GnSubstitutionsInfo.substitutions must be a struct
        assert_unpack_fails(
            &mut a,
            r#"[GnSubstitutionsInfo(substitutions = {"key": ["val"]})]"#,
            "GnSubstitutionsInfo.substitutions must be a struct, got {\"key\": [\"val\"]}",
        );

        // Expected list value in substitutions struct
        assert_unpack_fails(
            &mut a,
            r#"[GnSubstitutionsInfo(substitutions = struct(key = "not-a-list"))]"#,
            "Expected `list`, but got `string (repr: \"not-a-list\")`",
        );

        // Expected string or Args inside substitutions list
        assert_unpack_fails(
            &mut a,
            r#"[GnSubstitutionsInfo(substitutions = struct(key = [123]))]"#,
            "Expected `Args | str`, but got `int (repr: 123)`",
        );
    }
}
