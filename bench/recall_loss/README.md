# Recall-loss baselines

This directory keeps compact, reproducible summaries for
`nose verify --recall-loss-report`. The full JSON reports are local artifacts;
the checked-in files record the command, selected surface, hard gate, reason
rollups, and representative fixtures needed to reproduce or review a semantic
kernel PR.

## Regenerate

```sh
cargo run -q -p nose-cli -- verify crates \
  --max-violations 0 \
  --recall-loss-report target/recall-loss.crates.json

cargo run -q -p nose-cli -- verify \
  bench/repos/chi/middleware/content_type.go \
  bench/repos/boltons/boltons/iterutils.py \
  bench/repos/thor/lib/thor/actions.rb \
  bench/repos/radash/src/array.ts \
  bench/repos/hyperfine/src/util/number.rs \
  bench/repos/swift-metrics/Sources/CoreMetrics/Metrics.swift \
  --max-violations 0 \
  --recall-loss-report target/recall-loss.corpus-slice.json
```

Compare two full reports with:

```sh
python3 scripts/recall-loss-diff.py before.json after.json
```

Build a callee-identity census from a full report with:

```sh
python3 scripts/recall-loss-callee-census.py \
  target/recall-loss.crates.json \
  --format json \
  > target/callee-census.json
```

Build the full pinned-corpus priority census with:

```sh
python3 scripts/corpus-priority-census.py \
  --jobs 4 \
  --logs-dir target/corpus-priority-census-full \
  --output target/corpus-priority-census-full.json
```

Use `--scan-only` for a source-prevalence-only dry run, and
`--summarize-only` to rebuild the aggregate from existing per-repo
`--recall-loss-report` files. The source scan is a lexical pricing signal, not
semantic proof.

Full reports also include `import_snapshot_census`, which records successful
imported immutable snapshot counts and unresolved binding-import miss reasons.
Use that section directly when deciding the next import-backed provider-value
slice.

Build the Java `Arrays`/`Collections` partial-coverage audit with:

```sh
python3 scripts/java-arrays-collections-audit.py \
  --output target/java-arrays-collections-audit.v1.json
```

Build the Go `sort`/`slices`/`maps` partial-coverage audit with:

```sh
python3 scripts/go-stdlib-collections-audit.py \
  --output target/go-stdlib-collections-audit.v1.json
```

Build the JS/TS builtin partial-coverage audit with:

```sh
python3 scripts/js-ts-stdlib-partial-audit.py \
  --output target/js-ts-stdlib-partial-audit.v1.json
```

Build the Python HOF/runtime attribution audit with:

```sh
python3 scripts/python-hof-runtime-audit.py \
  --output target/python-hof-runtime-audit.v3.json
```

Build the Rust stdlib partial-coverage audit with:

```sh
python3 scripts/rust-stdlib-partial-audit.py \
  --output target/rust-stdlib-partial-audit.v2.json
```

Build the Swift stdlib partial-coverage audit with:

```sh
python3 scripts/swift-stdlib-partial-audit.py \
  --output target/swift-stdlib-partial-audit.v2.json
```

Build the #594 cross-language scheduling/error/callback boundary census with:

```sh
python3 scripts/cross-language-boundary-census.py \
  --output target/cross-language-boundary-census-594.v1.json
```

## Files

- [crates.baseline.v1.json](crates.baseline.v1.json) records the current
  `crates` baseline and the #570 attribution improvement from the first coarse
  #569 baseline.
- [corpus-slice.baseline.v1.json](corpus-slice.baseline.v1.json) records a
  small mixed-language corpus slice across Go, Python, Ruby, TypeScript, Rust,
  and Swift.
- [stdlib-support-slices-2026-06-28.v1.json](stdlib-support-slices-2026-06-28.v1.json)
  records the staged corpus-priority stdlib support slices, including focused
  fixtures, unsupported boundaries, and per-slice performance checks.
- [java-arrays-collections-audit-2026-06-28.v1.json](java-arrays-collections-audit-2026-06-28.v1.json)
  records method-level corpus prevalence and support/boundary classification
  for Java `Arrays` and `Collections`.
- [go-stdlib-collections-audit-2026-06-28.v1.json](go-stdlib-collections-audit-2026-06-28.v1.json)
  records alias-aware corpus prevalence and support/boundary classification for
  Go `sort`, `slices`, and `maps`.
- [js-ts-stdlib-partial-audit-2026-06-28.v1.json](js-ts-stdlib-partial-audit-2026-06-28.v1.json)
  records comment/string-masked JS/TS builtin prevalence across Array HOFs,
  Map/Set receiver selectors, Object key/reflection APIs, Promise/async
  protocol surfaces, mutation/effect methods, and simple receiver hints. The
  only 5,000+ group is the closed Promise/async protocol boundary.
- [python-hof-runtime-audit-2026-06-28.v1.json](python-hof-runtime-audit-2026-06-28.v1.json)
  records AST-based Python builtin/HOF, `itertools`, and `functools`
  attribution and boundary classification.
- [python-hof-runtime-audit-2026-06-28.v2.json](python-hof-runtime-audit-2026-06-28.v2.json)
  refines the Python HOF/runtime audit with call-shape counts, top repos,
  broader lexical binding collection, and ranked next-work groups for
  materializer domain proof, ordering semantics, HOF callback proof,
  `itertools` lifecycle, callable runtime identity, combinatoric iterators, and
  runtime attribution.
- [python-hof-runtime-audit-2026-06-28.v3.json](python-hof-runtime-audit-2026-06-28.v3.json)
  records the first high-volume candidate processing decision: the
  `8,432`-occurrence Python materializer-domain group is split into strict
  materializer subgroups without widening semantic admission.
- [rust-stdlib-partial-audit-2026-06-28.v1.json](rust-stdlib-partial-audit-2026-06-28.v1.json)
  records comment/string-masked lexical prevalence and support/boundary
  classification for Rust `Option`, `Result`, iterator adapters/HOFs, `Vec`,
  membership/map lookup, `std::collections` factories, mutation, ordering, and
  allocation/lifetime surfaces.
- [rust-stdlib-partial-audit-2026-06-28.v2.json](rust-stdlib-partial-audit-2026-06-28.v2.json)
  aligns the Rust audit with existing generic method/effect contracts and
  records high-volume processing decisions for iterator-domain, Option/Result
  channel, HOF callback, iterator-view lifecycle, and mutation/effect groups.
  Unsupported attribution moves `12,232 -> 3,949` while semantic admission stays
  closed and `sort_by_key` gains receiver-mutation evidence.
- [swift-stdlib-partial-audit-2026-06-28.v1.json](swift-stdlib-partial-audit-2026-06-28.v1.json)
  records comment/string-masked lexical prevalence and support/boundary
  classification for Swift cardinality properties, collection/map factories,
  sequence HOFs, membership, sequence views, mutation/effect, ordering, and
  reduction surfaces.
- [swift-stdlib-partial-audit-2026-06-28.v2.json](swift-stdlib-partial-audit-2026-06-28.v2.json)
  records the high-volume Swift cardinality processing decision: `count` and
  `isEmpty` remain on existing `ExactCollection` receiver-gated property and
  method contracts.
- [cross-language-boundary-census-594-2026-06-28.v1.json](cross-language-boundary-census-594-2026-06-28.v1.json)
  records the #594 starting census for scheduling, success/error channel, and
  callback demand/effect obligations across JS/TS, Python, Rust, Go, Java,
  Swift, Ruby, and C. It combines existing language audits with conservative
  Ruby/C lexical pricing and keeps `semantic_admission_delta = 0`.
- [issue-597-obligation-taxonomy-2026-06-28.v1.json](issue-597-obligation-taxonomy-2026-06-28.v1.json)
  records the first `--recall-loss-report` obligation-family rollup for #597:
  broad admission reasons now also expose diagnostics-only
  `obligation_family` and `obligation_subreason` fields while preserving the
  hard gate.
- [issue-598-hard-negative-inventory-2026-06-28.v1.json](issue-598-hard-negative-inventory-2026-06-28.v1.json)
  maps existing cross-language hard negatives to the #594 obligation vocabulary
  before producer/admission work starts. It considers JS/TS, Python, Rust, Go,
  Java, Swift, Ruby, and C, records no omitted languages, and keeps new exact
  admissions at zero.
- [issue-599-callback-obligation-slice-2026-06-28.v1.json](issue-599-callback-obligation-slice-2026-06-28.v1.json)
  records the first callback demand/effect reporting slice: `18,637`
  callback-shaped source-prevalence occurrences and `30` `crates`
  callback-demand/effect recall-loss rows, with no exact admission changes.
- [callback-demand-effect-diagnostics-2026-06-28.v1.json](callback-demand-effect-diagnostics-2026-06-28.v1.json)
  records the first post-#594 callback diagnostics refinement. The broad
  `hof-demand-effect-profile-missing` rollup stays at `30`, but its
  `by_obligation` subreasons split into callback-effect proof (`27`),
  callback identity/shape proof (`2`), and predicate callback profile (`1`)
  without opening exact admission.
- [callback-demand-effect-diagnostics-2026-06-28.v2.json](callback-demand-effect-diagnostics-2026-06-28.v2.json)
  refines the callback-effect proof bucket into concrete producer obligations:
  callback call effects (`22`), callback assignment effects (`5`),
  callback identity/shape (`2`), and predicate callback profile (`1`).
  Runtime-boundary callback effects are priced at `0` on the current `crates`
  surface, and exact admission remains closed.
- [callback-demand-effect-diagnostics-2026-06-28.v3.json](callback-demand-effect-diagnostics-2026-06-28.v3.json)
  refines callback call-effect proof into producer-facing call shapes: member
  call proof (`10`), Rust macro call proof (`8`), direct-function effect
  contracts (`3`), and imported-function effect contracts (`1`). Assignment
  effects (`5`), callback identity/shape (`2`), and predicate callback profile
  (`1`) remain separate; exact admission still records
  `semantic_admission_delta = 0`.
- [issue-600-channel-scheduling-obligation-slice-2026-06-28.v1.json](issue-600-channel-scheduling-obligation-slice-2026-06-28.v1.json)
  records the first channel/scheduling reporting slice: `95,805`
  channel/scheduling-shaped source-prevalence occurrences and `14` `crates`
  scheduling-boundary recall-loss rows, with Promise/Future/async/channel
  admission still closed.
- [promise-protocol-diagnostics-2026-06-28.v1.json](promise-protocol-diagnostics-2026-06-28.v1.json)
  records the Promise/async diagnostics split. The JS/TS source-prevalence group
  stays at `29,094` occurrences, but runtime-boundary report labels now separate
  await scheduling, async function scheduling, Promise executor callbacks,
  Promise factories, aggregate result channels, rejection channels, and
  non-construct Promise calls without opening exact admission.
- [promise-protocol-hard-negatives-2026-06-28.v1.json](promise-protocol-hard-negatives-2026-06-28.v1.json)
  records the follow-up Promise hard-negative slice. It keeps exact admission
  closed while pinning async-function/sync, Promise executor/sync,
  Promise.resolve/sync, Promise.then/custom receiver, thenable assimilation, and
  Promise.all/Promise.race boundaries with `4` new tests and `8` fail-closed
  assertions.
- [promise-resolve-recovery-2026-06-28.v1.json](promise-resolve-recovery-2026-06-28.v1.json)
  records the first narrow Promise recovery slice. Dependency-closed
  `Promise.resolve(value)` can now enter exact semantic families only when the
  receiver is proven as the global `Promise.resolve` and the argument is a
  non-thenable-safe literal/nullish/scalar value; sync payloads, possible
  thenables, explicit PromiseLike values, custom receivers, executors,
  aggregate channels, and rejection channels remain closed.
- [promise-rejection-continuation-diagnostics-2026-06-28.v1.json](promise-rejection-continuation-diagnostics-2026-06-28.v1.json)
  records a reporting-only Promise rejection split. `Promise.reject`, `.catch`,
  and `.finally` now emit distinct missing-evidence labels for rejected-value
  channels, rejection continuations, settlement continuations, and callback
  demand/effect obligations while exact Promise continuation admission remains
  closed.
- [promise-then-obligation-diagnostics-2026-06-28.v1.json](promise-then-obligation-diagnostics-2026-06-28.v1.json)
  records the reporting-only `.then` split. Selector-only and custom receivers
  now report PromiseLike receiver proof separately from fulfillment
  continuation, rejection continuation, and callback demand/effect labels; exact
  Promise continuation admission remains closed.
- [promise-continuation-report-rows-2026-06-28.v1.json](promise-continuation-report-rows-2026-06-28.v1.json)
  records the focused row-visibility follow-up. `.then`, `.catch`, and
  `.finally` now appear as oracle-interpretable recall-loss
  `admission_rejections`, and the next recovery queue is quantified as
  receiver-first across `68` unhinted `.then`/`.catch` occurrences.
- [promise-local-continuation-recovery-2026-06-29.v1.json](promise-local-continuation-recovery-2026-06-29.v1.json)
  records the first broader local Promise continuation recovery slice.
  First-party `Promise.reject`, `.catch`, and two-argument `.then` now have
  contract evidence, local fulfilled/rejected Promise states are represented in
  the value graph, handler-returned `Promise.resolve` is flattened, and
  `catch` converges with `then(undefined, onRejected)` only for recoverable
  first-party rejected producers. Broad async scheduling, arbitrary thenables,
  `.finally`, aggregate combinators, custom receivers, and sync payload
  equivalence remain closed.
- [promise-receiver-producer-diagnostics-2026-06-29.v1.json](promise-receiver-producer-diagnostics-2026-06-29.v1.json)
  records the follow-up reporting-only receiver-producer split for Promise
  continuations. `new Promise(...).then`, async-function-return `.then`/`.catch`,
  and generic call-return receivers now have separate missing-evidence labels;
  a 120-repo JS/TS source scan found `835` generic call-return receivers,
  `49` same-file async-function call receivers, and only `2` constructor
  receivers, so exact admission remains closed and the next recovery target is
  call-return producer attribution rather than constructor semantics.
- [promise-call-return-callee-diagnostics-2026-06-29.v1.json](promise-call-return-callee-diagnostics-2026-06-29.v1.json)
  records the next reporting-only split inside generic Promise call-return
  receivers. Missing evidence now distinguishes member, local/parameter,
  imported binding/member, known-target return-domain, and unknown callee
  shapes. The revised 120-repo JS/TS scan found `932` member call-return
  candidates, `184` local/parameter candidates, `105` imported-member
  candidates, and `73` imported-binding candidates, so exact admission remains
  closed until callee identity and returned `PromiseLike` domain proof are both
  explicit.
- [promise-async-function-return-recovery-2026-06-29.v1.json](promise-async-function-return-recovery-2026-06-29.v1.json)
  records the first same-file async-function producer recovery slice. Direct
  calls to source-proven async functions now emit `PromiseLike` result-domain
  evidence, and pure non-thenable-safe returned payloads can feed local `.then`
  fulfillment recovery while preserving the Promise boundary. Await,
  throw/rejection, possible thenables, opaque call results, constructor
  receivers, imported/member call returns, `.finally`, and aggregate
  combinators remain closed.
- [promise-direct-function-return-recovery-2026-06-29.v1.json](promise-direct-function-return-recovery-2026-06-29.v1.json)
  records the direct-function subset of the local/parameter Promise call-return
  queue. Same-file direct calls now receive `Domain(PromiseLike)` result
  evidence when the target is a non-async single-return function whose returned
  expression already has PromiseLike domain proof. Literal and typed
  non-thenable `Promise.resolve` returns, plus `Promise.reject` returns, can
  feed local `.then`/`.catch` recovery while preserving the Promise boundary.
  Parameter callees, member/imported call returns, unsafe thenables,
  constructors, `.finally`, and aggregate/scheduling paths remain closed.
- [promise-direct-method-return-recovery-2026-06-29.v1.json](promise-direct-method-return-recovery-2026-06-29.v1.json)
  records the proof-backed DirectMethod subset of the member Promise
  call-return queue. Existing DirectMethod target evidence plus returned
  expression `Domain(PromiseLike)` evidence now derives PromiseLike call-result
  proof for non-async single-return methods. Value-graph recovery evaluates only
  the returned expression, closes when it reads receiver context, and preserves
  the Promise boundary; selector-only member calls, dynamic dispatch, imported
  members, unsafe thenables, `.finally`, constructors, and aggregate/scheduling
  paths remain closed.
- [promise-imported-call-return-boundary-2026-06-29.v1.json](promise-imported-call-return-boundary-2026-06-29.v1.json)
  records the reporting-only imported function/member Promise call-return
  boundary. Target-present imported receivers now report missing settled-value
  contracts instead of return-domain proof, because imported call-target
  identity has no local body that can recover fulfilled or rejected payloads.
  The `105` imported-member and `73` imported-binding source candidates remain
  closed behind source-level hard negatives.
- [issue-601-first-slice-closeout-2026-06-28.v1.json](issue-601-first-slice-closeout-2026-06-28.v1.json)
  records the #601 decision to close the first exact-admission slice as a
  quantified closed boundary instead of forcing unsafe async/callback/channel
  admission.
- [issue-594-closeout-2026-06-28.v1.json](issue-594-closeout-2026-06-28.v1.json)
  records the #594 epic closeout: every child issue's artifact, quantitative
  summary, validation commands, remaining closed boundaries, and hard-gate
  status.
- [issue-570-cycles.v1.json](issue-570-cycles.v1.json) records the five focused
  top-bucket cycles and the explicit unsupported/fail-closed boundary decision.
- [issue-572-cycle.v1.json](issue-572-cycle.v1.json) records the first
  post-#570 refinement cycle: expression-statement effect boundaries and Rust
  macro surfaces are split out of the callee-identity bucket while preserving
  the hard gate.
- [issue-574-callee-census.v1.json](issue-574-callee-census.v1.json) records
  the #567/#574 census of the remaining callee-identity bucket by language and
  call-target surface.
- [issue-576-cycle.v1.json](issue-576-cycle.v1.json) records the first recovery
  slice after the census: Rust brace `use` imports now feed dependency-backed
  imported call-target evidence while wildcard/nested/relative brace imports
  stay closed.
- [issue-578-cycle.v1.json](issue-578-cycle.v1.json) records the next Rust
  scoped-path recovery slice: imported roots such as `Span::new` now feed
  dependency-backed imported member call-target evidence while raw `crate`,
  `std`, unimported, and ambiguous roots stay closed.
- [issue-580-cycle.v1.json](issue-580-cycle.v1.json) records the Rust
  struct-expression surface slice: Rust struct literals now carry exact-safe
  `SequenceSurface` evidence, recovering the imported-member target-present
  follow-ups from #578 while keeping untagged sequences and collection/map
  contracts separate.
- [issue-567-phase1-js-ts-constructors.v1.json](issue-567-phase1-js-ts-constructors.v1.json)
  records imported immutable provider snapshots for JS/TS `new Map(...)` and
  `new Set(...)`.
- [issue-567-phase2-collection-factories.v1.json](issue-567-phase2-collection-factories.v1.json)
  records imported immutable provider snapshots for existing Python and Java
  collection factory contracts.
- [issue-567-phase3-import-snapshot-census.v1.json](issue-567-phase3-import-snapshot-census.v1.json)
  records the report-infrastructure closeout: recall-loss artifacts now include
  import snapshot success counts and binding-import miss reasons.
- [issue-567-phase4-aggregate-boundary-triage.v1.json](issue-567-phase4-aggregate-boundary-triage.v1.json)
  records the first census-driven triage pass: the broad provider-aggregate
  miss bucket is split into non-import-literal sequence surfaces and child
  reference boundaries without admitting new snapshots.
- [issue-567-closeout.v1.json](issue-567-closeout.v1.json) records the epic
  closeout audit: requirement coverage, admitted imported-coordinate families,
  hard-negative inventory, false-merge hard gates, and product/runtime evidence.
- [issue-587-module-export-census.v1.json](issue-587-module-export-census.v1.json)
  records the #587 starting census for the next import-snapshot milestone:
  module/export miss counts by reason, crate, import surface, top files, and
  recommended implementation order.
- [issue-587-module-resolution-1-3.v1.json](issue-587-module-resolution-1-3.v1.json)
  records the first #587 implementation slice: Rust crate/module identity,
  `self::`/`super::` provider lookup, and boundary splitting for callable,
  type, module namespace, stdlib, and workspace-crate imports.
- [issue-587-reexport-pricing.v1.json](issue-587-reexport-pricing.v1.json)
  records the direct Rust public re-export slice: one-hop `pub use` lookup for
  already literal-safe provider exports, bare child module aliases, and
  re-export-specific boundary reasons.
- [issue-587-residual-census.v1.json](issue-587-residual-census.v1.json)
  records the post-re-export #587 residual census: the remaining generic
  module/export misses are mostly external crate imports, with a smaller
  relative-`super` and local export tail for follow-up triage.
- [issue-587-residual-boundary-split.v1.json](issue-587-residual-boundary-split.v1.json)
  records the #587 residual boundary split: external crate, residual stdlib, and
  residual workspace-crate imports are priced as explicit closed boundaries
  instead of generic provider-module misses.
- [issue-587-relative-super-closeout.v1.json](issue-587-relative-super-closeout.v1.json)
  records the #587 relative-super closeout: imports that name the parent module
  itself now resolve to callable/type boundaries, moving
  `provider-module-missing` `11 -> 0` without admitting new snapshots.
- [post-587-census.v1.json](post-587-census.v1.json) records the current
  post-#587 `crates` recall-loss census: module/provider-missing import
  snapshot work is complete on the checked surface, and the next largest
  capability buckets are receiver-domain proof, callee identity, and
  mutation/effect contracts.
- [corpus-priority-census-2026-06-28.v1.json](corpus-priority-census-2026-06-28.v1.json)
  records the first full 120-repo corpus priority census after #587, combining
  per-repo recall-loss reports with lexical stdlib/API source prevalence to
  choose the next semantic-kernel capability slices from corpus evidence.
