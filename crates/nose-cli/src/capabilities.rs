use anyhow::Result;

const CAPABILITIES_SCHEMA_VERSION: u32 = 3;

#[derive(serde::Serialize)]
struct Report {
    schema_version: u32,
    tool: Tool,
    platform: Platform,
    interfaces: Interfaces,
    commands: Commands,
    schemas: Schemas,
    query: QuerySurface,
    semantic_packs: SemanticPacks,
    il: Il,
    stats: Stats,
}

#[derive(serde::Serialize)]
struct Tool {
    name: &'static str,
    version: &'static str,
}

#[derive(serde::Serialize)]
struct Platform {
    os: &'static str,
    arch: &'static str,
    family: &'static str,
}

#[derive(serde::Serialize)]
struct Interfaces {
    capabilities_json: bool,
    version_json: bool,
    doctor_json: bool,
}

#[derive(serde::Serialize)]
struct Commands {
    stable: Vec<&'static str>,
    deprecated: Vec<&'static str>,
}

#[derive(serde::Serialize)]
struct Schemas {
    capabilities: Vec<u32>,
    query_json: Vec<u32>,
    semantic_packs: Vec<&'static str>,
    semantic_pack_conformance: Vec<u32>,
}

#[derive(serde::Serialize)]
struct QuerySurface {
    modes: Vec<&'static str>,
    default_modes: Vec<&'static str>,
    output_formats: Vec<&'static str>,
    sort_keys: Vec<&'static str>,
    config_keys: Vec<&'static str>,
    capabilities: std::collections::BTreeMap<&'static str, bool>,
}

#[derive(serde::Serialize)]
struct SemanticPacks {
    api_versions: Vec<&'static str>,
    loading: Vec<&'static str>,
    conformance: Vec<&'static str>,
    conformance_output_formats: Vec<&'static str>,
    trust: Vec<&'static str>,
    external_packs_enabled_by_default: bool,
    external_pack_influence: &'static str,
}

#[derive(serde::Serialize)]
struct Il {
    output_formats: Vec<&'static str>,
    normalized: bool,
    cfg_norm_toggle: bool,
}

#[derive(serde::Serialize)]
struct Stats {
    output_formats: Vec<&'static str>,
}

impl Report {
    fn current() -> Self {
        Report {
            schema_version: CAPABILITIES_SCHEMA_VERSION,
            tool: Tool {
                name: "nose",
                version: env!("CARGO_PKG_VERSION"),
            },
            platform: Platform {
                os: std::env::consts::OS,
                arch: std::env::consts::ARCH,
                family: std::env::consts::FAMILY,
            },
            interfaces: Interfaces {
                capabilities_json: true,
                version_json: false,
                doctor_json: false,
            },
            commands: Commands {
                stable: vec!["capabilities", "il", "query", "semantic-pack", "stats"],
                deprecated: Vec::new(),
            },
            schemas: Schemas {
                capabilities: vec![CAPABILITIES_SCHEMA_VERSION],
                query_json: vec![crate::schema_versions::QUERY_JSON_SCHEMA_VERSION],
                semantic_packs: vec![nose_semantics::SEMANTIC_PACK_API_VERSION],
                semantic_pack_conformance: vec![crate::semantic_pack::CONFORMANCE_SCHEMA_VERSION],
            },
            query: QuerySurface {
                modes: vec!["syntax", "semantic", "near"],
                default_modes: vec!["syntax", "semantic", "near"],
                output_formats: vec!["human", "json", "markdown", "sarif"],
                sort_keys: vec!["extractability", "value", "sites", "hazard"],
                config_keys: vec![
                    "exclude",
                    "ignore-file",
                    "min-lines",
                    "min-members",
                    "min-size",
                    "min-value",
                    "mode",
                    "semantic-packs",
                    "sort",
                ],
                capabilities: query_capability_flags(),
            },
            semantic_packs: SemanticPacks {
                api_versions: vec![nose_semantics::SEMANTIC_PACK_API_VERSION],
                loading: vec![
                    "compiled-builtin",
                    "local-manifest-file",
                    "local-manifest-directory",
                ],
                conformance: vec!["local-manifest-file", "local-manifest-directory"],
                conformance_output_formats: vec!["human", "json"],
                trust: vec!["builtin-default", "builtin-optional", "external-opt-in"],
                external_packs_enabled_by_default: false,
                external_pack_influence: "metadata-only",
            },
            il: Il {
                output_formats: vec!["sexpr", "json"],
                normalized: true,
                cfg_norm_toggle: true,
            },
            stats: Stats {
                output_formats: vec!["human", "json"],
            },
        }
    }
}

fn query_capability_flags() -> std::collections::BTreeMap<&'static str, bool> {
    [
        ("base_divergence", true),
        ("baseline", true),
        ("baseline_changed_detection", true),
        ("baseline_member_digest", true),
        ("cache", true),
        ("ci_fail_gate", true),
        ("family_drilldown", true),
        ("inline_suppression", true),
        ("multi_root", true),
        ("reinvented_view", true),
        ("semantic_pack_loading", true),
        ("structured_ignores", true),
    ]
    .into_iter()
    .collect()
}

pub(crate) fn print() -> Result<()> {
    let report = Report::current();
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
