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

pub struct ChatSession {
    client: reqwest::Client,
    api_key: String,
    model: String,
}

impl ChatSession {
    pub fn new(api_key: impl ToString, model: impl ToString) -> anyhow::Result<Self> {
        let client = reqwest::Client::builder().build()?;
        Ok(ChatSession {
            client,
            api_key: api_key.to_string(),
            model: model.to_string(),
        })
    }

    fn build_message(&self, prompt: &str) -> serde_json::Value {
        json!(
            {
                "model" : self.model,
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
        )
    }

    pub async fn send_prompt(
        &self,
        prompt: impl ToString,
        reply: Sender<ModelReply>,
    ) -> anyhow::Result<()> {
        const URL: &str = "https://openrouter.ai/api/v1/chat/completions";

        let message = self.build_message(&prompt.to_string());

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
                        reply.send(ModelReply::Content(content)).await?;
                    }
                }
            }
        }

        Ok(())
    }
}
