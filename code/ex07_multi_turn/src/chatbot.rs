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

pub enum ModelReply {
    Reasoning(String),
    Content(String),
}

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

pub struct ChatSession {
    client: reqwest::Client,
    api_key: String,
    model: String,
    messages: Vec<ChatTurn>,
}

impl ChatSession {
    pub fn new(
        api_key: impl ToString,
        model: impl ToString,
        system_prompt: impl ToString,
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
        })
    }

    fn build_message(&mut self, prompt: &str) -> anyhow::Result<serde_json::Value> {
        self.messages.push(ChatTurn::User {
            prompt: prompt.to_string(),
        });
        Ok(json!(
            {
                "model" : &self.model,
                "messages" : &self.get_messages()?,
                "reasoning": {
                    "enabled": true
                },
                "stream": true
            }
        ))
    }

    pub async fn send_prompt(
        &mut self,
        prompt: impl ToString,
        reply: Sender<ModelReply>,
    ) -> anyhow::Result<()> {
        const URL: &str = "https://openrouter.ai/api/v1/chat/completions";

        let message = self.build_message(&prompt.to_string())?;

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

        while let Some(Ok(event)) = response.next().await {
            if event.event_type == "message" {
                let Ok(choice) = serde_json::from_str::<ChoiceResponses>(&event.data) else {
                    break;
                };
                for delta in choice.choices {
                    if let Some(reasoning) = delta.delta.reasoning {
                        reply.send(ModelReply::Reasoning(reasoning)).await?;
                    } else if let Some(content) = delta.delta.content {
                        self.messages.push(ChatTurn::Assistant {
                            prompt: content.clone(),
                        });
                        reply.send(ModelReply::Content(content)).await?;
                    }
                }
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
