//! Graded equivalence witness (#315): anti-unification over two units' value DAGs.
//!
//! The exact channel proves "these compute the same thing" (equal fingerprint). The
//! near channel only scores similarity. This module bridges them: given two near
//! units' value DAGs, it computes their *least general generalization* — aligns the
//! two graphs node-by-node and reports the spots where they differ as **holes**. The
//! result grades the near family's witness from a bare score to "equal **except at
//! these k holes**", with each hole's value class and a soundness-relevant referent
//! check.
//!
//! It is **fail-closed**: a name both units consume that resolves to different
//! referents demotes the claim (`referent-mismatch`); a name that cannot be resolved
//! is reported as a scoped caveat; a pair too large or too deep to align soundly
//! yields no witness at all (`None`) rather than a guessed one. Recognized divergence
//! shapes (reordered effects, one-sided supersets, fragment containment) are reported
//! as patterns instead of noisy positional holes.
//!
//! proof-obligation: detect.graded_witness

mod analysis;
mod anti_unify;
mod dag;
mod model;

#[cfg(test)]
mod tests;

pub use analysis::graded_witness;
pub use model::{GradedWitness, WitnessHole};
