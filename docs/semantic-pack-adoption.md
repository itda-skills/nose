# Semantic pack adoption

Status: adoption and release-gate policy for builtin semantic packs.

This page defines how a semantic pack moves from external provider ownership to
nose-owned builtin support. The semantic vocabulary stays the same across lanes:
promotion changes ownership, trust, default enablement, CI gates, and rollback
responsibility. It must not create a second implementation path.

## Lanes

| lane | owner | default | meaning |
|---|---|---|---|
| `external-opt-in` | provider/user | off | Local manifests are explicit opt-ins and metadata-only until influence gates exist. |
| `builtin-optional` | nose | off | Shipped with nose, but not default-enabled until product risk is accepted. |
| `builtin-default` | nose | on | Shipped with nose and enabled by default. |

The same evidence vocabulary, contract rows, law rows, fixture refs, proof
obligations, and provenance concepts apply in every lane. Pack ids may stay the
same only through explicit ownership transfer. Otherwise promotion needs a
documented replacement or composition mapping, because builtin ids are reserved
and local external manifests cannot claim shipped nose ownership. Kernel
admission must continue to require dependency-backed evidence and fail-closed
conflict handling.

## External To Builtin Optional

Promote an `external-opt-in` pack to `builtin-optional` only when nose is ready to
own the support surface.

Requirements:

- The pack has a stable id, version policy, owner, and changelog entry.
- The manifest or descriptor passes structural conformance.
- Exact-capable rows have positive fixtures and hard negatives for every
  admitted contract/law family.
- Exact-capable rows have executable conformance or oracle gates that exist,
  run in CI, and pass before they can influence exact results. A plan is enough
  to track adoption work, not enough to open exact influence.
- Language packs additionally name parser/lowering ownership, language-version
  coverage, embedded-region boundaries where relevant, parse/lower failure
  behavior, and corpus fixtures before official support is advertised.
- Required symbol/import/source/effect/demand/version/shadowing/arity
  obligations are explicit enough for the kernel to fail closed.
- Conflicts with existing builtin rows are resolved by composition or replacement
  policy, not provider priority.
- Query-regression output drift and runtime drift are measured against `main`.
  Use the [Product Behavior Gate](semantic-pack-architecture.md#product-behavior-gate)
  and [Performance Gate](semantic-pack-architecture.md#performance-gate)
  thresholds.
- User-facing docs explain the support boundary and unsupported cases.
- A nose maintainer is named for release responsibility, regression response,
  and user-visible support decisions.
- Capabilities/query JSON changes are versioned or additive according to their
  documented contracts.

Promotion does not imply default enablement. A newly builtin-optional pack may
still have recall gaps or ecosystem-version risk, but its influence path must be
owned and testable by nose.

## Builtin Optional To Builtin Default

Promote `builtin-optional` to `builtin-default` only after corpus evidence shows
the default surface is useful and safe.

Requirements:

- Query-regression shows no unexplained family-output drift.
- False-merge risk has hard negatives and a rollback path.
- Default-surface noise has been reviewed on representative real repositories.
- Runtime stays within the pack-architecture budget or has an accepted product
  decision.
- Exact acceptance never widens without dependency-backed evidence and hard
  negatives.
- Docs, examples, and capabilities describe the default behavior.
- Release notes name the new default support and any known unsupported cases.

Default enablement is a product decision layered on top of the same kernel
admission checks. It must not bypass external/builtin parity rules.

## Rollback

If a builtin pack causes false merges, unacceptable noise, or runtime regression,
prefer the smallest reversible action:

- Disable the pack by default by moving it from `builtin-default` to
  `builtin-optional`.
- Disable only the risky contract/law row when the rest of the pack remains
  useful.
- Tighten admission requirements with additional dependency or hard-negative
  checks.
- Revert the descriptor/producer change if the issue is systemic.

Rollback must preserve provenance. Reports should still make it clear which pack
or row was responsible for the disabled behavior, so users and maintainers can
track the fix.

## Implementation Rule

Enablement is trust/default metadata plus gates:

- `source`: how the pack is loaded, such as `compiled-builtin` or
  `local-manifest`;
- `trust`: who owns the risk, such as `external-opt-in`, `builtin-optional`, or
  `builtin-default`;
- `enabled_by_default`: whether product analysis uses the pack by default;
- `influence`: whether admitted evidence/contracts can affect analysis.

Do not fork semantics between "external implementation" and "official
implementation." If a pack is promoted, reuse the same evidence and contract
vocabulary with stronger ownership, conformance, and release gates.

## Related

- [semantic-pack-architecture](semantic-pack-architecture.md)
- [semantic-pack-conformance](semantic-pack-conformance.md)
- [semantic-pack-extension-api-v0](semantic-pack-extension-api-v0.md)
- [semantic-pack-loading](semantic-pack-loading.md)
