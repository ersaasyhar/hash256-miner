use tiny_keccak::{Hasher, Keccak};

pub fn keccak256(bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Keccak::v256();
    hasher.update(bytes);
    let mut out = [0u8; 32];
    hasher.finalize(&mut out);
    out
}

#[inline(always)]
pub fn hash_meets_target_preimage(preimage: &mut [u8; 40], nonce: u64, target_be: &[u8; 32]) -> bool {
    preimage[32..].copy_from_slice(&nonce.to_be_bytes());
    let digest = keccak256(preimage);
    digest < *target_be
}
