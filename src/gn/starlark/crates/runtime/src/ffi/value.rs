// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use starlark::values::list::ListRef;
use starlark::values::{
    FreezeError, FrozenHeapRef, FrozenValue, OwnedFrozenValue, Value as StarlarkVal,
};

use crate::ffi::{Pair, ParseNode, Scope};
use crate::EvalContext;

// Represents a C++ `Value` object.
declare_opaque_type!(pub GnValue);

#[allow(non_camel_case_types)]
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
/// The types of values that can be passed between Rust and C++ GN.
pub enum ValueType {
    NONE,
    BOOLEAN,
    INTEGER,
    STRING,
    LIST,
    SCOPE,
    STARLARK_VALUE,
}

impl GnValue {
    /// Allocates a new C++ `Value` object on the C++ heap.
    pub fn new() -> *mut GnValue {
        extern "C" {
            fn CreateValue() -> *mut GnValue;
        }
        unsafe { CreateValue() }
    }

    /// Frees a C++ `Value` object on the C++ heap.
    pub fn free(val: *mut GnValue) {
        extern "C" {
            fn FreeValue(val: *mut GnValue);
        }
        unsafe {
            FreeValue(val);
        }
    }

    pub fn set_none(&mut self, origin: Option<&ParseNode>) {
        extern "C" {
            fn SetNoneValue(val: &mut GnValue, origin: Option<&ParseNode>);
        }
        unsafe {
            SetNoneValue(self, origin);
        }
    }

    pub fn set_bool(&mut self, origin: Option<&ParseNode>, b: bool) {
        extern "C" {
            fn SetBoolValue(val: &mut GnValue, origin: Option<&ParseNode>, b: bool);
        }
        unsafe {
            SetBoolValue(self, origin, b);
        }
    }

    pub fn set_int(&mut self, origin: Option<&ParseNode>, i: i64) {
        extern "C" {
            fn SetIntValue(val: &mut GnValue, origin: Option<&ParseNode>, i: i64);
        }
        unsafe {
            SetIntValue(self, origin, i);
        }
    }

    pub fn set_string(&mut self, origin: Option<&ParseNode>, s: &str) {
        extern "C" {
            fn SetStringValue(val: &mut GnValue, origin: Option<&ParseNode>, s: &str);
        }
        unsafe {
            SetStringValue(self, origin, s);
        }
    }

    pub fn set_list(&mut self, origin: Option<&ParseNode>) {
        extern "C" {
            fn SetListValue(val: &mut GnValue, origin: Option<&ParseNode>);
        }
        unsafe {
            SetListValue(self, origin);
        }
    }

    pub fn set_starlark_value(
        &mut self,
        origin: Option<&ParseNode>,
        rust_val: *mut OwnedFrozenValue,
    ) {
        extern "C" {
            fn SetStarlarkValue(
                val: &mut GnValue,
                origin: Option<&ParseNode>,
                rust_val: *mut OwnedFrozenValue,
            );
        }
        unsafe {
            SetStarlarkValue(self, origin, rust_val);
        }
    }

    pub fn resize_list(&mut self, size: usize) {
        extern "C" {
            fn ResizeListValue(val: &mut GnValue, size: usize);
        }
        unsafe {
            ResizeListValue(self, size);
        }
    }

    pub fn get_list_value_mut<'a>(&'a mut self, index: usize) -> &'a mut GnValue {
        extern "C" {
            fn GetListValueAt<'a>(val: &'a mut GnValue, index: usize) -> &'a mut GnValue;
        }
        unsafe { GetListValueAt(self, index) }
    }

    pub fn set_list_value_at(&mut self, index: usize, item: &GnValue) {
        extern "C" {
            fn SetListValueAt(val: &mut GnValue, index: usize, item: &GnValue);
        }
        unsafe {
            SetListValueAt(self, index, item);
        }
    }

    pub fn initialize_target_scope(&mut self, parent_scope: Option<&mut Scope>) {
        extern "C" {
            fn InitializeTargetScope(val: &mut GnValue, parent_scope: Option<&mut Scope>);
        }
        unsafe {
            InitializeTargetScope(self, parent_scope);
        }
    }

    pub fn initialize_record_scope(&mut self, parent_scope: Option<&mut Scope>) {
        extern "C" {
            fn InitializeRecordScope(val: &mut GnValue, parent_scope: Option<&mut Scope>);
        }
        unsafe {
            InitializeRecordScope(self, parent_scope);
        }
    }

    pub fn set_scope_value_at<'a>(&'a mut self, key: &str) -> Option<&'a mut Self> {
        extern "C" {
            fn SetScopeValueAt<'a>(
                scope_val: &'a mut GnValue,
                key: &str,
            ) -> Option<&'a mut GnValue>;
        }
        unsafe { SetScopeValueAt(self, key) }
    }

    pub fn collect_to_kwargs<'a>(&self) -> Vec<Pair<&'a str, *const GnValue>> {
        extern "C" {
            fn CollectValueToKwargs<'a>(
                value: &GnValue,
                out: *mut Pair<&'a str, *const GnValue>,
                max_len: usize,
            ) -> usize;
        }
        unsafe {
            let len = CollectValueToKwargs(self, std::ptr::null_mut(), 0);
            let mut out = Vec::with_capacity(len);
            CollectValueToKwargs(self, out.as_mut_ptr(), len);
            out.set_len(len);
            out
        }
    }

    pub fn type_(&self) -> ValueType {
        extern "C" {
            fn GetValueType(val: &GnValue) -> i32;
        }
        match unsafe { GetValueType(self) } {
            0 => ValueType::NONE,
            1 => ValueType::BOOLEAN,
            2 => ValueType::INTEGER,
            3 => ValueType::STRING,
            4 => ValueType::LIST,
            5 => ValueType::SCOPE,
            6 => ValueType::STARLARK_VALUE,
            _ => unreachable!(),
        }
    }

    pub fn boolean_value(&self) -> bool {
        extern "C" {
            fn GetBoolValue(val: &GnValue) -> bool;
        }
        unsafe { GetBoolValue(self) }
    }

    pub fn int_value(&self) -> i64 {
        extern "C" {
            fn GetIntValue(val: &GnValue) -> i64;
        }
        unsafe { GetIntValue(self) }
    }

    pub fn string_value(&self) -> &str {
        extern "C" {
            fn GetStringValue(val: &GnValue) -> &str;
        }
        unsafe { GetStringValue(self) }
    }

    pub fn list_value_len(&self) -> usize {
        extern "C" {
            fn GetListValueLen(val: &GnValue) -> usize;
        }
        unsafe { GetListValueLen(self) }
    }

    pub fn get_list_value(&self, index: usize) -> &GnValue {
        extern "C" {
            fn GetListValueAtConst(val: &GnValue, index: usize) -> &GnValue;
        }
        unsafe { GetListValueAtConst(self, index) }
    }

    pub fn starlark_value(&self) -> *const OwnedFrozenValue {
        extern "C" {
            fn GetStarlarkValue(val: &GnValue) -> *const OwnedFrozenValue;
        }
        unsafe { GetStarlarkValue(self) }
    }
}

declare_opaque_type!(pub(crate) StarlarkValue);

impl StarlarkValue {
    pub fn to_rust(&self) -> &OwnedFrozenValue {
        extern "C" {
            fn GetStarlarkValueInner(val: &StarlarkValue) -> &OwnedFrozenValue;
        }
        unsafe { GetStarlarkValueInner(self) }
    }
}

/// FFI endpoint called from C++ to clone a Starlark `OwnedFrozenValue`.
#[no_mangle]
pub unsafe extern "C" fn clone_starlark_value(
    val: *const OwnedFrozenValue,
) -> *mut OwnedFrozenValue {
    if val.is_null() {
        return std::ptr::null_mut();
    }
    Box::into_raw(Box::new((*val).clone()))
}

/// FFI endpoint called from C++ to format a Starlark `OwnedFrozenValue` into a debug string.
#[no_mangle]
pub unsafe extern "C" fn pretty_starlark_value(
    val: &OwnedFrozenValue,
    mut out: std::pin::Pin<&mut cxx::CxxString>,
) {
    out.as_mut().push_str(&format!("{}", val.value()));
}

/// FFI endpoint called from C++ to convert a native C++ `Value` into a Starlark `OwnedFrozenValue`.
#[no_mangle]
pub unsafe extern "C" fn convert_starlark_value(
    val: *const GnValue,
    session: &crate::session::StarlarkSession,
) -> *mut OwnedFrozenValue {
    if val.is_null() {
        return std::ptr::null_mut();
    }
    let val = &*val;

    let heap = starlark::values::FrozenHeap::new();
    let frozen_val = to_frozen_value(val, session, &heap);
    let heap_ref = heap.into_ref();

    Box::into_raw(Box::new(OwnedFrozenValue::new(heap_ref, frozen_val)))
}

/// FFI endpoint called from C++ to execute a Starlark function with arguments and populate the result into a C++ `Value`.
#[no_mangle]
pub unsafe extern "C" fn call_starlark_value(
    val: &StarlarkValue,
    args_ptr: *const *const OwnedFrozenValue,
    args_len: usize,
    kwargs_ptr: *const Pair<&str, *const OwnedFrozenValue>,
    kwargs_len: usize,
    result: &mut GnValue,
    scope: *mut Scope,
    origin: *const ParseNode,
    err: &mut crate::ffi::Err,
) {
    let val = val.to_rust();

    let source_dir = (&*scope).source_dir();
    let caller_pkg = source_dir.as_rust().to_owned();

    let extra = EvalContext::new(scope, origin, crate::EvalKind::Macro(caller_pkg));

    let args = if args_len == 0 {
        &[]
    } else {
        std::slice::from_raw_parts(args_ptr, args_len)
    };
    let kwargs = if kwargs_len == 0 {
        &[]
    } else {
        std::slice::from_raw_parts(kwargs_ptr, kwargs_len)
    };

    let args_refs: Vec<&OwnedFrozenValue> = args.iter().map(|&p| unsafe { &*p }).collect();

    let kwargs_refs: Vec<(&str, &OwnedFrozenValue)> = kwargs
        .iter()
        .map(|pair| (pair.first, unsafe { &*pair.second }))
        .collect();

    let res = crate::session::invoke_starlark_function(val, &args_refs, &kwargs_refs, &extra);

    crate::ffi::handle_result(
        err,
        origin,
        res.map(|v| {
            to_cxx_value(v.value(), result, val.owner(), &extra).unwrap();
        }),
    );
}

/// Converts a C++ `GnValue` to a frozen Starlark `FrozenValue`, allocating inside a Starlark frozen heap.
pub fn to_frozen_value(
    val: &GnValue,
    session: &crate::session::StarlarkSession,
    frozen_heap: &starlark::values::FrozenHeap,
) -> FrozenValue {
    unsafe {
        match val.type_() {
            ValueType::NONE => FrozenValue::new_none(),
            ValueType::BOOLEAN => FrozenValue::new_bool(val.boolean_value()),
            ValueType::INTEGER => frozen_heap.alloc(val.int_value()),
            ValueType::STRING => frozen_heap.alloc(val.string_value()),
            ValueType::LIST => {
                let len = val.list_value_len();
                let mut list_vals = Vec::new();
                for i in 0..len {
                    let item = val.get_list_value(i);
                    list_vals.push(to_frozen_value(item, session, frozen_heap));
                }
                frozen_heap.alloc(list_vals)
            }
            ValueType::SCOPE => {
                let entries = val.collect_to_kwargs();
                let struct_fields: Vec<(String, FrozenValue)> = entries
                    .iter()
                    .map(|item| {
                        let key = item.first.to_string();
                        let value = &*item.second;
                        let rust_val = to_frozen_value(value, session, frozen_heap);
                        (key, rust_val)
                    })
                    .collect();
                let struct_fields_refs: Vec<(&str, FrozenValue)> = struct_fields
                    .iter()
                    .map(|(k, v): &(String, FrozenValue)| (k.as_str(), *v))
                    .collect();
                let struct_val = starlark::values::structs::AllocStruct(struct_fields_refs);
                frozen_heap.alloc(struct_val)
            }
            ValueType::STARLARK_VALUE => {
                let rust_val = &*val.starlark_value();
                frozen_heap.add_reference(rust_val.owner());
                rust_val.unchecked_frozen_value()
            }
        }
    }
}

/// Populates a C++ `GnValue` from a Starlark value, handling deep structures and registering heaps appropriately.
pub fn to_cxx_value<'v>(
    val: StarlarkVal<'v>,
    out: &mut GnValue,
    heap_ref: &FrozenHeapRef,
    extra: &EvalContext,
) -> Result<(), FreezeError> {
    let origin = unsafe { extra.origin.as_ref() };
    if let Some(s) = val.unpack_str() {
        out.set_string(origin, s);
    } else if let Some(b) = val.unpack_bool() {
        out.set_bool(origin, b);
    } else if let Some(i) = val.unpack_i32() {
        out.set_int(origin, i as i64);
    } else if let Some(l) = ListRef::from_value(val) {
        out.set_list(origin);
        out.resize_list(l.len());
        for (i, item) in l.iter().enumerate() {
            let item_ref = out.get_list_value_mut(i);
            to_cxx_value(item, item_ref, heap_ref, extra)?;
        }
    } else if let Some(record_ref) = starlark::values::record::Record::from_value(val) {
        out.initialize_record_scope(unsafe { extra.scope.as_mut() });
        for (k, v) in record_ref.iter() {
            if let Some(value_ref) = out.set_scope_value_at(k) {
                to_cxx_value(v, value_ref, heap_ref, extra)?;
            }
        }
    } else if let Some(struct_ref) = starlark::values::structs::StructRef::from_value(val) {
        out.initialize_record_scope(unsafe { extra.scope.as_mut() });
        for (k, v) in struct_ref.iter() {
            if let Some(value_ref) = out.set_scope_value_at(k.as_str()) {
                to_cxx_value(v, value_ref, heap_ref, extra)?;
            }
        }
    } else {
        let frozen_val = val.unpack_frozen().unwrap();
        let owned = unsafe { OwnedFrozenValue::new(heap_ref.clone(), frozen_val) };
        let rust_val = Box::into_raw(Box::new(owned));
        out.set_starlark_value(origin, rust_val);
    }
    Ok(())
}

/// FFI endpoint called from C++ to deallocate a Starlark `OwnedFrozenValue`.
#[no_mangle]
pub unsafe extern "C" fn free_starlark_value(val: *mut OwnedFrozenValue) {
    if !val.is_null() {
        drop(Box::from_raw(val));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Assert;

    fn test_round_trip(expr: &str, want: Option<&str>, want_inverted: Option<&str>) {
        let want = want.unwrap_or(expr);
        let want_inverted = want_inverted.unwrap_or(want);

        let a = Assert::new();
        let val = a.pass(expr);
        let heap_ref = val.owner().clone();

        let out = GnValue::new();

        assert_eq!(format!("{}", val.value()), want);
        unsafe {
            to_cxx_value(val.value(), &mut *out, &heap_ref, a.context()).unwrap();
            let frozen_heap = starlark::values::FrozenHeap::new();
            let inverted = to_frozen_value(&*out, a.session(), &frozen_heap);
            assert_eq!(format!("{}", inverted.to_value()), want_inverted);
            GnValue::free(out);
        }
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
            Some("record[DefaultInfo](files=[], executable=None)"),
            Some("struct(executable=None, files=[])"),
        );
    }
}
