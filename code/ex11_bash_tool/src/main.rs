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
            "bash" => {
                // The arguments arrive as a JSON string, e.g. {"command":"ls"}.
                let command = serde_json::from_str::<serde_json::Value>(arguments)
                    .ok()
                    .and_then(|args| {
                        args.get("command")
                            .and_then(|value| value.as_str())
                            .map(str::to_string)
                    })
                    .unwrap_or_default();
                if command.is_empty() {
                    return ("error".to_string(), "No command provided".to_string());
                }

                // Print the command so everyone watching can see exactly what
                // the model is about to run. This is where the danger is.
                println!("💻 Running: {command}");

                // `bash -c` hands over the whole shell: pipes, redirection,
                // wildcards, `rm -rf` - the lot.
                match tokio::process::Command::new("bash")
                    .arg("-c")
                    .arg(&command)
                    .output()
                    .await
                {
                    Ok(output) => {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        ("output".to_string(), format!("{stdout}{stderr}"))
                    }
                    Err(err) => ("error".to_string(), err.to_string()),
                }
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
                name: "bash".to_string(),
                description: "Runs a shell command with bash and returns its output".to_string(),
                parameters: vec![("command".to_string(), "string".to_string())],
                required: vec!["command".to_string()],
            },
        ]
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load a local .env if there is one. Inside the demo container there isn't
    // one - the key arrives as a real environment variable instead - so we
    // ignore a missing file rather than failing.
    let _ = dotenvy::dotenv();
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
