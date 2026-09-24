# Wiring It Into a Harness

Once a server passes the Inspector, plugging it into an agent is small: a server is a command and
its arguments (for stdio), or a URL (for HTTP). The harness does the same thing our client did -
it launches or connects, initialises, and advertises your tools to the model.

For a stdio server you're telling the harness how to run your binary. The shape is the same
everywhere, even if the file and key names differ:

```json
{
  "mcpServers": {
    "calculator": {
      "command": "/absolute/path/to/target/release/calculator",
      "args": []
    }
  }
}
```

A few things that trip people up:

* **Use an absolute path.** The harness doesn't run in your crate directory, so `./target/debug/...`
  won't resolve. Build with `cargo build --release` and point at the release binary.
* **Restart the harness** after changing the config. Most read it at startup.
* **Keep stdout clean.** The trap from the minimal-server page applies here too - if your binary
  prints anything to stdout on startup, the handshake fails before a single tool is listed.
* **Environment matters.** If your server reads a key or a path from the environment, make sure the
  harness passes it. (Some harnesses filter the environment, which is a common source of "works in
  my shell, not in the agent".)

## What the model sees

Your tools appear in the harness's tool list, usually under a namespace so they don't collide with
built-in tools - `mcp__calculator__add`, or `calculator.add`, depending on the harness. The model
sees the **name**, the **description**, and the **input schema** you generated, and nothing else.
That trio is the entire interface, which is why the descriptions are worth as much care as the
code.

## Versioning, again

Pin your server version, and remember the lesson from the client chapters: the tool descriptions
you ship become text in someone's prompt. If you change a description, you've changed behaviour.
If you publish a server for others, treat the tool surface as an API - because that's exactly what
it is.

> Our own harness from `ex18` is just a small version of this chapter: a config, a spawn, and a
> translation of somebody else's tools into the `tools` array we send the model. Wearing the server
> hat, you now supply the other end of that same conversation.
