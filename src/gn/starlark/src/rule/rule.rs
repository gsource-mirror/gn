// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use allocative::Allocative;
use starlark::collections::SmallMap;
use starlark::eval::{Arguments, Evaluator, ParametersSpec};
use starlark::values::{
    Freeze, FreezeResult, Freezer, ProvidesStaticType, StarlarkValue, Trace, Value,
    ValueLike,
};
use starlark_derive::{starlark_value, NoSerialize};
use std::fmt::{self, Display, Formatter};

use crate::attr::{Attr, AttrSchema};
use crate::label::Label;
use crate::Error;
use starlark::starlark_complex_value;
use std::ptr::NonNull;

#[derive(Debug, Trace, ProvidesStaticType, NoSerialize, Allocative)]
#[repr(C)]
pub struct RuleCallableGen<V> {
    pub implementation: V,
    pub attrs: SmallMap<String, AttrSchema>,
    pub is_extension: bool,
    pub signature: ParametersSpec<V>,
}

unsafe impl<From, To> starlark::coerce::Coerce<RuleCallableGen<To>> for RuleCallableGen<From>
where
    From: starlark::coerce::Coerce<To>,
{}

starlark_complex_value!(pub RuleCallable);

impl<'v, V: ValueLike<'v>> Display for RuleCallableGen<V>
where
    Self: ProvidesStaticType<'v>,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if self.is_extension {
            write!(f, "rule_extension(...)")
        } else {
            write!(f, "rule(...)")
        }
    }
}

impl<'v> Freeze for RuleCallable<'v> {
    type Frozen = FrozenRuleCallable;
    fn freeze(self, freezer: &Freezer) -> FreezeResult<Self::Frozen> {
        let implementation = self.implementation.freeze(freezer)?;
        let attrs = self.attrs.freeze(freezer)?;
        let signature = self.signature.freeze(freezer)?;
        Ok(RuleCallableGen {
            implementation,
            attrs,
            is_extension: self.is_extension,
            signature,
        })
    }
}

impl<'v, V: ValueLike<'v>> RuleCallableGen<V>
where
    Self: ProvidesStaticType<'v>,
{
    fn get_or_create_target_ptr(
        &self,
        parser: &mut starlark::eval::ParametersParser<'v, '_>,
        eval: &mut Evaluator<'v, '_, '_>,
    ) -> starlark::Result<*mut crate::ffi::Target> {
        let target_opt: Option<Value<'v>> = parser.next_opt()?;
        let name_opt: Option<Value<'v>> = parser.next_opt()?;

        if self.is_extension {
            let target_val = target_opt.ok_or_else(|| Error::ExtensionTargetRequired)?;
            let target_ptr = if let Some(target_obj) = target_val.downcast_ref::<crate::target::TargetRef>() {
                target_obj.ptr()
            } else {
                return Err(Error::ArgumentToRuleMustBeTarget.into());
            };
            Ok(target_ptr)
        } else {
            let target_name = if let Some(name_val) = name_opt {
                name_val.unpack_str()
            } else if let Some(target_val) = target_opt {
                target_val.unpack_str()
            } else {
                None
            };
            let target_name = target_name.ok_or_else(|| Error::CustomRuleNameRequired)?;

            let extra: &crate::session::EvalContext = eval.into();
            let out = crate::ffi::Value::new();
            autocxx::prelude::moveit!(let mut out_pin = out);
            unsafe {
                crate::ffi::InitializeTargetScope(
                    out_pin.as_mut(),
                    extra.scope,
                );
            }

            let err_ctor = crate::ffi::Err::new();
            autocxx::prelude::moveit!(let mut err = err_ctor);
            let created_ptr = unsafe {
                crate::ffi::CreateTarget(
                    "".into(),
                    target_name.into(),
                    extra.origin,
                    out_pin.as_mut(),
                    err.as_mut().get_mut() as *mut crate::ffi::Err,
                )
            };

            if created_ptr.is_null() {
                let err_msg = crate::ffi::GetErrorMessage(err.as_ref().get_ref());
                let err_str = err_msg.to_str().unwrap_or_default();
                return Err(Error::TargetCreationError(err_str.to_owned()).into());
            }

            Ok(created_ptr as *mut crate::ffi::Target)
        }
    }
}

#[starlark_value(type = "rule_callable")]
impl<'v, V: ValueLike<'v>> StarlarkValue<'v> for RuleCallableGen<V>
where
    Self: ProvidesStaticType<'v>,
{
    fn invoke(
        &self,
        _me: Value<'v>,
        args: &Arguments<'v, '_>,
        _eval: &mut Evaluator<'v, '_, '_>,
    ) -> starlark::Result<Value<'v>> {
        self.signature.parser(args, _eval, |parser, _eval| {
            let mut attrs_map = starlark::collections::SmallMap::new();

            let created_ptr = self.get_or_create_target_ptr(parser, _eval)?;

            let extra: &crate::session::EvalContext = _eval.into();
            let caller_pkg = extra.current_package().to_owned();

            for (name, schema) in &self.attrs {
                // The ordering of the signature is the same as that of the attrs so we can safely call next_opt.
                let attr = Attr::create(&name, schema, parser.next_opt()?, created_ptr, extra)?;
                let static_name: &'static str = unsafe { crate::util::extend_lifetime(name.as_str()) };
                attrs_map.insert(static_name, attr);
            }

            let non_null_created = NonNull::new(created_ptr).unwrap();
            let frozen_me = _me.unpack_frozen().unwrap();
            let typed_me = starlark::values::FrozenValueTyped::new(frozen_me).unwrap();
            let rust_target = crate::target::Target::new_starlark(non_null_created, typed_me, attrs_map);
            let session = extra.session();
            let label = rust_target.label().to_owned();
            let toolchain = rust_target.toolchain().to_owned();
            session.register_target(rust_target);
            let target_ref = session.get_target_by_label(label.as_ref(), toolchain.as_ref());
            let target_val = _eval.heap().alloc(target_ref);

            {
                let module = _eval.module();
                let heap = _eval.heap();
                let extra_val = module.extra_value();
                let mut targets = if let Some(extra_val) = extra_val {
                    let list = starlark::values::list::ListRef::from_value(extra_val).unwrap();
                    list.iter().collect::<Vec<_>>()
                } else {
                    Vec::new()
                };
                targets.push(target_val);
                module.set_extra_value(heap.alloc(starlark::values::list::AllocList(targets)));

                Ok(if self.is_extension {
                    target_val
                } else {
                    let cpp_name: String = unsafe {
                        let label = &*crate::ffi::GetTargetLabel(&*created_ptr);
                        let name_cxx = label.name();
                        let name_str = name_cxx.to_str().unwrap();
                        if let Some((_, name)) = name_str.rsplit_once(':') {
                            name.to_owned()
                        } else {
                            name_str.to_owned()
                        }
                    };
                    let label_obj = Label::new(caller_pkg, cpp_name);
                    _eval.heap().alloc(label_obj)
                })
            }
        })
    }
}
