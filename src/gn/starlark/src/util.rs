use cxx::CxxString;
use starlark::environment::FrozenModule;

#[cfg(test)]
use starlark::environment::Module;
#[cfg(test)]
use crate::Error;
#[cfg(test)]
use starlark::eval::Evaluator;
#[cfg(test)]
use starlark::syntax::{AstModule, Dialect};
#[cfg(test)]
use starlark::values::FrozenHeapName;
#[cfg(test)]
use std::fs;

pub enum Choice<A, B> {
    Left(A),
    Right(B),
    Both(A, B),
}

pub fn zip_sorted<A, B, F>(
    left: impl IntoIterator<Item = A>,
    right: impl IntoIterator<Item = B>,
    mut compare: F,
) -> impl Iterator<Item = Choice<A, B>>
where
    F: FnMut(&A, &B) -> std::cmp::Ordering,
{
    let mut left = left.into_iter().peekable();
    let mut right = right.into_iter().peekable();

    std::iter::from_fn(move || match (left.peek(), right.peek()) {
        (Some(l), Some(r)) => match compare(l, r) {
            std::cmp::Ordering::Less => Some(Choice::Left(left.next().unwrap())),
            std::cmp::Ordering::Greater => Some(Choice::Right(right.next().unwrap())),
            std::cmp::Ordering::Equal => {
                Some(Choice::Both(left.next().unwrap(), right.next().unwrap()))
            }
        },
        (Some(_), None) => Some(Choice::Left(left.next().unwrap())),
        (None, Some(_)) => Some(Choice::Right(right.next().unwrap())),
        (None, None) => None,
    })
}

#[macro_export]
macro_rules! assert_err_contains {
    ($cond:expr, $pattern:expr) => {
        match $cond {
            Ok(v) => panic!("Expected Err, got Ok: {:?}", v),
            Err(e) => {
                let err_str = format!("{}", e);
                assert!(
                    err_str.contains($pattern),
                    "Error string did not contain '{}'. Got: '{}'",
                    $pattern,
                    err_str
                );
            }
        }
    };
}

/// Like std::mem::transmute, but limited to only touching the lifetime, rather than the type.
#[allow(dead_code)]
pub(crate) unsafe fn extend_lifetime<'to, 'from: 'from, T: ?Sized>(val: &'from T) -> &'to T {
    unsafe { std::mem::transmute(val) }
}

pub(crate) unsafe fn from_utf8_unchecked(s: &CxxString) -> &str {
    unsafe { std::str::from_utf8_unchecked(s.as_bytes()) }
}

#[cfg(test)]
pub(crate) fn run_starlark_test(code: &str) {
    let assertions_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("testdata/assertions.bzl");
    let assertions = fs::read_to_string(assertions_path).unwrap();
    run_starlark_code(&(assertions + "\n" + code)).unwrap();
}

#[cfg(test)]
#[cfg(test)]
pub(crate) fn run_starlark_code(code: &str) -> Result<FrozenModule, starlark::Error> {
    let ast = AstModule::parse("test.bzl", code.to_owned(), &Dialect::Extended)?;
    let globals = crate::globals::make_globals();
    let setup = crate::ffi::TestWithScope::new();
    let scope_ptr = setup.scope();
    let scope_wrapper = crate::session::EvalContext::new(
        scope_ptr,
        std::ptr::null(),
        crate::session::EvalKind::BzlFile(crate::label::Package("//".to_owned())),
    );

    let session_ptr = unsafe { crate::ffi::GetStarlarkSessionFromScope(scope_ptr) };
    let ffi_session = unsafe { &*session_ptr };
    use crate::ffi::AsRust;
    let rust_session = ffi_session.as_rust();

    let module = Module::with_temp_heap(|module| -> Result<FrozenModule, starlark::Error> {
        {
            let mut eval = Evaluator::new(&module);
            eval.extra = Some(&scope_wrapper);
            eval.eval_module(ast, globals)?;
        }
        module
            .freeze_named(FrozenHeapName::User(Box::new("test.bzl".to_owned())))
            .map_err(|e| starlark::Error::new_other(e))
    })?;
    register_targets_from_module(rust_session, &module);
    Ok(module)
}

pub(crate) fn register_targets_from_module(_session: &crate::session::StarlarkSession, _module: &FrozenModule) {}


#[cfg(test)]
#[allow(dead_code)]
pub(crate) fn eval_starlark(
    expr: &str,
) -> Result<starlark::values::OwnedFrozenValue, starlark::Error> {
    run_starlark_code(&format!("a = {expr}"))?
        .get("a")
        .map_err(|e| Error::GetFailed(e.to_string()).into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zip_sorted() {
        let left = vec!["a", "c", "d"];
        let right = vec!["a", "b", "d", "e"];
        let mut results = zip_sorted(left, right, |l, r| l.cmp(r));

        assert!(matches!(results.next(), Some(Choice::Both("a", "a"))));
        assert!(matches!(results.next(), Some(Choice::Right("b"))));
        assert!(matches!(results.next(), Some(Choice::Left("c"))));
        assert!(matches!(results.next(), Some(Choice::Both("d", "d"))));
        assert!(matches!(results.next(), Some(Choice::Right("e"))));
        assert!(matches!(results.next(), None));
    }
}
