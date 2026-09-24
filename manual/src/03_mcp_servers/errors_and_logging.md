# Errors and Logging

Two things go wrong in a server, and they need different treatment. Getting this right is the
difference between a tool the model can recover from and one that just fails mysteriously.

## Tool errors vs protocol errors

There are two ways a tool can fail, and the SDK makes you choose:

* **A tool-level error** - the tool ran, but the answer is "that didn't work". Return
  `Ok(CallToolResult::error(vec![...]))`. Your message is rendered to the model, so it can read
  it and try something else. This is what you want for almost every failure: no rows matched,
  the upstream API said 500, the file isn't there.
* **A protocol error** - the server couldn't route or process the request at all. Return
  `Err(McpError::invalid_params(...))` (or another JSON-RPC code). Clients render these
  *opaquely*, so the model does **not** get to see your carefully written message.

The rule of thumb is **whose problem is it**: if the tool ran and the world said no, that's a
tool error the model should see; if the request was malformed before you could do anything, that's
a protocol error.

```rust
#[tool(description = "Look up a record")]
async fn lookup(&self, Parameters(args): Parameters<LookupArgs>)
    -> Result<CallToolResult, McpError>
{
    if args.query.is_empty() {
        // Malformed request - we can't run anything.
        return Err(McpError::invalid_params("query must be non-empty", None));
    }

    let rows = self.run_query(&args.query).await;
    if rows.is_empty() {
        // The tool ran; there's just nothing there. The model should see this.
        return Ok(CallToolResult::error(vec![ContentBlock::text(
            format!("no rows matched '{}'", args.query),
        )]));
    }

    Ok(CallToolResult::success(vec![ContentBlock::text(format_rows(&rows))]))
}
```

> This is exactly the behaviour we saw from the *client* side in the safety chapters: an error
> from a tool becomes tool output, and the model decides what to do next. If that message is
> useful, the model can recover. If it says "error", the model flails.

## Logging (and the stdout trap)

Remember that on the stdio transport, stdout is the protocol channel. A single `println!` and the
client can no longer parse the conversation. So:

* Never write to stdout. Use **stderr** for logs.
* Wire up `tracing` and a `tracing-subscriber` that writes to stderr; then `tracing::info!` and
  friends are safe from anywhere in the server.
* Keep secrets out of logs. Tool arguments come from a model, and log lines go to places people
  read and paste into issues.

The SDK also supports the MCP **logging** capability for sending structured log messages to the
*client* (rather than to stderr), with a client-set minimum level. That's useful when the harness
wants to show them; stderr is still the simplest thing that works.

## Long-running work

If a tool takes a while, two more bits of the protocol help:

* **Progress notifications** let the server report "item 3 of 10" so the client can show something
  moving.
* **Cancellation** lets the client ask the server to stop, which you handle if your work is long
  enough to care.

The modern spec also adds a **tasks** extension for work too long to hold a request open. We'll
leave that to the SDK docs and the `servers_*` examples - the important habit is the error/logging
split, which you'll need on every server you write.
