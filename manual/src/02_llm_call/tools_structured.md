# Structured Output from Tools

The tool-call format is a little strange: both the tool's parameters *and* its output travel as serialized JSON inside a string. You build the parameters as JSON, and the result comes back as JSON too. Typically you end up with a single key like `value` holding a serialized blob — `\"field\":\"value\"`. Messy, but it works.

You don't have to return a single value from a tool call. It's often better to return a *series* of data points — a structured object the model can read field by field.

For example, a `get_weather` tool might accept a location. You could return a textual weather report, or you could return a structured object:

> This is in the repo as `code/ex16_structured`.

## The Tool Definition

The definition is the same shape as any other parameterised tool:

```rust
ToolDefinition {
    name: "get_weather".to_string(),
    description: "Gets the current weather and forecast for a city".to_string(),
    parameters: vec![("city".to_string(), "string".to_string())],
    required: vec!["city".to_string()],
}
```

## Returning Data, Not Prose

Inside `call_tool`, instead of formatting a sentence, we build a JSON object with named fields:

```rust
let report = serde_json::json!({
    "location": city,
    "conditions": "Sunny",
    "temperature_c": 24,
    "feels_like_c": 26,
    "humidity_percent": 62,
    "wind_kph": 14,
    "uv_index": 6,
    "precipitation_chance_percent": 20,
    "allergens": ["grass", "tree pollen"],
    "forecast": [
        { "day": "today",             "high_c": 27, "low_c": 16, "precipitation_chance_percent": 20, "conditions": "Sunny" },
        { "day": "tomorrow",          "high_c": 22, "low_c": 15, "precipitation_chance_percent": 70, "conditions": "Showers" },
        { "day": "day after tomorrow","high_c": 25, "low_c": 17, "precipitation_chance_percent": 10, "conditions": "Partly cloudy" },
    ],
});
("weather".to_string(), report.to_string())
```

(The real example hard-codes the numbers so the demo is deterministic; a production tool would be calling an API here.)

## Getting It Onto The Wire As JSON

There's one small change in `chatbot.rs`. The raw `(key, value)` pair used to be wrapped up like this:

```rust
content: json!({ (key): value }).to_string(),
```

If `value` is already a JSON string, that produces a *quoted* blob: `{"weather":"{\"location\":\"Columbia, MO\"...}"}`. The model can read it, but it's a string pretending to be an object. So we parse it first when we can, and only fall back to the string if it isn't JSON:

```rust
let parsed = match serde_json::from_str::<serde_json::Value>(&value) {
    Ok(parsed) => parsed,
    Err(_) => json!(value),
};
self.messages.push(ChatTurn::ToolCall {
    id: tool_call.id.clone(),
    content: json!({ (key): parsed }).to_string(),
});
```

Now the tool message carries a real object:

```json
{
  "role": "tool",
  "tool_call_id": "call_abc123",
  "content": "{\"weather\":{\"location\":\"Columbia, MO\",\"conditions\":\"Sunny\",\"temperature_c\":24,...}}"
}
```

## A Better Question

Because the model is handed fields and not prose, you can ask it things that need a little joining-up. Run the example:

```bash
cargo run -p ex16_structured
```

and ask:

```text
You: Do I need an umbrella today in Columbia, MO?
🔧 Tool call: get_weather
Assistant: **No umbrella needed today** — Columbia, MO is sunny right now with a high of 27°C
(about 81°F) and only a 20% chance of precipitation. ☀️

A couple of notes:
- **Tomorrow** is a different story: showers are likely with a 70% chance of rain, so you'll
  want the umbrella then.
- UV index is 6 (high), so sunscreen is a good idea if you're out midday.
- Grass and tree pollen are out, which may bother allergy sufferers.
```

It found today's precipitation chance, spotted tomorrow's, pulled in the UV index and the allergens — all without parsing anything out of a paragraph. That's the win: the *data* is unambiguous, and the model spends its effort on the answer rather than on decoding the input.

## Keep The Model Out Of Deterministic Decisions

Here's the caveat. Notice what the model just did: it read `precipitation_chance_percent: 20` and applied a rule — *under some threshold, no umbrella*. That rule came from the model's own judgement.

For a demo that's lovely. In a real system, if that rule matters — if "umbrella recommended" is a thing you care about getting right — do **not** leave it to the model. Compute it in Rust:

```rust
"umbrella_recommended": precipitation_chance_percent >= 40,
```

and return it as another field. Then the model's job is to *explain* and *converse*, which is what it's good at, rather than to act as a calculator with a policy in its head.

That's the general shape of good tool design:

* **Structured output** gives the model clean data, which it reads accurately and consistently.
* **Do the deterministic work in code** — thresholds, arithmetic, lookups, policy. You can test Rust; you can't test a vibe.
* **Let the model do the language.** It should limit its need to reason, because every inference is a place it can be wrong.

The data points flow into the model; the decisions, wherever you can pin them down, stay in your code.
