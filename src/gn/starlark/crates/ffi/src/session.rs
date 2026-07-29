// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::pin::Pin;

use allocative::Allocative;
use loader::FileLoader;
use starlark::environment::{FrozenModule, Globals};
use types::{Label, LabelRef, PackageRef, PathResolver, Session as TypesSession};

/// Represents a Starlark evaluation session exposed to C++ via FFI.
#[derive(Allocative)]
pub struct Session {
    #[allocative(skip)]
    loader: FileLoader,
    #[allocative(skip)]
    pub(crate) path_resolver: PathResolver,
    globals: Globals,
}

fn build_globals() -> Globals {
    let mut builder = starlark::environment::GlobalsBuilder::new();
    providers::globals::register_providers(&mut builder);
    depset::depset_globals!(&mut builder, crate::eval_context::EvalContext);
    rule::register_rule_globals!(&mut builder, crate::eval_context::EvalContext);
    builder.build()
}

impl Session {
    /// Creates a new `Session`.
    pub fn from_resolver(path_resolver: PathResolver) -> Self {
        Self {
            loader: FileLoader::default(),
            path_resolver,
            globals: build_globals(),
        }
    }

    /// Associated function for C++ constructor.
    pub fn new(source_root: &str, source_root_rel: &str) -> Box<Self> {
        Box::new(Self::from_resolver(PathResolver::new(
            std::path::PathBuf::from(source_root),
            source_root_rel.to_owned(),
        )))
    }

    /// Associated function for C++ constructor.
    pub fn new_for_testing() -> Box<Self> {
        Box::new(Self::from_resolver(PathResolver::new_for_testing()))
    }

    fn load(&self, label: LabelRef<'_>) -> starlark::Result<FrozenModule> {
        self.loader
            .load(label, &self.path_resolver, &self.globals, &|pkg| {
                crate::eval_context::EvalContext::new_bzl_file(self, pkg)
            })
    }

    /// Loads a Starlark module and populates multiple values by key.
    pub fn load_values(
        &self,
        label: &str,
        values: &mut [crate::bridge::KeyValueMut],
        mut err: Pin<&mut crate::bridge::Err>,
    ) {
        let label = match Label::parse(label, PackageRef::root()) {
            Ok(l) => l,
            Err(e) => {
                crate::bridge::PopulateErrWithMessage(
                    err.as_mut(),
                    &format!("Failed to parse label '{}'", label),
                    &e.to_string(),
                );
                return;
            },
        };

        let module = match err.as_mut().handle(self.load(label.as_ref())) {
            Some(m) => m,
            None => return,
        };

        for item in values.iter_mut() {
            let value = match module.get(item.key) {
                Ok(v) => v,
                Err(e) => {
                    crate::bridge::PopulateErrWithMessage(
                        err.as_mut(),
                        &format!("Key '{}' not found in module '{}'", item.key, label),
                        &e.to_string(),
                    );
                    return;
                },
            };
            item.value
                .as_mut()
                .assign(value.value(), std::ptr::null_mut(), std::ptr::null());
        }
    }
}

impl TypesSession for Session {
    type TargetRef = crate::target::TargetRef;

    fn get_target(&self, _label: LabelRef<'_>, _toolchain: LabelRef<'_>) -> Self::TargetRef {
        todo!()
    }

    fn register_dependency<'a>(
        &self,
        _source: Self::TargetRef,
        _label: LabelRef<'a>,
        _toolchain: LabelRef<'a>,
    ) {
        todo!()
    }
}
