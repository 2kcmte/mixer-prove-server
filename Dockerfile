# ───────────────────────────────────────────────────────────────
# 1) Builder stage: build the ELF & the server binary
# ───────────────────────────────────────────────────────────────
FROM ubuntu:22.04 AS builder

ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y \
    curl build-essential pkg-config libssl-dev git ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Install Rust
RUN curl -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Pin nightly so rustup doesn’t auto-update later
ENV RUST_TOOLCHAIN=nightly-2024-05-20
RUN rustup set profile minimal \
    && rustup toolchain install ${RUST_TOOLCHAIN} \
    && rustup default ${RUST_TOOLCHAIN} \
    && rustup component add rust-src --toolchain ${RUST_TOOLCHAIN} \
    && rustup toolchain uninstall stable


# Install the sp1up script
RUN curl -sL https://sp1up.succinct.xyz | bash

# Make sure the newly-installed sp1up is in PATH *and* immediately usable.
# We invoke it via its absolute path to avoid any shell-init magic.
RUN /root/.sp1/bin/sp1up
ENV PATH="/root/.sp1/bin:${PATH}"


WORKDIR /app
COPY . .

# Build the ZK program (ELF)
WORKDIR /app/program
RUN cargo prove build

# Copy ELF into a known location
RUN mkdir -p /app/script \
    && cp target/elf-compilation/riscv32im-succinct-zkvm-elf/release/mixer-program \
    /app/script/mixer-proof.elf \
    || true

# Build the HTTP/WebSocket server
WORKDIR /app/script
RUN cargo build --release

# ───────────────────────────────────────────────────────────────
# 2) Runtime stage: only Docker client + your server binary
# ───────────────────────────────────────────────────────────────
FROM ubuntu:22.04

ENV DEBIAN_FRONTEND=noninteractive
# Install Docker **client** and CA certs
RUN apt-get update && apt-get install -y \
    docker.io ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy your compiled server & ELF
COPY --from=builder /app/script/target/release/mixer /usr/local/bin/mixer
COPY --from=builder /app/script/mixer-proof.elf       /usr/local/bin/mixer-proof.elf

EXPOSE 3001
ENV PORT=3001
ENV RUST_LOG=info

ENTRYPOINT ["mixer"]
