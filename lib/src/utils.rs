use ark_bn254::Fr;
use light_poseidon::{Poseidon, PoseidonBytesHasher};
use num_bigint::BigUint;
use rand::RngCore;

/// Generate a uniformly random BigUint of `nbytes` bytes (little‐endian)
pub fn rbigint(nbytes: usize) -> BigUint {
    let mut buf = vec![0u8; nbytes];
    rand::thread_rng().fill_bytes(&mut buf);
    BigUint::from_bytes_le(&buf)
}

/// Make a 31‐byte little‐endian buffer from any BigUint (truncates or pads)
pub fn to_31_bytes(x: BigUint) -> [u8; 31] {
    let mut buf = x.to_bytes_le();
    buf.resize(31, 0);
    buf.try_into().unwrap()
}

/// Compute Poseidon‐Circom(1) over exactly one 32‐byte field encoding
pub fn hash1(input: &[u8; 32]) -> [u8; 32] {
    let mut pose = Poseidon::<Fr>::new_circom(1).unwrap();
    pose.hash_bytes_le(&[input]).unwrap()
}

/// Compute Poseidon‐Circom(2) over two 32‐byte field encodings
pub fn hash2(a: &[u8; 32], b: &[u8; 32]) -> [u8; 32] {
    let mut pose = Poseidon::<Fr>::new_circom(2).unwrap();
    pose.hash_bytes_le(&[a, b]).unwrap()
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

pub fn create_random_commitment() -> (
    BigUint,  // nullifier
    BigUint,  // secret
    Vec<u8>,  // preimage = nf31 || sec31
    [u8; 32], // commitment = H(nf,sec)
    [u8; 32],
) {
    let nullifier = rbigint(31);
    let secret = rbigint(31);

    let nf31 = to_31_bytes(nullifier.clone());
    let sec31 = to_31_bytes(secret.clone());

    let mut preimage = Vec::with_capacity(62);
    preimage.extend_from_slice(&nf31);
    preimage.extend_from_slice(&sec31);

    let nullifier_hash = hash1(&{
        let mut tmp = [0u8; 32];
        tmp[0..31].copy_from_slice(&nf31);
        tmp
    });
    let commitment = hash2(
        &{
            let mut t = [0u8; 32];
            t[0..31].copy_from_slice(&nf31);
            t
        },
        &{
            let mut t = [0u8; 32];
            t[0..31].copy_from_slice(&sec31);
            t
        },
    );
    println!("nullifier     = {}", nullifier);
    println!("secret        = {}", secret);
    println!("nullifierHash = {:?}", nullifier_hash);
    println!("commitment    = {:?}", commitment);
    println!("preimage      = 0x{}", hex::encode(&preimage));
    (nullifier, secret, preimage, commitment, nullifier_hash)
}
