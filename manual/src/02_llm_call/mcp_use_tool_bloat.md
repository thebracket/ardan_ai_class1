# Too Many Tools

> **Draft — planning notes.**

Connecting an MCP server is one line of config. That is also the trap: ten minutes later you
have connected five servers and shipped the model a tool list long enough to fill a small
book, on every single turn - before the user has typed anything.

## Covered here

* The two distinct failure modes, which are easy to conflate:
  * **Context cost** - every tool definition is tokens paid on every turn.
  * **Selection quality** - past a point the model picks the wrong tool, no tool at all, or the
    same tool every time. More options make the decision *harder*, not easier.
* The numbers, so this isn't hand-waving:
  * Selection accuracy starts to fall past roughly 30-50 tools.
  * GitHub's own MCP server exposes ~94 tools consuming ~17,600 tokens; Atlassian's is
    ~10,000; a typical five-server setup (GitHub, Slack, Sentry, Grafana, Splunk) can reach
    ~55k tokens before any work begins. Anthropic has measured tool definitions at 134k
    tokens before optimisation.
* Why MCP makes this worse than plain hand-written tools:
  * One config line can add dozens of tools at once.
  * Servers duplicate each other (`read_file` in three places, each with a different schema).
  * The spec offers no grouping or partial-loading primitive - `tools/list` is a flat list, so
    every mitigation lives *above* the protocol, in the harness. That is us.
* Harness-side strategies, roughly in order of effort:
  * **Filter** - allow-list or deny-list tools per task or per conversation.
  * **Consolidate** - one parameterised `github_issues(action, ...)` instead of 40 tools.
  * **Deduplicate and namespace** - collapse collisions, prefix by server.
  * **Progressive disclosure** - expose meta-tools (`search_tools` / `load_tool`), or retrieve
    a subset by similarity to the user's message (tool RAG).
  * **Compression proxies** - e.g. Atlassian's `mcp-compressor`, which replaces a server's whole
    toolset with `get_tool_schema` + `invoke_tool` (claimed 70-97% token reduction).
* Provider-side prior art worth showing: Anthropic's **Tool Search Tool** (`defer_loading`),
  which loads only the 3-5 relevant tools, cuts token use by ~85%, and reports real accuracy
  gains (Opus 4: 49% -> 74%; Opus 4.5: 79.5% -> 88.1%). Note that our OpenRouter/OpenAI-style
  API does *not* do this for us, so a client-side version is a natural workshop exercise.
* Takeaway: start small and sharp. Every server you connect is a decision, not a freebie.

## References

* Advanced tool use (Anthropic): <https://www.anthropic.com/engineering/advanced-tool-use>
* Tool search tool (Claude docs): <https://platform.claude.com/docs/en/agents-and-tools/tool-use/tool-search-tool>
* "Context bloat" discussion (MCP Python SDK): <https://github.com/modelcontextprotocol/python-sdk/issues/2619>
* mcp-compressor (Atlassian Labs): <https://github.com/atlassian-labs/mcp-compressor>
