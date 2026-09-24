# The Official Rust SDK

The Model Context Protocol is published by the MCP project, and it has an official Rust SDK:
[`rmcp`](https://crates.io/crates/rmcp). There's a companion procedural-macro crate, `rmcp-macros`,
which `rmcp` re-exports behind a feature flag. Between them they handle the JSON-RPC, the
lifecycle handshake, and - with a couple of attributes - the tool schema generation.

## Adding it

The crate is at version 3.x. Add it with the features you need:

```toml
[dependencies]
rmcp = { version = "3", features = ["server", "transport-io"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
anyhow = "1"
```

A few notes on those features, because the defaults are slightly surprising:

* `server` and `macros` are **on by default**, so in practice you're mostly choosing a transport.
* `transport-io` is what gives you the `stdio` transport. It is *not* in the defaults.
* For a remote server, swap it for `transport-streamable-http-server`.
* `schemars` (which generates the tool JSON Schema) comes in with `server`, and is re-exported as
  `rmcp::schemars`, so you don't need it as a separate dependency.

Pin the major version. The SDK tracks a fast-moving specification, and majors have broken the API
before (there's a 3.x migration guide in the repository for exactly that reason).

## What the SDK gives you

Strip away the marketing and there are four useful things:

1. **A `ServerHandler` trait.** Implement its methods and you have a server. Everything else is
   optional.
2. **Attribute macros.** `#[tool]`, `#[tool_router]`, and `#[tool_handler]` turn ordinary Rust
   methods into MCP tools with a generated schema. There are prompt equivalents too.
3. **The protocol types.** `ServerCapabilities`, `CallToolResult`, `ServerConfig`, `McpError`, and
   the rest of the spec, as real Rust types.
4. **Transports.** `stdio()`, and a Streamable HTTP server, so the same handler can be reached
   either way.

Two things it deliberately does *not* do: it doesn't decide which tools you should expose, and it
doesn't stop you from doing something dangerous inside one. That's all still on us - which is
where the safety chapters earn their keep.

> The SDK ships a pile of runnable examples. We borrowed one in "A Server to Play With": clone
> <https://github.com/modelcontextprotocol/rust-sdk> and run, say,
> `cargo run -p mcp-server-examples --example servers_calculator_stdio`. Reading a working server
> next to this chapter is worth ten minutes.
