load(":relative.bzl", "relative")
load("//load:absolute.bzl", "absolute")
load("//load:relative.bzl", relative_as_absolute = "relative")

absolute_value = absolute
relative_value = relative
relative_as_absolute_value = relative_as_absolute
