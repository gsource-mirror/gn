// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use starlark::values::{FrozenValueTyped, OwnedFrozenValue};

use crate::ffi::err::Err;
use crate::ffi::label::Label;
use crate::ffi::output_file::OutputFile;
use crate::ffi::value::GnValue;
use crate::ffi::ParseNode;

declare_opaque_type!(pub(crate) Target);

impl Target {
    pub fn create<'a>(
        target_type: &str,
        target_name: &str,
        origin: Option<&ParseNode>,
        kwargs_scope_val: &'a mut GnValue,
        err: &mut Err,
    ) -> Option<&'a mut Self> {
        extern "C" {
            fn CreateTarget<'a>(
                target_type: &str,
                target_name: &str,
                origin: Option<&ParseNode>,
                kwargs_scope_val: &'a mut GnValue,
                err: &mut Err,
            ) -> Option<&'a mut Target>;
        }
        unsafe { CreateTarget(target_type, target_name, origin, kwargs_scope_val, err) }
    }

    pub fn add_starlark_target_dependency(
        &mut self,
        dep_dir: &str,
        dep_name: &str,
        toolchain_dir: &str,
        toolchain_name: &str,
    ) {
        extern "C" {
            fn AddStarlarkTargetDependency(
                target: &mut Target,
                dep_dir: &str,
                dep_name: &str,
                toolchain_dir: &str,
                toolchain_name: &str,
            );
        }
        unsafe {
            AddStarlarkTargetDependency(self, dep_dir, dep_name, toolchain_dir, toolchain_name);
        }
    }

    pub fn default_toolchain_label<'a>(&'a self) -> types::LabelRef<'a> {
        extern "C" {
            fn GetDefaultToolchainLabel(target: &Target) -> &Label;
        }
        unsafe { GetDefaultToolchainLabel(self).as_label_ref() }
    }

    pub fn label<'a>(&'a self) -> types::LabelRef<'a> {
        extern "C" {
            fn GetTargetLabel(target: &Target) -> &Label;
        }
        unsafe { GetTargetLabel(self).as_label_ref() }
    }

    pub fn is_action(&self) -> bool {
        extern "C" {
            fn IsActionTarget(target: &Target) -> bool;
        }
        unsafe { IsActionTarget(self) }
    }

    pub fn output_files(&self) -> Vec<types::File> {
        extern "C" {
            fn GetTargetOutputFiles(target: &Target) -> &[OutputFile];
        }
        unsafe {
            GetTargetOutputFiles(self)
                .iter()
                .map(|f| f.as_rust())
                .collect()
        }
    }

    pub fn deps(&self) -> Vec<&Self> {
        extern "C" {
            fn GetTargetDeps(target: &Target, out: *mut *const Target, max_len: usize) -> usize;
        }
        unsafe {
            let len = GetTargetDeps(self, std::ptr::null_mut(), 0);
            let mut out = Vec::with_capacity(len);
            GetTargetDeps(self, out.as_mut_ptr(), len);
            out.set_len(len);
            out.iter().map(|&p| &*p).collect()
        }
    }

    pub fn public_deps(&self) -> Vec<&Self> {
        extern "C" {
            fn GetTargetPublicDeps(
                target: &Target,
                out: *mut *const Target,
                max_len: usize,
            ) -> usize;
        }
        unsafe {
            let len = GetTargetPublicDeps(self, std::ptr::null_mut(), 0);
            let mut out = Vec::with_capacity(len);
            GetTargetPublicDeps(self, out.as_mut_ptr(), len);
            out.set_len(len);
            out.iter().map(|&p| &*p).collect()
        }
    }

    pub fn public_sources(&self) -> Vec<&str> {
        extern "C" {
            fn GetTargetPublicSources(target: &Target, out: *mut &str, max_len: usize) -> usize;
        }
        unsafe {
            let len = GetTargetPublicSources(self, std::ptr::null_mut(), 0);
            let mut out = Vec::with_capacity(len);
            GetTargetPublicSources(self, out.as_mut_ptr(), len);
            out.set_len(len);
            out
        }
    }

    pub fn private_sources(&self) -> Vec<&str> {
        extern "C" {
            fn GetTargetPrivateSources(target: &Target, out: *mut &str, max_len: usize) -> usize;
        }
        unsafe {
            let len = GetTargetPrivateSources(self, std::ptr::null_mut(), 0);
            let mut out = Vec::with_capacity(len);
            GetTargetPrivateSources(self, out.as_mut_ptr(), len);
            out.set_len(len);
            out
        }
    }

    pub fn toolchain_label<'a>(&'a self) -> types::LabelRef<'a> {
        extern "C" {
            fn GetTargetToolchainLabel(target: &Target) -> &Label;
        }
        unsafe { GetTargetToolchainLabel(self).as_label_ref() }
    }

    pub fn set_starlark_target(&self, starlark_target: &crate::target::Target) {
        extern "C" {
            fn SetTargetStarlarkTarget(
                target: &Target,
                starlark_target: &crate::target::Target,
            );
        }
        unsafe {
            SetTargetStarlarkTarget(self, starlark_target);
        }
    }

    pub fn starlark_target(&self) -> *mut crate::target::Target {
        extern "C" {
            fn GetTargetStarlarkTarget(target: &Target) -> *mut crate::target::Target;
        }
        unsafe { GetTargetStarlarkTarget(self) }
    }

    pub fn as_rust(&self) -> crate::target_ref::TargetRef {
        let rust_target_ptr = self.starlark_target();
        let rust_target = unsafe { &*rust_target_ptr };
        crate::target_ref::TargetRef::from(rust_target)
    }
}

#[no_mangle]
pub unsafe extern "C" fn convert_target(
    target: *mut Target,
    rule: &OwnedFrozenValue,
    session: &crate::session::StarlarkSession,
) -> *mut OwnedFrozenValue {
    if target.is_null() {
        return std::ptr::null_mut();
    }
    let target_ref = &*target;

    let rule_val = rule.value();
    let rule_frozen = rule_val.unpack_frozen().expect("rule must be frozen");
    let rule_typed = FrozenValueTyped::new(rule_frozen).unwrap();
    let rust_target = crate::target::Target::new_starlark(
        target_ref,
        rule_typed,
        Vec::new(),
    );
    let target_ref = session.register_target(rust_target);

    let heap = starlark::values::FrozenHeap::new();
    let frozen_val = heap.alloc(target_ref);
    let heap_ref = heap.into_ref();

    Box::into_raw(Box::new(OwnedFrozenValue::new(heap_ref, frozen_val)))
}

#[no_mangle]
pub unsafe extern "C" fn new_native_gn_target(
    session: &crate::session::StarlarkSession,
    target: *mut Target,
) -> *mut crate::target::Target {
    if target.is_null() {
        return std::ptr::null_mut();
    }
    let target_ref = &*target;
    let rust_target = crate::target::Target::new_cxx(target_ref, session);
    let target_ref = session.register_target(rust_target);

    let target_static_ref: &'static crate::target::Target = target_ref.into();
    target_static_ref as *const crate::target::Target as *mut crate::target::Target
}

/// FFI endpoint called from C++ to generate custom Ninja rules for a Starlark-defined target.
#[no_mangle]
pub unsafe extern "C" fn get_custom_ninja(
    target: &Target,
    session: &crate::session::StarlarkSession,
    mut out: std::pin::Pin<&mut cxx::CxxString>,
) {
    let starlark_target = target.starlark_target();
    if starlark_target.is_null() {
        return;
    }

    let label = target.label();
    let toolchain = target.toolchain_label();

    let target_ref = session.get_target_by_label(label, toolchain);
    if let Ok(custom_ninja) = crate::ninja::generate_custom_ninja(&*target_ref) {
        out.as_mut().push_str(&custom_ninja);
    }
}

/// FFI endpoint called from C++ to retrieve the phony extra input file path for a target, if any.
#[no_mangle]
pub unsafe extern "C" fn get_extra_input(
    _session: &crate::session::StarlarkSession,
    starlark_target: *mut crate::target::Target,
) -> &'static str {
    let target = &*starlark_target;

    if let Some(f) = target.providers().extra_inputs_phony() {
        f.as_str()
    } else {
        ""
    }
}

