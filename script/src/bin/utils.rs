use anchor_client::{
    anchor_lang::{AnchorDeserialize, Discriminator},
    solana_client::{
        rpc_client::{GetConfirmedSignaturesForAddress2Config, RpcClient},
        rpc_config::RpcTransactionConfig,
    },
    solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey, signature::Signature},
};
use ark_bn254::Fr;
use base64::Engine;
use light_poseidon::{Poseidon, PoseidonBytesHasher};
use num_bigint::BigUint;

use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use solana_poseidon::{hashv, Endianness, Parameters};
use solana_transaction_status::{option_serializer::OptionSerializer, UiTransactionEncoding};
use std::{error::Error, str::FromStr};

pub const TREE_DEPTH: usize = 20;

pub const ZERO_HASHES: [[u8; 32]; TREE_DEPTH] = [
    [
        28, 225, 101, 203, 17, 36, 237, 58, 10, 148, 180, 226, 18, 170, 247, 232, 7, 159, 73, 178,
        251, 239, 145, 107, 194, 144, 197, 147, 253, 169, 9, 42,
    ],
    [
        44, 37, 33, 144, 170, 41, 196, 255, 177, 217, 13, 209, 96, 22, 63, 31, 189, 226, 225, 107,
        60, 59, 217, 73, 104, 85, 87, 161, 98, 46, 25, 23,
    ],
    [
        199, 192, 139, 62, 101, 110, 114, 88, 10, 223, 158, 90, 92, 156, 242, 121, 110, 186, 213,
        73, 160, 199, 139, 93, 59, 126, 247, 199, 180, 171, 213, 4,
    ],
    [
        195, 14, 203, 76, 69, 4, 58, 41, 155, 50, 70, 4, 20, 111, 183, 33, 118, 162, 254, 210, 250,
        13, 199, 140, 212, 199, 234, 11, 169, 89, 165, 14,
    ],
    [
        213, 209, 90, 217, 231, 76, 215, 254, 77, 109, 54, 56, 172, 83, 223, 190, 193, 157, 101,
        68, 174, 242, 152, 39, 120, 128, 239, 49, 155, 47, 245, 38,
    ],
    [
        240, 84, 3, 71, 171, 53, 201, 28, 190, 149, 211, 115, 45, 246, 189, 74, 50, 130, 179, 241,
        13, 241, 220, 214, 84, 86, 24, 240, 92, 124, 162, 47,
    ],
    [
        241, 195, 14, 235, 69, 82, 145, 169, 122, 158, 38, 203, 218, 80, 135, 166, 104, 169, 105,
        163, 220, 45, 188, 80, 35, 38, 28, 98, 57, 139, 192, 1,
    ],
    [
        56, 192, 212, 159, 83, 188, 37, 134, 9, 245, 223, 94, 83, 72, 113, 241, 166, 202, 248, 76,
        6, 24, 24, 181, 13, 5, 248, 85, 163, 179, 57, 42,
    ],
    [
        151, 168, 4, 21, 164, 64, 162, 185, 81, 25, 79, 39, 170, 241, 159, 101, 157, 166, 48, 202,
        8, 110, 32, 219, 252, 108, 223, 95, 75, 71, 248, 2,
    ],
    [
        70, 149, 214, 96, 127, 240, 140, 215, 118, 64, 43, 48, 52, 112, 145, 51, 143, 95, 194, 7,
        84, 125, 84, 225, 114, 148, 96, 162, 136, 133, 92, 37,
    ],
    [
        130, 131, 208, 217, 183, 159, 92, 94, 35, 33, 196, 166, 113, 52, 196, 195, 96, 224, 90,
        148, 86, 92, 171, 15, 144, 220, 203, 144, 48, 171, 1, 11,
    ],
    [
        141, 151, 69, 209, 91, 138, 65, 189, 97, 90, 100, 40, 12, 249, 148, 165, 249, 226, 43, 108,
        147, 173, 71, 107, 4, 128, 174, 222, 71, 9, 149, 21,
    ],
    [
        173, 114, 239, 45, 155, 6, 118, 201, 139, 149, 249, 136, 77, 38, 31, 154, 181, 196, 252,
        251, 160, 19, 140, 62, 107, 168, 69, 242, 142, 246, 249, 29,
    ],
    [
        17, 83, 128, 73, 34, 8, 223, 220, 113, 124, 66, 191, 201, 148, 152, 106, 170, 154, 56, 58,
        48, 215, 173, 163, 219, 20, 249, 195, 17, 95, 94, 33,
    ],
    [
        205, 216, 87, 136, 166, 161, 105, 162, 246, 238, 20, 213, 195, 163, 233, 4, 157, 147, 128,
        26, 2, 105, 145, 61, 108, 230, 63, 180, 126, 157, 223, 18,
    ],
    [
        175, 146, 71, 147, 69, 244, 233, 250, 163, 242, 9, 0, 149, 126, 33, 4, 12, 249, 153, 19,
        99, 47, 223, 234, 189, 144, 210, 226, 33, 239, 51, 39,
    ],
    [
        226, 97, 86, 241, 231, 249, 243, 205, 181, 78, 133, 95, 163, 78, 21, 148, 226, 156, 146,
        90, 204, 133, 121, 90, 23, 96, 139, 170, 212, 227, 93, 27,
    ],
    [
        25, 175, 151, 81, 90, 87, 246, 118, 51, 91, 229, 95, 50, 81, 156, 254, 8, 10, 122, 198,
        227, 101, 77, 141, 223, 35, 38, 196, 78, 33, 208, 34,
    ],
    [
        88, 209, 97, 10, 32, 208, 96, 164, 43, 49, 59, 43, 116, 173, 157, 144, 180, 83, 217, 22,
        21, 45, 49, 106, 39, 223, 133, 234, 157, 100, 95, 28,
    ],
    [
        173, 32, 70, 199, 47, 67, 249, 150, 239, 20, 221, 152, 219, 177, 16, 193, 121, 156, 212,
        216, 9, 218, 218, 11, 122, 25, 59, 228, 61, 23, 128, 43,
    ],
];

#[derive(Serialize, Deserialize)]
pub struct NoteState {
    pub bump: u8,
    pub administrator: String,
    pub merkle: NoteStateMerkle,
    pub current_root: [u8; 32],
}
#[derive(Serialize, Deserialize)]
pub struct NoteStateMerkle {
    pub next_index: u32,
    pub filled_subtrees: Vec<[u8; 32]>,
}

pub fn get_pubkeys_utils(program_pubkey: String) -> String {
    let (state_pubkey, _state_bump) = Pubkey::find_program_address(
        &[b"mixer_state"],
        &Pubkey::from_str(&program_pubkey).unwrap(),
    );
    state_pubkey.to_string()
}

fn main() {
    println!("This is a placeholder main function for the lib binary.");
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PoseidonHash(pub [u8; 32]);

impl PoseidonHash {
    pub fn hash_pair(a: &[u8; 32], b: &[u8; 32]) -> PoseidonHash {
        let out = hashv(
            Parameters::Bn254X5,
            Endianness::LittleEndian,
            &[&a[..], &b[..]],
        )
        .expect("poseidon failed")
        .to_bytes();
        PoseidonHash(out)
    }
    pub fn hash_single(a: &[u8; 32]) -> PoseidonHash {
        let out = hashv(Parameters::Bn254X5, Endianness::LittleEndian, &[&a[..]])
            .expect("poseidon failed")
            .to_bytes();
        PoseidonHash(out)
    }

    pub fn empty_leaf() -> PoseidonHash {
        let zero = [0u8; 32];
        let out = hashv(Parameters::Bn254X5, Endianness::LittleEndian, &[&zero])
            .expect("poseidon failed")
            .to_bytes();
        PoseidonHash(out)
    }
}

#[derive(Debug, BorshDeserialize)]
pub struct DepositEvent {
    pub commitment: [u8; 32],

    pub leaf_index: u32,

    pub depositor: Pubkey,
}
#[derive(Debug)]
struct LeafEntry {
    index: usize,
    commitment: [u8; 32],
}

pub fn fetch_deposits(
    commitment_to_find: [u8; 32],
    rpc_url: &str,
    program_id: &str,
) -> Result<
    (
        Vec<[u8; 32]>,
        Vec<(usize, [u8; 32])>,
        Vec<usize>,
        usize,
        bool,
    ),
    Box<dyn std::error::Error + Send + Sync>,
> {
    let rpc = RpcClient::new_with_commitment(rpc_url.to_string(), CommitmentConfig::confirmed());

    let program_id = Pubkey::from_str(program_id)?;

    let sigs = rpc.get_signatures_for_address_with_config(
        &program_id,
        GetConfirmedSignaturesForAddress2Config {
            before: None,
            until: None,
            limit: None,
            commitment: Some(CommitmentConfig::confirmed()),
        },
    )?;

    let mut commitments = Vec::with_capacity(sigs.len());
    let mut leaf_indices = Vec::with_capacity(sigs.len());
    let mut my_index: Option<usize> = None;
    let mut leaf_entries: Vec<LeafEntry> = Vec::new();

    const PREFIX: &str = "Program data: ";
    let prefix_len = PREFIX.len();

    for sig_info in sigs {
        let sig: Signature = sig_info.signature.parse()?;

        let tx = rpc.get_transaction_with_config(
            &sig,
            RpcTransactionConfig {
                encoding: Some(UiTransactionEncoding::Json),
                commitment: Some(CommitmentConfig::confirmed()),
                max_supported_transaction_version: Some(0),
            },
        )?;

        if let Some(OptionSerializer::Some(logs)) = tx.transaction.meta.map(|m| m.log_messages) {
            for log in logs.iter().filter(|l| l.starts_with(PREFIX)) {
                let b64 = &log[prefix_len..];
                if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(b64) {
                    if bytes.len() >= 8 {
                        let (disc, data) = bytes.split_at(8);

                        if disc == [120, 248, 61, 83, 31, 142, 107, 144] {
                            if let Ok(event) = DepositEvent::try_from_slice(data) {
                                let idx = event.leaf_index as usize;
                                leaf_entries.push(LeafEntry {
                                    index: idx,
                                    commitment: event.commitment,
                                });
                                if event.commitment == commitment_to_find {
                                    my_index = Some(idx);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    leaf_entries.sort_unstable_by_key(|e| e.index);

    (leaf_indices, commitments) = leaf_entries.iter().map(|e| (e.index, e.commitment)).unzip();

    let found = my_index.is_some();
    let index = my_index.unwrap_or(0);
    Ok((
        commitments,
        leaf_entries
            .iter()
            .map(|e| (e.index, e.commitment))
            .collect(),
        leaf_indices,
        index,
        found,
    ))
}

pub fn merkle_check<const D: usize>(
    expected_root: [u8; 32],
    leaf: [u8; 32],
    siblings: &[[u8; 32]; D],
    path_bits: &[u8; D],
) {
    let mut node = leaf;
    for i in 0..D {
        let (l, r) = if path_bits[i] == 0 {
            (node, siblings[i])
        } else {
            (siblings[i], node)
        };
        node = PoseidonHash::hash_pair(&l, &r).0;
    }
    assert_eq!(node, expected_root, "Merkle proof did not match");
}

pub fn compute_merkle_proof<const D: usize>(
    leaves: &[[u8; 32]],
    target_index: usize,
) -> (Vec<[u8; 32]>, Vec<u8>, [u8; 32]) {
    assert!(target_index < leaves.len());

    let leaf_count = leaves.len();
    let full_leaves = leaf_count.next_power_of_two();
    let mut level_nodes: Vec<[u8; 32]> = Vec::with_capacity(full_leaves);
    level_nodes.extend_from_slice(leaves);
    level_nodes.resize(full_leaves, ZERO_HASHES[0]);

    let mut layers = Vec::with_capacity(D + 1);
    layers.push(level_nodes);
    for level in 0..D {
        let prev = &layers[level];
        let mut next = Vec::with_capacity(prev.len() / 2);
        for chunk in prev.chunks(2) {
            let left = chunk[0];
            let right = chunk.get(1).copied().unwrap_or(ZERO_HASHES[level]);
            next.push(PoseidonHash::hash_pair(&left, &right).0);
        }
        layers.push(next);
    }

    let mut siblings = Vec::with_capacity(D);
    let mut bits = Vec::with_capacity(D);
    let mut idx = target_index;
    for level in 0..D {
        let row = &layers[level];
        let sib = if idx % 2 == 0 {
            // even: sibling is to the right (or zero)
            row.get(idx + 1).copied().unwrap_or(ZERO_HASHES[level])
        } else {
            row[idx - 1]
        };
        siblings.push(sib);
        bits.push((idx % 2) as u8);
        idx /= 2;
    }

    let full_root = layers[D][0];
    (siblings, bits, full_root)
}

/// Convert a byte‐array or BigUint to hex string with `0x` and fixed length
pub fn to_hex32(bytes: &[u8; 32]) -> String {
    let mut s = hex::encode(bytes);
    s.truncate(64);
    while s.len() < 64 {
        s.insert(0, '0');
    }
    format!("0x{}", s)
}

pub fn biguint_to_32_le_bytes(n: &BigUint) -> [u8; 32] {
    let mut v = n.to_bytes_le();
    if v.len() > 32 {
        panic!("BigUint doesn’t fit in 32 bytes");
    }
    v.resize(32, 0);
    v.try_into().unwrap()
}

pub fn to_hex_vec(v: &Vec<[u8; 32]>) -> Vec<String> {
    v.iter().map(|b| mixer_lib::utils::to_hex32(b)).collect()
}
