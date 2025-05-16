# Base image: Ubuntu with required tools
FROM ubuntu:22.04

# Set non-interactive installation
ENV DEBIAN_FRONTEND=noninteractive

# Install build dependencies
RUN apt-get update && apt-get install -y \
    curl \
    build-essential \
    pkg-config \
    libssl-dev \
    git \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*



# ---- Rust ------------------------------------------------------------------
RUN curl -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# ── NEW: pin toolchain & avoid future updates ───────────────────────────────
ENV RUST_TOOLCHAIN=nightly-2024-05-20   
RUN rustup set profile minimal \
    && rustup toolchain install ${RUST_TOOLCHAIN} \
    && rustup default ${RUST_TOOLCHAIN} \
    && rustup component add rust-src --toolchain ${RUST_TOOLCHAIN} \
    && rustup toolchain uninstall stable        # <── nothing left to “update”



# Install SP1 using sp1up
RUN curl -L https://sp1up.succinct.xyz | bash
ENV PATH="/root/.sp1/bin:${PATH}"
RUN sp1up

# Create and set working directory
WORKDIR /app

# Copy the entire project
COPY . .

# Build the SP1 program first
WORKDIR /app/program
RUN cargo prove build

# Copy the ELF file to the script directory
RUN mkdir -p /app/script/
RUN cp target/elf-compilation/riscv32im-succinct-zkvm-elf/release/mixer-program /app/script/mixer-proof.elf || \
    echo "ELF file not found with expected name. Trying alternative names..." && \
    find target/elf-compilation/riscv32im-succinct-zkvm-elf/release/ -type f -executable -exec cp {} /app/script/mixer-proof.elf \; || \
    echo "No ELF file found, but continuing"

# Set up the working directory for the server
WORKDIR /app/script

# Build the server in release mode
RUN cargo build --release

# Debug: Print Cargo metadata to check target directory
RUN cargo metadata --format-version 1

# Debug: Check if the binary exists in potential target directory
RUN ls -la /app/target/release/ || echo "/app/target/release/ does not exist"


# Install Docker client & CA certs
RUN apt-get update \
    && apt-get install -y --no-install-recommends docker.io ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy your built server binary
COPY --from=builder /app/script/target/release/mixer /usr/local/bin/mixer


# Expose server port
EXPOSE 3001

# Set logging
ENV RUST_LOG=info

# Run the server (assuming target directory is /app/target)
CMD ["/app/target/release/mixer"]