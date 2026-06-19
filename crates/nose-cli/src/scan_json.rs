use crate::legacy_prelude::*;

#[derive(serde::Serialize)]
pub(super) struct ScanJsonReport<'a> {
    schema_version: u32,
    tool_version: &'static str,
    scope: ScanJsonScope<'a>,
    semantic_packs: Vec<ScanJsonSemanticPack<'a>>,
    ranking: ScanJsonRanking,
    #[serde(skip_serializing_if = "Option::is_none")]
    baseline: Option<&'a BaselineSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ignore: Option<ignores::IgnoreSummary<'a>>,
    families: Vec<ScanJsonFamily<'a>>,
    /// Reinvented-helper containment findings (additive, experimental): a unit that
    /// reimplements an existing pure single-return helper inline instead of calling it.
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    reinvented_helpers: &'a [nose_detect::ReinventedHelper],
    #[serde(skip_serializing_if = "Vec::is_empty")]
    ignored_families: Vec<ScanJsonIgnoredFamily<'a>>,
}

#[derive(serde::Serialize)]
struct ScanJsonScope<'a> {
    files: usize,
    languages: Vec<ScanJsonLanguage<'a>>,
}

#[derive(serde::Serialize)]
struct ScanJsonLanguage<'a> {
    language: &'a str,
    files: usize,
}

#[derive(serde::Serialize)]
struct ScanJsonSemanticPack<'a> {
    id: &'a str,
    hash: String,
    kind: &'static str,
    version: &'a str,
    display_name: &'a str,
    trust: &'static str,
    enabled_by_default: bool,
    source: &'static str,
    influence: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    provider: &'a str,
    repository: &'a str,
    license: &'a str,
    supported_languages: &'a [String],
    counts: ScanJsonSemanticPackCounts,
}

#[derive(serde::Serialize)]
struct ScanJsonSemanticPackCounts {
    evidence_producers: usize,
    contracts: usize,
    value_laws: usize,
    positive_fixtures: usize,
    hard_negatives: usize,
}

#[derive(serde::Serialize)]
struct ScanJsonRanking {
    sort: &'static str,
    total_families: usize,
    shown_families: usize,
    limit: Option<usize>,
    surface_counts: ScanJsonSurfaceCounts,
}

#[derive(Default, serde::Serialize)]
struct ScanJsonSurfaceCounts {
    #[serde(rename = "default")]
    default_count: usize,
    review: usize,
    hidden: usize,
    debug: usize,
    /// Families classified as generated source, including generated-header families
    /// and CSS source-plus-compiled/minified build pipelines (#224).
    generated: usize,
    /// Families whose every member span is only import/include/use/re-export
    /// declarations — real duplication with no extraction action (the human
    /// report omits these from default output too).
    declaration: usize,
    /// Unproven families whose extracted helper would be mostly parameters
    /// (`shallow-extraction`) — a decidable non-action class the human report
    /// omits from default output (kept here and in `--top 0` JSON).
    shallow: usize,
    fragments: ScanJsonFragmentSurfaceCounts,
}

#[derive(Default, serde::Serialize)]
struct ScanJsonFragmentSurfaceCounts {
    total: usize,
    #[serde(rename = "default")]
    default_count: usize,
    review: usize,
    hidden: usize,
    debug: usize,
}

impl ScanJsonSurfaceCounts {
    fn from_families(
        families: &[nose_detect::RefactorFamily],
        overrides: &SurfaceOverrides,
    ) -> Self {
        let mut counts = Self::default();
        for family in families {
            let surface = effective_surface(family, overrides);
            counts.bump(surface);
            if family.locations.iter().any(|loc| loc.is_fragment) {
                counts.fragments.total += 1;
                counts.fragments.bump(surface);
            }
        }
        counts
    }

    fn bump(&mut self, surface: &str) {
        match surface {
            "default" => self.default_count += 1,
            "review" => self.review += 1,
            "hidden" => self.hidden += 1,
            "debug" => self.debug += 1,
            "generated" => self.generated += 1,
            "declaration" => self.declaration += 1,
            "shallow" => self.shallow += 1,
            _ => self.debug += 1,
        }
    }
}

impl ScanJsonFragmentSurfaceCounts {
    fn bump(&mut self, surface: &str) {
        match surface {
            "default" => self.default_count += 1,
            "review" => self.review += 1,
            "hidden" => self.hidden += 1,
            "debug" => self.debug += 1,
            _ => self.debug += 1,
        }
    }
}

#[derive(serde::Serialize)]
struct ScanJsonFamily<'a> {
    family_id: String,
    #[serde(flatten)]
    family: &'a nose_detect::RefactorFamily,
    recommended_surface: &'static str,
    /// The decidable, classification-not-verdict reason this family is NOT a clean
    /// default-surface refactor candidate: `shallow-extraction`, `trivial`,
    /// `declaration-run`, or `generated-source`. Absent for a clean candidate (#11).
    #[serde(skip_serializing_if = "Option::is_none")]
    actionability_reason: Option<&'static str>,
    /// The decidable structural shape of the fix IF a clean candidate is acted upon —
    /// `call-existing-helper` / `extract-helper` / `extract-method-from-block` /
    /// `consolidate-type` / `extract-base-class` / `consolidate-cross-language`. NOT a
    /// worth-it claim (§2). Present only when `actionability_reason` is absent (#11).
    #[serde(skip_serializing_if = "Option::is_none")]
    extraction_shape: Option<&'static str>,
    /// Present when this family is an overlapping slice of another default-
    /// surface family: the id of that primary. Consumers triaging
    /// opportunities can fold slices under their primary.
    #[serde(skip_serializing_if = "Option::is_none")]
    overlap_primary_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    baseline_status: Option<&'static str>,
}

#[derive(serde::Serialize)]
struct ScanJsonIgnoredFamily<'a> {
    family_id: String,
    #[serde(flatten)]
    family: &'a nose_detect::RefactorFamily,
    recommended_surface: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    actionability_reason: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    extraction_shape: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    baseline_status: Option<&'static str>,
    ignore: &'a ignores::IgnoreMatch,
}

pub(super) struct ScanJsonInput<'a> {
    pub(super) scope: &'a ScanScope,
    pub(super) reinvented: &'a [nose_detect::ReinventedHelper],
    pub(super) sort: SortKey,
    pub(super) top: usize,
    pub(super) families: &'a [nose_detect::RefactorFamily],
    pub(super) shown: &'a [&'a nose_detect::RefactorFamily],
    pub(super) baseline: Option<&'a BaselineComparison>,
    pub(super) ignore_set: Option<&'a ignores::IgnoreSet>,
    pub(super) ignored_families: &'a [IgnoredFamily],
    pub(super) semantic_packs: &'a nose_semantics::SemanticPackSet,
    pub(super) overrides: &'a SurfaceOverrides,
    pub(super) opportunities: &'a OpportunityGroups,
}

fn scan_json_family<'a>(
    family: &'a nose_detect::RefactorFamily,
    statuses: Option<&std::collections::HashMap<u64, BaselineStatus>>,
    overrides: &SurfaceOverrides,
    opportunities: &OpportunityGroups,
) -> ScanJsonFamily<'a> {
    let family_id = baseline::family_id(family);
    let actionability_reason = family_actionability_reason(family, overrides);
    ScanJsonFamily {
        overlap_primary_id: opportunities.primary_of.get(&family_id).cloned(),
        family_id,
        family,
        recommended_surface: effective_surface(family, overrides),
        actionability_reason,
        // The structural shape is meaningful only for a clean candidate;
        // a non-action family has nothing to extract.
        extraction_shape: actionability_reason
            .is_none()
            .then(|| family.extraction_shape()),
        baseline_status: statuses
            .and_then(|s| s.get(&baseline::family_key(family)))
            .map(BaselineStatus::as_str),
    }
}

fn scan_json_ignored_family<'a>(
    ignored: &'a IgnoredFamily,
    statuses: Option<&std::collections::HashMap<u64, BaselineStatus>>,
    overrides: &SurfaceOverrides,
) -> ScanJsonIgnoredFamily<'a> {
    let actionability_reason = family_actionability_reason(&ignored.family, overrides);
    ScanJsonIgnoredFamily {
        family_id: baseline::family_id(&ignored.family),
        family: &ignored.family,
        recommended_surface: effective_surface(&ignored.family, overrides),
        actionability_reason,
        extraction_shape: actionability_reason
            .is_none()
            .then(|| ignored.family.extraction_shape()),
        baseline_status: statuses
            .and_then(|s| s.get(&baseline::family_key(&ignored.family)))
            .map(BaselineStatus::as_str),
        ignore: &ignored.ignore,
    }
}

impl<'a> ScanJsonReport<'a> {
    pub(super) fn new(input: ScanJsonInput<'a>) -> Self {
        let statuses = input.baseline.map(|b| &b.statuses);
        ScanJsonReport {
            schema_version: schema_versions::SCAN_JSON_SCHEMA_VERSION,
            tool_version: env!("CARGO_PKG_VERSION"),
            scope: ScanJsonScope {
                files: input.scope.files,
                languages: input
                    .scope
                    .langs
                    .iter()
                    .map(|(language, files)| ScanJsonLanguage {
                        language,
                        files: *files,
                    })
                    .collect(),
            },
            semantic_packs: input
                .semantic_packs
                .packs()
                .iter()
                .map(|pack| ScanJsonSemanticPack {
                    id: &pack.id,
                    hash: pack.hash_hex(),
                    kind: pack.kind.as_str(),
                    version: &pack.version,
                    display_name: &pack.display_name,
                    trust: pack.trust.as_manifest_str(),
                    enabled_by_default: pack.enabled_by_default,
                    source: pack.source.as_str(),
                    influence: pack.influence.as_str(),
                    path: pack
                        .manifest_path
                        .as_ref()
                        .map(|path| path.display().to_string()),
                    provider: &pack.provider,
                    repository: &pack.repository,
                    license: &pack.license,
                    supported_languages: &pack.supported_languages,
                    counts: ScanJsonSemanticPackCounts {
                        evidence_producers: pack.counts.evidence_producers,
                        contracts: pack.counts.contracts,
                        value_laws: pack.counts.value_laws,
                        positive_fixtures: pack.counts.positive_fixtures,
                        hard_negatives: pack.counts.hard_negatives,
                    },
                })
                .collect(),
            ranking: ScanJsonRanking {
                sort: input.sort.json_name(),
                total_families: input.families.len(),
                shown_families: input.shown.len(),
                limit: (input.top != 0).then_some(input.top),
                surface_counts: ScanJsonSurfaceCounts::from_families(
                    input.families,
                    input.overrides,
                ),
            },
            baseline: input.baseline.map(|b| &b.summary),
            ignore: input
                .ignore_set
                .map(|set| set.summary(input.ignored_families.len())),
            families: input
                .shown
                .iter()
                .map(|family| {
                    scan_json_family(family, statuses, input.overrides, input.opportunities)
                })
                .collect(),
            reinvented_helpers: input.reinvented,
            ignored_families: input
                .ignored_families
                .iter()
                .map(|ignored| scan_json_ignored_family(ignored, statuses, input.overrides))
                .collect(),
        }
    }
}
