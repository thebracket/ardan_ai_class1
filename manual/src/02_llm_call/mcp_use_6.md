# Talking to It by Hand

> **Draft — planning notes.**

Workshop ethos: see the wire. Before writing any client, drive the server with raw JSON.

## Covered here

* Start the stdio server and paste a `tools/list` request into its stdin; watch the reply
  arrive on stdout.
* Show the framing: one JSON-RPC message per line, no embedded newlines.
* Show that stderr is for logs, and why a stray `println!` to stdout breaks everything.
* What the request envelope looks like on a 2026-07-28 server (the `_meta` fields), and what
  an older server expects instead (`initialize` first).
* This is the MCP equivalent of `curl`-ing the chat-completions API.
