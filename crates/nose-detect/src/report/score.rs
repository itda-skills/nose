/// Compute the refactoring-value score for a family's metrics.
///
/// `dup_lines` is the backbone (how much code disappears). Mean similarity scales
/// it (a 0.7-similar family needs more manual work than a 0.99 one). The design
/// multiplier rewards spread: cross-module duplication is a missing abstraction;
/// cross-language is a notable (if harder) design signal.
pub(super) fn refactor_value(
    mean_lines: u32,
    members: usize,
    mean_score: f64,
    files: usize,
    modules: usize,
    languages: usize,
) -> f64 {
    mean_lines as f64 * effective_copies(members) * mean_score * spread(files, modules, languages)
}

/// Copies, dampened. Removable code grows with the number of copies, but with
/// DIMINISHING returns: the first few dedups capture the design win, whereas a
/// fragment repeated across hundreds of sites is almost always an idiom / generated
/// / boilerplate pattern (a Javadoc nav block, test scaffolding), not an extractable
/// abstraction. So the copy count is linear up to a small knee, then
/// square-root-dampened — fanout no longer rewards the ranking *linearly* (a 400-copy
/// family is ~20× a 2-copy one, not 400×). The reported `dup_lines` stays the honest
/// `mean_lines × (members−1)`; only the ranking scores are dampened.
pub(super) fn effective_copies(members: usize) -> f64 {
    let copies = members.saturating_sub(1) as f64;
    const KNEE: f64 = 6.0;
    if copies <= KNEE {
        copies
    } else {
        KNEE + (copies - KNEE).sqrt()
    }
}

/// Design-spread multiplier: cross-module duplication is a missing abstraction;
/// cross-language is a notable (if harder) design signal.
pub(super) fn spread(files: usize, modules: usize, languages: usize) -> f64 {
    1.0 + 0.30 * (files.min(8) as f64 - 1.0).max(0.0)
        + 0.50 * (modules.min(6) as f64 - 1.0).max(0.0)
        + 0.50 * (languages as f64 - 1.0).max(0.0)
}
