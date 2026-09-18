# How a Server Describes Itself

> **Draft — planning notes.**

Before the model can use a server, the client has to ask what is there.

## Covered here

* What a server advertises: supported protocol versions, identity, and capabilities.
* Discovery: the `server/discover` RPC (2026-07-28) versus the older `initialize`
  handshake. Clients negotiate, often automatically.
* The three primitives — **tools**, **resources**, and **prompts** — and why we mostly
  care about tools in this workshop.
* `tools/list`: a name, a description, and a JSON Schema for the input (and, newly, the
  output). Show a real response.
* Compare it, field by field, to our own `ToolDefinition { name, description, parameters,
  required }` from `ex09`.
* The cost of discovery: every tool description is context. More servers means a bigger
  prompt before the user has said anything.
