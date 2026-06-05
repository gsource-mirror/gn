// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#![cfg(test)]

use std::rc::Rc;
use std::ops::{Deref, DerefMut};

/// A high-level test harness for Starlark integration tests in GN.
/// The test framework in use is starlark's assert framework, but it
/// needs to also configure a C++ environment for some tests to work.
pub struct Assert {
    assert: starlark::assert::Assert<'static>,
    setup: Rc<crate::ffi::TestWithScope>,
}

impl Assert {
    /// Creates a new Starlark test context.
    /// By default, initializes `EvalKind` to `BuildFile(Package("//"))`.
    pub fn new() -> Self {
        Self::with_setup_and_kind(
            Rc::new(crate::ffi::TestWithScope::new()),
            crate::session::EvalKind::BuildFile(crate::label::Package("//".to_owned())),
        )
    }

    /// Configures the current evaluation kind and context (e.g. BuildFile or BzlFile),
    /// reusing the same underlying C++ environment by cloning the Rc pointer.
    pub fn configure(self, kind: crate::session::EvalKind) -> Self {
        Self::with_setup_and_kind(self.setup.clone(), kind)
    }

    fn with_setup_and_kind(setup: Rc<crate::ffi::TestWithScope>, kind: crate::session::EvalKind) -> Self {
        let mut assert = starlark::assert::Assert::new();
        assert.globals_add(|builder| {
            for (name, val) in crate::globals::make_globals().iter() {
                builder.set(name, val);
            }
        });

        let dialect = match &kind {
            crate::session::EvalKind::BzlFile(_) => &crate::session::BZL_FILE_DIALECT,
            crate::session::EvalKind::BuildFile(_) => &crate::session::BUILD_FILE_DIALECT,
            _ => &starlark::syntax::Dialect::Extended,
        };
        assert.dialect(dialect);

        let context = crate::session::EvalContext::new(
            setup.scope(),
            std::ptr::null(),
            kind,
        );

        let state = (setup.clone(), context);
        assert.setup_eval(move |eval| {
            let _keep_alive = &state;
            let extra: &dyn starlark::any::AnyLifetime = &state.1;
            eval.extra = Some(unsafe { std::mem::transmute(extra) });
        });

        Self { assert, setup }
    }

    #[track_caller]
    pub fn eq<'v, T>(&self, code: &str, expected: T)
    where
        T: PartialEq + std::fmt::Debug + starlark::values::UnpackValue<'v>,
    {
        let owned_val = self.assert.pass(code);
        let owned_val_ref = unsafe { crate::util::extend_lifetime::<'v, '_>(&owned_val) };
        let val = <T>::unpack_value_err(owned_val_ref.value()).unwrap();
        assert_eq!(val, expected)
    }

    #[track_caller]
    pub fn eval<'v, T>(&self, code: &str) -> T
    where
        T: starlark::values::UnpackValue<'v>,
    {
        let owned_val = self.assert.pass(code);
        let owned_val_ref = unsafe { crate::util::extend_lifetime::<'v, '_>(&owned_val) };
        T::unpack_value_err(owned_val_ref.value()).unwrap()
    }

    #[track_caller]
    pub fn equivalent(&self, lhs_code: &str, rhs_code: &str) {
        self.assert.eq(lhs_code, rhs_code);
    }
}

impl Deref for Assert {
    type Target = starlark::assert::Assert<'static>;

    fn deref(&self) -> &Self::Target {
        &self.assert
    }
}

impl DerefMut for Assert {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.assert
    }
}
