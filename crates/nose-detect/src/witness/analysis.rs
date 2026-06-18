use super::{
    anti_unify::{Au, NONE},
    dag::Dag,
    model::{GradedWitness, WitnessHole, MAX_NODES, MAX_NODE_PRODUCT, MAX_SPOTS},
};
use nose_normalize::{ValueDag, VgOp, VgSinkKind, VG_PROTOCOL_AWAIT};
use rustc_hash::{FxHashMap, FxHashSet};

/// Pair the two DAGs' sinks of one `kind`. Effects align by an order-preserving LCS
/// over their hash sequences (so identical effects align without crossings); leftover
/// hash-equal effects at different positions signal a reorder; the rest pair by
/// position so a genuinely-differing sink is still unified to find its hole. Returns
/// the paired `(a_value, b_value)` roots plus the unpaired extras on each side.
struct SinkPairing {
    pairs: Vec<(u32, u32)>,
    extra_a: Vec<u32>,
    extra_b: Vec<u32>,
    reordered: bool,
}

fn pair_sinks(a: &Dag<'_>, b: &Dag<'_>, kind: VgSinkKind) -> SinkPairing {
    let want = |k: VgSinkKind| k as u8 == kind as u8;
    let mut sa: Vec<_> = a.dag.sinks.iter().filter(|s| want(s.kind)).collect();
    let mut sb: Vec<_> = b.dag.sinks.iter().filter(|s| want(s.kind)).collect();
    let mut reordered = false;
    if matches!(kind, VgSinkKind::Effect) {
        sa.sort_by_key(|s| (s.effect_ord, a.dag.nodes[s.value as usize].hash));
        sb.sort_by_key(|s| (s.effect_ord, b.dag.nodes[s.value as usize].hash));
        let ha: Vec<u64> = sa
            .iter()
            .map(|s| a.dag.nodes[s.value as usize].hash)
            .collect();
        let hb: Vec<u64> = sb
            .iter()
            .map(|s| b.dag.nodes[s.value as usize].hash)
            .collect();
        let (n, m) = (ha.len(), hb.len());
        let mut pairs: Vec<(usize, usize)> = Vec::new();
        let mut in_lcs_a = vec![false; n];
        let mut in_lcs_b = vec![false; m];
        if n.saturating_mul(m) <= 1_000_000 {
            let mut dp = vec![0u32; (n + 1) * (m + 1)];
            for i in (0..n).rev() {
                for j in (0..m).rev() {
                    dp[i * (m + 1) + j] = if ha[i] == hb[j] {
                        dp[(i + 1) * (m + 1) + j + 1] + 1
                    } else {
                        dp[(i + 1) * (m + 1) + j].max(dp[i * (m + 1) + j + 1])
                    };
                }
            }
            let (mut i, mut j) = (0, 0);
            while i < n && j < m {
                if ha[i] == hb[j] {
                    pairs.push((i, j));
                    in_lcs_a[i] = true;
                    in_lcs_b[j] = true;
                    i += 1;
                    j += 1;
                } else if dp[(i + 1) * (m + 1) + j] >= dp[i * (m + 1) + j + 1] {
                    i += 1;
                } else {
                    j += 1;
                }
            }
        }
        let mut by_hash: FxHashMap<u64, Vec<usize>> = FxHashMap::default();
        for j in (0..m).rev() {
            if !in_lcs_b[j] {
                by_hash.entry(hb[j]).or_default().push(j);
            }
        }
        let mut rest_a: Vec<usize> = Vec::new();
        let mut used_b: FxHashSet<usize> = FxHashSet::default();
        for i in 0..n {
            if in_lcs_a[i] {
                continue;
            }
            if let Some(j) = by_hash.get_mut(&ha[i]).and_then(Vec::pop) {
                pairs.push((i, j));
                used_b.insert(j);
                reordered = true;
            } else {
                rest_a.push(i);
            }
        }
        let rest_b: Vec<usize> = (0..m)
            .filter(|j| !in_lcs_b[*j] && !used_b.contains(j))
            .collect();
        let common = rest_a.len().min(rest_b.len());
        for i in 0..common {
            pairs.push((rest_a[i], rest_b[i]));
        }
        return SinkPairing {
            pairs: pairs
                .into_iter()
                .map(|(i, j)| (sa[i].value, sb[j].value))
                .collect(),
            extra_a: rest_a[common..].iter().map(|&i| sa[i].value).collect(),
            extra_b: rest_b[common..].iter().map(|&j| sb[j].value).collect(),
            reordered,
        };
    }
    if matches!(kind, VgSinkKind::Cond) {
        sa.sort_by_key(|s| a.dag.nodes[s.value as usize].hash);
        sb.sort_by_key(|s| b.dag.nodes[s.value as usize].hash);
    }
    let common = sa.len().min(sb.len());
    SinkPairing {
        pairs: sa[..common]
            .iter()
            .zip(&sb[..common])
            .map(|(x, y)| (x.value, y.value))
            .collect(),
        extra_a: sa[common..].iter().map(|s| s.value).collect(),
        extra_b: sb[common..].iter().map(|s| s.value).collect(),
        reordered,
    }
}

fn classify(a: &Dag<'_>, b: &Dag<'_>, x: u32, y: u32) -> &'static str {
    if x == NONE || y == NONE {
        let present = if x == NONE {
            &b.dag.nodes[y as usize]
        } else {
            &a.dag.nodes[x as usize]
        };
        if present.op == VgOp::Opaque && present.key == VG_PROTOCOL_AWAIT {
            return "async-mirror";
        }
        return "arity";
    }
    let (ta, tb) = (a.dag.nodes[x as usize].op, b.dag.nodes[y as usize].op);
    if ta != tb {
        return "shape";
    }
    match ta {
        VgOp::Const => "literal",
        VgOp::Input => "input",
        VgOp::Field => "field",
        VgOp::Call => "call",
        VgOp::Lambda => "lambda",
        VgOp::Bin | VgOp::Un => "operator",
        VgOp::Opaque | VgOp::Formula | VgOp::Recurrence | VgOp::Loop => "unmodeled",
        _ => "expr",
    }
}

fn lines(d: &Dag<'_>, v: u32) -> Option<(u32, u32)> {
    if v == NONE {
        return None;
    }
    let n = &d.dag.nodes[v as usize];
    (n.line_start != 0).then_some((n.line_start, n.line_end))
}

/// Compare the two units' resolved referents. A name both consume that resolves to
/// disjoint referent sets is a mismatch (fail-closed); a name unresolved on either
/// side is a scoped caveat.
fn compare_referents(a: &ValueDag, b: &ValueDag) -> (Vec<String>, Vec<String>) {
    type Acc = FxHashMap<u64, (String, FxHashSet<u64>, bool)>;
    let collect = |dag: &ValueDag| -> Acc {
        let mut m: Acc = FxHashMap::default();
        for r in &dag.referents {
            let e = m
                .entry(r.name_key)
                .or_insert_with(|| (r.name.clone(), FxHashSet::default(), false));
            match r.referent {
                Some(id) => {
                    e.1.insert(id);
                }
                None => e.2 = true,
            }
        }
        m
    };
    let (ra, rb) = (collect(a), collect(b));
    let (mut mism, mut caveat) = (Vec::new(), Vec::new());
    for (key, (name, set_a, unres_a)) in &ra {
        let Some((_, set_b, unres_b)) = rb.get(key) else {
            continue;
        };
        if !set_a.is_empty() && !set_b.is_empty() && set_a.is_disjoint(set_b) {
            mism.push(name.clone());
        } else if *unres_a || *unres_b {
            caveat.push(name.clone());
        }
    }
    mism.sort();
    mism.dedup();
    mism.truncate(16);
    caveat.sort();
    caveat.dedup();
    caveat.truncate(16);
    (mism, caveat)
}

/// Anti-unify two units' value DAGs into a graded witness. `a_lossy`/`b_lossy` mark
/// whether each unit passed lossy lowering (so the claim is scoped to the modeled
/// fraction). Returns `None` when the pair is too large or too deep to align soundly
/// (the witness fails closed to absent rather than guessed).
/// The aligned shape of a pair: the differing nodes (`holes`), the unmatched sinks
/// (`extra`, with the side each is on), and whether any effects were reordered.
struct Alignment {
    holes: Vec<(u32, u32)>,
    extra: Vec<(u32, bool)>,
    reordered: bool,
    /// One side awaited where the other did not (an async↔sync twin).
    async_mirror: bool,
}

/// Pair every sink kind and anti-unify the matched roots. `None` if the alignment hit
/// the depth guard (fail-closed).
fn align(da: &Dag<'_>, db: &Dag<'_>) -> Option<Alignment> {
    let mut au = Au::new(da, db);
    let mut extra: Vec<(u32, bool)> = Vec::new();
    let mut reordered = false;
    for kind in [
        VgSinkKind::Return,
        VgSinkKind::Cond,
        VgSinkKind::Effect,
        VgSinkKind::Break,
        VgSinkKind::Throw,
    ] {
        let sp = pair_sinks(da, db, kind);
        reordered |= sp.reordered;
        extra.extend(sp.extra_a.into_iter().map(|v| (v, true)));
        extra.extend(sp.extra_b.into_iter().map(|v| (v, false)));
        for (x, y) in sp.pairs {
            au.unify(x, y, 0);
        }
    }
    if au.truncated {
        return None;
    }
    Some(Alignment {
        holes: std::mem::take(&mut au.holes),
        extra,
        reordered,
        async_mirror: au.async_mirror,
    })
}

/// Itemize the holes and unmatched sinks into [`WitnessHole`]s (text filled later),
/// returning the spots and whether every hole is a small value leaf.
fn build_spots(
    da: &Dag<'_>,
    db: &Dag<'_>,
    holes: &[(u32, u32)],
    extra: &[(u32, bool)],
) -> (Vec<WitnessHole>, bool) {
    let mut spots: Vec<WitnessHole> = Vec::new();
    let mut all_leafy = true;
    for &(x, y) in holes {
        let class = classify(da, db, x, y);
        let wa = if x == NONE { 0 } else { da.weight[x as usize] };
        let wb = if y == NONE { 0 } else { db.weight[y as usize] };
        let effect =
            (x != NONE && da.effectish[x as usize]) || (y != NONE && db.effectish[y as usize]);
        if wa > 4 || wb > 4 || matches!(class, "unmodeled" | "shape" | "arity") {
            all_leafy = false;
        }
        if spots.len() < MAX_SPOTS {
            spots.push(WitnessHole {
                class,
                a_lines: lines(da, x),
                b_lines: lines(db, y),
                effect,
                a_text: String::new(),
                b_text: String::new(),
            });
        }
    }
    for &(v, on_a) in extra {
        let line = lines(if on_a { da } else { db }, v);
        if spots.len() < MAX_SPOTS {
            spots.push(WitnessHole {
                class: "extra-sink",
                a_lines: if on_a { line } else { None },
                b_lines: if on_a { None } else { line },
                effect: true,
                a_text: String::new(),
                b_text: String::new(),
            });
        }
    }
    (spots, all_leafy)
}

/// Derive the divergence-pattern labels from the alignment and sizes.
fn derive_patterns(al: &Alignment, na: usize, nb: usize, low_substance: bool) -> Vec<&'static str> {
    let mut patterns: Vec<&'static str> = Vec::new();
    if al.reordered {
        patterns.push("effects-reordered");
    }
    if !al.extra.is_empty() && al.holes.is_empty() {
        let a_extra = al.extra.iter().any(|&(_, on_a)| on_a);
        let b_extra = al.extra.iter().any(|&(_, on_a)| !on_a);
        if a_extra && !b_extra {
            patterns.push("sink-superset-a");
        } else if b_extra && !a_extra {
            patterns.push("sink-superset-b");
        }
    }
    let skew = na.min(nb) as f64 / na.max(nb).max(1) as f64;
    if !al.extra.is_empty() && skew < 0.3 {
        patterns.push("fragment-containment");
    }
    if low_substance {
        patterns.push("low-substance");
    }
    patterns
}

/// Anti-unify two units' value DAGs into a graded witness. `a_lossy`/`b_lossy` mark
/// whether each unit passed lossy lowering (so the claim is scoped to the modeled
/// fraction). Returns `None` when the pair is too large or too deep to align soundly
/// (the witness fails closed to absent rather than guessed).
pub fn graded_witness(
    a: &ValueDag,
    b: &ValueDag,
    a_lossy: bool,
    b_lossy: bool,
) -> Option<GradedWitness> {
    let (na, nb) = (a.nodes.len(), b.nodes.len());
    if na == 0 || nb == 0 || na > MAX_NODES || nb > MAX_NODES {
        return None;
    }
    if (na as u64).saturating_mul(nb as u64) > MAX_NODE_PRODUCT {
        return None;
    }
    let (da, db) = (Dag::new(a), Dag::new(b));
    let al = align(&da, &db)?;
    let (spots, all_leafy) = build_spots(&da, &db, &al.holes, &al.extra);

    let k = al.holes.len() + al.extra.len();
    let low_substance = k > 0 && na.min(nb) < 10;
    let mut patterns = derive_patterns(&al, na, nb, low_substance);

    if al.async_mirror {
        patterns.push("async-mirror");
    }

    let (referent_mismatches, caveat_names) = compare_referents(a, b);
    if !referent_mismatches.is_empty() {
        patterns.push("referent-mismatch");
    }

    // `async-mirror` is a *transformation* twin (async ↔ sync), never a behavioral equivalence —
    // a coroutine is not its resolved value. So it can never be `equal_modulo_holes`, regardless
    // of how cleanly the rest aligns.
    let equal_modulo_holes = al.extra.is_empty()
        && !al.reordered
        && all_leafy
        && !low_substance
        && !al.async_mirror
        && referent_mismatches.is_empty();

    Some(GradedWitness {
        holes: k,
        spots,
        patterns,
        referent_mismatches,
        caveat_names,
        equal_modulo_holes,
        modeled_caveat: a_lossy || b_lossy,
    })
}
