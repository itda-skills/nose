use super::*;

pub fn first_party_semantic_pack() -> SemanticPackSummary {
    SemanticPackSummary {
        id: FIRST_PARTY_PACK_ID.to_string(),
        hash: semantic_pack_hash(FIRST_PARTY_PACK_ID),
        kind: SemanticPackKind::LanguagePack,
        version: env!("CARGO_PKG_VERSION").to_string(),
        display_name: "nose first-party semantic kernel".to_string(),
        trust: PackTrust::DefaultFirstParty,
        enabled_by_default: true,
        source: SemanticPackSource::CompiledFirstParty,
        influence: SemanticPackInfluence::EvidenceAndContracts,
        manifest_path: None,
        provider: "Corca, Inc.".to_string(),
        repository: "https://github.com/corca-ai/nose".to_string(),
        license: "MIT".to_string(),
        supported_languages: Vec::new(),
        counts: SemanticPackCounts {
            evidence_producers: 0,
            contracts: 0,
            value_laws: 0,
            positive_fixtures: 0,
            hard_negatives: 0,
        },
    }
}

pub fn python_stdlib_type_domain_pack() -> SemanticPackSummary {
    SemanticPackSummary {
        id: PYTHON_STDLIB_TYPE_DOMAIN_PACK_ID.to_string(),
        hash: semantic_pack_hash(PYTHON_STDLIB_TYPE_DOMAIN_PACK_ID),
        kind: SemanticPackKind::StdlibPack,
        version: env!("CARGO_PKG_VERSION").to_string(),
        display_name: "nose Python stdlib type-domain pack".to_string(),
        trust: PackTrust::DefaultFirstParty,
        enabled_by_default: true,
        source: SemanticPackSource::CompiledFirstParty,
        influence: SemanticPackInfluence::EvidenceAndContracts,
        manifest_path: None,
        provider: "Corca, Inc.".to_string(),
        repository: "https://github.com/corca-ai/nose".to_string(),
        license: "MIT".to_string(),
        supported_languages: vec!["python".to_string()],
        counts: SemanticPackCounts {
            evidence_producers: 1,
            contracts: 1,
            value_laws: 0,
            positive_fixtures: PYTHON_STDLIB_TYPE_DOMAIN_ALIAS_CONTRACTS.len(),
            hard_negatives: 2,
        },
    }
}

pub fn first_party_value_law_pack() -> SemanticPackSummary {
    let laws = pack_facing_value_laws();
    SemanticPackSummary {
        id: FIRST_PARTY_VALUE_LAW_PACK_ID.to_string(),
        hash: semantic_pack_hash(FIRST_PARTY_VALUE_LAW_PACK_ID),
        kind: SemanticPackKind::LawPack,
        version: env!("CARGO_PKG_VERSION").to_string(),
        display_name: "nose value-graph law pack".to_string(),
        trust: PackTrust::DefaultFirstParty,
        enabled_by_default: true,
        source: SemanticPackSource::CompiledFirstParty,
        influence: SemanticPackInfluence::EvidenceAndContracts,
        manifest_path: None,
        provider: "Corca, Inc.".to_string(),
        repository: "https://github.com/corca-ai/nose".to_string(),
        license: "MIT".to_string(),
        supported_languages: Vec::new(),
        counts: SemanticPackCounts {
            evidence_producers: 0,
            contracts: 0,
            value_laws: laws.len(),
            positive_fixtures: laws
                .iter()
                .map(|law| {
                    law.conformance_refs
                        .iter()
                        .filter(|id| !id.contains("hard-negative"))
                        .count()
                })
                .sum(),
            hard_negatives: laws
                .iter()
                .map(|law| {
                    law.conformance_refs
                        .iter()
                        .filter(|id| id.contains("hard-negative"))
                        .count()
                })
                .sum(),
        },
    }
}

pub(super) fn compiled_first_party_packs() -> Vec<SemanticPackSummary> {
    vec![
        first_party_semantic_pack(),
        python_stdlib_type_domain_pack(),
        first_party_value_law_pack(),
    ]
}

pub(super) fn is_compiled_first_party_pack_id(pack_id: &str) -> bool {
    compiled_first_party_packs()
        .iter()
        .any(|pack| pack.id == pack_id)
}
