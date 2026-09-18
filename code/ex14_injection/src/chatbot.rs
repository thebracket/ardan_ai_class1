use futures_util::StreamExt;
use reqwest_sse::EventSource;
use serde::Deserialize;
use serde_json::json;
use tokio::sync::mpsc::Sender;

pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Vec<(String, String)>,
    pub required: Vec<String>,
}

impl ToolDefinition {
    fn to_json(&self) -> serde_json::Value {
        // A JSON Schema `properties` object maps each parameter name to its
        // schema. Build it from `parameters`, then let `json!` assemble the
        // rest of the OpenAI tool shape.
        let properties: serde_json::Map<String, serde_json::Value> = self
            .parameters
            .iter()
            .map(|(name, param_type)| (name.clone(), json!({ "type": param_type })))
            .collect();

        json!({
            "type": "function",
            "function": {
                "name": self.name,
                "description": self.description,
                "parameters": {
                    "type": "object",
                    "properties": properties,
                    "required": self.required
                }
            }
        })
    }
}

impl Default for ToolDefinition {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
            parameters: Vec::new(),
            required: Vec::new(),
        }
    }
}

pub trait ToolFactory {
    async fn call_tool(&mut self, tool_name: &str, arguments: &str) -> (String, String);
    fn tools(&self) -> Vec<ToolDefinition>;
}

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
    tool_calls: Option<Vec<ToolCallDelta>>,
}

pub enum ModelReply {
    Reasoning(String),
    Content(String),
}

/// A fully assembled tool call, ready to be echoed back to the model as part of
/// the assistant turn that requested it.
#[derive(Clone, Debug)]
pub struct StoredToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

pub enum ChatTurn {
    SystemPrompt {
        prompt: String,
    },
    User {
        prompt: String,
    },
    Assistant {
        content: Option<String>,
        tool_calls: Vec<StoredToolCall>,
    },
    ToolCall {
        id: String,
        content: String,
    },
}

impl ChatTurn {
    fn to_json(&self) -> serde_json::Value {
        match &self {
            ChatTurn::SystemPrompt { prompt } => {
                json!( { "role": "system", "content": prompt.clone() } )
            }
            ChatTurn::User { prompt } => json!( { "role" : "user", "content": prompt.clone() }),
            ChatTurn::Assistant {
                content,
                tool_calls,
            } => {
                // The API expects the assistant turn that *requested* the tools
                // to be preserved verbatim, `tool_calls` included, so the
                // `tool` messages that follow have a message to attach to.
                let mut message = json!( { "role": "assistant", "content": content });
                if !tool_calls.is_empty() {
                    message["tool_calls"] = json!(
                        tool_calls
                            .iter()
                            .map(|call| json!({
                                "id": call.id,
                                "type": "function",
                                "function": {
                                    "name": call.name,
                                    "arguments": call.arguments,
                                }
                            }))
                            .collect::<Vec<_>>()
                    );
                }
                message
            }
            ChatTurn::ToolCall { id, content } => {
                json!( { "role": "tool", "tool_call_id": id.clone(), "content": content.clone() })
            }
        }
    }
}

/// A tool call arrives split across several streaming deltas: the first carries
/// the `id` and function name, later ones carry fragments of the JSON
/// arguments. `index` tells us which call in the response a fragment belongs to.
#[derive(Deserialize, Debug)]
struct ToolCallDelta {
    index: Option<u32>,
    id: Option<String>,
    function: Option<ToolFunctionDelta>,
}

#[derive(Deserialize, Debug)]
struct ToolFunctionDelta {
    name: Option<String>,
    arguments: Option<String>,
}

/// Merge a streamed tool-call fragment into the accumulating list.
fn merge_tool_call(calls: &mut Vec<StoredToolCall>, fragment: ToolCallDelta) {
    let index = fragment.index.unwrap_or(0) as usize;
    while calls.len() <= index {
        calls.push(StoredToolCall {
            id: String::new(),
            name: String::new(),
            arguments: String::new(),
        });
    }

    let call = &mut calls[index];
    if let Some(id) = fragment.id {
        call.id = id;
    }
    if let Some(function) = fragment.function {
        if let Some(name) = function.name {
            call.name = name;
        }
        if let Some(arguments) = function.arguments {
            call.arguments.push_str(&arguments);
        }
    }
}

pub struct ChatSession<TOOLS: ToolFactory> {
    client: reqwest::Client,
    api_key: String,
    model: String,
    messages: Vec<ChatTurn>,
    tools: TOOLS,
}

impl<TOOLS: ToolFactory> ChatSession<TOOLS> {
    pub fn new(
        api_key: impl ToString,
        model: impl ToString,
        system_prompt: impl ToString,
        tools: TOOLS,
    ) -> anyhow::Result<Self> {
        let client = reqwest::Client::builder().build()?;
        let messages = vec![ChatTurn::SystemPrompt {
            prompt: system_prompt.to_string(),
        }];
        Ok(ChatSession {
            client,
            api_key: api_key.to_string(),
            model: model.to_string(),
            messages,
            tools,
        })
    }

    fn build_message(&mut self, prompt: &str) -> anyhow::Result<serde_json::Value> {
        if !prompt.is_empty() {
            self.messages.push(ChatTurn::User {
                prompt: prompt.to_string(),
            });
        }
        let tools: Vec<serde_json::Value> = self
            .tools
            .tools()
            .iter()
            .map(|tool| tool.to_json())
            .collect();
        Ok(json!({
            "model": &self.model,
            "messages": &self.get_messages()?,
            "reasoning": {
                "enabled": true
            },
            "stream": true,
            "tools": tools,
            "tool_choice": "auto"
        }))
    }

    pub async fn send_prompt(
        &mut self,
        prompt: impl ToString,
        reply: Sender<ModelReply>,
    ) -> anyhow::Result<()> {
        const URL: &str = "https://openrouter.ai/api/v1/chat/completions";

        let mut continuing = false;
        loop {
            let message = if continuing {
                self.build_message("")?
            } else {
                self.build_message(&prompt.to_string())?
            };

            let mut response = self
                .client
                .post(URL)
                .header("Content-Type", "application/json")
                .bearer_auth(&self.api_key)
                .body(message.to_string())
                .send()
                .await?
                .events()
                .await?;
            continuing = false;

            // Buffer the streamed response so we store one complete assistant
            // turn rather than a message per delta fragment.
            let mut content = String::new();
            let mut tool_calls: Vec<StoredToolCall> = Vec::new();

            while let Some(Ok(event)) = response.next().await {
                if event.event_type == "message" {
                    let Ok(choice) = serde_json::from_str::<ChoiceResponses>(&event.data) else {
                        break;
                    };
                    for delta in choice.choices {
                        if let Some(reasoning) = delta.delta.reasoning {
                            reply.send(ModelReply::Reasoning(reasoning)).await?;
                        }
                        if let Some(fragment) = delta.delta.content {
                            content.push_str(&fragment);
                            reply.send(ModelReply::Content(fragment)).await?;
                        }
                        if let Some(fragments) = delta.delta.tool_calls {
                            for fragment in fragments {
                                merge_tool_call(&mut tool_calls, fragment);
                            }
                        }
                    }
                }
            }

            // Record the assistant turn that produced this response. It has to
            // carry `tool_calls` so the `tool` results below have a matching
            // parent; providers reject a `tool` message that follows no call.
            let content = if content.is_empty() {
                None
            } else {
                Some(content)
            };
            if content.is_some() || !tool_calls.is_empty() {
                self.messages.push(ChatTurn::Assistant {
                    content,
                    tool_calls: tool_calls.clone(),
                });
            }

            // Execute the requested tools and append their results.
            for tool_call in &tool_calls {
                println!("🔧 Tool call: {}", tool_call.name);
                let (key, value) = self
                    .tools
                    .call_tool(&tool_call.name, &tool_call.arguments)
                    .await;

                // Print the raw result before it goes back into the conversation.
                // On the projector, this is the moment the injection arrives.
                println!("📨 Tool result: {value}");

                self.messages.push(ChatTurn::ToolCall {
                    id: tool_call.id.clone(),
                    content: json!({ (key): value }).to_string(),
                });
                continuing = true;
            }

            if !continuing {
                break;
            }
        }

        Ok(())
    }

    fn get_messages(&self) -> anyhow::Result<serde_json::Value> {
        let mut messages = Vec::new();
        for msg in &self.messages {
            messages.push(msg.to_json());
        }
        Ok(serde_json::to_value(&messages)?)
    }
}
