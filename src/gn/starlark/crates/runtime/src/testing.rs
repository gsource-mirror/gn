// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::ops::Deref;
use std::ops::DerefMut;

use starlark::environment::FrozenModule;
use starlark::environment::GlobalsBuilder;
use starlark::eval::Evaluator;
use starlark::starlark_simple_value;
use starlark::values::ProvidesStaticType;
use starlark::values::StarlarkValue;
use starlark::values::Trace;
use starlark::values::Tracer;
use starlark::values::Value;
use starlark_derive::starlark_module;
use starlark_derive::starlark_value;
use starlark_derive::NoSerialize;
use types::util::extend_lifetime;
use attr::EvalContext as _;

use crate::ctx::CtxState;
use crate::session::globals::populate_test_globals;
use crate::session::test_with_scope::TestWithScope;
use crate::session::{EvalContext, EvalKind};
use crate::File;
use crate::LabelRef;
use crate::PackageRef;
use crate::StarlarkSession;
use crate::TargetRef;

/// Global atomic flag indicating whether the code is executing within a test environment.
pub static IS_TESTING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

use starlark::any::AnyLifetime;
use starlark::syntax::Dialect;
use starlark::values::UnpackValue;

/// A high-level test harness for Starlark integration tests in GN.
/// The test framework in use is starlark's assert framework, but it
/// needs to also configure a C++ environment for some tests to work.
pub struct Assert {
    assert: starlark::assert::Assert<'static>,
    setup: TestWithScope,
    context: EvalContext,
    global_callbacks: Vec<Box<dyn Fn(&mut GlobalsBuilder)>>,
}

#[starlark_module]
pub(crate) fn test_globals(builder: &mut GlobalsBuilder) {
    fn make_file<'v>(
        eval: &mut Evaluator<'v, '_, '_>,
        path: String,
    ) -> starlark::Result<Value<'v>> {
        Ok(eval.heap().alloc(File::from_rust(path)))
    }
}

impl Assert {
    /// Creates a new Starlark test context.
    /// By default, initializes `EvalKind` to `BuildFile(Package("//"))`.
    pub fn new() -> Box<Self> {
        IS_TESTING.store(true, std::sync::atomic::Ordering::Relaxed);
        let setup = TestWithScope::new();
        let assert = starlark::assert::Assert::new();
        let scope = setup.scope();
        let mut a = Box::new(Self {
            assert,
            setup,
            context: EvalContext::new(scope, std::ptr::null(), EvalKind::VariableConversion),
            global_callbacks: Vec::new(),
        });

        // Add the default GN globals callback
        let assert_ptr = &*a as *const Assert;
        a.globals_add(move |builder| {
            let a = unsafe { &*assert_ptr };
            let builtins = a.setup.session().builtins();
            populate_test_globals(builder, builtins);
        });

        let assert_ptr = &*a as *const Assert;

        a.assert.setup_eval(move |eval| {
            let a = unsafe { &*assert_ptr };

            let extra: &dyn AnyLifetime = &a.context;
            eval.extra = Some(extra);

            eval.module()
                .set("assert", eval.heap().alloc(AssertValue { assert: a }));
        });

        a.load_build_file(PackageRef::new("//"));
        let label = LabelRef::new("//".into(), "default_target");
        let toolchain = a.context.current_toolchain();
        let target = a.setup.session().get_target_by_label(label, toolchain);
        a.configure(EvalKind::RuleEval(CtxState::new(TargetRef::from(&*target))));
        a
    }

    /// Returns the current toolchain label.
    pub fn current_toolchain(&self) -> LabelRef<'_> {
        self.context.current_toolchain()
    }

    /// Returns the Starlark session manager.
    pub fn session(&self) -> &StarlarkSession {
        self.setup.session()
    }

    /// Returns the evaluation context.
    pub fn context(&self) -> &EvalContext {
        &self.context
    }

    /// Configures the Starlark dialect and evaluation kind.
    pub fn configure(self: &mut Box<Self>, kind: EvalKind) {
        let dialect = match &kind {
            EvalKind::BzlFile(_) => &crate::session::BZL_FILE_DIALECT,
            EvalKind::Macro(_) => &crate::session::BUILD_FILE_DIALECT,
            _ => &Dialect::Extended,
        };
        self.dialect(dialect);
        self.context.kind = kind
    }

    /// Executes a closure with a temporary evaluation context.
    pub fn with_context<T>(
        self: &mut Box<Self>,
        kind: EvalKind,
        f: impl FnOnce(&mut Box<Self>) -> T,
    ) -> T {
        let old_kind = self.context.kind.clone();
        self.configure(kind);
        let ret = f(self);
        self.configure(old_kind);
        ret
    }

    /// Loads a `BUILD` file for the given package into the C++ GN Scope and registers its targets.
    pub fn load_build_file(self: &mut Box<Self>, package: &PackageRef) {
        let path = self.setup.session().absolute_path(package, "BUILD");
        let code = std::fs::read_to_string(&path).expect("failed to read build file");
        // SourceDir objects always end with "/"
        let source_dir = if package.as_str().ends_with('/') {
            package.to_string()
        } else {
            format!("{package}/")
        };
        unsafe {
            (&mut *self.context.scope).set_source_dir(&source_dir);
        }
        self.with_context(EvalKind::Macro(package.to_owned()), |a| {
            a.pass(&code);
        });
        unsafe {
            (&mut *self.context.scope).set_source_dir("//");
        }
    }

    /// Loads a `.bzl` file for the given label.
    pub fn load_bzl_file(self: &mut Box<Self>, label: LabelRef) -> FrozenModule {
        let path = self
            .setup
            .session()
            .absolute_path(label.package, label.name);
        let code = std::fs::read_to_string(&path).expect("failed to read bzl file");
        let label_str = label.to_string();
        self.with_context(EvalKind::BzlFile(label.package.to_owned()), |a| {
            a.module(&label_str, &code)
        })
    }

    /// Configures the evaluator to run in a rule evaluation context for the target specified by `label`.
    pub fn rule_eval<T>(
        self: &mut Box<Self>,
        label: LabelRef,
        f: impl FnOnce(&mut Box<Self>) -> T,
    ) -> T {
        let toolchain = self.context.current_toolchain();
        let target = self.setup.session().get_target_by_label(label, toolchain);
        self.with_context(
            EvalKind::RuleEval(CtxState::new(TargetRef::from(&*target))),
            f,
        )
    }

    /// Retrieves the static `Assert` harness from the evaluator's context.
    pub fn load<'a, 'v, 't, 'e>(eval: &'a Evaluator<'v, 't, 'e>) -> &'a Assert {
        let val = eval
            .module()
            .get("assert")
            .expect("assert variable not found");
        let assert_val = AssertValue::from_value(val).expect("Expected AssertValue");
        assert_val.assert
    }

    /// Asserts that evaluating Starlark code returns a value equal to `expected`.
    #[track_caller]
    pub fn eq<'v, T>(&self, code: &str, expected: T)
    where
        T: PartialEq + std::fmt::Debug + UnpackValue<'v>,
    {
        let owned_val = self.assert.pass(code);
        let owned_val_ref = unsafe { extend_lifetime::<'v, '_>(&owned_val) };
        let val = <T>::unpack_value_err(owned_val_ref.value()).unwrap();
        assert_eq!(val, expected)
    }

    /// Evaluates the given code and unpacks it to type `T`.
    #[track_caller]
    pub fn eval<'v, T>(&self, code: &str) -> T
    where
        T: UnpackValue<'v>,
    {
        let owned_val = self.assert.pass(code);
        let owned_val_ref = unsafe { extend_lifetime::<'v, '_>(&owned_val) };
        T::unpack_value_err(owned_val_ref.value()).unwrap()
    }

    /// Asserts that evaluating two Starlark expressions yields identical values.
    #[track_caller]
    pub fn equivalent(&self, lhs_code: &str, rhs_code: &str) {
        self.assert.eq(lhs_code, rhs_code);
    }

    /// Registers a custom global variable callback for compilation/evaluation.
    pub fn globals_add(&mut self, f: impl Fn(&mut GlobalsBuilder) + 'static) {
        self.global_callbacks.push(Box::new(f));
        self.rebuild_globals();
    }

    fn rebuild_globals(&mut self) {
        let callbacks = &self.global_callbacks;
        self.assert.globals_add(|builder| {
            for cb in callbacks {
                cb(builder);
            }
        });
    }
}

#[derive(ProvidesStaticType, NoSerialize, allocative::Allocative)]
struct AssertValue {
    #[allocative(skip)]
    assert: &'static Assert,
}

impl std::fmt::Debug for AssertValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AssertValue")
    }
}

unsafe impl Send for AssertValue {}
unsafe impl Sync for AssertValue {}

starlark_simple_value!(AssertValue);

#[starlark_value(type = "AssertValue")]
impl<'v> StarlarkValue<'v> for AssertValue {}

unsafe impl<'v> Trace<'v> for AssertValue {
    fn trace(&mut self, _tracer: &Tracer<'v>) {}
}

impl std::fmt::Display for AssertValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AssertValue")
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
