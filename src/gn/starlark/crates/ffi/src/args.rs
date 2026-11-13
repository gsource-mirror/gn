// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

//! Expansion of Starlark `Args` into command-line strings inside `StringOutputBuffer`.

use args::ArgVisitor;

use crate::{Cursor, StringOutputBuffer};

fn backspace_if_moved(buffer: &mut StringOutputBuffer<'_>, start: Cursor) {
    // Safety: `reserve(0)` is safe as long as you don't advance it.
    // We can safely subtract 1 because if `cursor != start` then we must have
    // written at least one byte into the current allocation chunk.
    unsafe {
        let cursor: &mut Cursor = buffer.reserve(0);
        if *cursor != start {
            // If we wrote some data, the last thing we wrote, by design, is
            // always a space (usually via finish_arg, though sometimes via write_unescaped).
            *cursor = (*cursor).sub(1);
        }
    }
}

pub(crate) struct PosixWriter<'a, 'b> {
    buffer: &'a mut StringOutputBuffer<'b>,
    start: Cursor,
}

impl<'a, 'b> PosixWriter<'a, 'b> {
    pub(crate) fn new(buffer: &'a mut StringOutputBuffer<'b>) -> Self {
        // Safety: We never write to this cursor.
        let start = unsafe { *buffer.reserve(0) };
        Self { buffer, start }
    }
}

impl ArgVisitor for PosixWriter<'_, '_> {
    fn add_arg_chunk(&mut self, part: &str) {
        self.buffer.append(crate::escape::NinjaCommandPosix(part));
    }

    fn finish_arg(&mut self) {
        self.buffer.append(b' ');
    }

    fn write_unescaped(&mut self, part: &str) {
        self.buffer.append(part);
    }

    fn write_u32(&mut self, n: u32) {
        self.buffer.append(n);
    }

    fn done(&mut self) {
        backspace_if_moved(self.buffer, self.start);
    }
}

// Windows can't perform zero-copy escaping because we don't know whether we
// need to add quotes until we read the entire arg.
pub(crate) struct WinWriter<'a, 'b> {
    buffer: &'a mut StringOutputBuffer<'b>,
    token: String,
    start: Cursor,
}

impl<'a, 'b> WinWriter<'a, 'b> {
    pub(crate) fn new(buffer: &'a mut StringOutputBuffer<'b>) -> Self {
        // Safety: We never write to this cursor.
        let start = unsafe { *buffer.reserve(0) };
        Self {
            buffer,
            token: String::new(),
            start,
        }
    }
}

impl ArgVisitor for WinWriter<'_, '_> {
    // Optimization: Skip the extra copy into token if we're writing a whole arg at a time.
    fn add_arg(&mut self, part: &str) {
        self.buffer
            .append((crate::escape::NinjaCommandWindows(part), b' '));
    }

    fn add_arg_chunk(&mut self, part: &str) {
        self.token.push_str(part);
    }

    fn finish_arg(&mut self) {
        self.buffer
            .append((crate::escape::NinjaCommandWindows(&self.token), b' '));
        self.token.clear();
    }

    fn write_unescaped(&mut self, part: &str) {
        self.buffer.append(part);
    }

    fn write_u32(&mut self, n: u32) {
        self.buffer.append(n);
    }

    fn done(&mut self) {
        backspace_if_moved(self.buffer, self.start);
    }
}

#[cfg(test)]
mod tests {
    use args::Args;
    use starlark::{
        environment::GlobalsBuilder,
        eval::Evaluator,
        values::{HeapEdge, UnpackValue as _, Value},
    };
    use starlark_derive::starlark_module;

    use super::*;
    use crate::{bridge, escape::EscapingPlatform};

    #[starlark_module]
    fn register_args_test_globals(builder: &mut GlobalsBuilder) {
        fn args<'v>(eval: &mut Evaluator<'v, '_, '_>) -> starlark::Result<Value<'v>> {
            Ok(eval.heap().alloc(Args::default()))
        }
    }

    struct ArgsAssert {
        assert: testutils::Assert,
    }

    impl ArgsAssert {
        fn new() -> Self {
            let mut a = testutils::Assert::default();
            a.modify_globals(|builder| {
                register_args_test_globals(builder);
                depset::depset_globals!(builder, testutils::FakeEvalContext);
            });
            Self { assert: a }
        }

        #[track_caller]
        fn assert_expands_to(&mut self, code: &str, platform: EscapingPlatform, expected: &str) {
            fn write_all<'m, 'v>(
                eval: &mut Evaluator<'m, '_, '_>,
                edge: HeapEdge<'m, 'v>,
                seq: &args::ArgsList<'v>,
                plans: &args::ArgSharingPlans<'v>,
                writer: &mut impl ArgVisitor,
            ) -> starlark::Result<()> {
                for arg in seq.items() {
                    plans.write(arg, eval, edge, writer)?;
                }
                writer.done();
                Ok(())
            }

            let val = self.assert.pass(code);

            let res: starlark::Result<_> =
                val.by_ref_with_reconstructor(|val_ref, reconstructor| {
                    let seq = <args::ArgsList>::unpack_value_err(*val_ref)?;
                    let plans = args::ArgSharingPlans::new(seq.items().iter());
                    starlark::environment::Module::with_temp_heap(|module| {
                        let mut eval = Evaluator::new(&module);
                        let edge = reconstructor.edge(module.heap());
                        let mut raw_buf = bridge::NewStringOutputBuffer();
                        let mut buffer = StringOutputBuffer::new(raw_buf.pin_mut());
                        match platform {
                            EscapingPlatform::Posix => {
                                write_all(
                                    &mut eval,
                                    edge,
                                    &seq,
                                    &plans,
                                    &mut PosixWriter::new(&mut buffer),
                                )?;
                            },
                            EscapingPlatform::Win => {
                                write_all(
                                    &mut eval,
                                    edge,
                                    &seq,
                                    &plans,
                                    &mut WinWriter::new(&mut buffer),
                                )?;
                            },
                            _ => unreachable!(),
                        }
                        Ok(buffer.to_string())
                    })
                });

            assert_eq!(res.unwrap(), expected);
        }
    }

    #[test]
    fn test_args_escape_command_line_posix() {
        let mut a = ArgsAssert::new();
        a.assert_expands_to(r#"[]"#, EscapingPlatform::Posix, "");
        a.assert_expands_to(
            r#"[args().add_joined(["my flag", "my value"], join_with=",")]"#,
            EscapingPlatform::Posix,
            "my\\$ flag,my\\$ value",
        );
    }

    #[test]
    fn test_args_escape_command_line_windows() {
        let mut a = ArgsAssert::new();
        a.assert_expands_to(r#"[]"#, EscapingPlatform::Win, "");
        a.assert_expands_to(
            r#"[args().add_joined(["my flag", "my value"], join_with=",")]"#,
            EscapingPlatform::Win,
            "\"my$ flag,my$ value\"",
        );
    }
}
