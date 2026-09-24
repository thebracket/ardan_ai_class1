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

    // An agent takes a goal, not a chat. Use the command line if we were given
    // a question, otherwise ask for one.
    let goal = std::env::args().skip(1).collect::<Vec<_>>().join(" ");
    let goal = if goal.trim().is_empty() {
        println!("What would you like me to find out?");
        readln()?
    } else {
        goal
    };

    // The system prompt is doing a lot of the work here: it tells the model to
    // search before answering, to follow references, and to stop when it knows
    // the answer. Nudging the wording is part of tuning the agent.
    let mut session = ChatSession::new(
        api_key,
        "deepseek/deepseek-v4.1-flash",
        "You are a research agent. Answer the user's question using the notes \
         tools. Always search the notes before you answer, and follow any \
         references you find to other notes. Never guess. When you have the \
         answer, reply with only the answer.",
    )?;

    // Spawn a consumer for the streamed replies, exactly as before. The
    // reasoning is shown in green, so we can watch the agent think.
    let (tx, mut rx) = tokio::sync::mpsc::channel(32);
    let consumer = tokio::spawn(async move {
        use colored::Colorize;
        while let Some(message) = rx.recv().await {
            match message {
                ModelReply::Reasoning(text) => print!("{}", &text.green()),
                ModelReply::Content(text) => print!("{}", &text.white()),
            }
        }
    });

    println!("🎯 Goal: {goal}");
    session.run_goal(goal, tx).await?;
    println!();

    // `run_goal` dropped the sender when it returned, so the consumer will
    // finish once it has printed everything. Wait for it, or the last of the
    // answer could be lost as the program exits.
    let _ = consumer.await;

    Ok(())
}
