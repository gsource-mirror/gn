// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use allocative::Allocative;
use starlark::environment::{Methods, MethodsBuilder, MethodsStatic};
use starlark::eval::Evaluator;
use starlark::values::{
    Coerce, Freeze, FreezeResult, Freezer, ProvidesStaticType, StarlarkValue, Trace, Value,
};
use starlark_derive::{starlark_module, starlark_value, NoSerialize};
use std::fmt::{self, Display, Formatter};
use starlark::starlark_complex_value;
use crate::ffi::ToRust;
use crate::target::TargetRef;

/// ctx.gn provides access to GN internals of a target.
/// It is only set for rule extension targets.
#[derive(Debug, Trace, Coerce, ProvidesStaticType, NoSerialize, Allocative)]
#[repr(C)]
pub struct GnGen<V> {
    #[allocative(skip)]
    target: TargetRef,
    phantom: std::marker::PhantomData<V>,
}

unsafe impl<V> Send for GnGen<V> {}
unsafe impl<V> Sync for GnGen<V> {}

starlark_complex_value!(pub Gn);

impl<'v, V: starlark::values::ValueLike<'v>> Display for GnGen<V>
where
    Self: ProvidesStaticType<'v>,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "gn")
    }
}

#[starlark_value(type = "gn")]
impl<'v, V: starlark::values::ValueLike<'v>> StarlarkValue<'v> for GnGen<V>
where
    Self: ProvidesStaticType<'v>,
{
    fn get_methods() -> Option<&'static Methods> {
        static RES: MethodsStatic = MethodsStatic::new("gn", gn_methods);
        Some(RES.methods())
    }
}

impl<'v> Freeze for Gn<'v> {
    type Frozen = FrozenGn;
    fn freeze(self, _freezer: &Freezer) -> FreezeResult<Self::Frozen> {
        Ok(GnGen {
            target: self.target,
            phantom: std::marker::PhantomData,
        })
    }
}

#[starlark_module]
pub fn gn_methods(builder: &mut MethodsBuilder) {
    fn get_output_files<'v>(
        this: &Gn<'v>,
        eval: &mut Evaluator<'v, '_, '_>,
    ) -> starlark::Result<Value<'v>> {
        let files = this.target.outputs();
        let files_val = files.iter().map(|f| eval.heap().alloc(f.clone())).collect::<Vec<_>>();
        Ok(eval.heap().alloc(files_val))
    }

    fn deps<'v>(
        this: &Gn<'v>,
        eval: &mut Evaluator<'v, '_, '_>,
    ) -> starlark::Result<Value<'v>> {
        let deps_cxx = unsafe { crate::ffi::GetTargetDeps(&*this.target.ptr()) };
        use crate::ffi::ToRust;
        
        let deps_val = deps_cxx
            .iter()
            .map(|t_ptr| eval.heap().alloc(t_ptr.to_rust()))
            .collect::<Vec<_>>();
        Ok(eval.heap().alloc(deps_val))
    }

    fn public_deps<'v>(
        this: &Gn<'v>,
        eval: &mut Evaluator<'v, '_, '_>,
    ) -> starlark::Result<Value<'v>> {
        let deps_cxx = unsafe { crate::ffi::GetTargetPublicDeps(&*this.target.ptr()) };
        use crate::ffi::ToRust;
        
        let deps_val = deps_cxx
            .iter()
            .map(|t_ptr| eval.heap().alloc(t_ptr.to_rust()))
            .collect::<Vec<_>>();
        Ok(eval.heap().alloc(deps_val))
    }

    fn sources<'v>(
        this: &Gn<'v>,
        eval: &mut Evaluator<'v, '_, '_>,
    ) -> starlark::Result<Value<'v>> {
        let sources_cxx = unsafe { crate::ffi::GetTargetPrivateSources(&*this.target.ptr()) };
        let files = sources_cxx
            .iter()
            .map(|s| {
                eval.heap().alloc(crate::file::File(std::path::Path::new(s.to_rust())))
            })
            .collect::<Vec<_>>();

        Ok(eval.heap().alloc(files))
    }

    fn public<'v>(
        this: &Gn<'v>,
        eval: &mut Evaluator<'v, '_, '_>,
    ) -> starlark::Result<Value<'v>> {
        let public_cxx = unsafe { crate::ffi::GetTargetPublicSources(&*this.target.ptr()) };
        let files = public_cxx
            .iter()
            .map(|s| {
                eval.heap().alloc(crate::file::File(std::path::Path::new(s.to_rust())))
            })
            .collect::<Vec<_>>();

        Ok(eval.heap().alloc(files))
    }
}

impl<V> GnGen<V> {
    pub fn new(target: TargetRef) -> Self {
        Self {
            target,
            phantom: std::marker::PhantomData,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use starlark::environment::Module;
    use starlark::eval::Evaluator;
    use std::path::PathBuf;

    #[test]
    fn test_gn_get_output_files() {
        let setup = crate::ffi::TestWithScope::new();
        let scope_ptr = setup.scope();

        let out = crate::ffi::Value::new();
        autocxx::prelude::moveit!(let mut out_pin = out);
        unsafe {
            crate::ffi::InitializeTargetScope(
                out_pin.as_mut(),
                scope_ptr,
            );
        }

        let err_ctor = crate::ffi::Err::new();
        autocxx::prelude::moveit!(let mut err = err_ctor);
        let target_ptr = unsafe {
            crate::ffi::CreateTarget(
                "".into(),
                "my_target".into(),
                std::ptr::null(),
                out_pin.as_mut(),
                err.as_mut().get_mut() as *mut crate::ffi::Err,
            )
        };
        assert!(!target_ptr.is_null());

        let rust_target = crate::target::Target::new_cxx(std::ptr::NonNull::new(target_ptr).unwrap());
        let session = crate::session::StarlarkSession::new(PathBuf::new(), PathBuf::new());
        let target_ref = session.register_target(rust_target);

        Module::with_temp_heap(|module| {
            let mut eval = Evaluator::new(&module);
            
            let gn_gen = GnGen::new(target_ref);
            let gn_val = module.heap().alloc(gn_gen);
            
            let method = gn_val.get_attr("get_output_files", module.heap()).unwrap().unwrap();
            let res = eval.eval_function(method, &[], &[]).unwrap();
            
            let list = starlark::values::list::ListRef::from_value(res).unwrap();
            assert_eq!(list.len(), 0);
        });
    }
}
