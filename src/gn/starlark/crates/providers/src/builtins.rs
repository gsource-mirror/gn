// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use depset::FrozenDepset;
use starlark::{
    environment::{FrozenModule, GlobalsBuilder, Module},
    eval::ParametersSpecParam::{Defaulted, Required},
    values::{typing::TypeInstanceId, FrozenValue, FrozenValueTyped},
};

use crate::{provider_instance::ProviderInstanceGen, provider_type::FrozenProviderType};

// Compile-time assertion that the layout matches u64, in case starlark-rs
// decides to change this.
const _: () = {
    assert!(std::mem::size_of::<TypeInstanceId>() == std::mem::size_of::<u64>());
    assert!(std::mem::align_of::<TypeInstanceId>() == std::mem::align_of::<u64>());
};

// Hardcoded stable IDs for builtin providers.
// These are verified by a unit test to guarantee correctness and prevent drift.
pub(crate) const DEFAULT_INFO_ID: TypeInstanceId =
    unsafe { std::mem::transmute::<u64, TypeInstanceId>(6904120031855430705) };
pub(crate) const INPUTS_INFO_ID: TypeInstanceId =
    unsafe { std::mem::transmute::<u64, TypeInstanceId>(10321705343315126159) };
pub(crate) const SUBSTITUTIONS_INFO_ID: TypeInstanceId =
    unsafe { std::mem::transmute::<u64, TypeInstanceId>(8875227055924591575) };

pub struct BuiltinProviders {
    pub default_defaultinfo: FrozenValue,
    pub module: FrozenModule,
}

pub(crate) fn register_builtin_providers(builder: &mut GlobalsBuilder) -> BuiltinProviders {
    let empty_depset = builder.alloc(FrozenDepset::default());
    let default_info = FrozenProviderType::new(
        DEFAULT_INFO_ID,
        "DefaultInfo",
        &[
            ("files", Defaulted(empty_depset)),
            ("executable", Defaulted(FrozenValue::new_none())),
        ],
        builder.frozen_heap(),
    );
    let default_info_value = builder.alloc(default_info);
    builder.set("DefaultInfo", default_info_value);

    let inputs = FrozenProviderType::new(
        INPUTS_INFO_ID,
        "GnInputsInfo",
        &[("files", Required)],
        builder.frozen_heap(),
    );

    let substitutions = FrozenProviderType::new(
        SUBSTITUTIONS_INFO_ID,
        "GnSubstitutionsInfo",
        &[("substitutions", Required)],
        builder.frozen_heap(),
    );

    BuiltinProviders {
        default_defaultinfo: builder.alloc(ProviderInstanceGen {
            provider_type: FrozenValueTyped::new(default_info_value).unwrap(),
            values: Box::new([Some(empty_depset), Some(FrozenValue::new_none())]),
        }),
        module: Module::with_temp_heap(|module: Module| {
            module.set(
                "GnSubstitutionsInfo",
                builder.alloc(substitutions).to_value(),
            );
            module.set("GnInputsInfo", builder.alloc(inputs).to_value());
            module
                .freeze_named(starlark::values::FrozenHeapName::User(Box::new(
                    "//builtins:providers.bzl".to_owned(),
                )))
                .unwrap()
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use starlark::values::typing::TypeIdDomain;

    struct GnBuiltinDomain;
    impl TypeIdDomain for GnBuiltinDomain {
        fn tag(&self) -> &'static str {
            "gn.builtin_provider"
        }
    }

    #[test]
    fn test_builtin_provider_ids_match_identity() {
        assert_eq!(
            DEFAULT_INFO_ID,
            TypeInstanceId::from_identity(GnBuiltinDomain, &"DefaultInfo")
        );
        assert_eq!(
            INPUTS_INFO_ID,
            TypeInstanceId::from_identity(GnBuiltinDomain, &"GnInputsInfo")
        );
        assert_eq!(
            SUBSTITUTIONS_INFO_ID,
            TypeInstanceId::from_identity(GnBuiltinDomain, &"GnSubstitutionsInfo")
        );
    }
}
