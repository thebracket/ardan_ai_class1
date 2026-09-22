# Talking to It by Hand

> **Draft — planning notes.**

In chapter three we wrote an MCP client, and it worked on the first try - which is exactly
when you should get suspicious. This chapter is the MCP equivalent of `curl`: no Rust, no SDK,
just a terminal and some JSON. It is how you find out whether a server is alive, what it
actually said, and whether your client is telling you the truth.

## Start the Server, Say Something

```bash
npx -y @modelcontextprotocol/server-everything
```

It sits there, waiting on stdin. Type a request, press Enter, and the reply comes back on
stdout. Let's start with the one we care about:

```json
{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}
```

Back comes one very long line - the whole tool list. Trimmed, it looks like this:

```text
{"jsonrpc":"2.0","id":1,"result":{"tools":[{"name":"echo","title":"Echo Tool","description":"Echoes back the input string","inputSchema":{"$schema":"http://json-schema.org/draft-07/schema#","type":"object","properties":{"message":{"type":"string","description":"Message to echo"}},"required":["message"]}, ... }]}}
```

Two things to notice, both worth a raised eyebrow:

* We never said hello. The protocol says `initialize` comes first; this server answered anyway.
  That's the everything server being friendly. Don't build a client that relies on it - a
  stricter server will tell you where to go, and quietly correct servers are the hardest ones
  to debug.
* The reply is *one line*. No pretty-printing, no blank lines. That's the framing rule, and
  breaking it - a stray newline inside the JSON - is the classic way to break every client at
  once.

If you want to stop the server, Ctrl-C. If you want the log lines to disappear, `2>/dev/null`
is your friend: the protocol lives on stdout, and the server's chatter lives on stderr. When
you see `Starting default (STDIO) server...` appear in your terminal, that's stderr doing its
job - it isn't part of the conversation.

## The Handshake, in a Pipeline

Typing JSON by hand is a great party trick and a terrible workflow, so let's script it with
`printf`. This is the same sequence our Rust client sends: hello, ready, list the tools.

```bash
{
  printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"by-hand","version":"1.0.0"}}}'
  printf '%s\n' '{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}'
  printf '%s\n' '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}'
} | npx -y @modelcontextprotocol/server-everything | jq -r 'select(.id == 2) | .result.tools[].name'
```

```text
echo
get-annotated-message
get-env
get-resource-links
get-resource-reference
get-structured-content
get-sum
get-tiny-image
gzip-file-as-resource
toggle-simulated-logging
toggle-subscriber-updates
trigger-long-running-operation
simulate-research-query
```

(`jq` is optional, but it turns a wall of JSON into a question you asked on purpose. Any JSON
tool will do; the server doesn't care.)

The three messages are worth reading in order. `initialize` announces our version and
capabilities; the server replies with its own. `notifications/initialized` has no `id` and
expects no reply - it's the client saying "ready". Then `tools/list`, which is the request we
actually care about. That's the whole handshake. It is not complicated; it has just been
complicated *at* us by years of spec churn.

## Asking About Versions

Before any of that, a 2026-era client would try the modern discovery call. Let's do that by
hand and watch a legacy server tell us where to get off:

```bash
printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"server/discover","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientInfo":{"name":"by-hand","version":"1.0.0"},"io.modelcontextprotocol/clientCapabilities":{}}}}' \
  | npx -y @modelcontextprotocol/server-everything
```

```json
{"jsonrpc":"2.0","id":1,"error":{"code":-32601,"message":"Method not found"}}
```

That's chapter three in one command: ask the modern way, get told `-32601`, fall back to
`initialize`. A modern server would instead have answered with `supportedVersions`,
`capabilities`, and `serverInfo`.

## Call Something

Listing tools is only half the conversation. Calling one is the same shape - a `tools/call`
with a name and an arguments object:

```bash
{
  printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"by-hand","version":"1.0.0"}}}'
  printf '%s\n' '{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}'
  printf '%s\n' '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"echo","arguments":{"message":"hello from the shell"}}}'
} | npx -y @modelcontextprotocol/server-everything | jq -c 'select(.id == 2)'
```

```json
{"result":{"content":[{"type":"text","text":"Echo: hello from the shell"}]},"jsonrpc":"2.0","id":2}
```

No model required, and no mystery: an MCP tool call is a function call with JSON in, JSON out,
over a pipe.

## Three Rules You Just Learned

* **One message per line.** Newlines delimit messages, so a message must never contain one.
  This is why servers log to stderr: stdout is a protocol stream, not a console.
* **Every response carries the `id` of its request.** Notifications have no `id`, and they can
  arrive at any moment. Want proof? Drop the `| jq` from the handshake pipeline and look at the
  raw order the server actually sent things:

  ```text
  {"jsonrpc":"2.0","method":"notifications/tools/list_changed"}
  {"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2025-11-25", ... }}
  ```

  The notification arrived *before* the response to `initialize`. Our client doesn't assume
  the next line is its answer; it reads until it sees the `id` it sent. So should yours.
* **Keep one request in flight.** If you send two requests and read the next line, you might
  read the wrong answer. Our client takes `&mut self` and waits for each reply - a deliberate
  simplification, and the reason the read loop is as simple as it is.

## This Is What Our Client Does

Open `code/ex17_mcp_client/src/mcp.rs` next to this chapter. `send` is the framing rule.
`read_message` is the line reader. `request` is the third rule - write, flush, read until the
`id` matches. `initialize_legacy` is the handshake above, and `list_tools` is the request
you ran in a pipeline. There is no magic in there; there is barely any code.

That's the point. When a server misbehaves in class, don't start by blaming the client - come
back to this page and ask the server directly. It will tell you what it thinks.

## References

* Transport spec (stdio framing): <https://modelcontextprotocol.io/specification/2026-07-28/basic/transports/stdio>
* Lifecycle and versioning: <https://modelcontextprotocol.io/specification/2026-07-28/basic/versioning>
* Tools (`tools/list`, `tools/call`): <https://modelcontextprotocol.io/specification/2026-07-28/server/tools>
