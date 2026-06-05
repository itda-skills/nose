//! FNV-1a 64-bit hashing primitives, shared by the content-cache key (`cache.rs`) and the
//! baseline family key (`baseline.rs`). The constants are the workspace-wide FNV parameters
//! defined in `nose-il` (the same ones `stable_symbol_hash` uses), so every FNV hash in the
//! project — string identities and these cache keys alike — shares one source of truth.

/// FNV-1a 64-bit offset basis — the accumulator seed.
pub(crate) const OFFSET_BASIS: u64 = nose_il::FNV_OFFSET_BASIS;

/// FNV-1a 64-bit prime — the per-step multiplier.
pub(crate) const PRIME: u64 = nose_il::FNV_PRIME;

/// One FNV-1a step: xor `x` into the accumulator `h`, then multiply by [`PRIME`].
pub(crate) fn mix(h: u64, x: u64) -> u64 {
    (h ^ x).wrapping_mul(PRIME)
}
