use anchor_client::solana_sdk::pubkey::Pubkey;
use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::Json,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use hex::{encode as hex_encode, FromHex};
use num_bigint::BigUint;
use regex::Regex;
use serde::{Deserialize, Serialize};
use sp1_sdk::{include_elf, HashableKey, Prover, ProverClient, SP1PublicValues, SP1Stdin};
mod utils;
use dotenv;
use serde_json::json;
use solana_poseidon::{hashv, Endianness, Parameters};
use std::str::FromStr;
use tokio::net::TcpListener;
use utils::*;

pub const MIXER_ELF: &[u8] = include_elf!("mixer-program");
const MERKLE_LEVELS: usize = 20;

#[derive(Deserialize, Serialize, Debug)]
pub struct ProveRequest {
    // ─── Public inputs ─────
    pub root: String,           // hex, 32 bytes
    pub nullifier_hash: String, // hex, 32 bytes
    pub recipient: String,      // hex, 32 bytes (Solana Pubkey)
    pub relayer: String,        // hex, 32 bytes
    pub fee: u64,
    pub refund: u64,
    // ─── Private inputs ──────
    pub nullifier: String, // hex, 32 bytes
    pub secret: String,    // hex, 32 bytes
    // ─── Merkle path ────────
    pub path_elements: Vec<String>, // each hex, 32 bytes
    pub path_indices: Vec<u8>,      // each 0 or 1
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProveResponse {
    pub proof: String, // hex-encoded Groth16 proof
    pub public_inputs: SP1PublicValues,
}

#[derive(Serialize, Deserialize, Debug)]
struct ProveResponseCustom {
    proof: String,
    public_inputs: PublicInputsWrapper,
}

#[derive(Serialize, Deserialize, Debug)]
struct PublicInputsWrapper {
    buffer: BufferData,
}

#[derive(Serialize, Deserialize, Debug)]
struct BufferData {
    data: Vec<u8>,
}

async fn prove_mix(Json(req): Json<ProveRequest>) -> impl IntoResponse {
    // Parse & validate all the hex inputs
    macro_rules! hex32 {

    ([ $($byte:expr),* ]) => {{

        let arr: [u8;32] = [ $($byte),* ];
        arr
    }};
    ($s:expr) => {{
        let s: &str = &$s;
        let s = s.strip_prefix("0x").unwrap_or(s);
        let vec = Vec::from_hex(s)
            .map_err(|e| (StatusCode::BAD_REQUEST, format!("invalid hex “{}”: {}", s, e)))?;
        if vec.len() != 32 {
            return Err((StatusCode::BAD_REQUEST, format!("hex length != 32: “{}”", s)));
        }
        let mut arr = [0u8;32];
        arr.copy_from_slice(&vec);
        arr
    }};
}

    let root_arr = hex32!(req.root);
    let null_hash_arr = hex32!(req.nullifier_hash);
    let recipient_arr = hex32!(req.recipient);
    let relayer_arr = hex32!(req.relayer);
    let nullifier_arr = hex32!(req.nullifier);
    let secret_arr = hex32!(req.secret);

    eprintln!(
        "Decoded value: {:?}",
        [
            root_arr,
            null_hash_arr,
            recipient_arr,
            relayer_arr,
            nullifier_arr,
            secret_arr
        ]
    );

    if req.path_elements.len() != MERKLE_LEVELS || req.path_indices.len() != MERKLE_LEVELS {
        return Err((
            StatusCode::BAD_REQUEST,
            format!(
                "Expected {} path elements & indices, got {} elems and {} idxs",
                MERKLE_LEVELS,
                req.path_elements.len(),
                req.path_indices.len()
            ),
        ));
    }
    let mut path_elems = [[0u8; 32]; MERKLE_LEVELS];
    for (i, hexstr) in req.path_elements.iter().enumerate() {
        path_elems[i] = hex32!(hexstr);
    }
    let path_idxs = req.path_indices.clone();

    sp1_sdk::utils::setup_logger();
    dotenv::dotenv().ok();

    let sp1_private_key_env = std::env::var("NETWORK_PRIVATE_KEY_SP1");
    let sp1_private_key = sp1_private_key_env.unwrap().to_string();

    let sp1_rpc_url = "https://rpc.production.succinct.xyz";

    let client = ProverClient::builder()
        .network()
        .private_key(&sp1_private_key)
        .rpc_url(sp1_rpc_url)
        .build();

    let mut stdin = SP1Stdin::new();

    // Write inputs in the exact order the circuit reads them:
    stdin.write(&root_arr);
    stdin.write(&null_hash_arr);
    stdin.write(&recipient_arr);
    stdin.write(&relayer_arr);
    stdin.write(&req.fee);
    stdin.write(&req.refund);

    stdin.write(&nullifier_arr);
    stdin.write(&secret_arr);

    let elem: [[u8; 32]; MERKLE_LEVELS] = path_elems;
    stdin.write(&elem);

    let idx_arr: [u8; MERKLE_LEVELS] = path_idxs.try_into().expect("wrong number of indices");
    stdin.write(&idx_arr);

    let (pk, vk) = client.setup(MIXER_ELF);
    let proof = match client.prove(&pk, &stdin).groth16().run() {
        Ok(p) => p,
        Err(e) => {
            let msg = format!("❌ proof generation failed: {}", e);
            return Err((StatusCode::INTERNAL_SERVER_ERROR, msg));
        }
    };

    if let Err(e) = client.verify(&proof, &vk) {
        let msg = format!("❌ proof verification failed: {}", e);
        return Err((StatusCode::INTERNAL_SERVER_ERROR, msg));
    }

    let proof_hex = hex_encode(proof.bytes());
    Ok((
        StatusCode::OK,
        Json(ProveResponse {
            proof: proof_hex,
            public_inputs: proof.public_values,
        }),
    ))
}

fn notmain() {
    let prover = ProverClient::builder().cpu().build();
    let (_, vk) = prover.setup(MIXER_ELF);
    eprintln!("VK key {}", vk.bytes32());
}

#[derive(Serialize)]
pub struct DepositDetails {
    pub nullifier: String,
    pub secret: String,
    pub note: String,
    pub commitment: [u8; 32],
}

#[derive(Deserialize)]
pub struct GenerateDepositDetailsRequest {
    pub amount: f64,
}

async fn generate_deposit_details(
    Json(req): Json<GenerateDepositDetailsRequest>,
) -> (StatusCode, Json<DepositDetails>) {
    let (nullifier, secret, preimage, commitment, nullifier_hash) =
        mixer_lib::utils::create_random_commitment();

    // build the note string as: solana-mixer-1-<nullifierHex>:<secretHex>
    let note = format!(
        "solana-mixer-{}-{}:{}",
        req.amount,
        hex::encode(nullifier.to_bytes_le()),
        hex::encode(secret.to_bytes_le())
    );

    println!("note = {}", note);

    (
        StatusCode::OK,
        Json(DepositDetails {
            nullifier: hex::encode(nullifier.to_bytes_le()),
            secret: hex::encode(secret.to_bytes_le()),
            note,
            commitment,
        }),
    )
}

#[derive(Deserialize)]
pub struct DecodeNoteDetailsRequest {
    pub note: String,
    pub program_pubkey: String,
}

#[derive(Serialize)]
pub struct DecodeNoteDetailsResponse {
    pub nullifier_str: String,
    pub secret_str: String,
    pub amount: f64,
    pub state_pubkey: String,
}

async fn decode_note_details(Json(req): Json<DecodeNoteDetailsRequest>) -> impl IntoResponse {
    let re = Regex::new(
        r"^solana-mixer-(?P<amount>\d+(?:\.\d+)?)-(?P<nullifier>[0-9A-Fa-f]+):(?P<secret>[0-9A-Fa-f]+)$"
    ).unwrap();

    let caps = match re.captures(&req.note) {
        Some(c) => c,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(DecodeNoteDetailsResponse {
                    nullifier_str: "".to_string(),
                    secret_str: "".to_string(),
                    amount: 0.0,
                    state_pubkey: "".to_string(),
                }),
            );
        }
    };

    let amt: f64 = caps.name("amount").unwrap().as_str().parse().unwrap();

    let nullifier_bytes = hex::decode(caps.name("nullifier").unwrap().as_str()).unwrap();
    let secret_bytes = hex::decode(caps.name("secret").unwrap().as_str()).unwrap();

    let nullifier_bn = BigUint::from_bytes_le(&nullifier_bytes);
    let secret_bn = BigUint::from_bytes_le(&secret_bytes);

    let nullifier_str = nullifier_bn.to_string();
    let secret_str = secret_bn.to_string();

    let state_pubkey = utils::get_pubkeys_utils(req.program_pubkey);

    (
        StatusCode::OK,
        Json(DecodeNoteDetailsResponse {
            nullifier_str,
            secret_str,
            amount: amt,
            state_pubkey: state_pubkey.to_string(),
        }),
    )
}

async fn get_pubkeys(program_pubkey: String) -> String {
    utils::get_pubkeys_utils(program_pubkey)
}

async fn ws_compute_proof_withdrawal(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_ws)
}
#[derive(Deserialize, Debug)]
pub struct WithdrawalComputeRequest {
    pub nullifier: String,
    pub secret: String,
    pub rpc_url: String,
    pub program_pubkey: String,
    pub new_withdrawal_recipient_address: String,
    pub new_relayer_address: String,
    pub server_url: String,
}

async fn handle_ws(mut socket: WebSocket) {
    let msg = match socket.recv().await {
        Some(Ok(Message::Text(txt))) => txt,
        _ => return,
    };
    let mut req: WithdrawalComputeRequest = match serde_json::from_str(&msg) {
        Ok(r) => r,
        Err(e) => {
            let _ = socket
                .send(Message::Text(
                    json!({
                        "error": format!("Invalid request JSON: {}", e)
                    })
                    .to_string(),
                ))
                .await;
            return;
        }
    };
    if req.server_url.is_empty() {
        req.server_url = "http://localhost:3001".to_string();
    }
    let nullifier_bn = BigUint::from_str(&req.nullifier).unwrap();
    let secret_bn = BigUint::from_str(&req.secret).unwrap();
    let nullifier_bytes = nullifier_bn.to_bytes_le();
    let secret_bytes = secret_bn.to_bytes_le();
    let commitment = hashv(
        Parameters::Bn254X5,
        Endianness::LittleEndian,
        &[&nullifier_bytes, &secret_bytes],
    )
    .unwrap()
    .to_bytes();

    let nullifier_hash = hashv(
        Parameters::Bn254X5,
        Endianness::LittleEndian,
        &[&nullifier_bytes],
    )
    .unwrap()
    .to_bytes();

    println!(
        "commitment: {:?}\nnullifier_hash: {:?}",
        commitment, nullifier_hash
    );

    let (all_commits, _entries, _indices, leaf_index, found) =
        match utils::fetch_deposits(commitment, &req.rpc_url, &req.program_pubkey) {
            Ok(r) => r,
            Err(e) => {
                let _ = socket
                    .send(Message::Text(
                        json!({
                            "error": format!("fetch_deposits failed: {}", e)
                        })
                        .to_string(),
                    ))
                    .await;
                return;
            }
        };

    if !found {
        let _ = socket
            .send(Message::Text(
                json!({
                    "error": "Commitment not found in on-chain history"
                })
                .to_string(),
            ))
            .await;
        return;
    }

    let (siblings, path_indices, root) =
        utils::compute_merkle_proof::<20>(&all_commits, leaf_index);

    let siblings_array: [[u8; 32]; 20] = siblings.clone().try_into().unwrap();
    let path_indices_array: [u8; 20] = path_indices.clone().try_into().unwrap();
    utils::merkle_check::<20>(root, commitment, &siblings_array, &path_indices_array);
    println!("Root: {:?}", root);
    let root: [u8; 32] = root;
    let nullifier_hash: [u8; 32] = nullifier_hash;
    let recipient: [u8; 32] = Pubkey::from_str(&req.new_withdrawal_recipient_address)
        .unwrap()
        .to_bytes();
    let relayer: [u8; 32] = Pubkey::from_str(&req.new_relayer_address)
        .unwrap()
        .to_bytes();

    let fee = 0;
    let refund = 0;
    let nullifier: BigUint = nullifier_bn;
    let secret: BigUint = secret_bn;
    let path_elems: Vec<[u8; 32]> = siblings.to_vec();
    let path_inds: Vec<u8> = path_indices.to_vec();

    let prove_req = ProveRequest {
        root: to_hex32(&root),
        nullifier_hash: to_hex32(&nullifier_hash),
        recipient: to_hex32(&recipient),
        relayer: to_hex32(&relayer),
        fee,
        refund,
        nullifier: to_hex32(&biguint_to_32_le_bytes(&nullifier)),
        secret: to_hex32(&biguint_to_32_le_bytes(&secret)),
        path_elements: to_hex_vec(&path_elems),
        path_indices: path_inds,
    };

    let client = reqwest::Client::new();
    let resp = match client
        .post(format!("{}/api/prove-mix", req.server_url))
        .json(&prove_req)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            let _ = socket
                .send(Message::Text(
                    json!({
                        "error": format!("Prover HTTP request failed: {}", e)
                    })
                    .to_string(),
                ))
                .await;
            return;
        }
    };
    let resp_text = match resp.text().await {
        Ok(t) => t,
        Err(e) => {
            let _ = socket
                .send(Message::Text(
                    json!({
                        "error": format!("Reading prover response failed: {}", e)
                    })
                    .to_string(),
                ))
                .await;
            return;
        }
    };

    let prove_resp: ProveResponseCustom = match serde_json::from_str(&resp_text) {
        Ok(r) => r,
        Err(e) => {
            let _ = socket
                .send(Message::Text(
                    json!({
                        "error": format!("Invalid prover response JSON: {}", e)
                    })
                    .to_string(),
                ))
                .await;
            return;
        }
    };

    let proof_bytes = match hex::decode(&prove_resp.proof) {
        Ok(b) => b,
        Err(e) => {
            let _ = socket
                .send(Message::Text(
                    json!({
                        "error": format!("Invalid proof hex: {}", e)
                    })
                    .to_string(),
                ))
                .await;
            return;
        }
    };
    let public_inputs = prove_resp.public_inputs.buffer.data;

    let result = json!({
        "proof_bytes": &proof_bytes,
        "public_inputs": public_inputs,
    });
    let _ = socket.send(Message::Text(result.to_string())).await;

    let _ = socket.close().await;
}

#[tokio::main]
async fn main() {
    let cors = tower_http::cors::CorsLayer::new()
        .allow_origin(tower_http::cors::Any)
        .allow_methods(tower_http::cors::Any)
        .allow_headers(tower_http::cors::Any);

    let app = Router::new()
        .route("/api/prove-mix", post(prove_mix))
        .route(
            "/api/generate-deposit-details",
            post(generate_deposit_details),
        )
        .route("/api/decode-note-details", post(decode_note_details))
        .route("/api/get-pubkeys", get(get_pubkeys))
        .route("/ws/compute_withdrawal", get(ws_compute_proof_withdrawal))
        .layer(cors);

    notmain();
    let listener = TcpListener::bind("0.0.0.0:3001").await.unwrap();
    println!("Starting proof API server on 0.0.0.0:3001...");
    axum::serve(listener, app).await.unwrap();
}
