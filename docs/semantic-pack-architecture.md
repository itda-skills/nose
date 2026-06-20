# Semantic pack architecture

Status: active migration plan for
[issue #473](https://github.com/corca-ai/nose/issues/473).

This page defines the target boundary for builtin and external semantic packs.
It is the contributor-facing rulebook for moving language, stdlib, library, and
law knowledge out of privileged kernel-only paths without changing product
behavior by accident.

## Terminology

Use `builtin` for packs maintained and shipped by nose.

| lane | meaning |
|---|---|
| `builtin-default` | shipped with nose and enabled by default |
| `builtin-optional` | shipped with nose but opt-in until default-surface risk is accepted |
| `external-opt-in` | provider/user responsibility; enabled only by explicit user choice |

Current v0 manifests and capabilities output still expose compatibility
spellings. Keep accepting them while their schema is supported:

- `default-first-party`: trust label for `builtin-default`;
- `first-party-optional`: trust label for `builtin-optional`;
- `compiled-first-party`: source/loading label for compiled builtin packs.

Use the builtin vocabulary in docs, internal naming, and future schema planning.
A public enum or field-value rename requires a schema/capabilities update, not a
silent string change.

The broad `nose.first_party` pack id is a temporary compatibility descriptor for
compiled semantic knowledge that has not yet moved behind narrower builtin pack
ids. New ownership should move toward explicit ids such as `nose.lang.python`,
`nose.python.builtins.collection_factories`,
`nose.python.stdlib.type_domain`, and `nose.value_graph.laws`.

## Boundary

The semantic kernel owns the stable admission machinery:

- IL and evidence vocabulary;
- evidence anchors and dependency validation;
- fail-closed admission policy;
- trust and channel policy;
- pack loading, registry, capabilities, and provenance reporting.

Packs own language, library, and law knowledge:

- file, language, and embedded-region identity;
- parser and lowering entrypoints for builtin compiled language packs;
- source, import, symbol, domain, type, guard, place, effect, call-target,
  library API, and sequence-surface evidence producers;
- API contract rows and law rows;
- demand and effect profiles attached to admitted operations;
- conformance fixtures, hard negatives, proof/oracle links, and unsupported
  boundaries;
- pack metadata, versioning, trust, and provenance.

Builtin language packs may keep parser and lowering implementation in tree. The
first goal is pack ownership, provenance, and shared admission vocabulary, not an
external parser plugin runtime.

## Contributor Rule

New builtin language, stdlib, library, or law semantics should be pack-owned. If
a change must add a temporary kernel/frontend shim before a pack-owned path
exists, the PR must link the tracking issue and state the removal condition.

Do not add new raw selector, raw name, raw type, or raw tag admission checks when
the same fact can be represented by a pack-owned evidence producer or contract
row. Selectors and syntax may be inputs to evidence production; they are not
admission proof by themselves.

## Product Behavior Gate

The pack migration is structural first. Descriptor, registry, naming, reporting,
and provenance changes should not change which clone families are found, hidden,
or accepted as exact/near unless the PR explicitly says it changes behavior.

Every implementation PR for this migration must run a product output comparison,
not just unit tests. Report changed family counts, surface counts, family shapes,
fragment reason codes, semantic-law provenance, or query JSON schema fields.
Classify each drift as one of:

- intentional metadata drift;
- precision improvement;
- measured recall change;
- bug fix.

Behavior-change defaults:

- Descriptor-only and reporting-only phases should preserve family output except
  intentional top-level pack metadata such as `semantic_packs`.
- Moving existing builtin semantic surfaces behind pack descriptors should
  preserve exact/near results unless the old path was demonstrably unsafe.
- Any widening of exact acceptance requires dependency-backed pack evidence and
  hard negatives.
- Any reduction in exact acceptance must be documented as a fail-closed
  precision fix or deferred recall with follow-up tracking.
- Metadata-only external packs must not change families, ranking, witnesses,
  surfaces, or exact/near results.

## Performance Gate

The migration should be performance-neutral or performance-positive. A pack
boundary must not put manifest parsing, string-heavy lookup, dynamic dispatch,
global locks, or repeated registry scans on per-node or per-unit hot paths.
Builtin descriptors should be static data or once-built indexes. External
manifest loading should happen before analysis, and metadata-only packs must not
add work inside normalize, detect, value-graph, fragment, or oracle loops.

Use the product query-regression path when a corpus is available:

```sh
cargo build --release --bin nose
python3 bench/type4/query_regression/query_regression.py baseline \
  --nose ./target/release/nose \
  --repos-root /path/to/main/bench/repos \
  --build-ref "main@$(git rev-parse --short HEAD)"
python3 bench/type4/query_regression/query_regression.py compare \
  --nose ./target/release/nose \
  --repos-root /path/to/main/bench/repos
```

The measured product command is:

```sh
nose query <repo> all top=0 --mode semantic --format json
```

For small or corpus-free changes, also run:

```sh
python3 bench/type4/query_regression/query_regression.py selftest
cargo bench -p nose-detect --bench pipeline
```

Use `<= 5%` median wall-clock and `NOSE_TIME` phase growth with a 5 ms floor as
the normal target. Any repo or phase above that target needs a root-cause note
and either an optimization or explicit follow-up. Any `> 10%` median growth on
the query-regression subset, or any HoF smoke budget failure, should block the
PR unless there is a documented product decision accepting the cost. Report
material binary-size growth from descriptor metadata or static tables, and
justify large static tables by the behavior or performance benefit they unlock.

## Issue #473 Phase Order

These phases are specific to issue #473. Older roadmap phase headings record
previous semantic-kernel tranches.

1. **Phase 0, boundary and measurement:** align docs, terminology, trust lanes,
   compatibility labels, contributor rules, and behavior/performance gates.
2. **Phase 1, builtin descriptor registry:** add a small descriptor shape for compiled
   builtin packs without adding plugin lifecycle, sandboxing, remote registries,
   or external execution.
3. **Phase 2, query reporting:** carry the active `SemanticPackSet` through
   `nose query` and add top-level `semantic_packs` to query JSON schema v5.
   Update [query-json](query-json.md) and [capabilities](capabilities.md) in
   the same PR because older query JSON schemas reject unknown fields.
4. **Phase 3, reference stdlib pack:** make `nose.python.stdlib.type_domain` the first
   end-to-end descriptor-backed builtin stdlib pack.
5. **Phase 4, builtin language slice:** add one `nose.lang.<language>` descriptor that
   owns language identity, parser/lowering metadata, and source-fact producer
   metadata while preserving behavior. The first vertical slice is
   `nose.lang.c`, covering the existing C parser/lowering binding and the
   explicit unsigned 32-bit byte-lane cast source-fact producer.
6. **Phase 5, builtin stdlib/library/law packs:** move official semantic rows behind
   narrow builtin pack ids and shared admitted-contract resolvers. The first
   slice is `nose.python.builtins.collection_factories`, which owns Python
   `list`, `set`, `frozenset`, and `tuple` collection-factory API occurrence
   provenance.
7. **Phase 6, external influence:** only after the builtin path is proven, start with a
   small data-only external row class behind explicit opt-in trust gates.
8. **Phase 7, adoption gates:** define `external-opt-in -> builtin-optional` and
   `builtin-optional -> builtin-default` promotion criteria, rollback behavior,
   corpus regression, docs, and performance budgets.

## See also

- [semantic-kernel](semantic-kernel.md)
- [semantic-kernel-roadmap](semantic-kernel-roadmap.md)
- [semantic-pack-extension-api-v0](semantic-pack-extension-api-v0.md)
- [semantic-pack-loading](semantic-pack-loading.md)
