# Testing Your Server

A server is unusually pleasant to test, because the protocol is inspectable. You can ask it
exactly what it exposes, and call it, without involving a model at all. Do that before you wire it
into anything.

`code/ex22_mcp_server` is what we'll test. The commands below use the debug binary it produces.

The *quickest* check is the example's own demo mode, which spawns the server and calls its tools:

```bash
cargo run -- --demo
```

The rest of this page is about the tools you'd reach for from outside the crate.

## 1. By hand, over stdio

The most direct check needs no tools at all. MCP over stdio is newline-delimited JSON-RPC, so you
can talk to your own server with a pipe:

```bash
cargo build
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"smoke","version":"0.1"}}}' \
  '{"jsonrpc":"2.0","method":"notifications/initialized"}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' \
  | ./target/debug/ex22_mcp_server
```

Two lines come back - the `initialize` result and the `tools/list` result - and you can read them
as plainly as we read `server-everything`'s replies in "Talking to It by Hand". This is the same
trick we used there, only now we're on the answering side. It's also the quickest way to confirm
that the schema the macros generated for our tools is what we expected.

## 2. The Inspector

The official Inspector, which we met in "A Server to Play With", is the comfortable version. One
CLI call lists the tools:

```bash
npx @modelcontextprotocol/inspector --cli ./target/debug/ex22_mcp_server \
  -- --method tools/list
```

and one calls a tool:

```bash
npx @modelcontextprotocol/inspector --cli ./target/debug/ex22_mcp_server \
  -- --method tools/call --tool-name add --tool-arg a=2 --tool-arg b=3
```

If `tools/list` shows the schema you expected and `tools/call` returns the right content, the wire
side works. (The web UI is nicer for exploring; the CLI is what you script.)

## 3. Our own client

We already wrote a client that spawns a stdio server, lists its tools, and calls one - that's
`ex17` through `ex19`. Point it at our server by changing its two constants:

```rust
const SERVER: &str = "./target/debug/ex22_mcp_server";
const SERVER_ARGS: [&str; 0] = [];
```

and run it from `code/`. This tests both halves at once, and because our client is deliberately
hand-rolled, any mismatch in the protocol shows up loudly rather than being papered over by an SDK.

## 4. In-process tests

Most of your logic shouldn't need a transport at all. Keep the real work in ordinary functions and
let the `#[tool]` methods be a thin adapter:

```rust
fn add(a: i64, b: i64) -> i64 { a + b }

#[tool(description = "Add two numbers")]
fn add_tool(&self, Parameters(AddArgs { a, b }): Parameters<AddArgs>) -> String {
    add(a, b).to_string()
}
```

Now `add` is a plain function with plain unit tests, and there's almost nothing in the tool glue
to get wrong. For exercising the handler end to end, the SDK's own integration tests are the best
guide, and the repository's `examples/` run a client against a server in-process - quicker than
spawning a subprocess, and far quicker than a running agent.

## 5. A smoke test in CI

Because both the pipe trick and the Inspector are scriptable, a one-line `tools/list` check is a
perfectly good CI gate: it catches the classic breakage where a server starts but advertises
nothing. If you want to be thorough, call one known tool with known arguments and assert on the
result.

The habit to build: **never debug a server through a model.** Ask it what it exposes first. If the
protocol is right, any remaining problem is on the client or prompt side - and you'll have ruled
out half the system before you spend a token.
