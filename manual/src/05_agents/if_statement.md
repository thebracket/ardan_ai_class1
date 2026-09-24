# That Could Have Been An If Statement

The tell-tale sign of an over-built agent is when you read the trace and think: *that could have
been an if statement.*

Take our notes agent. Ask it a fixed, known question - "what port does the deploy service use?" -
and watch it search, read a note, follow a reference, and read a second note. Very impressive.
But if that's *the* question, and there are only a handful of notes, then the whole thing is a
dictionary lookup:

```rust
match question {
    "what port does the deploy service use?" => "8443",
    _ => "I don't know",
}
```

No model, no tokens, no latency, and it's right every single time. The agent isn't *wrong* - it
works, and it handles paraphrasing the lookup wouldn't - but you're paying an enormous amount for
flexibility you may not need. This is the trap. Building an agent is fun; writing the `if` is
boring; and boring is frequently correct.

It gets worse when people deliberately route through a model for something the code already knows.
An agentic "triage" step that classifies a ticket into one of four fixed categories is often just
a `match` on a keyword, or at most a small model on a short prompt - not a reasoning model
planning its way through the problem.

The healthy version of this is to *use the agent for the part that's genuinely open-ended* and
hand the deterministic parts back to code. The agent decides which note is relevant; your program
does the lookup, the arithmetic, and the sending. Don't ask a model to do a thing a `match` does
better.

> The lesson isn't "never use an agent." It's "notice when you've built a workflow and called it
> an agent" - and be honest with yourself about whether the model is earning its keep.
