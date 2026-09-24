# A Minimal Server

Let's write a server that adds two numbers. It's deliberately trivial, because the point is the
*shape* of a server, not the arithmetic.

> This is in the repo as `code/ex22_mcp_server` (where it also grows a second tool, `get_time`).

```rust
use rmcp::{
    handler::server::wrapper::Parameters,
    schemars, tool, tool_handler, tool_router, ServerHandler, ServiceExt,
    transport::stdio,
};

/// The arguments the tool accepts. The JSON Schema we advertise to the client
/// is generated from these fields, so the types and doc comments are the
/// contract - there is no schema to write by hand.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct AddParams {
    a: i32,
    b: i32,
}

/// The server itself. `Clone` is required; the handler is shared, not owned.
#[derive(Clone)]
struct Calculator;

#[tool_router]
impl Calculator {
    #[tool(description = "Add two numbers")]
    fn add(&self, Parameters(AddParams { a, b }): Parameters<AddParams>) -> String {
        (a + b).to_string()
    }
}

#[tool_handler(name = "calculator", version = "1.0.0", instructions = "A simple calculator")]
impl ServerHandler for Calculator {}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // `stdio()` uses stdin and stdout as the transport. We start the service
    // (which performs the MCP initialise handshake) and then wait for it to
    // finish, which it does when the client closes the connection.
    let service = Calculator.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
```

That's a complete MCP server. Reading it top to bottom:

* **`AddParams`** is the tool's argument type. It derives `Deserialize` (to read the arguments)
  and `JsonSchema` (to advertise them). The generated `inputSchema` uses the field names, types,
  and documentation - the struct's own name is ignored.
* **`#[tool_router]`** on the `impl` block collects the `#[tool]` methods into a router. Each
  `#[tool]` method becomes one entry in `tools/list`.
* **`Parameters<T>`** is how a tool receives its arguments. It's the MCP equivalent of a web
  framework's extractor.
* **`#[tool_handler]`** on `impl ServerHandler` fills in the handler from the router and sets the
  server's identity and instructions. For a tools-only server you can shorten this with
  `#[tool_router(server_handler)]` on the `impl` block instead - but the explicit form is clearer
  when you add resources or prompts later.
* **`.serve(...)`** comes from `ServiceExt`; it runs the handshake and returns a running service.
  **`.waiting()`** blocks until the client goes away.

Build it, then point the Inspector at the binary before you point an agent at it:

```bash
cargo build
npx @modelcontextprotocol/inspector --cli ./target/debug/ex22_mcp_server \
  -- --method tools/list
```

You should see `add` come back with an `inputSchema` you never wrote by hand - and `get_time`
alongside it. That's the win the macros buy you.

`cargo run` on its own *starts* the server, which then waits for a client. But the example also has
a built-in demo mode, so you can watch a full round trip with nothing else installed:

```bash
cargo run -- --demo
```

```text
connected to workshop-server v0.1.0

2 tools available:
  add        Add two numbers
  get_time   Get the current time as a UNIX timestamp

add(2, 40)      -> 42
get_time()      -> 1790272994
```

That driver is just an SDK client that spawns our binary as a child process - the same thing a real
harness does - and calls the tools we advertised. `cargo run` without the flag is the server
itself, ready for the Inspector, raw JSON-RPC over a pipe, or one of our own clients.

> **The one trap that catches everybody:** on the stdio transport, **stdout *is* the protocol
> channel**. A stray `println!` corrupts the stream and the client will fail to parse the
> conversation. Never print to stdout in a server. Log to stderr - we'll come back to this.
