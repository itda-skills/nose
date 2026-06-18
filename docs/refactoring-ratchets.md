# Refactoring ratchets

nose keeps code quality pressure as ratchets: existing debt can be carried
temporarily, but it must not grow, and any real improvement should lower the
accepted ceiling in the same change.

The repository already ratchets function complexity and length through
[`clippy.toml`](../clippy.toml), test coverage through `cargo llvm-cov`, and
self-duplication through [`scripts/check-duplication.sh`](../scripts/check-duplication.sh).
The Rust file-length ratchet adds a coarser module-design signal on top.

## Rust file length

Run the gate directly with:

```sh
python3 scripts/check-file-lengths.py
```

The target is 600 lines for every Rust file under `crates/`. Files already above
that line are recorded in
[`scripts/file-length-budgets.json`](../scripts/file-length-budgets.json). A
budgeted file fails the gate if it grows. It also fails if it shrinks without its
budget being lowered, so the accepted ceiling moves down whenever a refactor pays
down debt.

CI runs the gate against the base ref with `--ratchet-base`, so the budget
file itself cannot be loosened in the same change: `default_max_lines` may not
increase, existing file budgets may not increase, and new over-target budget
entries are rejected.

Do not use the budget file to bless newly large modules. New modules should stay
under the 600-line target; if a split still produces a larger file, keep looking
for a sharper boundary.

## Refactoring direction

File length is a symptom, not the objective. Prefer changes that make ownership
and behavior easier to reason about:

- separate CLI orchestration from query planning, rendering, config parsing, and
  file/process effects;
- keep the CLI binary root focused on process setup and subcommand dispatch;
  argument models, legacy detect/IL adapters, scan baseline handling, graded
  witness enrichment, opportunity grouping, source-line diff/proposal logic,
  human/markdown/SARIF scan rendering, and CLI-side timing helpers now live in
  dedicated `nose-cli/src/{cli_args,detect_command,il_command,scan_*,timing}.rs`
  modules;
- keep shrinking `nose-cli/src/main.rs` by moving bounded report helpers into
  small sibling modules first; the scan JSON report, baseline comparison view,
  and family-display text now live outside the command dispatcher;
- keep shared verify-oracle support outside the command dispatcher; the oracle
  battery, behavior hashing, and behavioral-gate tally now live in
  `nose-cli/src/oracle_gate.rs`;
- keep `nose verify` collection separate from presentation; JSON, exclusion,
  soundness, completeness, calibration, and falsification output now live in
  `nose-cli/src/verify_report.rs`;
- keep the verify oracle's per-file collection path separate from command
  parsing; interpreted records, exclusion accounting, and canon-preservation
  collection now live in `nose-cli/src/verify_collect.rs`;
- keep hidden diagnostic and benchmark commands outside the dispatcher; `features`,
  `value-census`, `stats`, `eval`, and `ceiling` now live in
  `nose-cli/src/diagnostic_commands.rs`;
- keep the `nose query` surface outside the dispatcher: query model/JSON helpers,
  renderers, dashboard, family drilldown, and command orchestration now live in
  `nose-cli/src/query_*.rs`;
- keep shared scan dataset construction and deprecated `nose scan` command
  orchestration outside the dispatcher; they now live in
  `nose-cli/src/scan_commands.rs`;
- keep shared scan option parsing and report model types outside the dispatcher;
  mode parsing, scan-channel resolution, report formats, ranking keys, gate
  selectors, and scan scope summaries now live in `nose-cli/src/scan_options.rs`;
- keep deprecated `nose scan` report rendering and gate output outside the
  dispatcher; the JSON/markdown/human/SARIF format switch, hotspots, and
  reinvented-helper report section now live in `nose-cli/src/scan_report.rs`;
- keep post-lower Library API recognition out of the shared lowering context;
  the first pass lives in `nose-frontend/src/lower/library_api_post_lower.rs`;
- keep shared frontend control-flow lowering out of the shared lowering context;
  `switch`, `if`, `while`, block-wrapping, and C-style `for` helpers now live in
  `nose-frontend/src/lower/control_flow.rs`;
- keep the shared frontend lowering context as the small state/dispatch root;
  IL builders, semantic-evidence recording, import facts, parse/file setup,
  post-lower evidence helpers, expression helpers, and lowering tests now live
  in focused `nose-frontend/src/lower/*` modules;
- keep wide frontend language roots as dispatch surfaces instead of mixed
  parser-policy files; JS/TS import parsing, type declarations, declarations,
  control-flow rewrites, expression lowering, global-symbol proof, record-shape
  guards, JSX lowering, operators, syntax helpers, and tests now live in
  focused `nose-frontend/src/js_ts/*` modules;
- keep corpus-level import replacement split by concern; export discovery,
  binding-use safety, module-path hashing, snapshot/evidence copying, and tests
  now live in focused `nose-frontend/src/module_imports/*` modules;
- keep the verify oracle's value model separate from tree-walking evaluation;
  `Value`, `Behavior`, symbolic containment, and declared-domain coercion now
  live in `nose-normalize/src/interp/value.rs`;
- keep the verify oracle's primitive operation semantics separate from tree-walking
  evaluation; truthiness, builtin folds, ranges, int32 coercion, and unary/binary
  operator execution now live in `nose-normalize/src/interp/ops.rs`;
- keep the verify oracle root focused on entry points and execution state;
  field-state proof, statement execution, expression evaluation, call/builtin
  handling, higher-order evaluation, and oracle tests now live in focused
  `nose-normalize/src/interp/*` modules;
- keep detect unit extraction focused on root orchestration; the public unit
  model, shape/minhash feature extraction, unit timing, IL tree helpers,
  exact-fragment root dispatch, ordered effect sequences, Java self-field
  fragments, loop-effect fragments, fragment context-safety, and unit tests now
  live in focused `nose-detect/src/units/*` modules;
- keep strict exact-safety policy fail-closed but locally owned; fact collection,
  tree entry points, HoF/comprehension safety, primitive literal/sequence gates,
  static index membership, call dispatch, collection/map receivers, factory/map
  constructors, callee identity, and policy tests now live in focused
  `nose-detect/src/strict_exact/*` modules;
- keep the detect crate root and report/witness surfaces as thin APIs; detect options,
  scorer policy, public report/location models, reinvented-helper containment,
  orchestration, candidate/group construction, report ranking/path policy, report test
  suites, and graded-witness anti-unification now live in focused
  `nose-detect/src/{options,detectors,model,locations,reinvented,orchestration,candidates,report/*,witness/*}.rs`
  modules;
- move reusable semantic or detection rules toward the owning library crate
  instead of keeping them in `nose-cli`;
- split wide language and IL dispatch only around real concepts, such as
  expression lowering, declaration facts, effect evidence, or value-graph state;
- keep table-driven and cross-language tests readable by extracting shared
  fixtures only when the name explains the scenario being tested;
- keep exact-fragment CLI fixture scanning and family-selection helpers outside
  the oversized test body; shared exact-fragment test support now lives in
  `nose-cli/tests/cli/exact_fragments/support.rs`;
- keep Java `this` field exact-fragment scenarios together in their own CLI
  test module; assignment, guarded branch, body, and fluent `return this`
  fixtures now live in `nose-cli/tests/cli/exact_fragments/java_this_field.rs`;
- keep ordered branch exact-fragment matrices grouped by effect shape; ordered
  foreach-effect and mixed-effect branch fixtures now live in
  `nose-cli/tests/cli/exact_fragments/ordered_effect_branches.rs`;
- keep ordered conditional branch exact-fragment matrices grouped by control
  shape; conditional-only fixtures now live in
  `nose-cli/tests/cli/exact_fragments/ordered_conditional_branches.rs`, and
  loop-plus-conditional fixtures live in
  `nose-cli/tests/cli/exact_fragments/ordered_loop_conditional_branches.rs`;
- turn oversized integration-test files into small suite roots plus domain-named
  modules, keeping each new module under the 600-line target;
- keep CLI integration suites as thin roots that declare domain modules;
  command-surface, exact-fragment, and semantic-idiom scenarios now live under
  `nose-cli/tests/cli/{commands,exact_fragments,semantic_idioms}/`;
- lower a file budget only in the same change that makes the corresponding
  design boundary clearer.

When a large file is reduced below 600 lines, remove its entry from
`scripts/file-length-budgets.json`.
