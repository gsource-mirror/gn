ExtraFilesInfo = provider(fields = {"files": "depset[File]"})

def _map_config(c):
  return list(c)

def _static_library_impl(ctx):
  extra_files = depset(
    direct = ctx.files.public,
    transitive = [dep[ExtraFilesInfo].files for dep in ctx.attr.extra_files] + [depset(ctx.gn.get_output_files())])
  args = ctx.actions.args()
  args.add_all(
    extra_files,
    before_each="--extra-file",
  )
  return [
    GnSubstitutionInfo(substitutions = struct(
      extra_args = [args],
    )),
    ExtraFilesInfo(files = extra_files),
  ]

static_library = rule_extension(
  implementation = _static_library_impl,
  attrs = {
    "static_library_attr": attr.string(mandatory = True),
    "extra_files": attr.label_list(),
  },
)

def _executable_impl(ctx):
  return [GnSubstitutionInfo(substitutions = struct(
    executable_sub = ctx.attr.executable_attr,
  ))]

executable = rule_extension(
  implementation = _executable_impl,
  attrs = {
    "executable_attr": attr.string(mandatory = True),
  },
)

def _extra_files_impl(ctx):
  return [
    ExtraFilesInfo(
      files = depset(ctx.files.extra_files)
    ),
  ]

extra_files = rule(
  implementation = _extra_files_impl,
  attrs = {
    "extra_files": attr.label_list(allow_files = True),
  },
)