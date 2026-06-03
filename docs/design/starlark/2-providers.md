# Starlark in GN: Embedding the Interpreter

This document outlines the architecture for embedding the Starlark interpreter natively in GN using `starlark-rust`. 

To align with a Bazel-like design, the **majority of the provider management and rule evaluation logic is implemented in Rust**. C++ manages the initial parsing and the final build graph, while Rust maintains the provider registry, constructs the rule execution contexts (`ctx`), and runs the Starlark rule implementation functions.

---

## 1. Core Architecture

The Starlark runtime, custom rules, target attributes, and the provider dependency graph live inside the Rust static library.

```
+----------------------------------+          +-----------------------------------------+
|              GN C++              |          |             Rust Static Lib             |
|                                  |          |         (starlark_rust_bridge)          |
|                                  |          |                                         |
|  1. Parse BUILD.gn               |          |                                         |
|  2. OnTargetResolved(Target A):  |          |                                         |
|     Pass target object of A &    |          |                                         |
|     deps' targets ---------->    | --FFI--> |                                         |
|                                  |          | 3. Construct 'ctx' object from target   |
|                                  |          | 4. Execute _rule_impl(ctx)              |
|                                  |          | 6. Save returned Providers in Target    |
|     Use modified attributes      |          | 7. Return error                         |
|     (e.g., cflags)  <----------- | <--FFI-- |                                         |
+----------------------------------+          +-----------------------------------------+
```

---

## 2. Starlark API Design (Bazel-like)

Starlark rules execute implementation functions that receive a rule context (`ctx`) and return a list of provider instances.

### Example Starlark Rule & Extension:
```python

CustomInfo = provider(fields = {
    'foo': 'depset[str] representing all foos',
})

def _static_library_extension(ctx):
  cc_infos = [dep[CcInfo] for dep in ctx.attr.deps]
  public_cc_infos = [dep[CcInfo] for dep in ctx.attr.public_deps]

  # deps required to build a PCM
  interface_module_deps = depset(transitive = [dep.module_deps_no_self for dep in public_cc_infos])
  pcm = ctx.attr.declare_file(ctx.attr.name + ".pcm")
  # deps required to use the PCM
  modules = depset(direct = [pcm], transitive = interface_module_deps)
  # Deps required to build the .o files
  impl_module_deps = depset(direct = [pcm], transitive = [interface_module_deps] + [dep.modules for dep in cc_infos])

  custom_info = CustomInfo(foo = depset(
      direct = ['hello'],
      transitive = [dep[CustomInfo] for dep in ctx.attr.deps],
  ))

  # For rule extensions, we need to wrap the old CcInfo. This means getters and setters for it.
  # This shouldn't be required for rules, only for rule extensions.
  ctx.set_cc_info(
    modules = modules,
    cc_flags = ctx.target[CcInfo].cc_flags + custom_info.to_list()
  )

  return [
    custom_info
  ]

# Register implementation function to extend a built-in target type
extend_builtin_rule("static_library", _static_library_extension)
```

## 3. Data Modeling in Rust

The Rust library wraps Starlark objects and maintains target state.

```rust
// A generic provider, returned by the `provider` function.
struct GenericProviderType {
    keys: Set<str>,
}

// A provider, validated against GenericProviderType
type GenericProviderValue {
    value: StarlarkStruct,
}

// A builtin provider. Has starlark attributes that default to calling methods on target.
struct CcInfo {
    target: Target,
    modules: Option<Depset<File>>,
}

// ctx object in starlark
struct StarlarkRuleContext {
    target: &Target,
    attributes: StarlarkAttributes
}

// ctx.attr object
struct StarlarkAttributes {
    // Only supports builtin attributes for now. Eg. ctx.attr.sources -> target.sources
    target: &Target,
}

// Implementation of rust additions to the C++ target FFI struct.
impl Target {
    fn GetProvider(&self, key: StarlarkValue) -> StarlarkValue {
        match key {
            GenericProviderType => return target.custom_providers[key]
            CcInfo => return self.CcInfo(),
            ...
        }
    }
}
```

## Rust / C++ interop

We will use `rust-bindgen` to generate Rust bindings for the C++ `Target` object, allowing the Rust library to safely access target properties.

### Importing rules
To extend existing rules, we will overload the gn load function to support bzl files. We will also improve the load function to only import specific symbols. This improves clarity, as it's currently nontrivial to work out where a symbol came from in GN.
```
load("//path_to/rules.bzl", "static_library", "shared_library")
```

This will call rust's `RunBzl` function, which will execute the bzl file and return `StarlarkValue` objects.
* If it sees an object convertible to a C++ `Value` object (eg. int, bool, string, list, scope), it will do so.
* If it sees a starlark rule extension object, it will add it to the list of rule extensions in the `Settings` object.
  * This will be hardcoded to only be allowed in `BUILDCONFIG.gn`
* `RunBzl` will be cached to only run once per `bzl` file, *not* once per toolchain.
  * This is for performance reasons. Rules themselves are aware of the toolchain they are executing in.
* If it sees a starlark rule object, it will use a similar technique to `scope->AddTemplate()` to add the rule to the target's scope.
  * Note: Rules will not be part of the initial version of this feature. I mention them only so we can see how easy it is to add them later.

### Evaluating rules

* Rule extensions will simply allow additional metadata to be returned in the providers list. This will then be what is read by target objects once they are constructed.
* Rules will be a new type of action, because they create their own decisions about what actions to create.
* During GN's resolution phase (`OnTargetResolved`), C++ passes the target to rust. It simply calls rust's `Resolve(target: &Target)`.

### Using rule results
Substitutions such as `module_deps` will simply read from the CcInfo provider instead of querying the target directly.
