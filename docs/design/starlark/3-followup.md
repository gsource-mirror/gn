# Starlark in GN: Future Horizons & Follow-Up Ideas

This document outlines high-value features and extensions that can build upon the Starlark rule/provider engine in GN.

---

## 1. Custom Rules Definition (`rule()`)

While the initial iteration focuses on *extending* existing built-in rules (via `extend_builtin_rule`), the logical next step is to support defining **completely new rules** from Starlark. This would allow us to:
* Support new languages
* Easily add new features to existing languages (eg. modules work in GN)

```python
def _my_code_gen_impl(ctx):
  output_file = ctx.actions.declare_file(ctx.attr.name + "_gen.cc")
  
  ctx.actions.run(
      outputs = [output_file],
      inputs = ctx.files.srcs,
      executable = ctx.executable.generator_tool,
      arguments = ["--out", output_file.path] + [f.path for f in ctx.files.srcs],
  )
  
  return [DefaultInfo(files = [output_file])]

my_custom_code_gen = rule(
    implementation = _my_code_gen_impl,
    attrs = {
        "srcs": attr.label_list(allow_files = True),
        "generator_tool": attr.label(executable = True),
    },
)
```

## 2. File objects in bzl files
`rebase_path` is commonly used in gn to add arguments to command-lines. For example:
```
template("foo_rule") {
action("foo") {
  inputs = [invoker.file]
  args = [
    "--foo", rebase_path(invoker.file, root_build_dir),
  ]
}
}
```

`rebase_path` makes various assumptions about paths, and is generally an unsafe way of doing things, and could instead be written as:
```python
def _foo_rule_impl(ctx):
  ctx.actions.run(
    inputs = [ctx.attr.file],
    args = ["--foo", ctx.attr.file]
  )

foo_rule = rule(
  implementation = _foo_rule_impl,
  attrs = {
    "file": attr.label(mandatory = True, allow_files = True),
  }
)
```

## 3. Toolchain Configuration via providers
If we define a `ToolchainInfo` provider, then this would be relatively trivial.

## 4. Siso config in GN
We could create a SisoConfig provider. This could contain metadata such as remotability of actions.

## 5. Replace BUILD.gn files with starlark BUILD files
It would be relatively trivial at this point to replace some BUILD.gn files with files written in starlark.
This would allow us to add constraints to the language. In particular, we could significantly improve parsing performance by only evaluating a BUILD file once, rather than once per toolchain.

We could achieve this via a technique similar to bazel, where we, instead of writing:
```
if (is_android) {
  deps = ["foo"]
  deps += [android_dep]
}
```

We could write:
```
deps = ["foo"] + select({is_android: [android_dep]})
```

With select evaluating to, similar to bazel, an object that can be concatenated to lists, and evaluated lazily based on the rule context.