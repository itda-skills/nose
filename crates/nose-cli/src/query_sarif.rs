use anyhow::Result;

use crate::baseline;
use crate::query_opportunities::family_hint;
use crate::report_text::plural;

/// Build a SARIF 2.1.0 document — one result per family, every member site a
/// location so SARIF consumers can annotate each. The first location is primary;
/// the rest are `relatedLocations`.
/// `shown` is the (possibly `--top`-truncated) slice that gets emitted; `total` is the
/// full active-family count before truncation. A SARIF consumer
/// otherwise can't tell a truncated upload from a complete one, so the run carries both
/// counts in `properties` and — when families were hidden — a `note` notification telling
/// the reader to pass `--top 0` for the full set.
pub(crate) fn refactor_sarif(
    shown: &[&nose_detect::RefactorFamily],
    total: usize,
) -> Result<String> {
    use serde_json::json;
    let phys = |l: &nose_detect::Loc| {
        json!({
            "physicalLocation": {
                "artifactLocation": { "uri": l.file },
                "region": { "startLine": l.start_line, "endLine": l.end_line }
            }
        })
    };
    let results: Vec<_> = shown
        .iter()
        .map(|f| {
            let msg = format!(
                "{} — {} sites, {} {}, ~{} duplicated lines (sim {:.2})",
                family_hint(f),
                f.members,
                f.files,
                plural(f.files, "file", "files"),
                f.dup_lines,
                f.mean_score
            );
            json!({
                "ruleId": "duplicate-family",
                "level": "warning",
                "message": { "text": msg },
                "locations": f.locations.first().map(phys).into_iter().collect::<Vec<_>>(),
                "relatedLocations": f.locations.iter().skip(1).map(phys).collect::<Vec<_>>(),
                "properties": { "family_id": baseline::family_id(f) },
            })
        })
        .collect();
    let mut run = json!({
        "tool": { "driver": {
            "name": "nose",
            "informationUri": "https://github.com/",
            "version": env!("CARGO_PKG_VERSION"),
            "rules": [{
                "id": "duplicate-family",
                "name": "DuplicateFamily",
                "shortDescription": { "text": "Duplicated code worth refactoring" }
            }]
        }},
        "results": results,
        "properties": { "total_families": total, "shown_families": shown.len() },
    });
    if shown.len() < total {
        run["invocations"] = json!([{
            "executionSuccessful": true,
            "toolExecutionNotifications": [{
                "level": "note",
                "message": { "text": format!(
                    "Showing {} of {total} clone families (the --top limit). \
                     Pass top=0 to emit every family.",
                    shown.len()
                ) }
            }]
        }]);
    }
    let doc = json!({
        "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
        "version": "2.1.0",
        "runs": [run],
    });
    Ok(serde_json::to_string_pretty(&doc)?)
}
