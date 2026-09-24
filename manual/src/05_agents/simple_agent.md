# A Very Simple Agent

Time to build one. We'll keep it as small as we possibly can, because the *shape* of the loop is
the lesson - not the cleverness of the tools. The transport doesn't change at all: it's the same
streaming request, the same `tool_calls`, and the same `ToolCall` turns we've been feeding back
since the tool chapters. The only new ingredients are a *goal* and a *loop guard*.

> This is in the repo as `code/ex20_agent`.

Our agent answers a **single question** about a small set of notes. You run it with a question, it
works, and it prints an answer - there's no chat prompt, and no human in the middle:

```bash
cd code/ex20_agent
cargo run -- "what port does the deploy service listen on?"
```

The notes are baked straight into the binary:

```rust
const NOTES: &[(&str, &str)] = &[
    ("deploy-process.md", include_str!("../notes/deploy-process.md")),
    ("service.toml", include_str!("../notes/service.toml")),
    ("onboarding.md", include_str!("../notes/onboarding.md")),
    ("incident-log.md", include_str!("../notes/incident-log.md")),
    ("teams.md", include_str!("../notes/teams.md")),
];
```

That's a deliberate choice. There's no filesystem to sandbox, no paths to get wrong, and no
difference between running from `code/` and running from the example's own directory. The tools
just search and read that in-memory slice. Tool safety is a solved problem by now; this example is
about the loop.

We give the model two tools. `search_notes` looks for a word or phrase across every note and
returns the names of the notes that matched, plus the lines that matched. `read_note` returns the
whole body of one note:

```rust
fn search_notes(query: &str) -> String {
    let needle = query.to_lowercase();
    let mut results = Vec::new();
    for (name, body) in NOTES {
        let matched_lines: Vec<&str> = body
            .lines()
            .filter(|line| line.to_lowercase().contains(&needle))
            .collect();
        if !matched_lines.is_empty() {
            results.push(json!({ "note": name, "matches": matched_lines }));
        }
    }
    json!(results).to_string()
}
```

The corpus is built so that one note points at another. `deploy-process.md` says the service
listens on "the port declared in `service.toml`", and `service.toml` says `port = 8443`. Ask
*"what port does the deploy service listen on?"* and the agent has to search, read the first note,
follow the reference, and read the second. One prompt, several dependent steps, and the model
decides every one of them.

The system prompt does a surprising amount of work, and it's worth studying:

```rust
let mut session = ChatSession::new(
    api_key,
    "deepseek/deepseek-v4.1-flash",
    "You are a research agent. Answer the user's question using the notes \
     tools. Always search the notes before you answer, and follow any \
     references you find to other notes. Never guess. When you have the \
     answer, reply with only the answer.",
)?;
```

Small wording changes move the model from "chatty assistant" to "researcher". Telling it to
*always search* stops it answering from memory; telling it to *reply with only the answer* stops
it narrating its way to the result. This is a lesson in itself: an agent's behaviour lives as much
in its instructions as in its code.

The loop is the one from `ex08`, reframed. We cap it, and we stop the moment the model replies
without asking for a tool:

```rust
const MAX_ITERATIONS: usize = 8;

for iteration in 1..=MAX_ITERATIONS {
    println!("\n--- iteration {iteration} ---");
    // ... call the model, stream the reasoning and content ...
    // ... assemble any tool_calls ...

    // No tool calls means the model has answered: we're done.
    if tool_calls.is_empty() {
        return Ok(());
    }

    // Run the requested tools and append their results for the next trip around.
    for tool_call in &tool_calls {
        println!("🔧 Tool call: {}", tool_call.name);
        // ... match on the name, run the tool, push a ChatTurn::ToolCall ...
    }
}
```

Two small details are worth calling out. First, a wrong note name doesn't crash the run - it
returns a message the model can read and recover from, so a bad guess is just another observation.
Second, the iteration cap means a confused agent can't loop forever; if it runs out of road, we
say so and stop. The best agents are the ones you can switch off.

> Watch the trace scroll past. The interesting part isn't the answer - it's the moment the model
> reads the first note, notices it references a second one, and decides to go and look.

Next, we'll look at when you shouldn't reach for an agent at all - and why the last few seconds of
that trace could sometimes have been a single `match` statement.
