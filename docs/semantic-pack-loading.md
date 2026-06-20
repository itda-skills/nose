# Semantic pack loading

Status: nose can validate local semantic-pack v0 manifests on `nose query`, and
it can run a separate local conformance check for manifests and declared fixture
assets. External packs are explicit opt-ins and are currently `metadata-only`:
they do not emit evidence, open exact contracts, mint fingerprints, approve clone
pairs, or change exact/near query results.

## Local entry points

Use `--semantic-pack <file-or-dir>` on `nose query` to opt into local pack
metadata validation for one run:

```sh
nose query src --format json --semantic-pack semantic-packs/python-math-prod.json
```

Commit stable project opt-ins in `nose.toml`:

```toml
[query]
semantic-packs = ["semantic-packs/python-math-prod.json"]
```

Each path may be a manifest file or a directory. Paths from `[query].semantic-packs`
are resolved relative to the config file that declared them; paths from
`--semantic-pack` are resolved by the shell/current working directory like other
CLI paths. Directory loading reads direct `*.json` children in sorted order; it
does not recurse and it does not contact a registry or network service.

## Conformance entry point

Pack authors and users can check the same local manifest paths without loading
them into an analysis run:

```sh
nose semantic-pack check semantic-packs/python-math-prod.json
nose semantic-pack check semantic-packs --format json
```

The conformance command validates manifest structure, trust policy, dependency
references, exact-capable contract obligations, conformance fixture references,
fixture expectation labels, and fixture file existence. It does not execute
external producers or certify semantic correctness. See
[semantic-pack-conformance](semantic-pack-conformance.md).

## Trust policy

Trust is separate from channel eligibility.

- Compiled builtin packs are enabled by default and are the only packs that
  currently influence evidence and contracts. Current v0 machine output still
  uses compatibility labels such as `compiled-first-party` and
  `default-first-party`; docs and new implementation work should use
  `builtin`. `nose.first_party` is the temporary broad compatibility facade;
  `nose.lang.c` is the first narrow builtin language pack slice for C file
  identity, parser/lowering metadata, and unsigned-cast source provenance;
  `nose.python.builtins.collection_factories` is the first narrow Python
  builtins pack for `list`, `set`, `frozenset`, and `tuple` collection factory
  API occurrence provenance;
  `nose.python.stdlib.collection_factories` owns Python `collections.deque`
  imported binding, alias, and namespace collection factory API occurrence
  provenance;
  `nose.python.stdlib.math` owns Python `math.prod` imported namespace product
  reduction API occurrence provenance;
  `nose.ruby.stdlib.set` owns Ruby `require "set"; Set.new(...)` collection
  factory API occurrence provenance;
  `nose.rust.stdlib.vec` owns Rust `Vec::new` and `vec!` collection factory API
  occurrence provenance;
  `nose.rust.stdlib.option` owns Rust `Some`, `None`, and `and_then` Option API
  occurrence provenance;
  `nose.javascript.builtins.promise` owns JS/TS `Promise.resolve` and `.then`
  Promise API occurrence provenance;
  `nose.javascript.builtins.array` owns JS/TS `Array.from` and
  `Array.isArray` API occurrence provenance;
  `nose.javascript.builtins.collection_constructors` owns JS/TS `new Set(...)`
  and `new Map(...)` API occurrence provenance;
  `nose.rust.stdlib.collection_factories` owns selected Rust
  `std::collections::{HashSet,BTreeSet,VecDeque}::from` collection factory API
  occurrence provenance;
  `nose.rust.stdlib.map_factories` owns selected Rust
  `std::collections::{HashMap,BTreeMap}::from` map factory API occurrence
  provenance;
  `nose.java.stdlib.map_factories` owns Java `java.util.Map.of` and
  `java.util.Map.ofEntries` map factory API occurrence provenance;
  `nose.java.stdlib.map_entries` owns Java `java.util.Map.entry` map-entry API
  occurrence provenance;
  `nose.java.stdlib.collection_factories` owns Java `java.util.List.of`,
  `Set.of`, and `Arrays.asList` collection factory API occurrence provenance;
  `nose.java.stdlib.collection_constructors` owns Java empty `new
  ArrayList<>()` and `new LinkedList<>()` collection constructor API occurrence
  provenance;
  `nose.java.stdlib.static_collection_adapters` owns Java
  `java.util.Arrays.stream` static collection adapter API occurrence provenance;
  `nose.python.stdlib.type_domain` is the first narrow stdlib pilot pack for
  Python `typing`, `collections.abc`, and `asyncio` type-domain aliases;
  `nose.value_graph.laws` is the first LawPack pilot for selected proof-backed
  value-graph law provenance.
- Local external packs require explicit user opt-in through CLI or config.
- Local manifests must declare `trust = "external-opt-in"` and
  `enabled_by_default = false`; manifests that claim builtin trust or default
  enablement are rejected.
- Duplicate pack ids fail the run instead of letting provenance become
  ambiguous.

`nose query --format json` validates configured and CLI-provided semantic-pack
paths before analysis and reports the active builtin/local pack set in the
top-level `semantic_packs` array. Local external packs remain metadata-only
while builtin compiled packs report `evidence-and-contracts` influence.

## Current limits

The loader validates manifest shape and pack provenance only. It does not yet:

- execute external evidence producers;
- register external contract rows with exact consumers;
- execute fixture contents or run a behavioral oracle;
- compare semantic version ranges against the installed nose version beyond
  requiring a parseable declared compatibility field;
- install packs from a registry or remote source.

Future loader work should keep this boundary: external pack claims can become
usable only through dependency-backed evidence records and fail-closed kernel
contracts, never through raw selectors or manifest presence alone.

## See also

- [semantic-pack-extension-api-v0](semantic-pack-extension-api-v0.md)
- [semantic-pack-conformance](semantic-pack-conformance.md)
- [semantic-kernel](semantic-kernel.md)
