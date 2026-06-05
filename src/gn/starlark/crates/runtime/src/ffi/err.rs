// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use crate::ffi::ParseNode;

declare_opaque_type!(Err);

impl Err {
    pub fn populate(&mut self, message: &str) {
        extern "C" {
            fn PopulateErr(err: &mut Err, message: &str);
        }
        unsafe {
            PopulateErr(self, message);
        }
    }

    pub fn populate_with_location(&mut self, message: &str, origin: Option<&ParseNode>) {
        extern "C" {
            fn PopulateErrWithLocation(err: &mut Err, message: &str, origin: Option<&ParseNode>);
        }
        unsafe {
            PopulateErrWithLocation(self, message, origin);
        }
    }

    pub fn populate_with_help(&mut self, message: &str, help: &str, origin: Option<&ParseNode>) {
        extern "C" {
            fn PopulateErrWithHelp(
                err: &mut Err,
                message: &str,
                help: &str,
                origin: Option<&ParseNode>,
            );
        }
        unsafe {
            PopulateErrWithHelp(self, message, help, origin);
        }
    }

    pub fn message(&self) -> String {
        extern "C" {
            fn GetErrorMessage(err: &Err) -> String;
        }
        unsafe { GetErrorMessage(self) }
    }

    pub fn has_error(&self) -> bool {
        extern "C" {
            fn ErrHasError(err: &Err) -> bool;
        }
        unsafe { ErrHasError(self) }
    }

    pub fn new() -> *mut Err {
        extern "C" {
            fn CreateErr() -> *mut Err;
        }
        unsafe { CreateErr() }
    }

    pub fn free(err: *mut Err) {
        extern "C" {
            fn FreeErr(err: *mut Err);
        }
        unsafe {
            FreeErr(err);
        }
    }
}
