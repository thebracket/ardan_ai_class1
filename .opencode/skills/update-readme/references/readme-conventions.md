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
cargo install mdbook
cd manual
mdbook serve --open
```

and, for a static build, `mdbook build manual`.

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

When this table changes, update it here too.

## Contents

Mirror `manual/src/SUMMARY.md` exactly:

- an entry whose page exists → `- [Title](manual/src/path.md)`
- an entry whose page is missing (empty `()` in SUMMARY) → `- Title (placeholder)`

Nesting and titles come straight from `SUMMARY.md`. Do not invent or reorder
chapters.

## Known intentional quirks

- `MCP Server Use`, `Make a Library`, `Wrap Up`, etc. are placeholders.
