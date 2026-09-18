# Wiring MCP Into Our Tool System

> **Draft — planning notes.**

The payoff — and it should fit surprisingly neatly into what we already built.

## Covered here

* Translating each MCP tool descriptor into our `ToolDefinition`.
* Routing `call_tool` by name to the MCP client instead of a local `match` arm.
* Mixing local tools and MCP tools in a single request.
* Passing arguments: the model hands us a JSON *string*; MCP wants a JSON *object*.
* Name collisions and namespacing again, now that calls have to route back out.
