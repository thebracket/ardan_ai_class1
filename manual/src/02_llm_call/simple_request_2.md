# Making the Call

Now let's hop into `main.rs`. We'll add a function to make LLM calls. This is an extra-annotated
version; the actual version in `code/ex02_simple_request` is cleaner!

```rust
use serde_json::json;

async fn send_prompt(api_key: String, model: String, prompt: String) -> anyhow::Result<String> {
    // Build a Reqwest client. A lot of the time you want to pool these for performance.
    let client = reqwest::Client::builder().build()?;

    // The OpenRouter path
    const URL: &str = "https://openrouter.ai/api/v1/chat/completions";

    // The json! macro is super-helpful and lets us build some arbitrary JSON quite easily.
    // In the future, we'll strongly type this.
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
        }
    }
    );

    // Send the actual POST request
    let response = client
        .post(URL) // With an URL
        .header("Content-Type", "application/json") // As JSON
        .bearer_auth(api_key) // With bearer auth
        .body(message.to_string()) // And the body
        .send()
        .await?;

    // Receive the body (it's going to be JSON)
    let body: serde_json::Value = response.json().await?;
    // And just dump it
    let content = body.to_string();

    Ok(content)
}
```
