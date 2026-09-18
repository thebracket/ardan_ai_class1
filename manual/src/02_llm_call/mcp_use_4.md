# Fitting MCP Into the Model's Context

> **Draft — planning notes.**

The model knows nothing about MCP. Translation is our job.

## Covered here

* `tools/list` → the `tools` array we already send to OpenRouter. Same shape, different source.
* Namespacing: two servers can both expose `search`, so prefix names (`github__search`).
* Schema translation: MCP sends JSON Schema; our current `ToolDefinition` is a simplified
  `(name, type)` list. Do we grow it, or keep it deliberately simple?
* When discovery happens: once at startup, optionally refreshed when a server announces
  that its tool list changed.
* Keeping the model-facing shape identical to `ex09`/`ex10`, so the rest of the chat loop
  doesn't have to change.
