# Classify, Don't Generate

Look at the list of things we ask models to produce structured output for: route this ticket, pick
a category, decide yes or no, extract these fields, score this on a scale. Very little of that is
*generation*. It's **classification**, and we've been using a sledgehammer on a nut.

A classifier is a different kind of tool. You give it some messy text and a fixed set of
possibilities, and it gives you back one of them, with a score. The crucial property is that it
**cannot** return something outside your set. No invented categories, no prose to parse, no
trailing chatter. The output is typed because the tool only knows how to emit types.

Two good options:

* **Jev** (TypeSafe AI's "System One" decision model). It takes messy text in and gives typed
  decisions out - a choice from your enumeration, an independent yes/no, or a graded score - in a
  single round trip, with nothing to train and nothing to host. It's a hosted model, but it's
  served on OpenRouter as `typesafe/jev-1.13`, so you call it with the same key you already use
  for the chat models - no separate account. As a general-purpose classifier it slots neatly into
  an agent loop: use it to route, triage, or gate, and let the LLM do the open-ended reasoning
  around it. (It can also be wired into OpenCode and other harnesses as an MCP tool via
  `jev-mcp` / `jev-code`, though it isn't built in.)
* **A local BERT-family classifier.** If you have labelled data, a small fine-tuned model (BERT and
  its descendants, or a sentence-transformer on top) runs locally, costs nothing per call, works
  offline, and keeps your data in the building. The price is that you need the data and a training
  step, and it only knows the labels you taught it.

The trade-off, roughly:

| | LLM "just output JSON" | Classifier |
| --- | --- | --- |
| Output | Free text you constrain and hope | Typed, constrained by construction |
| Cost / latency | High | Low |
| Needs training data | No | Yes (for a local model) |
| Best at | Open-ended reasoning | Choosing from a known set |

This is the same lesson as "Different Model Types" and "Be Deterministic When Possible", just
sharper: match the tool to the job. Use the model for what only a model can do; use a classifier
for decisions.

> The next page, [A Jev Classifier in Rust](./jev_classifier.md), is the worked example
> (`code/ex21_jev`). It runs on your existing `OPENROUTER_KEY`, because Jev is served on
> OpenRouter; the `jev-mcp` / `jev-code` MCP servers can also wire it into OpenCode and other
> harnesses.
