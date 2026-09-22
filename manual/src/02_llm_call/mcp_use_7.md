# Keeping a Server Alive

> **Draft — planning notes.**

In chapter three the client spawned a server, asked two questions, and exited - and the server
went with it. That's the wrong shape for a chat app. A conversation lasts many turns, and the
server should be part of the furniture for all of them.

There's no new crate in this chapter. We're pointing `mcp.rs` at the app it's about to move
into, and talking through what changes when it lives there. The next chapter does the moving.

## Spawn Once, Keep It

Spawning a server per request is tempting and wrong. You'd pay `npx`'s package resolution, a
process start, and - on any server that still speaks a legacy revision - the `initialize`
handshake, on every single tool call. Then you'd throw away everything the server learned
while you weren't looking: subscriptions, session resources, the knowledge that you existed.

So the client we built in chapter three is the right shape already: one `McpClient`, spawned
once, living as long as the app. The chat loop borrows it when it needs a tool; the client
owns the process; the process owns the conversation's state. One client per server, too - if
you connect three servers, that's three processes, and you should budget for all of them.

## The Handle Is the Process

The important part of chapter three's `spawn` was not the command line:

```rust
let mut child = Command::new(command)
    .args(args)
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::inherit())
    // The server is our child: when the client goes, it goes.
    .kill_on_drop(true)
    .spawn()
    .with_context(|| format!("failed to run `{command}`"))?;
```

The `_child` field exists for one reason: we have to keep the `Child` alive, or `kill_on_drop`
takes our own server out. When the app exits normally - including a panic that unwinds -
destructors run, and the server dies with it. Ctrl-C is usually fine too, because the terminal
sends the interrupt to the whole foreground process group, `npx` and `node` included. The case
that bites is a hard kill or a crash that skips destructors; if you ever find a stray server:

```bash
pkill -f server-everything
```

It's not elegant, but it's honest, and it's the same lifecycle problem every harness has.

## One Request at a Time Is a Feature

Our client's methods take `&mut self`. That's not an accident of laziness: it means one request
is in flight at a time, so "write a line, then read until the `id` matches" is a complete
concurrency strategy. The chat harness is serial anyway - the model asks for one or more tool
calls, `ex09`'s loop runs them in order, and we send the results back together. Per server,
one-at-a-time costs us nothing and removes a whole class of bug.

The moment you genuinely need concurrent calls to the *same* server, this design has to change:
a background reader task, a map from `id` to the caller waiting for it, timeouts, cancellation.
That's a real client. It's also not this workshop, and adding it before we need it would hide
the protocol we're here to learn.

## The Blind Spot

While we're streaming the model's reply, nothing is reading the server's stdout. Anything it
says in that window queues in the OS pipe - fine for notifications, dangerous for a server that
chatters enough to fill it.

There's a subtler version of this: legacy servers can send *requests* (sampling, elicitation,
roots) that expect an answer. Our client ignores them, which is exactly why the everything
server's `trigger-elicitation-request` tool is not something we call in this example. Modern
2026-07-28 servers replace the common case with Multi Round-Trip Requests: a `tools/call` can
answer `input_required`, the client shows the user a prompt, and the client retries the call
with the answer attached. No held-open channel required. We don't need it for simple tools, but
it's the mechanism to reach for the moment a tool needs human confirmation.

## When the Server Dies

`read_message` returns an error when stdout closes, and a request in flight fails with it. In
the chat app, that failure should become a tool result - exactly like the permission error from
`ex12` - so the model can tell the user the tool server is gone, rather than taking the
conversation down with a panic.

Restart policy is out of scope here; if you add one, one restart per call, with the failure
reported to the model if it fails again, is a sane place to start.

## Next

The client can now live in the app. The remaining work is the interesting part: turning those
tool descriptors into our own `ToolDefinition`, and routing a model's tool call back out over
the pipe. That's the next chapter.

## References

* stdio transport: <https://modelcontextprotocol.io/specification/2026-07-28/basic/transports/stdio>
* Multi Round-Trip Requests: <https://modelcontextprotocol.io/specification/2026-07-28/basic/patterns/mrtr>
* `tokio::process::Command`: <https://docs.rs/tokio/latest/tokio/process/struct.Command.html>
