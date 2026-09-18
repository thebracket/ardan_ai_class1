# stdio and Streamable HTTP

> **Draft — planning notes.**

The spec defines two standard transports. The messages are the same; the plumbing differs.

## Covered here

* **stdio** — the client launches the server as a subprocess and speaks newline-delimited
  JSON-RPC over its stdin/stdout. Logs go to stderr, and *nothing else* may go to stdout.
  The server lives on your machine (or inside your container) and dies with the client.
* **Streamable HTTP** — a single HTTP endpoint. The client POSTs each message; replies
  come back as a JSON body or a request-scoped SSE stream. The server is a service: it can
  live on localhost or on the open internet, and it outlives the client.
* Where each one *resides*, and what that means for trust and lifecycle.
* A comparison table: launch vs connect, `command`+`args` vs `url`, auth, lifetime.
* History box: the original 2024-11-05 HTTP+SSE transport (two endpoints) is deprecated,
  replaced by Streamable HTTP in 2025-03-26. Don't teach the old one as current.
* Note the 2026-07-28 revision: protocol-level sessions and the `initialize` handshake are
  gone from Streamable HTTP, and requests carry version/capabilities in `_meta`.
