use super::*;
use std::collections::HashSet;
use std::fmt;
use std::path::{Path, PathBuf};

use super::compiled::is_compiled_builtin_pack_id;
pub fn check_semantic_pack_conformance(
    paths: &[PathBuf],
) -> Result<SemanticPackConformanceReport, SemanticPackLoadError> {
    let manifest_paths = discover_manifest_paths(paths)?;
    let mut manifests: Vec<SemanticPackConformanceManifest> = Vec::new();
    for path in manifest_paths {
        let manifest = read_local_manifest(&path)?;
        let conformance_command = manifest.conformance.command.clone();
        let proof_links = manifest.conformance.proofs.clone();
        let fixtures = collect_fixture_checks(&path, &manifest);
        let pack =
            SemanticPackSummary::from_manifest(path.clone(), manifest).map_err(|message| {
                SemanticPackLoadError::InvalidManifest {
                    path: path.clone(),
                    message,
                }
            })?;
        if is_compiled_builtin_pack_id(&pack.id) {
            return Err(SemanticPackLoadError::DuplicatePackId {
                id: pack.id,
                first_path: None,
                second_path: Some(path),
            });
        }
        if let Some(existing) = manifests
            .iter()
            .find(|existing| existing.pack.id == pack.id)
        {
            return Err(SemanticPackLoadError::DuplicatePackId {
                id: pack.id,
                first_path: Some(existing.manifest_path.clone()),
                second_path: Some(path),
            });
        }
        manifests.push(SemanticPackConformanceManifest {
            pack,
            manifest_path: path,
            conformance_command,
            proof_links,
            fixtures,
        });
    }
    Ok(SemanticPackConformanceReport { manifests })
}

pub fn discover_manifest_paths(paths: &[PathBuf]) -> Result<Vec<PathBuf>, SemanticPackLoadError> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for path in paths {
        if path.is_file() {
            push_unique_manifest(path, &mut seen, &mut out)?;
        } else if path.is_dir() {
            discover_manifest_directory(path, &mut seen, &mut out)?;
        } else {
            return Err(SemanticPackLoadError::NotFound { path: path.clone() });
        }
    }
    Ok(out)
}

fn discover_manifest_directory(
    path: &Path,
    seen: &mut HashSet<PathBuf>,
    out: &mut Vec<PathBuf>,
) -> Result<(), SemanticPackLoadError> {
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(path).map_err(|source| SemanticPackLoadError::Io {
        path: path.to_path_buf(),
        source,
    })? {
        let entry = entry.map_err(|source| SemanticPackLoadError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        let entry_path = entry.path();
        if entry_path.extension().is_some_and(|ext| ext == "json") {
            entries.push(entry_path);
        }
    }
    entries.sort();
    if entries.is_empty() {
        return Err(SemanticPackLoadError::DirectoryHasNoManifests {
            path: path.to_path_buf(),
        });
    }
    for entry in entries {
        push_unique_manifest(&entry, seen, out)?;
    }
    Ok(())
}

fn push_unique_manifest(
    path: &Path,
    seen: &mut HashSet<PathBuf>,
    out: &mut Vec<PathBuf>,
) -> Result<(), SemanticPackLoadError> {
    let canonical = path
        .canonicalize()
        .map_err(|source| SemanticPackLoadError::Io {
            path: path.to_path_buf(),
            source,
        })?;
    if seen.insert(canonical.clone()) {
        out.push(canonical);
    }
    Ok(())
}

pub fn load_local_manifest(path: &Path) -> Result<SemanticPackSummary, SemanticPackLoadError> {
    let manifest = read_local_manifest(path)?;
    SemanticPackSummary::from_manifest(path.to_path_buf(), manifest).map_err(|message| {
        SemanticPackLoadError::InvalidManifest {
            path: path.to_path_buf(),
            message,
        }
    })
}

fn read_local_manifest(path: &Path) -> Result<SemanticPackManifest, SemanticPackLoadError> {
    let text = std::fs::read_to_string(path).map_err(|source| SemanticPackLoadError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    serde_json::from_str::<SemanticPackManifest>(&text).map_err(|source| {
        SemanticPackLoadError::Json {
            path: path.to_path_buf(),
            source,
        }
    })
}

fn collect_fixture_checks(
    manifest_path: &Path,
    manifest: &SemanticPackManifest,
) -> Vec<SemanticPackFixtureCheck> {
    let mut checks = Vec::new();
    for fixture in &manifest.conformance.positive_fixtures {
        checks.push(fixture_check(
            manifest_path,
            SemanticPackFixtureKind::Positive,
            fixture,
        ));
    }
    for fixture in &manifest.conformance.hard_negatives {
        checks.push(fixture_check(
            manifest_path,
            SemanticPackFixtureKind::HardNegative,
            fixture,
        ));
    }
    checks
}

fn fixture_check(
    manifest_path: &Path,
    kind: SemanticPackFixtureKind,
    fixture: &ManifestFixture,
) -> SemanticPackFixtureCheck {
    let resolved_path = fixture
        .path
        .as_deref()
        .map(|path| resolve_fixture_path(manifest_path, path));
    let mut issues = Vec::new();
    if fixture.path.is_none() {
        issues.push(SemanticPackFixtureIssue::MissingPath);
    } else if let Some(declared_path) = fixture.path.as_deref() {
        if Path::new(declared_path).is_absolute() {
            issues.push(SemanticPackFixtureIssue::AbsolutePath);
        }
        if !resolved_path.as_ref().is_some_and(|path| path.is_file()) {
            issues.push(SemanticPackFixtureIssue::MissingFile);
        }
    }
    if fixture.expectation.is_none() {
        issues.push(SemanticPackFixtureIssue::MissingExpectation);
    }
    SemanticPackFixtureCheck {
        kind,
        id: fixture.id.clone(),
        description: fixture.description.clone(),
        declared_path: fixture.path.clone(),
        resolved_path,
        expectation: fixture.expectation.clone(),
        issues,
    }
}

fn resolve_fixture_path(manifest_path: &Path, declared_path: &str) -> PathBuf {
    let path = Path::new(declared_path);
    if path.is_absolute() {
        return path.to_path_buf();
    }
    manifest_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(path)
}

#[derive(Debug)]
pub enum SemanticPackLoadError {
    NotFound {
        path: PathBuf,
    },
    DirectoryHasNoManifests {
        path: PathBuf,
    },
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    Json {
        path: PathBuf,
        source: serde_json::Error,
    },
    InvalidManifest {
        path: PathBuf,
        message: String,
    },
    DuplicatePackId {
        id: String,
        first_path: Option<PathBuf>,
        second_path: Option<PathBuf>,
    },
}

impl fmt::Display for SemanticPackLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SemanticPackLoadError::NotFound { path } => {
                write!(f, "semantic pack path not found: {}", path.display())
            }
            SemanticPackLoadError::DirectoryHasNoManifests { path } => write!(
                f,
                "semantic pack directory contains no JSON manifests: {}",
                path.display()
            ),
            SemanticPackLoadError::Io { path, source } => {
                write!(f, "reading semantic pack {}: {source}", path.display())
            }
            SemanticPackLoadError::Json { path, source } => {
                write!(f, "parsing semantic pack {}: {source}", path.display())
            }
            SemanticPackLoadError::InvalidManifest { path, message } => {
                write!(f, "invalid semantic pack {}: {message}", path.display())
            }
            SemanticPackLoadError::DuplicatePackId {
                id,
                first_path,
                second_path,
            } => write!(
                f,
                "duplicate semantic pack id `{id}` between {} and {}",
                display_optional_path(first_path),
                display_optional_path(second_path)
            ),
        }
    }
}

impl std::error::Error for SemanticPackLoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SemanticPackLoadError::Io { source, .. } => Some(source),
            SemanticPackLoadError::Json { source, .. } => Some(source),
            SemanticPackLoadError::NotFound { .. }
            | SemanticPackLoadError::DirectoryHasNoManifests { .. }
            | SemanticPackLoadError::InvalidManifest { .. }
            | SemanticPackLoadError::DuplicatePackId { .. } => None,
        }
    }
}

fn display_optional_path(path: &Option<PathBuf>) -> String {
    path.as_ref()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<compiled builtin>".to_string())
}
