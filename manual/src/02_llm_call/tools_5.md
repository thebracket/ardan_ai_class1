# DANGER Will Robinson

You now have an interface through which you can expose *any* code. It could call an API, send commands to bash. Consequently, it could perform `rm -rf /` and erase your hard drive. It could grab all of your customer data and send it to *insert nefarious body here*. You obviously don't want that!

It's always tempting to make very generic tool calls. Let the agent write some code. Let the agent browse the web. Let the agent call `curl`. Maybe let the agent run `bash` commands.

We're going to talk about *governance* and *safety* in future sections, where we discuss permissions and auditing. But for now, **be careful**.

So let's make the mistake on purpose, in a place where it can't hurt anyone.

> This is in the repo as `code/ex11_bash_tool`.

The tool itself is tiny. We describe it to the model like any other:

```rust
ToolDefinition {
    name: "bash".to_string(),
    description: "Runs a shell command with bash and returns its output".to_string(),
    parameters: vec![("command".to_string(), "string".to_string())],
    required: vec!["command".to_string()],
},
```

And then, in `call_tool`, we do the unthinkable:

```rust
"bash" => {
    // The arguments arrive as a JSON string, e.g. {"command":"ls"}.
    let command = serde_json::from_str::<serde_json::Value>(arguments)
        .ok()
        .and_then(|args| {
            args.get("command")
                .and_then(|value| value.as_str())
                .map(str::to_string)
        })
        .unwrap_or_default();
    if command.is_empty() {
        return ("error".to_string(), "No command provided".to_string());
    }

    println!("💻 Running: {command}");

    match tokio::process::Command::new("bash")
        .arg("-c")
        .arg(&command)
        .output()
        .await
    {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            ("output".to_string(), format!("{stdout}{stderr}"))
        }
        Err(err) => ("error".to_string(), err.to_string()),
    }
}
```

`bash -c` hands over the *whole shell*, not just a single program: pipes, redirection, wildcards, `&&`, `rm -rf`. Whatever the model thinks of, we run. The `println!` is there so everyone watching can see the command before the damage is done.

Ask it something like:

> List the files in this directory, then read me the secret.

It will run `ls`, spot `secret.txt`, `cat` it, and happily tell you what it says. Which brings us to the interesting part.

## Contain it!

Do **not** run this on your laptop. The whole point of the demo is that the model can do real damage - so we put it somewhere that "real damage" means "a container we're about to throw away".

The example ships with a `Dockerfile` and a very fake `secret.txt`:

```dockerfile
# Stage 1: build the example
FROM rust:1.95-bookworm AS builder

WORKDIR /src

COPY Cargo.toml ./
COPY src ./src
RUN cargo build --release

# Stage 2: a deliberately unsafe playground
FROM debian:bookworm-slim

# bash itself, plus the CA certificates reqwest needs to reach OpenRouter over
# HTTPS. (reqwest 0.13 uses rustls, so there's no OpenSSL to install.)
RUN apt-get update \
    && apt-get install -y --no-install-recommends bash ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /src/target/release/ex11_bash_tool /usr/local/bin/ex11_bash_tool

# The directory the tool runs in, so `cat secret.txt` lands in the right place.
WORKDIR /work

# A fake secret, included purely so the live demo can read it back out.
COPY secret.txt /work/secret.txt

CMD ["ex11_bash_tool"]
```

There's a companion `.dockerignore` that keeps `target/` and, crucially, `.env` out of the image. Build from the `code/` directory:

```bash
cd code
docker build -t ex11_bash_tool ex11_bash_tool
```

Put your key in `code/ex11_bash_tool/.env` (the usual `OPENROUTER_KEY=...`), then run it with a TTY. The chat loop reads from stdin, so `-i` keeps stdin open and `-t` gives it a terminal - without them you can't type anything:

```bash
docker run -it --rm --env-file ex11_bash_tool/.env ex11_bash_tool
```

The first `docker build` compiles the crate inside the image, so it takes a minute. After that the layer is cached and rebuilds are quick.

### Getting a shell instead

The image's default command is the chatbot, so `docker run` drops you straight into the chat. If you'd rather poke around the environment the tool runs in - and show the audience `cat secret.txt` yourself - override the entrypoint:

```bash
docker run --rm -it --entrypoint bash ex11_bash_tool
```

You land in `/work`, with `secret.txt` sitting right next to you. (That's the whole point: the *tool* has the same view of the filesystem as this shell.)

If you want to drive the chatbot from inside that shell, pass the key in and run the binary by hand:

```bash
docker run --rm -it --env-file ex11_bash_tool/.env --entrypoint bash ex11_bash_tool
# at the container's shell prompt:
ex11_bash_tool
```

A few rules for the live demo:

* Never mount your home directory (`-v ~:/host`) into this container. That turns a toy into a shredder.
* Don't pass real credentials or API keys to a container that has a shell and an internet connection. Use a throwaway key.
* Don't leave it running. `--rm` means it disappears when you're done.
* The "secret" is deliberately silly. Resist the urge to use a real one, even "just for the demo".

You *can* also just run `cargo run -p ex11_bash_tool` on your own machine. That is exactly the mistake this section is warning you about. Don't.
