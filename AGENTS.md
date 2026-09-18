# AGENTS.md

Guidance for AI agents assisting **Herbert Wolverson** with this Rust/AI workshop.
This is teaching material, not production code. Optimise for the student, not for
the machine.

## What this repo is

A workshop taught live. There are two halves that are deliberately kept in sync:

- **`code/`** — a Rust cargo workspace of progressive examples, `ex01_hello`
  through `ex15_provenance`. Each example is a snapshot of where the class
  should be at that point in the lesson.
- **`manual/`** — an [mdBook](https://rust-lang.github.io/mdBook/). `manual/src/`
  is the source; `manual/book/` is generated output (gitignored). Students keep a
  local copy and Herbert refers to it while teaching.

The examples build a chat client from scratch against the
[OpenRouter](https://openrouter.ai) chat-completions API, gradually introducing
JSON, streaming, SSE parsing, multi-turn state, and tool calls — using raw
`reqwest` rather than a high-level LLM SDK, so students see what is actually
happening on the wire. The later examples turn to tool *safety*: running
untrusted code in containers and sandboxes, prompt injection, and provenance.

### The single most important rule

**Examples MUST favour clarity and teaching over efficiency or cleverness.**
Do not "optimise" an example by collapsing steps, introducing abstractions early,
or using tricks a student cannot yet follow. Extra lines, explicit types, named
locals, and explanatory comments are all *desirable* here. If a change makes the
code shorter but harder to read on a projector, it is the wrong change.

This applies with extra force to the early examples. `ex01`–`ex05` exist to show
one idea at a time; keep them primitive.

## Progression and the lesson map

Examples are cumulative. Later examples intentionally duplicate earlier code so
each is self-contained (for example `ex06`–`ex09` follow the same `chatbot.rs` /
`main.rs` split). **Do not deduplicate across examples or try to make them share a
crate** — copy-forward is the point.

Manual chapter → code correspondence:

| Manual (`manual/src/`)                  | Example (`code/`)        | Topic                                  |
| --------------------------------------- | ------------------------ | -------------------------------------- |
| `01_start/intro.md`                     | `ex01_hello`             | Async hello world / toolchain check    |
| `02_llm_call/intro.md`                  | —                        | Intro to calling models                |
| `02_llm_call/simple_request_1..4.md`    | `ex02_simple_request`    | One non-streaming request              |
| `02_llm_call/streaming_1.md`            | `ex03_streaming`         | Raw byte stream                        |
| `02_llm_call/streaming_2..3.md`         | `ex04_streaming`         | Parsing SSE events by hand             |
| `02_llm_call/streaming_4.md`            | `ex05_streaming`         | Typed structs + `mpsc` channel         |
| `02_llm_call/multi_1.md`                | —                        | Why chats are stateless (JSON example) |
| `02_llm_call/multi_2.md`                | `ex06_refactor`          | `ChatSession` type, `chatbot.rs`       |
| `02_llm_call/multi_3..4.md`             | `ex07_multi_turn`        | `ChatTurn` history, growth warning     |
| `02_llm_call/tools_1.md`                | —                        | The tool-call protocol (JSON example)  |
| `02_llm_call/tools_2.md`                | `ex08_tools`             | Hard-coded `get_time` tool             |
| `02_llm_call/tools_3.md`                | `ex09_tools_generic`     | `ToolFactory` trait / `ToolDefinition` |
| `02_llm_call/tools_4.md`                | `ex10_tools_params`      | Parameterised tool (`roll_dice`)       |
| `02_llm_call/tools_structured.md`       | —                        | Structured output from tools (WIP)     |
| `02_llm_call/tools_5.md`                | `ex11_bash_tool`         | Deliberately unsafe `bash` tool        |
| `02_llm_call/tools_6.md`                | —                        | Safety options: the four axes          |
| `02_llm_call/tools_docker.md`           | `ex12_docker_safety`     | Non-root container refuses a read      |
| `02_llm_call/tools_bwrap.md`            | `ex13_bwrap_sandbox`     | `bash` in a Bubblewrap sandbox         |
| `02_llm_call/prompt_injection.md`       | —                        | Direct vs indirect prompt injection    |
| `02_llm_call/prompt_injection_2.md`     | `ex14_injection`         | Tool output smuggles an instruction    |
| `02_llm_call/provenance*.md`            | `ex15_provenance`        | Explicit vs enforced provenance        |

`multi_1.md`, `tools_1.md`, `tools_6.md`, and `prompt_injection.md` are conceptual
and reference no example. `provenance*.md` is the `provenance.md` parent plus its
three sub-pages (`provenance_explicit.md`, `provenance_enforced.md`, and
`provenance_takeaways.md`). `Structured AI Call Output` and `Governance` are
placeholders; don't assume the table is complete.

## Repo layout and build commands

```text
code/
  Cargo.toml            # workspace; every exNN crate is a member
  src/main.rs           # stub that tells you to run a workshop member
  ex01_hello/ ... ex15_provenance/
    Cargo.toml
    src/main.rs         # some also have src/chatbot.rs
    .env                # local only, gitignored (contains OPENROUTER_KEY)
manual/
  book.toml
  src/                  # edit here
  book/                 # generated by `mdbook build`, gitignored
```

Run a single example from its own directory. Each crate loads its own `.env`
from the current directory, so running from the workspace root will not find it:

```bash
cd code/ex02_simple_request
cargo run
```

Compile/check everything:

```bash
cargo check --workspace
```

Build or live-preview the manual (mdbook is installed):

```bash
mdbook build manual      # writes manual/book/
mdbook serve manual      # live preview at http://localhost:3000
```

The workspace root is `code/`, so run cargo commands from `code/` or pass
`--manifest-path code/Cargo.toml`.

## Secrets

`.env` files hold `OPENROUTER_KEY`. They are gitignored and must stay that way.

- Never print, echo, copy, or commit the value of `OPENROUTER_KEY`.
- Never add `.env` files to git or remove them from `.gitignore`.
- Code reads the key via `dotenvy::dotenv()` then
  `std::env::var("OPENROUTER_KEY")`. Keep that pattern.

`.gitignore` currently ignores secrets with the glob patterns `.env` and `.env.*`
(so every example's `.env` is covered, present or future), re-allows
`.env.example` templates, and also covers `*.pem`, `*.key`, and `*.p12`. Rust
build output is covered by `target/` and the compiled manual by `/manual/book/`.
When adding new secret-bearing files, prefer a broad pattern (and a `!` negation
for any safe template) over listing individual paths, so nothing is missed.

## Conventions in the code examples

- Rust edition 2024; dependencies are pinned to explicit versions in each crate's
  `Cargo.toml`. When adding a crate to one example, mirror the version already
  used elsewhere rather than bumping the workspace.
- `anyhow::Result` throughout, `?` for propagation. Error handling is deliberately
  light so it doesn't distract from the lesson.
- Comments explain **why**, not what. The manual calls out that it presents
  "extra-annotated" versions while the code in `code/` is the cleaner one — keep
  the repo examples readable but not over-commented relative to the prose.
- The model is `deepseek/deepseek-v4.1-flash`, with `"reasoning": { "enabled": true }`
  so reasoning deltas can be shown in green via the `colored` crate. The one
  deliberate exception is `ex14_injection`, which uses `google/gemma-3-12b-it` so
  the prompt-injection demo actually lands (see `prompt_injection_2.md`); don't
  "fix" it back.
- Streaming examples use `reqwest-sse`'s `.events()` and `futures_util::StreamExt`.
- Tool-call handling differentiates `ToolFactory` (ex09) and `merge_tool_call`
  (ex08/ex09) intentionally; those are advanced examples, but comments still need
  to be teaching-grade.
- `cargo check` must stay warning-free, but clippy lints are **not** applied
  blindly. Two are knowingly left in place; do not "fix" them without asking:
  `collapsible_if` in `ex04_streaming` (nested `if let`s are walked through in
  `streaming_3.md`; let-chains are never taught) and `derivable_impls` in
  `ex09_tools_generic` through `ex15_provenance` (the hand-written `Default`
  impl predates the still-stubbed `tools_3.md` lesson, and later tool examples
  copy it forward).

## How to help (working agreements)

1. **Keep code and manual in sync.** A change to an example usually needs a
   matching edit in its manual page (and vice versa), plus `SUMMARY.md` if a page
   is added or renamed. Check the table above to find the counterpart.
2. **Don't silently clean up.** If you spot something that looks like a mistake
   (e.g. the unused `use std::default;` in `ex09_tools_generic/src/main.rs`), ask
   before "fixing" it. Herbert may be preserving it on purpose for the live demo,
   or want to fix it in front of students.
3. **Progress in order.** New concepts belong in the next example in sequence, not
   retrofitted into earlier ones. Do not change `ex0N` to depend on a concept that
   is only introduced in `ex0M` where `M > N`.
4. **Never refactor across the whole workspace** without being asked. The
   duplication between examples is intentional.
5. **Prefer running the real thing.** Compile with `cargo check -p <crate>` after
   editing. Examples need network and a valid key to run, so don't run them
   unattended or assume a runtime failure means broken code.
6. **Match the existing voice** in the manual: first person, conversational,
   occasionally wry ("Goodness gracious…"). Don't flatten it into formal docs.
7. **When adding a new example**: add the directory under `code/`, add it to
   `[workspace] members` in `code/Cargo.toml`, create its `.env` (the existing
   `.env` glob in `.gitignore` already covers it), add a manual page under
   `manual/src/02_llm_call/`, register it in `manual/src/SUMMARY.md`, and update
   `README.md` (Examples table and Contents) via the `update-readme` skill. Keep
   the build output (`manual/book/`) out of commits.
