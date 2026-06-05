// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use providers::UnpackProvider;
use starlark::environment::{FrozenModule, Module};
use starlark::eval::Evaluator;
use starlark::values::list::UnpackList;
use starlark::values::record::Record;
use starlark::values::Heap;
use starlark::values::UnpackValue;
use starlark::values::Value;

use crate::attr::AllowFilesSchema;
use crate::ctx::CtxGen;
use crate::ctx::CtxState;
use crate::ffi;
use crate::rule::FrozenRule;
use crate::session::{EvalContext, EvalKind};
use crate::StarlarkSession;
use crate::Target;
use crate::TargetRef;

fn prepare_ctx<'v>(
    target: &Target,
    rule: &FrozenRule,
    session: &StarlarkSession,
    heap: &Heap<'v>,
) -> starlark::Result<Value<'v>> {
    let mut resolved_attrs_vec = Vec::new();
    let mut resolved_file_vec = Vec::new();
    let mut resolved_files_vec = Vec::new();

    let current_toolchain = target.toolchain().to_owned();

    assert_eq!(target.attrs.len(), rule.attrs.len());
    for ((_name, attr), schema) in std::iter::zip(target.get_unevaluated_attrs(), rule.attrs.values()) {
        let attr_val = attr.to_value(
            schema,
            session,
            &current_toolchain.as_ref(),
            &target.label(),
            heap,
        )?;

        resolved_attrs_vec.push(attr_val.attr);

        if matches!(schema.allow_files(), AllowFilesSchema::Single(_)) {
            resolved_file_vec.push(attr_val.file.unwrap_or(Value::new_none()));
        }
        if schema.file_matcher().is_some() {
            resolved_files_vec.push(
                attr_val
                    .files
                    .unwrap_or_else(|| heap.alloc(Vec::<Value>::new())),
            );
        }
    }

    let attr = heap.alloc_complex(Record {
        typ: rule.attrs_record_type.to_value(),
        values: resolved_attrs_vec.into_boxed_slice(),
    });

    let file = heap.alloc_complex(Record {
        typ: rule.file_record_type.to_value(),
        values: resolved_file_vec.into_boxed_slice(),
    });

    let files = heap.alloc_complex(Record {
        typ: rule.files_record_type.to_value(),
        values: resolved_files_vec.into_boxed_slice(),
    });

    Ok(heap.alloc(CtxGen::new(attr, file, files, heap)))
}

fn run_target_rule_implementation_inner(
    target: &mut Target,
    scope: *mut ffi::Scope,
    session: &StarlarkSession,
) -> starlark::Result<()> {
    let rule = target.rule().unwrap();

    let extra = EvalContext::new(
        scope,
        std::ptr::null(),
        EvalKind::RuleEval(CtxState::new(TargetRef::from(&*target))),
    );

    let frozen_module = Module::with_temp_heap(|module| -> starlark::Result<FrozenModule> {
        let ctx = prepare_ctx(target, rule.as_ref(), session, &module.heap())?;
        {
            let mut eval = Evaluator::new(&module);
            eval.extra = Some(&extra);

            let function_result =
                eval.eval_function(rule.implementation.to_value(), &[ctx], &[])?;
            module.set("", function_result);
        }
        Ok(module.freeze()?)
    })?;

    let binding = frozen_module.get("").unwrap();
    let providers_val = binding.value();
    let providers = UnpackList::<UnpackProvider>::unpack_value_err(providers_val)?.items;

    if let EvalKind::RuleEval(state) = &extra.kind {
        target.phonies = state.phonies.clone();
    }

    target.set_providers(frozen_module.frozen_heap(), &providers, session.builtins())?;
    Ok(())
}

/// FFI endpoint called from C++ to evaluate a Starlark rule's implementation function for a target.
#[no_mangle]
pub unsafe extern "C" fn run_target_rule_implementation(
    starlark_target: *mut crate::target::Target,
    scope: *mut ffi::Scope,
    session: &crate::session::StarlarkSession,
    err: &mut ffi::Err,
) -> bool {
    // Safety: For the borrow checker to not complain, this should be accessed through a lock.
    // But by design, while running the implementation, no other threads will access this target.
    let target = unsafe { &mut *starlark_target };
    let res = run_target_rule_implementation_inner(target, scope, session);

    ffi::handle_result(err, std::ptr::null(), res.map(|_| true))
}
