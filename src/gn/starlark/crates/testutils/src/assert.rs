// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use starlark::values::UnpackValue;

/// A simple wrapper around starlark::Assert that provides some basic extra functionality.
pub struct Assert {
    assert: starlark::assert::Assert<'static>,
    pub context: Box<crate::FakeEvalContext>,
}

impl Default for Assert {
    fn default() -> Self {
        Self::new()
    }
}

impl Assert {
    /// Creates a new `Assert` helper instance.
    pub fn new() -> Self {
        let mut assert = starlark::assert::Assert::new();
        let context = Box::new(crate::FakeEvalContext::new("//"));
        let context_ptr = &*context as *const crate::FakeEvalContext;

        assert.setup_eval(move |eval| {
            // Safety: The context is owned by Assert, which outlives the evaluator run.
            let context_ref = unsafe { &*context_ptr };
            eval.extra = Some(context_ref);
        });

        Self { assert, context }
    }

    /// Asserts that the result of evaluating code is equal to expected.
    #[track_caller]
    pub fn eq<'v, T>(&self, code: &str, expected: T)
    where
        T: PartialEq + std::fmt::Debug + UnpackValue<'v>,
    {
        let owned_val = self.assert.pass(code);
        let owned_val_ref = unsafe { types::util::extend_lifetime(&owned_val) };
        let val = <T>::unpack_value_err(owned_val_ref.value()).unwrap();
        assert_eq!(val, expected);
    }

    /// Evaluates code and unpacks it to a given type.
    #[track_caller]
    pub fn eval<'v, T>(&self, code: &str) -> T
    where
        T: UnpackValue<'v>,
    {
        let owned_val = self.assert.pass(code);
        let owned_val_ref = unsafe { types::util::extend_lifetime(&owned_val) };
        T::unpack_value_err(owned_val_ref.value()).unwrap()
    }

    /// Asserts that the two pieces of code produce something equivalent.
    #[track_caller]
    pub fn equivalent(&self, lhs_code: &str, rhs_code: &str) {
        let lhs_val = self.assert.pass(lhs_code);
        let rhs_val = self.assert.pass(rhs_code);
        assert_eq!(lhs_val.value(), rhs_val.value());
    }
}

// We implement deref to get for free all the methods on starlark::Assert.
impl std::ops::Deref for Assert {
    type Target = starlark::assert::Assert<'static>;
    fn deref(&self) -> &Self::Target {
        &self.assert
    }
}

impl std::ops::DerefMut for Assert {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.assert
    }
}
