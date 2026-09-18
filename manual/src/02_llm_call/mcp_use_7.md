# Spawning a stdio Server

> **Draft — planning notes.**

Our client becomes the host: it launches the server and keeps it alive across turns.

## Covered here

* Spawning the subprocess with `tokio::process::Command`, wiring up stdin and stdout.
* Sending discovery (`server/discover`, falling back to `initialize`), then `tools/list`.
* Reading newline-framed JSON from stdout without blocking the rest of the program.
* Lifecycle: keep the child running between turns, and make sure it dies with us.
* Where this code lives: a new `mcp.rs` alongside `chatbot.rs`.
