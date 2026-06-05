// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use allocative::Allocative;
use args::ArgsGenFrozen;
use depset::{UnpackDepset, UnpackFileDepset};
use either::Either;
use starlark::collections::SmallMap;
use starlark::environment::{FrozenModule, Module};
use starlark::values::list::ListRef;
use starlark::values::record::{FrozenRecord, Record};
use starlark::values::structs::StructRef;
use starlark::values::{
    FrozenHeapRef, FrozenValue, FrozenValueTyped, UnpackValue as _, Value, ValueLike as _,
};
use types::File;

use crate::builtin_providers::BuiltinModule;
use crate::errors::Error;
use crate::provider::{TypeId, UnpackProvider};

/// Holds the collection of providers produced by a target, along with pre-extracted output files and substitutions.
#[derive(Debug, Allocative, Default)]
pub struct TargetProviders {
    // This field is set to ensure that the frozen values don't get deallocated.
    #[allow(dead_code)]
    heap: FrozenHeapRef,
    providers: SmallMap<TypeId, FrozenValue>,
    // From providers[DefaultInfo]
    output_files: Vec<File>,
    extra_inputs_phony: Option<File>,
    // We can use &'static str safely, because the String objects are stored in the frozen heap
    // that this object keeps alive.
    #[allocative(skip)]
    substitutions: Vec<(
        &'static str,
        Vec<Either<&'static str, FrozenValueTyped<'static, ArgsGenFrozen>>>,
    )>,
}

fn read_builtin_field(value: Value, index: usize) -> Value {
    let record = value.downcast_ref::<FrozenRecord>().unwrap();
    record.values[index].to_value()
}

fn new_defaultinfo<'v>(
    files: Vec<File>,
    default_info_type: Value<'v>,
    heap: &starlark::values::Heap<'v>,
) -> Value<'v> {
    let depset = depset::Depset::new_file_depset(files, heap);
    heap.alloc(Record {
        typ: default_info_type,
        values: vec![heap.alloc(depset), Value::new_none()].into_boxed_slice(),
    })
}

fn parse_defaultinfo(value: Value) -> starlark::Result<Vec<File>> {
    let files = read_builtin_field(value, 0);
    UnpackFileDepset::unpack_value_err(files).map_err(|_| Error::ExpectedDefaultInfoFilesDepset)?;

    let depset = UnpackDepset::unpack_value_err(files).unwrap();
    Ok(depset
        .iter()
        .map(|v| v.downcast_ref::<File>().cloned().unwrap())
        .collect())
}

type ParsedSubstitutions = Vec<(
    &'static str,
    Vec<Either<&'static str, FrozenValueTyped<'static, ArgsGenFrozen>>>,
)>;

fn parse_substitutions(value: FrozenValue) -> starlark::Result<ParsedSubstitutions> {
    let subs_val = read_builtin_field(value.to_value(), 0);
    let mut substitutions = Vec::new();
    let struct_ref = StructRef::from_value(subs_val).ok_or_else(|| {
        starlark::Error::from(Error::SubstitutionsMustBeStruct(
            subs_val.get_type().to_owned(),
        ))
    })?;
    for (name, val) in struct_ref.iter() {
        let list = ListRef::from_value(val).ok_or_else(|| {
            starlark::Error::from(Error::SubstitutionValueMustBeList(
                val.get_type().to_owned(),
            ))
        })?;

        let mut items = Vec::new();
        for item in list.iter() {
            let arg_val = item.unpack_frozen().unwrap();
            if let Some(s) = arg_val.to_value().unpack_str() {
                // Safety: This lasts as long as the frozen value does.
                let s_static = unsafe { types::util::extend_lifetime(s) };
                items.push(Either::Left(s_static));
            } else if let Some(args_obj) = FrozenValueTyped::new(arg_val) {
                items.push(Either::Right(args_obj));
            } else {
                return Err(starlark::Error::new_other(Error::InvalidSubstitutionType(
                    arg_val.to_value().get_type().to_owned(),
                )));
            }
        }
        // Safety: This lasts as long as the frozen value does.
        let name_static = unsafe { types::util::extend_lifetime(name.as_str()) };
        substitutions.push((name_static, items));
    }
    Ok(substitutions)
}

impl TargetProviders {
    /// Creates a new set of providers for a pure cxx target (no rule extensions).
    pub fn new_cxx(output_files: Vec<File>, builtins: &BuiltinModule) -> Self {
        let frozen_module =
            Module::with_temp_heap(|module| -> Result<FrozenModule, starlark::Error> {
                let heap = module.heap();
                let default_info = new_defaultinfo(
                    output_files.clone(),
                    builtins.default_info.to_value(),
                    &heap,
                );
                module.set("", default_info);
                Ok(module.freeze()?)
            })
            .unwrap();

        let default_info = frozen_module
            .get("")
            .unwrap()
            .value()
            .unpack_frozen()
            .unwrap();
        let mut map = SmallMap::new();
        map.insert(builtins.default_info_type, default_info);

        Self {
            heap: frozen_module.frozen_heap().clone(),
            providers: map,
            output_files,
            extra_inputs_phony: None,
            substitutions: vec![],
        }
    }

    /// Extracts target providers from a raw list of `UnpackProvider` items, parsing built-ins like `DefaultInfo` and `GnInputsInfo`.
    pub fn new(
        heap: FrozenHeapRef,
        providers: &[UnpackProvider],
        builtins: &BuiltinModule,
    ) -> starlark::Result<Self> {
        let mut map = providers
            .iter()
            .map(|p| (p.0, p.1.unpack_frozen().unwrap()))
            .collect::<SmallMap<_, _>>();

        // Extract output_files from DefaultInfo:
        let output_files = if let Some(default_info) = map.get(&builtins.default_info_type) {
            parse_defaultinfo(default_info.to_value())?
        } else {
            map.insert(builtins.default_info_type, builtins.empty_default_info);
            vec![]
        };

        let extra_inputs_phony = if let Some(value) = map.get(&builtins.gn_inputs_info_type) {
            let unpacked =
                UnpackFileDepset::unpack_value_err(read_builtin_field(value.to_value(), 0))?;
            unpacked.0.cloned()
        } else {
            None
        };

        let substitutions = if let Some(provider_val) = map.get(&builtins.gn_substitution_info_type)
        {
            parse_substitutions(*provider_val)?
        } else {
            vec![]
        };

        Ok(Self {
            heap,
            providers: map,
            output_files,
            extra_inputs_phony,
            substitutions,
        })
    }

    /// Returns the parsed substitutions extracted from `GnSubstitutionInfo` provider.
    pub fn substitutions(
        &self,
    ) -> &[(
        &'static str,
        Vec<Either<&'static str, FrozenValueTyped<'static, ArgsGenFrozen>>>,
    )] {
        &self.substitutions
    }

    /// Retrieves a provider value by its `TypeId`, pinning the value to the heap's lifetime.
    pub fn get<'v>(
        &self,
        type_id: &TypeId,
        heap: &starlark::values::Heap<'v>,
    ) -> Option<Value<'v>> {
        self.providers.get(type_id).map(|frozen| {
            heap.add_reference(&self.heap);
            Value::new_frozen(*frozen)
        })
    }

    /// Returns true if the collection contains a provider of the given `TypeId`.
    pub fn contains_key(&self, type_id: &TypeId) -> bool {
        self.providers.contains_key(type_id)
    }

    /// Returns the target's output files extracted from its `DefaultInfo` provider.
    pub fn output_files(&self) -> &[File] {
        &self.output_files
    }

    /// Returns the phony input file extracted from the target's `GnInputsInfo` provider, if present.
    pub fn extra_inputs_phony(&self) -> Option<&File> {
        self.extra_inputs_phony.as_ref()
    }
}
