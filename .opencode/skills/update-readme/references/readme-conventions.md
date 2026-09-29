# README conventions

## Skeleton

The README has these sections, in this order, and nothing else:

1. `# Rust/AI Workshop`
2. `![Ardan Labs](./manual/src/ardanlabs-logo.png)`
3. The "Work in progress" blockquote (keep this; do not remove it)
4. `## Repository Layout`
5. `## Getting the Code`
6. `## Viewing the Manual`
7. `## Examples`
8. `## Contents`

The file ends after Contents. There is no other introduction, no extra status
note, and no separate "running", "model", or "secrets" section.

## Repository Layout

A single fenced `text` block showing the `code/` and `manual/` trees. Update it
only when top-level directories or notable files change.

## Getting the Code / Viewing the Manual

Fenced `bash` blocks plus at most one short sentence. The commands are:

```bash
git clone https://github.com/thebracket/ardan_ai_class1.git
cd ardan_ai_class1
```

```bash
cargo install mdbook mdbook-mermaid
cd manual
mdbook serve --open
```

and, for a static build, `mdbook build manual`. Diagrams are Mermaid blocks
rendered by `mdbook-mermaid`; the runtime is vendored in `manual/`.

## Examples table

Three columns:

```markdown
| Example | Chapter | What it shows |
| ------- | ------- | ------------- |
```

- **Example** — ``[`exNN_name`](code/exNN_name)`` (backticked link text).
- **Chapter** — `[Title](manual/src/...md)`, or `—` if no chapter exists.
- **What it shows** — one short sentence.

Rows follow the order of `[workspace] members` in `code/Cargo.toml`.

### Current mapping

| Example | Chapter | What it shows |
| ------- | ------- | ------------- |
| `ex01_hello` | `01_start/intro.md` | An async "hello world" with Tokio — a toolchain sanity check |
| `ex02_simple_request` | `02_llm_call/simple_request_1.md` | One non-streaming request, and reading the reply |
| `ex03_streaming` | `02_llm_call/streaming_1.md` | Streaming the raw bytes back from the API |
| `ex04_streaming` | `02_llm_call/streaming_2.md` | Parsing server-sent events (SSE) by hand |
| `ex05_streaming` | `02_llm_call/streaming_4.md` | Typed message structs and an `mpsc` channel |
| `ex06_refactor` | `02_llm_call/multi_2.md` | Pulling the chat logic into a `ChatSession` (`chatbot.rs`) |
| `ex07_multi_turn` | `02_llm_call/multi_3.md` | A growing `ChatTurn` history — and why it matters |
| `ex08_tools` | `02_llm_call/tools_2.md` | A hard-coded `get_time` tool and the tool-call loop |
| `ex09_tools_generic` | `02_llm_call/tools_3.md` | A `ToolFactory` trait and reusable `ToolDefinition` |
| `ex10_tools_params` | `02_llm_call/tools_4.md` | A parameterised tool (`roll_dice` with a `sides` argument) |
| `ex11_bash_tool` | `02_llm_call/tools_5.md` | The deliberately unsafe `bash` tool, confined to a container |
| `ex12_docker_safety` | `02_llm_call/tools_docker.md` | A non-root container refusing a file read, and the error reaching the model |
| `ex13_bwrap_sandbox` | `02_llm_call/tools_bwrap.md` | The `bash` tool wrapped in a Bubblewrap sandbox, with a Docker fallback for non-Linux hosts |
| `ex14_injection` | `02_llm_call/prompt_injection_2.md` | A harmless weather tool that smuggles a pirate instruction into the conversation |
| `ex15_provenance` | `02_llm_call/provenance.md` | Explicit and enforced provenance: refusing privileged tools after untrusted data |
| `ex16_structured` | `02_llm_call/tools_structured.md` | Returning structured data from a tool so the model reads fields, not prose |
| `ex17_mcp_client` | `02_llm_call/mcp_use_3.md` | Spawning a stdio MCP server, negotiating the protocol, and listing its tools |
| `ex18_mcp_tools` | `02_llm_call/mcp_use_8.md` | Translating MCP tools into `ToolDefinition`s and routing the model's calls to a server |
| `ex19_mcp_remote` | `02_llm_call/mcp_use_10.md` | A Streamable HTTP client: probe a remote server, list its tools, and call one |
| `ex20_agent` | `05_agents/simple_agent.md` | A goal-driven agent loop over a small notes corpus, with an iteration cap |
| `ex21_jev` | `05_agents/jev_classifier.md` | Calling Jev for a typed decision, then routing on it in code |
| `ex22_mcp_server` | `03_mcp_servers/a_minimal_server.md` | A minimal MCP server built with the official Rust SDK (`rmcp`) |
| `ex23_multimodal` | `02_llm_call/multimodal.md` | Sending an image with a text prompt, and reading the description back |
| `ex24_embeddings` | `04_rag/searching_by_meaning.md`, `04_rag/search_is_a_distance.md` | Local `fastembed` embeddings, cosine similarity, top-k, and a PCA plot |
| `ex25_text_search` | `04_rag/keyword_search.md`, `04_rag/full_text_search.md` | Corpus loading, naive keyword search, and SQLite FTS5 |

When this table changes, update it here too.

## Contents

Mirror `manual/src/SUMMARY.md` exactly:

- an entry whose page exists → `- [Title](manual/src/path.md)`
- an entry whose page is missing (empty `()` in SUMMARY) → `- Title (placeholder)`

Nesting and titles come straight from `SUMMARY.md`. Do not invent or reorder
chapters.

## Known intentional quirks

- `Make a Library`, `Wrap Up`, etc. are placeholders.
