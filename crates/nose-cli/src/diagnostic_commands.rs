use super::*;

/// Dump each unit's detection features as JSON `{units: [...]}` — the raw value-graph,
/// shape, return and literal fingerprints, before candidate generation/thresholding.
pub(super) fn cmd_features(
    paths: Vec<PathBuf>,
    min_lines: u32,
    min_tokens: usize,
    no_cfg_norm: bool,
    no_blocks: bool,
) -> Result<()> {
    let refs = paths_as_refs(&paths);
    let corpus = nose_frontend::lower_corpus_many(&refs);
    warn_if_empty(&corpus, &paths);
    let opts = nose_detect::DetectOptions {
        min_lines,
        min_tokens,
        cfg_norm: !no_cfg_norm,
        block_units: !no_blocks,
        ..Default::default()
    };
    let units: Vec<nose_detect::UnitFeat> = corpus
        .files
        .iter()
        .flat_map(|il| nose_detect::units_of_file(il, &corpus.interner, &opts))
        .collect();
    println!(
        "{}",
        serde_json::to_string(&serde_json::json!({ "units": units }))?
    );
    Ok(())
}

/// #391 prevalence probe: walk every function unit, build its value graph with the opaque
/// census on, and aggregate per IL construct how many `Opaque` fallbacks were minted (the
/// value-graph analog of `stats`'s lowering `Raw`). `total_fallback` opaques are full coverage
/// gaps (the construct could not be modeled at all); the rest are partial/semantic opaques.
pub(super) fn cmd_value_census(paths: Vec<PathBuf>, no_cfg_norm: bool) -> Result<()> {
    use std::collections::{BTreeMap, HashSet};
    let refs = paths_as_refs(&paths);
    let corpus = nose_frontend::lower_corpus_many(&refs);
    warn_if_empty(&corpus, &paths);
    let opts = nose_normalize::NormalizeOptions {
        cfg_norm: !no_cfg_norm,
        ..Default::default()
    };
    // Build the value graph (the dominant cost) for every unit in parallel, each file producing a
    // local census, then reduce. The census API builds a fresh per-unit graph, so this is safe.
    type Census = (
        BTreeMap<(String, bool), u64>,
        BTreeMap<(String, bool), u64>,
        u64,
        u64,
    );
    let (opaque_nodes, affected_units, total_units, units_with_opaque): Census = corpus
        .files
        .par_iter()
        .map(|il| {
            let n = nose_normalize::normalize(il, &corpus.interner, &opts);
            let mut on: BTreeMap<(String, bool), u64> = BTreeMap::new();
            let mut au: BTreeMap<(String, bool), u64> = BTreeMap::new();
            let (mut tu, mut uo): (u64, u64) = (0, 0);
            for u in &n.units {
                if n.kind(u.root) != nose_il::NodeKind::Func {
                    continue; // count function-level units only (blocks would double-count)
                }
                tu += 1;
                let census =
                    nose_normalize::value_graph_opaque_census(&n, u.root, &corpus.interner);
                if !census.is_empty() {
                    uo += 1;
                }
                let mut seen: HashSet<(String, bool)> = HashSet::new();
                for (kind, total, count) in census {
                    let key = (format!("{kind:?}"), total);
                    *on.entry(key.clone()).or_insert(0) += u64::from(count);
                    if seen.insert(key.clone()) {
                        *au.entry(key).or_insert(0) += 1;
                    }
                }
            }
            (on, au, tu, uo)
        })
        .reduce(
            || (BTreeMap::new(), BTreeMap::new(), 0, 0),
            |mut a, b| {
                for (k, v) in b.0 {
                    *a.0.entry(k).or_insert(0) += v;
                }
                for (k, v) in b.1 {
                    *a.1.entry(k).or_insert(0) += v;
                }
                (a.0, a.1, a.2 + b.2, a.3 + b.3)
            },
        );
    let mut by_construct: Vec<_> = opaque_nodes
        .iter()
        .map(|((kind, total), n)| {
            serde_json::json!({
                "construct": kind,
                "total_fallback": total,
                "opaque_nodes": n,
                "units": affected_units.get(&(kind.clone(), *total)).copied().unwrap_or(0),
            })
        })
        .collect::<Vec<_>>();
    by_construct.sort_by(|a, b| {
        b["opaque_nodes"]
            .as_u64()
            .cmp(&a["opaque_nodes"].as_u64())
            .then_with(|| a["construct"].as_str().cmp(&b["construct"].as_str()))
    });
    println!(
        "{}",
        serde_json::to_string(&serde_json::json!({
            "files": corpus.files.len(),
            "function_units": total_units,
            "units_with_opaque": units_with_opaque,
            "by_construct": by_construct,
        }))?
    );
    Ok(())
}

pub(super) fn cmd_ceiling(gold: PathBuf, units: PathBuf, candidates: PathBuf) -> Result<()> {
    let gold_json = std::fs::read_to_string(&gold).with_context(|| format!("reading {gold:?}"))?;
    let units_json =
        std::fs::read_to_string(&units).with_context(|| format!("reading {units:?}"))?;
    let cands_json =
        std::fs::read_to_string(&candidates).with_context(|| format!("reading {candidates:?}"))?;
    let r = nose_eval::ceiling(&gold_json, &units_json, &cands_json)?;
    println!("{}", serde_json::to_string_pretty(&r)?);

    let pct = |n: usize, d: usize| {
        if d == 0 {
            0.0
        } else {
            100.0 * n as f64 / d as f64
        }
    };
    eprintln!(
        "\nrecall funnel ({} units, {} candidate pairs):",
        r.units, r.candidate_pairs
    );
    for (name, s) in [("all", &r.all), ("type4_semantic", &r.type4_semantic)] {
        eprintln!(
            "  {name:<16} gold={:<4} unit-reachable={:<4} ({:.1}%)  candidate-reachable={:<4} ({:.1}%)",
            s.gold,
            s.unit_reachable,
            pct(s.unit_reachable, s.gold),
            s.candidate_reachable,
            pct(s.candidate_reachable, s.gold),
        );
    }
    Ok(())
}

pub(super) fn cmd_eval(
    gold: PathBuf,
    predictions: PathBuf,
    hard_negatives: Option<PathBuf>,
    corpus: Option<PathBuf>,
) -> Result<()> {
    let gold_json = std::fs::read_to_string(&gold).with_context(|| format!("reading {gold:?}"))?;
    let preds_json = std::fs::read_to_string(&predictions)
        .with_context(|| format!("reading {predictions:?}"))?;
    let hn_json = match &hard_negatives {
        Some(p) => std::fs::read_to_string(p).with_context(|| format!("reading {p:?}"))?,
        None => String::new(),
    };
    let corpus_json = match &corpus {
        Some(p) => std::fs::read_to_string(p).with_context(|| format!("reading {p:?}"))?,
        None => String::new(),
    };

    let report = nose_eval::evaluate(&gold_json, &preds_json, &hn_json, &corpus_json)?;
    println!("{}", serde_json::to_string_pretty(&report)?);

    // headline numbers to stderr
    eprintln!(
        "\npredictions={} gold={}  HN-FP-rate={:.3} ({}/{})",
        report.prediction_count,
        report.gold_count,
        report.hn_fp_rate,
        report.hn_matched,
        report.hn_total
    );
    for (name, m) in &report.slices {
        eprintln!(
            "  slice {name:<26} P={:.4} R={:.4} F1={:.4} (gold {})",
            m.precision, m.recall, m.f1, m.gold
        );
    }
    for m in &report.macro_f1 {
        eprintln!(
            "  macro[{}]  dev_F1={:.4} ({} repos)  heldout_F1={:.4} ({} repos)",
            m.slice, m.dev_f1, m.dev_repos, m.heldout_f1, m.heldout_repos
        );
    }
    Ok(())
}

pub(super) fn cmd_stats(paths: Vec<PathBuf>, top: usize, json: bool) -> Result<()> {
    require_paths_exist(&paths)?;
    let refs = paths_as_refs(&paths);
    let corpus = nose_frontend::lower_corpus_many(&refs);
    warn_if_empty(&corpus, &paths);
    let report = nose_frontend::coverage(&corpus, top);

    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }

    let gap_raw = report.raw_nodes.saturating_sub(report.boundary_raw);
    println!(
        "files: {}   IL nodes: {}   Raw nodes: {} ({:.3}%)   = {} lowering-gap + {} protocol-boundary",
        report.files,
        report.total_nodes,
        report.raw_nodes,
        report.raw_ratio * 100.0,
        gap_raw,
        report.boundary_raw,
    );
    println!("\nper language (worst coverage first):");
    println!(
        "  {:<12} {:>7} {:>10} {:>9} {:>8} {:>9}",
        "lang", "files", "nodes", "raw", "raw%", "gap"
    );
    for l in &report.per_lang {
        println!(
            "  {:<12} {:>7} {:>10} {:>9} {:>7.3}% {:>9}",
            l.lang,
            l.files,
            l.nodes,
            l.raw_nodes,
            l.raw_ratio * 100.0,
            l.raw_nodes.saturating_sub(l.boundary_raw),
        );
    }
    println!("\ntop unhandled constructs (surface kind → Raw; `boundary` = by-design, not a gap):");
    println!(
        "  {:<12} {:<34} {:>8}  kind",
        "lang", "surface_kind", "count"
    );
    for u in &report.top_unhandled {
        let kind = if u.boundary { "boundary" } else { "gap" };
        println!(
            "  {:<12} {:<34} {:>8}  {kind}",
            u.lang, u.surface_kind, u.count
        );
    }
    Ok(())
}
