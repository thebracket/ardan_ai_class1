# Hard-Coded Tool

Receiving a tool call, processing it and returning the answer introduces two new items:

1. You have to read and handle the actual tool data.
2. Your session is now *multi-turn* internally, needing to call tools and make a call with the result - stopping only when the LLM gives an answer (or you hit a limit).

> This is in the repo as `code/ex08_tools`.

So we start by defining the tool call request from the schema, and add it into the deserialized conversation:

```rust
#[derive(Deserialize, Debug)]
struct ToolCall {
    id: String,
    #[serde(alias = "type")]
    call_type: String,
    function: ToolFunction,
}

#[derive(Deserialize, Debug)]
struct ToolFunction {
    name: String,
    arguments: String,
}

#[derive(Deserialize, Debug)]
struct Message {
    reasoning: Option<String>,
    content: Option<String>,
    tool_calls: Option<Vec<ToolCall>>,
}
```

We also need to allow it as a reply type:

```rust
pub enum ChatTurn {
    SystemPrompt { prompt: String },
    User { prompt: String },
    Assistant { prompt: String },
    ToolCall { id: String, content: String },
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
            ChatTurn::ToolCall { id, content } => {
                json!( { "role": "tool", "tool_call_id": id.clone(), "content": content.clone() })
            }
        }
    }
}
```

We'll extend `build_message`:

```rust
    fn build_message(&mut self, prompt: &str) -> anyhow::Result<serde_json::Value> {
        // If the prompt is empty, we don't add it - that makes continuing easier
        if !prompt.is_empty() {
            self.messages.push(ChatTurn::User {
                prompt: prompt.to_string(),
            });
        }
        // We've hard-coded a single tool
        Ok(json!(
              {
                  "model" : &self.model,
                  "messages" : &self.get_messages()?,
                  "reasoning": {
                      "enabled": true
                  },
                  "stream": true,
                  "tools": [
                      {
                        "type": "function",
                        "function": {
                        "name": "get_time",
                        "description": "Get the current time",
                        "parameters": {
                        "type": "object",
                        "properties": { },
                        "required": []
                      }
                }
              }
        ],

                "tool_choice": "auto"

              }
          ))
    }
```

Finally, we update the actual `send_prompt` function a bit:

```rust
    pub async fn send_prompt(
        &mut self,
        prompt: impl ToString,
        reply: Sender<ModelReply>,
    ) -> anyhow::Result<()> {
        const URL: &str = "https://openrouter.ai/api/v1/chat/completions";

        // Are we continuing
        let mut continuing = false;
        loop {
            // If we're continuing - use an empty prompt (not added to history)
            let message = if continuing {
                self.build_message("")?
            } else {
                self.build_message(&prompt.to_string())?
            };

            let mut response = self // the same
            continuing = false; // Set not continuing

            while let Some(Ok(event)) = response.next().await { // Mostly the same
                if event.event_type == "message" {
                    let Ok(choice) = serde_json::from_str::<ChoiceResponses>(&event.data) else {
                        break;
                    };
                    for delta in choice.choices {
                        } else if let Some(tool_calls) = delta.delta.tool_calls {
                            //println!("{tool_calls:?}");
                            for tool_call in &tool_calls {
                                let call_id = tool_call.id.clone();
                                println!("🔧 Tool call: {}", tool_call.function.name);
                                match tool_call.function.name.as_str() {
                                    "get_time" => {
                                        let now = std::time::SystemTime::now();
                                        let unix_time =
                                            now.duration_since(std::time::UNIX_EPOCH).unwrap();
                                        self.messages.push(ChatTurn::ToolCall {
                                            id: call_id,
                                            content:
                                                json!( { "unix_time": unix_time.as_secs().to_string() } )
                                                    .to_string(),
                                        });
                                        continuing = true;
                                    }
                                    _ => println!("Unknown function: {}", tool_call.function.name),
                                }
                            }
                        }
                    }
                }
            }
            // Stop if not continuing
            if !continuing {
                break;
            }
        }

        Ok(())
    }

```

Your LLM can now tell you the time! It does a bunch of extra work, because you are giving it a UNIX timestamp - and LLMs work with
text - but the tool call works.
