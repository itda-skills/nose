use serde::Serialize;

/// Per-pair guards. A witness is best-effort enrichment, so a pathological pair
/// (huge generated file, a degenerately deep expression) fails closed to *no*
/// witness rather than burning time or risking the worker stack.
pub(super) const MAX_NODES: usize = 6_000;
pub(super) const MAX_NODE_PRODUCT: u64 = 16_000_000;
pub(super) const MAX_DEPTH: u32 = 1_000;
/// Holes are itemized up to this many; the count `holes` is always exact.
pub(super) const MAX_SPOTS: usize = 24;

/// The graded witness attached to a near family: how equal its two representative
/// copies really are, beyond the similarity score.
#[derive(Clone, Serialize)]
pub struct GradedWitness {
    /// `k` — the number of spots where the two value DAGs differ. `0` means equal in
    /// the modeled fraction; small `k` means "equal except these few parameters".
    pub holes: usize,
    /// Per-hole detail (capped at `MAX_SPOTS`); source text is filled by the
    /// presentation layer, which has file access.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub spots: Vec<WitnessHole>,
    /// Recognized divergence shapes: `effects-reordered`, `sink-superset-a/b`,
    /// `fragment-containment`, `low-substance`, `referent-mismatch`, `async-mirror`
    /// (one side awaits where the other does not — an async↔sync transformation twin).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub patterns: Vec<&'static str>,
    /// Names BOTH units consume that resolve to different referents (same-named but
    /// behaviorally distinct — e.g. `equals` on two classes). Non-empty ⇒ the witness
    /// is demoted: the copies are NOT equal-modulo-holes.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub referent_mismatches: Vec<String>,
    /// Names unresolved on at least one side — the claim is scoped past these (a
    /// reviewer should confirm they denote the same thing).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub caveat_names: Vec<String>,
    /// The two copies' **value graphs** are equal except at the listed holes, every
    /// hole is a small value-leaf, the behavior sinks aligned, and no referent
    /// mismatched — the strongest grade this channel makes. Scope: this is a claim
    /// about the unit body the value graph models, NOT its definition-site decorators,
    /// annotations, or signature (a `@deco(x)` vs `@deco(y)` difference outside the
    /// compared body is not seen — see `docs/graded-witness.md`). Near-channel
    /// evidence, not an exact-channel proof.
    pub equal_modulo_holes: bool,
    /// Either unit passed lossy lowering, so "equal" means equal in the *modeled*
    /// fraction; identically-keyed unmodeled constructs may still differ.
    pub modeled_caveat: bool,
}

/// One differing spot between the two value DAGs — the hole an extracted helper would
/// parameterize, classified by what kind of value differs.
#[derive(Clone, Serialize)]
pub struct WitnessHole {
    /// `literal` / `input` / `field` / `call` / `lambda` / `operator` / `expr` =
    /// value-leaf differences (clean parameters); `arity` / `shape` / `unmodeled` /
    /// `extra-sink` = structural divergence (not a clean parameter); `async-mirror` =
    /// an `await` present on one side only (the async↔sync twin point).
    pub class: &'static str,
    /// Source line range of the spot in the first copy, when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub a_lines: Option<(u32, u32)>,
    /// Source line range in the second copy, when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub b_lines: Option<(u32, u32)>,
    /// The spot's value feeds an effect/throw/break (ordered behavior), so swapping it
    /// is observable — a hole here is not freely parameterizable.
    pub effect: bool,
    /// Trimmed, length-capped source text of the spot in the first copy (filled by the
    /// presentation layer).
    #[serde(skip_serializing_if = "String::is_empty")]
    pub a_text: String,
    /// Same, second copy.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub b_text: String,
}
