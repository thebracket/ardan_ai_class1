use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde::Deserialize;
use serde_json::json;

// The non-streaming reply shape. The interesting part is `message.content`:
// whether or not we sent a picture, the answer comes back as plain text.
#[derive(Deserialize, Debug)]
struct ChoiceResponses {
    choices: Vec<Choice>,
}

#[derive(Deserialize, Debug)]
struct Choice {
    message: Message,
}

#[derive(Deserialize, Debug)]
struct Message {
    content: Option<String>,
}

async fn describe_image(
    api_key: String,
    model: String,
    image_path: String,
    prompt: String,
) -> anyhow::Result<String> {
    // A picture travels as base64 text inside the JSON body. Read the file,
    // then encode it - the model never sees the path, only the bytes.
    let image_bytes = std::fs::read(&image_path)?;
    let image_base64 = STANDARD.encode(&image_bytes);

    // OpenRouter wants a MIME type in the data URL. We keep it simple and
    // infer it from the file extension.
    let mime_type = if image_path.ends_with(".jpg") || image_path.ends_with(".jpeg") {
        "image/jpeg"
    } else {
        "image/png"
    };

    let client = reqwest::Client::builder().build()?;
    const URL: &str = "https://openrouter.ai/api/v1/chat/completions";

    // This is the whole trick: `content` is an *array* of parts instead of a
    // plain string. One part is the text, the other is the image.
    let message = json!(
    {
        "model" : model,
        "messages" : [
            {
                "role": "user",
                "content": [
                    {
                        "type": "text",
                        "text": prompt
                    },
                    {
                        "type": "image_url",
                        "image_url": {
                            "url": format!("data:{mime_type};base64,{image_base64}")
                        }
                    }
                ]
            }
        ],
        "reasoning": {
            "enabled": true
        }
    }
    );

    let response = client
        .post(URL)
        .header("Content-Type", "application/json")
        .bearer_auth(api_key)
        .body(message.to_string())
        .send()
        .await?;

    let body: ChoiceResponses = response.json().await?;

    for choice in body.choices {
        if let Some(content) = choice.message.content {
            return Ok(content);
        }
    }

    Ok("The model sent no text back.".to_string())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Make sure you have OPENROUTER_KEY as an environment variable or in .env
    let _ = dotenvy::dotenv()?;
    let api_key = std::env::var("OPENROUTER_KEY")?;

    // Use the picture passed on the command line, or the crab we ship with
    // the example.
    let image_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "bert.png".to_string());

    let response = describe_image(
        api_key,
        "deepseek/deepseek-v4.1-flash".to_string(),
        image_path,
        "Please describe the picture.".to_string(),
    )
    .await?;

    println!("{response}");
    Ok(())
}
