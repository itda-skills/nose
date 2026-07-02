# C# grammar bump pre-measurement — 2026-07-02

A pre-measurement of the `tree-sitter-c-sharp` upstream `master` against the
pinned C# corpus, run for the standing bump watch
([itda-skills/nose#2](https://github.com/itda-skills/nose/issues/2)): the
corpus-wide C# lowering-gap remainder is dominated by grammar-level parse
`ERROR`s (`#if` around C#8 default interface members —
[tree-sitter-c-sharp#377](https://github.com/tree-sitter/tree-sitter-c-sharp/issues/377),
[#376](https://github.com/tree-sitter/tree-sitter-c-sharp/issues/376)), so
every grammar advance gets re-measured before a pin bump. See
[languages](languages.md) for the lowering itself and the gap methodology.

## Trigger status

crates.io's latest release is **0.23.5** — already in `Cargo.lock`, so there is
no version to bump to. Upstream `master` (`af29416d`) carries three unreleased
commits past v0.23.5: C# 14 language support
([#429](https://github.com/tree-sitter/tree-sitter-c-sharp/pull/429)), a
`(IDENT) && EXPR` logical-AND parse fix
([#430](https://github.com/tree-sitter/tree-sitter-c-sharp/pull/430)), and
unsafe-block highlighting queries. This audit measured `master` directly so the
verdict is ready before the next release is cut.

## Method

Temporary `[patch.crates-io]` pointing `tree-sitter-c-sharp` at `master`
(`af29416d`), then a **`cargo clean` release build** (the stale-grammar-object
discipline from the #402 false-green), then `nose stats` over the 15 pinned C#
repos (pins verified against `bench/goldens/corpus.json` first). The patch was
fully reverted afterwards and the baseline re-reproduced exactly.

## Results

| metric | 0.23.5 (pinned) | master `af29416d` |
|---|---|---|
| C# IL nodes | 3,305,469 | 3,305,699 |
| lowering gap (raw − boundary) / nodes | **0.0682%** | **0.1277%** |
| `ERROR` raw | **611** | **718 (+107)** |
| `parameter` / `modifier` / `parameter_list` fallout | 282 / 251 / 185 | 614 / 495 / 397 |

- **`master` is a net regression for the corpus.** C# 14 introduces node kinds
  the frontend does not lower yet (`extension_declaration` 46,
  `extension_body` 46, `receiver_parameter` 47), and the parse `ERROR` count
  itself rises by 107.
- The root-cause pattern is **not fixed**: serilog `ILogger.cs` still parses
  with a root `ERROR` (82/1,071 nodes raw) under `master`; upstream #377/#376
  remain open.
- The C# convergence suite (43 tests: 32 surfaces + 11 LINQ
  transparent-identifier) passes **43/43 under the `master` grammar** — a bump
  would not break convergence, only coverage.
- Baseline drift since the watch was filed: `ERROR` 651 → 611 and gap 0.081% →
  0.0682%, from the LINQ transparent-identifier completion (`0f87cc78`).

## Verdict and bump prerequisites

Keep the pin at `tree-sitter-c-sharp = "0.23"`. When the next release is cut
from `master`, two work items must land **with** (or before) the bump:

1. **C# 14 extension-member lowering** — `extension_declaration`,
   `extension_body`, `receiver_parameter` need real IL shapes, or they arrive
   as ~139 new lowering-gap Raw nodes.
2. **Explain the `ERROR` +107 regression** — plausibly the `extension`
   contextual keyword colliding with existing identifiers; confirm before
   trusting the new grammar on real corpora.

Then the standard bump procedure applies: `cargo clean` release build →
`nose stats` re-measurement → convergence suite → `nose verify
--max-violations 0` across all 15 pinned C# repos.

## Reproduce

```sh
cargo build --release -p nose-cli
target/release/nose stats bench/repos/<the 15 C# repo dirs…> --top 200 --format json
# per_lang lang=="csharp": gap = (raw_nodes − boundary_raw) / nodes
# top_unhandled lang=="csharp", surface_kind=="ERROR": the ERROR count
```
