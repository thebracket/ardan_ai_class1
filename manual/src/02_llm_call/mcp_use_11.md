# A Free Remote Server to Test Against

> **Draft — planning notes.**

We want a no-auth, public Streamable HTTP endpoint for the live demo.

## Covered here

* Candidates:
  * **DeepWiki** — `https://mcp.deepwiki.com/mcp` (Streamable HTTP, no auth).
  * **GitMCP** — turn any GitHub repo into an endpoint, `https://gitmcp.io/OWNER/REPO`.
    Depending on the client it may speak SSE or Streamable HTTP; verify before class.
  * Cloudflare's public docs MCP server — check the current guide for its endpoint.
* The safe fallback: run the "everything" server locally in `streamableHttp` mode. Same
  transport, none of the trust issues, no dependency on someone else's uptime.
* Compare the tool list a remote server exposes with our local one.
* **Before the class:** re-check that every endpoint is still live and still Streamable
  HTTP. This ecosystem moves.
