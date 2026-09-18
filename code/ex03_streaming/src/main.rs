use futures_util::StreamExt;
use serde_json::json;

async fn send_prompt(api_key: String, model: String, prompt: String) -> anyhow::Result<String> {
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
        .bytes_stream();

    while let Some(item) = response.next().await {
        let chunk = item?;
        println!("{chunk:?}");
    }

    Ok(String::new())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Make sure you have OPENROUTER_KEY as an environment variable or in .env
    let _ = dotenvy::dotenv()?;
    let api_key = std::env::var("OPENROUTER_KEY")?;

    // Prompt the model
    let response = send_prompt(
        api_key,
        "deepseek/deepseek-v4.1-flash".to_string(),
        "This is a test message!".to_string(),
    )
    .await?;
    println!("{response}");
    Ok(())
}
