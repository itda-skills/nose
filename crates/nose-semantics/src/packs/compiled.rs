use super::*;

const NO_LANGUAGES: &[&str] = &[];
const PYTHON_LANGUAGE: &[&str] = &["python"];
const NO_PACKAGES: &[&str] = &[];
const PYTHON_STDLIB_TYPE_DOMAIN_PACKAGES: &[&str] = &["typing", "collections.abc", "asyncio"];
const NO_IDS: &[&str] = &[];
const PYTHON_STDLIB_TYPE_DOMAIN_CONTRACT_IDS: &[&str] =
    &["python.stdlib.type-domain-alias.contract"];
const PYTHON_STDLIB_TYPE_DOMAIN_PRODUCER_IDS: &[&str] = &[PYTHON_STDLIB_TYPE_DOMAIN_PRODUCER_ID];
const PYTHON_STDLIB_TYPE_DOMAIN_HARD_NEGATIVE_REFS: &[&str] =
    &["python-typing-domain-wrong-module-hard-negative"];

#[derive(Clone, Copy, Debug)]
pub struct BuiltinPackDescriptor {
    pub id: &'static str,
    pub kind: SemanticPackKind,
    pub display_name: &'static str,
    pub trust: PackTrust,
    pub enabled_by_default: bool,
    pub supported_languages: &'static [&'static str],
    pub supported_packages: &'static [&'static str],
    pub evidence_producer_ids: &'static [&'static str],
    pub contract_ids: &'static [&'static str],
    static_value_law_ids: &'static [&'static str],
    dynamic_value_law_ids: Option<fn() -> Vec<&'static str>>,
    static_conformance_refs: &'static [&'static str],
    dynamic_conformance_refs: Option<fn() -> Vec<&'static str>>,
    counts: fn() -> SemanticPackCounts,
}

impl BuiltinPackDescriptor {
    pub fn value_law_ids(self) -> Vec<&'static str> {
        let mut ids = self.static_value_law_ids.to_vec();
        if let Some(dynamic_ids) = self.dynamic_value_law_ids {
            ids.extend(dynamic_ids());
        }
        ids
    }

    pub fn conformance_refs(self) -> Vec<&'static str> {
        let mut refs = self.static_conformance_refs.to_vec();
        if let Some(dynamic_refs) = self.dynamic_conformance_refs {
            refs.extend(dynamic_refs());
        }
        refs
    }

    pub fn counts(self) -> SemanticPackCounts {
        (self.counts)()
    }

    fn summary(self) -> SemanticPackSummary {
        SemanticPackSummary {
            id: self.id.to_string(),
            hash: semantic_pack_hash(self.id),
            kind: self.kind,
            version: env!("CARGO_PKG_VERSION").to_string(),
            display_name: self.display_name.to_string(),
            trust: self.trust,
            enabled_by_default: self.enabled_by_default,
            source: SemanticPackSource::CompiledFirstParty,
            influence: SemanticPackInfluence::EvidenceAndContracts,
            manifest_path: None,
            provider: "Corca, Inc.".to_string(),
            repository: "https://github.com/corca-ai/nose".to_string(),
            license: "MIT".to_string(),
            supported_languages: self
                .supported_languages
                .iter()
                .map(|language| (*language).to_string())
                .collect(),
            counts: self.counts(),
        }
    }
}

fn empty_counts() -> SemanticPackCounts {
    SemanticPackCounts {
        evidence_producers: 0,
        contracts: 0,
        value_laws: 0,
        positive_fixtures: 0,
        hard_negatives: 0,
    }
}

fn python_stdlib_type_domain_counts() -> SemanticPackCounts {
    SemanticPackCounts {
        evidence_producers: PYTHON_STDLIB_TYPE_DOMAIN_PRODUCER_IDS.len(),
        contracts: PYTHON_STDLIB_TYPE_DOMAIN_CONTRACT_IDS.len(),
        value_laws: 0,
        positive_fixtures: PYTHON_STDLIB_TYPE_DOMAIN_ALIAS_CONTRACTS.len(),
        hard_negatives: 2,
    }
}

fn python_stdlib_type_domain_conformance_refs() -> Vec<&'static str> {
    let mut refs = Vec::with_capacity(PYTHON_STDLIB_TYPE_DOMAIN_ALIAS_CONTRACTS.len() * 2);
    for row in PYTHON_STDLIB_TYPE_DOMAIN_ALIAS_CONTRACTS {
        refs.push(row.positive_fixture);
        refs.push(row.hard_negative_fixture);
    }
    refs.extend(PYTHON_STDLIB_TYPE_DOMAIN_HARD_NEGATIVE_REFS);
    refs.sort_unstable();
    refs.dedup();
    refs
}

fn value_graph_law_ids() -> Vec<&'static str> {
    pack_facing_value_laws()
        .iter()
        .map(|law| law.law_id)
        .collect()
}

fn value_graph_law_conformance_refs() -> Vec<&'static str> {
    let mut refs = pack_facing_value_laws()
        .iter()
        .flat_map(|law| law.conformance_refs.iter().copied())
        .collect::<Vec<_>>();
    refs.sort_unstable();
    refs.dedup();
    refs
}

fn value_graph_law_counts() -> SemanticPackCounts {
    let laws = pack_facing_value_laws();
    SemanticPackCounts {
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
    }
}

static BUILTIN_PACK_DESCRIPTORS: &[BuiltinPackDescriptor] = &[
    BuiltinPackDescriptor {
        id: FIRST_PARTY_PACK_ID,
        kind: SemanticPackKind::LanguagePack,
        display_name: "nose first-party semantic kernel",
        trust: PackTrust::DefaultFirstParty,
        enabled_by_default: true,
        supported_languages: NO_LANGUAGES,
        supported_packages: NO_PACKAGES,
        evidence_producer_ids: NO_IDS,
        contract_ids: NO_IDS,
        static_value_law_ids: NO_IDS,
        dynamic_value_law_ids: None,
        static_conformance_refs: NO_IDS,
        dynamic_conformance_refs: None,
        counts: empty_counts,
    },
    BuiltinPackDescriptor {
        id: PYTHON_STDLIB_TYPE_DOMAIN_PACK_ID,
        kind: SemanticPackKind::StdlibPack,
        display_name: "nose Python stdlib type-domain pack",
        trust: PackTrust::DefaultFirstParty,
        enabled_by_default: true,
        supported_languages: PYTHON_LANGUAGE,
        supported_packages: PYTHON_STDLIB_TYPE_DOMAIN_PACKAGES,
        evidence_producer_ids: PYTHON_STDLIB_TYPE_DOMAIN_PRODUCER_IDS,
        contract_ids: PYTHON_STDLIB_TYPE_DOMAIN_CONTRACT_IDS,
        static_value_law_ids: NO_IDS,
        dynamic_value_law_ids: None,
        static_conformance_refs: NO_IDS,
        dynamic_conformance_refs: Some(python_stdlib_type_domain_conformance_refs),
        counts: python_stdlib_type_domain_counts,
    },
    BuiltinPackDescriptor {
        id: FIRST_PARTY_VALUE_LAW_PACK_ID,
        kind: SemanticPackKind::LawPack,
        display_name: "nose value-graph law pack",
        trust: PackTrust::DefaultFirstParty,
        enabled_by_default: true,
        supported_languages: NO_LANGUAGES,
        supported_packages: NO_PACKAGES,
        evidence_producer_ids: NO_IDS,
        contract_ids: NO_IDS,
        static_value_law_ids: NO_IDS,
        dynamic_value_law_ids: Some(value_graph_law_ids),
        static_conformance_refs: NO_IDS,
        dynamic_conformance_refs: Some(value_graph_law_conformance_refs),
        counts: value_graph_law_counts,
    },
];

pub fn builtin_pack_descriptors() -> &'static [BuiltinPackDescriptor] {
    BUILTIN_PACK_DESCRIPTORS
}

pub fn builtin_pack_descriptor(pack_id: &str) -> Option<&'static BuiltinPackDescriptor> {
    BUILTIN_PACK_DESCRIPTORS
        .iter()
        .find(|descriptor| descriptor.id == pack_id)
}

pub fn first_party_semantic_pack() -> SemanticPackSummary {
    builtin_pack_descriptor(FIRST_PARTY_PACK_ID)
        .expect("builtin compatibility pack descriptor exists")
        .summary()
}

pub fn python_stdlib_type_domain_pack() -> SemanticPackSummary {
    builtin_pack_descriptor(PYTHON_STDLIB_TYPE_DOMAIN_PACK_ID)
        .expect("Python stdlib type-domain descriptor exists")
        .summary()
}

pub fn first_party_value_law_pack() -> SemanticPackSummary {
    builtin_pack_descriptor(FIRST_PARTY_VALUE_LAW_PACK_ID)
        .expect("value-graph law descriptor exists")
        .summary()
}

pub(super) fn compiled_first_party_packs() -> Vec<SemanticPackSummary> {
    BUILTIN_PACK_DESCRIPTORS
        .iter()
        .map(|descriptor| descriptor.summary())
        .collect()
}

pub(super) fn is_compiled_first_party_pack_id(pack_id: &str) -> bool {
    compiled_first_party_packs()
        .iter()
        .any(|pack| pack.id == pack_id)
}
