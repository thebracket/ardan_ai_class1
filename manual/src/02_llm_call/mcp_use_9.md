# The Round Trip

> **Draft — planning notes.**

Model asks for a tool → we call the server → we feed the result back.

## Covered here

* Building the `tool` message from an MCP tool result.
* Result shapes: text first, then note structured content, images, audio, and embedded
  resources.
* Errors: a tool-level error (the tool ran and failed) versus a protocol error (the server
  couldn't process the request at all), and how each should surface to the model.
* Several tool calls in one turn.
* Timeouts and cancellation — on stdio, the client sends `notifications/cancelled`.
