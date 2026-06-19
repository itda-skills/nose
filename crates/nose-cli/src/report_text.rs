pub(crate) fn semantic_pack_summary_line(
    packs: &nose_semantics::SemanticPackSet,
) -> Option<String> {
    let first_party_count = packs
        .packs()
        .iter()
        .filter(|pack| pack.source == nose_semantics::SemanticPackSource::CompiledFirstParty)
        .count();
    let local = packs
        .packs()
        .iter()
        .filter(|pack| pack.source == nose_semantics::SemanticPackSource::LocalManifest)
        .map(|pack| format!("{}@{} ({})", pack.id, pack.version, pack.influence.as_str()))
        .collect::<Vec<_>>();
    (!local.is_empty()).then(|| {
        format!(
            "semantic packs: {first_party_count} first-party default · {} local opt-in: {}",
            local.len(),
            local.join(", ")
        )
    })
}

// ===================== shared report text helpers =====================

/// The right noun form for a count: singular when `n == 1`, plural otherwise (so `0`
/// reads "0 families"). Returns just the noun — the caller prints the number.
pub(crate) fn plural<'a>(n: usize, one: &'a str, many: &'a str) -> &'a str {
    if n == 1 {
        one
    } else {
        many
    }
}
