# The Round Trip

> **Draft — planning notes.**

The last chapter got a tool call out of the model and into the server. This one is about what
comes back, and how it becomes something the model can read. It's all `render_result` and its
helper in `mcp_tools.rs`, which is a small amount of code for a surprisingly large part of the
teaching.

> This is in the repo as `code/ex18_mcp_tools`.

## Three Shapes You'll Meet

MCP's `tools/call` result has a `content` array, an optional `structuredContent` field, and an
`isError` flag. Here are three real ones from the everything server, and what our adapter makes
of them.

**Plain text.** `echo` does what it says:

```json
{
  "content": [
    { "type": "text", "text": "Echo: hello from the shell" }
  ]
}
```

`render_result` flattens the text blocks and hands the session a `("result", text)` pair. `ChatTurn` wraps that into the tool message the model actually reads:

```json
{"result": "Echo: hello from the shell"}
```

**Structured content.** `get-structured-content` declares an `outputSchema`, and returns both a
backwards-compatible text block *and* the real object:

```json
{
  "content": [
    {
      "type": "text",
      "text": "{\"temperature\":36,\"conditions\":\"Light rain / drizzle\",\"humidity\":82}"
    }
  ],
  "structuredContent": {
    "temperature": 36,
    "conditions": "Light rain / drizzle",
    "humidity": 82
  }
}
```

We prefer `structuredContent` when it's there, so the model gets an object rather than a string
containing an object:

```json
{"result": {"temperature": 36, "conditions": "Light rain / drizzle", "humidity": 82}}
```

That's the `ex16_structured` lesson arriving from the other direction: the server declared the
shape in advance, and the host can trust the fields.

**A tool that ran and failed.** `isError: true` is the server saying "I tried, and no":

```json
{
  "content": [
    {
      "type": "text",
      "text": "MCP error -32602: Input validation error: Invalid arguments for tool get-annotated-message: Invalid option: expected one of \"error\"|\"success\"|\"debug\" at messageType"
    }
  ],
  "isError": true
}
```

We send that to the model as an error, and it gets a chance to fix its arguments:

```json
{"error": "MCP error -32602: Input validation error: ..."}
```

Here's the code that decides all three:

```rust
fn render_result(result: &Value) -> (String, String) {
    // A tool that ran and failed is still a result. The model reads the
    // message and gets a chance to fix its arguments.
    if result["isError"].as_bool() == Some(true) {
        return ("error".to_string(), content_text(result));
    }

    // Structured content is the good stuff: a real object the model can read
    // field by field, which our session will parse back out of the string.
    if let Some(structured) = result.get("structuredContent") {
        return ("result".to_string(), structured.to_string());
    }

    ("result".to_string(), content_text(result))
}
```

## The Content Zoo

Text is the common case, but the `content` array can hold other things, and a well-behaved
client should know what it's dropping. From `get-tiny-image`, trimmed (the base64 is enormous):

```json
{
  "content": [
    { "type": "text", "text": "Here's the image you requested:" },
    {
      "type": "image",
      "data": "iVBORw0KGgoAAAANSUhEUg...(base64 trimmed)",
      "mimeType": "image/png"
    },
    { "type": "text", "text": "The image above is the MCP logo." }
  ]
}
```

The protocol's content types are `text`, `image`, `audio`, `resource_link`, and `resource` (an
embedded resource, inline). Our adapter understands `text` and describes everything else:

```text
Here's the image you requested:
[the tool also returned image content, which this client ignores for now]
The image above is the MCP logo.
```

Why not send the image on? Because our chat client still sends string content to OpenRouter;
multimodal messages are their own topic. When we get there, this is where the base64 would be
turned into an image content part. Until then, telling the model what it's missing beats
silently dropping it - a model that knows an image exists can say so.

(Annotations - audience, priority, last-modified - travel alongside content items too. We
ignore those as well; they're UI hints, not data.)

## Tool Errors vs Protocol Errors

There are two very different kinds of "no", and MCP keeps them separate.

A **tool error** is a result with `isError: true`: the request was fine, the tool ran (or its
arguments failed validation), and the server is reporting an outcome. That's the one above, and
it belongs in the conversation - the model can act on it.

A **protocol error** is a JSON-RPC error envelope: the server couldn't process the request at
all. Here's a real one, produced by a deliberately malformed `tools/call`:

```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "error": {
    "code": -32603,
    "message": "[{\"expected\":\"string\",\"code\":\"invalid_type\",\"path\":[\"params\",\"name\"],\"message\":\"Invalid input: expected string, received undefined\"}]"
  }
}
```

Our `mcp.rs` turns that into an `Err`, and the adapter catches it:

```rust
Err(error) => ("error".to_string(), format!("MCP call failed: {error}")),
```

so the chat survives and the model is told. It probably can't fix a protocol error - but at
least the user hears about it, instead of the conversation dying in a stack trace.

One surprise, while you're here: the everything server reports unknown tools *and* argument
validation failures as tool errors, not protocol errors. Servers differ on where they draw
that line, so a client has to handle both.

## Several Calls in One Turn

Nothing in `chatbot.rs` changed for this, because `ex09`'s loop already handles it: the
streamed reply can contain several tool calls, we run each one, and the results go back
together in the next request. Each call is one MCP request, and because our client is
one-request-at-a-time, they run in order. If you ever want them in parallel, this is the loop
to change - and the first thing you'd need is a client that can match responses to callers
out of order.

## Timeouts and Cancellation

We don't do either, and it's the biggest gap in the example. A server that never answers makes
`request()` wait forever, which means a hung tool hangs the chat.

The spec is clear about what a real client should do: impose a timeout on every request, and on
stdio, cancel by notification:

```json
{
  "jsonrpc": "2.0",
  "method": "notifications/cancelled",
  "params": {
    "requestId": 2,
    "reason": "User requested cancellation"
  }
}
```

(Over Streamable HTTP there's no notification: closing the response stream *is* the
cancellation.)

If you want to add this to the example, `tokio::time::timeout` around `self.request(...)` is
the first half, and treating the expiry as a tool error gets it into the conversation. The
other half is sending `notifications/cancelled` with the id that timed out, so the server can
stop working. Note the request id is ours to remember - the read loop already knows it - and
that a legacy server is allowed to ignore the notification entirely.

## Where To Grow This

A few exercises, roughly in order of effort:

* **Keep the schemas.** Grow `ToolDefinition` to carry per-property descriptions (or the whole
  schema) so the model sees `a` and `b` described, not just typed.
* **Add a second server.** Point `McpTools` at the memory server as well, and watch namespacing
  stop being ceremony. The tool lists merge; the routing already works.
* **Make it robust.** Timeouts first, then cancellation, then decide what a dead server means
  for the rest of the conversation.
* **Try a real server.** The filesystem server on a scratch directory is the natural next
  experiment - it's useful, scoped, and its tool list is a very good prompt-engineering
  workout.

## References

* Tools, results, and error handling: <https://modelcontextprotocol.io/specification/2026-07-28/server/tools>
* Cancellation: <https://modelcontextprotocol.io/specification/2026-07-28/basic/patterns/cancellation>
* Progress notifications: <https://modelcontextprotocol.io/specification/2026-07-28/basic/patterns/progress>
