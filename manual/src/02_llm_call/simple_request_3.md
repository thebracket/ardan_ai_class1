# Using The Requestor

Now that we have the function, let's use it:

```rust
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
```

So we're sending "This is a test message" to DeepSeek V4.1 Flash. Let's see what it returns (your result may vary - LLMs are not deterministic). I've
prettied up the JSON.

```json
{
  "choices": [
    {
      "finish_reason": "stop",
      "index": 0,
      "logprobs": null,
      "message": {
        "content": "Test received! ✅ How can I help you?",
        "reasoning": "We need answer to user. User says \"This is a test message!\" Need likely respond appropriately. We are API assistant. Need concise. Maybe acknowledge. No task. Need final.",
        "reasoning_details": [
          {
            "format": "unknown",
            "index": 0,
            "text": "We need answer to user. User says \"This is a test message!\" Need likely respond appropriately. We are API assistant. Need concise. Maybe acknowledge. No task. Need final.",
            "type": "reasoning.text"
          }
        ],
        "refusal": null,
        "role": "assistant"
      },
      "native_finish_reason": "stop"
    }
  ],
  "created": 1789576175,
  "id": "gen-1789576175-0fSB98Lp6F52Kff82hh3",
  "model": "deepseek/deepseek-v4.1-flash",
  "object": "chat.completion",
  "provider": "Fireworks",
  "service_tier": null,
  "system_fingerprint": null,
  "usage": {
    "completion_tokens": 49,
    "completion_tokens_details": {
      "audio_tokens": 0,
      "image_tokens": 0,
      "reasoning_tokens": 37
    },
    "cost": 0.00004026,
    "cost_details": {
      "upstream_inference_completions_cost": 0.00003234,
      "upstream_inference_cost": 0.00004026,
      "upstream_inference_prompt_cost": 0.00000792
    },
    "is_byok": false,
    "prompt_tokens": 36,
    "prompt_tokens_details": {
      "audio_tokens": 0,
      "cache_write_tokens": 0,
      "cached_tokens": 0,
      "video_tokens": 0
    },
    "total_tokens": 85
  }
}
```
