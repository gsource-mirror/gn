// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::hash::{Hash, Hasher};

use starlark::{
    collections::SmallMap,
    eval::Evaluator,
    values::{HeapEdge, ValueTyped},
};

use crate::{ArgVisitor, ArgsLike, FrozenArgs, expand::expand_args};

/// Describes how a shared or inline `FrozenArgs` slice (`args[..size]`) should be serialized.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ArgPlan {
    /// Variable ID assigned if this slice is referenced more than once.
    id: Option<u32>,
    /// Parent variable ID whose expanded prefix this slice extends.
    parent: Option<u32>,
    /// Number of leading entries already covered by `parent`.
    prev_size: usize,
}

/// Wrapper comparing and hashing `FrozenArgs` values by heap identity rather
/// than value equality.
#[derive(Clone, Copy, Debug)]
struct FrozenArgsId<'v>(ValueTyped<'v, FrozenArgs<'v>>);

impl<'v> PartialEq for FrozenArgsId<'v> {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_value().identity() == other.0.to_value().identity()
    }
}

impl<'v> Eq for FrozenArgsId<'v> {}

impl<'v> Hash for FrozenArgsId<'v> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.to_value().identity().hash(state);
    }
}

/// Map from each non-empty `FrozenArgs` slice `(id, size)` to how it should
/// be emitted or reused as a Ninja variable.
#[derive(Debug, PartialEq, Eq)]
pub struct ArgSharingPlans<'v> {
    plans: SmallMap<(FrozenArgsId<'v>, usize), ArgPlan>,
}

fn write_var(visitor: &mut impl ArgVisitor, prefix: &str, n: u32, suffix: &str) {
    visitor.write_unescaped(prefix);
    visitor.write_u32(n);
    visitor.write_unescaped(suffix);
}

impl<'v> ArgSharingPlans<'v> {
    /// Plans how to expand each `FrozenArgs` without duplicating shared prefixes.
    ///
    /// - Args used once are inlined (`id: None`); slices reused or extended later are assigned an
    ///   `id` so callers define them once and reference `$id` elsewhere.
    /// - Because `Args` is append-only, later slices are assigned the prior slice's `id` as
    ///   `parent` and `prev_size`, so they expand to `$parent` plus only the new
    ///   `[prev_size..size]` tail.
    /// - This function assumes that entries are ordered so every `parent` precedes its children
    ///   - This is inherent to starlark (I cannot call ctx.actions.run once with args, then later
    ///     with a smaller args because it's append-only).
    pub fn new<'a>(args: impl Iterator<Item = &'a ArgsLike<'v>>) -> Self
    where
        'v: 'a,
    {
        struct ArgState {
            id: Option<u32>,
            size: usize,
        }

        let mut arg_states: SmallMap<FrozenArgsId<'v>, ArgState> = SmallMap::new();
        let mut plans: SmallMap<(FrozenArgsId<'v>, usize), ArgPlan> = SmallMap::new();
        let mut next_id = 0u32;

        let mut alloc_id = || {
            let id = next_id;
            next_id += 1;
            id
        };

        for arg in args {
            let ArgsLike::FrozenArgs(v, size) = *arg else {
                continue;
            };
            if size == 0 {
                continue;
            }
            if let Some(state) = arg_states.get_mut(&FrozenArgsId(v)) {
                // We've seen this args object before. Promote the previous slice
                // to a named variable so it isn't expanded multiple times.
                let id = *state.id.get_or_insert_with(|| {
                    let id = alloc_id();
                    plans.get_mut(&(FrozenArgsId(v), state.size)).unwrap().id = Some(id);
                    id
                });
                // Append-only mutations slice cleanly into `$parent <delta>`.
                if size > state.size {
                    plans.insert(
                        (FrozenArgsId(v), size),
                        ArgPlan {
                            id: None,
                            parent: Some(id),
                            prev_size: state.size,
                        },
                    );
                    *state = ArgState { id: None, size };
                }
            } else {
                arg_states.insert(FrozenArgsId(v), ArgState { id: None, size });
                plans.insert(
                    (FrozenArgsId(v), size),
                    ArgPlan {
                        id: None,
                        parent: None,
                        prev_size: 0,
                    },
                );
            }
        }

        Self { plans }
    }

    /// Returns whether any Ninja variable definitions are needed for shared slices.
    pub fn has_shared_args(&self) -> bool {
        self.plans.values().any(|plan| plan.id.is_some())
    }

    /// Writes Ninja variable definitions for all shared slices.
    pub fn write_shared_args<'m>(
        &self,
        eval: &mut Evaluator<'m, '_, '_>,
        edge: HeapEdge<'m, 'v>,
        visitor: &mut impl ArgVisitor,
    ) -> starlark::Result<()> {
        for ((args_id, size), plan) in &self.plans {
            let Some(id) = plan.id else {
                continue;
            };
            write_var(visitor, "args_", id, " = ");
            if let Some(parent_id) = plan.parent {
                write_var(visitor, "$args_", parent_id, " ");
            }
            expand_args(
                &edge.rebrand(args_id.0).arguments[plan.prev_size..*size],
                eval,
                visitor,
            )?;
            visitor.done();
            visitor.write_unescaped("\n");
        }
        Ok(())
    }

    /// Writes a single argument entry, referencing shared variables or expanding inline as planned.
    pub fn write<'m>(
        &self,
        arg: &ArgsLike<'v>,
        eval: &mut Evaluator<'m, '_, '_>,
        edge: HeapEdge<'m, 'v>,
        visitor: &mut impl ArgVisitor,
    ) -> starlark::Result<()> {
        match *arg {
            ArgsLike::String(s) => visitor.add_arg(s.as_str()),
            ArgsLike::Args(..) => {
                unreachable!("ArgSharingPlans::write should never see non-frozen Args")
            },
            ArgsLike::FrozenArgs(v, size) => {
                if size == 0 {
                    return Ok(());
                }
                let plan = self.plans.get(&(FrozenArgsId(v), size)).unwrap();
                if let Some(id) = plan.id {
                    write_var(visitor, "$args_", id, " ");
                } else {
                    if let Some(parent_id) = plan.parent {
                        write_var(visitor, "$args_", parent_id, " ");
                    }
                    expand_args(
                        &edge.rebrand(v).arguments[plan.prev_size..size],
                        eval,
                        visitor,
                    )?;
                }
            },
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use starlark::{
        environment::{GlobalsBuilder, Module},
        syntax::{AstModule, Dialect},
        values::{UnpackValue as _, Value, list::ListRef, none::NoneType},
    };
    use starlark_derive::starlark_module;

    use super::*;
    use crate::{Args, TestArgVisitor};

    #[starlark_module]
    fn register_arg_sharing_test_globals(builder: &mut GlobalsBuilder) {
        fn args<'v>(eval: &mut Evaluator<'v, '_, '_>) -> starlark::Result<Value<'v>> {
            Ok(eval.heap().alloc(Args::default()))
        }

        fn capture<'v>(
            arg: ValueTyped<'v, Args<'v>>,
            eval: &mut Evaluator<'v, '_, '_>,
        ) -> starlark::Result<NoneType> {
            let mut captured = eval
                .module()
                .extra_value()
                .map(|v| ListRef::from_value(v).unwrap().content().to_vec())
                .unwrap_or_default();
            let size = i32::try_from(arg.arguments.borrow().len()).unwrap();
            captured.push(eval.heap().alloc((arg, size)));
            eval.module().set_extra_value(eval.heap().alloc(captured));
            Ok(NoneType)
        }
    }

    fn eval_captured(code: &str) -> String {
        let mut globals = GlobalsBuilder::standard();
        register_arg_sharing_test_globals(&mut globals);
        let globals = globals.build();

        let frozen = Module::with_temp_heap(|module| {
            {
                let mut eval = Evaluator::new(&module);
                eval.eval_module(
                    AstModule::parse("//test.scl", code.to_owned(), &Dialect::Standard).unwrap(),
                    &globals,
                )
                .unwrap();
            }
            module.freeze().unwrap().extra_value().unwrap()
        });

        frozen.by_ref_with_reconstructor(|val, reconstructor| {
            let plans = ArgSharingPlans::new(
                ListRef::from_value(*val)
                    .unwrap()
                    .iter()
                    .map(|item| {
                        let (args, size) =
                            <(ValueTyped<'_, FrozenArgs<'_>>, i32)>::unpack_value_err(item)
                                .unwrap();
                        ArgsLike::FrozenArgs(args, usize::try_from(size).unwrap())
                    })
                    .collect::<Vec<_>>()
                    .iter(),
            );
            Module::with_temp_heap(|module| {
                let mut eval = Evaluator::new(&module);
                let mut visitor = TestArgVisitor::default();
                plans
                    .write_shared_args(&mut eval, reconstructor.edge(module.heap()), &mut visitor)
                    .unwrap();
                visitor.args
            })
        })
    }

    #[test]
    fn test_plan_arg_sharing_ignores_empty_and_inline_singletons() {
        let shared = eval_captured(
            r#"
empty = args()
later = args()
solo = args().add("--solo")
capture(empty)
capture(later)
capture(solo)
later.add("--added")
capture(empty)
capture(later)
"#,
        );
        assert_eq!(shared, "");
    }

    #[test]
    fn test_plan_arg_sharing_reuses_same_size_and_promotes_extensions() {
        let shared = eval_captured(
            r#"
reused = args().add("reused")
capture(reused)
capture(reused)
lazy = args().add("lazy1")
capture(lazy)
lazy.add("lazy2")
capture(lazy)
lazy.add("lazy3")
capture(lazy)
"#,
        );
        assert_eq!(
            shared,
            "args_0 = reused\nargs_1 = lazy1\nargs_2 = $args_1 lazy2\n"
        );
    }
}
