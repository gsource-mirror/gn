// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::pin::Pin;
use starlark::environment::Module;
use starlark::eval::Evaluator;
use starlark::values::{FreezeError, FrozenHeapRef, OwnedFrozenValue};
use starlark::values::list::ListRef;
use autocxx::prelude::*;

use crate::ffi::bindings as ffi;
use crate::ffi::{AsRust, IntoCxx, IntoRust};
use crate::ffi::rust_types::move_constructor;
use crate::ffi::{ResizeListValue, SetListValueAt};
use crate::session::EvalContext;

#[no_mangle]
pub unsafe extern "C" fn clone_starlark_value(
    val: *const ffi::rust::OwnedFrozenValue,
) -> *mut ffi::rust::OwnedFrozenValue {
    val.as_rust()
        .map(|rust_val| Box::new(rust_val.clone()).into_cxx())
        .unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub unsafe extern "C" fn free_starlark_value(val: *mut ffi::rust::OwnedFrozenValue) {
    let _: Option<Box<OwnedFrozenValue>> = val.into_rust();
}

#[no_mangle]
pub unsafe extern "C" fn pretty_starlark_value(
    val: &OwnedFrozenValue,
    mut out: Pin<&mut cxx::CxxString>,
) {
    out.as_mut().push_str(&format!("{}", val.value()));
}

#[no_mangle]
pub unsafe extern "C" fn convert_starlark_value(
    val: *const ffi::Value,
    session: &ffi::rust::StarlarkSession,
) -> *mut ffi::rust::OwnedFrozenValue {
    if val.is_null() {
        return std::ptr::null_mut();
    }
    let val = &*val;

    let heap = starlark::values::FrozenHeap::new();
    let frozen_val = to_frozen_value(val, session, &heap);
    let heap_ref = heap.into_ref();

    Box::new(OwnedFrozenValue::new(heap_ref, frozen_val)).into_cxx()
}

#[no_mangle]
pub unsafe extern "C" fn convert_target(
    target: *mut ffi::Target,
    rule: &ffi::rust::OwnedFrozenValue,
    session: &ffi::rust::StarlarkSession,
) -> *mut ffi::rust::OwnedFrozenValue {
    if target.is_null() {
        return std::ptr::null_mut();
    }
    let rust_session = session.as_rust();
    let non_null_target = std::ptr::NonNull::new(target).expect("target cannot be null");
    
    let rule_ptr = rule as *const ffi::rust::OwnedFrozenValue;
    let rule_rust = rule_ptr.as_rust().expect("rule cannot be null");
    let rule_val = rule_rust.value();
    let rule_frozen = rule_val.unpack_frozen().expect("rule must be frozen");
    let rule_typed = starlark::values::FrozenValueTyped::new(rule_frozen).unwrap();
    let rust_target = crate::target::Target::new_starlark(non_null_target, rule_typed, starlark::collections::SmallMap::new());
    let target_ref = rust_session.register_target(rust_target);

    let heap = starlark::values::FrozenHeap::new();
    let frozen_val = heap.alloc(target_ref);
    let heap_ref = heap.into_ref();

    Box::new(OwnedFrozenValue::new(heap_ref, frozen_val)).into_cxx()
}

#[no_mangle]
pub unsafe extern "C" fn new_native_gn_target(
    session: &ffi::rust::StarlarkSession,
    target: *mut ffi::Target,
) -> *mut ffi::rust::RustTarget {
    if target.is_null() {
        return std::ptr::null_mut();
    }
    let rust_session = session.as_rust();
    let non_null_target = std::ptr::NonNull::new(target).unwrap();
    let rust_target = crate::target::Target::new_cxx(non_null_target);
    let target_ref = rust_session.register_target(rust_target);

    target_ref.0 as *const crate::target::Target as *mut crate::target::Target as *mut ffi::rust::RustTarget
}

#[no_mangle]
pub unsafe extern "C" fn call_starlark_value(
    val: &ffi::StarlarkValue,
    args: &cxx::CxxVector<ffi::StarlarkValue>,
    kwargs: &cxx::CxxVector<ffi::KeyVal>,
    result: &mut ffi::Value,
    scope: *mut ffi::Scope,
    origin: *const ffi::ParseNode,
    err: std::pin::Pin<&mut ffi::Err>,
) {
    let val = val.to_rust().as_rust();
    let session_ptr = unsafe { crate::ffi::GetStarlarkSessionFromScope(scope) };
    let ffi_session = unsafe { &*session_ptr };
    use crate::ffi::AsRust;
    let rust_session = ffi_session.as_rust();
    let result = Pin::new_unchecked(result);

    let source_dir_ptr = unsafe { crate::ffi::GetScopeSourceDir(scope) };
    let source_dir = unsafe { &*source_dir_ptr };
    let mut caller_pkg_str = source_dir.value().to_str().unwrap().to_owned();
    if caller_pkg_str.ends_with('/') && caller_pkg_str.len() > 2 {
        caller_pkg_str.pop();
    }
    let caller_pkg = crate::label::Package(caller_pkg_str);

    let extra = EvalContext::new(
        scope,
        origin,
        crate::session::EvalKind::BuildFile(caller_pkg),
    );

    let res = Module::with_temp_heap(|module| -> Result<(), String> {

        let eval_result = {
            let mut eval = Evaluator::new(&module);
            eval.extra = Some(&extra);
            let heap = module.heap();

            let convert_val = |inner_ptr: *const ffi::rust::OwnedFrozenValue| {
                let rust_val: &OwnedFrozenValue = inner_ptr.as_rust().unwrap();
                let val_v = rust_val.value();
                heap.add_reference(rust_val.owner());
                val_v
            };

            let positional_args: Vec<starlark::values::Value> = args
                .iter()
                .map(|item| convert_val(item.to_rust()))
                .collect();

            let named_args: Vec<(String, starlark::values::Value)> = kwargs
                .iter()
                .map(|item| (item.key().to_string(), convert_val(item.value() as *const ffi::rust::OwnedFrozenValue)))
                .collect();
            let named_args_slices: Vec<(&str, starlark::values::Value)> =
                named_args.iter().map(|(k, v): &(String, starlark::values::Value)| (k.as_str(), *v)).collect();



            let val_v = val.value();
            module.heap().add_reference(val.owner());
            let r = eval.eval_function(
                val_v,
                &positional_args,
                &named_args_slices,
            )
            .map_err(|e| format!("{}", e));
            r
        };

        match &eval_result {
            Ok(res) => {
                module.set("__result__", *res);

                if module.extra_value().is_none() {
                    module.set_extra_value(module.heap().alloc(starlark::values::list::AllocList::EMPTY));
                }

                let frozen_module = module.freeze().map_err(|e| format!("{:?}", e))?;

                crate::util::register_targets_from_module(rust_session, &frozen_module);

                let frozen_res = frozen_module.get("__result__").unwrap();
                let heap_ref = frozen_module.owned_extra_value().unwrap().owner().clone();


                to_cxx_value(
                    frozen_res.value(),
                    result,
                    &heap_ref,
                    &extra,
                )
                .map_err(|e| e.err_msg)?;
            }
            Err(e) => {
                return Err(e.clone());
            }
        }
        Ok(())
    });

    let _: () = crate::ffi::handle_result(err, origin, res);
}



fn to_frozen_value(
    val: &ffi::Value,
    session: &ffi::rust::StarlarkSession,
    frozen_heap: &starlark::values::FrozenHeap,
) -> starlark::values::FrozenValue {
    unsafe {
        match val.type_() {
            ffi::Value_Type::NONE => starlark::values::FrozenValue::new_none(),
            ffi::Value_Type::BOOLEAN => starlark::values::FrozenValue::new_bool(*val.boolean_value1()),
            ffi::Value_Type::INTEGER => frozen_heap.alloc(*val.int_value1()),
            ffi::Value_Type::STRING => frozen_heap.alloc(val.string_value1().to_str().unwrap()),
            ffi::Value_Type::LIST => {
                let list_cxx = val.list_value1();
                let mut list_vals = Vec::new();
                for item in list_cxx.iter() {
                    list_vals.push(to_frozen_value(item, session, frozen_heap));
                }
                frozen_heap.alloc(list_vals)
            }
            ffi::Value_Type::SCOPE => {
                let entries = ffi::collect_value_to_kwargs(val, session);
                let struct_fields: Vec<(String, starlark::values::FrozenValue)> = entries
                    .iter()
                    .map(|item| {
                        let key = item.key().to_string();
                        let value = item.value();
                        let rust_val = value.as_rust();
                        frozen_heap.add_reference(rust_val.owner());
                        (key, rust_val.unchecked_frozen_value())
                    })
                    .collect();
                let struct_fields_refs: Vec<(&str, starlark::values::FrozenValue)> = struct_fields
                    .iter()
                    .map(|(k, v): &(String, starlark::values::FrozenValue)| (k.as_str(), *v))
                    .collect();
                let struct_val = starlark::values::structs::AllocStruct(struct_fields_refs);
                frozen_heap.alloc(struct_val)
            }
            ffi::Value_Type::STARLARK_VALUE => {
                let rust_val_inner = val.starlark_value().to_rust();
                let rust_val = rust_val_inner.as_rust();
                frozen_heap.add_reference(rust_val.owner());
                rust_val.unchecked_frozen_value()
            }
        }
    }
}

pub(crate) fn to_cxx_value<'v>(
    val: starlark::values::Value<'v>,
    mut out: Pin<&mut ffi::Value>,
    heap_ref: &FrozenHeapRef,
    extra: &EvalContext,
) -> Result<(), FreezeError> {
    // Put string first because it's the most likely type
    if let Some(s) = val.unpack_str() {
        move_constructor(
            out.as_mut(),
            unsafe { ffi::Value::new5(extra.origin, s) },
        );
    } else if let Some(b) = val.unpack_bool() {
        move_constructor(
            out.as_mut(),
            unsafe { ffi::Value::new2(extra.origin, b) },
        );
    } else if let Some(i) = val.unpack_i32() {
        move_constructor(
            out.as_mut(),
            unsafe { ffi::Value::new3(extra.origin, i as i64) },
        );
    } else if let Some(l) = ListRef::from_value(val) {
        move_constructor(
            out.as_mut(),
            unsafe { ffi::Value::new1(extra.origin, ffi::Value_Type::LIST) },
        );
        ResizeListValue(out.as_mut(), l.len());
        for (i, item) in l.iter().enumerate() {
            let item_cxx_ctor = ffi::Value::new();
            moveit!(let mut item_cxx = item_cxx_ctor);
            to_cxx_value(item, item_cxx.as_mut(), heap_ref, extra)?;
            SetListValueAt(out.as_mut(), i, item_cxx.as_ref().get_ref());
        }
    } else if let Some(record_ref) = starlark::values::record::Record::from_value(val) {
        unsafe { ffi::InitializeRecordScope(out.as_mut(), extra.scope); }
        for (k, v) in record_ref.iter() {
            let value_ptr = ffi::SetScopeValueAt(out.as_mut(), k.into());
            let value_pin = unsafe { Pin::new_unchecked(&mut *value_ptr) };
            to_cxx_value(v, value_pin, heap_ref, extra)?;
        }
    } else if let Some(struct_ref) = starlark::values::structs::StructRef::from_value(val) {
        unsafe { ffi::InitializeRecordScope(out.as_mut(), extra.scope); }
        for (k, v) in struct_ref.iter() {
            let value_ptr = ffi::SetScopeValueAt(out.as_mut(), k.as_str().into());
            let value_pin = unsafe { Pin::new_unchecked(&mut *value_ptr) };
            to_cxx_value(v, value_pin, heap_ref, extra)?;
        }
    } else {
        let frozen_val = val.unpack_frozen().unwrap();
        let owned = unsafe { OwnedFrozenValue::new(heap_ref.clone(), frozen_val) };
        let rust_val = Box::new(owned);
        let cpp_val_ptr = rust_val.into_cxx();

        let temp_val_ctor = unsafe { ffi::StarlarkValue::new(cpp_val_ptr) };
        moveit!(let temp_val = temp_val_ctor);

        unsafe {
            move_constructor(
                out.as_mut(),
                ffi::Value::new8(extra.origin, temp_val.as_ref().get_ref()),
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::eval_starlark;
    use crate::ffi::rust_types::AsCxx as _;

    fn test_round_trip(expr: &str, want: Option<&str>, want_inverted: Option<&str>) {
        let want = want.unwrap_or(expr);
        let want_inverted = want_inverted.unwrap_or(want);
        let setup = crate::ffi::TestWithScope::new();

        let scope_ptr = setup.scope();
        let extra = EvalContext::new(
            scope_ptr,
            std::ptr::null(),
            crate::session::EvalKind::BzlFile(crate::label::Package("//".to_owned())),
        );

        let val = eval_starlark(expr).unwrap();
        let heap_ref = val.owner().clone();

        let mut out = ffi::Value::new().within_unique_ptr();
        let testdata_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("testdata");
        let session = Box::pin(crate::session::StarlarkSession::new(testdata_path, std::path::PathBuf::from("../..")));

        assert_eq!(format!("{}", val.value()), want);
        to_cxx_value(val.value(), out.pin_mut(), &heap_ref, &extra).unwrap();

        let frozen_heap = starlark::values::FrozenHeap::new();
        let inverted =
            to_frozen_value(out.as_ref().unwrap(), session.as_ref().as_cxx(), &frozen_heap);

        assert_eq!(format!("{}", inverted.to_value()), want_inverted);
    }

    #[test]
    fn test_round_trip_integer() {
        test_round_trip("42", None, None);
    }

    #[test]
    fn test_round_trip_bool() {
        test_round_trip("True", None, None);
        test_round_trip("False", None, None);
    }

    #[test]
    fn test_round_trip_string() {
        test_round_trip("\"hello\"", None, None);
    }

    #[test]
    fn test_round_trip_list() {
        test_round_trip("[1, True, \"test\"]", None, None);
    }

    #[test]
    fn test_round_trip_struct() {
        test_round_trip("struct(a=42, b=\"hello\", c=[1, 2])", None, None);
    }

    #[test]
    fn test_round_trip_provider() {
        test_round_trip(
            "DefaultInfo(executable = None, files = [])",
            Some("record[DefaultInfo](executable=None, files=[])"),
            Some("struct(executable=None, files=[])"),
        );
    }
}
