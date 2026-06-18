//! Proof-sensitive value-graph rules.
//!
//! # Two tiers of canonicalization
//!
//! Value-graph rewrites live in one of two places, by whether their soundness depends on a
//! value-domain *proof*:
//!
//! 1. **Value-INDEPENDENT canon — `canonicalize/*` rewrite-family modules** (e.g.
//!    `ordering::order_bin_operands`, `unary::unary_canon`'s De Morgan,
//!    `binary::bool_and_or_canon`, `binary::ac_chain_canon`'s flattening). These hold for
//!    *every* operand type — commuting an AC chain, comparison-direction canon, De Morgan —
//!    so they need no operand evidence and carry no formal obligation. They run
//!    unconditionally from `mk`.
//! 2. **Value-DEPENDENT / proof-backed rules — this module tree** (e.g. `clamp`,
//!    `factor_distribute`, `promise_then`). These fire only when a value-domain precondition
//!    is *proven* (numeric operands, a bound-order fact, an admitted promise contract), so an
//!    unproven case must fail closed.
//!
//! Any new semantic rewrite in this module tree must have a matching
//! `formal/obligations/normalize/value_graph/<rule>/meta.toml` entry. The
//! formal-obligation linter checks that directory/file pairing mechanically.
//!
//! Rule of thumb for a NEW rewrite: if it could change behavior on *some* operand type (string
//! vs number, float vs int, NaN), it is tier 2 and belongs here with a proof obligation; if it
//! is sound for all types by construction, it is tier 1 and belongs in the focused
//! `canonicalize/*` module for that rewrite family.

pub(super) mod clamp;
pub(super) mod factor_distribute;
pub(super) mod promise_then;
