def hello_shared_macro(name, **kwargs):
  shared_library(
    name = name,
    defines = ["HELLO_SHARED_IMPLEMENTATION"],
    **kwargs,
  )
