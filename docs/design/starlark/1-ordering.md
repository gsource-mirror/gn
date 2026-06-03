# Starlark in GN: Target Evaluation Ordering

This document describes how we ensure correct evaluation ordering in GN to support **Providers** (structured data packages returned by dependency targets and consumed by their dependents).

---

In the previous iteration of providers design, we proposed a deferred-evaluation scheduler to run target blocks only after their dependencies had evaluated. This was required because in order to evaluate the block itself, you needed to read the providers from the dependencies' blocks.

However, with a starlark-based approach, this is not required. The parameters to a rule are known at parse time, but evaluating the provider graph itself is deferred to target resolution time. This is how build systems such as bazel operate.

In particular, we can utilize the approach GN currently takes, and put all provider evaluation into the `OnTargetResolved` function. This function is to GN what rules are to bazel. It ensures that it is called after all of a target's dependencies are evaluated, but before the target itself is considered resolved. Thus, we get ordering support for free.