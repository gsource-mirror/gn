// Copyright 2026 The GN Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.


use starlark::values::{Value, UnpackValue, Heap, Freeze, Freezer, FreezeResult, none::NoneOr};
use starlark::values::list::UnpackList;
use crate::label::Label;
use crate::session::EvalContext;
use crate::errors::Error;
use super::schema::AttrSchema;
use super::AttrKind;
use super::allow_files::AllowFiles;

use starlark::collections::SmallMap;
use allocative::Allocative;
use std::pin::Pin;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Allocative)]
pub enum LabelOrFile {
    Label(Label),
    File(std::path::PathBuf),
}

impl LabelOrFile {
    pub fn to_value<'v>(&'static self, heap: &Heap<'v>) -> Value<'v> {
        match self {
            Self::Label(l) => heap.alloc(l.clone()),
            Self::File(p) => heap.alloc(crate::file::File(p.as_path())),
        }
    }
}

/// Represents an actual parameter passed to a target.
/// Guarunteed to match the corresponding AttrSchema,
/// with the exception of allow_[single_]file, since at the time we resolve the
/// parameter, we haven't yet resolved the label to a Target, so don't know
/// what files it provides.
#[cfg_attr(test, derive(starlark::values::ProvidesStaticType, starlark_derive::NoSerialize))]
#[derive(Clone, Debug, PartialEq, Eq, Allocative)]
pub enum Attr {
    None,
    Bool(bool),
    Int(i32),
    String(String),
    IntList(Vec<i32>),
    StringList(Vec<String>),
    StringListDict(SmallMap<String, Vec<String>>),
    Label(LabelOrFile),
    LabelList(Vec<LabelOrFile>),
    StringDict(SmallMap<String, String>),
    LabelKeyedStringDict(SmallMap<LabelOrFile, String>),
    StringKeyedLabelDict(SmallMap<String, LabelOrFile>),
    LabelListDict(SmallMap<String, Vec<LabelOrFile>>),
}

impl std::fmt::Display for Attr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Freeze for Attr {
    type Frozen = Attr;

    fn freeze(self, _freezer: &Freezer) -> FreezeResult<Self::Frozen> {
        Ok(self)
    }
}

fn register_dependency(target: *mut crate::ffi::Target, context: &EvalContext, dep_lbl: &Label) {
    if target.is_null() {
        return;
    }

    let toolchain = context.current_toolchain();

    unsafe { crate::ffi::AddStarlarkTargetDependency(
        Pin::new_unchecked(&mut *target),
        dep_lbl.package.as_str(),
        dep_lbl.name.as_str(),
        toolchain.package.as_str(),
        toolchain.name,
    ) };
}

impl Attr {
    pub fn create<'v>(
        param_name: &str,
        schema: &AttrSchema,
        // Distinguish between an explicit = None and an implicit value not provided.
        value: Option<NoneOr<Value<'v>>>,
        target: *mut crate::ffi::Target,
        eval_context: &EvalContext,
    ) -> starlark::Result<Self> {
        match value {
            Some(NoneOr::Other(v)) => Self::create_without_defaults(param_name, schema, v, target, eval_context),
            Some(NoneOr::None) => {
                Self::create_without_defaults(param_name, schema, Value::new_none(), target, eval_context)
            }
            None => match &schema.default {
                    None => Err(
                        starlark::Error::from(Error::MandatoryAttribute {
                            param: param_name.to_owned(),
                        })
                    ),
                    Some(default) => Ok(default.clone())
               }
        }
    }

    pub fn create_without_defaults<'v>(
        param_name: &str,
        schema: &AttrSchema,
        value: Value<'v>,
        target: *mut crate::ffi::Target,
        eval_context: &EvalContext,
    ) -> starlark::Result<Self> {
        let package = eval_context.current_package().to_owned();
        let session = eval_context.session();

        let parse_label = |s: &str| -> starlark::Result<LabelOrFile> {
            let allowed = match &schema.allow_files {
                super::schema::AllowFilesSchema::None => &AllowFiles::None,
                super::schema::AllowFilesSchema::Single(a) | super::schema::AllowFilesSchema::Many(a) => a,
            };
            let lf = super::allow_files::parse_label_like(
                s,
                allowed,
                &package,
                session,
                target,
            )?;
            if let LabelOrFile::Label(lbl) = &lf {
                register_dependency(target, eval_context, lbl);
            }
            Ok(lf)
        };

        match &schema.kind {
            AttrKind::Bool => {
                let b = bool::unpack_named_param(value, param_name)?;
                Ok(Attr::Bool(b))
            }
            AttrKind::Int { allowed } => {
                let v = i32::unpack_named_param(value, param_name)?;
                if let Some(allowed) = allowed {
                    if !allowed.contains(&v) {
                        return Err(Error::IntNotAllowed(v).into());
                    }
                }
                Ok(Attr::Int(v))
            }
            AttrKind::String { allowed } => {
                let v = <String>::unpack_named_param(value, param_name)?;
                if let Some(allowed) = allowed {
                    if !allowed.contains(v.as_str()) {
                        return Err(Error::StringNotAllowed(v).into());
                    }
                }
                Ok(Attr::String(v))
            }
            AttrKind::IntList => {
                let list = UnpackList::<i32>::unpack_named_param(value, param_name)?;
                if schema.disallow_empty && list.items.is_empty() {
                    return Err(Error::EmptyListDisallowed.into());
                }
                Ok(Attr::IntList(list.items))
            }
            AttrKind::StringList => {
                let list = UnpackList::<String>::unpack_named_param(value, param_name)?;
                if schema.disallow_empty && list.items.is_empty() {
                    return Err(Error::EmptyListDisallowed.into());
                }
                Ok(Attr::StringList(list.items))
            }
            AttrKind::StringListDict => {
                let dict = SmallMap::<&str, UnpackList<String>>::unpack_named_param(value, param_name)?;
                if schema.disallow_empty && dict.is_empty() {
                    return Err(Error::EmptyDictDisallowed.into());
                }
                let res = dict.into_iter().map(|(k, v)| (k.to_string(), v.items)).collect();
                Ok(Attr::StringListDict(res))
            }
            AttrKind::Label => {
                match NoneOr::<&str>::unpack_named_param(value, param_name)? {
                    NoneOr::None => Ok(Attr::None),
                    NoneOr::Other(s) => {
                        let lf = parse_label(&s)?;
                        Ok(Attr::Label(lf))
                    }
                }
            }
            AttrKind::LabelList => {
                let list = UnpackList::<&str>::unpack_named_param(value, param_name)?;
                if schema.disallow_empty && list.items.is_empty() {
                    return Err(Error::EmptyListDisallowed.into());
                }
                Ok(Attr::LabelList(list.items.iter().map(|s| parse_label(s)).collect::<starlark::Result<Vec<_>>>()?))
            }
            AttrKind::StringDict => {
                let dict = SmallMap::<String, String>::unpack_named_param(value, param_name)?;
                if schema.disallow_empty && dict.is_empty() {
                    return Err(Error::EmptyDictDisallowed.into());
                }
                Ok(Attr::StringDict(dict))
            }
            AttrKind::LabelKeyedStringDict => {
                let dict = SmallMap::<&str, &str>::unpack_named_param(value, param_name)?;
                if schema.disallow_empty && dict.is_empty() {
                    return Err(Error::EmptyDictDisallowed.into());
                }
                Ok(Attr::LabelKeyedStringDict(
                    dict.into_iter()
                        .map(|(k, v)| {
                            let key = parse_label(k)?;
                            Ok((key, v.to_string()))
                        })
                        .collect::<starlark::Result<SmallMap<_, _>>>()?,
                ))
            }
            AttrKind::StringKeyedLabelDict => {
                let dict = SmallMap::<&str, &str>::unpack_named_param(value, param_name)?;
                if schema.disallow_empty && dict.is_empty() {
                    return Err(Error::EmptyDictDisallowed.into());
                }
                Ok(Attr::StringKeyedLabelDict(
                    dict.into_iter()
                        .map(|(k, v)| Ok((k.to_string(), parse_label(v)?)))
                        .collect::<starlark::Result<SmallMap<_, _>>>()?,
                ))
            }
            AttrKind::LabelListDict => {
                let dict = SmallMap::<&str, UnpackList<&str>>::unpack_named_param(value, param_name)?;
                if schema.disallow_empty && dict.is_empty() {
                    return Err(Error::EmptyDictDisallowed.into());
                }
                Ok(Attr::LabelListDict(
                    dict.into_iter()
                        .map(|(k, v)| {
                            let label_list = v.items.iter().map(|s| parse_label(s)).collect::<starlark::Result<Vec<_>>>();
                            Ok((k.to_string(), label_list?))
                        })
                        .collect::<starlark::Result<SmallMap<_, _>>>()?,
                ))
            }
        }
    }
}

// Allow Attr to be returned by starlark functions during a a.
#[cfg(test)]
starlark::starlark_simple_value!(Attr);

#[cfg(test)]
#[starlark_derive::starlark_value(type = "Attr")]
impl<'v> starlark::values::StarlarkValue<'v> for Attr {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Assert;
    use starlark::environment::GlobalsBuilder;
    use starlark::eval::Evaluator;
    use starlark_derive::starlark_module;

    #[starlark_module]
    fn test_globals(builder: &mut GlobalsBuilder) {
        fn validate<'v>(
            schema: &AttrSchema,
            value: Option<Value<'v>>,
            eval: &mut Evaluator<'v, '_, '_>,
        ) -> starlark::Result<Attr> {
            let val_opt = value.map(|v| if v.is_none() { NoneOr::None } else { NoneOr::Other(v) });
            let eval_context: &EvalContext = eval.into();
            Attr::create("$name", schema, val_opt, std::ptr::null_mut(), eval_context)
        }
    }

    fn attr_assert() -> Assert {
        let mut a = Assert::new();
        a.globals_add(|builder| {
            for (name, val) in crate::globals::make_globals().iter() {
                builder.set(name, val);
            }
            test_globals(builder);
        });
        a
    }

    // TODO: not all these test cases are correct, fix them.
    #[test]
    fn test_validate_attr_bool() {
        let a = attr_assert();
        a.eq("validate(attr.bool(), False)", &Attr::Bool(false));
        a.eq("validate(attr.bool(default=True))", &Attr::Bool(true));
        a.eq("validate(attr.bool(mandatory=True), True)", &Attr::Bool(true));
        a.fail("validate(attr.bool(mandatory=True))", "Attribute `$name` is mandatory");
        a.fail("validate(attr.bool(), None)", "expected `bool`, actual `NoneType");
    }

    #[test]
    fn test_validate_attr_int() {
        let a = attr_assert();
        a.eq("validate(attr.int())", &Attr::Int(0));
        a.eq("validate(attr.int(default=42))", &Attr::Int(42));
        a.eq("validate(attr.int(values=[1, 2], default=2))", &Attr::Int(2));
        a.fail("validate(attr.int(values=[1, 2], default=1), 3)", "Value 3 is not in allowed set");
        // Perhaps unintuitively, bazel allows this.
        a.eq("validate(attr.int(values=[1, 2]))", &Attr::Int(0));
        a.fail("validate(attr.int(mandatory=True))", "Attribute `$name` is mandatory");
    }

    #[test]
    fn test_validate_attr_int_list() {
        let a = attr_assert();
        a.eq("validate(attr.int_list())", &Attr::IntList(vec![]));
        a.fail("validate(attr.int_list(allow_empty=False, mandatory=True), [])", "Want non-empty list, got []");
        a.eq("validate(attr.int_list(), [1, 2, 3])", &Attr::IntList(vec![1, 2, 3]));
        a.fail("validate(attr.int_list(), [1, 'two', 3])", "expected `list[int]`");
    }

    #[test]
    fn test_validate_attr_string() {
        let a = attr_assert();
        a.eq("validate(attr.string())", &Attr::String("".to_owned()));
        a.eq("validate(attr.string(default='$name'))", &Attr::String("$name".to_owned()));
        a.eq("validate(attr.string(values=['a', 'b']))", &Attr::String("".to_owned()));
        a.fail("validate(attr.string(values=['a', 'b'], default='a'), 'c')", "Value \"c\" is not in allowed set");
    }

    #[test]
    fn test_validate_attr_string_dict() {
        let a = attr_assert();
        a.eq("validate(attr.string_dict())", &Attr::StringDict(SmallMap::new()));
        a.fail("validate(attr.string_dict(allow_empty=False, mandatory=True), {})", "Want non-empty dict, got {}");
        a.fail("validate(attr.string_dict(), {'a': 123})", "expected `dict[str, str]`");
    }

    #[test]
    fn test_validate_attr_string_list() {
        let a = attr_assert();
        a.eq("validate(attr.string_list())", &Attr::StringList(vec![]));
        a.fail("validate(attr.string_list(allow_empty=False, mandatory=True), [])", "Want non-empty list, got []");
        a.fail("validate(attr.string_list(), ['a', 123])", "expected `list[str]`");
    }

    #[test]
    fn test_validate_attr_string_list_dict() {
        let a = attr_assert();
        a.eq("validate(attr.string_list_dict())", &Attr::StringListDict(SmallMap::new()));
        a.fail("validate(attr.string_list_dict(allow_empty=False, mandatory=True), {})", "Want non-empty dict, got {}");
        a.fail("validate(attr.string_list_dict(), {'a': ['b', 123]})", "expected `dict[str, list[str]]`");
    }

    #[test]
    fn test_validate_attr_label() {
        let a = attr_assert();
        a.eq(
            "validate(attr.label(), '//pkg:bar')",
            &Attr::Label(LabelOrFile::Label(Label::new(
                crate::label::Package("//pkg".to_owned()),
                "bar".to_owned()
            ))),
        );
        a.eq(
            "validate(attr.label(default=':bar'))",
            &Attr::Label(LabelOrFile::Label(Label::new(
                crate::label::Package("//".to_owned()),
                "bar".to_owned()
            ))),
        );
    }

    #[test]
    fn test_validate_attr_label_list() {
        let a = attr_assert();
        a.eq(
            "validate(attr.label_list(), ['//pkg:bar', ':baz'])",
            &Attr::LabelList(vec![
                LabelOrFile::Label(Label::new(
                    crate::label::Package("//pkg".to_owned()),
                    "bar".to_owned()
                )),
                LabelOrFile::Label(Label::new(
                    crate::label::Package("//".to_owned()),
                    "baz".to_owned()
                ))
            ]),
        );
    }
}
