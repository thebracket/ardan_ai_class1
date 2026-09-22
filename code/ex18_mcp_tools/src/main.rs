mod chatbot;
mod mcp;
mod mcp_tools;

use chatbot::{ChatSession, ModelReply};
use mcp_tools::McpTools;

use crate::chatbot::{ToolDefinition, ToolFactory};

/// The reference server from the last chapter. Nothing on the chat side cares
/// which server this is - that's the whole point of the exercise.
const SERVER: &str = "npx";
const SERVER_ARGS: [&str; 2] = ["-y", "@modelcontextprotocol/server-everything"];

fn readln() -> anyhow::Result<String> {
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}

/// Our tool system: the MCP server's tools, plus one hand-written tool to prove
/// that both kinds really do share a single list.
struct Tools {
    mcp: McpTools,
}

impl ToolFactory for Tools {
    async fn call_tool(&mut self, tool_name: &str, arguments: &str) -> (String, String) {
        match tool_name {
            "get_time" => {
                let now = std::time::SystemTime::now();
                let unix_time = now.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
                ("unix_time".to_string(), unix_time.to_string())
            }
            // Everything else the model asked for belongs to the MCP server.
            _ => self.mcp.call_tool(tool_name, arguments).await,
        }
    }

    fn tools(&self) -> Vec<ToolDefinition> {
        let mut tools = vec![ToolDefinition {
            name: "get_time".to_string(),
            description: "Gets the current time in seconds since the UNIX epoch".to_string(),
            ..Default::default()
        }];
        tools.extend(self.mcp.tools());
        tools
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Make sure you have OPENROUTER_KEY as an environment variable or in .env
    let _ = dotenvy::dotenv()?;
    let api_key = std::env::var("OPENROUTER_KEY")?;

    // Bring the server up before we bother the model with it: connect,
    // negotiate, then translate its tool list into ours.
    let mcp = McpTools::connect("everything", SERVER, &SERVER_ARGS).await?;
    let server = mcp.server();
    println!(
        "MCP server: {} v{}, speaking {}",
        server.name, server.version, server.protocol_version
    );
    println!("Capabilities:");
    println!("{}", serde_json::to_string_pretty(&server.capabilities)?);
    println!("Tools on offer:");
    println!("  get_time (ours)");
    for tool in mcp.tools() {
        println!("  {}", tool.name);
    }

    // From here on this is the chat loop from `ex16_structured`, unchanged.
    // The model has no idea that some of its tools live in another process.
    let tools = Tools { mcp };
    let mut session = ChatSession::new(
        api_key,
        "deepseek/deepseek-v4.1-flash",
        "You are a helpful assistant",
        tools,
    )?;

    // Spawn a consumer for the streamed replies. The SSE stream itself is not
    // `Send`, so it is awaited in the foreground below rather than spawned.
    let (tx, mut rx) = tokio::sync::mpsc::channel(32);
    tokio::spawn(async move {
        use colored::Colorize;
        while let Some(message) = rx.recv().await {
            match message {
                ModelReply::Reasoning(text) => print!("{}", &text.green()),
                ModelReply::Content(text) => print!("{}", &text.white()),
            }
        }
    });

    // Chat session
    loop {
        println!("Enter your message, or /quit to quit");
        let input = readln()?;
        if input == "/quit" {
            break;
        }

        session.send_prompt(input, tx.clone()).await?;
        println!();
    }

    Ok(())
}
