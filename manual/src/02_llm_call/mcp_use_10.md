# Remote Servers over Streamable HTTP

> **Draft — planning notes.**

The same messages, now over HTTP.

## Covered here

* One endpoint, JSON-RPC POSTed to it. Replies are a JSON body or a request-scoped SSE
  stream.
* The stateless model (2026-07-28): no `Mcp-Session-Id`, no GET stream, `_meta` on every
  request. Note the backwards compatibility story for 2025-era servers.
* Why choose remote: a shared, org-hosted server with no local install.
* Why it's riskier: the server, its code, and its tool list belong to someone else — which
  sets up the security page.
