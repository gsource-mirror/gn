use std::borrow::Cow;

use either::Either;
use starlark::{
    eval::Evaluator,
    typing::Ty,
    values::{UnpackValue, Value, ValueTyped, list::ListRef, type_repr::StarlarkTypeRepr},
};

use crate::{FrozenArgs, formatter::Formatter};

impl<'v> UnpackValue<'v> for Formatter {
    type Error = starlark::Error;

    fn unpack_value_impl(value: Value<'v>) -> Result<Option<Self>, Self::Error> {
        let s = <&'v str>::unpack_value_err(value)?;
        Self::new(s).map(Some)
    }
}

#[derive(Debug, Default)]
pub struct FrozenArgsSequence<'v>(pub Vec<Either<Cow<'v, str>, ValueTyped<'v, FrozenArgs<'v>>>>);

impl<'v> StarlarkTypeRepr for FrozenArgsSequence<'v> {
    type Canonical = starlark::values::Value<'v>;

    fn starlark_type_repr() -> Ty {
        Ty::list(Ty::any())
    }
}

impl<'v> UnpackValue<'v> for FrozenArgsSequence<'v> {
    type Error = starlark::Error;

    fn unpack_value_impl(value: Value<'v>) -> Result<Option<Self>, Self::Error> {
        // We unpack the list elements one by one, because when you pass [1] to
        // UnpackList<&str>, it says "expected list, but got list".
        // This way, it says "expected string | Args, but got int"
        Ok(Some(FrozenArgsSequence(
            <&ListRef>::unpack_value_err(value)?
                .iter()
                .map(<Either<&'v str, ValueTyped<'v, FrozenArgs<'v>>>>::unpack_value_err)
                .map(|elem| elem.map(|either| either.map_left(types::escape_ninja_token)))
                .collect::<Result<Vec<_>, _>>()?,
        )))
    }
}

impl<'v> FrozenArgsSequence<'v> {
    /// Writes the sequence of arguments into the given stream.
    pub fn write(
        &self,
        eval: &mut Evaluator<'v, '_, '_>,
        stream: &mut impl crate::expand::Ostream,
    ) -> starlark::Result<()> {
        let mut needs_space = false;
        for item in &self.0 {
            match item {
                Either::Left(s) => stream.write_token(s, &mut needs_space),
                Either::Right(args) => {
                    crate::expand::write(stream, args.as_ref(), eval, &mut needs_space)?;
                },
            }
        }
        Ok(())
    }
}
