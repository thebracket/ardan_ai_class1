# Adding State

> We're using the code `ex07_multi_turn` from the repo.

Looking at the OpenAI example we saw earlier, there are (for now) three types of state. We can represent these with an enum:

```rust
pub enum ChatTurn {
    SystemPrompt { prompt: String },
    User { prompt: String },
    Assistant { prompt: String },
}

impl ChatTurn {
    fn to_json(&self) -> serde_json::Value {
        match &self {
            ChatTurn::SystemPrompt { prompt } => {
                json!( { "role": "system", "content": prompt.clone() } )
            }
            ChatTurn::User { prompt } => json!( { "role" : "user", "content": prompt.clone() }),
            ChatTurn::Assistant { prompt } => {
                json!( { "role": "assistant", "content": prompt.clone() })
            }
        }
    }
}
```

That lets us put a vector of messages into the `ChatSession` type. We add to it:

1. On startup (system prompt).
2. When the user sends a prompt (user prompt).
3. When the assistant replies - but not their reasoning traces (assistant prompt).

Then we extend the `build_message` function to return the entire chat history.

That's all it takes to allow for multi-turn chat!
