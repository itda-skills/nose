# Semantic pack conformance

Status: nose provides a local semantic-pack v0 conformance harness for manifest
structure and declared fixture assets. The harness is a provider/user workflow,
not nose approval of third-party semantic correctness. External packs remain
`metadata-only` when loaded by `nose query`.

## Goal

The conformance workflow gives pack providers a reproducible way to show that a
pack meets the extension contract's minimum structural obligations:

- the manifest parses as semantic-pack v0;
- stable ids, enum values, trust/default policy, provenance, and compatibility
  declarations are present;
- evidence producers, contracts, laws, dependencies, and conformance references
  are internally linked;
- contracts and value laws declare object-shaped semantics, and exact-capable
  declarations add evidence requirements plus demand/effect semantics;
- positive fixtures and hard negatives are declared with expectation labels;
- fixture files exist at paths relative to the manifest file.

This keeps the ecosystem boundary narrow: packs publish evidence, contracts, and
fixtures; the kernel decides whether evidence is admissible for a channel; users
decide whether to enable external packs.

## Non-goals

The harness does not:

- execute external evidence producers;
- register external contract rows with exact consumers;
- treat a passing structural check as permission for external rows to influence
  analysis;
- run a behavioral oracle over fixture contents;
- prove that the provider's semantic claims are complete or true;
- certify, approve, rank, or endorse external packs;
- compare `compatibility.nose` against the installed nose version beyond checking
  that the declared requirement is parseable as a version constraint.

Builtin default packs are different. nose owns their tests, hard negatives,
proof obligations, release gates, and documentation because those packs ship with
the binary and affect exact analysis by default. The Python stdlib type-domain
example manifest mirrors the first compiled builtin pilot pack, but local
copies of that manifest still load only as external metadata unless nose ships
the pack as compiled builtin code.

## Command

Run the harness against a manifest file or a directory of direct `*.json`
manifests:

```sh
nose semantic-pack check semantic-packs/python-math-prod.json
nose semantic-pack check docs/examples/semantic-packs/v0 --format json
```

Directory checking follows the same local discovery rule as pack loading: direct
JSON children only, sorted by filename, no recursion, no registry, and no network.
Relative fixture paths are resolved from the manifest's directory.

The command exits non-zero when any manifest is invalid, when pack ids collide
with each other or any compiled builtin pack id, or when declared conformance
fixtures are missing a path, missing an expectation, or point at a file that does
not exist. For `--format json`, the command still writes the report before
returning the non-zero exit.
The example [law pack](examples/semantic-packs/v0/law-pack.json) uses this
workflow to declare value-law positives and hard negatives. Passing the check
confirms only that the law metadata and fixture assets are structurally present;
it does not register those external laws with exact consumers.
Its expectation labels distinguish report-visible positives such as
`semantic-law-provenance-present` from narrower unit-level fixtures such as
`internal-law-unit-positive`; the harness preserves those labels but does not
execute them.

## JSON Report

`--format json` emits schema version 1:

```json
{
  "schema_version": 1,
  "status": "ok",
  "totals": {
    "manifests": 1,
    "positive_fixtures": 1,
    "hard_negatives": 2,
    "fixture_issues": 0,
    "influence_rows": 1,
    "blocked_influence_rows": 1
  },
  "influence_preflight": {
    "status": "blocked",
    "rows": [
      {
        "kind": "contract",
        "row_id": "python.example.contract",
        "row_hash": "0123456789abcdef",
        "pack_id": "com.example.semantic-pack",
        "pack_hash": "fedcba9876543210",
        "manifest_path": "/repo/packs/example.json",
        "channel": "exact-empirical",
        "passed": false,
        "blockers": [
          "data-only-registration",
          "dependency-backed-evidence-unavailable",
          "explicit-influence-trust-gate-missing",
          "executable-conformance-unavailable"
        ]
      }
    ]
  },
  "manifests": []
}
```

Each manifest entry includes pack provenance, declaration counts, the optional
provider-supplied conformance command, proof links, and per-fixture issue labels
such as `missing-path`, `missing-expectation`, `missing-file`, and
`absolute-path`.

The JSON report also includes `influence_preflight`. This is a row-level
admission preview for local external declarations, not a grant of influence. In
v0, structurally valid external producer, contract, and value-law rows still
report blockers such as `data-only-registration`,
`dependency-backed-evidence-unavailable`,
`explicit-influence-trust-gate-missing`,
`executable-conformance-unavailable`, and `row-conflict`. The `totals` object
includes `influence_rows` and `blocked_influence_rows` so integrations can fail a
provider workflow when a pack is structurally valid but still not admissible for
analysis. Row and pack hashes are stable 16-hex-digit strings.

Integrations should discover support through [capabilities](capabilities.md):
`commands.stable` includes `semantic-pack`, `schemas.semantic_pack_conformance`
lists supported report schema versions, and `semantic_packs.conformance` lists
accepted local input sources.

## Provider Responsibilities

External pack providers own:

- the truth of every language, runtime, package, API, protocol, law, demand,
  effect, mutation, exception, and version claim;
- fixture quality and hard-negative coverage;
- proof quality for any `exact-proven` claim;
- provenance, license, repository, support contact, and release notes;
- the decision to publish updates or deprecate unsupported claims.

Passing `nose semantic-pack check` means the pack is structurally well-formed and
its declared fixtures are present. It does not mean the pack is safe to enable in
every user's risk model, and it does not let external rows influence query,
normalize, value-graph, exact, or detection consumers.

## User Responsibilities

Users who opt into external packs own the enablement decision. They should inspect:

- provider identity and repository history;
- package/version ranges and unsupported boundaries;
- positive and hard-negative fixtures;
- whether exact-capable contracts are appropriate for their codebase;
- the pack's own test or proof evidence outside nose.

`nose query --semantic-pack` and `[query].semantic-packs` validate local
external pack manifests and query JSON schema v6 reports the active pack set in
top-level `semantic_packs`. Future producer execution must keep the same
provenance and fail-closed behavior before external packs can affect `near` or
exact results.

## See also

- [semantic-pack-extension-api-v0](semantic-pack-extension-api-v0.md)
- [semantic-pack-loading](semantic-pack-loading.md)
- [semantic-kernel](semantic-kernel.md)
