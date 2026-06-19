# Hazard tuning — measured results

Evidence for calibrating `hazard()`. Pipeline + analysis in this directory
(`mine.py`, `analyze.py`, `tune.py`, `run_corpus.sh`); methodology in
[../../docs/hazard-benchmark.md](../../docs/hazard-benchmark.md); on each nose release
follow [../../docs/hazard-release-checklist.md](../../docs/hazard-release-checklist.md).

## Corpus (v1)

Monthly snapshots over the most recent ~60 months, `--mode semantic,near`, nose 0.5.0.
Labels from git over nose-identified family member spans (Kim Inconsistent-Change):
G1 = some siblings edited not all; G0c = all together; G0s = none; **G2 = a G1 whose
changed sibling was modified by a bug-fix commit that did not propagate** (gold; loose
file-level/interval proxy — see limits).

| repo | lang | stratum | G1 | G2 |
|---|---|---|---|---|
| pandas | Python | S | 1248 | 58 |
| kafka | Java | S | 800 | 5 |
| django | Python | S | 709 | 18 |
| terraform | Go | S | 648 | 32 |
| hugo | Go | S | 434 | 25 |
| tokio | Rust | S | 266 | 8 |
| grpc | C++ | **X** | 181 | 2 |
| redis | C | S | 132 | 15 |
| thrift | C++ | **X** | 119 | 12 |
| ripgrep | Rust | S | 60 | 0 |
| vue-core | TypeScript | S | 37 | 6 |
| express | JavaScript | S | 5 | 0 |
| **total** | **8 langs** | | **4,639** | **181** |

462,569 family-interval events; 15,199 distinct families (24.8% ever G1, 1.1% ever G2).
G2 uses function-level bug-fix attribution (git `-L:funcname`), landing the G2-among-G1
rate at ~1.1% (family) — matching the literature's 1–3% release-level rate.

> **⚠️ But an LLM-judge audit of all 181 G2 events found the automatic G2 label is only
> ~11% precise** (see [Gold-label audit](#gold-label-audit-llm-judge)). The rate matched
> the literature, but the precision did not — `rate-match ≠ precision`. **So G2 is NOT a
> usable gold label, and any "validated against bug-linked harm" claim is retracted.**
> The clean, directly-observed **G1** label (some siblings changed, others not — no
> fragile bug-fix attribution) remains the real validation below.

## Refresh — current main (re-mined 2026-06-06)

Re-ran the full pipeline against the current detector — HEAD after the #43–#65 Type-4
exact-fragment / proof-obligation / flat-map work, **+189 commits past the v1 corpus's
`nose 0.5.0` build** — on the same cached clones. Per the
[release checklist](../../docs/hazard-release-checklist.md), detection output changed so
the dataset was regenerated, but the **formula still holds** (weights stable, best
candidate AUC unchanged) → no re-calibration; the shipped `hazard()` is untouched.

| | v1 (0.5.0 early) | refresh (current main) | Δ |
|---|---|---|---|
| families | 15,199 | 14,942 | −1.7% |
| ever-G1 | 24.8% | 24.8% | — |
| true cross-language families (langs > 1) | 37 | 36 | −1 |
| **v5 (shipped) G1 AUC** | **0.644** | **0.641** | −0.003 |
| v7 G1 AUC | 0.659 | 0.655 | −0.004 |
| logistic G1 AUC | 0.639 | 0.635 | −0.004 |

Every shift is within leave-one-repo-out noise; weight signs and order are unchanged
(`mean_lines` +0.45 top, `mean_sem` −0.27 anti-predictive, `invisibility` +0.15). The
large Type-4 engine expansion barely moved the function-level family backbone the corpus
is built on, and did **not** populate the structurally-rare cross-language stratum
(37 → 36) — reconfirming, from a fresh angle, that the ~0.60 structural ceiling is
invariant to detection quality (a strong harm ranker still needs the semantic layer).

> **Tooling fix made during this refresh:** `mine.py` still invoked the pre-consolidation
> `--threshold` flag, which the current CLI rejects (→ 0 families on every snapshot, a
> silently-broken refresh). Updated to the inline `--mode near:T` syntax.

## Headline finding — the literature-derived formula was mis-specified

Leave-one-repo-out logistic weights (stable, low variance across 12 held-out repos):

| feature | weight | direction |
|---|---|---|
| `mean_lines` | **+0.43** | ↑ hazard (strongest) |
| `modules` (dispersion) | **+0.28** | ↑ hazard |
| `mean_sem` (semantic size) | **−0.27** | **↓ hazard (anti-predictive)** |
| `invisibility` (1−tightness) | **+0.14** | ↑ hazard |
| `members` (copies) | +0.13 | ↑ (redundant with lines/modules) |
| `params` | +0.04 | ~noise (flipped sign from −0.06 at 7 repos) |
| `languages` | +0.03 | ↑ weak |

The pre-data design led with `mean_sem` as the **primary multiplier** — but semantic
size is *anti-predictive* for divergent-edit ranking (typical divergences are in
smaller-fingerprint families; the mean is a large-tail artifact). Source **line** span
is the real magnitude signal. `invisibility` is robustly positive (+0.14 across all 12
held-out repos) — copies that share less literal text, even within one language
(renamed / restructured Type-3 near-misses), are harder to recognize as siblings and so
get edited inconsistently more often (consistent with Saha's Type-3 finding).

> **Correction (honest):** an earlier draft claimed invisibility was "the top signal in
> the cross-language stratum (AUC 0.67)." That number came from a **repo-level** tag
> (thrift + grpc, treated as "X") — but only **33 of those 1,606 families are actually
> cross-language**. Corpus-wide, **true cross-language families (languages > 1) are just
> 37 of 15,199** (2 ever-G2), and a polyglot repo like apache/arrow yields **0 of 928**
> families cross-language: the same logic in C++ vs Python rarely converges to one
> value-fingerprint. So cross-language Type-4 is a real but **structurally rare**
> capability, too sparse to validate a cross-language-specific signal. invisibility
> stands as a *general* predictor, not a cross-language one.

## Candidate formulas (leave-one-repo-out test AUC)

**The validation is the G1 column.** The G2 column ranks against a label the audit found
is only ~11% precise, so its absolute values are not meaningful — shown only because the
formula ordering is stable across both.

| formula | vs G1 (clean) | vs G2 (~11% precise — informational) |
|---|---|---|
| **v5 = mean_lines × spread(files,modules,langs) × invisibility × scope** | **0.644** | 0.704 |
| v7 = v5 × 1/(1+0.5·params) | 0.659 | 0.669 |
| v1 = the original size-led design | 0.609 | 0.668 |
| value (raw-volume baseline) | 0.611 | 0.671 |
| random | ~0.49 | ~0.49 |

On the clean G1 label v5 beats the size-led design (0.644 vs 0.609), the value baseline
(0.611), and random — the param-dampening term (v7) rests on a sign-unstable weight and
is **dropped**.

## Decision: the implemented formula

```
hazard = mean_lines
       × spread(files, modules, languages)   // dispersion (existing helper)
       × invisibility                        // 0.3 + 0.7·(1 − tightness)
       × scope_weight                        // prod 1.0 / mixed 0.5 / test 0.25
```

Validated on the clean **G1** label (0.644 vs 0.609 for the size-led design, 0.611
value-baseline, ~0.49 random). All terms reuse existing `RefactorFamily` fields;
**`mean_sem` is dropped** (anti-predictive *for G1*), **`params` is not used** (noise).
Implemented as opt-in `--sort hazard` (`crates/nose-detect/src/report.rs::hazard`,
`SortKey::Hazard`) — **NOT the default**, because of the gold-harm result below.

## Gold harm validation — the formula predicts propensity, NOT harm

The 0.644 above is on **G1 = "did this family get edited inconsistently?"** A separate,
trustworthy **gold harm label** was then built (Phase B/C): an LLM judged 1,390 G1
candidates blind, *with the actual diff*, into harm / should-propagate / benign, and an
adversarial pass refuted weak positives (`build_candidates.py` → `gold-label-divergence`
workflow → `gold_eval.py`). Only **22 (strict) / 53 (lenient)** of 1,390 realized
divergences are genuine should-propagate harms (~1.6–3.8% — independently reproducing the
literature's 1–3% harmful rate, now semantically validated). On this gold:

| scorer | AUC: harmful-vs-benign divergence (the task that matters) |
|---|---|
| `mean_sem` only | **0.61–0.64** (best — the *dropped* feature) |
| `extractability` | 0.59–0.64 |
| **`hazard` (the formula)** | **0.51 — chance** |
| `value` | 0.42–0.47 |
| random | ~0.3 |

**The G1 result does not transfer to harm.** Predicting *which* clones get edited
inconsistently (propensity) is not the same as predicting *which inconsistencies are
harmful*, and the formula does the former, not the latter. Worse, `mean_sem` — dropped
because it was anti-predictive *for G1* — is the best (still weak) *harm* signal, so the
G1 proxy actively misled the design. Even the best static feature caps at ~0.6: **static
structural features have a low harm ceiling**, because harm depends on whether a specific
change *applies to the sibling* — a semantic question. (Caveat: 22–53 positives → wide
CIs; the robust claim is the *transfer failure*, not the exact numbers.)

Also surfaced: **698 of 1,390 candidates (50%) are not genuine clones** per the LLM — a
`near@0.70` precision problem that adds noise to everything downstream.

**Consequences:** the default stays `extractability`; `hazard` is experimental opt-in.

### Round 2 — larger gold + git-history (the ceiling is real)

We then did exactly what the round-1 limits called for: a **clone-quality gate**
(`shared_weight ≥ 4`, the best static is-clone separator, AUC 0.68), a **larger gold**
(2,296 labeled, reusing round-1 + 1,602 fresh LLM labels with adversarial verify → **51
confirmed harm positives**, 2.2%), and a **git-history** feature (blame the changed vs
lagging member's function at the snapshot — were they last touched together?). Harm-AUC,
now with usable CIs (±~0.07):

| scorer | harm-AUC (51 positives) |
|---|---|
| `-skew_days` (git-history: touched closer in time → harm) | **0.600** |
| `mean_sem` | 0.572 |
| `same_commit` (git-history) | 0.568 |
| `hazard` | 0.531 |
| `extractability` | 0.475 |
| leave-one-repo-out logistic **combination** of all | **0.524 (no lift)** |

**The ceiling is ~0.60, and combining static + git-history does not beat the best single
signal.** git-history is real and theoretically sound (harmful divergences happen in
families previously maintained *together*, consistent with Barbour/Kim) but weak, and
only computable for ~52% of candidates (git funcname tracking). The clone-quality gate
still left 46% non-clones — `near@0.70` precision is a deep issue.

### Round 3 — cognitive complexity / edit-surface (issue #23) moved the ceiling

The #23 direction (per-copy *edit-surface*, à la Cognitive Complexity) was the most
productive structural angle yet — tested on the same gold from the member code/diff
already captured (`cogcomplexity.py`, `harm_model.py`), no re-mining:

| signal (#23) | harm-AUC | availability |
|---|---|---|
| `diff_per_cog` — small change in a *complex* function (Krinke "critical change") | **0.650** | post-divergence (needs the diff) |
| `cog` — member cognitive complexity (branches × nesting) | 0.61 | **pre-divergence** (query time) |
| `maxnest` | 0.59 | pre-divergence |
| (prior best: git-history −skew 0.60, mean_sem 0.57, hazard 0.53) | | |

So the best **pre-divergence** signal is `cog` (~0.61, ≈ the prior ceiling); the best
signal overall, `diff_per_cog` (~0.65), needs the actual diff and so is a **post-
divergence** signal: *given* a clone has been edited apart, a small subtle change in
complex logic is the harmful, easy-to-miss kind. The axis-B "edit-surface *symmetry*"
hypothesis from #23 was wrong (cog asymmetry AUC 0.44); absolute complexity × change
locality is the signal. A leave-one-repo-out logistic over all signals still does not
beat the single best (0.595) — combinations do not generalize on 51 positives.

**Revised conclusion (better than round 2's):** harm is best assessed **after** a
divergence (it is a property of the realized edit), and there the #23 signal reaches
~0.65 — a usable *post-divergence* ranker, the actionable form ("this clone already
diverged and a fix likely did not propagate"). Pre-divergence ranking still caps ~0.61.

### Round 4 — does nose's IL obscure cognitive complexity? (tested)

The natural worry: cognitive complexity is a *surface* property, and nose's IL normalizes
to detect *equivalence* — so the IL might erase it. **Tested** (`il_cog.py`): compute cog
from `nose il --normalized --format json` (count If/Loop with nesting + And/Or BinOps) vs
the source-text proxy, on the gold. Result on the IL-parsed subset (95% parse rate):

| cog source | harm-AUC |
|---|---|
| source-text proxy | 0.597 |
| nose IL (`--normalized`) | 0.599 |

**Essentially identical — the IL does *not* obscure cog.** Control structure survives
`il --normalized` (if/loop/&&/|| are preserved as `If`/`Loop`/`BinOp` nodes); only the
deeper *value-fingerprint* collapse (which makes loop≡comprehension, i.e. `mean_sem`)
erases it, and we don't compute cog from that. The flip side: **a proper IL-based cog
will not beat the text proxy** — both cap at ~0.60. cog is ~0.60 regardless of
representation; the only signal above it (`diff_per_cog`, 0.65) needs the realized diff.

**Firmly established now:** the *pre-divergence* structural harm ceiling is ~0.60 across
every representation (source / IL) and feature (size, dispersion, invisibility,
git-history, cognitive complexity, and their combinations). A *strong* harm ranker needs
the semantic layer (the bounded-LLM pass); structural signals give at best a weak
pre-divergence prior (~0.60) and a usable post-divergence ranker (~0.65).

## Gold-label audit (LLM-judge)

An LLM judge (standing in for the human auditor) reviewed **all 181 G2 events** blind
(`audit_sample.py` reconstructs the two clone members' code + the bug-fix commit; verdict
schema in `g2-audit-result.json`). Result:

- **Strict precision 11.1% (20 / 180 genuine).** False breakdown: `message_false_match`
  48 (the bug-fix keyword caught version drops, feature additions, typo/docs/readme/config
  changes), `intentional_divergence` 47 (async/sync pairs, virtual/stored variants, test
  fixtures that *legitimately* differ), `not_clones` 41 (the near channel grouped unrelated
  trivial stubs — e.g. two functions that only both `raise NotImplementedError`),
  `fix_not_applicable` 22 (real clone + real fix the sibling didn't need).
- Genuine examples it confirmed: django MD5 vs SHA1 hashers (a FIPS fix applies to both);
  Hugo template helpers; pandas reverse-FK `create`/`get_or_create`.
- The X-tagged repos contributed **0** genuine G2 (thrift 0/12, grpc 0/2).

**Lessons:** (1) `rate-match ≠ precision` — matching the literature's 1–3% rate said
nothing about correctness. (2) A real gold label needs all three of: a much better
bug-fix classifier (exclude non-behavioral commits), a same-vs-intentional-divergence
judgment, and tighter clone precision than near@0.70 — i.e. **the LLM judge *is* the
labeler**, not just an auditor. The 20 confirmed positives are the seed of a real
(small) gold set; the path forward is to LLM-label more G1 candidates rather than trust
the automatic G2.

## Honest limits

- AUC ≈ 0.64 (G1) is a useful *ranking* signal, not a precise predictor — divergent-edit
  propensity is inherently noisy from static features.
- **The automatic G2 label is ~11% precise** (audit above) — not usable as a gold yard
  stick; the formula stands on G1 alone.
- **Cross-language stratum is structurally unfillable, not just thin.** True
  cross-language families (languages > 1) are 37 corpus-wide (2 ever-G2); arrow yields
  0 of 928. Adding polyglot repos does *not* help — nose rarely detects cross-language
  Type-4 clones in real code. The benchmark's S/X balance goal is therefore not
  achievable for X; report it as a measured limit, not a TODO.
- Re-run on a new nose version: `run_corpus.sh` then `tune.py all-events.jsonl`
  (see [hazard-release-checklist](../../docs/hazard-release-checklist.md)).
