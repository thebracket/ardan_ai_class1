//! A tiny MCP server, built with the official Rust SDK (`rmcp`).
//!
//! It speaks the protocol over stdin/stdout, so it is meant to be *driven* by
//! something else - the MCP Inspector, one of our clients, or its own `--demo`
//! mode. See the manual's "Writing MCP Servers" section.

mod demo;

use rmcp::{
    ServerHandler, ServiceExt,
    handler::server::wrapper::Parameters,
    schemars, tool, tool_handler, tool_router,
    transport::stdio,
};

/// The arguments the `add` tool accepts. The JSON Schema the client sees is
/// generated from these fields, so the types *are* the contract - there is no
/// schema to write by hand.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct AddParams {
    a: i64,
    b: i64,
}

/// The server. `Clone` is required because the SDK shares the handler across
/// requests rather than owning it.
#[derive(Clone)]
struct WorkshopServer;

#[tool_router]
impl WorkshopServer {
    // A tool is just a method. The `#[tool]` attribute turns it into a
    // `tools/list` entry and routes `tools/call` to it.
    #[tool(description = "Add two numbers")]
    fn add(&self, Parameters(AddParams { a, b }): Parameters<AddParams>) -> String {
        (a + b).to_string()
    }

    #[tool(description = "Get the current time as a UNIX timestamp")]
    fn get_time(&self) -> String {
        let now = std::time::SystemTime::now();
        let since_epoch = now
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock is before 1970");
        since_epoch.as_secs().to_string()
    }
}

// `#[tool_handler]` fills in the `ServerHandler` implementation from the router
// above, and gives the server its identity. For a tools-only server this is all
// the handler you need.
#[tool_handler(
    name = "workshop-server",
    version = "0.1.0",
    instructions = "A tiny example MCP server from the Rust/AI workshop."
)]
impl ServerHandler for WorkshopServer {}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // One binary, two modes. With `--demo` we drive ourselves so the example is
    // runnable on its own; otherwise we are the server, waiting for a client.
    if std::env::args().any(|arg| arg == "--demo") {
        return demo::run().await;
    }

    // On the stdio transport, stdout *is* the protocol channel, so we must not
    // print to it: a stray `println!` would corrupt the conversation. Log to
    // stderr instead, where a harness or a terminal can show it safely.
    eprintln!("workshop-server listening on stdio - drive it with the Inspector:");
    eprintln!("  npx @modelcontextprotocol/inspector ./target/debug/ex22_mcp_server");

    // `serve` runs the MCP initialise handshake; `waiting` blocks until the
    // client closes the connection.
    let service = WorkshopServer.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
