# syntax=docker/dockerfile:1

# ---------------------------------------------------------------------------
# Stage 1: build the example
# ---------------------------------------------------------------------------
# Pinned to the Rust version the workshop uses (edition 2024 needs 1.85+). The
# full image, rather than `-slim`, already has the C toolchain and cmake that
# `aws-lc-sys` (rustls's crypto backend, pulled in by reqwest) needs to build.
FROM rust:1.95-bookworm AS builder

WORKDIR /src

# Build the crate on its own. Copying only what we need keeps the context small
# and, thanks to .dockerignore, keeps secrets and build output out of the image.
COPY Cargo.toml ./
COPY src ./src
RUN cargo build --release

# ---------------------------------------------------------------------------
# Stage 2: a deliberately unsafe playground
# ---------------------------------------------------------------------------
# This image hands the model a real bash shell. Run it in a container, on a
# throwaway machine, and never with your real credentials.
FROM debian:bookworm-slim

# `bash` itself, plus the CA certificates `reqwest` needs to reach OpenRouter
# over HTTPS. (reqwest 0.13 uses rustls, so there's no OpenSSL to install.)
RUN apt-get update \
    && apt-get install -y --no-install-recommends bash ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /src/target/release/ex11_bash_tool /usr/local/bin/ex11_bash_tool

# The directory the tool runs in, so `cat secret.txt` lands in the right place.
WORKDIR /work

# A fake secret, included purely so the live demo can read it back out again.
COPY secret.txt /work/secret.txt

# The API key is supplied at run time (--env-file); it is never baked into the
# image. The container needs a TTY for the interactive chat loop:
#   docker run -it --rm --env-file ex11_bash_tool/.env ex11_bash_tool
# Or drop into a shell in the same working directory instead:
#   docker run --rm -it --entrypoint bash ex11_bash_tool
CMD ["ex11_bash_tool"]
