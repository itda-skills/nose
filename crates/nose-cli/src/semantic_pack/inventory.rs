use anyhow::Result;

pub(crate) const INVENTORY_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, PartialEq, clap::ValueEnum)]
pub(crate) enum InventoryFormat {
    Human,
    Json,
}

#[derive(serde::Serialize)]
struct InventoryJsonReport {
    schema_version: u32,
    status: &'static str,
    totals: InventoryJsonTotals,
    evidence_policy: InventoryJsonEvidencePolicy,
    packs: Vec<InventoryJsonPack>,
}

#[derive(serde::Serialize)]
struct InventoryJsonTotals {
    packs: usize,
    builtin_packs: usize,
    exact_capable_packs: usize,
    packs_needing_coverage: usize,
    positive_fixtures: usize,
    hard_negatives: usize,
    conformance_refs: usize,
    unsupported_refs: usize,
}

#[derive(serde::Serialize)]
struct InventoryJsonEvidencePolicy {
    product_output: &'static str,
    performance: &'static str,
}

#[derive(serde::Serialize)]
struct InventoryJsonPack {
    id: String,
    hash: String,
    kind: &'static str,
    version: String,
    display_name: String,
    trust: &'static str,
    enabled_by_default: bool,
    source: &'static str,
    influence: &'static str,
    provider: String,
    repository: String,
    license: String,
    supported_languages: Vec<String>,
    supported_packages: Vec<String>,
    declarations: InventoryJsonDeclarations,
    conformance: InventoryJsonConformance,
    audit: InventoryJsonAudit,
}

#[derive(serde::Serialize)]
struct InventoryJsonDeclarations {
    evidence_producers: Vec<String>,
    source_fact_producers: Vec<String>,
    contracts: Vec<String>,
    type_domain_aliases: Vec<String>,
    value_laws: Vec<String>,
    counts: InventoryJsonCounts,
}

#[derive(serde::Serialize)]
struct InventoryJsonCounts {
    evidence_producers: usize,
    contracts: usize,
    value_laws: usize,
    positive_fixtures: usize,
    hard_negatives: usize,
}

#[derive(serde::Serialize)]
struct InventoryJsonConformance {
    refs: Vec<String>,
    positive_refs: Vec<String>,
    hard_negative_refs: Vec<String>,
    unsupported_refs: Vec<String>,
}

#[derive(serde::Serialize)]
struct InventoryJsonAudit {
    exact_capable: bool,
    coverage_status: &'static str,
    gaps: Vec<&'static str>,
    product_output_evidence: &'static str,
    performance_evidence: &'static str,
}

impl InventoryJsonReport {
    fn new() -> Self {
        let builtin = nose_semantics::SemanticPackSet::builtin_only();
        let packs = nose_semantics::builtin_pack_descriptors()
            .iter()
            .map(|descriptor| {
                let summary = builtin
                    .packs()
                    .iter()
                    .find(|summary| summary.id == descriptor.id)
                    .expect("builtin descriptor should have a matching summary");
                InventoryJsonPack::new(summary, *descriptor)
            })
            .collect::<Vec<_>>();
        let exact_capable_packs = packs.iter().filter(|pack| pack.audit.exact_capable).count();
        let packs_needing_coverage = packs
            .iter()
            .filter(|pack| pack.audit.coverage_status == "needs-coverage")
            .count();
        Self {
            schema_version: INVENTORY_SCHEMA_VERSION,
            status: if packs_needing_coverage == 0 {
                "ok"
            } else {
                "needs-coverage"
            },
            totals: InventoryJsonTotals {
                packs: packs.len(),
                builtin_packs: packs.len(),
                exact_capable_packs,
                packs_needing_coverage,
                positive_fixtures: packs
                    .iter()
                    .map(|pack| pack.declarations.counts.positive_fixtures)
                    .sum(),
                hard_negatives: packs
                    .iter()
                    .map(|pack| pack.declarations.counts.hard_negatives)
                    .sum(),
                conformance_refs: packs.iter().map(|pack| pack.conformance.refs.len()).sum(),
                unsupported_refs: packs
                    .iter()
                    .map(|pack| pack.conformance.unsupported_refs.len())
                    .sum(),
            },
            evidence_policy: InventoryJsonEvidencePolicy {
                product_output: "required-on-implementation-pr",
                performance: "required-on-implementation-pr",
            },
            packs,
        }
    }
}

impl InventoryJsonPack {
    fn new(
        summary: &nose_semantics::SemanticPackSummary,
        descriptor: nose_semantics::BuiltinPackDescriptor,
    ) -> Self {
        let declarations = InventoryJsonDeclarations::new(summary, descriptor);
        let conformance = InventoryJsonConformance::new(descriptor);
        let exact_capable = declarations.exact_capable(&conformance);
        let audit = InventoryJsonAudit::new(exact_capable, &conformance, &declarations.counts);
        Self {
            id: summary.id.clone(),
            hash: summary.hash_hex(),
            kind: summary.kind.as_str(),
            version: summary.version.clone(),
            display_name: summary.display_name.clone(),
            trust: summary.trust.as_manifest_str(),
            enabled_by_default: summary.enabled_by_default,
            source: summary.source.as_str(),
            influence: summary.influence.as_str(),
            provider: summary.provider.clone(),
            repository: summary.repository.clone(),
            license: summary.license.clone(),
            supported_languages: summary.supported_languages.clone(),
            supported_packages: descriptor
                .supported_packages
                .iter()
                .map(|package| (*package).to_string())
                .collect(),
            declarations,
            conformance,
            audit,
        }
    }
}

impl InventoryJsonDeclarations {
    fn new(
        summary: &nose_semantics::SemanticPackSummary,
        descriptor: nose_semantics::BuiltinPackDescriptor,
    ) -> Self {
        Self {
            evidence_producers: ids_to_strings(descriptor.evidence_producer_ids),
            source_fact_producers: ids_to_strings(descriptor.source_fact_producer_ids),
            contracts: ids_to_strings(descriptor.contract_ids),
            type_domain_aliases: descriptor
                .type_domain_alias_contracts
                .iter()
                .map(alias_contract_coordinate)
                .collect(),
            value_laws: descriptor
                .value_law_ids()
                .into_iter()
                .map(str::to_string)
                .collect(),
            counts: InventoryJsonCounts {
                evidence_producers: summary.counts.evidence_producers,
                contracts: summary.counts.contracts,
                value_laws: summary.counts.value_laws,
                positive_fixtures: summary.counts.positive_fixtures,
                hard_negatives: summary.counts.hard_negatives,
            },
        }
    }

    fn exact_capable(&self, conformance: &InventoryJsonConformance) -> bool {
        let has_declared_exact_rows = self.counts.contracts > 0 || self.counts.value_laws > 0;
        let has_conformance_backed_producers = !conformance.refs.is_empty()
            && (!self.evidence_producers.is_empty() || !self.source_fact_producers.is_empty());
        has_declared_exact_rows || has_conformance_backed_producers
    }
}

impl InventoryJsonConformance {
    fn new(descriptor: nose_semantics::BuiltinPackDescriptor) -> Self {
        let refs = descriptor
            .conformance_refs()
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>();
        Self {
            positive_refs: refs
                .iter()
                .filter(|reference| reference.contains("-positive"))
                .cloned()
                .collect(),
            hard_negative_refs: refs
                .iter()
                .filter(|reference| reference.contains("hard-negative"))
                .cloned()
                .collect(),
            unsupported_refs: refs
                .iter()
                .filter(|reference| reference.contains("unsupported"))
                .cloned()
                .collect(),
            refs,
        }
    }
}

impl InventoryJsonAudit {
    fn new(
        exact_capable: bool,
        conformance: &InventoryJsonConformance,
        counts: &InventoryJsonCounts,
    ) -> Self {
        let gaps = coverage_gaps(
            exact_capable,
            conformance,
            counts,
            &conformance.positive_refs,
            &conformance.hard_negative_refs,
        );
        let coverage_status = if exact_capable {
            if gaps.is_empty() {
                "covered"
            } else {
                "needs-coverage"
            }
        } else {
            "tracked-no-exact-rows"
        };
        Self {
            exact_capable,
            coverage_status,
            gaps,
            product_output_evidence: if exact_capable {
                "required-on-change"
            } else {
                "not-required-for-descriptor-only-change"
            },
            performance_evidence: if exact_capable {
                "required-on-change"
            } else {
                "not-required-for-descriptor-only-change"
            },
        }
    }
}

fn ids_to_strings(ids: &[&str]) -> Vec<String> {
    ids.iter().map(|id| (*id).to_string()).collect()
}

fn alias_contract_coordinate(contract: &nose_semantics::BuiltinTypeDomainAliasContract) -> String {
    format!(
        "{}:{}.{}:{:?}",
        contract.contract_id, contract.module, contract.exported, contract.domain
    )
    .to_ascii_lowercase()
}

fn coverage_gaps(
    exact_capable: bool,
    conformance: &InventoryJsonConformance,
    counts: &InventoryJsonCounts,
    positive_refs: &[String],
    hard_negative_refs: &[String],
) -> Vec<&'static str> {
    if !exact_capable {
        return Vec::new();
    }
    let mut gaps = Vec::new();
    if positive_refs.is_empty() {
        gaps.push("exact-capable-missing-positive-fixtures");
    }
    if hard_negative_refs.is_empty() {
        gaps.push("exact-capable-missing-hard-negatives");
    }
    if conformance
        .refs
        .iter()
        .any(|reference| !reference.contains("-positive") && !reference.contains("hard-negative"))
    {
        gaps.push("conformance-ref-polarity-unclassified");
    }
    if counts.positive_fixtures != positive_refs.len()
        || counts.hard_negatives != hard_negative_refs.len()
    {
        gaps.push("fixture-count-mismatch");
    }
    gaps
}

pub(crate) fn cmd_inventory(format: InventoryFormat) -> Result<()> {
    let report = InventoryJsonReport::new();
    match format {
        InventoryFormat::Human => print_inventory_human(&report),
        InventoryFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
    }
    Ok(())
}

fn print_inventory_human(report: &InventoryJsonReport) {
    println!("builtin semantic-pack inventory: {}", report.status);
    println!(
        "packs: {}; exact-capable: {}; needing coverage: {}; fixtures: {} positive, {} hard-negative",
        report.totals.packs,
        report.totals.exact_capable_packs,
        report.totals.packs_needing_coverage,
        report.totals.positive_fixtures,
        report.totals.hard_negatives
    );
    for pack in &report.packs {
        println!(
            "  {}: {} ({} producer(s), {} contract(s), {} law(s), {} conformance ref(s))",
            pack.id,
            pack.audit.coverage_status,
            pack.declarations.counts.evidence_producers,
            pack.declarations.counts.contracts,
            pack.declarations.counts.value_laws,
            pack.conformance.refs.len()
        );
        if !pack.audit.gaps.is_empty() {
            println!("    gaps: {}", pack.audit.gaps.join(", "));
        }
    }
}
