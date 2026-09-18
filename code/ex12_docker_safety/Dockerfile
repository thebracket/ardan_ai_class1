# syntax=docker/dockerfile:1

# ---------------------------------------------------------------------------
# Stage 1: build the example
# ---------------------------------------------------------------------------
# Pinned to the Rust version the workshop uses (edition 2024 needs 1.85+). The
# full image, rather than `-slim`, already has the C toolchain and cmake that
# `aws-lc-sys` (rustls's crypto backend, pulled in by reqwest) needs to build.
FROM rust:1.95-bookworm AS builder

WORKDIR /src

# Copying only what we need keeps the context small and, thanks to
# .dockerignore, keeps secrets and build output out of the image.
COPY Cargo.toml ./
COPY src ./src
RUN cargo build --release

# ---------------------------------------------------------------------------
# Stage 2: the same playground, with the doors locked
# ---------------------------------------------------------------------------
# Compare this with ex11_bash_tool. The tool-calling code is almost identical;
# what changed is what the container will let it do.
FROM debian:bookworm-slim

# `bash` for the (still present) bash tool, plus the CA certificates reqwest
# needs to reach OpenRouter over HTTPS.
RUN apt-get update \
    && apt-get install -y --no-install-recommends bash ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create an unprivileged user and run as them from here on. As root this
# container can read /etc/shadow; as `appuser` it cannot. That one line is the
# difference between a demo and a liability.
RUN useradd --create-home --uid 10001 appuser

COPY --from=builder /src/target/release/ex12_docker_safety /usr/local/bin/ex12_docker_safety

# A world-readable file the tools *are* allowed to read - the positive half of
# the demo. This is the fake secret from ex11.
WORKDIR /work
COPY secret.txt /work/secret.txt
RUN chown -R appuser:appuser /work

# Everything below runs as `appuser`.
USER appuser

# The API key is supplied at run time (--env-file); it is never baked into the
# image. Run with the hardening flags from the chapter, and never mount the
# Docker socket:
#   docker run -it --rm \
#     --env-file ex12_docker_safety/.env \
#     --read-only --tmpfs /tmp \
#     --cap-drop ALL \
#     --security-opt no-new-privileges \
#     ex12_docker_safety
CMD ["ex12_docker_safety"]
