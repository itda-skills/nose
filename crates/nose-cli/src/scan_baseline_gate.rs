use crate::legacy_prelude::*;

pub(crate) fn write_scan_baseline(
    args: &ScanArgs,
    families: &[nose_detect::RefactorFamily],
) -> Result<()> {
    let path = args
        .baseline
        .as_ref()
        .expect("--write-baseline requires --baseline");
    baseline::write(path, families, family_hint)
        .with_context(|| format!("writing baseline {}", path.display()))?;
    eprintln!(
        "nose: wrote baseline of {} families to {}",
        families.len(),
        path.display()
    );
    Ok(())
}

/// Compare against an accepted baseline: build the comparison, then drop already-accepted
/// families in place so only new/changed duplication is reported and gated. `None` when no
/// `--baseline` is set. (`--write-baseline` is handled earlier, before this runs.)
pub(crate) fn apply_scan_baseline(
    args: &ScanArgs,
    families: &mut Vec<nose_detect::RefactorFamily>,
) -> Result<Option<BaselineComparison>> {
    let Some(path) = args.baseline.as_ref() else {
        return Ok(None);
    };
    let accepted = baseline::load(path)?;
    let comparison = BaselineComparison::new(path, &accepted, families);
    families.retain(|f| !accepted.keys.contains(&baseline::family_key(f)));
    Ok(Some(comparison))
}

/// `since=<baseline>`: compare to a saved snapshot WITHOUT hiding anything — the temporal
/// exploration lens. Unlike `--baseline` (which drops accepted families for the gate), this
/// keeps every family and lets the caller slice by the `status` field. `nose query <path>
/// since=B status=new --fail-on any` is the composable equivalent of `--baseline B --fail-on
/// new`; the two baseline paths converge as the gate folds into query (the review-unification).
pub(crate) fn compare_since(
    path: &str,
    families: &[nose_detect::RefactorFamily],
) -> Result<BaselineComparison> {
    let snapshot = baseline::load(std::path::Path::new(path))?;
    Ok(BaselineComparison::new(
        std::path::Path::new(path),
        &snapshot,
        families,
    ))
}

/// A family's status against a `since=` snapshot: `new`/`changed` (in the comparison) or
/// `unchanged` (present in the snapshot, so absent from the changed/new map).
pub(crate) fn family_status(
    f: &nose_detect::RefactorFamily,
    cmp: &BaselineComparison,
) -> &'static str {
    cmp.statuses
        .get(&baseline::family_key(f))
        .map_or("unchanged", BaselineStatus::as_str)
}

pub(crate) fn partition_ignored(
    families: Vec<nose_detect::RefactorFamily>,
    ignore_set: Option<&ignores::IgnoreSet>,
) -> (Vec<nose_detect::RefactorFamily>, Vec<IgnoredFamily>) {
    let Some(ignore_set) = ignore_set else {
        return (families, Vec::new());
    };
    let mut active = Vec::with_capacity(families.len());
    let mut ignored_families = Vec::new();
    for family in families {
        if let Some(ignore) = ignore_set.match_family(&family) {
            ignored_families.push(IgnoredFamily { family, ignore });
        } else {
            active.push(family);
        }
    }
    (active, ignored_families)
}
