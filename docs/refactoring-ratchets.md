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
- move reusable semantic or detection rules toward the owning library crate
  instead of keeping them in `nose-cli`;
- split wide language and IL dispatch only around real concepts, such as
  expression lowering, declaration facts, effect evidence, or value-graph state;
- keep table-driven and cross-language tests readable by extracting shared
  fixtures only when the name explains the scenario being tested;
- lower a file budget only in the same change that makes the corresponding
  design boundary clearer.

When a large file is reduced below 600 lines, remove its entry from
`scripts/file-length-budgets.json`.
