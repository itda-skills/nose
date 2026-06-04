#!/usr/bin/env python3
"""Rank Type-4 frontier candidates from the pinned real-repo corpus."""

from __future__ import annotations

import argparse
import json
import math
import re
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable


ROOT = Path(__file__).resolve().parents[2]
DEFAULT_CORPUS = ROOT / "bench" / "goldens" / "corpus.json"
DEFAULT_REPOS_ROOT = ROOT / "bench" / "repos"

SKIP_DIRS = {
    ".git",
    ".hg",
    ".svn",
    ".cache",
    ".mypy_cache",
    ".pytest_cache",
    ".ruff_cache",
    ".tox",
    ".venv",
    ".yarn",
    "build",
    "coverage",
    "dist",
    "node_modules",
    "target",
    "vendor",
}
LANG_BY_EXT = {
    ".c": "c",
    ".h": "c",
    ".go": "go",
    ".java": "java",
    ".js": "javascript",
    ".jsx": "javascript",
    ".mjs": "javascript",
    ".cjs": "javascript",
    ".py": "python",
    ".rb": "ruby",
    ".rs": "rust",
    ".ts": "typescript",
    ".tsx": "typescript",
}
@dataclass(frozen=True)
class PatternSpec:
    lang: str
    regex: re.Pattern[str]


@dataclass(frozen=True)
class Candidate:
    candidate_id: str
    title: str
    scope: str
    implementation_cost: int
    soundness_risk: int
    status: str
    why: str
    next_probe: str
    patterns: tuple[PatternSpec, ...]


def pat(lang: str, regex: str) -> PatternSpec:
    return PatternSpec(lang, re.compile(regex))


CANDIDATES = [
    Candidate(
        "collection_empty_check",
        "Collection emptiness and non-emptiness",
        "all-language",
        2,
        2,
        "open",
        "Most languages expose both length comparison and named emptiness predicates.",
        "Generate `len(x) == 0` / `.is_empty()` / `.isEmpty()` / `.empty?` positives, with nonzero and wrong-collection negatives.",
        (
            pat("c", r"\b(?:len|length|n|count)\s*(?:==|!=|>|>=)\s*0\b"),
            pat("go", r"\blen\s*\([^)]{1,80}\)\s*(?:==|!=|>|>=)\s*0\b"),
            pat("java", r"\.\s*(?:isEmpty|size)\s*\(\s*\)\s*(?:(?:==|!=|>|>=)\s*0)?"),
            pat("javascript", r"\.\s*length\s*(?:={2,3}|!==|!=|>|>=)\s*0\b"),
            pat("python", r"\blen\s*\([^)]{1,80}\)\s*(?:==|!=|>|>=)\s*0\b|\bif\s+(?:not\s+)?[A-Za-z_][\w.]*\s*:"),
            pat("ruby", r"\.\s*(?:empty\?|any\?)\b"),
            pat("rust", r"\.\s*(?:is_empty|len)\s*\(\s*\)\s*(?:(?:==|!=|>|>=)\s*0)?"),
            pat("typescript", r"\.\s*length\s*(?:={2,3}|!==|!=|>|>=)\s*0\b"),
        ),
    ),
    Candidate(
        "string_prefix_suffix",
        "String prefix/suffix predicates",
        "all-language",
        2,
        2,
        "open",
        "The API names differ by language but the strict predicate coordinate is simple.",
        "Lower case-sensitive starts-with/ends-with calls to prefix/suffix facts; keep regex, contains, and case-folding boundaries.",
        (
            pat("go", r"\bstrings\.(?:HasPrefix|HasSuffix)\s*\("),
            pat("java", r"\.\s*(?:startsWith|endsWith)\s*\("),
            pat("javascript", r"\.\s*(?:startsWith|endsWith)\s*\("),
            pat("python", r"\.\s*(?:startswith|endswith)\s*\("),
            pat("ruby", r"\.\s*(?:start_with\?|end_with\?)\s*\("),
            pat("rust", r"\.\s*(?:starts_with|ends_with)\s*\("),
            pat("typescript", r"\.\s*(?:startsWith|endsWith)\s*\("),
        ),
    ),
    Candidate(
        "membership_contains",
        "Membership and contains predicates",
        "multi-language",
        3,
        3,
        "open",
        "Common but semantically overloaded: substring, list membership, map key membership, and set membership must stay distinct.",
        "Start with static set/list membership only; keep substring, regex, and map-key boundaries separate.",
        (
            pat("go", r"\b(?:slices\.Contains|maps\.Keys|_,\s*\w+\s*:=\s*\w+\s*\[[^\]]+\])"),
            pat("java", r"\.\s*(?:contains|containsKey)\s*\("),
            pat("javascript", r"\.\s*(?:includes|has|indexOf)\s*\("),
            pat("python", r"\b(?:in|not\s+in)\b"),
            pat("ruby", r"\.\s*(?:include\?|key\?|has_key\?)\s*\("),
            pat("rust", r"\.\s*(?:contains|contains_key)\s*\("),
            pat("typescript", r"\.\s*(?:includes|has|indexOf)\s*\("),
        ),
    ),
    Candidate(
        "null_option_presence",
        "Null, nil, none, and option presence guards",
        "all-language",
        3,
        3,
        "partially-covered",
        "Nullish default is covered for JS-family, but cross-language presence guards are not modeled as a common proof fact.",
        "Generate pure presence predicates and hard negatives for falsy values, absent options, and pointer aliases.",
        (
            pat("c", r"(?:==|!=)\s*NULL\b|\bNULL\s*(?:==|!=)"),
            pat("go", r"(?:==|!=)\s*nil\b|\bnil\s*(?:==|!=)"),
            pat("java", r"(?:==|!=)\s*null\b|\bnull\s*(?:==|!=)"),
            pat("javascript", r"(?:={2,3}|!==|!=)\s*(?:null|undefined)\b|\?\?"),
            pat("python", r"\bis\s+(?:not\s+)?None\b"),
            pat("ruby", r"\.\s*nil\?\b"),
            pat("rust", r"\.\s*(?:is_some|is_none)\s*\(\s*\)|\b(?:Some|None)\b"),
            pat("typescript", r"(?:={2,3}|!==|!=)\s*(?:null|undefined)\b|\?\?"),
        ),
    ),
    Candidate(
        "map_default_lookup",
        "Map/dict lookup with default",
        "multi-language",
        4,
        4,
        "open",
        "Potentially high value, but absent-key semantics and mutation/effects vary heavily.",
        "Start with literal immutable maps and static keys; hard-negative missing-key/default-value changes.",
        (
            pat("go", r"\w+\s*,\s*\w+\s*:=\s*\w+\s*\[[^\]]+\]"),
            pat("java", r"\.\s*(?:getOrDefault|get)\s*\("),
            pat("javascript", r"\.\s*get\s*\(|\?\?|\|\|"),
            pat("python", r"\.\s*get\s*\("),
            pat("ruby", r"\.\s*(?:fetch|dig)\s*\("),
            pat("rust", r"\.\s*get\s*\([^)]*\)\s*\.\s*(?:unwrap_or|unwrap_or_else)"),
            pat("typescript", r"\.\s*get\s*\(|\?\?|\|\|"),
        ),
    ),
    Candidate(
        "numeric_minmax_abs",
        "Scalar min/max/abs idioms",
        "all-language",
        2,
        2,
        "partially-covered",
        "Aggregate abs is covered, but scalar min/max/abs expression facts are separate and cheap.",
        "Generate scalar `abs`, `min`, and `max` positives with sign/order hard negatives.",
        (
            pat("c", r"\b(?:abs|labs|fmin|fmax|min|max)\s*\("),
            pat("go", r"\b(?:math\.(?:Abs|Min|Max)|min|max)\s*\("),
            pat("java", r"\bMath\.(?:abs|min|max)\s*\("),
            pat("javascript", r"\bMath\.(?:abs|min|max)\s*\("),
            pat("python", r"\b(?:abs|min|max)\s*\("),
            pat("ruby", r"\.\s*abs\b|\[(?:[^\]]+)\]\.\s*(?:min|max)\b"),
            pat("rust", r"\.\s*(?:abs|min|max)\s*\("),
            pat("typescript", r"\bMath\.(?:abs|min|max)\s*\("),
        ),
    ),
    Candidate(
        "property_type_guard",
        "Property type guards",
        "language-family",
        2,
        3,
        "open",
        "Very frequent in JS-family repos, but the scope is narrow and should wait behind broader axes.",
        "Generate `typeof obj.field === <type>` variants with dynamic-key and shadowing boundaries.",
        (
            pat("javascript", r"typeof\s+[A-Za-z_$][\w$]*(?:\.[A-Za-z_$][\w$]*|\[['\"][^'\"]+['\"]\])\s*={2,3}\s*['\"](?:string|number|boolean|function|object)['\"]"),
            pat("typescript", r"typeof\s+[A-Za-z_$][\w$]*(?:\.[A-Za-z_$][\w$]*|\[['\"][^'\"]+['\"]\])\s*={2,3}\s*['\"](?:string|number|boolean|function|object)['\"]"),
        ),
    ),
    Candidate(
        "own_property_guard",
        "Own-property guard forms",
        "language-family",
        1,
        3,
        "covered-current",
        "Covered by the current loop; retained so future reports show why it should not be picked again.",
        "No new loop; monitor real-corpus yield.",
        (
            pat("javascript", r"\bObject\.hasOwn\s*\(|Object\.prototype\.hasOwnProperty\.call\s*\(|\.hasOwnProperty\s*\(|['\"][\w$-]+['\"]\s+in\s+\w+"),
            pat("typescript", r"\bObject\.hasOwn\s*\(|Object\.prototype\.hasOwnProperty\.call\s*\(|\.hasOwnProperty\s*\(|['\"][\w$-]+['\"]\s+in\s+\w+"),
        ),
    ),
]


def load_repos(corpus_path: Path, repos_root: Path) -> list[dict]:
    if corpus_path.exists():
        doc = json.loads(corpus_path.read_text())
        return [
            {
                "id": repo["id"],
                "split": repo.get("split", ""),
                "primary_language": repo.get("primary_language", ""),
                "path": repos_root / repo["id"],
            }
            for repo in doc.get("repositories", [])
            if (repos_root / repo["id"]).is_dir()
        ]
    return [
        {"id": p.name, "split": "", "primary_language": "", "path": p}
        for p in sorted(repos_root.iterdir())
        if p.is_dir()
    ]


def iter_source_files(repo_path: Path, max_bytes: int) -> Iterable[tuple[Path, str]]:
    for path in repo_path.rglob("*"):
        if not path.is_file():
            continue
        if any(part in SKIP_DIRS for part in path.parts):
            continue
        if path.name.endswith(".min.js"):
            continue
        lang = LANG_BY_EXT.get(path.suffix)
        if lang is None:
            continue
        try:
            if path.stat().st_size > max_bytes:
                continue
        except OSError:
            continue
        yield path, lang


def line_number(text: str, offset: int) -> int:
    return text.count("\n", 0, offset) + 1


def snippet(text: str, start: int, end: int) -> str:
    line_start = text.rfind("\n", 0, start) + 1
    line_end = text.find("\n", end)
    if line_end == -1:
        line_end = len(text)
    return " ".join(text[line_start:line_end].strip().split())[:180]


def scope_weight(scope: str) -> float:
    return {
        "all-language": 1.35,
        "multi-language": 1.15,
        "language-family": 0.7,
        "single-language": 0.45,
    }.get(scope, 1.0)


def status_weight(status: str) -> float:
    return {
        "open": 1.0,
        "partially-covered": 0.65,
        "covered-current": 0.08,
    }.get(status, 1.0)


def rank_score(matches: int, repos: int, languages: int, candidate: Candidate) -> float:
    if matches == 0:
        return 0.0
    impact = math.log1p(matches) * math.sqrt(repos) * (1.0 + 0.35 * max(0, languages - 1))
    cost = candidate.implementation_cost + candidate.soundness_risk
    return impact * scope_weight(candidate.scope) * status_weight(candidate.status) / cost


def analyze(corpus_path: Path, repos_root: Path, max_bytes: int, sample_limit: int) -> dict:
    repos = load_repos(corpus_path, repos_root)
    totals = {
        candidate.candidate_id: {
            "candidate": candidate,
            "matches": 0,
            "repos": set(),
            "languages": set(),
            "splits": {},
            "samples": [],
        }
        for candidate in CANDIDATES
    }
    files_scanned = 0
    bytes_scanned = 0

    for repo in repos:
        repo_path = repo["path"]
        for path, lang in iter_source_files(repo_path, max_bytes):
            try:
                text = path.read_text(errors="ignore")
            except OSError:
                continue
            files_scanned += 1
            bytes_scanned += len(text)
            rel = str(path.relative_to(repo_path))
            for candidate in CANDIDATES:
                specs = [spec for spec in candidate.patterns if spec.lang == lang]
                if not specs:
                    continue
                total_for_file = 0
                first_matches = []
                for spec in specs:
                    matches = list(spec.regex.finditer(text))
                    total_for_file += len(matches)
                    first_matches.extend(matches[: max(0, sample_limit - len(first_matches))])
                if total_for_file == 0:
                    continue
                bucket = totals[candidate.candidate_id]
                bucket["matches"] += total_for_file
                bucket["repos"].add(repo["id"])
                bucket["languages"].add(lang)
                split = repo.get("split") or "unknown"
                bucket["splits"][split] = bucket["splits"].get(split, 0) + total_for_file
                for match in first_matches:
                    if len(bucket["samples"]) >= sample_limit:
                        break
                    bucket["samples"].append(
                        {
                            "repo": repo["id"],
                            "split": split,
                            "language": lang,
                            "path": rel,
                            "line": line_number(text, match.start()),
                            "snippet": snippet(text, match.start(), match.end()),
                        }
                    )

    rows = []
    for candidate in CANDIDATES:
        bucket = totals[candidate.candidate_id]
        repos_count = len(bucket["repos"])
        languages_count = len(bucket["languages"])
        rows.append(
            {
                "candidate_id": candidate.candidate_id,
                "title": candidate.title,
                "scope": candidate.scope,
                "status": candidate.status,
                "matches": bucket["matches"],
                "repos": repos_count,
                "languages": sorted(bucket["languages"]),
                "language_count": languages_count,
                "splits": dict(sorted(bucket["splits"].items())),
                "implementation_cost": candidate.implementation_cost,
                "soundness_risk": candidate.soundness_risk,
                "score": rank_score(bucket["matches"], repos_count, languages_count, candidate),
                "why": candidate.why,
                "next_probe": candidate.next_probe,
                "samples": bucket["samples"],
            }
        )
    rows.sort(key=lambda row: row["score"], reverse=True)
    return {
        "schema_version": "0.1.0",
        "repos_root": str(repos_root),
        "corpus": str(corpus_path),
        "repo_count": len(repos),
        "files_scanned": files_scanned,
        "bytes_scanned": bytes_scanned,
        "max_bytes_per_file": max_bytes,
        "ranking": rows,
    }


def markdown_report(result: dict, top: int) -> str:
    lines = [
        "# Type-4 frontier priorities",
        "",
        "This report is generated from the pinned benchmark repos by",
        "`bench/type4/prioritize_frontier.py`. Scores combine real-code frequency,",
        "repo/language spread, estimated implementation cost, soundness risk, scope,",
        "and whether a frontier is already covered.",
        "",
        f"- repos scanned: {result['repo_count']}",
        f"- files scanned: {result['files_scanned']}",
        f"- max bytes per file: {result['max_bytes_per_file']}",
        "",
        "| rank | candidate | scope | status | score | matches | repos | languages | cost | risk |",
        "|---:|---|---|---|---:|---:|---:|---:|---:|---:|",
    ]
    for idx, row in enumerate(result["ranking"][:top], start=1):
        lines.append(
            "| {rank} | `{candidate_id}` | {scope} | {status} | {score:.2f} | {matches} | {repos} | {language_count} | {implementation_cost} | {soundness_risk} |".format(
                rank=idx,
                **row,
            )
        )
    lines.extend(["", "## Recommended Order", ""])
    open_rows = [row for row in result["ranking"] if row["status"] == "open"]
    for idx, row in enumerate(open_rows[:top], start=1):
        langs = ", ".join(row["languages"])
        lines.extend(
            [
                f"{idx}. `{row['candidate_id']}`",
                f"   - why: {row['why']}",
                f"   - evidence: {row['matches']} matches across {row['repos']} repos and {row['language_count']} languages ({langs})",
                f"   - next probe: {row['next_probe']}",
            ]
        )
    lines.extend(["", "## Samples", ""])
    for row in result["ranking"][:top]:
        lines.append(f"### `{row['candidate_id']}`")
        if not row["samples"]:
            lines.append("")
            continue
        for sample in row["samples"][:5]:
            lines.append(
                f"- `{sample['repo']}/{sample['path']}:{sample['line']}` ({sample['language']}): {sample['snippet']}"
            )
        lines.append("")
    return "\n".join(lines).rstrip() + "\n"


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--corpus", type=Path, default=DEFAULT_CORPUS)
    parser.add_argument("--repos-root", type=Path, default=DEFAULT_REPOS_ROOT)
    parser.add_argument("--max-bytes", type=int, default=512_000)
    parser.add_argument("--sample-limit", type=int, default=12)
    parser.add_argument("--top", type=int, default=8)
    parser.add_argument("--json-out", type=Path)
    parser.add_argument("--markdown-out", type=Path)
    args = parser.parse_args()

    result = analyze(args.corpus, args.repos_root, args.max_bytes, args.sample_limit)
    if args.json_out:
        args.json_out.parent.mkdir(parents=True, exist_ok=True)
        args.json_out.write_text(json.dumps(result, indent=2, ensure_ascii=False) + "\n")
    if args.markdown_out:
        args.markdown_out.parent.mkdir(parents=True, exist_ok=True)
        args.markdown_out.write_text(markdown_report(result, args.top))
    if not args.json_out and not args.markdown_out:
        print(json.dumps(result, indent=2, ensure_ascii=False))


if __name__ == "__main__":
    main()
