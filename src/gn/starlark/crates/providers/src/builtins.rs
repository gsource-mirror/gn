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
// This allows us to avoid having to store them anywhere, which would require
// expensive lookups in complex data structures. We pick an arbitrary number to
// subtract from u64::max as it's possible that starlark-rs might choose to use
// a similar technique to reserve some types itself.
// Safety: We have a compile time assertion to check that the layouts match.
pub(crate) const DEFAULT_INFO_ID: TypeInstanceId =
    unsafe { std::mem::transmute::<u64, TypeInstanceId>(u64::MAX - 58373) };
pub(crate) const INPUTS_INFO_ID: TypeInstanceId =
    unsafe { std::mem::transmute::<u64, TypeInstanceId>(u64::MAX - 58374) };
pub(crate) const SUBSTITUTIONS_INFO_ID: TypeInstanceId =
    unsafe { std::mem::transmute::<u64, TypeInstanceId>(u64::MAX - 58375) };

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
