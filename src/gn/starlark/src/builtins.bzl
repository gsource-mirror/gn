DefaultInfo = provider(fields = [
  "executable",
  "files",
])

empty_default_info = DefaultInfo(executable = None, files = depset([]))

GnSubstitutionInfo = provider(fields = [
  "substitutions",
])
