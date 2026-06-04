use std::collections::hash_map::{Entry, HashMap};
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::path::PathBuf;
use std::fs;
use anyhow::{Result, anyhow};
use starlark::environment::{Globals, Module, FrozenModule};
use starlark::eval::Evaluator;
use starlark::eval::ReturnFileLoader;
use starlark::syntax::{AstModule, Dialect};
use starlark::values::FrozenHeapName;

// starlark::Error can't be cloned.
type StarlarkResult<T> = Result<T, Arc<starlark::Error>>;

pub struct FileLoader {
    root_source_dir: PathBuf,
    files: RwLock<HashMap<String, Arc<Mutex<FileStatus>>>>,
}

enum FileStatus {
    Loading {
        // A CondVar to wait on for the filestatus to resolve to Loaded.
        // Using Arc allows us to clone it and release the mutex lock before waiting.
        wait: Arc<Condvar>,
        // What needs to finish evaluating before this can be evaluated.
        // This is used purely for cycle detection.
        needs: Option<String>,
    },
    Loaded(StarlarkResult<FrozenModule>),
}

impl Default for FileStatus {
    fn default() -> Self {
        FileStatus::Loading {
            wait: Arc::new(Condvar::new()),
            needs: None,
        }
    }
}

fn wait_for_load(file_status: &Arc<Mutex<FileStatus>>) -> StarlarkResult<FrozenModule> {
    let mut status = file_status.lock().unwrap();
    while let FileStatus::Loading { wait, .. } = &*status {
        let wait = wait.clone();
        status = wait.wait(status).unwrap();
    }
    match &*status {
        FileStatus::Loading { .. } => unreachable!(),
        FileStatus::Loaded(m) => m.clone(),
    }
}

fn set_needs(file_status: &Arc<Mutex<FileStatus>>, value: &str) {
    let mut status = file_status.lock().unwrap();
    if let FileStatus::Loading { ref mut needs, .. } = &mut *status {
        *needs = Some(value.to_owned());
    }
}

fn set_complete(file_status: &Arc<Mutex<FileStatus>>, result: StarlarkResult<FrozenModule>) {
    let mut status = file_status.lock().unwrap();
    if let FileStatus::Loading { wait, .. } = &*status {
        wait.notify_all();
    }
    *status = FileStatus::Loaded(result);
}

impl FileLoader {
    pub fn new(root_source_dir: PathBuf) -> Self {
        Self { root_source_dir, files: Default::default() }
    }

    pub fn load(&self, path: &str, relative_to: &str) -> StarlarkResult<FrozenModule> {
        let (package, name) = Self::resolve_path(path, relative_to)?;
        let file_status = {
            let mut loader = self.files.write().unwrap();
            match loader.entry(path.to_owned()) {
                Entry::Occupied(entry) => {
                    let file_status = entry.get().clone();
                    drop(loader);
                    return wait_for_load(&file_status);
                }
                // We're about to start evaluating it.
                Entry::Vacant(entry) => entry.insert(Default::default()).clone()
            }
        };

        // Read and parse the file to get its dependencies.
        let content = fs::read_to_string(self.disk_path(&package, &name))
            .map_err(|_| Arc::new(starlark::Error::new_other(anyhow!("Failed to read //{package}:{name}"))))?;
        let ast = AstModule::parse(&path, content, &Dialect::Standard)
            .map_err(Arc::new)?;

        let mut deps: Vec<(String, FrozenModule)> = Default::default();
        if !ast.loads().is_empty() {
            for load in ast.loads() {
                set_needs(&file_status, &load.module_id);
                if let Some(cycle) = self.find_cycle_path(&path, &load.module_id) {
                    let err = starlark::Error::new_other(anyhow!("cycle detected: {:?}", cycle));
                    let shared_err = Arc::new(err);
                    set_complete(&file_status, Err(shared_err.clone()));
                    return Err(shared_err);
                }

                deps.push((load.module_id.to_owned(), self.load(&load.module_id, &package)?));
            }
        }

        let globals = Globals::standard();
        let deps_map: HashMap<&str, &FrozenModule> = deps.iter().map(|(k, v)| (k.as_str(), v)).collect();
        let loader = ReturnFileLoader{modules: &deps_map};
        let module = Module::with_temp_heap(|module| {
            {
                let mut eval = Evaluator::new(&module);
                eval.set_loader(&loader);
                eval.eval_module(ast, &globals)?;
            }
            // After creating a module we freeze it, preventing further mutation.
            // It can now be used as the input for other Starlark modules.
            // Each frozen module is given a name to identify its heap.
            module.freeze_named(FrozenHeapName::User(Box::new(path.to_owned())))
                .map_err(|e| starlark::Error::new_other(e))
        }).map_err(Arc::new);

        set_complete(&file_status, module.clone());
        module
    }

    fn find_cycle_path(&self, start: &str, target: &str) -> Option<Vec<String>> {
        let loader = self.files.read().unwrap();
        let mut cur = target.to_owned();
        let mut cycle = vec![cur.clone()];
        while let Some(status_mutex) = loader.get(&cur) {
            let status = status_mutex.lock().unwrap();
            if let FileStatus::Loading { needs: Some(need), .. } = &*status {
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

    fn resolve_path<'a>(path: &'a str, relative_to: &'a str) -> StarlarkResult<(&'a str, &'a str)> {
        if path.starts_with(':') {
            return Ok((relative_to, &path[1..]));
        } else if let Some(path) = path.strip_prefix("//") {
            if let Some((dir, name)) = path.split_once(':') {
                return Ok((dir, name));
            }
        }
        Err(starlark::Error::new_other(anyhow!("Invalid load target: must be of the form //package:file (got {path}")).into())
    }
    
    fn disk_path(&self, dir: &str, name: &str) -> PathBuf {
        if dir.is_empty() {
            self.root_source_dir.join(name)
        } else {
            self.root_source_dir.join(dir).join(name)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub fn run_starlark(path: &str) -> StarlarkResult<FrozenModule> {
        let loader = FileLoader::new(PathBuf::from("testdata"));
        // always load from the root dir.
        loader.load(&path, "")
    }

    #[test]
    fn test_run_starlark_simple() {
        let module = run_starlark("//:simple.bzl").unwrap();
        assert_eq!(module.get("n").unwrap().unpack_i32(), Some(42));
    }

    #[test]
    fn test_load_dependencies() {
        let module = run_starlark("//load:root.bzl").unwrap();
        assert_eq!(module.get("root").unwrap().unpack_i32(), Some(2));
    }

    #[test]
    fn test_cycle_detection_single_thread() {
        let res = run_starlark("//cycle:a.bzl");
        assert!(res.is_err());
        let err_msg = res.unwrap_err().to_string();
        assert!(err_msg.contains("cycle detected"), "expected cycle error, got: {}", err_msg);
    }
}