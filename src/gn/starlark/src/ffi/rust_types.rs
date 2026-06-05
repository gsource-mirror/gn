// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::pin::Pin;

use crate::ffi::bindings as ffi;
use crate::session::StarlarkSession;
use starlark::values::OwnedFrozenValue;

pub(crate) fn move_constructor<T, N: autocxx::moveit::new::New<Output = T>>(
    mut dest: Pin<&mut T>,
    constructor: N,
) -> Pin<&mut T> {
    unsafe {
        let raw_ptr = dest.as_mut().get_unchecked_mut() as *mut T;
        // Specifically does not call the old destructor by design.
        let uninit_dest = Pin::new_unchecked(&mut *(raw_ptr as *mut std::mem::MaybeUninit<T>));
        constructor.new(uninit_dest);
        Pin::new_unchecked(&mut *raw_ptr)
    }
}

pub unsafe fn extend_lifetime<'a, T>(value: Pin<&'a T>) -> Pin<&'static T> {
    Pin::new_unchecked(&*(value.get_ref() as *const T))
}

pub unsafe fn extend_lifetime_mut<'a, T>(value: Pin<&'a mut T>) -> Pin<&'static mut T> {
    std::mem::transmute(value)
}

// AsRust casts C++ to rust
pub trait AsRust {
    type Target;
    fn as_rust(self) -> Self::Target;
}

// AsCxx casts rust to C++
pub trait AsCxx {
    type Target;
    fn as_cxx(self) -> Self::Target;
}

// IntoRust takes ownership of the C++ value.
pub trait IntoRust {
    type Target;
    fn into_rust(self) -> Self::Target;
}

// IntoCxx releases ownership of the rust value to C++
pub trait IntoCxx {
    type Target;
    fn into_cxx(self) -> Self::Target;
}

// ToRust makes a copy of the C++ value in rust
pub trait ToRust<T> {
    fn to_rust(&self) -> T;
}

// ToCxx makes a copy of the rust value in C++
pub trait ToCxx<T> {
    fn to_cxx(&self) -> T;
}

/// Registers a type that is opaque to C++.
macro_rules! register_cxx_opaque_type {
    ($rust_type:ty, $cxx_type:ty) => {
        impl<'a> AsRust for &'a $cxx_type {
            type Target = &'static $rust_type;
            fn as_rust(self) -> &'static $rust_type {
                unsafe { &*(self as *const $cxx_type as *const $rust_type) }
            }
        }
        impl<'a> AsRust for &'a mut $cxx_type {
            type Target = &'static mut $rust_type;
            fn as_rust(self) -> &'static mut $rust_type {
                unsafe { &mut *(self as *mut $cxx_type as *mut $rust_type) }
            }
        }
        impl AsRust for *const $cxx_type {
            type Target = Option<&'static $rust_type>;
            fn as_rust(self) -> Option<&'static $rust_type> {
                if self.is_null() {
                    None
                } else {
                    Some(unsafe { &*(self as *const $rust_type) })
                }
            }
        }
        impl AsRust for *mut $cxx_type {
            type Target = Option<&'static mut $rust_type>;
            fn as_rust(self) -> Option<&'static mut $rust_type> {
                if self.is_null() {
                    None
                } else {
                    Some(unsafe { &mut *(self as *mut $rust_type) })
                }
            }
        }
        impl<'a> AsRust for Pin<&'a $cxx_type> {
            type Target = Pin<&'static $rust_type>;
            fn as_rust(self) -> Pin<&'static $rust_type> {
                unsafe { Pin::new_unchecked(&*(self.get_ref() as *const $cxx_type as *const $rust_type)) }
            }
        }
        impl<'a> AsCxx for Pin<&'a $rust_type> {
            type Target = &'a $cxx_type;
            fn as_cxx(self) -> &'a $cxx_type {
                unsafe { &*(self.get_ref() as *const $rust_type as *const $cxx_type) }
            }
        }
        impl<'a> AsCxx for Option<Pin<&'a $rust_type>> {
            type Target = *const $cxx_type;
            fn as_cxx(self) -> *const $cxx_type {
                match self {
                    None => std::ptr::null(),
                    Some(pin) => pin.get_ref() as *const $rust_type as *const $cxx_type,
                }
            }
        }
        impl<'a> AsRust for Pin<&'a mut $cxx_type> {
            type Target = Pin<&'static mut $rust_type>;
            fn as_rust(mut self) -> Pin<&'static mut $rust_type> {
                unsafe {
                    let raw_ptr = self.as_mut().get_unchecked_mut() as *mut $cxx_type as *mut $rust_type;
                    Pin::new_unchecked(&mut *raw_ptr)
                }
            }
        }
        impl IntoRust for *mut $cxx_type {
            type Target = Option<Box<$rust_type>>;
            fn into_rust(self) -> Option<Box<$rust_type>> {
                if self.is_null() {
                    None
                } else {
                    Some(unsafe { Box::from_raw(self as *mut $rust_type) })
                }
            }
        }
        impl IntoCxx for Box<$rust_type> {
            type Target = *mut $cxx_type;
            fn into_cxx(self) -> *mut $cxx_type {
                Box::into_raw(self) as *mut $cxx_type
            }
        }
    };
}

register_cxx_opaque_type!(StarlarkSession, ffi::rust::StarlarkSession);
register_cxx_opaque_type!(OwnedFrozenValue, ffi::rust::OwnedFrozenValue);
register_cxx_opaque_type!(starlark::environment::FrozenModule, ffi::rust::StarlarkModule);
register_cxx_opaque_type!(crate::target::Target, ffi::rust::RustTarget);

impl ToRust<crate::file::File> for crate::ffi::OutputFile {
    fn to_rust(&self) -> crate::file::File {
        let path_str: &str = &*crate::ffi::GetOutputFilePath(self);
        // Safety: This lifetime is managed by C++.
        let static_str: &'static str = unsafe { crate::util::extend_lifetime(path_str) };
        crate::file::File(std::path::Path::new(static_str))
    }
}

impl ToRust<crate::target::TargetRef> for crate::ffi::Target {
    fn to_rust(&self) -> crate::target::TargetRef {
        let rust_target_ptr = unsafe { crate::ffi::GetTargetStarlarkTarget(self as *const crate::ffi::Target) };
        let rust_target = rust_target_ptr.as_rust().unwrap();
        crate::target::TargetRef(rust_target)
    }
}


impl ToRust<crate::target::TargetRef> for crate::ffi::TargetPtr {
    fn to_rust(&self) -> crate::target::TargetRef {
        let target_ref = unsafe { &*(self.ptr as *const crate::ffi::Target) };
        target_ref.to_rust()
    }
}

impl ToRust<&'static str> for crate::ffi::RustStrWrapper {
    fn to_rust(&self) -> &'static str {
        unsafe {
            let slice = std::slice::from_raw_parts(self.ptr as *const u8, self.len);
            let s_str = std::str::from_utf8_unchecked(slice);
            crate::util::extend_lifetime(s_str)
        }
    }
}