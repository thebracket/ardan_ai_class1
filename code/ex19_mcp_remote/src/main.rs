mod mcp;

use anyhow::Result;
use mcp::McpClient;

/// The server we demo against. DeepWiki is public, needs no auth, and its
/// tools answer questions about GitHub repositories.
const DEFAULT_URL: &str = "https://mcp.deepwiki.com/mcp";
const DEFAULT_TOOL: &str = "read_wiki_structure";
const DEFAULT_ARGS: &str = r#"{"repoName":"modelcontextprotocol/rust-sdk"}"#;

#[tokio::main]
async fn main() -> Result<()> {
    // Everything is overridable, so pointing this at another server is a
    // command line away:
    //   cargo run -- <url> <tool> <json-arguments>
    let mut args = std::env::args().skip(1);
    let url = args.next().unwrap_or_else(|| DEFAULT_URL.to_string());
    let tool = args.next().unwrap_or_else(|| DEFAULT_TOOL.to_string());
    let tool_args = args.next().unwrap_or_else(|| DEFAULT_ARGS.to_string());

    let mut client = McpClient::connect(&url).await?;

    // Step one: who are we talking to, and what do they speak?
    let server = client.discover().await?;
    println!();
    println!("Server:   {} v{}", server.name, server.version);
    println!("Protocol: {}", server.protocol_version);
    println!("Capabilities: {}", server.capabilities);

    // Step two: ask what tools it has.
    let tools = client.list_tools().await?;
    println!();
    println!("{} tools available:", tools.len());
    for tool in &tools {
        let description = tool["description"]
            .as_str()
            .unwrap_or("")
            .lines()
            .next()
            .unwrap_or("");
        println!(
            "  {:<44} {}",
            tool["name"].as_str().unwrap_or("?"),
            description
        );
    }

    // Step three: call one, so the round trip isn't hand-waving.
    if tools
        .iter()
        .any(|candidate| candidate["name"] == tool.as_str())
    {
        let arguments: serde_json::Value = serde_json::from_str(&tool_args)?;
        println!();
        println!("Calling {tool} with {arguments}...");
        let result = client.call_tool(&tool, arguments).await?;
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!();
        println!("`{tool}` is not offered by this server; skipping the demo call.");
    }

    println!();
    println!("Usage: ex19_mcp_remote [url] [tool] [json-arguments]");
    Ok(())
}
