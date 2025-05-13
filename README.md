# Prove Server

A standalone HTTP/WebSocket service that generates zero‐knowledge proofs for Solana Mixer withdrawals using Succinct SP1's Groth16 backend. Clients send their note (nullifier + secret), RPC URL, and mixer program ID; the server replays deposit events to build a Merkle proof, then calls the SP1 prover to create a zk-SNARK, and finally returns the raw proof_bytes and public_inputs needed for on-chain verification.

## Table of Contents

- [Features](#features)
- [Prerequisites](#prerequisites)
- [Installation](#installation)
- [Configuration](#configuration)
- [Running the Server](#running-the-server)
- [API Endpoints](#api-endpoints)
  - [WebSocket /ws/compute_withdrawal](#websocket-wscompute_withdrawal)
- [Example Usage](#example-usage)
- [Internals](#internals)
- [SP1 Prover Network](#sp1-prover-network)
- [Troubleshooting](#troubleshooting)

## Features

- Merkle‐proof replay: Fetches all DepositEvent logs from Solana, rebuilds the tree, and extracts siblings & path bits for any leaf index.
- SP1-powered proving: Uses Succinct SP1's prover network (or local SP1 binary) to generate Groth16 proofs for Solana Mixer circuits.
- Dual API:
  - HTTP for simple REST clients
  - WebSocket for long‐running proof jobs with streaming progress & timeouts

## Prerequisites

- Rust ≥1.70
- Solana CLI & local validator (for local development)
- Network access to the target Solana RPC (Devnet, Testnet, Mainnet)
- Optional: Local SP1-zkVM prover (if running proofs locally)

## Installation

```bash
git clone https://github.com/your-org/solana-mixer-prove-server.git
cd solana-mixer-prove-server
cargo build --release
```

## Configuration

Copy `.env.example` to `.env` and adjust as needed:

```bash
# Required for SP1 prover network
NETWORK_PRIVATE_KEY_SP1= "your_private_key"  
```

## Running the Server

Using environment variables from `.env`:

```bash
cargo run --release
```

This will start:
- HTTP server on `http://0.0.0.0:<HTTP_PORT>`
- WebSocket server on `ws://0.0.0.0:<WS_PORT>/ws/compute_withdrawal`
- hard coded to `3001`

## API Endpoints

### WebSocket /ws/compute_withdrawal

1. Client opens a WebSocket to `ws://<host>:<WS_PORT>/ws/compute_withdrawal`
2. Client sends the same JSON payload as the HTTP `/api/withdraw` endpoint
3. Server streams back either:
   - `{ "proof_bytes": [...], "public_inputs": [...] }` on success
   - `{ "error": "..." }` on failure
4. Connection closes automatically after success or timeout (~20 minutes default)

## Example Usage

### HTTP (curl)

```bash
curl -X POST http://localhost:3001/api/withdraw \
  -H "Content-Type: application/json" \
  -d '{
    "nullifier":"123456...",
    "secret":"abcdef...",
    "rpc_url":"http://127.0.0.1:8899",
    "program_pubkey":"B7odahygLXdwCYmJteVyBFXXe9qEW5hyvCXieRGBoTTz",
    "new_withdrawal_recipient_address":"FK3...",
    "new_relayer_address":"G9h..."
  }'
```

### WebSocket (JavaScript)

```javascript
const socket = new WebSocket("ws://localhost:3000/ws/compute_withdrawal");
socket.onopen = () => {
  socket.send(JSON.stringify({ /* same fields as above */ }));
};
socket.onmessage = (evt) => {
  const data = JSON.parse(evt.data);
  if (data.error) console.error("Proof failed:", data.error);
  else {
    console.log("Proof bytes:", data.proof_bytes);
    console.log("Public inputs:", data.public_inputs);
  }
};
```

## Internals

1. **fetch_deposits**
   - Uses Solana RPC to scan all DepositEvent logs for your program
   - Decodes each event's base64‐encoded data via Borsh into (leafIndex, commitment)

2. **Merkle Proof Builder**
   - Replays deposits into a full 2^20‐sized Merkle tree (padding with on-chain zero‐hash constants)
   - Extracts sibling-hash array and index bits for the target leaf

3. **SP1 Prover**
   - Sends the nullifier, secret, Merkle siblings & indices to SP1's prover (via HTTP/WebSocket or local CLI)
   - Receives a Groth16 proof + public input buffer

4. **Result**
   - Returns proof bytes and public inputs ready to pass into your Anchor withdraw(...) call

## SP1 Prover Network

The server uses Succinct's SP1 prover network by default for generating proofs. This is because generating proofs locally is computationally intensive. However, there are important considerations:

1. **Cost:** Each proof generation costs 0.5 credits (0.5$ of your SP1 account balance)
2. **Network Dependencies:** The service requires an active SP1 prover network connection
3. **Local Alternative:** You can switch to local proof generation by modifying the `prove_mix` function:

```rust
// Instead of using the network client:
let client = ProverClient::builder()
    .network()
    .private_key(&sp1_private_key)
    .rpc_url(sp1_rpc_url)
    .build();

// Use the local CPU client:
let client = ProverClient::builder()
    .cpu()
    .build();
```

## Troubleshooting

Common issues that may cause the service to stop working:

1. **SP1 Prover Network Credits**
   - If your SP1 account runs out of credits, proof generation will fail
   - Solution: Refill your SP1 account credits or switch to local proof generation
   - To refill credits, visit the SP1 dashboard and add funds to your account

2. **Network Connectivity**
   - Ensure stable internet connection to the SP1 prover network
   - Check if the SP1 prover network is experiencing downtime

3. **Local Proof Generation**
   - If using local proof generation, ensure sufficient CPU resources
   - Local proving is significantly slower but doesn't incur network costs
   - Consider using a high-performance machine for local proving

4. **Memory Usage**
   - Monitor system memory when running local proofs
   - Proof generation can be memory-intensive
   - Consider increasing swap space if running into memory issues

If you have issues, feel free to create an issue on this repository.

