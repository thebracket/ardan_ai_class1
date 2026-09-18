use futures_util::StreamExt;
use reqwest_sse::EventSource;
use serde::Deserialize;
use serde_json::json;
use tokio::sync::mpsc::Sender;

#[derive(Deserialize, Debug)]
struct ChoiceResponses {
    choices: Vec<Delta>,
}

#[derive(Deserialize, Debug)]
struct Delta {
    delta: Message,
}

#[derive(Deserialize, Debug)]
struct Message {
    reasoning: Option<String>,
    content: Option<String>,
}

enum ModelReply {
    Reasoning(String),
    Content(String),
}

async fn send_prompt(
    api_key: String,
    model: String,
    prompt: String,
    reply: Sender<ModelReply>,
) -> anyhow::Result<()> {
    let client = reqwest::Client::builder().build()?;
    const URL: &str = "https://openrouter.ai/api/v1/chat/completions";

    let message = json!(
    {
        "model" : model,
        "messages" : [
            {
                "role": "user",
                "content": prompt
            }
        ],
        "reasoning": {
            "enabled": true
        },
        "stream": true
    }
    );

    let mut response = client
        .post(URL)
        .header("Content-Type", "application/json")
        .bearer_auth(api_key)
        .body(message.to_string())
        .send()
        .await?
        .events()
        .await?;
    while let Some(Ok(event)) = response.next().await {
        if event.event_type == "message" {
            let Ok(choice) = serde_json::from_str::<ChoiceResponses>(&event.data) else {
                break;
            };
            for delta in choice.choices {
                if let Some(reasoning) = delta.delta.reasoning {
                    reply.send(ModelReply::Reasoning(reasoning)).await?;
                } else if let Some(content) = delta.delta.content {
                    reply.send(ModelReply::Content(content)).await?;
                }
            }
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Make sure you have OPENROUTER_KEY as an environment variable or in .env
    let _ = dotenvy::dotenv()?;
    let api_key = std::env::var("OPENROUTER_KEY")?;

    // Make the stream
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

    // Prompt the model
    send_prompt(
        api_key,
        "deepseek/deepseek-v4.1-flash".to_string(),
        "Please write a 3 paragraph story about a boy who finds a dog in the park".to_string(),
        tx,
    )
    .await?;

    println!();
    Ok(())
}
