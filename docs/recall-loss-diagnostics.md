# Recall-loss diagnostics

`nose verify --recall-loss-report <file>` writes a local JSON report for the
strictness/recall tradeoff in exact semantic matching. It is a diagnostics
artifact, not telemetry: nose does not send it anywhere, and raw source snippets
are omitted by default.

Use this report when a semantic-kernel or semantic-pack change touches exact
admission. The hard rule stays unchanged: stricter admission must not introduce
false merges. The report adds the missing second question: if exact admission got
stricter, what recall did it close and which capability or evidence gap explains
that loss?

## Command

```sh
nose verify <path> --max-violations 0 --recall-loss-report recall-loss.json
```

The command reuses the same interpreter oracle as `nose verify`. The human
stdout remains the existing soundness/completeness report; the JSON artifact is
written only to the requested path.

Compare two local reports with:

```sh
python3 scripts/recall-loss-diff.py before.json after.json
```

The comparison is deterministic and suitable for PR comments: it shows hard gate
deltas, completeness and under-merge deltas, oracle exclusion deltas, admission
rejection deltas by reason, and top opportunities added or removed.

## Report shape

The current schema is `recall_loss_report.v1.json`:

| field | meaning |
|---|---|
| `schema_version` | Report schema version. Starts at `1`. |
| `privacy` | Local-artifact flags. `remote_collection` is `false`; `raw_source_snippets_included` is `false` by default. |
| `summary` | Total units, interpretable units, excluded units, canon checks, and exact-admission rejection count. |
| `soundness_gate` | Fingerprint groups, false merges, advisory disagreements, canon-preservation violations, `--max-violations`, and gate result. |
| `completeness` | Behavior groups, behavior-equal pairs, fingerprint-equal pairs, completeness percentage, and under-merged groups. |
| `oracle_under_merges` | Behavior-equal but fingerprint-split pairs, sorted by value-Jaccard nearness. This is the structured form of the `--leads` signal. |
| `oracle_exclusions` | Fail-closed oracle exclusions by reason and unit location. |
| `import_snapshot_census` | Corpus-level imported immutable snapshot diagnostics: successful snapshot record counts, unresolved binding-import miss counts by reason/language, and stable hash/location rows for follow-up fixtures. |
| `admission_rejections` | Interpretable units whose exact semantic claim is closed, with structured reason, gate, capability, missing evidence, #594 obligation family/subreason, oracle status, and stable location. |
| `by_reason` | Rollups for admission rejections by reason/gate/capability. |
| `by_obligation` | Rollups for admission rejections by #594 obligation family and stable subreason. |
| `top_opportunities` | Ranked under-merge opportunities that future capability work can turn into fixtures or focused follow-up issues. |

The current admission-rejection taxonomy is diagnostics-only; it does not widen
or narrow product admission by itself.

| reason | meaning |
|---|---|
| `import-symbol-callee-identity-proof-missing` | An ordinary call is interpretable, but exact admission lacks reusable proof of the callee/import/symbol target. |
| `receiver-domain-proof-missing` | A receiver method call needs receiver-domain evidence rather than selector-name inference. |
| `library-api-occurrence-proof-missing` | A canonical builtin/API occurrence lacks admitted pack or producer evidence. |
| `hof-demand-effect-proof-missing` | A higher-order surface lacks a demand, effect, and materialization profile. |
| `source-surface-proof-missing` | A source construct, operator, comprehension, Rust macro invocation, or syntax distinction is required but not proven. |
| `mutation-effect-boundary` | Mutation, place, side-effecting call, or effect obligations close exact admission until an effect-preserving contract exists. |
| `unsupported-runtime-boundary` | Runtime/protocol boundaries such as raw lowered constructs, try/throw, splat, or keyword-argument surfaces intentionally fail closed. |
| `value-fingerprint-too-small` | The unit is strict-exact-safe, but its value fingerprint is below the non-degenerate exact-claim floor. |
| `unattributed-strict-exact-unsafe` | Fallback for unknown strict-exact rejection. This should stay visible and should trend toward zero. |

Unknown cases must remain explicit as `unattributed-strict-exact-unsafe`; do not
guess.

Each admission rejection also carries an `obligation_family` and
`obligation_subreason`. These fields are diagnostics-only and refine broad
reason buckets into the cross-language vocabulary from [scheduling-channel-callback-obligations-594](scheduling-channel-callback-obligations-594.md).
They do not change exact admission.

| obligation family | typical subreason | meaning |
|---|---|---|
| `callback-demand-effect` | `callback-member-call-effect-proof-missing`, `callback-rust-macro-call-effect-proof-missing`, `callback-direct-function-call-effect-contract-missing`, `callback-imported-function-call-effect-contract-missing`, `callback-assignment-effect-proof-missing`, `callback-runtime-boundary-proof-missing`, `callback-identity-or-shape-proof-missing`, `mapping-callback-demand-effect-profile-missing`, `predicate-callback-demand-effect-profile-missing`, `flattening-callback-demand-effect-profile-missing`, `optional-callback-demand-effect-profile-missing`, or `reduction-callback-demand-effect-profile-missing` | A HOF/callback surface lacks timing, callback identity, effect visibility, result role, call-shape proof, or materialization proof. |
| `receiver-mutation` | `effect-preserving-contract-missing` | A mutation/place/effect boundary blocks exact admission. |
| `scheduling-boundary` | `runtime-protocol-boundary-contract-missing` | A lowered runtime/protocol construct needs scheduling or protocol semantics before exact use. |
| `ambiguous-selector-boundary` | `receiver-domain-proof-missing`, `library-api-occurrence-evidence-missing`, or a call-target proof label | Selector, receiver, library API, or callee identity proof is missing. |
| `source-protocol-boundary` | `source-surface-contract-missing`, `rust-macro-expansion-contract-missing` | A source/protocol syntax distinction is required but not proven. |
| `non-degenerate-fingerprint-floor` | `non-degenerate-value-fingerprint` | The unit is otherwise exact-safe but too small for a non-degenerate exact claim. |
| `unattributed-boundary` | `strict-exact-safe-tree-missing` | A strict-exact rejection still lacks a more specific capability attribution. |

The checked-in baseline summaries and the five-cycle recovery log are described
in [recall-loss-recovery-loop](recall-loss-recovery-loop.md).
The #572 cycle also records a diagnostics-only refinement: expression-statement
calls that need an effect contract stay in the effect boundary bucket, and
unmodeled Rust macros such as `format!` stay in the source-surface bucket until
a macro expansion or library contract proves their behavior.
The #574 cycle keeps the same `import-symbol-callee-identity-proof-missing`
reason but splits its `missing_evidence` labels by call-target surface, such as
`local-or-parameter-call-target-proof`, `scoped-path-call-target-proof`,
`member-call-target-proof`, imported/global target proof labels, and admitted
target-present call-contract proof labels. Build the checked-in census with
`scripts/recall-loss-callee-census.py`.

The post-#594 callback diagnostics refinement keeps the same
`hof-demand-effect-proof-missing` reason, but HOF rejections now also expose
kind-specific and callback-specific `missing_evidence` labels such as
`hof-map-callback-demand-effect-profile`, `hof-filter-callback-demand-effect-profile`,
`hof-callback-call-effect-proof`, `hof-callback-assignment-effect-proof`,
`hof-callback-runtime-boundary-proof`, and `hof-callback-identity-proof`. The
checked baselines are [callback-demand-effect-diagnostics-2026-06-28.v1.json](../bench/recall_loss/callback-demand-effect-diagnostics-2026-06-28.v1.json)
and [callback-demand-effect-diagnostics-2026-06-28.v2.json](../bench/recall_loss/callback-demand-effect-diagnostics-2026-06-28.v2.json).
The follow-up [callback-demand-effect-diagnostics-2026-06-28.v3.json](../bench/recall_loss/callback-demand-effect-diagnostics-2026-06-28.v3.json)
keeps exact admission closed but splits callback call-effect proof by
producer-facing call shape: member calls (`10`), Rust macro calls (`8`),
direct-function effect contracts (`3`), and imported-function effect contracts
(`1`) on the local `crates` surface.

`import_snapshot_census` is also diagnostics-only. It does not make an imported
value exact-safe. It records why a proven binding import did not become an
imported immutable snapshot after corpus import resolution. Current miss reasons
include:

| reason | meaning |
|---|---|
| `provider-module-missing` | The imported module hash has no provider file in the analyzed corpus. |
| `provider-export-missing` | A provider module exists, but no matching exported binding was found. |
| `provider-export-ambiguous` | More than one provider binding could own the same module/export coordinate. |
| `provider-external-crate-boundary` | The import targets a known external crate dependency, which is outside same-corpus provider lookup. |
| `provider-reexport-ambiguous` | More than one direct public re-export could own the requested module/export coordinate. |
| `provider-reexport-callable-boundary` | A direct public re-export resolves to a callable item, not an immutable literal provider value. |
| `provider-reexport-type-boundary` | A direct public re-export resolves to a type item, not an immutable literal provider value. |
| `provider-reexport-module-namespace-boundary` | A direct public re-export resolves to a module namespace, not an immutable literal provider value. |
| `provider-reexport-external-crate-boundary` | A direct public re-export target resolves to a known external crate boundary. |
| `provider-reexport-target-export-missing` | A direct public re-export exists, but its target module has no matching export in the analyzed corpus. |
| `provider-reexport-target-module-missing` | A direct public re-export exists, but its target module is not resolved in the analyzed corpus. |
| `cross-language-boundary` | A same-coordinate provider exists only in a different lowered language. |
| `self-import-boundary` | The only matching provider is the importer file itself. |
| `importer-binding-mutated` | The importer mutates the imported binding before it could be snapshotted. |
| `provider-binding-unsafe` | The provider binding is mutated or escapes through an opaque call argument. |
| `provider-library-api-proof-missing` | The provider RHS is a factory call without admitted `LibraryApi` proof. |
| `provider-factory-arguments-not-exact-safe` | The provider factory is proven, but its arguments are not export-safe. |
| `provider-aggregate-children-not-exact-safe` | The provider aggregate has a surface proof, but its children are not export-safe imported literal values. |
| `provider-sequence-surface-not-import-literal-safe` | The provider aggregate has a proven sequence surface, but that surface is not an imported-literal value surface. |
| `provider-aggregate-child-reference-boundary` | The provider aggregate contains a child reference, field path, or index expression rather than a literal/export-safe value. |
| `provider-aggregate-child-import-coordinate-boundary` | The provider aggregate contains an import-coordinate placeholder; coordinates are proof, not imported literal values. |
| `provider-aggregate-child-surface-not-exact-safe` | A nested provider aggregate child has a sequence surface that is not exact-tree-safe. |
| `provider-aggregate-child-call-boundary` | A provider aggregate child is a call expression without a supported imported-literal child contract. |
| `provider-sequence-surface-proof-missing` | The provider aggregate lacks the sequence-surface proof required for imported literal export. |
| `unsupported-provider-rhs-shape` | The provider RHS is not a literal, supported aggregate, or supported factory call. |

The #567 import-backed immutable provenance closeout is the reference example
for using this census to end a capability slice without widening admission. See [import-backed immutable provenance closeout](import-backed-immutable-provenance-closeout-567.md)
and the checked-in [closeout artifact](../bench/recall_loss/issue-567-closeout.v1.json).

## PR reporting

For any PR that changes exact semantic admission, include this table or the same
fields in prose:

| metric | before | after | note |
|---|---:|---:|---|
| false merges |  |  | Hard gate: must stay `0` on the selected verification surface. |
| canon-preservation violations |  |  | Hard gate: must stay `0`. |
| completeness percentage |  |  | Soft signal: explain meaningful movement. |
| under-merged behavior groups |  |  | Soft signal: increased misses need attribution. |
| oracle exclusions by reason |  |  | Soft signal: budget/path/uninterpretable growth needs a cause. |
| admission rejections by structured reason |  |  | Main recall-loss signal. |
| import snapshot misses by reason |  |  | Process signal for deciding the next imported-value capability slice. |
| top attributed recall-loss bucket |  |  | Name the follow-up capability, fixture, or unsupported boundary. |

Use `scripts/recall-loss-diff.py before.json after.json` for the before/after
table when both full local reports are available.

Hard gate:

- `false_merges == 0`;
- `canon_preservation_violations == 0`.

Soft regression gate:

- any increase in under-merged groups, oracle exclusions, or admission rejections
  should be attributed to a structured reason bucket;
- intentional fail-closed recall loss should name a follow-up capability,
  fixture, or unsupported boundary;
- recall gains should state which strict evidence or capability made them safe.

## Relationship to other diagnostics

- [`oracle-value-model`](oracle-value-model.md) explains the interpreter oracle,
  value model, and `--falsify` search.
- [`type4-adversarial-coverage`](type4-adversarial-coverage.md) explains how
  `nose verify --leads` becomes Type-4 target packets.
- [`semantic-pack-architecture`](semantic-pack-architecture.md) defines the
  product behavior gate for semantic-pack and semantic-kernel changes.
- [`recall-loss-recovery-loop`](recall-loss-recovery-loop.md) defines the
  checked-in baseline summaries, report diff workflow, and cycle contract.
- [`source-facts`](source-facts.md) and [`evidence-records`](evidence-records.md)
  define the evidence that future narrow admission-rejection buckets should
  reference.
