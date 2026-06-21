use super::*;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SemanticPackRequirementSummary {
    pub ref_id: String,
    pub subject: String,
    pub required: bool,
    pub same_anchor_as: Option<String>,
    pub within_scope: Option<String>,
    pub before: Option<String>,
    pub after: Option<String>,
}

impl SemanticPackRequirementSummary {
    pub(super) fn from_manifest(requirement: &ManifestRequirement) -> Self {
        Self {
            ref_id: requirement.ref_id.clone(),
            subject: requirement.subject.clone(),
            required: requirement.required,
            same_anchor_as: requirement.same_anchor_as.clone(),
            within_scope: requirement.within_scope.clone(),
            before: requirement.before.clone(),
            after: requirement.after.clone(),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ExternalEvidenceProducerRow {
    pub pack_id: String,
    pub pack_hash: u64,
    pub manifest_path: PathBuf,
    pub producer_id: String,
    pub producer_hash: u64,
    pub kind: String,
    pub anchors: Vec<SemanticPackAnchor>,
    pub channel: SemanticPackChannel,
    pub emits: Vec<String>,
    pub requirements: Vec<SemanticPackRequirementSummary>,
    pub stable_hash_inputs: Vec<String>,
    pub conflict_policy: String,
    pub notes: Option<String>,
}

impl ExternalEvidenceProducerRow {
    pub(super) fn from_manifest(
        manifest_path: &std::path::Path,
        manifest: &SemanticPackManifest,
        producer: &ManifestEvidenceProducer,
    ) -> Self {
        Self {
            pack_id: manifest.pack.id.clone(),
            pack_hash: semantic_pack_hash(&manifest.pack.id),
            manifest_path: manifest_path.to_path_buf(),
            producer_id: producer.id.clone(),
            producer_hash: stable_symbol_hash(&producer.id),
            kind: producer.kind.clone(),
            anchors: producer.anchors.clone(),
            channel: producer.channel,
            emits: producer.emits.clone(),
            requirements: producer
                .requires
                .iter()
                .map(SemanticPackRequirementSummary::from_manifest)
                .collect(),
            stable_hash_inputs: producer.stable_hash_inputs.clone(),
            conflict_policy: producer.conflict_policy.clone(),
            notes: producer.notes.clone(),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ExternalContractRow {
    pub pack_id: String,
    pub pack_hash: u64,
    pub manifest_path: PathBuf,
    pub contract_id: String,
    pub contract_hash: u64,
    pub surface: serde_json::Value,
    pub requirements: Vec<SemanticPackRequirementSummary>,
    pub semantics: serde_json::Value,
    pub channel: SemanticPackChannel,
    pub proof_status: SemanticPackProofStatus,
    pub conformance_refs: Vec<String>,
    pub known_unsupported: Vec<String>,
    pub notes: Option<String>,
}

impl ExternalContractRow {
    pub(super) fn from_manifest(
        manifest_path: &std::path::Path,
        manifest: &SemanticPackManifest,
        contract: &ManifestContract,
    ) -> Self {
        Self {
            pack_id: manifest.pack.id.clone(),
            pack_hash: semantic_pack_hash(&manifest.pack.id),
            manifest_path: manifest_path.to_path_buf(),
            contract_id: contract.id.clone(),
            contract_hash: stable_symbol_hash(&contract.id),
            surface: contract.surface.clone(),
            requirements: contract
                .requires
                .iter()
                .map(SemanticPackRequirementSummary::from_manifest)
                .collect(),
            semantics: contract.semantics.clone(),
            channel: contract.channel,
            proof_status: contract.proof_status,
            conformance_refs: contract.conformance_refs.clone(),
            known_unsupported: contract.known_unsupported.clone(),
            notes: contract.notes.clone(),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ExternalValueLawRow {
    pub pack_id: String,
    pub pack_hash: u64,
    pub manifest_path: PathBuf,
    pub law_id: String,
    pub law_hash: u64,
    pub channel: SemanticPackChannel,
    pub proof_status: SemanticPackProofStatus,
    pub requirements: Vec<SemanticPackRequirementSummary>,
    pub conformance_refs: Vec<String>,
    pub semantics: serde_json::Value,
}

impl ExternalValueLawRow {
    pub(super) fn from_manifest(
        manifest_path: &std::path::Path,
        manifest: &SemanticPackManifest,
        law: &ManifestValueLaw,
    ) -> Self {
        Self {
            pack_id: manifest.pack.id.clone(),
            pack_hash: semantic_pack_hash(&manifest.pack.id),
            manifest_path: manifest_path.to_path_buf(),
            law_id: law.id.clone(),
            law_hash: stable_symbol_hash(&law.id),
            channel: law.channel,
            proof_status: law.proof_status,
            requirements: law
                .requires
                .iter()
                .map(SemanticPackRequirementSummary::from_manifest)
                .collect(),
            conformance_refs: law.conformance_refs.clone(),
            semantics: law.semantics.clone(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ExternalRowKind {
    EvidenceProducer,
    Contract,
    ValueLaw,
}

impl ExternalRowKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            ExternalRowKind::EvidenceProducer => "evidence-producer",
            ExternalRowKind::Contract => "contract",
            ExternalRowKind::ValueLaw => "value-law",
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ExternalRowConflict {
    pub kind: ExternalRowKind,
    pub row_id: String,
    pub row_hash: u64,
    pub external_pack_id: String,
    pub external_pack_hash: u64,
    pub external_manifest_path: PathBuf,
    pub conflicting_pack_id: String,
    pub conflicting_pack_hash: u64,
    pub conflicting_source: SemanticPackSource,
    pub conflicting_manifest_path: Option<PathBuf>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ExternalRowConflictReport {
    pub conflicts: Vec<ExternalRowConflict>,
}

impl ExternalRowConflictReport {
    pub fn passed(&self) -> bool {
        self.conflicts.is_empty()
    }

    pub fn conflict_count(&self) -> usize {
        self.conflicts.len()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ExternalInfluenceBlocker {
    DataOnlyRegistration,
    DependencyBackedEvidenceUnavailable,
    ExplicitInfluenceTrustGateMissing,
    ExecutableConformanceUnavailable,
    RowConflict,
}

impl ExternalInfluenceBlocker {
    pub const fn as_str(self) -> &'static str {
        match self {
            ExternalInfluenceBlocker::DataOnlyRegistration => "data-only-registration",
            ExternalInfluenceBlocker::DependencyBackedEvidenceUnavailable => {
                "dependency-backed-evidence-unavailable"
            }
            ExternalInfluenceBlocker::ExplicitInfluenceTrustGateMissing => {
                "explicit-influence-trust-gate-missing"
            }
            ExternalInfluenceBlocker::ExecutableConformanceUnavailable => {
                "executable-conformance-unavailable"
            }
            ExternalInfluenceBlocker::RowConflict => "row-conflict",
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ExternalRowInfluencePreflight {
    pub kind: ExternalRowKind,
    pub row_id: String,
    pub row_hash: u64,
    pub pack_id: String,
    pub pack_hash: u64,
    pub manifest_path: PathBuf,
    pub channel: SemanticPackChannel,
    pub blockers: Vec<ExternalInfluenceBlocker>,
}

impl ExternalRowInfluencePreflight {
    pub fn passed(&self) -> bool {
        self.blockers.is_empty()
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ExternalInfluencePreflightReport {
    pub rows: Vec<ExternalRowInfluencePreflight>,
}

impl ExternalInfluencePreflightReport {
    pub fn passed(&self) -> bool {
        self.rows.iter().all(ExternalRowInfluencePreflight::passed)
    }

    pub fn blocked_count(&self) -> usize {
        self.rows.iter().filter(|row| !row.passed()).count()
    }
}

#[derive(Clone)]
pub(super) struct ExternalRowCoordinate {
    pub(super) kind: ExternalRowKind,
    pub(super) row_id: String,
    pub(super) row_hash: u64,
    pub(super) pack_id: String,
    pub(super) pack_hash: u64,
    pub(super) manifest_path: PathBuf,
    pub(super) channel: SemanticPackChannel,
}

impl ExternalRowCoordinate {
    pub(super) fn from_producer(row: &ExternalEvidenceProducerRow) -> Self {
        Self {
            kind: ExternalRowKind::EvidenceProducer,
            row_id: row.producer_id.clone(),
            row_hash: row.producer_hash,
            pack_id: row.pack_id.clone(),
            pack_hash: row.pack_hash,
            manifest_path: row.manifest_path.clone(),
            channel: row.channel,
        }
    }

    pub(super) fn from_contract(row: &ExternalContractRow) -> Self {
        Self {
            kind: ExternalRowKind::Contract,
            row_id: row.contract_id.clone(),
            row_hash: row.contract_hash,
            pack_id: row.pack_id.clone(),
            pack_hash: row.pack_hash,
            manifest_path: row.manifest_path.clone(),
            channel: row.channel,
        }
    }

    pub(super) fn from_law(row: &ExternalValueLawRow) -> Self {
        Self {
            kind: ExternalRowKind::ValueLaw,
            row_id: row.law_id.clone(),
            row_hash: row.law_hash,
            pack_id: row.pack_id.clone(),
            pack_hash: row.pack_hash,
            manifest_path: row.manifest_path.clone(),
            channel: row.channel,
        }
    }
}
