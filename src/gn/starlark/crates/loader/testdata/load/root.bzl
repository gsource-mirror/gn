load(":relative.bzl", "relative")
load("//load:absolute.bzl", "absolute")

root = relative + absolute
