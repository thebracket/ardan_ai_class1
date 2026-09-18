# syntax=docker/dockerfile:1

# ---------------------------------------------------------------------------
# Stage 1: build the example
# ---------------------------------------------------------------------------
# Pinned to the Rust version the workshop uses (edition 2024 needs 1.85+). The
# full image, rather than `-slim`, already has the C toolchain and cmake that
# `aws-lc-sys` (rustls's crypto backend, pulled in by reqwest) needs to build.
FROM rust:1.95-bookworm AS builder

WORKDIR /src

COPY Cargo.toml ./
COPY src ./src
RUN cargo build --release

# ---------------------------------------------------------------------------
# Stage 2: a Linux environment for the bubblewrap sandbox
# ---------------------------------------------------------------------------
# This image exists purely so that people on macOS and Windows can run a
# Linux-only example. Inside the container the chatbot runs, and every `bash`
# tool call is sandboxed by `bwrap` - the sandbox is nested inside the
# container. See the chapter for the flags that nesting requires.
FROM debian:bookworm-slim

# `bubblewrap` is the whole point; `bash` is what the tool runs; the CA
# certificates let reqwest reach OpenRouter over HTTPS.
RUN apt-get update \
    && apt-get install -y --no-install-recommends bubblewrap bash ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Run as an unprivileged user, the same as the Docker chapter taught.
RUN useradd --create-home --uid 10001 appuser

COPY --from=builder /src/target/release/ex13_bwrap_sandbox /usr/local/bin/ex13_bwrap_sandbox

# The directory the sandbox exposes to the model as `/work`.
WORKDIR /app
COPY sandbox /app/sandbox
RUN chown -R appuser:appuser /app

USER appuser

# The API key is supplied at run time (--env-file); it is never baked into the
# image. The security options below are needed to let `bwrap` create its nested
# namespaces - they are the minimum that worked, and are much narrower than
# --privileged:
#   docker run -it --rm \
#     --env-file ex13_bwrap_sandbox/.env \
#     --security-opt systempaths=unconfined \
#     --security-opt seccomp=unconfined \
#     --security-opt apparmor=unconfined \
#     ex13_bwrap_sandbox
CMD ["ex13_bwrap_sandbox"]
