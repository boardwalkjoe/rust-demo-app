# ── Stage 1: Build ──────────────────────────────────────────
FROM rust:1.83-slim AS builder

RUN apt-get update && \
    apt-get install -y musl-tools pkg-config && \
    rustup target add x86_64-unknown-linux-musl && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Cache dependency builds
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo 'fn main(){}' > src/main.rs && \
    cargo build --release --target x86_64-unknown-linux-musl && \
    rm -rf src

# Build the real application
COPY src/ ./src/
RUN touch src/main.rs && \
    cargo build --release --target x86_64-unknown-linux-musl

# ── Stage 2: Minimal runtime ───────────────────────────────
FROM scratch

# TLS certificates (needed if your app makes HTTPS calls)
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/

# The static binary
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/openshift-rustdemo /app

# OpenShift assigns a random UID — this is just a signal
USER 1001

EXPOSE 8080

ENV PORT=8080

ENTRYPOINT ["/app"]
