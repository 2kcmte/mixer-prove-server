use sp1_sdk::{include_elf, HashableKey, Prover, ProverClient};

/// The ELF (executable and linkable format) file for the Succinct RISC-V zkVM.
pub const MIXER_ELF: &[u8] = include_elf!("mixer-program");

fn main() {
    let prover = ProverClient::builder().cpu().build();
    let (_, vk) = prover.setup(MIXER_ELF);
    eprintln!("VK key {}", vk.bytes32());
}
