use futures_util::StreamExt;
use reqwest_sse::EventSource;
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
        .events()
        .await?;

    /*while let Some(Ok(event)) = response.next().await {
        println!("{event:#?}");
    }
    return Ok(String::new());*/

    let mut result = String::new();

    while let Some(Ok(event)) = response.next().await {
        if event.event_type == "message" {
            if let Ok(data) = serde_json::from_str::<serde_json::Value>(&event.data) {
                // Get the choices array
                let Some(choices) = data.get("choices") else {
                    break;
                };
                let Some(choices) = choices.as_array() else {
                    break;
                };
                for choice in choices {
                    let Some(delta) = choice.get("delta") else {
                        break;
                    };
                    if delta.get("reasoning").is_some() {
                        // We have reasoning! For now, skip it and only collect content.
                        break;
                    }
                    if let Some(content) = delta.get("content") {
                        if let Some(content) = content.as_str() {
                            result.push_str(content);
                        }
                    }
                }
            }
        }
    }

    Ok(result)
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
