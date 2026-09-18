# Provenance

The previous page ended with a promise, and this section has to deliver on it: if a scolding system prompt isn't a boundary, what is?

The idea is **provenance**: every piece of text in the context window came from *somewhere*, and where it came from should decide how much authority it gets. There are two levels to that:

* **Explicit provenance** — we *tell the model* where each chunk of text came from, and ask it not to treat tool output as instructions.
* **Enforced provenance** — our *code* tracks where text came from, and refuses privileged actions that were driven by untrusted data.

The first is a speed bump. The second is the control. We're going to build both, and watch the first one fail.

> This is in the repo as `code/ex15_provenance`.

## The Setup

`ex15_provenance` is a copy of `ex14_injection` with two additions.

The first is a `send_email` tool, marked `privileged`. It never actually sends anything — it prints a line instead — but it stands in for any tool with real-world consequences. The distinction lives on the tool definition:

```rust
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Vec<(String, String)>,
    pub required: Vec<String>,
    /// `true` for tools with side effects (sending, writing, deleting).
    pub privileged: bool,
}
```

The second is a `DEFENCE_LEVEL` constant, so we can walk the escalation live:

```rust
///   0 - none. Exactly like `ex14_injection`: raw tool data, "You are a helpful assistant".
///   1 - ask the model, in the system prompt, to treat tool output as data.
///   2 - level 1, plus wrap every tool result in an explicit provenance envelope.
///   3 - level 2, plus *enforce* provenance: a privileged tool is refused when the
///       model asks for it after reading untrusted tool output.
const DEFENCE_LEVEL: u8 = 2;
```

The `ATTACK` constant picks what the compromised weather tool actually says. `Pirate` is the joke from the previous page; `Email` is a benign-sounding privileged action; `EmailAdapted` is a payload reworded to defeat the provenance envelope. The example ships with all three.

## In This Section

* [Explicit Provenance](./provenance_explicit.md) — try to fix it by asking nicely, and by labelling the data.
* [Enforced Provenance](./provenance_enforced.md) — stop asking, and make the code refuse.
* [What Provenance Buys You](./provenance_takeaways.md) — what holds, and what's still missing.
