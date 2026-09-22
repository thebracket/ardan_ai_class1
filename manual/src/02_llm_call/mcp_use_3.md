# How a Server Describes Itself

Everything we've called a "tool" so far, we wrote ourselves. An MCP server flips that around:
somebody else wrote the tool, and we have no idea what's in it until we ask.

The model doesn't see any of this. It sees entries in the `tools` array - a name, a description,
a JSON Schema - exactly like the ones we've been hand-writing since `ex08`. Everything between
"the server starts" and "the model can call it" is our job, and the first job is *discovery*:
launch the server, work out which revision of the protocol we're both speaking, and list what
it has to offer.

We're going to build that as a standalone example, `ex17_mcp_client`, because before we hand
tools to the model it's worth holding them in our own hands. There's no model call in this
chapter, so for once you don't need an API key.

> This is in the repo as `code/ex17_mcp_client`.

## Getting a Server

We need something to talk to. The friendliest starting point is the official "everything"
server: a reference implementation that deliberately exercises as much of the protocol as it
can. It's a Node package, so you'll need Node and `npx` installed:

```bash
node --version
npx -y @modelcontextprotocol/server-everything
```

Run that second line and... nothing appears to happen. That's correct. It's a `stdio` server:
it sits waiting for JSON-RPC on stdin and writes its answers to stdout. The `-y` tells `npx`
not to ask before downloading the package; the first run takes a few seconds, and after that
it's cached. Press Ctrl-C to stop it. (We'll meet some alternatives - and the MCP Inspector -
in **A Server to Play With**.)

Two things are worth noticing before we leave:

* Anything the server wants to *say* goes to stderr. Stdout belongs to the protocol. If a
  server ever prints a log line to stdout, it has just corrupted the conversation, and every
  client will choke on it.
* The server is just a program. We're about to launch it ourselves with
  `tokio::process::Command`.

## Spawning It

Create the crate the same way we've made every other one:

```bash
cd code
cargo new ex17_mcp_client
```

The dependencies are the lightest we've had in a while. No `reqwest`, no `colored` - just
`anyhow` for error handling, `serde_json` for the wire, and Tokio:

```toml
[dependencies]
anyhow = "1.0.104"
serde_json = "1.0.151"
tokio = { version = "1.53.1", features = ["full"] }
```

`src/main.rs` sets things up and asks the server about itself; the interesting parts live next
door in `src/mcp.rs`. Start with the launch:

```rust
pub async fn spawn(command: &str, args: &[&str]) -> Result<Self> {
    println!("Launching {} {}", command, args.join(" "));

    let mut child = Command::new(command)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        // The protocol owns stdout. Server logs go to stderr, which we
        // inherit so they show up on our terminal instead of corrupting
        // the conversation.
        .stderr(Stdio::inherit())
        // The server is our child: when the client goes, it goes.
        .kill_on_drop(true)
        .spawn()
        .with_context(|| format!("failed to run `{command}`"))?;

    let stdin = child.stdin.take().context("server has no stdin")?;
    let stdout = BufReader::new(child.stdout.take().context("server has no stdout")?);
    ...
}
```

A few details matter here:

* `stdin` and `stdout` are piped, because that *is* the conversation. `stderr` is inherited, so
  the server's logs land on your terminal where you can read them - and, more importantly,
  don't land in the protocol stream.
* `kill_on_drop(true)` is the whole lifecycle story in one line. The server is our child; when
  the `McpClient` is dropped at the end of `main`, the server dies with it.
* We keep the `Child` handle in the struct, as `_child`. Drop it and the server is killed
  instantly - we need it alive for as long as the client is.

```rust
pub struct McpClient {
    /// Held only so the server dies when we do (`kill_on_drop`).
    _child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: u64,
    ...
}
```

## One Message Per Line

stdio MCP is newline-delimited JSON-RPC: one JSON object per line, and no newlines *inside* a
message. Sending is a one-liner in the middle of some async plumbing:

```rust
async fn send(&mut self, message: Value) -> Result<()> {
    // stdio framing: one JSON object per line, and no newlines inside it.
    let mut line = serde_json::to_string(&message)?;
    line.push('\n');
    self.stdin.write_all(line.as_bytes()).await?;
    self.stdin.flush().await?;
    Ok(())
}
```

Reading needs a little more care. Messages arrive in whatever order the server feels like, and
not every message is an answer. Notifications - fire-and-forget messages with no `id` - can
turn up at any moment. So rather than assume the next line is our reply, we read until we see
the `id` we sent:

```rust
loop {
    let message = self.read_message().await?;

    // Notifications have no `id`, and can arrive at any time. They
    // aren't the answer we're waiting for, but it's worth seeing them.
    if message.get("id").is_none() {
        println!("[server notification] {}", serde_json::to_string(&message)?);
        continue;
    }

    if message["id"] == json!(id) {
        return Ok(message);
    }
    ...
}
```

Everything else in `mcp.rs` builds on those two functions.

## Saying Hello

Now the protocol part. There are two handshakes in the wild, and the difference is exactly what
we talked about in the last chapter: 2026-07-28 made MCP stateless.

Clients don't *have* to discover anything up front: in principle you can fire a normal request
and handle the `-32022` if the version doesn't fit. Probing first is what the stdio transport
recommends, because it fails deterministically against old servers instead of by timeout.

A modern server wants every request to stand alone, so our client repeats its version and
identity in `_meta` on every request. The discovery RPC looks like this:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "server/discover",
  "params": {
    "_meta": {
      "io.modelcontextprotocol/protocolVersion": "2026-07-28",
      "io.modelcontextprotocol/clientInfo": {
        "name": "workshop-mcp-client",
        "version": "1.0.0"
      },
      "io.modelcontextprotocol/clientCapabilities": {}
    }
  }
}
```

If the server is modern, that's our whole introduction. It answers with the versions it
supports, its capabilities, and its identity:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "resultType": "complete",
    "supportedVersions": ["2026-07-28", "2025-11-25"],
    "capabilities": {
      "tools": { "listChanged": true },
      "resources": { "subscribe": true, "listChanged": true },
      "prompts": { "listChanged": true }
    },
    "_meta": {
      "io.modelcontextprotocol/serverInfo": { "name": "ExampleServer", "version": "1.0.0" }
    },
    "instructions": "...",
    "ttlMs": 3600000,
    "cacheScope": "public"
  }
}
```

Older servers have never heard of `server/discover`. Ours hasn't: it answers `-32601`, "Method
not found", which is our cue to fall back to the 2025 handshake. The decision lives in
`discover`:

```rust
let code = error["code"].as_i64().unwrap_or(0);
match code {
    // -32022: a modern server that can't speak our revision, telling us
    // which ones it can. Take the legacy handshake if it's on offer.
    -32022 => {
        let supported = supported_versions(error);
        println!("  Version rejected; server speaks {supported:?}");
        if supported.contains(&LEGACY_VERSION) {
            return self.initialize_legacy().await;
        }
        anyhow::bail!(
            "server speaks {supported:?}, but this client only speaks \
             {PREFERRED_VERSION} and {LEGACY_VERSION}"
        );
    }
    // The other reserved modern errors mean the server understood us but
    // didn't like what we sent; falling back won't help.
    -32020 | -32021 => anyhow::bail!("server rejected discovery: {error}"),
    // Anything else - in practice -32601, "Method not found" - is a
    // legacy server that has never heard of `server/discover`.
    _ => {
        println!("  Not supported ({code}); falling back to the old handshake.");
        self.initialize_legacy().await
    }
}
```

The legacy handshake is the `initialize` request you'll meet in older MCP tutorials:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "initialize",
  "params": {
    "protocolVersion": "2025-11-25",
    "capabilities": {},
    "clientInfo": { "name": "workshop-mcp-client", "version": "1.0.0" }
  }
}
```

...and this is the everything server's answer, trimmed in the middle:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "protocolVersion": "2025-11-25",
    "capabilities": {
      "tools": { "listChanged": true },
      "prompts": { "listChanged": true },
      "resources": { "subscribe": true, "listChanged": true },
      "logging": {},
      "tasks": { "list": {}, "cancel": {}, "requests": { "tools": { "call": {} } } },
      "completions": {}
    },
    "serverInfo": {
      "name": "mcp-servers/everything",
      "title": "Everything Reference Server",
      "version": "2.0.0"
    },
    "instructions": "# Everything Server - Server Instructions\n\nAudience: These instructions are written for an LLM ..."
  }
}
```

Then we send `notifications/initialized` to say "ready", and the connection is negotiated. From
here on the two eras diverge again: modern requests keep carrying `_meta`, legacy requests
don't. That's the `modern` flag inside our client, flipped during discovery.

One more case is worth knowing about. A modern server that can't speak our version doesn't just
hang up; it tells us what it *can* speak, and we retry or fall back:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32022,
    "message": "Unsupported protocol version",
    "data": { "supported": ["2025-11-25", "2024-11-05"], "requested": "2026-07-28" }
  }
}
```

That's the whole negotiation: try, get told, pick something. No state required.

## The Capabilities Menu

That `capabilities` object is more interesting than it looks. It says which of the three MCP
primitives the server offers:

| Primitive | Driven by | What it is | Calls |
| --- | --- | --- | --- |
| **Tools** | the model | Functions with JSON Schema arguments | `tools/list`, `tools/call` |
| **Resources** | the application (or user) | Data addressed by URI: files, records, blobs | `resources/list`, `resources/read` |
| **Prompts** | the user | Parameterised message templates, usually surfaced as slash commands | `prompts/list`, `prompts/get` |

Only tools map onto the `tools` array we already send. Resources and prompts never reach the
model as callable things - the host decides which resource content goes into context, and the
*user* decides when to run a prompt. That's why this workshop concentrates on tools. Our
everything server advertises all three, plus legacy `logging` and a `tasks` extension, which is
exactly why it's a good test subject.

## Listing the Tools

Now the request that matters. `tools/list` takes no arguments beyond an optional `cursor`, and
the answer is a page of tool definitions. We follow `nextCursor` until the server stops handing
us one:

```rust
pub async fn list_tools(&mut self) -> Result<Vec<Value>> {
    let mut tools = Vec::new();
    let mut cursor: Option<String> = None;

    loop {
        let params = match &cursor {
            Some(cursor) => json!({ "cursor": cursor }),
            None => json!({}),
        };
        let response = self.request("tools/list", params).await?;
        let result = result_of(&response, "tools/list")?;

        if let Some(page) = result["tools"].as_array() {
            tools.extend(page.iter().cloned());
        }

        // Long lists arrive in pages; keep asking until the server stops
        // handing us cursors.
        match result["nextCursor"].as_str() {
            Some(next) => cursor = Some(next.to_string()),
            None => break,
        }
    }

    Ok(tools)
}
```

Here are two of the thirteen tools our server sent back, pretty-printed to suit the page:

```json
[
  {
    "name": "echo",
    "title": "Echo Tool",
    "description": "Echoes back the input string",
    "inputSchema": {
      "$schema": "http://json-schema.org/draft-07/schema#",
      "type": "object",
      "properties": {
        "message": { "type": "string", "description": "Message to echo" }
      },
      "required": ["message"]
    },
    "annotations": {
      "readOnlyHint": true,
      "destructiveHint": false,
      "idempotentHint": true,
      "openWorldHint": false
    },
    "execution": { "taskSupport": "forbidden" }
  },
  {
    "name": "get-structured-content",
    "title": "Get Structured Content Tool",
    "description": "Returns structured content along with an output schema for client data validation",
    "inputSchema": {
      "$schema": "http://json-schema.org/draft-07/schema#",
      "type": "object",
      "properties": {
        "location": {
          "type": "string",
          "enum": ["New York", "Chicago", "Los Angeles"],
          "description": "Choose city"
        }
      },
      "required": ["location"]
    },
    "outputSchema": {
      "$schema": "http://json-schema.org/draft-07/schema#",
      "type": "object",
      "properties": {
        "temperature": { "type": "number", "description": "Temperature in celsius" },
        "conditions": { "type": "string", "description": "Weather conditions description" },
        "humidity": { "type": "number", "description": "Humidity percentage" }
      },
      "required": ["temperature", "conditions", "humidity"],
      "additionalProperties": false
    }
  }
]
```

A guided tour:

* `name` is the identifier we'll route calls by; `title` and `description` are for humans.
  MCP names are 1-128 characters, and the spec asks for letters, digits, `_`, `-` and `.`
  only - your model provider may be pickier about function names, so sanitise on the way
  through if you meet one.
* `inputSchema` is a real JSON Schema (2020-12 by default; `$schema` can pin an older draft,
  as this 2025-era server does). It can carry per-property descriptions, `enum`s, `default`s,
  nested objects, arrays - everything our current `ToolDefinition` can't. Tools that take no
  parameters aren't allowed to send `null`: they should say
  `{ "type": "object", "additionalProperties": false }`.
* `outputSchema` is the newer half. A tool can promise the shape of its result, and if it does,
  structured results must conform to it. This is `ex16_structured` with a contract attached.
* `annotations` are hints for the host UI: read-only, destructive, idempotent, open-world.
  They are also untrusted input. A hostile server can label a destructive tool
  `"readOnlyHint": true`, and the spec is explicit that clients should only trust annotations
  from servers they trust.
* `execution` is an extension field this server happens to send (`taskSupport: "forbidden"`).
  Unknown fields are normal - servers grow new ones over time, and clients ignore what they
  don't understand.
* On a 2026-07-28 server, the *result* also carries `ttlMs` and `cacheScope`, telling the client
  how long it may cache the list and whether the list is shared between users. The `listChanged`
  capability, visible in the handshake, is how the server announces the list has changed; a
  modern client subscribes to a notification stream, a lazy one just re-lists at startup. Ours
  is lazy.

## Running It

```bash
cd code/ex17_mcp_client
cargo run
```

Here's the run from the machine this book was written on:

```text
Launching npx -y @modelcontextprotocol/server-everything
Trying `server/discover` (2026-07-28)...
Starting default (STDIO) server...
  Not supported (-32601); falling back to the old handshake.
Initializing the 2025 way (2025-11-25)...

Server:   mcp-servers/everything v2.0.0
Protocol: 2025-11-25
Capabilities:
{
  "completions": {},
  "logging": {},
  "prompts": { "listChanged": true },
  "resources": { "listChanged": true, "subscribe": true },
  "tasks": {
    "cancel": {},
    "list": {},
    "requests": { "tools": { "call": {} } }
  },
  "tools": { "listChanged": true }
}
[server notification] {"jsonrpc":"2.0","method":"notifications/tools/list_changed"}

13 tools available:
  echo                           Echoes back the input string
  get-annotated-message          Demonstrates how annotations can be used to provide metadata about content.
  get-env                        Returns all environment variables, helpful for debugging MCP server configuration
  ...
  simulate-research-query        Simulates a deep research operation that gathers, analyzes, and synthesizes information. ...
```

The `Starting default (STDIO) server...` line comes from the server's stderr, as advertised.
Then the client tries the modern discovery call, gets told it isn't supported, and settles for
the old handshake. Your output may well be shorter next time - `npx` caches the package - and
could take a different branch entirely if the server is updated to speak 2026-07-28. That's the
point of negotiating rather than assuming.

The `[server notification]` line is real protocol traffic, not debug noise: our server
announced that its tool list had changed before we'd even asked for it. Our read loop saw a
message with no `id`, printed it, and carried on waiting for the response we actually wanted.

(The program goes on to print the first tool in full - that's the next section.)

## The Shape of a Tool

Look at `echo` next to the `ToolDefinition` we wrote back in `ex09`:

```rust
ToolDefinition {
    name: "echo".to_string(),
    description: "Echoes back the input string".to_string(),
    parameters: vec![("message".to_string(), "string".to_string())],
    required: vec!["message".to_string()],
}
```

The JSON Schema on the wire is almost exactly the object our `to_json` builds for `parameters`:

```json
{
  "type": "function",
  "function": {
    "name": "echo",
    "description": "Echoes back the input string",
    "parameters": {
      "type": "object",
      "properties": {
        "message": { "type": "string" }
      },
      "required": ["message"]
    }
  }
}
```

MCP's `inputSchema` *is* the value of `parameters`, and `required` sits in the same place. The
only thing our version loses is the per-property description, which our type can't represent.
So the translation is mostly a loop and a `json!` call:

| MCP tool entry | Our `ToolDefinition` | What we do with it |
| --- | --- | --- |
| `name` | `name` | straight through (sanitised and namespaced) |
| `description` | `description` | straight through |
| `inputSchema.properties` | `parameters` | name + type per entry; extra keywords are lost |
| `inputSchema.required` | `required` | straight through |
| `title`, `annotations`, `execution`, `icons` | — | display and UI hints; drop |
| `outputSchema` | — | can't declare it; drop, or grow the type later |
| `_meta` | — | protocol extras; drop |

The design decision hiding in that table is that our `(String, String)` parameters are a
*narrowing* of MCP's schema. For a flat tool like `echo` the loss is cosmetic; a nested object
argument or an `enum` is a different story. Do we grow `ToolDefinition`, or keep the workshop
version deliberately simple? That's the question for the next chapter.

## What Discovery Costs

Discovery is a one-off conversation per server, but the *result* is not free. The tool list
lands in the model's context and stays there: every description, every schema, every `enum`,
on every turn, before the user has typed anything.

Numbers from the run above, so this isn't hand-waving:

* Thirteen tools came to about 8 kilobytes of JSON - call it 2,000 tokens once wrapped in the
  `tools` array. That's the *whole* context budget of a small model, for tools.
* The `instructions` field was 1,574 characters on its own - another few hundred tokens, from
  one `initialize` response, whether or not the host ever uses it.
* GitHub's MCP server exposes around 94 tools and ~17,600 tokens. A five-server setup can clear
  55k tokens of definitions before any work starts. We collect the mitigations in
  **Too Many Tools**.

The takeaway: connecting a server is a decision, not a freebie. The protocol makes discovery
easy; it can't make the result cheap.

## References

* Discovery: <https://modelcontextprotocol.io/specification/2026-07-28/server/discover>
* Tools, including `tools/list` and output schemas: <https://modelcontextprotocol.io/specification/2026-07-28/server/tools>
* Versioning and compatibility (modern versus legacy eras): <https://modelcontextprotocol.io/specification/2026-07-28/basic/versioning>
* `_meta` and per-request fields: <https://modelcontextprotocol.io/specification/2026-07-28/basic#meta>
* Caching hints: <https://modelcontextprotocol.io/specification/2026-07-28/server/utilities/caching>
