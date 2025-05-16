use ark_bn254::Fr;
use light_poseidon::{Poseidon, PoseidonBytesHasher};

/// Verifies a Merkle‐proof of depth `LEVEL` with Circom‐compatible Poseidon(2).
/// Panics if the reconstructed root doesn’t match `root`.
pub fn merkle_check<const LEVEL: usize>(
    root: [u8; 32],
    leaf: [u8; 32],
    siblings: &[[u8; 32]; LEVEL],
    path_indices: &[u8; LEVEL],
) {
    let mut node = leaf;
    for i in 0..LEVEL {
        let (l, r) = if path_indices[i] == 0 {
            (node, siblings[i])
        } else {
            (siblings[i], node)
        };
        let mut pose = Poseidon::<Fr>::new_circom(2).unwrap();
        node = pose.hash_bytes_le(&[&l, &r]).unwrap();
    }
    assert!(node == root, "Merkle check failed");
}
