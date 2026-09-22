# Stdio and Streamable HTTP

There are two types of MCP servers (many can operate as either):

* `stdio` MCP servers are executed by the harness, and pass newline-delimited JSON-RPC over stdin/stdout.
* `Streamable HTTP` servers run elsewhere. The harness POSTs each message, replies come back as a JSON body or a request-scoped SSE stream (just like how we're talking to LLM servers).

The primary difference is where the MCP server runs. `stdio` driven servers run on your hardware, more like local plugins to your harness. This can be beneficial for privacy, and development velocity - but also shifts the local maintenance burden to you. `Streamable HTTP` servers run anywhere. This may have privacy implications (the same as any other hosted API), and you may be at the mercy of your provider for maintenance and updates. As always with software engineering, it's a trade-off.

> Note that the original specification from November 2024 specifying HTTP+SSE transport (two endpoints) is now deprecated. It's also worth noting that in July of 2026, protocol-level sessions and the `initialize` handshake were removed; version/capabilities were moved to a `_meta` stream. The AI world moves fast. MCP servers are more complicated than local tools, and you can expect the standard to keep changing.
