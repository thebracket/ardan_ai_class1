# Containers - Docker

In the last chapter we took the `bash` tool - something that will cheerfully run `rm -rf` for anyone who asks - and dropped it inside a Docker container. That is containment, and it is the right instinct. But a default `docker run` is not a safety switch. Out of the box, a container still runs as **root**, keeps a full set of Linux capabilities, has an open network connection, and can see everything you mounted. Get the details wrong and the container is a costume, not a cage.

So this chapter is a tour of the knobs Docker actually gives you, followed by a small example - `ex12_docker_safety` - that proves the container is doing its job.

## The Checklist

### 1. Don't run as root

By default, the process inside a container runs as `root`. Container root is not host root, of course - namespaces and cgroups keep it apart - but it is still root *in the container*, and root is exactly the account you don't want running code the model dreamed up.

Create a real unprivileged user in the `Dockerfile` and switch to it:

```dockerfile
RUN useradd --create-home --uid 10001 appuser
...
USER appuser
```

The full `USER` reference is [here](https://docs.docker.com/reference/dockerfile/#user). You can also override it at run time with `--user 10001:10001`.

Why it matters: as root, our file-reading tool can read `/etc/shadow`. As `appuser`, it gets `Permission denied (os error 13)`. Same tool, same container, one line of difference.

> One caveat: `--user root` can override the image's `USER`. A `Dockerfile` directive is a *default*, not a policy. If you need a guarantee, enforce it where the container is launched, not only in the image.

### 2. Don't mount the Docker socket

This is the big one. `/var/run/docker.sock` gives whatever holds it the right to start containers. A container started with `-v /:/host --privileged` is root on the host. Mounting the socket into your agent's container doesn't sandbox it - it hands it the keys to the building.

**Never** do this:

```bash
-v /var/run/docker.sock:/var/run/docker.sock   # don't
```

If you genuinely need a container to orchestrate other containers, look at socket proxies that filter the Docker API down to a small allow-list, and treat that proxy as a privileged component in its own right.

Our example never mounts the socket, so a tool that tries to read it just gets `No such file or directory`.

### 3. Limit (or remove) the network

The best egress control is no egress at all:

```bash
docker run --network none ...
```

That is also the awkward one, because our chatbot *needs* the network to reach OpenRouter. You cannot both call a hosted model and have no network. The honest options are:

* Call the model from **outside** the sandbox, and let only the tool-runner live in a `--network none` container. The harness holds the API key and the network; the sandbox holds neither.
* Put the container on a Docker [`internal` network](https://docs.docker.com/engine/network/) and route only the model endpoint through a forward proxy with an allow-list. Everything else is refused.
* If the model runs locally, `--network none` becomes easy again.

Docker's [network documentation](https://docs.docker.com/engine/network/) covers bridges, `internal` networks and `none`. Don't skip this axis just because it's inconvenient - "the agent can reach anything" is how data walks out of the building.

### 4. Drop capabilities

Linux capabilities split root's power into pieces. Most agent workloads need almost none of them, so start from nothing and add back only what's required:

```bash
docker run --cap-drop ALL ...
```

`--cap-add NET_BIND_SERVICE` if you truly need to bind a low port, and so on. Capabilities are defence in depth: dropping them won't stop a root process reading files (that's job one), but it does take away `mount`, raw sockets, and the other tools an escape needs.

### 5. Make the filesystem read-only

```bash
docker run --read-only --tmpfs /tmp ...
```

`--read-only` makes the container's root filesystem immutable from the inside. A tool that tries to overwrite your binary, drop a payload, or rewrite a config gets `Read-only file system`. Hand back a small writable scratch space with `--tmpfs` where the tool genuinely needs one, or mount a named volume for just the directory it works in.

### 6. Put a ceiling on resources

```bash
docker run --pids-limit 256 --memory 512m --cpus 1 ...
```

These are not security boundaries on their own, but they stop one over-eager agent from fork-bombing the machine or eating every byte of RAM while it "helps". Blast radius matters.

### 7. Keep the default security profiles

Docker ships with a default [seccomp](https://docs.docker.com/engine/security/seccomp/) profile, and on most hosts an AppArmor/SELinux policy as well. Together they block a large set of dangerous syscalls. They are on unless you turn them off - so the main rule is **don't** run `--privileged`, which disables all of it.

Add this cheap extra:

```bash
docker run --security-opt no-new-privileges ...
```

It stops a setuid binary from gaining privileges it didn't start with. There is no reason not to include it.

### 8. Consider rootless Docker

Everything above assumes the Docker daemon itself runs as root. [Rootless mode](https://docs.docker.com/engine/security/rootless/) runs the daemon as an unprivileged user, so a daemon or container escape lands as an ordinary account. It is a bigger change, but it closes off a whole class of bad day.

## Proving it: `ex12_docker_safety`

Enough theory - let's watch a container refuse to do something. We follow the usual workflow: make the crate, copy the dependencies, copy the previous example's source, then add one tool.

```bash
cd code
cargo new ex12_docker_safety
cd ex12_docker_safety
```

The dependencies are identical to `ex11_bash_tool`, so copy them straight across and change the package `name`:

```bash
cp ../ex11_bash_tool/Cargo.toml Cargo.toml   # then rename the package
cp ../ex11_bash_tool/src/chatbot.rs src/chatbot.rs
cp ../ex11_bash_tool/secret.txt secret.txt
cp ../ex11_bash_tool/.dockerignore .dockerignore
```

That gives us the same chat client and the same `bash` tool as before. Now we add the new one: a narrow `read_file` tool. It isn't a shell - it reads one file and nothing else, which is its own small lesson from the [Safety Options](tools_6.md) chapter.

In `call_tool`:

```rust
"read_file" => {
    // The arguments arrive as a JSON string, e.g. {"path":"/etc/shadow"}.
    let path = serde_json::from_str::<serde_json::Value>(arguments)
        .ok()
        .and_then(|args| {
            args.get("path")
                .and_then(|value| value.as_str())
                .map(str::to_string)
        })
        .unwrap_or_default();
    if path.is_empty() {
        return ("error".to_string(), "No path provided".to_string());
    }

    println!("📄 Reading: {path}");

    // A failed read is a *result* of the tool call, not a crash. We hand the
    // operating system's error straight back to the model and let it explain
    // the refusal to the user. Note that this is the container refusing us -
    // not our code deciding to stop.
    match tokio::fs::read_to_string(&path).await {
        Ok(contents) => ("contents".to_string(), contents),
        Err(err) => ("error".to_string(), err.to_string()),
    }
}
```

And the definition, alongside the existing tools:

```rust
ToolDefinition {
    name: "read_file".to_string(),
    description: "Reads a file and returns its contents".to_string(),
    parameters: vec![("path".to_string(), "string".to_string())],
    required: vec!["path".to_string()],
},
```

The important part is the `Err` arm. A permission error is not an exception we have to survive - it is just data. `call_tool` returns a `(key, value)` pair, the session wraps it into a tool message, and the model sees:

```json
{"error": "Permission denied (os error 13)"}
```

That is the error propagating upwards exactly as we want it to: the model reads the refusal, tells the user it couldn't read the file, and the conversation carries on. No panic, no dead loop, no stack trace on the projector. (If you want the contrast, the `bash` tool reports a failed `cat` the same way, but folded into its combined `output` - a dedicated tool gives you a clean `error` key.)

## The Dockerfile

Here is the whole thing. It is `ex11_bash_tool`'s Dockerfile with two meaningful additions: the `appuser` and the `USER` line.

```dockerfile
# syntax=docker/dockerfile:1

# ---------------------------------------------------------------------------
# Stage 1: build the example
# ---------------------------------------------------------------------------
FROM rust:1.95-bookworm AS builder

WORKDIR /src

COPY Cargo.toml ./
COPY src ./src
RUN cargo build --release

# ---------------------------------------------------------------------------
# Stage 2: the same playground, with the doors locked
# ---------------------------------------------------------------------------
FROM debian:bookworm-slim

# `bash` for the (still present) bash tool, plus the CA certificates reqwest
# needs to reach OpenRouter over HTTPS.
RUN apt-get update \
    && apt-get install -y --no-install-recommends bash ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create an unprivileged user and run as them from here on. As root this
# container can read /etc/shadow; as `appuser` it cannot.
RUN useradd --create-home --uid 10001 appuser

COPY --from=builder /src/target/release/ex12_docker_safety /usr/local/bin/ex12_docker_safety

# A world-readable file the tools *are* allowed to read - the positive half of
# the demo. This is the fake secret from ex11.
WORKDIR /work
COPY secret.txt /work/secret.txt
RUN chown -R appuser:appuser /work

# Everything below runs as `appuser`.
USER appuser

CMD ["ex12_docker_safety"]
```

Note that the API key is still supplied at run time, never baked in - and `.dockerignore` still keeps `.env` and `target/` out of the image.

## Build and run

From `code/`:

```bash
docker build -t ex12_docker_safety ex12_docker_safety
```

Then run it with the hardening flags from the checklist. Put your key in `code/ex12_docker_safety/.env` first (the usual `OPENROUTER_KEY=...`):

```bash
docker run -it --rm \
  --env-file ex12_docker_safety/.env \
  --read-only --tmpfs /tmp \
  --cap-drop ALL \
  --security-opt no-new-privileges \
  ex12_docker_safety
```

There's no `--network none` here on purpose - the chatbot still has to talk to OpenRouter. See axis three above for the ways around that.

## What to ask it

Try these in order:

```text
Read /work/secret.txt
Read /etc/shadow
Read /var/run/docker.sock
Try to delete /work/secret.txt with bash
```

Roughly what you should see:

* `/work/secret.txt` reads fine. It's world-readable and owned by `appuser`.
* `/etc/shadow` comes back as `Permission denied (os error 13)` - we are not root.
* `/var/run/docker.sock` comes back as `No such file or directory` - we never mounted it.
* The delete fails with `Read-only file system`. `--read-only` is doing the work here; the tool reports the refusal rather than the harness falling over.

The model will sometimes reach for `bash` instead of `read_file`. That's fine - the container stops it either way, which is the whole point. The container is the boundary; the prompt is not.

## References

* [Docker security](https://docs.docker.com/engine/security/) - the overview, and where the daemon-level advice lives.
* [`docker run` reference](https://docs.docker.com/reference/cli/docker/container/run/) - every flag used above.
* [`Dockerfile` reference](https://docs.docker.com/reference/dockerfile/) - `USER`, `COPY --chown`, and friends.
* [Docker networking](https://docs.docker.com/engine/network/) - bridges, `internal`, and `none`.
* [seccomp profiles](https://docs.docker.com/engine/security/seccomp/).
* [Rootless mode](https://docs.docker.com/engine/security/rootless/).
* [OWASP Docker Security Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Docker_Security_Cheat_Sheet.html) - a compact checklist if you want someone else's version of this chapter.
