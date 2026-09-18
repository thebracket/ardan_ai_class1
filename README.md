# Rust/AI Workshop

![Ardan Labs](./manual/src/ardanlabs-logo.png)

> **Work in progress.** This is the live-teaching material for the Ardan Labs
> Rust/AI workshop. The `code/` examples are further along than the `manual/`
> prose; chapters that are still being written are marked *(planned)* below.
> Feedback very welcome.

We build a chat client from scratch against the
[OpenRouter](https://openrouter.ai) chat-completions API — starting with a single
non-streaming request and working up through SSE streaming, multi-turn state, and
tool calls. The emphasis is on raw [`reqwest`](https://docs.rs/reqwest) rather
than a high-level SDK, so you can see exactly what goes over the wire.

There are two halves, kept deliberately in sync:

- **`code/`** — a Cargo workspace of progressive examples, `ex01_hello` through
  `ex11_bash_tool`. Each example is a self-contained snapshot of where the class
  should be at that point in the lesson.
- **`manual/`** — an [mdBook](https://rust-lang.github.io/mdBook/) containing the
  slides/prose that accompany the examples.

## Getting the Code

```bash
git clone https://github.com/thebracket/ardan_ai_class1.git
cd ardan_ai_class1
```

## Viewing the Slides Locally

The manual is an mdBook. If you don't already have it:

```bash
cargo install mdbook
```

Then serve the slides with live reload:

```bash
cd manual
mdbook serve --open
```

This opens <http://localhost:3000> in your browser. To produce a static copy
instead (written to `manual/book/`, which is gitignored):

```bash
mdbook build manual
```

## Running the Examples

The examples live in the `code/` workspace. Each one needs an
[OpenRouter](https://openrouter.ai) API key in a local `.env` file:

```bash
cd code/ex02_simple_request
echo 'OPENROUTER_KEY=your-key-here' > .env
```

Then run it from the workspace root:

```bash
cd ..                       # back to code/
cargo run -p ex02_simple_request
```

To check that everything compiles:

```bash
cargo check --workspace
```

> **Note:** `ex11_bash_tool` deliberately hands a real `bash` shell to the model.
> It is a teaching example about danger, not something to run on your laptop —
> follow the container instructions in
> [DANGER Will Robinson](manual/src/02_llm_call/tools_5.md).

## Examples

| Example | Chapter | What it shows |
| ------- | ------- | ------------- |
| [`ex01_hello`](code/ex01_hello) | [Getting Started](manual/src/01_start/intro.md) | An async "hello world" with Tokio — a toolchain sanity check |
| [`ex02_simple_request`](code/ex02_simple_request) | [A Simple Request](manual/src/02_llm_call/simple_request_1.md) | One non-streaming request, and reading the reply |
| [`ex03_streaming`](code/ex03_streaming) | [Streaming Requests](manual/src/02_llm_call/streaming_1.md) | Streaming the raw bytes back from the API |
| [`ex04_streaming`](code/ex04_streaming) | [Reqwest Streams](manual/src/02_llm_call/streaming_2.md) | Parsing server-sent events (SSE) by hand |
| [`ex05_streaming`](code/ex05_streaming) | [Interactivity with Channels](manual/src/02_llm_call/streaming_4.md) | Typed message structs and an `mpsc` channel |
| [`ex06_refactor`](code/ex06_refactor) | [A Quick Refactor](manual/src/02_llm_call/multi_2.md) | Pulling the chat logic into a `ChatSession` (`chatbot.rs`) |
| [`ex07_multi_turn`](code/ex07_multi_turn) | [Adding State](manual/src/02_llm_call/multi_3.md) | A growing `ChatTurn` history — and why it matters |
| [`ex08_tools`](code/ex08_tools) | [Hard-Coded Tool](manual/src/02_llm_call/tools_2.md) | A hard-coded `get_time` tool and the tool-call loop |
| [`ex09_tools_generic`](code/ex09_tools_generic) | [Generic Tool Calls](manual/src/02_llm_call/tools_3.md) | A `ToolFactory` trait and reusable `ToolDefinition` |
| [`ex10_tools_params`](code/ex10_tools_params) | [Tool Calls with Parameters](manual/src/02_llm_call/tools_4.md) | A parameterised tool (`roll_dice` with a `sides` argument) |
| [`ex11_bash_tool`](code/ex11_bash_tool) | [DANGER Will Robinson](manual/src/02_llm_call/tools_5.md) | The deliberately unsafe `bash` tool, confined to a container |

The model used throughout is `deepseek/deepseek-v4.1-flash`, with reasoning
enabled so the reasoning deltas can be shown in green.

## Contents

The full table of contents is [`manual/src/SUMMARY.md`](manual/src/SUMMARY.md).
Chapters written so far:

- [Getting Started](manual/src/01_start/intro.md)
- [Talking to AI Models](manual/src/02_llm_call/intro.md)
  - [A Simple Request](manual/src/02_llm_call/simple_request_1.md)
    - [Making the Call](manual/src/02_llm_call/simple_request_2.md)
    - [Using The Requestor](manual/src/02_llm_call/simple_request_3.md)
    - [Understanding the Response](manual/src/02_llm_call/simple_request_4.md)
  - [Streaming Requests](manual/src/02_llm_call/streaming_1.md)
    - [Reqwest Streams](manual/src/02_llm_call/streaming_2.md)
    - [Fetching the Response](manual/src/02_llm_call/streaming_3.md)
    - [Interactivity with Channels](manual/src/02_llm_call/streaming_4.md)
  - [Multi-Turn Chats](manual/src/02_llm_call/multi_1.md)
    - [A Quick Refactor](manual/src/02_llm_call/multi_2.md)
    - [Adding State](manual/src/02_llm_call/multi_3.md)
    - [Warning: Unbounded Growth](manual/src/02_llm_call/multi_4.md)
  - [Tool Use](manual/src/02_llm_call/tools_1.md)
    - [Hard-Coded Tool](manual/src/02_llm_call/tools_2.md)
    - [Generic Tool Calls](manual/src/02_llm_call/tools_3.md)
    - [Tool Calls with Parameters](manual/src/02_llm_call/tools_4.md)
    - [DANGER Will Robinson](manual/src/02_llm_call/tools_5.md)

### Planned

- **Introduction** — Who Am I?, Format, Signposting
- **Talking to AI Models** — Safety Options for Tools (Docker, Bubblewrap),
  Governance, MCP Server Use, Make a Library, Wrap Up
- **Writing MCP Servers**
- **Retrieval Augmented Generation (RAG)**
- **Agents** — The Agentic Loop, A Very Simple Agent, Be Deterministic When
  Possible, Lots More
- **Production Concerns** — Concurrency, Retries, Observability, Testing
- **Wrap Up**

## Repository Layout

```text
code/
  Cargo.toml            # workspace; every exNN crate is a member
  ex01_hello/ ... ex11_bash_tool/
    Cargo.toml
    src/main.rs         # some also have src/chatbot.rs
    .env                # local only, gitignored (contains OPENROUTER_KEY)
manual/
  book.toml
  src/                  # edit the prose here
  theme/                # Ardan Labs styling
  book/                 # generated by `mdbook build`, gitignored
```

## Secrets

Never commit `.env` files — they hold your `OPENROUTER_KEY`. The repo's
`.gitignore` covers `.env` and `.env.*` in every directory, along with build
output (`target/`) and the compiled manual (`manual/book/`).
