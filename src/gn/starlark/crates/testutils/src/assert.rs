// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use crate::FakeEvalContext;
use starlark::values::UnpackValue;
use types::{EvaluatorContextExt, UnpackedOwnedValue};

/// A simple wrapper around starlark::Assert that provides some basic extra functionality.
pub struct Assert {
    assert: starlark::assert::Assert<'static>,
    pub context: Box<FakeEvalContext>,
}

impl Default for Assert {
    fn default() -> Self {
        Self::new(FakeEvalContext::default())
    }
}

impl Assert {
    /// Creates a new `Assert` helper instance with the given context.
    pub fn new(context: FakeEvalContext) -> Self {
        let mut assert = starlark::assert::Assert::new();
        // By default, starlark runs the code 3 times, with always gc, auto gc, and disable gc.
        // Because we use extra_mut to allow passing a mutating state to the evaluator,
        // this means that the state can mutate three times.
        assert.always_gc();
        let mut context = Box::new(context);
        let context_ptr = &mut *context as *mut FakeEvalContext;

        assert.setup_eval(move |eval| {
            // Safety: The context is owned by Assert, which outlives the evaluator run.
            let context_mut = unsafe { &mut *context_ptr };
            eval.set_context(context_mut);
        });

        Self { assert, context }
    }

    /// Asserts that the result of evaluating code is equal to expected.
    #[track_caller]
    pub fn eq<'v, T>(&self, code: &str, expected: T)
    where
        T: PartialEq + std::fmt::Debug + UnpackValue<'v>,
    {
        assert_eq!(*self.eval::<T>(code), expected);
    }

    /// Evaluates code and unpacks it to a given type.
    #[track_caller]
    pub fn eval<'v, T>(&self, code: &str) -> UnpackedOwnedValue<'v, T>
    where
        T: UnpackValue<'v>,
    {
        let owned_val = self.assert.pass(code);
        UnpackedOwnedValue::<T>::try_from(owned_val).unwrap()
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
