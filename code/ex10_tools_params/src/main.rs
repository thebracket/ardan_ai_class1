mod chatbot;

use chatbot::{ChatSession, ModelReply};

use crate::chatbot::{ToolDefinition, ToolFactory};

fn readln() -> anyhow::Result<String> {
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}

struct Tools;

impl ToolFactory for Tools {
    async fn call_tool(&mut self, tool_name: &str, arguments: &str) -> (String, String) {
        match tool_name {
            "get_time" => {
                let now = std::time::SystemTime::now();
                let unix_time = now.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
                ("unix_time".to_string(), unix_time.to_string())
            }
            "roll_dice" => {
                // Tool arguments arrive as a JSON string, e.g. `{"sides":6}`.
                // Parse it and pull out the `sides` parameter the model chose.
                let sides = serde_json::from_str::<serde_json::Value>(arguments)
                    .ok()
                    .and_then(|args| args.get("sides").and_then(|value| value.as_u64()))
                    .unwrap_or(6)
                    .max(1);
                let roll = rand::random_range(1..=sides);
                ("roll".to_string(), roll.to_string())
            }
            _ => (tool_name.to_string(), "Unkown Tool".to_string()),
        }
    }

    fn tools(&self) -> Vec<ToolDefinition> {
        vec![
            ToolDefinition {
                name: "get_time".to_string(),
                description: "Gets the current time in seconds since the UNIX epoch".to_string(),
                ..Default::default()
            },
            ToolDefinition {
                name: "roll_dice".to_string(),
                description: "Rolls a dice with the given number of sides".to_string(),
                parameters: vec![("sides".to_string(), "integer".to_string())],
                required: vec!["sides".to_string()],
            },
        ]
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Make sure you have OPENROUTER_KEY as an environment variable or in .env
    let _ = dotenvy::dotenv()?;
    let api_key = std::env::var("OPENROUTER_KEY")?;

    // Establish the session
    let tools = Tools {};
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
