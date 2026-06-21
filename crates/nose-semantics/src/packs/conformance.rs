use super::*;

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
