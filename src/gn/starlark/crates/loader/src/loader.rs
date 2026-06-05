// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fs;
use std::pin::Pin;
use std::sync::{Arc, Condvar, Mutex, RwLock};

use starlark::environment::FrozenModule;
use starlark::environment::Globals;
use starlark::environment::Module;
use starlark::eval::Evaluator;
use starlark::eval::FileLoader as StarlarkFileLoader;
use starlark::syntax::{AstModule, Dialect};
use starlark::values::AnyLifetime;
use starlark::values::FrozenHeapName;
use types::PackageRef;

use crate::Error;

/// The Starlark compilation dialect used for `.bzl` configuration files.
pub const BZL_FILE_DIALECT: Dialect = Dialect {
    enable_lambda: false,
    enable_load_reexport: false,
    ..Dialect::Standard
};

enum FileStatus {
    Loading {
        // A CondVar to wait on for the filestatus to resolve to Loaded.
        wait: Arc<Condvar>,
        // What needs to finish evaluating before this can be evaluated.
        // This is used purely for cycle detection.
        needs: Option<String>,
    },
    Loaded(starlark::Result<Pin<Box<FrozenModule>>>),
}

impl Default for FileStatus {
    fn default() -> Self {
        Self::Loading {
            wait: Arc::new(Condvar::new()),
            needs: None,
        }
    }
}

fn set_needs(file_status: &Arc<Mutex<FileStatus>>, value: &str) {
    let mut status = file_status.lock().unwrap();
    if let FileStatus::Loading { ref mut needs, .. } = &mut *status {
        *needs = Some(value.to_owned());
    }
}

/// A thread-safe loader for reading, compiling, and caching Starlark modules (`.bzl` files).
pub struct FileLoader {
    path_resolver: types::PathResolver,
    files: RwLock<HashMap<String, Arc<Mutex<FileStatus>>>>,
}

impl FileLoader {
    /// Creates a new `FileLoader` with the given path resolver.
    pub fn new(path_resolver: types::PathResolver) -> Self {
        Self {
            path_resolver,
            files: Default::default(),
        }
    }

    /// Returns a reference to the path resolver used by this loader.
    pub fn path_resolver(&self) -> &types::PathResolver {
        &self.path_resolver
    }

    fn wait_for_load<'a>(
        &'a self,
        file_status: &Arc<Mutex<FileStatus>>,
    ) -> starlark::Result<Pin<&'a FrozenModule>> {
        let mut status = file_status.lock().unwrap();
        while let FileStatus::Loading { wait, .. } = &*status {
            let wait = wait.clone();
            status = wait.wait(status).unwrap();
        }
        self.get_loaded(&status)
    }

    fn set_complete<'a>(
        &'a self,
        file_status: &Arc<Mutex<FileStatus>>,
        result: starlark::Result<FrozenModule>,
    ) -> starlark::Result<Pin<&'a FrozenModule>> {
        let mut status = file_status.lock().unwrap();
        if let FileStatus::Loading { wait, .. } = &*status {
            wait.notify_all();
        }
        *status = FileStatus::Loaded(result.map(Box::pin));
        self.get_loaded(&status)
    }

    fn get_loaded<'a>(&'a self, status: &FileStatus) -> starlark::Result<Pin<&'a FrozenModule>> {
        match status {
            FileStatus::Loading { .. } => unreachable!(),
            // Safety: By design, we never overwrite entries in the file hash map, so this is safe.
            FileStatus::Loaded(Ok(m)) => Ok(unsafe {
                std::mem::transmute::<Pin<&FrozenModule>, Pin<&'a FrozenModule>>(m.as_ref())
            }),
            // starlark::Error doesn't implement Clone, so we do a poor man's clone.
            FileStatus::Loaded(Err(e)) => Err(starlark::Error::new_other(anyhow::anyhow!("{e}"))),
        }
    }

    /// Loads, parses, compiles, and evaluates a Starlark module, resolving dependencies recursively and caching the result.
    pub fn load<'a>(
        &'a self,
        path: &str,
        package: &PackageRef,
        globals: &Globals,
        make_extra: &dyn Fn(&PackageRef) -> Option<Box<dyn AnyLifetime<'static>>>,
    ) -> starlark::Result<Pin<&'a FrozenModule>> {
        let label = types::Label::parse(path, package)?;
        let file_status = {
            let mut loader = self.files.write().unwrap();
            match loader.entry(path.to_owned()) {
                Entry::Occupied(entry) => {
                    let file_status = entry.get().clone();
                    drop(loader);
                    return self.wait_for_load(&file_status);
                }
                // We're about to start evaluating it.
                Entry::Vacant(entry) => entry.insert(Default::default()).clone(),
            }
        };

        // Read and parse the file to get its dependencies.
        let absolute_path = self
            .path_resolver
            .absolute_path(label.package(), label.name());
        let content =
            fs::read_to_string(&absolute_path).map_err(|_| Error::ReadFailed(label.to_string()))?;
        let ast = AstModule::parse(path, content, &BZL_FILE_DIALECT)?;

        let mut deps: Vec<(String, Pin<&FrozenModule>)> = Default::default();
        if !ast.loads().is_empty() {
            for load in ast.loads() {
                set_needs(&file_status, load.module_id);
                if let Some(cycle) = self.find_cycle_path(path, load.module_id) {
                    return self
                        .set_complete(&file_status, Err(Error::CycleDetected(cycle).into()));
                }

                deps.push((
                    load.module_id.to_owned(),
                    self.load(load.module_id, label.package(), globals, make_extra)?,
                ));
            }
        }

        let deps_map: HashMap<&str, &FrozenModule> = deps
            .iter()
            .map(|(k, v)| (k.as_str(), v.get_ref()))
            .collect();

        let loader = PreloadedLoader { modules: &deps_map };
        let module = Module::with_temp_heap(|module| {
            let extra = make_extra(label.package());
            {
                let mut eval = Evaluator::new(&module);
                if let Some(ref e) = extra {
                    eval.extra = Some(&**e);
                }
                eval.set_loader(&loader);
                eval.eval_module(ast, globals)?;
            }
            Ok(module.freeze_named(FrozenHeapName::User(Box::new(path.to_owned())))?)
        });

        self.set_complete(&file_status, module)
    }

    fn find_cycle_path(&self, start: &str, target: &str) -> Option<Vec<String>> {
        let loader = self.files.read().unwrap();
        let mut cur = target.to_owned();
        let mut cycle = vec![cur.clone()];
        while let Some(status_mutex) = loader.get(&cur) {
            let status = status_mutex.lock().unwrap();
            if let FileStatus::Loading {
                needs: Some(need), ..
            } = &*status
            {
                cycle.push(need.clone());
                cur = need.clone();
            } else {
                break;
            }
            if cur == start {
                return Some(cycle);
            }
        }
        None
    }
}

/// Helper loader to load preloaded dependencies during evaluator execution.
pub struct PreloadedLoader<'a> {
    /// A map of pre-loaded Starlark modules indexed by their module path.
    pub modules: &'a HashMap<&'a str, &'a FrozenModule>,
}

impl StarlarkFileLoader for PreloadedLoader<'_> {
    fn load(&self, path: &str) -> starlark::Result<FrozenModule> {
        match self.modules.get(path) {
            Some(m) => Ok((*m).clone()),
            None => unreachable!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use types::PathResolver;

    use super::*;

    fn get_testdata_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("testdata")
            .canonicalize()
            .unwrap()
    }

    #[test]
    fn test_simple_load() {
        let testdata = get_testdata_dir();
        let resolver = PathResolver::new(testdata, "../../../../../".into());
        let loader = FileLoader::new(resolver);

        let module_ref = loader
            .load(
                "//load:absolute.bzl",
                PackageRef::new("//"),
                &Globals::standard(),
                &|_pkg| None,
            )
            .unwrap();
        let module = module_ref.get_ref();
        assert_eq!(module.get("absolute").unwrap().unpack_i32(), Some(1));
    }

    #[test]
    fn test_dependency_load() {
        let testdata = get_testdata_dir();
        let resolver = PathResolver::new(testdata, "../../../../../".into());
        let loader = FileLoader::new(resolver);

        let module_ref = loader
            .load(
                "//load:root.bzl",
                PackageRef::new("//"),
                &Globals::standard(),
                &|_pkg| None,
            )
            .unwrap();
        let module = module_ref.get_ref();
        assert_eq!(module.get("root").unwrap().unpack_i32(), Some(2));
    }

    #[test]
    fn test_cycle_detection() {
        let testdata = get_testdata_dir();
        let resolver = PathResolver::new(testdata, "../../../../../".into());
        let loader = FileLoader::new(resolver);

        let res = loader.load(
            "//cycle:a.bzl",
            PackageRef::new("//"),
            &Globals::standard(),
            &|_pkg| None,
        );
        assert!(res.is_err());
        let err_msg = res.unwrap_err().to_string();
        assert!(err_msg.contains("cycle detected"));
    }
}
