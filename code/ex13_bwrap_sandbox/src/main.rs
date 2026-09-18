mod chatbot;

use std::path::Path;

use chatbot::{ChatSession, ModelReply};

use crate::chatbot::{ToolDefinition, ToolFactory};

fn readln() -> anyhow::Result<String> {
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}

/// Build the `bwrap` argument list that locks a command inside a tiny sandbox.
///
/// The rules are deliberately blunt:
///   * the system directories are mounted **read-only**, so the tools can run
///     but nothing on the host can be changed;
///   * the sandbox directory is the *only* thing mounted read-write, and it is
///     mounted at `/work`;
///   * `--unshare-all` puts the process in its own user, PID, network and IPC
///     namespaces, so it cannot see host processes and has no network at all;
///   * `--die-with-parent` means a crashed harness doesn't leave orphans behind.
///
/// Only the paths that actually exist are bound, so this works on distros that
/// have merged `/bin` into `/usr` and on those that haven't.
fn bwrap_args(sandbox_dir: &Path, command: &str) -> Vec<String> {
    let mut args = vec!["--unshare-all".to_string(), "--die-with-parent".to_string()];

    // Read-only system directories, so `bash`, `cat`, `ls` and their libraries
    // are available inside the sandbox.
    for dir in ["/usr", "/bin", "/lib", "/lib64", "/sbin"] {
        if Path::new(dir).exists() {
            args.push("--ro-bind".to_string());
            args.push(dir.to_string());
            args.push(dir.to_string());
        }
    }

    // A minimal /proc and /dev, needed by almost any program.
    args.push("--proc".to_string());
    args.push("/proc".to_string());
    args.push("--dev".to_string());
    args.push("/dev".to_string());

    // The one writable window: the sandbox directory, mounted at /work.
    args.push("--bind".to_string());
    args.push(sandbox_dir.display().to_string());
    args.push("/work".to_string());
    args.push("--chdir".to_string());
    args.push("/work".to_string());

    // Finally, the command to run inside the sandbox.
    args.push("bash".to_string());
    args.push("-c".to_string());
    args.push(command.to_string());

    args
}

struct Tools {
    /// The host directory exposed to the model as `/work`. Nothing outside it
    /// is visible, and it is the only place the model can write.
    sandbox_dir: std::path::PathBuf,
}

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
                // the model is about to run.
                println!("💻 Running in bwrap: {command}");

                // The command is run through `bwrap`, which creates the sandbox
                // and then executes `bash -c <command>` inside it.
                let args = bwrap_args(&self.sandbox_dir, &command);
                match tokio::process::Command::new("bwrap")
                    .args(&args)
                    .output()
                    .await
                {
                    Ok(output) => {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        // A sandbox denial is a normal tool result, not a crash:
                        // `bwrap` (or the command inside it) writes to stderr and
                        // we hand that back to the model, which explains it.
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
    // Load a local .env if there is one.
    let _ = dotenvy::dotenv();
    let api_key = std::env::var("OPENROUTER_KEY")?;

    // The sandbox directory is a real directory on the host. Inside the
    // sandbox it appears as `/work`, and it is the only writable place.
    let sandbox_dir = std::env::current_dir()?.join("sandbox");
    std::fs::create_dir_all(&sandbox_dir)?;

    // Establish the session
    let tools = Tools { sandbox_dir };
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
