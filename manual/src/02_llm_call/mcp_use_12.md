# Auth, Sessions and Spec Churn

> **Draft — planning notes.**

A short history, so students can read older tutorials — and this book's own earlier drafts —
without being misled.

## Covered here

* Remote servers on the open internet need authorisation; the spec leans on OAuth (bearer
  tokens, resource indicators, protected-resource metadata).
* Local stdio servers usually need no auth: the subprocess boundary *is* the trust boundary.
* Revision timeline:
  * **2024-11-05** — stdio + HTTP+SSE; `initialize` handshake.
  * **2025-03-26** — Streamable HTTP replaces SSE.
  * **2025-06-18** — auth, elicitation, and structured output updates.
  * **2025-11-25** — further revisions.
  * **2026-07-28** — stateless; sessions and `initialize` removed; `server/discover` added.
* Practical advice: let an SDK negotiate versions; don't hand-roll it.
* Versioning is date-stamped and carried per request, so old and new servers can coexist.
