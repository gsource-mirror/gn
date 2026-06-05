// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

/// Declares a C++ opaque type on the Rust side of the FFI boundary.
///
/// Under the hood, this creates a `#[repr(C)]` struct with a private zero-sized
/// member. This representation allows us to use Rust's standard reference
/// qualifiers directly in FFI signatures:
///
/// * `&OpaqueType` => Maps to C++ `const OpaqueType&` (guaranteed non-null).
/// * `&mut OpaqueType` => Maps to C++ `OpaqueType&` (guaranteed non-null).
/// * `*const OpaqueType` => Maps to C++ `const OpaqueType*` (nullable).
/// * `*mut OpaqueType` => Maps to C++ `OpaqueType*` (nullable).
/// * `Option<&OpaqueType>` => Maps to C++ `const OpaqueType*` (nullable, mapped to `nullptr` if `None` due to Null Pointer Optimization).
///
/// **What does NOT work:**
/// * Do **not** pass or return these opaque types by-value (e.g. `OpaqueType` as a function parameter or return type).
///   Since they are empty structs in Rust, passing them by-value will result in size-mismatches across the FFI boundary.
/// * There is no verification that the C++ definitions match the rust declarations declared in rust.
macro_rules! declare_opaque_type {
    ($name:ident) => {
        declare_opaque_type!(pub $name);
    };
    ($vis:vis $name:ident) => {
        #[repr(C)]
        $vis struct $name {
            // Private member prevents external construction or instantiation by-value.
            // Zero-sized array keeps the struct zero-sized and has no impact on alignment.
            _private: [u8; 0],
        }
    };
}

#[repr(C)]
pub struct Pair<T, U> {
    pub first: T,
    pub second: U,
}
