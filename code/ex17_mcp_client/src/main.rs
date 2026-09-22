mod mcp;

use anyhow::Result;
use mcp::McpClient;

/// The reference server. It tries to exercise every part of the protocol,
/// which makes it a great thing to point a brand-new client at. `npx -y`
/// downloads it on first use, then caches it.
const SERVER: &str = "npx";
const SERVER_ARGS: [&str; 2] = ["-y", "@modelcontextprotocol/server-everything"];

#[tokio::main]
async fn main() -> Result<()> {
    let mut client = McpClient::spawn(SERVER, &SERVER_ARGS).await?;

    // Step one: who are we talking to, and what do they speak?
    let server = client.discover().await?;
    println!();
    println!("Server:   {} v{}", server.name, server.version);
    println!("Protocol: {}", server.protocol_version);
    println!("Capabilities:");
    println!("{}", serde_json::to_string_pretty(&server.capabilities)?);

    // Step two: what can it do?
    let tools = client.list_tools().await?;
    println!();
    println!("{} tools available:", tools.len());
    for tool in &tools {
        println!(
            "  {:<30} {}",
            tool["name"].as_str().unwrap_or("?"),
            tool["description"].as_str().unwrap_or("")
        );
    }

    // Print one in full, so we can see the shape we'll be translating into
    // our own `ToolDefinition`.
    if let Some(first) = tools.first() {
        println!();
        println!("First tool, in full:");
        println!("{}", serde_json::to_string_pretty(first)?);
    }

    // `client` is dropped here, which kills the server - no stray processes
    // left behind.
    Ok(())
}
