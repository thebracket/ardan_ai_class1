//! A tiny driver for the server, so the example can be demonstrated on its own.
//!
//! It launches *this same binary* in server mode as a child process - exactly
//! what a real harness does - then connects with the SDK's client, lists the
//! tools, and calls them. No chat model, no Inspector, nothing else needed.

use anyhow::Context;
use rmcp::{
    ServiceExt, model::CallToolRequestParams, object, transport::TokioChildProcess,
};
use tokio::process::Command;

pub async fn run() -> anyhow::Result<()> {
    // Spawn ourselves with `--server`, which runs the stdio server on the
    // other end of the child process's pipes.
    let program = std::env::current_exe().context("cannot find our own binary")?;
    let mut command = Command::new(&program);
    command.arg("--server");

    let client = ().serve(TokioChildProcess::new(command)?).await?;
    if let Some(info) = client.peer_info() {
        if let Some(server) = &info.server_info {
            println!("connected to {} v{}", server.name, server.version);
        }
    }

    let tools = client.list_all_tools().await?;
    println!("\n{} tools available:", tools.len());
    for tool in &tools {
        println!(
            "  {:<10} {}",
            tool.name,
            tool.description.as_deref().unwrap_or("")
        );
    }

    // Call each tool in turn and show what comes back. The arguments are the
    // ones the schema we generated advertised.
    let result = client
        .call_tool(
            CallToolRequestParams::new("add").with_arguments(object!({ "a": 2, "b": 40 })),
        )
        .await?;
    println!("\nadd(2, 40)      -> {}", first_text(&result));

    let result = client
        .call_tool(CallToolRequestParams::new("get_time"))
        .await?;
    println!("get_time()      -> {}", first_text(&result));

    // Politely stop; dropping the client would also kill the child.
    client.cancel().await?;
    Ok(())
}

/// Pull the first text block out of a tool result.
fn first_text(result: &rmcp::model::CallToolResult) -> String {
    result
        .content
        .iter()
        .find_map(|block| block.as_text().map(|text| text.text.clone()))
        .unwrap_or_else(|| "(no text content)".to_string())
}
