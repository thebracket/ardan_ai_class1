# A Free Remote Server to Test Against

> **Draft — planning notes.**

You need a remote server to practise on, and you need it to be free, public, and not mind
strangers hammering it. Here are three that worked on the day this page was written, each
showing off a different part of the transport. None of them need an API key.

> The example is `code/ex19_mcp_remote`; swap the URL on the command line.

## DeepWiki: the General-Purpose One

```bash
cargo run -- https://mcp.deepwiki.com/mcp
```

DeepWiki indexes GitHub repositories and answers questions about them. It speaks 2025-11-25, so
it's the legacy path: probe, fall back, `initialize`, go.

Its three tools, listed by the example:

```text
ask_wiki_question      Ask any question about a GitHub repository's codebase and get an AI-powered answer
read_wiki_contents     View documentation about a GitHub repository.
read_wiki_structure    Get a list of documentation topics for a GitHub repository.
```

A real call, trimmed (the full output lists every page of the rust-sdk docs):

```bash
cargo run -- https://mcp.deepwiki.com/mcp read_wiki_structure \
  '{"repoName":"modelcontextprotocol/rust-sdk"}'
```

```json
{
  "content": [
    {
      "text": "Available pages for modelcontextprotocol/rust-sdk:\n\n- 1 Overview\n  - 1.1 Getting Started\n  ...",
      "type": "text"
    }
  ],
  "isError": false,
  "structuredContent": {
    "result": "Available pages for modelcontextprotocol/rust-sdk:\n..."
  }
}
```

Notice the shape: text plus `structuredContent`, exactly what chapter nine taught us to prefer.
This is the server to use for the demo everyone remembers, because the answers are useful:
point it at a repository the audience knows and ask what it contains.

## Cloudflare Docs: the Modern One

```bash
cargo run -- https://docs.mcp.cloudflare.com/mcp \
  search_cloudflare_documentation '{"query":"What is Workers KV?"}'
```

```text
Trying `server/discover` (2026-07-28)...
  Modern server: no handshake, no session.

Server:   docs-ai-search v0.4.13
Protocol: 2026-07-28
Capabilities: {"prompts":{"listChanged":true},"tools":{"listChanged":true}}

2 tools available:
  search_cloudflare_documentation    Search the Cloudflare documentation.
  migrate_pages_to_workers_guide     ALWAYS read this guide before migrating Pages projects to Workers.
```

This one is special: it's a live **2026-07-28** server, so it exercises the modern path in the
example - per-request `_meta`, the `MCP-Protocol-Version` / `Mcp-Method` / `Mcp-Name` headers,
no handshake, no session. Its discover response also carries `ttlMs: 0` and
`cacheScope: "private"`, which is nice to show when someone asks what those fields are for.

The tool result is a pile of documentation chunks, both as text and as structured search
results. That makes it a good contrast with DeepWiki: same protocol, completely different
content policy.

## GitMCP: the Session One

GitMCP turns any GitHub repository into an endpoint. Replace `docs` with `owner/repo`:

```bash
cargo run -- https://gitmcp.io/modelcontextprotocol/rust-sdk
```

```text
Protocol: 2025-03-26
Capabilities: {"tools":{"listChanged":true}}

5 tools available:
  match_common_libs_owner_repo_mapping         Match a library name to an owner/repo...
  fetch_generic_documentation                  Fetch documentation for any GitHub repository...
  search_generic_documentation                 Semantically search in documentation...
  search_generic_code                          Search for code in any GitHub repository...
  fetch_generic_url_content                    Generic tool to fetch content from any absolute URL...
```

It speaks the oldest Streamable HTTP revision and it uses protocol sessions: the initialize
response carries an `Mcp-Session-Id`, and every later request has to send it back. That makes
GitMCP the server to reach for when demonstrating the legacy session machinery - and the one
that will break when the world finally drops sessions, which is a lesson in itself.

## Poking at Them Without Code

The Inspector knows about remote servers too. From chapter five, remember that with `--cli` the
target comes before the `--` and Inspector flags come after:

```bash
npx @modelcontextprotocol/inspector --cli https://mcp.deepwiki.com/mcp \
  -- --transport http --method tools/list
```

The web and TUI versions take the same URL in their "Streamable HTTP" connection form. It's
worth doing once just to watch a remote tool call happen in a GUI, with the message log next to
it.

## Before Class

This is the chapter that rots fastest. Before teaching:

* Re-run every endpoint here. Public MCP servers appear, change transport, change auth, and
  quietly disappear. If DeepWiki is down, the Cloudflare server is a good fallback; if both are
  down, run the everything server locally in `streamableHttp` mode and point the example at
  `http://localhost:<port>/mcp`.
* Check the tool names and shapes, because a server can rename a tool between terms.
* Check the tool *behaviour*, too. A search tool that returns garbage is worse in a live demo
  than no server at all.

The wider catalogue lives at the [MCP Registry](https://registry.modelcontextprotocol.io/), but
remember what chapter four established: a registry entry is a URL, not a character reference.
Read the tool list before you point a model at it.

## References

* DeepWiki: <https://mcp.deepwiki.com/mcp>
* Cloudflare docs server: <https://docs.mcp.cloudflare.com/mcp>
* GitMCP: <https://gitmcp.io>
* MCP Registry: <https://registry.modelcontextprotocol.io/>
* Inspector: <https://github.com/modelcontextprotocol/inspector>
