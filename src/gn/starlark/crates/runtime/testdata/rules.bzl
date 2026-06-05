
MyProvider = provider(fields = ["val"])

def _my_rule_impl(ctx):
    return [MyProvider(val = ctx.attr.val)]

my_rule = rule(
    implementation = _my_rule_impl,
    attrs = {
        "val": attr.string(default = "hello"),
    },
)
