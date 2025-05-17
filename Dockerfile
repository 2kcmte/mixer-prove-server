
FROM ubuntu:22.04 AS builder

ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y \
    curl build-essential pkg-config libssl-dev git ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN curl -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"


ENV RUST_TOOLCHAIN=nightly-2024-05-20
RUN rustup set profile minimal \
    && rustup toolchain install ${RUST_TOOLCHAIN} \
    && rustup default ${RUST_TOOLCHAIN} \
    && rustup component add rust-src --toolchain ${RUST_TOOLCHAIN} \
    && rustup toolchain uninstall stable



RUN curl -sL https://sp1up.succinct.xyz | bash


RUN /root/.sp1/bin/sp1up
ENV PATH="/root/.sp1/bin:${PATH}"


WORKDIR /app
COPY . .


WORKDIR /app/program
RUN cargo prove build


RUN mkdir -p /app/script \
    && cp target/elf-compilation/riscv32im-succinct-zkvm-elf/release/mixer-program \
    /app/script/mixer-proof.elf \
    || true


WORKDIR /app/script
RUN cargo build --release


FROM ubuntu:22.04

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y \
    docker.io ca-certificates \
    && rm -rf /var/lib/apt/lists/*


COPY --from=builder /app/script/target/release/mixer /usr/local/bin/mixer
COPY --from=builder /app/script/mixer-proof.elf       /usr/local/bin/mixer-proof.elf

EXPOSE 3001
ENV PORT=3001
ENV RUST_LOG=info

ENTRYPOINT ["mixer"]
