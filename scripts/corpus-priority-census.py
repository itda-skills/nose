#!/usr/bin/env python3
"""Build a corpus-wide priority census from the pinned benchmark repositories.

The report intentionally separates two signals:

- `recall_loss`: oracle/strict-admission data from `nose verify --recall-loss-report`;
- `source_stdlib_occurrences`: lexical source prevalence for stdlib/API surfaces.

The second signal is a prevalence heuristic, not semantic proof. Use it to decide
what to price next, then confirm with focused fixtures and recall-loss deltas.
"""

from __future__ import annotations

import argparse
import concurrent.futures
import json
import os
import re
import subprocess
import sys
import tempfile
from collections import Counter, defaultdict
from dataclasses import dataclass
from pathlib import Path
from typing import Any


DEFAULT_MANIFEST = "bench/goldens/corpus.json"
DEFAULT_REPOS_ROOT = "bench/repos"
DEFAULT_LOGS_DIR = "target/corpus-priority-census"
DEFAULT_OUTPUT = "target/corpus-priority-census.v1.json"

SKIP_DIRS = {
    ".git",
    ".gradle",
    ".mypy_cache",
    ".pytest_cache",
    ".venv",
    "__pycache__",
    "build",
    "dist",
    "node_modules",
    "Pods",
    "target",
    "vendor",
    "venv",
}

LANG_BY_EXT = {
    ".c": "c",
    ".h": "c",
    ".go": "go",
    ".java": "java",
    ".py": "python",
    ".rb": "ruby",
    ".rs": "rust",
    ".swift": "swift",
    ".ts": "typescript",
    ".tsx": "typescript",
    ".js": "javascript",
    ".jsx": "javascript",
}


@dataclass(frozen=True)
class SourcePattern:
    surface: str
    language: str
    capability: str
    status: str
    regex: str

    def compile(self) -> re.Pattern[str]:
        return re.compile(self.regex)


SOURCE_PATTERNS = [
    SourcePattern(
        "c.stdlib.string_memory",
        "c",
        "string-memory-operations",
        "candidate",
        r"\b(?:str(?:len|cmp|ncmp|cpy|ncpy|cat|ncat|chr|rchr|str|tok)|mem(?:cpy|move|set|cmp|chr))\s*\(",
    ),
    SourcePattern(
        "c.stdlib.allocation",
        "c",
        "allocation-lifetime",
        "candidate",
        r"\b(?:malloc|calloc|realloc|free)\s*\(",
    ),
    SourcePattern(
        "c.stdlib.sort_search",
        "c",
        "collection-ordering",
        "candidate",
        r"\b(?:qsort|bsearch)\s*\(",
    ),
    SourcePattern(
        "c.stdlib.character",
        "c",
        "character-classification",
        "candidate",
        r"\b(?:isalpha|isdigit|isalnum|isspace|tolower|toupper)\s*\(",
    ),
    SourcePattern(
        "go.stdlib.strings_affix_contains",
        "go",
        "string-affix-containment",
        "covered-partial",
        r"\bstrings\.(?:Contains|HasPrefix|HasSuffix)\s*\(",
    ),
    SourcePattern(
        "go.stdlib.strings_transform",
        "go",
        "string-transform",
        "candidate",
        r"\bstrings\.(?:Trim|TrimSpace|Split|Join|ToLower|ToUpper|Replace|Index|EqualFold)\s*\(",
    ),
    SourcePattern(
        "go.stdlib.slices",
        "go",
        "collection-membership-ordering",
        "candidate",
        r"\bslices\.(?:Contains|Index|Sort|Equal|Clone|Delete|Insert|Compact)\s*\(",
    ),
    SourcePattern(
        "go.stdlib.maps",
        "go",
        "map-view-copy-equality",
        "candidate",
        r"\bmaps\.(?:Clone|Copy|Equal|Keys|Values)\s*\(",
    ),
    SourcePattern(
        "go.stdlib.sort",
        "go",
        "collection-ordering",
        "candidate",
        r"\bsort\.(?:Slice|Strings|Ints|Search)\s*\(",
    ),
    SourcePattern(
        "go.stdlib.math",
        "go",
        "numeric-scalar-methods",
        "candidate",
        r"\bmath\.(?:Abs|Min|Max|Floor|Ceil|Round|Sqrt|Pow)\s*\(",
    ),
    SourcePattern(
        "java.stdlib.math_core",
        "java",
        "numeric-scalar-methods",
        "covered-partial",
        r"\bMath\.(?:abs|min|max)\s*\(",
    ),
    SourcePattern(
        "java.stdlib.math_other",
        "java",
        "numeric-scalar-methods",
        "candidate",
        r"\bMath\.(?:floor|ceil|round|pow|sqrt)\s*\(",
    ),
    SourcePattern(
        "java.stdlib.collection_factories",
        "java",
        "collection-factory",
        "covered-partial",
        r"\b(?:List|Set|Map)\.of(?:Entries)?\s*\(",
    ),
    SourcePattern(
        "java.stdlib.map_entry",
        "java",
        "map-entry",
        "covered",
        r"\bMap\.entry\s*\(",
    ),
    SourcePattern(
        "java.stdlib.arrays",
        "java",
        "array-collection-adapters",
        "covered-partial",
        r"\bArrays\.(?:asList|stream|copyOf|sort|equals)\s*\(",
    ),
    SourcePattern(
        "java.stdlib.collections",
        "java",
        "collection-factory-ordering",
        "covered-partial",
        r"\bCollections\.(?:emptyList|emptySet|emptyMap|singleton(?:List|Map)?|unmodifiable(?:List|Set|Map)|sort|reverse)\s*\(",
    ),
    SourcePattern(
        "java.stdlib.optional",
        "java",
        "option-result-channel",
        "candidate",
        r"\bOptional\.(?:of|ofNullable|empty)\s*\(|\.(?:orElse|orElseGet|isPresent)\s*\(",
    ),
    SourcePattern(
        "python.builtins.collection_factories",
        "python",
        "collection-factory",
        "covered-partial",
        r"\b(?:list|set|dict|tuple)\s*\(",
    ),
    SourcePattern(
        "python.builtins.iteration_hof",
        "python",
        "iterator-hof-materialization",
        "covered-partial",
        r"\b(?:sum|min|max|any|all|len|zip|enumerate|range|map|filter)\s*\(",
    ),
    SourcePattern(
        "python.stdlib.collections",
        "python",
        "collection-factory-domain",
        "covered-partial",
        r"\bcollections\.(?:deque|defaultdict|Counter|OrderedDict)\s*\(",
    ),
    SourcePattern(
        "python.stdlib.itertools",
        "python",
        "iterator-hof-materialization",
        "candidate",
        r"\bitertools\.(?:chain|zip_longest|product|combinations|groupby|islice)\s*\(",
    ),
    SourcePattern(
        "python.stdlib.functools",
        "python",
        "hof-callback-contracts",
        "candidate",
        r"\bfunctools\.(?:reduce|lru_cache|partial|cached_property)\s*\(",
    ),
    SourcePattern(
        "python.stdlib.math",
        "python",
        "numeric-scalar-methods",
        "covered-partial",
        r"\bmath\.(?:fabs|floor|ceil|sqrt|isclose|prod|fsum)\s*\(",
    ),
    SourcePattern(
        "ruby.stdlib.set",
        "ruby",
        "collection-factory",
        "covered",
        r"\bSet\.new\s*\(",
    ),
    SourcePattern(
        "ruby.enumerable_hof",
        "ruby",
        "iterator-hof-materialization",
        "covered-partial",
        r"\.(?:map|select|filter|reject|reduce|inject|any\?|all\?|flat_map)\b",
    ),
    SourcePattern(
        "ruby.string_affix_contains",
        "ruby",
        "string-affix-containment",
        "covered-partial",
        r"\.(?:start_with\?|end_with\?|include\?)\s*\(",
    ),
    SourcePattern(
        "ruby.stdlib.json",
        "ruby",
        "serialization-boundary",
        "candidate",
        r"\bJSON\.(?:parse|generate|dump|load)\s*\(",
    ),
    SourcePattern(
        "rust.stdlib.vec",
        "rust",
        "collection-factory",
        "covered-partial",
        r"\bVec::(?:new|with_capacity|from)\s*\(|\bvec!\s*\[",
    ),
    SourcePattern(
        "rust.stdlib.option",
        "rust",
        "option-result-channel",
        "covered-partial",
        r"\b(?:Some|None)\b|\.(?:unwrap_or|unwrap_or_else|and_then|map_or)\s*\(",
    ),
    SourcePattern(
        "rust.stdlib.result",
        "rust",
        "option-result-channel",
        "covered-partial",
        r"\b(?:Ok|Err)\s*\(|\.(?:is_ok|is_err)\s*\(",
    ),
    SourcePattern(
        "rust.stdlib.iterators",
        "rust",
        "iterator-hof-materialization",
        "covered-partial",
        r"\.(?:iter|into_iter|iter_mut|map|filter|filter_map|flat_map|collect|any|all|fold|reduce)\s*\(",
    ),
    SourcePattern(
        "rust.stdlib.maps",
        "rust",
        "map-factory",
        "covered-partial",
        r"\b(?:HashMap|BTreeMap)::(?:new|from|with_capacity)\s*\(",
    ),
    SourcePattern(
        "rust.stdlib.sets",
        "rust",
        "collection-factory",
        "candidate",
        r"\b(?:HashSet|BTreeSet)::(?:new|from|with_capacity)\s*\(",
    ),
    SourcePattern(
        "swift.stdlib.collection_factories",
        "swift",
        "collection-factory",
        "covered-partial",
        r"\b(?:Array|Set|Dictionary)\s*\(",
    ),
    SourcePattern(
        "swift.stdlib.sequence_hof",
        "swift",
        "iterator-hof-materialization",
        "covered-partial",
        r"\.(?:map|filter|reduce|flatMap|compactMap|contains|allSatisfy)\s*\(",
    ),
    SourcePattern(
        "js_ts.builtins.array_hof",
        "javascript",
        "iterator-hof-materialization",
        "covered-partial",
        r"\.(?:map|filter|reduce|flatMap|some|every|find|includes)\s*\(",
    ),
    SourcePattern(
        "js_ts.builtins.collection_constructors",
        "javascript",
        "collection-factory",
        "covered-partial",
        r"\bnew\s+(?:Map|Set)\s*\(",
    ),
    SourcePattern(
        "js_ts.builtins.object_key_views",
        "javascript",
        "map-key-view",
        "covered-partial",
        r"\bObject\.(?:keys|values|entries)\s*\(",
    ),
    SourcePattern(
        "js_ts.builtins.promise",
        "javascript",
        "promise-protocol",
        "covered-partial",
        r"\bPromise\.(?:resolve|all|race|reject)\s*\(|\.then\s*\(",
    ),
    SourcePattern(
        "js_ts.builtins.regex",
        "javascript",
        "regex-protocol",
        "covered-partial",
        r"\.(?:test|exec)\s*\(",
    ),
    SourcePattern(
        "js_ts.builtins.boolean",
        "javascript",
        "primitive-coercion",
        "covered",
        r"\bBoolean\s*\(",
    ),
]


def default_jobs() -> int:
    return max(1, min(6, os.cpu_count() or 2))


def load_manifest(path: Path) -> list[dict[str, Any]]:
    data = json.loads(path.read_text(encoding="utf-8"))
    repos = data.get("repositories")
    if not isinstance(repos, list):
        raise SystemExit(f"{path}: expected repositories list")
    return repos


def select_repos(repos: list[dict[str, Any]], filters: set[str]) -> list[dict[str, Any]]:
    known = {str(repo["id"]) for repo in repos}
    unknown = sorted(filters - known)
    if unknown:
        raise SystemExit(f"unknown corpus repo id(s): {', '.join(unknown)}")
    return [repo for repo in repos if not filters or str(repo["id"]) in filters]


def language_for_path(path: Path) -> str | None:
    suffix = path.suffix.lower()
    lang = LANG_BY_EXT.get(suffix)
    if lang == "typescript" and path.name.endswith(".d.ts"):
        return "typescript"
    return lang


def iter_source_files(repo_dir: Path) -> list[tuple[Path, str]]:
    out: list[tuple[Path, str]] = []
    for root, dirs, files in os.walk(repo_dir):
        dirs[:] = [d for d in dirs if d not in SKIP_DIRS]
        root_path = Path(root)
        for name in files:
            path = root_path / name
            lang = language_for_path(path)
            if lang is not None:
                out.append((path, lang))
    return out


def scan_repo_sources(repo: dict[str, Any], repo_dir: Path) -> dict[str, Any]:
    compiled = [(pattern, pattern.compile()) for pattern in SOURCE_PATTERNS]
    language_files: Counter[str] = Counter()
    language_lines: Counter[str] = Counter()
    surface_counts: Counter[str] = Counter()
    surface_files: defaultdict[str, set[str]] = defaultdict(set)
    surface_languages: defaultdict[str, Counter[str]] = defaultdict(Counter)

    for path, lang in iter_source_files(repo_dir):
        language_files[lang] += 1
        try:
            text = path.read_text(encoding="utf-8", errors="ignore")
        except OSError:
            continue
        language_lines[lang] += text.count("\n") + (0 if text.endswith("\n") else 1)
        pattern_lang = "javascript" if lang in {"javascript", "typescript"} else lang
        for pattern, regex in compiled:
            if pattern.language != pattern_lang:
                continue
            matches = regex.findall(text)
            if not matches:
                continue
            count = len(matches)
            surface_counts[pattern.surface] += count
            surface_files[pattern.surface].add(str(path))
            surface_languages[pattern.surface][lang] += count

    surfaces = []
    by_id = {pattern.surface: pattern for pattern in SOURCE_PATTERNS}
    for surface, count in surface_counts.items():
        pattern = by_id[surface]
        surfaces.append(
            {
                "surface": surface,
                "language": pattern.language,
                "capability": pattern.capability,
                "status": pattern.status,
                "occurrences": count,
                "files": len(surface_files[surface]),
                "repo": repo["id"],
                "source_languages": sorted(surface_languages[surface].items()),
            }
        )
    surfaces.sort(key=lambda row: (-row["occurrences"], row["surface"]))

    return {
        "repo": repo["id"],
        "primary_language": repo["primary_language"].lower(),
        "source_files_by_language": dict(sorted(language_files.items())),
        "source_lines_by_language": dict(sorted(language_lines.items())),
        "stdlib_surfaces": surfaces,
    }


def run_verify_for_repo(
    repo: dict[str, Any],
    repo_dir: Path,
    nose: str,
    logs_dir: Path,
    timeout: int | None,
) -> dict[str, Any]:
    repo_id = str(repo["id"])
    report_path = logs_dir / "reports" / f"{repo_id}.recall-loss.json"
    log_path = logs_dir / "logs" / f"{repo_id}.log"
    report_path.parent.mkdir(parents=True, exist_ok=True)
    log_path.parent.mkdir(parents=True, exist_ok=True)

    if not repo_dir.is_dir():
        log_path.write_text(f"missing pinned corpus repo: {repo_dir}\n", encoding="utf-8")
        return {
            "repo": repo_id,
            "status": "missing",
            "exit_code": 127,
            "report": str(report_path),
            "log": str(log_path),
        }

    cmd = [
        nose,
        "verify",
        str(repo_dir),
        "--max-violations",
        "0",
        "--recall-loss-report",
        str(report_path),
    ]
    try:
        completed = subprocess.run(
            cmd,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            timeout=timeout,
            check=False,
        )
        log_path.write_text(completed.stdout, encoding="utf-8", errors="replace")
        return {
            "repo": repo_id,
            "status": "ran",
            "exit_code": completed.returncode,
            "report": str(report_path),
            "log": str(log_path),
        }
    except subprocess.TimeoutExpired as exc:
        output = exc.stdout or ""
        log_path.write_text(output + "\nTIMEOUT\n", encoding="utf-8", errors="replace")
        return {
            "repo": repo_id,
            "status": "timeout",
            "exit_code": 124,
            "report": str(report_path),
            "log": str(log_path),
        }


def load_recall_report(path: Path) -> dict[str, Any] | None:
    if not path.exists():
        return None
    try:
        report = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return None
    if report.get("schema_version") != 1:
        return None
    if report.get("report_kind") != "recall-loss-diagnostics":
        return None
    return report


def count_rows(rows: list[dict[str, Any]], key_field: str) -> Counter[str]:
    out: Counter[str] = Counter()
    for row in rows:
        out[str(row.get(key_field, "unknown"))] += int(row.get("count", 0))
    return out


def add_counter(dst: Counter[str], src: dict[str, int] | Counter[str]) -> None:
    for key, count in src.items():
        dst[key] += int(count)


def sorted_count_rows(counter: Counter[str], key_name: str = "key") -> list[dict[str, Any]]:
    rows = [{key_name: key, "count": count} for key, count in counter.items()]
    rows.sort(key=lambda row: (-row["count"], row[key_name]))
    return rows


def summarize_reports(
    repos: list[dict[str, Any]],
    logs_dir: Path,
    source_scans: list[dict[str, Any]],
    run_results: dict[str, dict[str, Any]],
) -> dict[str, Any]:
    repos_by_id = {str(repo["id"]): repo for repo in repos}
    totals: Counter[str] = Counter()
    hard_gate: Counter[str] = Counter()
    by_reason: Counter[str] = Counter()
    by_reason_repo: defaultdict[str, set[str]] = defaultdict(set)
    by_reason_primary_language: defaultdict[str, Counter[str]] = defaultdict(Counter)
    by_reason_unit_language: defaultdict[str, Counter[str]] = defaultdict(Counter)
    by_missing_evidence: Counter[str] = Counter()
    exclusions: Counter[str] = Counter()
    import_snapshot: Counter[str] = Counter()
    import_misses: Counter[str] = Counter()
    top_opportunities: list[dict[str, Any]] = []
    per_repo: list[dict[str, Any]] = []

    for repo in repos:
        repo_id = str(repo["id"])
        primary = str(repo.get("primary_language", "unknown")).lower()
        report_path = logs_dir / "reports" / f"{repo_id}.recall-loss.json"
        report = load_recall_report(report_path)
        run = run_results.get(repo_id, {})
        if report is None:
            per_repo.append(
                {
                    "repo": repo_id,
                    "primary_language": primary,
                    "status": run.get("status", "missing-report"),
                    "exit_code": run.get("exit_code"),
                    "report": str(report_path),
                }
            )
            continue

        summary = report.get("summary", {})
        soundness = report.get("soundness_gate", {})
        completeness = report.get("completeness", {})
        totals["reported_repos"] += 1
        for key in [
            "total_units",
            "interpretable_units",
            "excluded_units",
            "canon_checked",
            "admission_rejections",
        ]:
            totals[key] += int(summary.get(key, 0))
        for key in [
            "false_merges",
            "lossy_fingerprint_collisions",
            "advisory_disagreements",
            "canon_preservation_violations",
        ]:
            hard_gate[key] += int(soundness.get(key, 0))
        for key in [
            "behavior_equal_pairs",
            "fingerprint_equal_pairs",
            "under_merged_behavior_groups",
            "structurally_near_under_merged_groups",
        ]:
            totals[key] += int(completeness.get(key, 0))

        repo_reason = count_rows(report.get("by_reason", []), "reason")
        add_counter(by_reason, repo_reason)
        for reason, count in repo_reason.items():
            by_reason_repo[reason].add(repo_id)
            by_reason_primary_language[reason][primary] += count

        for rejection in report.get("admission_rejections", []):
            reason = str(rejection.get("reason", "unknown"))
            loc = rejection.get("loc", {})
            unit_lang = str(loc.get("language", "unknown"))
            by_reason_unit_language[reason][unit_lang] += 1
            for label in rejection.get("missing_evidence", []):
                by_missing_evidence[str(label)] += 1

        add_counter(exclusions, count_rows(report.get("oracle_exclusions", {}).get("counts", []), "reason"))

        import_census = report.get("import_snapshot_census", {})
        import_summary = import_census.get("summary", {})
        for key in ["snapshot_records", "unresolved_binding_imports", "reported_misses"]:
            import_snapshot[key] += int(import_summary.get(key, 0))
        for row in import_census.get("misses_by_reason", []):
            import_misses[str(row.get("key", "unknown"))] += int(row.get("count", 0))

        for opportunity in report.get("top_opportunities", [])[:5]:
            top_opportunities.append(
                {
                    "repo": repo_id,
                    "reason": opportunity.get("reason"),
                    "value_jaccard": opportunity.get("value_jaccard"),
                    "structurally_near": opportunity.get("structurally_near"),
                    "a": loc_key(opportunity.get("a", {})),
                    "b": loc_key(opportunity.get("b", {})),
                }
            )

        per_repo.append(
            {
                "repo": repo_id,
                "primary_language": primary,
                "status": run.get("status", "existing-report"),
                "exit_code": run.get("exit_code", 0),
                "summary": summary,
                "soundness_gate": soundness,
                "completeness": completeness,
                "by_reason": [
                    {"reason": reason, "count": count}
                    for reason, count in sorted(
                        repo_reason.items(), key=lambda item: (-item[1], item[0])
                    )
                ],
            }
        )

    source_summary = summarize_source_scans(source_scans)
    reason_rows = []
    manifest_files = {
        row["language"]: row["manifest_source_files"]
        for row in manifest_language_summary(repos)
    }
    for reason, count in by_reason.items():
        by_primary = sorted_count_rows(by_reason_primary_language[reason], "language")
        for row in by_primary:
            denom = manifest_files.get(row["language"], 0)
            row["per_1k_manifest_source_files"] = (
                round(1000.0 * row["count"] / denom, 3) if denom else None
            )
        reason_rows.append(
            {
                "reason": reason,
                "count": count,
                "repos": len(by_reason_repo[reason]),
                "by_primary_language": by_primary,
                "by_unit_language": sorted_count_rows(
                    by_reason_unit_language[reason], "language"
                ),
            }
        )
    reason_rows.sort(key=lambda row: (-row["count"], row["reason"]))

    candidate_priorities = [
        row
        for row in source_summary["stdlib_surfaces"]
        if row["status"] == "candidate" or row["status"] == "covered-partial"
    ]
    candidate_priorities.sort(
        key=lambda row: (
            0 if row["status"] == "candidate" else 1,
            -row["occurrences"],
            row["surface"],
        )
    )

    top_opportunities.sort(
        key=lambda row: (-(row.get("value_jaccard") or 0), row["repo"], row["a"])
    )

    return {
        "schema_version": 1,
        "report_kind": "corpus-priority-census",
        "corpus": {
            "manifest": DEFAULT_MANIFEST,
            "repositories_selected": len(repos),
            "repositories_reported": totals["reported_repos"],
            "by_primary_language": manifest_language_summary(repos),
        },
        "hard_gate": dict(sorted(hard_gate.items())),
        "summary": dict(sorted(totals.items())),
        "recall_loss": {
            "by_reason": reason_rows,
            "missing_evidence": sorted_count_rows(by_missing_evidence, "missing_evidence"),
            "oracle_exclusions": sorted_count_rows(exclusions, "reason"),
            "top_opportunities": top_opportunities[:50],
        },
        "import_snapshot_census": {
            "summary": dict(sorted(import_snapshot.items())),
            "misses_by_reason": sorted_count_rows(import_misses, "reason"),
        },
        "source_scan": source_summary,
        "recommended_source_stdlib_priorities": candidate_priorities[:30],
        "per_repo": per_repo,
    }


def loc_key(loc: dict[str, Any]) -> str:
    return (
        f"{loc.get('file', '')}:"
        f"{loc.get('start_line', 0)}:"
        f"{loc.get('end_line', 0)}"
    )


def summarize_source_scans(scans: list[dict[str, Any]]) -> dict[str, Any]:
    files_by_lang: Counter[str] = Counter()
    lines_by_lang: Counter[str] = Counter()
    surface_counts: Counter[str] = Counter()
    surface_files: Counter[str] = Counter()
    surface_repos: defaultdict[str, set[str]] = defaultdict(set)
    surface_languages: defaultdict[str, Counter[str]] = defaultdict(Counter)
    pattern_by_id = {pattern.surface: pattern for pattern in SOURCE_PATTERNS}

    for scan in scans:
        add_counter(files_by_lang, scan.get("source_files_by_language", {}))
        add_counter(lines_by_lang, scan.get("source_lines_by_language", {}))
        for row in scan.get("stdlib_surfaces", []):
            surface = row["surface"]
            surface_counts[surface] += int(row["occurrences"])
            surface_files[surface] += int(row["files"])
            surface_repos[surface].add(str(row["repo"]))
            for language, count in row.get("source_languages", []):
                surface_languages[surface][language] += int(count)

    surfaces = []
    for surface, count in surface_counts.items():
        pattern = pattern_by_id[surface]
        lang_files = files_by_lang[pattern.language]
        if pattern.language == "javascript":
            lang_files += files_by_lang["typescript"]
        surfaces.append(
            {
                "surface": surface,
                "language": pattern.language,
                "capability": pattern.capability,
                "status": pattern.status,
                "occurrences": count,
                "files": surface_files[surface],
                "repos": len(surface_repos[surface]),
                "per_1k_scanned_source_files": (
                    round(1000.0 * count / lang_files, 3) if lang_files else None
                ),
                "source_languages": sorted_count_rows(surface_languages[surface], "language"),
            }
        )
    surfaces.sort(key=lambda row: (-row["occurrences"], row["surface"]))

    capability_counts: defaultdict[str, Counter[str]] = defaultdict(Counter)
    for row in surfaces:
        capability_counts[row["capability"]]["occurrences"] += row["occurrences"]
        capability_counts[row["capability"]]["surfaces"] += 1
    capabilities = [
        {
            "capability": capability,
            "occurrences": counts["occurrences"],
            "surfaces": counts["surfaces"],
            "per_surface_mean_occurrences": round(
                counts["occurrences"] / counts["surfaces"], 3
            ),
        }
        for capability, counts in capability_counts.items()
    ]
    capabilities.sort(key=lambda row: (-row["occurrences"], row["capability"]))

    return {
        "source_files_by_language": sorted_count_rows(files_by_lang, "language"),
        "source_lines_by_language": sorted_count_rows(lines_by_lang, "language"),
        "stdlib_surfaces": surfaces,
        "capabilities": capabilities,
    }


def manifest_language_summary(repos: list[dict[str, Any]]) -> list[dict[str, Any]]:
    counts: defaultdict[str, Counter[str]] = defaultdict(Counter)
    for repo in repos:
        language = str(repo.get("primary_language", "unknown")).lower()
        counts[language]["repos"] += 1
        counts[language]["manifest_source_files"] += int(repo.get("source_file_count", 0))
    rows = [
        {
            "language": language,
            "repos": counter["repos"],
            "manifest_source_files": counter["manifest_source_files"],
        }
        for language, counter in counts.items()
    ]
    rows.sort(key=lambda row: row["language"])
    return rows


def write_json(path: Path, data: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(data, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def run_census(args: argparse.Namespace) -> dict[str, Any]:
    manifest = Path(args.manifest)
    repos_root = Path(args.repos_root)
    logs_dir = Path(args.logs_dir)
    repos = select_repos(load_manifest(manifest), set(args.repo))
    logs_dir.mkdir(parents=True, exist_ok=True)

    source_scans = []
    for repo in repos:
        source_scans.append(scan_repo_sources(repo, repos_root / str(repo["id"])))

    run_results: dict[str, dict[str, Any]] = {}
    if not args.scan_only and not args.summarize_only:
        with concurrent.futures.ThreadPoolExecutor(max_workers=args.jobs) as pool:
            futures = {
                pool.submit(
                    run_verify_for_repo,
                    repo,
                    repos_root / str(repo["id"]),
                    args.nose,
                    logs_dir,
                    args.timeout,
                ): str(repo["id"])
                for repo in repos
            }
            for future in concurrent.futures.as_completed(futures):
                repo_id = futures[future]
                result = future.result()
                run_results[repo_id] = result
                print(
                    f"{repo_id}: {result['status']} exit={result.get('exit_code')}",
                    file=sys.stderr,
                    flush=True,
                )

    if args.summarize_only or args.scan_only:
        for repo in repos:
            report = logs_dir / "reports" / f"{repo['id']}.recall-loss.json"
            run_results[str(repo["id"])] = {
                "repo": str(repo["id"]),
                "status": "existing-report" if report.exists() else "missing-report",
                "exit_code": 0 if report.exists() else None,
                "report": str(report),
            }

    report = summarize_reports(repos, logs_dir, source_scans, run_results)
    report["corpus"]["manifest"] = str(manifest)
    report["corpus"]["repos_root"] = str(repos_root)
    report["command"] = {
        "script": "scripts/corpus-priority-census.py",
        "nose": args.nose,
        "logs_dir": str(logs_dir),
        "jobs": args.jobs,
        "scan_only": args.scan_only,
        "summarize_only": args.summarize_only,
        "repo_filters": args.repo,
    }
    return report


def self_test() -> None:
    with tempfile.TemporaryDirectory(prefix="nose-corpus-priority-census.") as tmp:
        root = Path(tmp)
        manifest = root / "corpus.json"
        repos = root / "repos"
        logs = root / "logs"
        out = root / "out.json"
        (repos / "pyrepo").mkdir(parents=True)
        (repos / "rsrepo").mkdir(parents=True)
        (repos / "pyrepo" / "x.py").write_text(
            "import collections\nxs = collections.deque([1])\nmath.sqrt(4)\n",
            encoding="utf-8",
        )
        (repos / "rsrepo" / "x.rs").write_text(
            "use std::collections::HashMap;\nfn f() { let _ = Vec::new(); let _m = HashMap::new(); }\n",
            encoding="utf-8",
        )
        manifest.write_text(
            json.dumps(
                {
                    "repositories": [
                        {
                            "id": "pyrepo",
                            "primary_language": "Python",
                            "source_file_count": 1,
                        },
                        {
                            "id": "rsrepo",
                            "primary_language": "Rust",
                            "source_file_count": 1,
                        },
                    ]
                }
            ),
            encoding="utf-8",
        )
        fake_nose = root / "fake_nose.py"
        fake_nose.write_text(
            """#!/usr/bin/env python3
import json
import pathlib
import sys
repo = pathlib.Path(sys.argv[2]).name
report = pathlib.Path(sys.argv[sys.argv.index('--recall-loss-report') + 1])
report.parent.mkdir(parents=True, exist_ok=True)
reason = 'receiver-domain-proof-missing' if repo == 'pyrepo' else 'import-symbol-callee-identity-proof-missing'
report.write_text(json.dumps({
  'schema_version': 1,
  'report_kind': 'recall-loss-diagnostics',
  'summary': {'total_units': 2, 'interpretable_units': 1, 'excluded_units': 1, 'canon_checked': 0, 'canon_preservation_violations': 0, 'admission_rejections': 1},
  'soundness_gate': {'fingerprint_groups': 0, 'false_merges': 0, 'lossy_fingerprint_collisions': 0, 'advisory_disagreements': 0, 'canon_preservation_violations': 0, 'gate_passed': True},
  'completeness': {'behavior_groups': 0, 'behavior_equal_pairs': 0, 'fingerprint_equal_pairs': 0, 'under_merged_behavior_groups': 0, 'structurally_near_under_merged_groups': 0},
  'oracle_exclusions': {'counts': [{'reason': 'uninterpretable', 'count': 1}], 'units': []},
  'import_snapshot_census': {'summary': {'snapshot_records': 0, 'unresolved_binding_imports': 0, 'reported_misses': 0}, 'misses_by_reason': []},
  'admission_rejections': [{'reason': reason, 'missing_evidence': ['receiver-domain-proof'], 'loc': {'file': repo + '/x', 'start_line': 1, 'end_line': 1, 'tokens': 3, 'language': repo}}],
  'by_reason': [{'reason': reason, 'admission_gate': 'gate', 'capability_id': 'cap', 'count': 1, 'oracle_interpretable': 1}],
  'top_opportunities': []
}), encoding='utf-8')
print('ok')
""",
            encoding="utf-8",
        )
        fake_nose.chmod(0o755)
        args = parse_args(
            [
                "--manifest",
                str(manifest),
                "--repos-root",
                str(repos),
                "--logs-dir",
                str(logs),
                "--output",
                str(out),
                "--nose",
                str(fake_nose),
                "--jobs",
                "2",
            ]
        )
        report = run_census(args)
        write_json(out, report)
        assert report["corpus"]["repositories_reported"] == 2
        assert report["hard_gate"]["false_merges"] == 0
        reasons = {
            row["reason"]: row["count"]
            for row in report["recall_loss"]["by_reason"]
        }
        assert reasons["receiver-domain-proof-missing"] == 1
        surfaces = {
            row["surface"]: row["occurrences"]
            for row in report["source_scan"]["stdlib_surfaces"]
        }
        assert surfaces["python.stdlib.collections"] == 1
        assert surfaces["rust.stdlib.vec"] == 1
        assert surfaces["rust.stdlib.maps"] == 1


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Run and summarize a corpus-wide recall-loss/source stdlib priority census."
    )
    parser.add_argument("--manifest", default=DEFAULT_MANIFEST)
    parser.add_argument("--repos-root", default=DEFAULT_REPOS_ROOT)
    parser.add_argument("--logs-dir", default=DEFAULT_LOGS_DIR)
    parser.add_argument("--output", default=DEFAULT_OUTPUT)
    parser.add_argument("--nose", default="target/release/nose")
    parser.add_argument("--jobs", type=int, default=default_jobs())
    parser.add_argument("--timeout", type=int, default=None)
    parser.add_argument("--repo", action="append", default=[])
    parser.add_argument("--scan-only", action="store_true")
    parser.add_argument("--summarize-only", action="store_true")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args(argv)
    if args.jobs < 1:
        parser.error("--jobs must be positive")
    if args.scan_only and args.summarize_only:
        parser.error("--scan-only and --summarize-only are mutually exclusive")
    return args


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    if args.self_test:
        self_test()
        print("ok corpus priority census self-test")
        return 0
    report = run_census(args)
    write_json(Path(args.output), report)
    print(args.output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
