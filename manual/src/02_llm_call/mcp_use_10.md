# Remote Servers over Streamable HTTP

> **Draft — planning notes.**

Every server so far has been a subprocess: we spawn it, we own it, and the wire is a pipe. A
remote MCP server is just a URL - and the protocol is the same. What changes is the plumbing,
and most of it will look familiar: one endpoint, one POST per message, and the answer comes back
as either a JSON body or a request-scoped SSE stream. Those `data:` lines we parsed by hand back
in `ex04_streaming`? Same thing.

> This is in the repo as `code/ex19_mcp_remote`.

## The Rules

Five of them, and the rest is detail:

* **One endpoint.** The server exposes a single URL that accepts POST. For DeepWiki it's
  `https://mcp.deepwiki.com/mcp`.
* **One POST per message.** Requests and notifications each travel as their own HTTP request.
  The body is a single JSON-RPC message; it must not be a batch, and it must not be a response.
* **Ask for both answer types.** Every POST carries
  `Accept: application/json, text/event-stream`. The server chooses per request: a single JSON
  object, or an SSE stream carrying any request-related notifications first and the response
  last, then closing.
* **On modern servers, the interesting body fields are mirrored into headers.** `Mcp-Method`
  always, `Mcp-Name` for named requests like `tools/call`, and `MCP-Protocol-Version` on every
  POST to a modern server. That lets gateways route and meter without parsing JSON.
* **Legacy servers may hand out a session.** `Mcp-Session-Id` arrives on the initialize
  response, and every later request has to carry it back. 2026-07-28 removed sessions entirely.

The fallback logic from chapter three still applies, with one HTTP twist: a legacy server may
answer the modern probe with a `400`, `404` or `405` and no JSON-RPC body at all. An empty or
unrecognised body means legacy; a recognised modern error means modern. `discover` handles both.

## Running It

The example is a standalone client, like `ex17`, but pointed at a URL instead of a subprocess.
It defaults to DeepWiki, and can be pointed anywhere:

```bash
cd code/ex19_mcp_remote
cargo run -- [url] [tool] [json-arguments]
```

Here's the default run, trimmed in the middle:

```text
Connecting to https://mcp.deepwiki.com/mcp
Trying `server/discover` (2026-07-28)...
  Not supported (-32600); falling back to the old handshake.
Initializing the 2025 way (2025-11-25)...

Server:   DeepWiki v2.14.3
Protocol: 2025-11-25
Capabilities: {"experimental":{},"prompts":{"listChanged":true},"resources":{"listChanged":true,"subscribe":false},"tools":{"listChanged":true}}

3 tools available:
  ask_wiki_question                            Ask any question about a GitHub repository's codebase and get an AI-powered answer
  read_wiki_contents                           View documentation about a GitHub repository.
  read_wiki_structure                          Get a list of documentation topics for a GitHub repository.

Calling read_wiki_structure with {"repoName":"modelcontextprotocol/rust-sdk"}...
{
  "content": [
    {
      "text": "Available pages for modelcontextprotocol/rust-sdk:\n\n- 1 Overview\n  - 1.1 Getting Started\n  - 1.2 Workspace Structure\n- 2 Core Framework\n  ...",
      "type": "text"
    }
  ],
  "isError": false,
  "structuredContent": {
    "result": "Available pages for modelcontextprotocol/rust-sdk:\n..."
  }
}
```

Same three-step dance, same tool shapes, same `structuredContent` as the last chapter. The only
difference is that the JSON travelled over HTTPS instead of a pipe.

Point it somewhere else and nothing about the client changes:

```bash
cargo run -- https://docs.mcp.cloudflare.com/mcp \
  search_cloudflare_documentation '{"query":"What is Workers KV?"}'
```

Cloudflare's docs server is a real 2026-07-28 server, so there's no handshake at all:

```text
Trying `server/discover` (2026-07-28)...
  Modern server: no handshake, no session.
```

Its discover response is also where the caching hints from chapter three become real: the result
carries `"ttlMs": 0` and `"cacheScope": "private"`, telling us not to cache the list at all.

## Inside `post`

The transport lives in one method. Stripped to the interesting parts:

```rust
async fn post(&mut self, message: &Value) -> Result<(StatusCode, Vec<Value>)> {
    let method = message["method"].as_str().unwrap_or_default();

    let mut request = self
        .http
        .post(&self.url)
        .header("Content-Type", "application/json")
        // We support both shapes of answer, and modern servers may pick
        // either one per request.
        .header("Accept", "application/json, text/event-stream")
        .json(message);

    if self.modern {
        // Modern servers mirror the interesting body fields into headers,
        // so gateways can route without parsing the body.
        request = request
            .header("MCP-Protocol-Version", self.protocol_version.as_str())
            .header("Mcp-Method", method);
        if let Some(name) = message["params"]["name"].as_str() {
            request = request.header("Mcp-Name", name);
        }
    }

    if let Some(session) = &self.session_id {
        request = request.header("Mcp-Session-Id", session);
    }
    ...
```

Then the two shapes of answer:

```rust
    if status == StatusCode::OK && content_type.starts_with("text/event-stream") {
        // A request-scoped SSE stream: any notifications first, then the
        // response, then the stream closes.
        let body = response.text().await?;
        Ok((status, parse_sse(&body)?))
    } else {
        // A single JSON object - or, for a notification, a 202 with no body.
        let body = response.text().await?;
        if body.trim().is_empty() {
            return Ok((status, Vec::new()));
        }
        let message = serde_json::from_str::<Value>(&body)?;
        Ok((status, vec![message]))
    }
```

A request may also come back as an HTTP error status with a JSON-RPC error body - DeepWiki does
exactly that when we ask for a version it doesn't speak. `request` doesn't treat that as a
transport failure: the envelope comes back, the caller inspects the error, and `discover`
decides whether that means "negotiate" or "fall back".

## Parsing SSE by Hand

We wrote `parse_sse` because of a real bug we hit while building this example. The first version
used a popular Rust SSE helper. It works beautifully against OpenRouter. Against DeepWiki it
silently produced zero messages, and the client sat waiting for a response that had already
arrived.

DeepWiki frames its SSE with CRLF line endings. The helper assumed LF, so it never saw an empty
line, so it never emitted an event. The protocol says either ending is fine, so we do the
obvious thing and parse the `data:` lines ourselves - which is the same exercise we did in
`ex04_streaming`:

```rust
fn parse_sse(body: &str) -> Result<Vec<Value>> {
    let mut messages = Vec::new();
    let mut data = String::new();

    // `lines()` treats LF and CRLF the same, which is the point.
    for line in body.lines() {
        if line.is_empty() {
            // A blank line dispatches the event we've been accumulating.
            if !data.is_empty() {
                messages.push(serde_json::from_str(&data)?);
                data.clear();
            }
            continue;
        }
        if let Some(rest) = line.strip_prefix("data:") {
            if !data.is_empty() {
                data.push('\n');
            }
            data.push_str(rest.strip_prefix(' ').unwrap_or(rest));
        }
        // `event:`, `id:`, `retry:` and `:` comments don't matter to us.
    }

    // A final event with no trailing blank line is still an event.
    if !data.is_empty() {
        messages.push(serde_json::from_str(&data)?);
    }

    Ok(messages)
}
```

The lesson is worth the ink: helpers are great until they meet a server. The protocol is small,
the bytes are right there, and a stuck client is usually a framing problem - not a mystery.

## Sessions, and What Modern Replaced Them With

Run the example against GitMCP, which is a legacy server that really uses sessions:

```bash
cargo run -- https://gitmcp.io/docs
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

GitMCP returns an `Mcp-Session-Id` header on the initialize response, and our client stores it
and sends it on every later request. If you forget: send a `tools/list` without it and you get
`{"code":-32000,"message":"Bad Request: Mcp-Session-Id header is required"}`. We found that out
by accident, which is the best way to learn anything in this book.

2026-07-28 deleted protocol-level sessions. The per-request headers above are what replaced
them: no session to lose, no sticky routing, and any request can land on any server instance
behind a plain load balancer.

## What We Left Out

Two things, both deliberate:

* **Auth.** All three servers in this chapter are open - no key, no login. Real remote servers
  will answer `401` and expect an OAuth dance. That's the next-but-one chapter.
* **Header encoding and `x-mcp-header`.** The spec has Base64 escaping rules for `Mcp-Name`
  when a tool name isn't header-safe, and requires clients to mirror certain tool arguments
  into `Mcp-Param-*` headers. Our tool names are ASCII and our demo servers don't use the
  feature, so we skipped it. A production client can't.

## References

* Streamable HTTP: <https://modelcontextprotocol.io/specification/2026-07-28/basic/transports/streamable-http>
* Backward compatibility: <https://modelcontextprotocol.io/specification/2026-07-28/basic/transports/streamable-http#backward-compatibility>
* Caching hints (`ttlMs`, `cacheScope`): <https://modelcontextprotocol.io/specification/2026-07-28/server/utilities/caching>
