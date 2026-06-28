# Scheduling, channel, and callback obligations

Status: planning and reporting contract for issue
[#594](https://github.com/corca-ai/nose/issues/594).

This page defines the minimal cross-language obligation vocabulary for
scheduling boundaries, success/error channels, and callback demand/effect
surfaces. It is deliberately a capability vocabulary, not a feature list. New
language or stdlib support should map evidence and contracts onto these
obligations instead of adding selector-specific admission paths.

## Census

The starting evidence is [cross-language-boundary-census-594-2026-06-28.v1.json](../bench/recall_loss/cross-language-boundary-census-594-2026-06-28.v1.json).
It normalizes the existing JS/TS, Python, Rust, Go, Java, and Swift audit
reports and adds conservative Ruby/C lexical pricing. The report is
source-prevalence evidence only; it records `semantic_admission_delta = 0`.

Top obligation families in the census:

| family | occurrences | policy |
|---|---:|---|
| success/error result channel | 62,945 | already large because Rust `Option`/`Result` constructors are supported, but helper/default/callback forms still need receiver and channel proof |
| lifecycle/materialization boundary | 42,010 | iterators, views, factories, allocation, and materializers need source/receiver/domain proof before exact use |
| scheduling boundary | 28,751 | dominated by JS/TS `await` and async functions; keep closed until scheduling and channel obligations are explicit |
| ambiguous selector boundary | 19,116 | selector/property spelling needs receiver, symbol, domain, and occurrence proof |
| receiver mutation | 16,530 | mutation is effect evidence, not value equivalence |
| callback families combined | 18,637 | useful next producer target because it reuses existing HOF demand/effect contracts across languages |

The census recommendation is to design and report obligations first, then start
with callback demand/effect reporting before any broad async or channel
admission.

The first reporting slices are:

- [issue-599-callback-obligation-slice-2026-06-28.v1.json](../bench/recall_loss/issue-599-callback-obligation-slice-2026-06-28.v1.json) for callback demand/effect rows;
- [issue-600-channel-scheduling-obligation-slice-2026-06-28.v1.json](../bench/recall_loss/issue-600-channel-scheduling-obligation-slice-2026-06-28.v1.json) for channel and scheduling-boundary rows.

The first exact-admission decision is [issue-601-first-slice-closeout-2026-06-28.v1.json](../bench/recall_loss/issue-601-first-slice-closeout-2026-06-28.v1.json):
no slice is opened in this milestone because the measured candidates still lack
the complete callback, channel, scheduling, or aggregate-result obligations
needed for exact convergence.

The first follow-up diagnostics artifact is [callback-demand-effect-diagnostics-2026-06-28.v1.json](../bench/recall_loss/callback-demand-effect-diagnostics-2026-06-28.v1.json).
It keeps exact admission closed but splits the local `crates` HOF
demand/effect rows by callback-effect proof, callback identity/shape proof, and
predicate callback profile.

The follow-up [callback-demand-effect-diagnostics-2026-06-28.v2.json](../bench/recall_loss/callback-demand-effect-diagnostics-2026-06-28.v2.json) splits callback-effect proof further into callback call effects, callback assignment effects, and callback runtime boundaries.

The next [callback-demand-effect-diagnostics-2026-06-28.v3.json](../bench/recall_loss/callback-demand-effect-diagnostics-2026-06-28.v3.json)
keeps the same exact-admission boundary and splits callback call effects by
producer-facing call shape: member calls (`10`), Rust macro calls (`8`),
direct-function effect contracts (`3`), and imported-function effect contracts
(`1`) on `crates`.

The [promise-protocol-diagnostics-2026-06-28.v1.json](../bench/recall_loss/promise-protocol-diagnostics-2026-06-28.v1.json)
slice keeps Promise exact admission closed while making the JS/TS Promise/async
source-prevalence group (`29,094` occurrences) reportable as scheduling,
executor callback, rejection-channel, aggregate-result, factory, and
non-construct call obligations. JS/TS async functions now emit a fail-closed
`Source::Protocol(AsyncFunction)` boundary even when the body has no `await`.
The follow-up [promise-protocol-hard-negatives-2026-06-28.v1.json](../bench/recall_loss/promise-protocol-hard-negatives-2026-06-28.v1.json)
pins the Promise-specific hard negatives before any recovery slice opens:
async-function/sync, Promise executor/sync, Promise.resolve/sync,
Promise.then/custom receiver, possible or explicit thenable assimilation, and
Promise.all/Promise.race convergence all remain closed.
The first narrow recovery slice, [promise-resolve-recovery-2026-06-28.v1.json](../bench/recall_loss/promise-resolve-recovery-2026-06-28.v1.json),
lets dependency-closed `Promise.resolve(value)` enter exact semantic matching only
when `value` is proven non-thenable-safe, so the Promise factory boundary is
retained and broader scheduling, executor, aggregate, and rejection channels stay
closed.
The follow-up [promise-rejection-continuation-diagnostics-2026-06-28.v1.json](../bench/recall_loss/promise-rejection-continuation-diagnostics-2026-06-28.v1.json)
is reporting-only: `Promise.reject`, `.catch`, and `.finally` now have separate
missing-evidence labels for rejected-value channels, rejection continuations,
settlement continuations, and callback demand/effect obligations. It does not
admit Promise continuation equivalence.

## Minimal Vocabulary

| obligation | existing substrate | exact-channel rule |
|---|---|---|
| synchronous callback | `DemandEffectProfile` per-element/eager profiles, `LibraryApi` HOF rows, `CallTarget` when needed | callback identity, receiver/domain proof, arity, result role, and effect visibility must be explicit |
| lazy callback | pull-lazy HOF profiles, iterator producer/source proof, terminal materialization proof | construction must not expose callback effects or exceptions unless a terminal demand proves observation |
| eager callback | eager HOF profiles and source comprehension facts | callback errors/effects may be visible only when the API occurrence and receiver/source evidence prove eager demand |
| effect-only callback | HOF/effect contracts plus `Effect` evidence | ignored callback results must not be consumed as value equivalence; observable effects must stay ordered |
| reduction callback | fold/reduction demand profiles | accumulator identity, callback result role, initial value, and effect order must be represented |
| executor callback | source/API occurrence plus async/protocol demand profile | executor timing and thrown/rejected outcomes must be represented before any producer/factory convergence |
| success/error result channel | `Domain(Option/Result/FutureLike/PromiseLike)`, constructor/predicate rows, default contracts | success, empty, default, error, panic, and rejection channels must remain distinct |
| exception channel | `Source::Protocol`, static-error control, effect-free throw checks | thrown/rescued/non-local control must not be collapsed into ordinary return values |
| rejection channel | Promise/Future-like contracts and async demand profiles | rejected values, catch continuations, finally settlement, aggregate rejection, and thenable assimilation stay closed until proven |
| scheduling boundary | `DemandOperation::AsyncContinuation`, `GeneratorSuspension`, `ChannelOperation`, `ProtocolBoundary` | task/thread/goroutine/microtask timing is not synchronous equivalence proof |
| cancellation/early exit | short-circuit demand profiles and future protocol facts | cancellation, stop, break, first-settled, and early-exit behavior must be explicit |
| lifecycle/materialization | `SequenceSurface`, `Domain`, iterator adapter/materializer rows | one-shot views, reusable collections, type-directed materializers, and allocation/lifetime are separate |
| receiver mutation | `Effect(ReceiverMutation)`, place/effect contracts | mutation can close later exact receiver use; it does not create pure value equality |
| ambiguous selector | `Symbol`, `Import`, `CallTarget`, `Domain`, `LibraryApi` occurrence proof | a method/property/function name is only a selector until all required evidence is dependency-closed |

## Existing Mapping

- `DemandEffectProfile` already carries eager, fold, short-circuit, lazy HOF,
  async continuation, generator, channel, and protocol-boundary timing. This is
  the contract side of the vocabulary.
- `Source` facts anchor syntax/protocol distinctions such as await, async
  functions, yield, casts, calls, comprehensions, ranges, and patterns. They do
  not approve exact clones by themselves.
- `Effect` facts cover builder append, binding writes, receiver mutation, fixed
  self-field writes, non-overloadable index writes, and opaque argument escape
  risks. They are used to close unsafe paths or prove narrow place/effect
  contracts.
- `LibraryApiContract` rows own admitted API occurrences and attach result,
  receiver, source, shadowing, demand, and effect obligations.
- `CallTarget` and `Symbol` facts prove identity. They are required before a raw
  callee or imported/member selector can mean a stable operation.
- `Domain` and `SequenceSurface` facts prove receiver/result shape. They prevent
  selector-only collection, map, iterator, option/result, Promise/Future, and
  materializer admission.

This mapping means #594 does not need a new public pack API first. The next code
work should improve reporting and producers that attach these existing concepts
to specific obligation buckets.

## Language Mapping

| language | current #594 surfaces | first safe direction |
|---|---|---|
| JS/TS | `await`, async functions, Promise executor/combinators/rejection, Array HOFs, mutations | report scheduling/rejection/executor separately; keep broad async convergence closed |
| Python | builtins `map`/`filter`, `itertools`, `functools`, decorators, materializers | callback/lifecycle reporting, then narrow producer evidence for already admitted iterator builtins |
| Rust | iterator HOFs, `Option`/`Result`, mutation/effect, iterator views | reuse lazy callback and channel vocabulary; keep type-directed `collect` and mutating APIs closed |
| Go | `sort`/`slices`/`maps`, mutation callbacks, channel/goroutine surfaces for future scans | add effect/callback reporting before exact sort or goroutine/channel semantics |
| Java | `Arrays`/`Collections`, Optional/Future/Stream-shaped domains, mutation/wrapper APIs | split receiver mutation, wrapper aliasing, channel, and stream callback obligations |
| Swift | Sequence HOFs, cardinality, mutation, views, reductions, `throws`/`async` future work | reuse callback/effect and lifecycle buckets; keep selector-only collection methods closed |
| Ruby | Enumerable blocks, `raise`/`rescue`, Thread/Fiber surfaces | block timing and exception-channel reporting before expanding Enumerable support |
| C | callback comparators, allocation/lifetime, memory mutation, `errno`, non-local jumps, threads | keep pointer/lifetime and mutation separate from callback/error-channel evidence |

## Hard-Negative Classes

Every behavior-changing leaf under #594 must include adjacent hard negatives for
the relevant families:

- synchronous callback vs deferred/asynchronous callback;
- callback invoked zero, one, many, or unknown times;
- callback result consumed vs ignored;
- callback side effect visible before vs after the surrounding expression;
- success value vs empty/default/error/exception/rejection channel;
- aggregate success vs first-success, first-error, or first-settled behavior;
- cancellation, early exit, throw, panic-like, or non-local jump;
- receiver mutation vs pure factory/materializer/view;
- one-shot iterator/view/stream reuse vs reusable collection;
- shadowed, ambiguous, wrong-language, or dependency-broken selector evidence.

The current inventory is checked in as [issue-598-hard-negative-inventory-2026-06-28.v1.json](../bench/recall_loss/issue-598-hard-negative-inventory-2026-06-28.v1.json).
It maps existing tests and checked-in audit reports to these families before any
new exact admission is opened.

## Non-API Statement

This page does not add a public semantic-pack API. The vocabulary should narrow
future pack schema names only after the internal reporting and producer paths
are proven. Until then, packs continue to emit facts and contracts; the kernel
continues to decide whether dependency-closed obligations are sufficient for
exact admission.

## See Also

- [demand-effect-semantics](demand-effect-semantics.md) explains the existing demand/effect substrate that #594 reuses.
- [evidence-records](evidence-records.md) defines the evidence rows that carry proof into the kernel.
- [recall-loss-recovery-loop](recall-loss-recovery-loop.md) records the measured recovery process and checked-in baselines.
- [semantic-pack-architecture](semantic-pack-architecture.md) sets the pack/kernel responsibility boundary for exact admission.
- [semantic-kernel-roadmap](semantic-kernel-roadmap.md) tracks remaining demand-aware semantic work.
