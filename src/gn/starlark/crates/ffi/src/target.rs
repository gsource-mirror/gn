// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::cell::OnceCell;

use starlark::{
    collections::SmallSet,
    values::{HeapEdge, OwnedFrozen, ValueTyped},
};

use crate::{
    StringOutputBuffer, TargetRef,
    escape::{EscapingPlatform, Ninja},
    eval_context::EvalContext,
};

pub struct Target {
    pub(crate) rule: &'static rule::Rule<'static, EvalContext>,
    pub(crate) attrs: Vec<attr::Attr>,
    pub(crate) evaluated:
        OnceCell<OwnedFrozen<ValueTyped<'static, rule::EvaluatedTarget<'static>>>>,
}

/// Drops an initialized `Target` stored inline in C++ `Item::rust_target_storage_`.
///
/// # Safety
/// `ptr` must point to an initialized, properly aligned `Target` that will
/// not be accessed after this call.
pub unsafe fn drop_ffi_target(ptr: *mut Target) {
    // Safety: Caller guarantees exclusive ownership of an initialized `Target`.
    unsafe { std::ptr::drop_in_place(ptr) };
}

impl Target {
    /// Constructs a built-in `Target` in-place inside `cxx`.
    ///
    /// # Safety
    /// `cxx` must have uninitialized `Item::rust_target_storage_`.
    pub unsafe fn new_builtin(
        session: &crate::Session,
        mut cxx: std::pin::Pin<&mut crate::bridge::CxxTarget>,
    ) {
        let target = Self {
            rule: session.builtin_rule(cxx.output_type()),
            attrs: Vec::new(),
            evaluated: Default::default(),
        };
        // Safety: `claim_rust_target_storage()` returns uninitialized storage inside
        // `cxx`, whose size and alignment are verified by `Session::from_resolver`
        // and the compile-time assert below.
        unsafe { std::ptr::write(cxx.as_mut().claim_rust_target_storage(), target) };
    }

    /// Returns `true` if this target uses a built-in GN rule.
    pub fn is_builtin(&self) -> bool {
        self.rule.is_builtin()
    }

    /// Writes out Ninja phony rules declared by this target into the provided buffer.
    pub fn write_phonies(&self, buffer: std::pin::Pin<&mut crate::bridge::StringOutputBuffer>) {
        if let Some(evaluated) = self.evaluated.get() {
            let mut buf = StringOutputBuffer::new(buffer);
            evaluated.by_ref(|evaluated| {
                for phony in &evaluated.phonies {
                    buf.append(("build ", Ninja(phony.phony.as_str()), ": phony"));
                    for dep in &phony.expansion {
                        buf.append((b' ', Ninja(dep.as_str())));
                    }
                    buf.append(b'\n');
                }
            });
        }
    }

    /// Writes Ninja variables and build edges declared by this target.
    pub fn write_ninja(
        &self,
        buffer: std::pin::Pin<&mut crate::bridge::StringOutputBuffer>,
        platform: EscapingPlatform,
        mut err: std::pin::Pin<&mut crate::bridge::Err>,
    ) {
        if let Some(evaluated) = self.evaluated.get() {
            let mut buf = StringOutputBuffer::new(buffer);
            evaluated.by_ref_with_reconstructor(|evaluated, reconstructor| {
                if !evaluated.providers.substitutions.is_empty() || !evaluated.actions.is_empty() {
                    err.as_mut()
                        .handle(starlark::environment::Module::with_temp_heap(|module| {
                            write_ninja_for_platform(
                                module,
                                reconstructor,
                                &evaluated.providers.substitutions,
                                &evaluated.actions,
                                &mut buf,
                                platform,
                            )
                        }));
                }
            });
        }
    }

    /// Writes the inputs for `outputs` declared by GnInputsInfo to the ninja
    /// file.
    pub fn write_inputs(
        &self,
        outputs: &cxx::CxxVector<crate::bridge::OutputFile>,
        mut has_inputs: bool,
        buffer: std::pin::Pin<&mut crate::bridge::StringOutputBuffer>,
    ) -> bool {
        if let Some(evaluated) = self.evaluated.get() {
            let mut buf = StringOutputBuffer::new(buffer);
            let mut seen = SmallSet::new();
            evaluated.by_ref(|evaluated| {
                for (suffix, phony) in &evaluated.providers.inputs_phony {
                    if outputs
                        .iter()
                        .any(|out| out.value().ends_with(suffix.as_str()))
                        && seen.insert(phony.as_str())
                    {
                        if !has_inputs {
                            buf.append(" |");
                            has_inputs = true;
                        }
                        buf.append((b' ', Ninja(phony.as_str())));
                    }
                }
            });
        }
        has_inputs
    }

    /// Returns the evaluated rule result for this target.
    /// Invariant: The target has already been evaluated.
    pub fn evaluated(&self) -> &OwnedFrozen<ValueTyped<'static, rule::EvaluatedTarget<'static>>> {
        self.evaluated
            .get()
            .expect("Should only be called on an evaluated target")
    }
}

/// Executes the rule implementation for a target if it has a custom
/// Starlark rule.
pub fn execute_rule_impl(
    target: &'static crate::bridge::CxxTarget,
    session: &'static crate::Session,
    err: std::pin::Pin<&mut crate::bridge::Err>,
) -> &'static str {
    let target_ref = TargetRef::new(target);
    let res = rule::run(&target_ref, |t| {
        crate::eval_context::EvalContext::new_rule_impl(session, *t)
    });
    if let Some(evaluated) = err.handle(res) {
        let phony = evaluated.by_ref(|evaluated| {
            session.record_mnemonics(&evaluated.actions);
            evaluated
                .providers
                .outputs_phony
                .as_ref()
                .map(|f| f.as_str())
                .unwrap_or_default()
        });
        target_ref
            .evaluated
            .set(evaluated)
            .expect("Rules can only be executed once");
        phony
    } else {
        ""
    }
}

// Item::rust_target_storage_ in C++ uses `alignof(void*)`.
const _: () = assert!(std::mem::align_of::<Target>() <= std::mem::align_of::<*const ()>());

impl crate::bridge::CxxTarget {
    /// Returns the output type of the target as a u8 discriminant.
    pub fn output_type(&self) -> u8 {
        crate::bridge::output_type_u8(self)
    }

    /// Returns the settings for the target.
    pub fn settings(&self) -> &crate::Settings {
        // Safety: Settings pointer is always valid and non-null on constructed Targets.
        unsafe { self.settings_cxx().as_ref() }.unwrap()
    }

    /// Returns the toolchain label for the target.
    pub fn toolchain(&self) -> types::LabelRef<'_> {
        self.settings().toolchain_label().as_ref()
    }
}

fn write_ninja<'m, 'v>(
    plans: &args::ArgSharingPlans<'v>,
    substitutions: &[(String, args::ArgsList<'v>)],
    actions: &[rule::action::Action<'v>],
    eval: &mut starlark::eval::Evaluator<'m, '_, '_>,
    edge: HeapEdge<'m, 'v>,
    visitor: &mut impl args::ArgVisitor,
) -> starlark::Result<()> {
    plans.write_shared_args(eval, edge, visitor)?;

    for (name, args) in substitutions {
        visitor.write(("starlark_", name.as_str(), " = "));
        for arg in args.items() {
            plans.write(arg, eval, edge, visitor)?;
        }
        visitor.done();
        visitor.write(b'\n');
    }

    for action in actions {
        visitor.write("build");
        for out in &action.outputs {
            visitor.write((b' ', crate::escape::Ninja::from(*out)));
        }
        visitor.write((
            ": action_",
            action.mnemonic.map(|m| m.as_str()).unwrap_or("ACTION"),
        ));
        if !action.inputs.is_empty() {
            visitor.write(" |");
            for input in &action.inputs {
                visitor.write((" ", crate::escape::Ninja::from(*input)));
            }
        }
        visitor.write("\n  command = ");
        visitor.add_arg(match action.executable {
            either::Either::Left(file) => file.as_str(),
            either::Either::Right(s) => s.as_str(),
        });
        for arg in action.arguments.items() {
            plans.write(arg, eval, edge, visitor)?;
        }
        visitor.done();
        visitor.write(b'\n');
    }
    Ok(())
}

fn write_ninja_for_platform<'v>(
    module: starlark::environment::Module,
    reconstructor: starlark::values::OwnedFrozenReconstructor<'v>,
    substitutions: &[(String, args::ArgsList<'v>)],
    actions: &[rule::action::Action<'v>],
    buf: &mut StringOutputBuffer<'_>,
    platform: EscapingPlatform,
) -> starlark::Result<()> {
    let mut eval = starlark::eval::Evaluator::new(&module);
    let edge = reconstructor.edge(eval.heap());
    let plans = args::ArgSharingPlans::new(
        actions
            .iter()
            .flat_map(|action| action.arguments.items())
            .chain(substitutions.iter().flat_map(|(_, args)| args.items())),
    );

    match platform {
        EscapingPlatform::Posix => write_ninja(
            &plans,
            substitutions,
            actions,
            &mut eval,
            edge,
            &mut crate::args::PosixWriter::new(buf),
        ),
        EscapingPlatform::Win => write_ninja(
            &plans,
            substitutions,
            actions,
            &mut eval,
            edge,
            &mut crate::args::WinWriter::new(buf),
        ),
        _ => unreachable!(),
    }
}
