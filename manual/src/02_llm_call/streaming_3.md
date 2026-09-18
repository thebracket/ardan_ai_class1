# Fetching the Response

> We're still working with `ex04_streaming`.

That gives us enough to do a *quick and dirty* result extraction:

```rust
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
```

You can run the program now, and see a response to the prompt. We've done a lot of fancy streaming, and haven't really gained anything yet. Let's fix that!
