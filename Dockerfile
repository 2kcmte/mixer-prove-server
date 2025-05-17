FROM ubuntu:22.04

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y \
    curl \
    build-essential \
    pkg-config \
    libssl-dev \
    git \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN curl -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

ENV RUST_TOOLCHAIN=nightly-2024-05-20   
RUN rustup set profile minimal \
    && rustup toolchain install ${RUST_TOOLCHAIN} \
    && rustup default ${RUST_TOOLCHAIN} \
    && rustup component add rust-src --toolchain ${RUST_TOOLCHAIN} \
    && rustup toolchain uninstall stable

RUN curl -L https://sp1up.succinct.xyz | bash
ENV PATH="/root/.sp1/bin:${PATH}"
RUN sp1up

WORKDIR /app

COPY . .

WORKDIR /app/program
RUN cargo prove build

RUN mkdir -p /app/script/
RUN cp target/elf-compilation/riscv32im-succinct-zkvm-elf/release/mixer-program /app/script/mixer-proof.elf || \
    echo "ELF file not found with expected name. Trying alternative names..." && \
    find target/elf-compilation/riscv32im-succinct-zkvm-elf/release/ -type f -executable -exec cp {} /app/script/mixer-proof.elf \; || \
    echo "No ELF file found, but continuing"

WORKDIR /app/script

RUN cargo build --release

RUN cargo metadata --format-version 1

RUN ls -la /app/target/release/ || echo "/app/target/release/ does not exist"

EXPOSE 3001

ENV RUST_LOG=info
CMD ["/app/target/release/mixer"]