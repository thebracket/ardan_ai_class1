# MCP Security and Provenance

> **Draft — planning notes.**

Everything we said about tools is true here — and MCP makes the mistake easier to make.

## Covered here

* Tool descriptions are untrusted input. **Tool poisoning**: instructions hidden in a tool's
  description or schema, aimed at the model rather than the user.
* **Rug pulls**: the tool list can change after you approved it, swapping a friendly tool
  for a hostile one.
* Remote is doubly scary: you don't control the server, its code, or its tool list, and it
  can exfiltrate anything you put into context.
* Confused-deputy problems and over-broad credentials; scoping OAuth tokens.
* Provenance: keep track of which text and tools came from where, and never let untrusted
  tool output become instructions.
* Governance: allow-list servers, pin versions, audit calls, read before you share.
* Callbacks to the prompt-injection and provenance pages, and to the tool-safety axes.
