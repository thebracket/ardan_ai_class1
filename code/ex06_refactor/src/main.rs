mod chatbot;
use chatbot::{ChatSession, ModelReply};

fn readln() -> anyhow::Result<String> {
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Make sure you have OPENROUTER_KEY as an environment variable or in .env
    let _ = dotenvy::dotenv()?;
    let api_key = std::env::var("OPENROUTER_KEY")?;

    // Establish the session
    let session = ChatSession::new(api_key, "deepseek/deepseek-v4.1-flash")?;

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
