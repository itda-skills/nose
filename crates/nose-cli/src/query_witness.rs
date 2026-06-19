use crate::legacy_prelude::*;

pub(crate) fn enrich_graded_witnesses(
    families: &mut [nose_detect::RefactorFamily],
    opts: &nose_detect::DetectOptions,
) {
    use std::collections::HashMap;
    let is_near = |f: &nose_detect::RefactorFamily| {
        f.languages == 1
            && f.locations.len() >= 2
            && f.witness.as_ref().map(|w| w.kind) == Some("structural-similarity")
    };
    if !families.iter().any(is_near) {
        return;
    }
    // The representative spans needed, grouped by source file.
    let mut wanted: HashMap<String, Vec<(u32, u32)>> = HashMap::new();
    for f in families.iter().filter(|f| is_near(f)) {
        for loc in [&f.locations[0], &f.locations[1]] {
            wanted
                .entry(loc.file.clone())
                .or_default()
                .push((loc.start_line, loc.end_line));
        }
    }
    // Lower each needed file once; export the value DAGs of its requested unit spans.
    let mut dags: HashMap<(String, (u32, u32)), (nose_normalize::ValueDag, bool)> = HashMap::new();
    for (file, spans) in &wanted {
        let Some(lang) = Lang::from_path(file) else {
            continue;
        };
        let Ok(src) = std::fs::read(file) else {
            continue;
        };
        let interner = Interner::new();
        let Ok(il) = nose_frontend::lower_source(FileId(0), file, &src, lang, &interner) else {
            continue;
        };
        let mut uniq = spans.clone();
        uniq.sort_unstable();
        uniq.dedup();
        let exported = nose_detect::unit_dags_at(&il, &interner, opts, &uniq);
        for (span, dag) in uniq.into_iter().zip(exported) {
            if let Some(dag) = dag {
                dags.insert((file.clone(), span), dag);
            }
        }
    }
    // Compute and attach each family's witness, filling hole source text.
    let mut lines = FileLineCache::default();
    for f in families.iter_mut().filter(|f| is_near(f)) {
        let a_file = f.locations[0].file.clone();
        let a_lines = (f.locations[0].start_line, f.locations[0].end_line);
        let b_file = f.locations[1].file.clone();
        let b_lines = (f.locations[1].start_line, f.locations[1].end_line);
        let (Some((da, a_exact)), Some((db, b_exact))) = (
            dags.get(&(a_file.clone(), a_lines)),
            dags.get(&(b_file.clone(), b_lines)),
        ) else {
            continue;
        };
        let Some(mut witness) = nose_detect::graded_witness(da, db, !a_exact, !b_exact) else {
            continue;
        };
        for hole in &mut witness.spots {
            if let Some((s, e)) = hole.a_lines {
                hole.a_text = witness_spot_text(&mut lines, &a_file, s, e);
            }
            if let Some((s, e)) = hole.b_lines {
                hole.b_text = witness_spot_text(&mut lines, &b_file, s, e);
            }
        }
        // Definition-site modifiers (decorators/attributes) are erased at lowering, so
        // the value graph cannot see a `@deco(x)` vs `@deco(y)` difference. Compare them
        // from source here: if the two copies' decorator/attribute lines differ, the
        // bodies being equal-modulo-holes is NOT the whole story — record the difference
        // as a hole and demote the claim (fail-closed). Identical decorators leave the
        // witness untouched.
        let lang = f.locations[0].lang.as_str();
        let a_decos = decorator_lines(&mut lines, lang, &a_file, a_lines.0, a_lines.1);
        let b_decos = decorator_lines(&mut lines, lang, &b_file, b_lines.0, b_lines.1);
        if let Some((a_only, b_only)) = decorator_difference(&a_decos, &b_decos) {
            witness.spots.push(nose_detect::WitnessHole {
                class: "decorator",
                a_lines: None,
                b_lines: None,
                effect: false,
                a_text: cap_join(&a_only),
                b_text: cap_join(&b_only),
            });
            witness.holes += 1;
            witness.equal_modulo_holes = false;
            if !witness.patterns.contains(&"decorator-differs") {
                witness.patterns.push("decorator-differs");
            }
        }
        if let Some(w) = f.witness.as_mut() {
            w.graded = Some(witness);
        }
    }
}

/// The line prefix that marks a definition-site modifier in `lang`: `@` for the
/// decorator/annotation languages, `#[` for Rust attributes. `None` for languages with
/// no such syntax — crucially Ruby, where a leading `@` is an *instance variable*
/// (`@token = …`), not a decorator, so it must NOT be treated as one.
pub(crate) fn decorator_prefix(lang: &str) -> Option<&'static str> {
    match lang {
        "python" | "java" | "javascript" | "typescript" => Some("@"),
        "rust" => Some("#["),
        _ => None,
    }
}

/// The sorted decorator/attribute lines inside a unit's source span. These modify
/// behavior at the definition site but their arguments are dropped at lowering, so the
/// value graph is blind to them (a nested `@click.argument("x")` vs
/// `@click.argument("x", metavar="m")` produces the same IL). Comparing the source text
/// is the only place the difference is visible.
fn decorator_lines(
    lines: &mut FileLineCache,
    lang: &str,
    file: &str,
    start: u32,
    end: u32,
) -> Vec<String> {
    let Some(prefix) = decorator_prefix(lang) else {
        return Vec::new();
    };
    let Some(slice) = lines.slice(file, start, end) else {
        return Vec::new();
    };
    let mut out: Vec<String> = slice
        .iter()
        .map(|l| l.trim())
        .filter(|l| l.starts_with(prefix))
        .map(str::to_string)
        .collect();
    out.sort();
    out
}

/// Multiset difference of two decorator-line lists: `Some((a_only, b_only))` when they
/// differ, `None` when identical.
pub(crate) fn decorator_difference(
    a: &[String],
    b: &[String],
) -> Option<(Vec<String>, Vec<String>)> {
    let mut b_remaining: Vec<&String> = b.iter().collect();
    let mut a_only = Vec::new();
    for d in a {
        if let Some(pos) = b_remaining.iter().position(|x| *x == d) {
            b_remaining.remove(pos);
        } else {
            a_only.push(d.clone());
        }
    }
    let b_only: Vec<String> = b_remaining.into_iter().cloned().collect();
    (!a_only.is_empty() || !b_only.is_empty()).then_some((a_only, b_only))
}

/// Join lines with a visible separator, capped on a char boundary (witness hole text).
fn cap_join(lines: &[String]) -> String {
    const CAP: usize = 160;
    let joined = lines.join(" ⏎ ");
    if joined.len() > CAP {
        let mut end = CAP;
        while !joined.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}…", &joined[..end])
    } else {
        joined
    }
}

/// Trimmed, length-capped source text of lines `start..=end` of `file`, for a witness
/// hole. Multi-line spots are joined with a visible separator; the result is capped on
/// a char boundary so the JSON stays compact.
fn witness_spot_text(lines: &mut FileLineCache, file: &str, start: u32, end: u32) -> String {
    const TEXT_CAP: usize = 160;
    let Some(slice) = lines.slice(file, start, end) else {
        return String::new();
    };
    let joined = slice
        .iter()
        .map(|l| l.trim())
        .collect::<Vec<_>>()
        .join(" ⏎ ");
    let joined = joined.trim();
    if joined.len() > TEXT_CAP {
        let mut end = TEXT_CAP;
        while !joined.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}…", &joined[..end])
    } else {
        joined.to_string()
    }
}
