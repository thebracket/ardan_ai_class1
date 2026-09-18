# Tool Calls with Parameters

So far our tools have been a bit one-sided. `get_time` takes no arguments at all, so the model can only ask
*for* it — it can't tell it anything. The interesting tools are the ones you can hand data to: "what's the
weather in Columbia?", "roll me a d20", "add these two numbers".

> This is in the repo as `code/ex10_tools_params`.

Back when we made `ToolDefinition` generic, we quietly left room for arguments:

```rust
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Vec<(String, String)>,
    pub required: Vec<String>,
}
```

Each entry in `parameters` is a `(name, type)` pair, and `required` lists the ones the model *must* supply
(anything not listed is optional). The types are JSON Schema types — `"string"`, `"integer"`, `"number"`,
`"boolean"` and so on — because that's exactly what the model gets to see.

`to_json` turns that list into the `properties` object the API expects:

```rust
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
```

Because `parameters` is a `Vec`, a tool can have as many as you like — we just hadn't used any yet. `get_time`
still passes an empty list, which is why it uses `..Default::default()`.

Let's add a second tool that actually takes something: a dice roller. In `tools()` we describe it to the model:

```rust
ToolDefinition {
    name: "roll_dice".to_string(),
    description: "Rolls a dice with the given number of sides".to_string(),
    parameters: vec![("sides".to_string(), "integer".to_string())],
    required: vec!["sides".to_string()],
},
```

That says "there's a tool called `roll_dice`, and it needs an integer called `sides`". Which value to use is up
to the model — ask it for a d20 and it will work that out for itself.

Now the other half: *we* have to read the arguments. As we saw when we looked at the protocol, they arrive as a
*string* containing JSON:

```json
"arguments": "{\"sides\":6}"
```

So `call_tool` receives `arguments: &str` and we parse it ourselves:

```rust
"roll_dice" => {
    // Tool arguments arrive as a JSON string, e.g. `{"sides":6}`.
    // Parse it and pull out the `sides` parameter the model chose.
    let sides = serde_json::from_str::<serde_json::Value>(arguments)
        .ok()
        .and_then(|args| args.get("sides").and_then(|value| value.as_u64()))
        .unwrap_or(6)
        .max(1);
    let roll = rand::random_range(1..=sides);
    ("roll".to_string(), roll.to_string())
}
```

There are a few defensive touches here, and they're worth pointing out — you can't trust the model to give you
good arguments, and a panic inside a tool takes the whole conversation down with it:

- `.ok()` and `.and_then` mean malformed JSON falls back to a default instead of panicking.
- If `sides` is missing entirely, we default to a standard six-sided dice.
- `.max(1)` stops the model asking for a zero-sided dice. (It can't ask for a negative one — that wouldn't
  parse as a `u64`.)

We also need `cargo add rand` for `random_range`.

That's really all there is to parameters. The generic machinery does the rest: `call_tool` returns a
`(key, value)` pair, and the session wraps it into a JSON object for the tool reply — same as `unix_time`
before it.

If you add a second parameter, say `("count".to_string(), "integer".to_string())`, the model just returns both
name/value pairs in the same JSON object:

```json
"arguments": "{\"sides\":6,\"count\":3}"
```

...and you pull out whichever ones you need. Do remember that `required` only tells the model what you'd
*like*; it is a hint, not a guarantee. If your tool genuinely can't run without a parameter, handle its absence
in `call_tool` rather than assuming it will be there.
