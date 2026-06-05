use starlark::{environment::GlobalsBuilder, eval::Evaluator, values::Value};
use starlark_derive::starlark_module;

use crate::errors::Error;

/// Registers the global `provider()` and `type()` override functions in
/// Starlark.
#[starlark_module]
pub(crate) fn register_providers_globals(builder: &mut GlobalsBuilder) {
    fn provider<'v>(
        #[starlark(require = named)] fields: Value<'v>,
        eval: &mut Evaluator<'v, '_, '_>,
    ) -> starlark::Result<Value<'v>> {
        let mut field_names = Vec::new();
        for field in fields.iterate(eval.heap())? {
            let s = field.unpack_str().ok_or(Error::FieldsMustBeStrings)?;
            field_names.push(s.to_owned());
        }

        let provider_type = crate::provider_type::ProviderType::new(field_names);
        Ok(eval.heap().alloc_complex(provider_type))
    }
}

pub fn register_providers(builder: &mut GlobalsBuilder) {
    register_providers_globals(builder);
}
