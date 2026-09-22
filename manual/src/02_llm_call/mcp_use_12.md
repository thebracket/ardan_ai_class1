# Auth, Sessions and Spec Churn

> **Draft — planning notes.**

A remote server is somebody else's machine, and that changes the trust story twice over: now
there's a network in the middle, and now the server probably wants to know who you are. This
chapter is the tour of what that means, plus a short history of the protocol so that you can
read an older tutorial - or this book's own earlier drafts - without being misled.

## Auth on the Open Internet

Every server in the last chapter is anonymous. That's a privilege of being a *test* server:
DeepWiki doesn't care who asks, Cloudflare's docs are public, and GitMCP fetches public repos.
Real remote MCP servers - your company's issue tracker, somebody's SaaS product - will answer
`401 Unauthorized` and expect credentials.

MCP leans on OAuth for this. The shape, roughly:

1. The client gets a `401` (or the server advertises its protected-resource metadata).
2. The client discovers the authorisation server and what it supports.
3. The user goes through an authorisation flow in a browser (the SDKs run the loopback dance,
   the same idea as "sign in with..." flows).
4. The client stores the token and presents it as `Authorization: Bearer ...` on later requests,
   refreshing it as needed.

The spec's 2026 revision hardened this area: resource indicators, protected-resource metadata,
and alignment with ordinary OAuth and OpenID Connect deployments. The details are worth
reading, but the practical advice is shorter: **for anything real, use an SDK.** Hand-rolling
OAuth is a good way to ship a subtle security bug. Our workshop client skips it entirely, and
that's a deliberate scope decision, not an oversight.

Two related cautions:

* **Scope tokens narrowly.** A token that can read every repository because it was easier is
  the confused-deputy problem wearing a badge. Give the server the least it needs, and expect
  to revoke.
* **Local stdio servers are different.** They don't speak HTTP auth; the subprocess boundary is
  the trust boundary. Credentials usually arrive as environment variables, configured by the
  client, and the spec says stdio servers should not use the HTTP authorisation framework.
  That's chapter one's "local servers run on your hardware" advantage, in practice.

## Sessions: a Short Life

Sessions are a 2025 idea that 2026 deleted, and watching that happen in one book is a nice
reminder of how young this ecosystem is.

Protocol versions `2025-03-26` through `2025-11-25` let a server hand out a session id on the
initialize response (`Mcp-Session-Id`), which every later request carried back. There was also
a standalone GET stream for server-initiated messages, and a DELETE to end the session. Our
GitMCP run in the last chapter is a live example: without that header, its `tools/list` answers
`{"code":-32000,"message":"Bad Request: Mcp-Session-Id header is required"}`.

`2026-07-28` removed all of it. No session header, no GET stream, no sticky routing. Every
request carries its version, identity and capabilities in `_meta`, so it can land on any server
instance behind any load balancer. That's the change that made remote MCP *boring* - in the
best way - and it's why our modern path needs no state at all.

## The Revision Timeline

Worth keeping straight, because tutorials mix them freely:

* **2024-11-05** - stdio plus the old HTTP+SSE transport (two endpoints); `initialize` handshake.
* **2025-03-26** - Streamable HTTP replaces SSE; sessions appear.
* **2025-06-18** - auth, elicitation, and structured output updates.
* **2025-11-25** - further revisions; still session-based.
* **2026-07-28** - stateless core; sessions and `initialize` removed; `server/discover` added;
  headers mirror request fields; Multi Round-Trip Requests replace server-initiated requests.

The protocol has also adopted a formal lifecycle with a minimum twelve-month deprecation
window, so anything deprecated now keeps working for a while - but "keeps working" and "is
worth teaching" are different things.

## Practical Advice

* **Let an SDK negotiate versions.** Our hand-rolled dance is excellent for understanding and
  terrible for production; the official SDKs know which revisions exist and how to fall forward
  and back.
* **Old and new coexist.** Versions are date-stamped and travel per request (modern) or per
  session (legacy). One client can talk to a 2025 server and a 2026 server in the same process,
  which is what `ex19` demonstrates.
* **Test your servers before class, and pin the ones you depend on.** The public servers move;
  the spec moves; your slides don't.
* **Know what we skipped.** `ex19_mcp_remote` implements discovery, the legacy handshake,
  sessions, both response shapes and the modern headers. It does not implement OAuth, tool
  argument header mirroring (`x-mcp-header`), or Base64 `Mcp-Name` encoding. Those are real
  compliance gaps; they just aren't this workshop's teaching gaps.

## References

* Authorization: <https://modelcontextprotocol.io/specification/2026-07-28/basic/authorization>
* Versioning and compatibility: <https://modelcontextprotocol.io/specification/2026-07-28/basic/versioning>
* Key changes in 2026-07-28: <https://modelcontextprotocol.io/specification/2026-07-28/changelog>
* Feature lifecycle and deprecation: <https://modelcontextprotocol.io/community/feature-lifecycle>
