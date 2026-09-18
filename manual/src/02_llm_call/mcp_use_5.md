# A Server to Play With

> **Draft — planning notes.**

We need something runnable before we explain how servers are *written* — that comes much
later, in **Writing MCP Servers**.

## Covered here

* Easiest option, no build required: the official "everything" test server.

  ```bash
  # stdio
  npx -y @modelcontextprotocol/server-everything

  # Streamable HTTP on localhost
  npx @modelcontextprotocol/server-everything streamableHttp
  ```

  Source: <https://github.com/modelcontextprotocol/servers/tree/main/src/everything>
* Rust-native alternatives from the official SDK examples (`counter_stdio`,
  `calculator_stdio`):
  <https://github.com/modelcontextprotocol/rust-sdk/tree/main/examples/servers>. The SDK
  itself is <https://github.com/modelcontextprotocol/rust-sdk> — we cover it properly in
  **Writing MCP Servers**; here we only point at it.
* `npx @modelcontextprotocol/inspector` is a useful GUI for poking at any server.
* Optionally, we add our own tiny one-tool server later so the class doesn't depend on Node.
