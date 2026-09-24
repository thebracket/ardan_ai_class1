# Defining a Workflow

There's no single "workflow framework" you have to adopt. A workflow is just a plan you wrote down,
and the simplest honest way to do that in Rust is a sequence of steps and a `match`:

```rust
enum Workflow {
    Refund,
    BillingEnquiry,
    BugReport,
}

fn run(workflow: Workflow, request: &str) -> anyhow::Result<String> {
    match workflow {
        Workflow::Refund => refund_steps(request),
        Workflow::BillingEnquiry => billing_steps(request),
        Workflow::BugReport => bug_steps(request),
    }
}
```

Each `*_steps` function is an ordinary function that might call a model, might call an API, and
might hand its result to the next step. Nothing about it is magic, and that's the point: you can
read it, test it, and step through it.

The other common shape is **data-driven**: describe the steps and their transitions in JSON or
YAML, and write a small runner that walks the graph. That trades type safety for reach - a
non-programmer can edit the workflow, and you can review and version it like any other config. It
also means you now maintain an interpreter, so reach for it when the workflows outnumber the
developers, not before.

Whatever the shape, the rule is the same: **every branch is written down**. A model may be *inside*
a step, but it doesn't get to decide which step comes next. When you find yourself wanting a step
to "do whatever seems right here", that's the moment to reach back for an agent - on purpose, as a
single step, rather than as the whole program.
