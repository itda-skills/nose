//! Semantic pack manifest loading and provenance summaries.
//!
//! Loading a manifest is an explicit opt-in metadata path. External packs do not
//! emit evidence or approve exact results from this module; consumers must still
//! require occurrence evidence and kernel-owned admission checks.

use crate::{
    pack_facing_value_laws, FirstPartyTypeDomainAliasContract, PackTrust, FIRST_PARTY_PACK_ID,
    FIRST_PARTY_VALUE_LAW_PACK_ID, PYTHON_STDLIB_TYPE_DOMAIN_ALIAS_CONTRACTS,
    PYTHON_STDLIB_TYPE_DOMAIN_PACK_ID, PYTHON_STDLIB_TYPE_DOMAIN_PRODUCER_ID,
};
use nose_il::stable_symbol_hash;
use serde::Deserialize;
use std::path::PathBuf;

mod compiled;
mod loading;
mod validation;

pub use compiled::{
    builtin_pack_descriptor, builtin_pack_descriptors, first_party_semantic_pack,
    first_party_value_law_pack, python_stdlib_type_domain_pack, BuiltinPackDescriptor,
};
pub use loading::{
    check_semantic_pack_conformance, discover_manifest_paths, load_local_manifest,
    SemanticPackLoadError,
};

use compiled::compiled_first_party_packs;
use validation::validate_manifest;
pub const SEMANTIC_PACK_API_VERSION: &str = "nose.semantic-pack.v0";

const ALLOWED_REQUIREMENT_PREFIXES: &[&str] = &[
    "Source.",
    "Symbol.",
    "Import.",
    "Domain.",
    "Type.",
    "Guard.",
    "Place.",
    "Effect.",
    "LibraryApi.",
    "CallTarget.",
    "SequenceSurface.",
    "nose.",
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SemanticPackSource {
    CompiledFirstParty,
    LocalManifest,
}

impl SemanticPackSource {
    pub const fn as_str(self) -> &'static str {
        match self {
            SemanticPackSource::CompiledFirstParty => "compiled-first-party",
            SemanticPackSource::LocalManifest => "local-manifest",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SemanticPackInfluence {
    EvidenceAndContracts,
    MetadataOnly,
}

impl SemanticPackInfluence {
    pub const fn as_str(self) -> &'static str {
        match self {
            SemanticPackInfluence::EvidenceAndContracts => "evidence-and-contracts",
            SemanticPackInfluence::MetadataOnly => "metadata-only",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Deserialize)]
pub enum SemanticPackKind {
    LanguagePack,
    StdlibPack,
    LibraryPack,
    ProtocolPack,
    LawPack,
}

impl SemanticPackKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            SemanticPackKind::LanguagePack => "LanguagePack",
            SemanticPackKind::StdlibPack => "StdlibPack",
            SemanticPackKind::LibraryPack => "LibraryPack",
            SemanticPackKind::ProtocolPack => "ProtocolPack",
            SemanticPackKind::LawPack => "LawPack",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum SemanticPackAnchor {
    SourceSpan,
    Node,
    Param,
    Binding,
    Sequence,
    Module,
    Package,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SemanticPackChannel {
    SyntaxOnly,
    NearOnly,
    AbstractionWitness,
    ExactEmpirical,
    ExactProven,
}

impl SemanticPackChannel {
    pub const fn as_str(self) -> &'static str {
        match self {
            SemanticPackChannel::SyntaxOnly => "syntax-only",
            SemanticPackChannel::NearOnly => "near-only",
            SemanticPackChannel::AbstractionWitness => "abstraction-witness",
            SemanticPackChannel::ExactEmpirical => "exact-empirical",
            SemanticPackChannel::ExactProven => "exact-proven",
        }
    }

    pub const fn exact_capable(self) -> bool {
        matches!(
            self,
            SemanticPackChannel::ExactEmpirical | SemanticPackChannel::ExactProven
        )
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SemanticPackProofStatus {
    Proven,
    Covered,
    Missing,
    EmpiricalOnly,
    RejectedCounterexample,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum SemanticPackStatus {
    DraftExample,
    Experimental,
    Stable,
    Deprecated,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum SemanticPackSchemaVersion {
    V0,
}

impl PackTrust {
    pub const fn as_manifest_str(self) -> &'static str {
        match self {
            PackTrust::DefaultFirstParty => "default-first-party",
            PackTrust::FirstPartyOptional => "first-party-optional",
            PackTrust::ExternalOptIn => "external-opt-in",
        }
    }
}

impl<'de> Deserialize<'de> for PackTrust {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match String::deserialize(deserializer)?.as_str() {
            "default-first-party" => Ok(PackTrust::DefaultFirstParty),
            "first-party-optional" => Ok(PackTrust::FirstPartyOptional),
            "external-opt-in" => Ok(PackTrust::ExternalOptIn),
            other => Err(serde::de::Error::custom(format!(
                "unknown pack trust `{other}`"
            ))),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SemanticPackCounts {
    pub evidence_producers: usize,
    pub contracts: usize,
    pub value_laws: usize,
    pub positive_fixtures: usize,
    pub hard_negatives: usize,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SemanticPackSummary {
    pub id: String,
    pub hash: u64,
    pub kind: SemanticPackKind,
    pub version: String,
    pub display_name: String,
    pub trust: PackTrust,
    pub enabled_by_default: bool,
    pub source: SemanticPackSource,
    pub influence: SemanticPackInfluence,
    pub manifest_path: Option<PathBuf>,
    pub provider: String,
    pub repository: String,
    pub license: String,
    pub supported_languages: Vec<String>,
    pub counts: SemanticPackCounts,
}

impl SemanticPackSummary {
    pub fn hash_hex(&self) -> String {
        format!("{:016x}", self.hash)
    }

    fn from_manifest(path: PathBuf, manifest: SemanticPackManifest) -> Result<Self, String> {
        validate_manifest(&manifest)?;
        let id = manifest.pack.id;
        let supported_languages = manifest
            .supported_languages
            .into_iter()
            .map(|language| language.id)
            .collect();
        let counts = SemanticPackCounts {
            evidence_producers: manifest.declares.evidence_producers.len(),
            contracts: manifest.declares.contracts.len(),
            value_laws: manifest.declares.value_laws.len(),
            positive_fixtures: manifest.conformance.positive_fixtures.len(),
            hard_negatives: manifest.conformance.hard_negatives.len(),
        };
        Ok(Self {
            hash: semantic_pack_hash(&id),
            id,
            kind: manifest.pack.kind,
            version: manifest.pack.version,
            display_name: manifest.pack.display_name,
            trust: manifest.pack.trust,
            enabled_by_default: manifest.pack.enabled_by_default,
            source: SemanticPackSource::LocalManifest,
            influence: SemanticPackInfluence::MetadataOnly,
            manifest_path: Some(path),
            provider: manifest.provenance.provider.name,
            repository: manifest.provenance.repository,
            license: manifest.provenance.license,
            supported_languages,
            counts,
        })
    }
}

pub fn semantic_pack_hash(pack_id: &str) -> u64 {
    stable_symbol_hash(pack_id)
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SemanticPackSet {
    packs: Vec<SemanticPackSummary>,
}

impl SemanticPackSet {
    pub fn new_local(paths: &[PathBuf]) -> Result<Self, SemanticPackLoadError> {
        let manifest_paths = discover_manifest_paths(paths)?;
        let mut packs = compiled_first_party_packs();
        for path in manifest_paths {
            let pack = load_local_manifest(&path)?;
            if let Some(existing) = packs.iter().find(|existing| existing.id == pack.id) {
                return Err(SemanticPackLoadError::DuplicatePackId {
                    id: pack.id,
                    first_path: existing.manifest_path.clone(),
                    second_path: Some(path),
                });
            }
            packs.push(pack);
        }
        Ok(Self { packs })
    }

    pub fn first_party_only() -> Self {
        Self {
            packs: compiled_first_party_packs(),
        }
    }

    pub fn packs(&self) -> &[SemanticPackSummary] {
        &self.packs
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SemanticPackFixtureKind {
    Positive,
    HardNegative,
}

impl SemanticPackFixtureKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            SemanticPackFixtureKind::Positive => "positive",
            SemanticPackFixtureKind::HardNegative => "hard-negative",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SemanticPackFixtureIssue {
    MissingPath,
    MissingFile,
    MissingExpectation,
    AbsolutePath,
}

impl SemanticPackFixtureIssue {
    pub const fn as_str(self) -> &'static str {
        match self {
            SemanticPackFixtureIssue::MissingPath => "missing-path",
            SemanticPackFixtureIssue::MissingFile => "missing-file",
            SemanticPackFixtureIssue::MissingExpectation => "missing-expectation",
            SemanticPackFixtureIssue::AbsolutePath => "absolute-path",
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SemanticPackFixtureCheck {
    pub kind: SemanticPackFixtureKind,
    pub id: String,
    pub description: String,
    pub declared_path: Option<String>,
    pub resolved_path: Option<PathBuf>,
    pub expectation: Option<String>,
    pub issues: Vec<SemanticPackFixtureIssue>,
}

impl SemanticPackFixtureCheck {
    pub fn passed(&self) -> bool {
        self.issues.is_empty()
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SemanticPackConformanceManifest {
    pub pack: SemanticPackSummary,
    pub manifest_path: PathBuf,
    pub conformance_command: Option<String>,
    pub proof_links: Vec<String>,
    pub fixtures: Vec<SemanticPackFixtureCheck>,
}

impl SemanticPackConformanceManifest {
    pub fn passed(&self) -> bool {
        self.fixture_issue_count() == 0
    }

    pub fn fixture_issue_count(&self) -> usize {
        self.fixtures
            .iter()
            .map(|fixture| fixture.issues.len())
            .sum()
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SemanticPackConformanceReport {
    pub manifests: Vec<SemanticPackConformanceManifest>,
}

impl SemanticPackConformanceReport {
    pub fn passed(&self) -> bool {
        self.manifests
            .iter()
            .all(SemanticPackConformanceManifest::passed)
    }

    pub fn manifest_count(&self) -> usize {
        self.manifests.len()
    }

    pub fn positive_fixture_count(&self) -> usize {
        self.manifests
            .iter()
            .map(|manifest| manifest.pack.counts.positive_fixtures)
            .sum()
    }

    pub fn hard_negative_count(&self) -> usize {
        self.manifests
            .iter()
            .map(|manifest| manifest.pack.counts.hard_negatives)
            .sum()
    }

    pub fn fixture_issue_count(&self) -> usize {
        self.manifests
            .iter()
            .map(SemanticPackConformanceManifest::fixture_issue_count)
            .sum()
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SemanticPackManifest {
    #[serde(rename = "$schema")]
    _schema: Option<String>,
    api_version: String,
    pack: ManifestPack,
    provenance: ManifestProvenance,
    compatibility: ManifestCompatibility,
    supported_languages: Vec<ManifestLanguage>,
    #[serde(default)]
    packages: Vec<ManifestPackage>,
    #[serde(default)]
    dependencies: Vec<ManifestDependency>,
    declares: ManifestDeclares,
    conformance: ManifestConformance,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ManifestPack {
    id: String,
    kind: SemanticPackKind,
    version: String,
    display_name: String,
    #[serde(default)]
    description: Option<String>,
    trust: PackTrust,
    enabled_by_default: bool,
    // Documented v0 schema field the engine does not consume; listed (typed) so
    // `deny_unknown_fields` still accepts and validates conforming manifests.
    #[serde(default, rename = "status")]
    _status: Option<SemanticPackStatus>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ManifestProvenance {
    provider: ManifestProvider,
    license: String,
    repository: String,
    #[serde(default)]
    source_revision: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ManifestProvider {
    name: String,
    #[serde(default)]
    contact: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ManifestCompatibility {
    nose: String,
    #[serde(default, rename = "schema")]
    _schema: Option<SemanticPackSchemaVersion>,
    #[serde(default)]
    notes: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ManifestLanguage {
    id: String,
    #[serde(default)]
    language_version: Option<String>,
    #[serde(default)]
    runtime: Option<String>,
    #[serde(default)]
    runtime_versions: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ManifestDependency {
    id: String,
    version: String,
    required: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ManifestPackage {
    ecosystem: String,
    name: String,
    versions: String,
    #[serde(default, rename = "stdlib")]
    _stdlib: Option<bool>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ManifestDeclares {
    evidence_producers: Vec<ManifestEvidenceProducer>,
    contracts: Vec<ManifestContract>,
    #[serde(default)]
    value_laws: Vec<ManifestValueLaw>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ManifestEvidenceProducer {
    id: String,
    kind: String,
    anchors: Vec<SemanticPackAnchor>,
    channel: SemanticPackChannel,
    #[serde(default)]
    emits: Vec<String>,
    #[serde(default)]
    requires: Vec<ManifestRequirement>,
    stable_hash_inputs: Vec<String>,
    conflict_policy: String,
    #[serde(default)]
    notes: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ManifestContract {
    id: String,
    surface: serde_json::Value,
    requires: Vec<ManifestRequirement>,
    semantics: serde_json::Value,
    channel: SemanticPackChannel,
    proof_status: SemanticPackProofStatus,
    conformance_refs: Vec<String>,
    #[serde(default)]
    known_unsupported: Vec<String>,
    #[serde(default)]
    notes: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ManifestValueLaw {
    id: String,
    requires: Vec<ManifestRequirement>,
    semantics: serde_json::Value,
    channel: SemanticPackChannel,
    proof_status: SemanticPackProofStatus,
    conformance_refs: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ManifestRequirement {
    #[serde(rename = "ref")]
    ref_id: String,
    subject: String,
    required: bool,
    #[serde(default)]
    same_anchor_as: Option<String>,
    #[serde(default)]
    within_scope: Option<String>,
    #[serde(default)]
    before: Option<String>,
    #[serde(default)]
    after: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ManifestConformance {
    positive_fixtures: Vec<ManifestFixture>,
    hard_negatives: Vec<ManifestFixture>,
    known_unsupported: Vec<String>,
    #[serde(default)]
    command: Option<String>,
    #[serde(default)]
    proofs: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ManifestFixture {
    id: String,
    description: String,
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    expectation: Option<String>,
}

#[cfg(test)]
mod tests;
