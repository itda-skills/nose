# Type-4 Coevolution Handoff

Date: 2026-06-05
Updated: 2026-06-05, paused after semantic performance-enabling work

This records where the adversarial Type-4 coevolution work stopped and how to resume it.
The work was intentionally paused after a real-repository evaluation pass. Do not start
another autonomous frontier loop unless the user explicitly resumes it.

## Current State

- Branch: `main`.
- Last handoff/documentation commit: `d237b21 docs(type4): sharpen coevolution resume notes`.
- Last committed detector/frontier change: `0b63ad9 feat(type4): prove python module membership`.
- The strict frontier loop itself is still paused after the Python module membership batch.
  No new semantic frontier axis has been accepted after loop 406.
- A separate performance-enabling pass was started after that pause and is currently
  uncommitted. It changes:
  - `crates/nose-detect/src/units.rs`;
  - `crates/nose-normalize/src/lib.rs`;
  - `crates/nose-normalize/src/value_graph.rs`.
- Installed baseline used for real-repo comparison: `/opt/homebrew/bin/nose`, version `0.2.0`.
- Current candidate used for comparison: `target/release/nose`, version `0.4.0`.
- Worktree at this pause: dirty with the three uncommitted performance files above and
  this handoff update, plus the pre-existing untracked `.claude/` directory.

The exact stop line is:

```text
loop 406 completed -> generated gates clean -> real-repo sample compared ->
performance pass opened -> core hotspots fixed -> 105-repo sweep exposed raylib/sympy/sqlite ->
shared subtree hash cache implemented -> large AC expression fast path implemented ->
iterative AC flatten implemented -> weakly-justified flatten cache removed ->
shadowed callback collection recursion fixed -> final validation and representative timing rerun ->
pause before commit or next hotspot pass
```

There is no half-implemented semantic frontier to finish. There is, however, an
uncommitted performance patch that supports the coevolution workflow by making real-repo
semantic audits cheaper. Do not start a new frontier loop until this patch is either
finished and committed, or deliberately shelved.

Current uncommitted code state:

- `cargo fmt` and `cargo build --release -p nose-cli` passed after the latest code edits.
- `cargo test -p nose-normalize`, `cargo test -p nose-detect`, `cargo test -p nose-cli`,
  and `GATE=core CROSS=all NOSE=target/release/nose ./scripts/type4-smoke.sh` passed
  after the latest logic change.
- The final code output checks performed so far were byte-identical for the profiled
  baseline repos listed below.
- The briefly added `flatten_cache` in `value_graph.rs` was removed because SQLite
  improved from the iterative flatten rewrite itself, while the extra cache did not
  materially improve the measured scan.
- A `prettier` stack overflow was fixed by rejecting local collection initializer
  inlining when the initializer RHS contains the same canonical id. In the current flat
  cid model this is the conservative strict proof boundary: the same id may be a true
  self-reference or a shadowed nested callback parameter, and this helper cannot
  distinguish those scopes safely.

## Current Performance Pass

This pass was started because real-repo semantic scans became too slow for the intended
adversarial loop cadence. The goal is not a new Type-4 proof rule; it is to make future
real-repo audits cheap enough to run inside the loop.

Baseline release timings before the performance changes:

| repo | wall time | `normalize+extract` | semantic families |
|---|---:|---:|---:|
| `bench/repos/zod` | 6.748s | 6207ms | 9 |
| `bench/repos/zstd` | 5.679s | 5574ms | 45 |
| `bench/repos/regex` | 1.024s | 945ms | 22 |
| `bench/repos/sqlalchemy` | 57.693s | 57345ms | 447 |
| `bench/repos/clap` | >120s timeout | not completed | not completed |

Nine performance/design changes are currently present in the worktree:

1. Numeric-method recognition now checks the method shape before recursively evaluating
   the receiver. This fixed a pathological fluent-chain case in `clap_builder`:

   ```text
   bench/repos/clap/clap_builder/src/builder/arg.rs
   before: >45s timeout for one file
   after:  0.67s, normalize+extract 74ms
   full clap scan after: 1.78s, 6 families
   ```

   The root cause was `eval_proven_numeric_method_call` evaluating receivers for
   non-numeric chains like `.field(...).field(...)` before confirming that the method was
   one of `abs`, `min`, or `max`.

2. Value fingerprinting now has a file-level context that reuses module seed facts and
   function-binding hashes across all units from the same file. This avoids repeating
   top-level assignment scans, mutation scans, unit-symbol checks, and safe-function hash
   collection for every block unit.

3. Loop reduction recognition now uses a per-loop `ReductionCache` for both reduction
   classification and value-DAG reference checks. This fixed the remaining `zstd`
   bottleneck, where a single large C CLI parser caused repeated recursive
   `as_reduction` and `references` walks while most rayon workers were idle.

4. `StrictFacts::collect_immutable_bindings` now computes top-level assignment counts,
   candidate module names, top-level membership, and mutated bindings once per file
   instead of rescanning every node for every candidate binding. This preserves the old
   direct-receiver mutation semantics:

   - `Append(receiver, value)` marks only a direct receiver symbol as mutated;
   - mutating field calls mark only a direct receiver symbol as mutated;
   - non-top-level assignment left-hand sides still use recursive symbol containment.

5. Module value seeding now uses a per-file `ModuleSeedContext` and seeds only bindings
   required by the current unit plus the dependency closure of those bindings. This keeps
   module/global constant capture strict, while avoiding repeated evaluation of unrelated
   top-level tables for every block unit.

6. Shared structural subtree hashes are now available through `ValueFingerprintContext`.
   Contextual per-unit builders use a shared `OnceLock<Vec<u64>>` instead of recomputing
   `crate::subtree_hashes(il, interner)` for every unit in the same file. This was the
   direct follow-up from the raylib profile.

7. Very large associative/commutative source expressions now have a fast path. For
   same-operator binary expression trees with at least 64 operands, the builder collects
   source operands first, evaluates leaves once, canonicalizes once, and rebuilds one
   canonical AC chain. This targets generated or symbolic algebra style expressions
   without changing the small-expression path.

8. AC value flattening is now iterative instead of recursive and no longer clones each
   nested node's argument vector while flattening. A per-builder `flatten_cache` was tried
   and then removed because it added state without a measured benefit.

9. Local collection binding proof now refuses to inline an initializer whose RHS contains
   the same cid. This fixed a stack overflow on Prettier's
   `scripts/clean-cspell.js`, where a callback parameter shadowed the outer `words`
   binding and `words.includes(...)` recursively re-entered the initializer proof. A
   focused CLI regression test was added:
   `scan_mode_semantic_handles_shadowed_callback_collection_name`.

Representative measured state after the first three core fixes:

| repo | baseline `normalize+extract` | final measured `normalize+extract` | output check |
|---|---:|---:|---|
| `bench/repos/zod` | 6207ms | 177ms | byte-identical |
| `bench/repos/zstd` | 5574ms | 74-127ms | byte-identical |
| `bench/repos/regex` | 945ms | 52ms | byte-identical |
| `bench/repos/sqlalchemy` | 57345ms | 572-839ms | byte-identical |
| `bench/repos/clap` | >120s timeout before the first fix | 27-33ms after all fixes | byte-identical to the post-timeout baseline |

Latest measured state after the performance continuation:

| repo | latest measured `normalize+extract` | wall time | output check | note |
|---|---:|---:|---|---|
| `bench/repos/raylib` | 1787-1851ms | 2.55-2.79s | byte-identical | shared subtree cache fixed the distributed per-unit cost |
| `bench/repos/sqlalchemy` | 146.7ms | 0.44s | byte-identical | remained stable after shared context work |
| `bench/repos/sympy` | 1214.0ms | 2.16s | byte-identical | large AC fast path fixed `eqs_165x165` style formulas |
| `bench/repos/sqlite` | 3093.6ms | 3.94s | byte-identical | iterative flatten helped; cache later removed |
| `bench/repos/netty` | 225.2ms | 0.81s | byte-identical | large AC path removed the main outlier cost |
| `bench/repos/zstd` | 88.5ms | not recorded here | byte-identical | no longer a priority target |
| `bench/repos/clap` | 34.2ms | not recorded here | byte-identical | keep as a regression target |
| `bench/repos/zod` | 41.3ms | not recorded here | byte-identical | no longer a priority target |
| `bench/repos/regex` | 54.0ms | not recorded here | byte-identical | no longer a priority target |

Final rerun after removing `flatten_cache` and fixing the Prettier stack overflow:

| repo | `normalize+extract` | wall time | output check | note |
|---|---:|---:|---|---|
| `bench/repos/zod` | 77.4ms | 0.63s | byte-identical | JS/TS guard regression target |
| `bench/repos/raylib` | 1835.7ms | 2.91s | byte-identical | still useful as many-unit regression target |
| `bench/repos/sqlite` | 3416.0ms | 4.16s | byte-identical | still the best next hotspot |
| `bench/repos/prettier` | 47.6ms | 1.06s | previously crashed | stack overflow fixed |

The original `sqlalchemy` output mismatch is resolved. The cause was semantic drift in the
new file-level mutation summary: it recursively collected symbols from `Append` and
mutating-field receivers, while the old proof checked only direct receiver symbols for
those cases.

The broader 105-repo sweep after the core fixes wrote temporary artifacts under
`/tmp/nose-perf-sweep/`. The top outliers from that sweep, and their current status, are:

| repo | wall time | semantic families | status |
|---|---:|---:|---|
| `raylib` | 83.88s | 223 | reduced to about 2.6-2.8s; final representative rerun pending |
| `sympy` | 6.57s | 502 | reduced to 2.16s with large AC fast path |
| `sqlite` | 4.59s | 74 | reduced to about 4.16s wall / 3416ms normalize; still the best next hotspot |
| `vim` | 2.28s | 19 | reduced to about 1.85s wall / 1545ms normalize |
| `nats-server` | 1.91s | 48 | reduced to about 0.47s wall / 226ms normalize |
| `netty` | 1.76s | 182 | reduced to 0.81s with large AC fast path |
| `sqlalchemy` | 0.91s | 447 | reduced to 0.44s; acceptable after fixes |

`prettier` originally failed during the sweep with `rc=134` and:

```text
thread '<unknown>' (...) has overflowed its stack
fatal runtime error: stack overflow, aborting
```

That failure is now fixed. It narrowed to one file:
`bench/repos/prettier/scripts/clean-cspell.js`. The trigger was a shadowed callback
parameter named `words` inside the initializer for an outer `words` collection. The local
collection binding proof tried to inline that initializer while proving
`words.includes(...)`, re-entering the same initializer recursively. The fix is the
conservative self-cid RHS guard described above. Full Prettier semantic scan now
completes in about 1.06s wall time.

Raylib investigation after the sweep:

| state | wall time | output check |
|---|---:|---|
| after core fixes, before raylib-specific work | 83.88s | baseline for this phase |
| after selective module binding seeding | 61.75s | byte-identical to `/tmp/nose-perf-sweep/raylib.full.json` |
| after `StrictFacts` mutation-summary rewrite | 5.11s | byte-identical to `/tmp/nose-perf-sweep/raylib.full.json` |
| after shared subtree-hash cache | 2.79s | byte-identical to `/tmp/nose-perf-sweep/raylib.full.json` |
| after large AC fast path | 2.55s | byte-identical to previous current output |

`NOSE_TIME=1` on raylib after the `StrictFacts` rewrite showed:

```text
discover 4.7ms
parse+lower 307ms
lower 311.9ms
normalize+extract 4077.7ms
candidates 11ms
```

`NOSE_TIME_UNITS=1` produced no individual unit over the 10ms reporting threshold, so the
remaining raylib cost looks distributed across many units rather than one giant unit.
The latest sample, `/tmp/nose-raylib-current.sample.txt`, showed repeated
`nose_normalize::commutative::subtree_hashes` calls from
`Builder::build_unit_with_context` / `process_stmt` / `eval`.

The shared subtree-hash patch was implemented as a lazy `OnceLock<Vec<u64>>`, not as an
eager vector. That was the right shape: files that never need structural subtree hashes
still do not pay the whole-file pass, while contextual builders for large files share the
cost when `Raw` or `Lambda` paths need it.

Additional outlier notes:

- `sympy` was dominated by a giant generated arithmetic expression:
  `eqs_165x165` spent about 5889ms in value extraction before the large-AC fast path.
  After the patch, the full repo `normalize+extract` time was 1214ms.
- `sqlite` is now dominated by repeated extraction around `walChecksumBytes` and nested
  blocks in the same region. Iterative AC flatten reduced the repo from 4263.6ms to about
  3100ms, but the next improvement likely needs a cleaner answer to repeated nested-block
  extraction or value interning cost.
- `netty` improved from 1421.4ms `normalize+extract` after subtree-cache work to 225.2ms
  after the large-AC path.
- `vim` currently finishes in about 1.85s wall time, with `normalize+extract` at
  1545.2ms. Unit timing shows a few visible C functions (`json_decode_item`,
  `reg_equi_class`, `nfa_emit_equi_class`), but much of the remaining cost is
  distributed rather than one single pathological unit.
- `nats-server` is no longer a priority after the latest rerun: about 0.47s wall time and
  226.2ms `normalize+extract`.

Hotspots found and improved in this pass:

- non-numeric fluent chains were evaluated by the numeric method recognizer before method
  filtering;
- top-level module statement lists were rebuilt once per unit;
- module assignment counts were recomputed once per unit;
- stable module binding mutation scans were repeated per candidate and per unit;
- direct mutating-method classification and interner resolution were repeated inside
  those mutation scans;
- unit-name shadow checks were repeated per module assignment and per unit;
- safe function-binding proofs were repeated once per unit;
- literal-sensitive function subtree hashes were rebuilt once per unit;
- ordinary structural subtree hashes were rebuilt once per unit in some `Raw` and
  `Lambda` paths; this was fixed with shared file-level subtree hashes;
- giant generated associative/commutative expressions repeatedly flattened and rebuilt
  nested pairs; this was fixed with a large-AC expression fast path;
- recursive AC flattening cloned nested argument vectors; this was reduced by iterative
  flattening;
- local collection binding proof could recursively re-enter an initializer when callback
  parameters shadowed the outer collection cid; this is fixed by refusing self-cid RHS
  inlining;
- reduction recognition recomputed the same `(value, loopv)` classifications inside
  nested `Phi` branches;
- reduction recognition repeatedly walked the same value DAGs for `references(value,
  loopv)`;
- one pathological unit caused practical rayon underuse: most workers idled while a
  single large function monopolized value extraction.

The hidden `NOSE_TIME_UNITS=1` instrumentation is intentionally kept as a small
`UnitTimer` helper in `units.rs`. It matches the existing `NOSE_TIME` convention and is
useful for future profiling without changing normal output.

## Resume The Performance Pass

If the next session continues performance work, resume here before starting any new
Type-4 frontier:

1. Inspect the dirty worktree:

   ```text
   git status --short
   git diff -- crates/nose-detect/src/units.rs crates/nose-normalize/src/lib.rs crates/nose-normalize/src/value_graph.rs
   ```

2. The current patch has good measured wins and has passed the core validation set. Before
   committing, inspect the final diff for accidental breadth and decide whether to include
   the CLI regression test in the same commit or split it with the stack-overflow fix.

3. If profiling further, start from a fresh release build:

   ```text
   cargo fmt
   cargo build --release -p nose-cli
   ```

4. Treat SQLite as the next performance target.

   Removing `flatten_cache` kept SQLite output byte-identical and retained most of the
   iterative-flatten win (`normalize+extract` about 3.2-3.4s). The next improvement likely
   needs a design answer for repeated nested-block extraction around `walChecksumBytes`,
   not another local cache.

5. Re-run the before/after semantic output checks after that cleanup decision. At minimum:

   ```text
   NOSE_TIME=1 target/release/nose scan bench/repos/sqlalchemy --mode semantic --format json --top 0 > /tmp/nose-perf-baseline/sqlalchemy.final.full.json
   cmp /tmp/nose-perf-baseline/sqlalchemy.after1.full.json /tmp/nose-perf-baseline/sqlalchemy.final.full.json
   ```

   Also compare raylib against the sweep baseline:

   ```text
   NOSE_TIME=1 target/release/nose scan bench/repos/raylib --mode semantic --format json --top 0 > /tmp/nose-perf-sweep/raylib.final.json 2> /tmp/nose-perf-sweep/raylib.final.err
   cmp /tmp/nose-perf-sweep/raylib.full.json /tmp/nose-perf-sweep/raylib.final.json
   ```

   If `cmp` differs, compare parsed JSON family sets before assuming a regression.
   A semantic difference is a blocker. A pure ordering difference should be made
   deterministic before commit if practical.

6. Re-run the core validation set:

   ```text
   cargo test -p nose-normalize
   cargo test -p nose-detect
   GATE=core CROSS=all NOSE=target/release/nose ./scripts/type4-smoke.sh
   ```

7. Re-run a small top-outlier sweep after validation. The next likely targets are:

   - `bench/repos/sqlite`, still around 4.16s wall and currently the best next hotspot;
   - `bench/repos/vim`, around 1.85s wall and 1545ms `normalize+extract`;
   - `bench/repos/sympy`, now improved but still useful as a generated-formula regression
     target;
   - `bench/repos/raylib`, now improved but still useful as a many-unit regression target.

   For each outlier, use both global timing and unit timing:

   ```text
   NOSE_TIME=1 NOSE_TIME_UNITS=1 target/release/nose scan <repo> --mode semantic --format json --top 0 > /tmp/nose-next-profile.json 2> /tmp/nose-next-profile.err
   ```

   `zstd`, `clap`, `regex`, `zod`, `netty`, `nats-server`, `prettier`, and `sqlalchemy`
   are no longer the best profiling targets unless they regress.

8. Use modern hardware deliberately in the next pass. File-level rayon parallelism works
   well after the per-file fixed costs are gone, but single huge units can still cause
   load imbalance. Do not add coarse parallelism blindly; first identify whether the next
   outlier is a many-file fixed-cost problem, a single-unit DAG problem, or candidate
   generation/scoring work.

Validation already run before the raylib-specific continuation:

```text
cargo build --release -p nose-cli
cargo test -p nose-normalize
cargo test -p nose-detect
GATE=core CROSS=all NOSE=target/release/nose ./scripts/type4-smoke.sh
```

Final smoke result:

```text
positive recall: 613/613
hard-negative false merges: 0/1201
Raw nodes: 0/66123
```

Additional validation after the raylib-specific and later changes:

```text
cargo build --release -p nose-cli
cargo test -p nose-normalize
cargo test -p nose-detect
cargo test -p nose-cli
GATE=core CROSS=all NOSE=target/release/nose ./scripts/type4-smoke.sh
cmp /tmp/nose-perf-sweep/raylib.full.json /tmp/nose-perf-sweep/raylib.after-binding-filter.json
cmp /tmp/nose-perf-sweep/raylib.full.json /tmp/nose-perf-sweep/raylib.after-strictfacts.json
cmp /tmp/nose-perf-sweep/raylib.full.json /tmp/nose-perf-sweep/raylib.after-subtree-cache.json
cmp /tmp/nose-perf-after-subtree/sympy.json /tmp/nose-perf-after-subtree/sympy.after-large-ac.json
cmp /tmp/nose-perf-after-subtree/sqlite.json /tmp/nose-perf-final/sqlite.after-guard.json
```

Latest validation passed after the final large-AC, flatten, cache-removal, and Prettier
guard changes. The exact `/tmp` output names above are historical; the final rerun used
`/tmp/nose-perf-final/*.after-guard.*` for zod/raylib/sqlite/prettier.

## Last Completed Coevolution Loop

The last completed strict-frontier loop was `Batch-3 Python module collection membership`
recorded in `bench/type4/ITERATIONS.md` as loops 402-406.

The batch opened:

- `axis_membership_module_python_tuple_identity`;
- `axis_membership_module_python_set_identity`;
- `axis_membership_module_python_mutated_boundary`.

The detector change:

- canonicalized Python module-level tuple/set literal bindings as strict collection
  membership values only after stable-binding proof;
- rejected normalized `@Append(receiver, value)` mutations for module/local bindings,
  closing a false merge where `VALUES.append(...)` bypassed the old field-method scanner.

Final validation:

```text
focused:                  4/4 positives, 0/6 false merges
literal-membership core:  175/175 positives, 0/424 false merges
compact all-cross core:   613/613 positives, 0/1201 false merges
```

Afterwards, a broader generated smoke run also showed:

```text
same-surface/full smoke: 1089/1089 positives, 0/2010 false merges
```

## Where Work Stopped

After the last completed loop, the next candidate was explored but not implemented.

The likely next synthetic frontier candidate was `map_default_lookup` for Ruby
`Hash#fetch(key, default)` on a proven dynamic receiver. A probe showed Ruby fetch lowers
to this normalized IL shape:

```lisp
(call
  (field "fetch"
    (var v0))
  (var v2)
  (lit int=0))
```

No files were changed for that candidate. It is only a candidate note.

Do not treat this as an accepted next axis. It lacks:

- real-repo frequency evidence;
- a mined positive family showing useful refactoring value;
- hard negatives for custom `fetch`-like methods or mutated receivers;
- a focused generated batch;
- a before/after detector measurement.

## Real-Repo Evaluation Pass

Before continuing synthetic work, a real-repo sample comparison was run as requested:
two repos per supported language by actual file extension where possible, comparing
installed `nose 0.2.0` against current `nose 0.4.0` with:

```text
nose scan <repo> --mode semantic --format json --top 0
```

Results were written to:

```text
/tmp/nose-real-compare2/summary.json
```

The `/tmp` artifacts existed at handoff time, but they are intentionally not durable.
If they are missing, rerun the sample before making a resume decision.

Final sampled repos:

| language | repos |
|---|---|
| C | `tmux`, `zstd` |
| Go | `chi`, `gin` |
| Java | `gson`, `jsoup` |
| JavaScript | `axios`, `marked` |
| Python | `scrapy`, `sqlalchemy` |
| Ruby | `rspec-core`, `pry` |
| Rust | `regex`, `alacritty` |
| TypeScript | `zod`, `trpc` |

The initially selected Rust repo `clap` timed out on the then-current binary after 90s and
was replaced by `alacritty`. The uncommitted performance pass later addressed that
specific `clap` bottleneck; keep `clap` as a regression target, not the next profiling
priority.

Aggregate real-repo sample result:

```text
semantic families: 2021 -> 635  (delta -1386)
prod families:      944 -> 260  (delta -684)
test families:     1030 -> 346  (delta -684)
dup lines:        29265 -> 5047 (delta -24218)
value sum:        43916.2 -> 7661.5
added families:     256
removed families:  1642
```

Interpretation:

- The current detector is much stricter than the installed version. It removes many broad
  semantic families that are not proven strict Type-4.
- The current detector still adds some new useful strict families, especially in Python,
  C legacy-version code, Rust shared utility code, and TypeScript helper predicates.
- This is a good direction for strict Type-4, but the result also shows that synthetic
  recall alone is no longer the right success metric.

Examples that looked useful:

- `zstd`: repeated legacy-version blocks across `zstd_v02`-`zstd_v07`.
- `regex`: duplicated `mkwordset` logic across `regex-automata` and `regex-lite`.
- `scrapy`: repeated `from_crawler` / setup-family methods.
- `sqlalchemy`: repeated test setup/mapping patterns.
- `trpc`: small repeated predicate/helper functions.

Examples with low refactoring value:

- one-line `axios` test callbacks;
- short constructor boilerplate in `gson`/`jsoup`;
- short `expecting` helpers in `alacritty`.

## What To Do Differently Next

1. Do not keep running only synthetic batch loops.

   The generated strict suite is currently clean. More synthetic batches can still widen
   the frontier, but the marginal value is lower unless the new proof invariant is backed
   by real-repo evidence.

2. Make real-repo useful yield part of the loop gate.

   A future loop should pass all of these:

   - focused generated batch improves recall or closes a false merge;
   - axis-core and compact all-cross remain at zero false merges;
   - installed-vs-current real-repo sample produces at least a few human-useful added
     families, or demonstrably removes unsafe installed-version families;
   - runtime does not regress badly on representative repos.

3. Preserve batch-3, but choose batches from one invariant.

   Good batch shape:

   - two or three positives sharing the same proof rule;
   - one hard-negative boundary sharing the same coordinates;
   - one focused baseline before implementation;
   - one axis-core and one compact all-cross after implementation.

   Bad batch shape:

   - mixing unrelated language features;
   - adding examples just because generator coverage is easy;
   - broadening ambiguous receiver methods without type/import/mutation proof.

4. Before Ruby `fetch`, investigate real examples.

   If resuming the Ruby `Hash#fetch(key, default)` idea, first mine `rubocop`,
   `fastlane`, `rspec-core`, and `pry` for concrete repeated fetch-default patterns.
   Only implement it if receiver/key/default coordinates can be proven without trusting
   arbitrary `fetch` methods.

5. Treat performance as a first-class loop constraint.

   The coevolution loop only works if generated gates and real-repo audits are cheap
   enough to run repeatedly. Finish the open performance pass before adding more frontier
   rules. After that, keep a small top-outlier timing set in the loop gate so broadening
   the strict frontier does not silently make audits unusable.

6. Persist important real-repo audit outputs outside `/tmp`.

   The generated benchmark artifacts can stay temporary, but real-repo deltas that affect
   frontier selection should be copied or summarized under `bench/type4/` before pausing.
   Otherwise the next session has to reconstruct too much context.

7. Separate three kinds of success.

   A loop should report them independently:

   - strict-frontier coverage: generated positives found, generated hard negatives clean;
   - product usefulness: real-repo families that a human would plausibly refactor;
   - operational cost: scan/build/gate runtime, especially on large real repos.

   Do not let a clean generated smoke hide weak real-repo usefulness or a serious runtime
   regression.

8. Prefer corpus-driven all-language axes.

   Recent improvements included language-specific proof work, but the broader objective is
   not to bias the detector toward one language family. Use
   `bench/type4/prioritize_frontier.py` to find proof invariants that appear across the
   pinned multi-language corpus, then implement language-specific lowerings only as needed
   to express that common invariant.

9. Keep the accelerated loop shape, but cap each batch by one invariant.

   The 3-item batch shape worked for loops 392-406 because each batch shared one proof
   invariant. Continue grouping two or three related surfaces per loop, but split the work
   when the proof story changes. Faster batching is useful only while false-merge
   diagnosis remains simple.

10. Add a short qualitative review before merging more frontier code.

    For each real-repo comparison, inspect representative added and removed families.
    Mark each as `useful`, `low-value`, `unsafe/removed`, or `unclear`. A strict detector
    can still produce low-value clones; those should steer ranking and reporting even when
    they are technically exact.

## Suggested Resume Sequence

1. Finish the open performance pass first.

   Rerun the core validation set, representative output comparisons, and a small
   top-outlier timing check after the `flatten_cache` removal. Only then decide whether to
   commit the performance patch.

2. Rebuild current binary:

   ```text
   cargo fmt
   cargo build --release -p nose-cli
   ```

3. Re-run a small strict generated smoke to make sure the baseline is still clean:

   ```text
   GATE=core CROSS=all NOSE=target/release/nose ./scripts/type4-smoke.sh
   ```

4. Re-run the real-repo sample if `/tmp/nose-real-compare2/summary.json` is gone.

5. Refresh frontier priorities from the pinned corpus:

   ```text
   python3 bench/type4/prioritize_frontier.py \
     --cache /tmp/nose-frontier-priorities.cache.json \
     --json-out /tmp/nose-frontier-priorities.json \
     --md-out bench/type4/FRONTIER_PRIORITIES.md
   ```

6. Pick the next frontier only after reviewing real examples. Ruby
   `Hash#fetch(key, default)` under `map_default_lookup` is a plausible note, but it is
   not committed to the plan. It should lose to any higher-yield corpus-backed invariant.

7. For the selected frontier, run the loop in this order:

   - mine real examples and write the proof invariant;
   - add two or three generated positives plus matching hard negatives;
   - measure focused baseline against the previous release/current baseline;
   - implement the smallest detector/lowering change that proves the invariant;
   - run focused, axis-core, and compact all-cross gates;
   - run a small real-repo installed-vs-current or before-vs-after comparison;
   - qualitatively review added/removed families before deciding whether to keep it.

8. If implementing a new batch, record it in `bench/type4/ITERATIONS.md` and keep this
   handoff file updated at the end of the session.

## Resume Decision Checklist

Before starting another autonomous run, answer these questions in the next session notes:

- Which proof invariant is being widened?
- Which languages express the same invariant, and which are intentionally deferred?
- What real-repo samples justify the axis?
- What hard negatives prevent over-merging?
- Which generated gate is expected to fail before the detector change?
- Which real-repo audit will decide whether the loop was useful?
- What runtime budget is acceptable for that audit?

If any answer is missing, spend the next turn on mining or profiling rather than detector
code.
