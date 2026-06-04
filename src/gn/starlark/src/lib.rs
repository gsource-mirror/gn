pub mod loader;

use crate::loader::FileLoader;
use starlark::environment::{FrozenModule, Globals, Module};
use starlark::eval::Evaluator;
use starlark::values::list::ListRef;
use starlark::values::{FrozenHeapRef, OwnedFrozenValue, FreezeError};
use std::path::PathBuf;
use std::pin::Pin;
use cxx::CxxString;

pub struct StarlarkFrozenModule(pub FrozenModule);

pub struct StarlarkOpaqueValue(pub OwnedFrozenValue);

fn to_cxx_value(
    val: starlark::values::Value,
    mut out: Pin<&mut ffi::Value>,
    origin: *const ffi::ParseNode,
    heap_ref: &FrozenHeapRef,
) -> Result<(), FreezeError> {
    // all ffi methods are unsafe.
    unsafe {
        match val.get_type() {
            "bool" => ffi::set_bool_value(out.as_mut(), origin, val.unpack_bool().unwrap()),
            "int" => ffi::set_int_value(out.as_mut(), origin, val.unpack_i32().unwrap() as i64),
            "string" => ffi::set_string_value(out.as_mut(), origin, val.unpack_str().unwrap()),
            "list" => {
                if let Some(l) = ListRef::from_value(val) {
                    ffi::set_list_value_size(out.as_mut(), origin, l.len());
                    for (i, item) in l.iter().enumerate() {
                        to_cxx_value(item, ffi::get_list_index(out.as_mut(), i), origin, heap_ref)?;
                    }
                } else {
                    unreachable!()
                }
            }
            _ => {
                let frozen_val = val.unpack_frozen().unwrap();
                let owned = OwnedFrozenValue::new(heap_ref.clone(), frozen_val);
                ffi::set_starlark_value(out.as_mut(), origin, Box::new(StarlarkOpaqueValue(owned)));
            }
        }
    }
    Ok(())
}

fn to_rust_value<'v>(
    val: &ffi::Value,
    heap: starlark::values::Heap<'v>,
) -> starlark::values::Value<'v> {
    match ffi::get_value_kind(val) {
        ffi::ValueKind::NONE => starlark::values::Value::new_none(),
        ffi::ValueKind::BOOLEAN => starlark::values::Value::new_bool(ffi::get_bool_value(val)),
        ffi::ValueKind::INTEGER => heap.alloc(ffi::get_int_value(val)),
        ffi::ValueKind::STRING => heap.alloc(ffi::get_string_value(val)),
        ffi::ValueKind::LIST => {
            let size = ffi::get_list_size(val);
            let mut list = Vec::with_capacity(size);
            for i in 0..size {
                list.push(to_rust_value(ffi::get_list_value_index(val, i), heap));
            }
            heap.alloc(list)
        }
        ffi::ValueKind::STARLARK_VALUE => {
            let opaque = ffi::get_starlark_value(val);
            heap.access_owned_frozen_value(&opaque.0)
        }
        ffi::ValueKind::SCOPE => {
            let mut entries = Vec::new();
            ffi::get_scope_values(val, &mut entries);

            heap.alloc(starlark::values::structs::AllocStruct(
                entries.iter().map(|kv| (kv.key, to_rust_value(kv.value, heap)))))
        }
        _ => unreachable!(),
    }
}

#[cxx::bridge(namespace = "starlark_ffi")]
mod ffi {
    enum ValueKind {
        NONE = 0,
        BOOLEAN,
        INTEGER,
        STRING,
        LIST,
        SCOPE,
        STARLARK_VALUE,
    }

    unsafe extern "C++" {
        include!("gn/value.h");
        include!("gn/starlark_values.h");

        #[namespace = ""]
        type Value;
        #[namespace = ""]
        type ParseNode;

        unsafe fn set_bool_value(val: Pin<&mut Value>, origin: *const ParseNode, b: bool);
        unsafe fn set_int_value(val: Pin<&mut Value>, origin: *const ParseNode, n: i64);
        unsafe fn set_string_value(val: Pin<&mut Value>, origin: *const ParseNode, s: &str);
        unsafe fn set_list_value_size(val: Pin<&mut Value>, origin: *const ParseNode, size: usize);
        fn get_list_index(val: Pin<&mut Value>, index: usize) -> Pin<&mut Value>;
        unsafe fn set_starlark_value(val: Pin<&mut Value>, origin: *const ParseNode, func: Box<StarlarkOpaqueValue>);

        fn get_value_kind(val: &Value) -> ValueKind;
        fn get_bool_value(val: &Value) -> bool;
        fn get_int_value(val: &Value) -> i64;
        fn get_string_value(val: &Value) -> &str;
        fn get_list_size(val: &Value) -> usize;
        fn get_list_value_index(val: &Value, index: usize) -> &Value;
        fn get_starlark_value(val: &Value) -> &StarlarkOpaqueValue;
        fn get_scope_values(val: &Value, out: &mut Vec<KeyValuePair>);
    }

    struct KeyValuePair<'a> {
        key: &'a str,
        value: &'a Value,
    }

    extern "Rust" {
        type FileLoader;
        type StarlarkFrozenModule;
        type StarlarkOpaqueValue;

        fn new_file_loader(path: &str) -> Box<FileLoader>;
        fn load(
            file_loader: &FileLoader,
            path: &str,
            error: Pin<&mut CxxString>,
        ) -> *const StarlarkFrozenModule;
        unsafe fn free_frozen_module(module: *const StarlarkFrozenModule);
        unsafe fn get_value_from_module(
            module: &StarlarkFrozenModule,
            name: &str,
            err: Pin<&mut CxxString>,
            out: Pin<&mut Value>,
            origin: *const ParseNode,
        );
        fn starlark_value_to_string(val: &StarlarkOpaqueValue) -> String;
        fn clone_starlark_value(val: &StarlarkOpaqueValue) -> Box<StarlarkOpaqueValue>;
        unsafe fn call_starlark_function(
            func: &StarlarkOpaqueValue,
            origin: *const ParseNode,
            args: &Value,
            kwargs: &Value,
            err: Pin<&mut CxxString>,
            out: Pin<&mut Value>,
        );
    }
}

fn new_file_loader(path: &str) -> Box<FileLoader> {
    Box::new(FileLoader::new(PathBuf::from(path)))
}

fn load(file_loader: &FileLoader, path: &str, mut error: Pin<&mut CxxString>) -> *const StarlarkFrozenModule {
    match file_loader.load(path, "") {
        Ok(module) => Box::into_raw(Box::new(StarlarkFrozenModule(module))),
        Err(e) => {
            error.as_mut().clear();
            error.as_mut().push_str(&e.to_string());
            std::ptr::null()
        }
    }
}

fn get_value_from_module(
    module: &StarlarkFrozenModule,
    name: &str,
    mut err: Pin<&mut CxxString>,
    out: Pin<&mut ffi::Value>,
    origin: *const ffi::ParseNode,
) {
    match module.0.get(name) {
        Ok(v) => {
            to_cxx_value(v.value(), out, origin, v.owner());
        }
        Err(e) => {
            err.as_mut().push_str(&e.to_string());
        }
    }
}

fn free_frozen_module(module: *const StarlarkFrozenModule) {
    if !module.is_null() {
        unsafe {
            let _ = Box::from_raw(module as *mut StarlarkFrozenModule);
        }
    }
}

fn starlark_value_to_string(val: &StarlarkOpaqueValue) -> String {
    val.0.value().to_repr()
}

fn clone_starlark_value(val: &StarlarkOpaqueValue) -> Box<StarlarkOpaqueValue> {
    Box::new(StarlarkOpaqueValue(val.0.clone()))
}

unsafe fn call_starlark_function(
    func: &StarlarkOpaqueValue,
    origin: *const ffi::ParseNode,
    args: &ffi::Value,
    kwargs: &ffi::Value,
    mut err: Pin<&mut CxxString>,
    mut out: Pin<&mut ffi::Value>,
) {
    let res = Module::with_temp_heap(|module| {
        let rust_args = to_rust_value(args, module.heap());
        let list_ref = ListRef::from_value(rust_args).unwrap();
        let pos_args: Vec<starlark::values::Value> = list_ref.iter().collect();

        let mut rust_kwargs = Vec::new();
        ffi::get_scope_values(kwargs, &mut rust_kwargs);
        let mut named_args = Vec::new();
        for kv in rust_kwargs.iter() {
            named_args.push((kv.key, to_rust_value(kv.value, module.heap())));
        }

        let starlark_func = module.heap().access_owned_frozen_value(&func.0);
        let v = {
            let mut eval = Evaluator::new(&module);
            eval.eval_function(starlark_func, &pos_args, &named_args)?
        };
        // There's no public API to freeze a value directly, so we just fereze this module we created.
        module.set("", v);
        let frozen_module = module.freeze().map_err(|e| starlark::Error::new_other(anyhow::anyhow!(e)))?;
        let result_value = frozen_module.get("").map_err(|e| starlark::Error::new_other(e))?;
        Ok::<OwnedFrozenValue, starlark::Error>(result_value)
    });

    match res {
        Ok(result_value) => {
            if let Err(e) = to_cxx_value(result_value.value(), out, origin, result_value.owner()) {
                err.as_mut().push_str(&e.err_msg);
            }
        }
        Err(e) => {
            err.as_mut().push_str(&e.to_string());
        }
    }
}