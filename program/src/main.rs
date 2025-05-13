#![no_main]

use sp1_zkvm::entrypoint;
use sp1_zkvm::io::{commit, read};

use ark_bn254::Fr;
use light_poseidon::{Poseidon, PoseidonBytesHasher};
use mixer_lib::mix::merkle_check;

entrypoint!(main);

fn main() {
    const LEVEL: usize = 20;

    // ─── Public inputs ────-
    let root: [u8; 32] = read::<[u8; 32]>();
    let nullifier_hash_public: [u8; 32] = read::<[u8; 32]>();
    let recipient: [u8; 32] = read::<[u8; 32]>(); // Solana Pubkey
    let relayer: [u8; 32] = read::<[u8; 32]>(); // Solana Pubkey
    let fee: u64 = read::<u64>();
    let refund: u64 = read::<u64>();

    // ─── Private inputs ─────
    let nullifier: [u8; 32] = read::<[u8; 32]>(); // 32-byte nullifier
    let secret: [u8; 32] = read::<[u8; 32]>(); // 32-byte secret

    let path_element = read::<[[u8; 32]; LEVEL]>();
    let path_indices = read::<[u8; LEVEL]>();

    // ─── Poseidon(nullifier) → nullifier_hash ───────
    let mut poseidon_nullifier = Poseidon::<Fr>::new_circom(1).unwrap();
    let nullifier_hash_bytes: [u8; 32] = poseidon_nullifier.hash_bytes_le(&[&nullifier]).unwrap();
    assert_eq!(nullifier_hash_bytes, nullifier_hash_public);

    // ─── Poseidon(nullifier, secret) → commitment ────────
    let mut poseidon_commitment = Poseidon::<Fr>::new_circom(2).unwrap();
    let commitment_bytes: [u8; 32] = poseidon_commitment
        .hash_bytes_le(&[&nullifier, &secret])
        .unwrap();

    // ───  Merkle‐proof check ────────
    merkle_check::<LEVEL>(root, commitment_bytes, &path_element, &path_indices);
    // ─── Tie‐off constraints ───────────
    let _ = fee.wrapping_mul(fee);
    let _ = refund.wrapping_mul(refund);

    // ─── Commit public outputs ─────────
    commit(&root);

    commit(&nullifier_hash_bytes);

    commit(&recipient);
    commit(&relayer);

    commit(&fee);
    commit(&refund);
}
