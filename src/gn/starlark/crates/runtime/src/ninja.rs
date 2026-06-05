use either::Either;
use starlark::environment::Module;
use starlark::eval::Evaluator;

use crate::ctx::CtxState;
use crate::target::Target;
use crate::target_ref::TargetRef;
use crate::{EvalContext, EvalKind};

fn escape_for_ninja(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for c in s.chars() {
        if c == '$' || c == ' ' || c == ':' {
            result.push('$');
        }
        result.push(c);
    }
    result
}

fn write_substitution_info(target: &Target, out: &mut String) -> starlark::Result<()> {
    // We need an evaluator specifically only for map_each.
    Module::with_temp_heap(|module| -> starlark::Result<()> {
        let extra = EvalContext::new(
            std::ptr::null_mut(),
            std::ptr::null(),
            EvalKind::RuleEval(CtxState::new(TargetRef::from(target))),
        );
        let mut eval = Evaluator::new(&module);
        eval.extra = Some(&extra);

        // Action targets write custom variables directly on their build statement
        // (requiring 2 spaces indentation). Other targets write them as top-level
        // variables in their generated `.ninja` file (no indentation).
        let is_action = unsafe { (&*target.ptr()).is_action() };
        let indent_str = if is_action { "  " } else { "" };

        for (name, values) in target.providers().substitutions() {
            let mut expanded_values = Vec::new();
            for val in values {
                match val {
                    Either::Left(s) => {
                        expanded_values.push((*s).to_owned());
                    }
                    Either::Right(args_obj) => {
                        if let Ok((expanded_strings, _)) = args_obj.expand(&mut eval) {
                            expanded_values.extend(expanded_strings);
                        }
                    }
                }
            }

            out.push_str(indent_str);
            out.push_str(name);
            out.push_str(" =");
            for val in expanded_values {
                out.push(' ');
                out.push_str(&escape_for_ninja(&val));
            }
            out.push('\n');
        }

        Ok(())
    })
}

fn write_phonies(target: &Target, out: &mut String) {
    for (phony, deps) in &target.phonies {
        out.push_str("build ");
        out.push_str(&phony.ninja_escaped_path());
        out.push_str(": phony");
        for dep in deps {
            out.push(' ');
            out.push_str(&dep.ninja_escaped_path());
        }
        out.push('\n');
    }
}

/// Generates the custom Ninja code (phonies and substitutions) for a Starlark-defined target.
pub fn generate_custom_ninja(target: &Target) -> starlark::Result<String> {
    let mut custom_ninja = String::new();
    write_phonies(target, &mut custom_ninja);
    write_substitution_info(target, &mut custom_ninja)?;
    Ok(custom_ninja)
}
