# Wiring MCP Into Our Tool System

> **Draft — planning notes.**

The last few chapters got us a living, talking MCP client and a feel for what servers look like
from the outside. This chapter is the payoff: the model calls MCP tools, and the chat loop
doesn't change.

That's worth saying plainly before we write a line. `ToolFactory` has been the seam for "where
do tools come from?" since `ex09`. Our `ChatSession` asks it for definitions, and calls it when
the model wants one. MCP slides into that seam almost without a bump. If we've done this right,
`chatbot.rs` barely changes.

> This is in the repo as `code/ex18_mcp_tools`.

## The Pieces

```text
src/chatbot.rs    copied from ex16_structured, with a `derive(Clone)` and a tidier tool line
src/mcp.rs        copied from ex17_mcp_client, plus `call_tool`
src/mcp_tools.rs  new: the adapter that translates and routes
src/main.rs       the chat loop, with a `Tools` struct that mixes local and MCP tools
```

The `Clone` derive is small but necessary: `ToolFactory::tools` hands out owned
`ToolDefinition`s, and the adapter keeps its own copy to route calls with.

## The Adapter

`McpTools` is one server, wearing the same clothes as our hand-written tools:

```rust
pub struct McpTools {
    client: McpClient,
    server: ServerDescription,
    tools: Vec<ToolDefinition>,
    /// The name we showed the model -> the name the server knows.
    server_names: HashMap<String, String>,
}
```

Setting it up is the three-step routine from chapter three, then a translation pass:

```rust
pub async fn connect(namespace: &str, command: &str, args: &[&str]) -> Result<Self> {
    let mut client = McpClient::spawn(command, args).await?;
    let server = client.discover().await?;

    let mut tools = Vec::new();
    let mut server_names = HashMap::new();
    for raw in client.list_tools().await? {
        let definition = translate(namespace, &raw);
        server_names.insert(
            definition.name.clone(),
            raw["name"].as_str().unwrap_or_default().to_string(),
        );
        tools.push(definition);
    }

    Ok(Self {
        client,
        server,
        tools,
        server_names,
    })
}
```

The server is now up, negotiated, and holding a tool list we understand. Nothing has touched
the model yet.

## Translating a Descriptor

`translate` is where MCP's rich JSON Schema meets our deliberately simple `ToolDefinition`.
Here's the real `get-sum` descriptor from the everything server, trimmed to the interesting
parts:

```json
{
  "name": "get-sum",
  "title": "Get Sum Tool",
  "description": "Returns the sum of two numbers",
  "inputSchema": {
    "type": "object",
    "properties": {
      "a": { "type": "number", "description": "First number" },
      "b": { "type": "number", "description": "Second number" }
    },
    "required": ["a", "b"]
  }
}
```

...and what comes out the other side:

```rust
ToolDefinition {
    name: "everything__get-sum".to_string(),
    description: "Returns the sum of two numbers".to_string(),
    parameters: vec![
        ("a".to_string(), "number".to_string()),
        ("b".to_string(), "number".to_string()),
    ],
    required: vec!["a".to_string(), "b".to_string()],
}
```

The code is a walk over `inputSchema.properties`, taking each property's `type` and leaning on
the required list as-is:

```rust
fn translate(namespace: &str, raw: &Value) -> ToolDefinition {
    let schema = &raw["inputSchema"];

    let required = schema["required"]
        .as_array()
        .map(|names| {
            names
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();

    let parameters = schema["properties"]
        .as_object()
        .map(|properties| {
            properties
                .iter()
                .map(|(name, property)| {
                    // A nested schema becomes a bare `"object"` or `"array"`;
                    // the server still validates the real thing on the way in.
                    let kind = property["type"].as_str().unwrap_or("string");
                    (name.clone(), kind.to_string())
                })
                .collect()
        })
        .unwrap_or_default();

    ToolDefinition {
        name: format!("{namespace}__{}", raw["name"].as_str().unwrap_or("unnamed")),
        description: raw["description"].as_str().unwrap_or("").to_string(),
        parameters,
        required,
    }
}
```

Two decisions are hiding in there.

**We are narrowing the schema.** Per-property descriptions, `enum`s, defaults, and nested
object arguments are all dropped, because `(String, String)` can't carry them. For a flat tool
like `get-sum` that costs us nothing that matters. For a tool that expects an object, the model
gets told "object" and the server validates the details on the way in. If you want to keep the
full schema, grow `ToolDefinition` - that's the exercise on the next page.

**We are namespacing.** Every tool is prefixed with the namespace we chose at `connect` -
`main.rs` passes `"everything"` for our single server, so `get-sum` becomes
`everything__get-sum`. With one server it looks like ceremony. With two, it's the difference
between a `search` tool that routes to GitHub and a `search` tool that routes to Slack.
`server_names` remembers the mapping, which is what makes the return trip possible.

## Routing the Call

When the model asks for a tool, `ChatSession` calls our `Tools::call_tool`, which sends
everything that isn't the local `get_time` down to the adapter:

```rust
pub async fn call_tool(&mut self, tool_name: &str, arguments: &str) -> (String, String) {
    let Some(server_name) = self.server_names.get(tool_name).cloned() else {
        return ("error".to_string(), format!("Unknown tool: {tool_name}"));
    };

    // The model hands us arguments as a JSON *string*; MCP wants a JSON
    // *object*. If the model sent nonsense, send an empty object and let
    // the server explain what's missing.
    let arguments = serde_json::from_str::<Value>(arguments).unwrap_or_else(|_| json!({}));

    match self.client.call_tool(&server_name, arguments).await {
        Ok(result) => render_result(&result),
        // A protocol-level failure is data too: hand it to the model rather
        // than panicking the chat loop.
        Err(error) => ("error".to_string(), format!("MCP call failed: {error}")),
    }
}
```

Three things worth noticing:

* The model's arguments arrive as a *string* containing JSON (we've seen this since `ex08`),
  and MCP wants a real object. That's the one place the two protocols rub together.
* We look the name up before calling. If the model hallucinates a tool, we never bother the
  server - it gets an error result and can recover.
* Anything the server says, including "no", comes back as a `(key, value)` pair. Nothing in
  this path can panic the chat loop.

`render_result` does the return trip: MCP result in, tool-message material out. It's short, and
it's interesting enough to have its own chapter - which is next.

## Mixing Local and MCP Tools

The chat loop's `Tools` owns the adapter, keeps one local tool, and merges the lists:

```rust
struct Tools {
    mcp: McpTools,
}

impl ToolFactory for Tools {
    async fn call_tool(&mut self, tool_name: &str, arguments: &str) -> (String, String) {
        match tool_name {
            "get_time" => {
                let now = std::time::SystemTime::now();
                let unix_time = now.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
                ("unix_time".to_string(), unix_time.to_string())
            }
            // Everything else the model asked for belongs to the MCP server.
            _ => self.mcp.call_tool(tool_name, arguments).await,
        }
    }

    fn tools(&self) -> Vec<ToolDefinition> {
        let mut tools = vec![ToolDefinition {
            name: "get_time".to_string(),
            description: "Gets the current time in seconds since the UNIX epoch".to_string(),
            ..Default::default()
        }];
        tools.extend(self.mcp.tools());
        tools
    }
}
```

The model sees one flat list, fourteen entries long, with no idea that one of them came from
`main.rs` and thirteen arrived over a pipe. That's the whole claim of MCP in a function
signature.

## Running It

The example reads `OPENROUTER_KEY` from the shared `code/.env` (see **What You Need**), then:

```bash
cd code/ex18_mcp_tools
cargo run
```

Before any model traffic, the client does its negotiation and prints what it found. This is a
real run:

```text
Launching npx -y @modelcontextprotocol/server-everything
Trying `server/discover` (2026-07-28)...
  Not supported (-32601); falling back to the old handshake.
Initializing the 2025 way (2025-11-25)...
[server notification] {"jsonrpc":"2.0","method":"notifications/tools/list_changed"}
MCP server: mcp-servers/everything v2.0.0, speaking 2025-11-25
Capabilities:
{ ... }
Tools on offer:
  get_time (ours)
  everything__echo
  everything__get-annotated-message
  everything__get-env
  ...
  everything__simulate-research-query
Enter your message, or /quit to quit
```

Now ask it for things that need the server. Roughly what to try:

```text
What is 21 + 34? Use the get-sum tool.
Echo this back exactly: the llama is loose.
What's the weather in Chicago? Use the structured content tool.
What time is it in unix seconds?
```

The last one is our local tool; the first three are the server's. Here are two turns of a real
run, with the reasoning abbreviated:

```text
You: What is 21 + 34? Use the get-sum tool.
The user wants me to use the get-sum tool to calculate 21 + 34.
🔧 Tool call: everything__get-sum
21 + 34 = **55**
You: Echo this back exactly: the llama is loose.

🔧 Tool call: everything__echo
The tool returned: `Echo: the llama is loose`
```

The reasoning streams first (green in a terminal), then the tool line gets a row of its own -
the example prints a newline before it, because streamed text doesn't end with one. Then the
answer arrives in white. If you see the tool call, the round trip worked. If the model ignores
the tool, phrase the prompt more directly (`Use the echo tool...` is a perfectly honest way to
steer a demo), because tool *selection* is a model problem, not a plumbing one.

## Next

We can get a tool call out to a server and hand something back. The next chapter is about that
something: the result shapes an MCP server can return, and how each one becomes a message the
model can read.

## References

* Tools (`tools/list`, `tools/call`): <https://modelcontextprotocol.io/specification/2026-07-28/server/tools>
* JSON Schema usage in MCP: <https://modelcontextprotocol.io/specification/2026-07-28/basic#json-schema-usage>
