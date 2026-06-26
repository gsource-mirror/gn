// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::borrow::Cow;
use std::path::Path;

use starlark::collections::StarlarkHasher;
use starlark::environment::Methods;
use starlark::environment::MethodsBuilder;
use starlark::environment::MethodsStatic;
use starlark::starlark_simple_value;
use starlark::values::Freeze;
use starlark::values::FreezeResult;
use starlark::values::Freezer;
use starlark::values::ProvidesStaticType;
use starlark::values::StarlarkValue;
use starlark::values::Trace;
use starlark::values::Tracer;
use starlark::values::ValueLike;
use starlark_derive::starlark_module;
use starlark_derive::starlark_value;
use starlark_derive::NoSerialize;

/// File contains a path relative to the build directory.
/// It is an interned string (via StringAtom) and thus is safely 'static.
///
/// Note that while GN has two forms of a file, SourceFile and OutputFile,
/// we only have one. All files are OutputFiles.
///
/// We use a &str instead of an &Path, as would be standard in rust, because:
/// * Path::new(&str) is a zero cost transmute and always succeeds
/// * Path::new(&str).to_string_lossy() always returns the input str but has to perform an error check.
#[derive(Clone, Debug, ProvidesStaticType, NoSerialize, allocative::Allocative)]
pub struct File(#[allocative(skip)] &'static str);

starlark_simple_value!(File);

impl File {
    /// Creates a `File` from a raw static string slice originating from C++.
    pub fn from_cxx(path: &'static str) -> Self {
        Self(path)
    }

    /// Creates a `File` from a Rust `String` by interning it into a static string slice.
    pub fn from_rust(s: String) -> Self {
        extern "C" {
            fn intern_string(s: &str) -> &'static str;
        }
        // Safety: Just an ffi function
        Self(unsafe { intern_string(&s) })
    }

    /// Returns the file path as a &Path for use in rust.
    pub fn as_path(&self) -> &'static Path {
        // Note: This is a zero-cost operation.
        // It's basically just a reinterpret cast.
        Path::new(self.0)
    }

    /// Returns the file path as a &str for use in C++.
    pub fn as_str(&self) -> &'static str {
        self.0
    }

    /// Returns the file path with proper escaping applied for use in the inputs section of a ninja file.
    pub fn ninja_escaped_path(&self) -> Cow<'static, str> {
        let s = self.0;
        if s.contains(|c| c == ' ' || c == '$' || c == ':') {
            let mut result = String::with_capacity(s.len());
            for c in s.chars() {
                if c == '$' || c == ' ' || c == ':' {
                    result.push('$');
                }
                result.push(c);
            }
            Cow::Owned(result)
        } else {
            Cow::Borrowed(s)
        }
    }
}

// Because we do string interning, we can make PartialEq and Hashing based on
// pointers, making storing files in depsets extremely fast.
impl PartialEq for File {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.0, other.0)
    }
}
impl Eq for File {}

impl std::hash::Hash for File {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::ptr::hash(self.0, state);
    }
}

impl std::fmt::Display for File {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

unsafe impl<'v> Trace<'v> for File {
    fn trace(&mut self, _tracer: &Tracer<'v>) {}
}

impl Freeze for File {
    type Frozen = File;
    fn freeze(self, _freezer: &Freezer) -> FreezeResult<Self::Frozen> {
        Ok(self)
    }
}

#[starlark_value(type = "File")]
impl<'v> StarlarkValue<'v> for File {
    fn get_methods() -> Option<&'static Methods>
    where
        Self: Sized,
    {
        static RES: MethodsStatic = MethodsStatic::new("File", file_methods);
        Some(RES.methods())
    }

    fn write_hash(&self, hasher: &mut StarlarkHasher) -> starlark::Result<()> {
        use std::hash::Hash;
        self.hash(hasher);
        Ok(())
    }

    fn equals(&self, other: starlark::values::Value<'v>) -> starlark::Result<bool> {
        Ok(other.downcast_ref::<File>().is_some_and(|o| self == o))
    }

    fn collect_repr(&self, collector: &mut String) {
        use std::fmt::Write;
        write!(collector, "{:?}", self).unwrap();
    }

    fn collect_str(&self, collector: &mut String) {
        use std::fmt::Write;
        write!(collector, "{}", self).unwrap();
    }
}

#[starlark_module]
fn file_methods(methods: &mut MethodsBuilder) {
    #[starlark(attribute)]
    fn basename<'v>(this: &'v File) -> starlark::Result<&'static str> {
        Ok(this
            .as_path()
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(""))
    }

    #[starlark(attribute)]
    fn dirname<'v>(this: &'v File) -> starlark::Result<&'static str> {
        Ok(this
            .as_path()
            .parent()
            .and_then(|s| s.to_str())
            .unwrap_or(""))
    }

    #[starlark(attribute)]
    fn extension<'v>(this: &'v File) -> starlark::Result<&'static str> {
        Ok(this
            .as_path()
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or(""))
    }

    #[starlark(attribute)]
    fn is_source(this: &File) -> starlark::Result<bool> {
        Ok(this.as_path().starts_with(".."))
    }

    #[starlark(attribute)]
    fn path<'v>(this: &'v File) -> starlark::Result<&'static str> {
        Ok(this.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_interning() {
        let f1 = File::from_rust("foo/bar.txt".to_owned());
        let f2 = File::from_rust("foo/bar.txt".to_owned());

        assert_eq!(f1.as_str(), "foo/bar.txt");
        // Verify pointer equality (they point to the exact same static string backing memory)
        assert!(std::ptr::eq(f1.as_str(), f2.as_str()));
    }

    #[test]
    fn test_file_equality() {
        let f1 = File::from_rust("foo/bar.txt".to_owned());
        let f2 = File::from_rust("foo/bar.txt".to_owned());
        let f3 = File::from_rust("other.txt".to_owned());

        assert_eq!(f1, f2);
        assert_ne!(f1, f3);
    }

    #[test]
    fn test_file_starlark_api() {
        let mut a = starlark::assert::Assert::new();
        a.globals_add(move |builder| {
            builder.set(
                "source_file",
                File::from_rust("../foo/bar/baz.txt".to_owned()),
            );
            builder.set(
                "generated_file",
                File::from_rust("foo/bar/baz.txt".to_owned()),
            );
        });
        a.eq("source_file.basename", "\"baz.txt\"");
        a.eq("source_file.dirname", "\"../foo/bar\"");
        a.eq("source_file.extension", "\"txt\"");
        a.eq("source_file.is_source", "True");
        a.eq("source_file.path", "\"../foo/bar/baz.txt\"");
        a.eq("str(source_file)", "\"../foo/bar/baz.txt\"");
        a.eq("repr(source_file)", "'File(\"../foo/bar/baz.txt\")'");

        a.eq("generated_file.basename", "\"baz.txt\"");
        a.eq("generated_file.dirname", "\"foo/bar\"");
        a.eq("generated_file.extension", "\"txt\"");
        a.eq("generated_file.is_source", "False");
        a.eq("generated_file.path", "\"foo/bar/baz.txt\"");
        a.eq("str(generated_file)", "\"foo/bar/baz.txt\"");
        a.eq("repr(generated_file)", "'File(\"foo/bar/baz.txt\")'");
    }
}

#[allow(dead_code)]
fn dummy_to_force_cxx_linking() {
    // The C++ code depends on the cxx crate being linked for the rust::Str type.
    // Because the rust code doesn't depend on the cxx crate, we need a dummy
    // function that uses some part of cxx to ensure it links.
    drop(cxx::UniquePtr::<cxx::CxxString>::null());
}
