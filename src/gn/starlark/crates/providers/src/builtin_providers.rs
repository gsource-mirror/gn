// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use starlark::environment::Module;
use starlark::eval::Evaluator;
use starlark::values::record::{Record, RecordType};
use starlark::values::{
    FrozenHeapName, FrozenHeapRef, FrozenValue, StarlarkValue as _, Value, ValueLike as _,
};

use crate::provider::{provider_fields, TypeId};

/// Stores the compiled Starlark objects and type IDs for GN's built-in Starlark providers (e.g. DefaultInfo).
pub struct BuiltinModule {
    /// The heap in which all these members are stored to prevent the reference count falling to zero.
    #[allow(dead_code)]
    heap: FrozenHeapRef,
    /// The `DefaultInfo` provider type object.
    pub default_info: FrozenValue,
    /// The `GnSubstitutionInfo` provider type object.
    pub gn_substitution_info: FrozenValue,
    /// The `GnInputsInfo` provider type object.
    pub gn_inputs_info: FrozenValue,
    /// The type ID for `DefaultInfo`.
    pub default_info_type: TypeId,
    /// An empty `DefaultInfo` instance.
    pub empty_default_info: FrozenValue,
    /// The type ID for `GnInputsInfo`.
    pub gn_inputs_info_type: TypeId,
    /// The type ID for `GnSubstitutionInfo`.
    pub gn_substitution_info_type: TypeId,
}

/// Loads and instantiates the built-in providers, freezing them into a static `BuiltinModule`.
pub fn load_builtin_providers() -> BuiltinModule {
    let frozen = Module::with_temp_heap(|module| {
        let heap = module.heap();

        let default_info = {
            let mut evaluator = Evaluator::new(&module);
            let mut create_provider = |name, fields: &[&str]| {
                let record_type = RecordType::new(provider_fields(fields));
                record_type.export_as(name, &mut evaluator).unwrap();
                let val = heap.alloc(record_type);
                module.set(name, val);
                val
            };

            create_provider("GnSubstitutionInfo", &["substitutions"]);
            create_provider("GnInputsInfo", &["files"]);
            create_provider("DefaultInfo", &["files", "executable"])
        };

        // Create empty_default_info = DefaultInfo(executable = None, files = empty_depset)
        module.set(
            "empty_default_info",
            heap.alloc(Record {
                typ: default_info,
                values: vec![heap.alloc(depset::Depset::default()), Value::new_none()]
                    .into_boxed_slice(),
            }),
        );

        module
            .freeze_named(FrozenHeapName::User(Box::new("builtins".to_owned())))
            .unwrap()
    });

    let default_info = frozen
        .get("DefaultInfo")
        .unwrap()
        .value()
        .unpack_frozen()
        .unwrap();
    let gn_inputs_info = frozen
        .get("GnInputsInfo")
        .unwrap()
        .value()
        .unpack_frozen()
        .unwrap();
    let gn_substitution_info = frozen
        .get("GnSubstitutionInfo")
        .unwrap()
        .value()
        .unpack_frozen()
        .unwrap();
    let empty_default_info = frozen
        .get("empty_default_info")
        .unwrap()
        .value()
        .unpack_frozen()
        .unwrap();

    BuiltinModule {
        heap: frozen.frozen_heap().clone(),
        default_info,
        gn_substitution_info,
        gn_inputs_info,
        default_info_type: default_info
            .to_value()
            .downcast_ref::<starlark::values::record::FrozenRecordType>()
            .unwrap()
            .type_id(),
        empty_default_info,
        gn_inputs_info_type: gn_inputs_info
            .to_value()
            .downcast_ref::<starlark::values::record::FrozenRecordType>()
            .unwrap()
            .type_id(),
        gn_substitution_info_type: gn_substitution_info
            .to_value()
            .downcast_ref::<starlark::values::record::FrozenRecordType>()
            .unwrap()
            .type_id(),
    }
}
