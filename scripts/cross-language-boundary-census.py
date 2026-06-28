#!/usr/bin/env python3
"""Build the #594 cross-language scheduling/error/callback boundary census.

This is a source-prevalence and recall-loss attribution report, not semantic
proof. It normalizes the existing language-specific audit reports into one
obligation taxonomy and adds conservative Ruby/C lexical pricing for the two
languages that do not yet have dedicated stdlib partial-coverage audits.
"""

from __future__ import annotations

import argparse
import json
import re
from collections import Counter, defaultdict
from dataclasses import dataclass
from pathlib import Path
from typing import Any


DEFAULT_MANIFEST = "bench/goldens/corpus.json"
DEFAULT_REPOS_ROOT = "bench/repos"
DEFAULT_OUTPUT = "target/cross-language-boundary-census-594.v1.json"
DEFAULT_GENERATED_ON = "2026-06-28"

DEFAULT_REPORTS = {
    "go": "bench/recall_loss/go-stdlib-collections-audit-2026-06-28.v1.json",
    "java": "bench/recall_loss/java-arrays-collections-audit-2026-06-28.v1.json",
    "js-ts": "bench/recall_loss/js-ts-stdlib-partial-audit-2026-06-28.v1.json",
    "python": "bench/recall_loss/python-hof-runtime-audit-2026-06-28.v3.json",
    "rust": "bench/recall_loss/rust-stdlib-partial-audit-2026-06-28.v2.json",
    "swift": "bench/recall_loss/swift-stdlib-partial-audit-2026-06-28.v2.json",
}

SKIP_DIRS = {
    ".bundle",
    ".git",
    ".gradle",
    ".next",
    ".nuxt",
    ".pytest_cache",
    ".svelte-kit",
    ".tox",
    ".venv",
    "__pycache__",
    "build",
    "coverage",
    "DerivedData",
    "dist",
    "node_modules",
    "out",
    "Pods",
    "target",
    "vendor",
    "venv",
}

RUBY_METHOD_BOUNDARIES: dict[str, tuple[str, str, str, str]] = {
    "collect": ("hof-callback-proof", "callback-demand-effect", "synchronous Enumerable callback"),
    "each": ("effect-callback", "effect-preserving-contracts", "effect-oriented block iteration"),
    "each_pair": ("effect-callback", "effect-preserving-contracts", "effect-oriented pair block iteration"),
    "each_with_index": ("effect-callback", "effect-preserving-contracts", "effect-oriented indexed block iteration"),
    "each_with_object": (
        "effect-callback",
        "effect-preserving-contracts",
        "effect-oriented block iteration with accumulator object",
    ),
    "filter": ("hof-callback-proof", "callback-demand-effect", "synchronous Enumerable predicate"),
    "flat_map": ("hof-callback-proof", "callback-demand-effect", "synchronous flattening callback"),
    "inject": ("reduction-callback", "callback-demand-effect", "reduction block"),
    "lazy": ("iterator-lifecycle", "iterator-hof-materialization", "lazy Enumerator lifecycle"),
    "map": ("hof-callback-proof", "callback-demand-effect", "synchronous Enumerable callback"),
    "reduce": ("reduction-callback", "callback-demand-effect", "reduction block"),
    "reject": ("hof-callback-proof", "callback-demand-effect", "synchronous Enumerable predicate"),
    "select": ("hof-callback-proof", "callback-demand-effect", "synchronous Enumerable predicate"),
    "sort": ("ordering-callback", "ordering-semantics", "optional comparator block"),
    "sort_by": ("ordering-callback", "ordering-semantics", "key callback ordering"),
    "tap": ("effect-callback", "effect-preserving-contracts", "effect-oriented callback"),
    "then": ("hof-callback-proof", "callback-demand-effect", "synchronous object-yield callback"),
}

RUBY_METHOD_RE = re.compile(
    r"\.\s*(?P<method>"
    + "|".join(re.escape(method) for method in sorted(RUBY_METHOD_BOUNDARIES, key=len, reverse=True))
    + r")[!?]?\b"
)
RUBY_RAISE_RE = re.compile(r"\braise\b")
RUBY_RESCUE_RE = re.compile(r"\brescue\b")
RUBY_THREAD_RE = re.compile(r"\b(?:Thread|Fiber)\s*\.\s*(?:new|schedule)\b")

C_PATTERNS: list[tuple[str, str, str, str, str, re.Pattern[str]]] = [
    (
        "c.stdlib.callback",
        "qsort",
        "ordering-callback",
        "callback-demand-effect",
        "stdlib comparator callback",
        re.compile(r"\bqsort\s*\("),
    ),
    (
        "c.stdlib.callback",
        "bsearch",
        "ordering-callback",
        "callback-demand-effect",
        "stdlib comparator callback",
        re.compile(r"\bbsearch\s*\("),
    ),
    (
        "c.stdlib.allocation",
        "malloc/calloc/realloc/free",
        "allocation-lifetime",
        "allocation-lifetime",
        "manual allocation and lifetime boundary",
        re.compile(r"\b(?:malloc|calloc|realloc|free)\s*\("),
    ),
    (
        "c.stdlib.string_memory",
        "memcpy/memmove/memset",
        "mutation-effect",
        "effect-preserving-contracts",
        "pointer/memory mutation boundary",
        re.compile(r"\b(?:memcpy|memmove|memset)\s*\("),
    ),
    (
        "c.stdlib.string_memory",
        "strcpy/strncpy/strcat/strncat",
        "mutation-effect",
        "effect-preserving-contracts",
        "C string mutation and bounds boundary",
        re.compile(r"\b(?:strcpy|strncpy|strcat|strncat)\s*\("),
    ),
    (
        "c.language.nonlocal_jump",
        "setjmp/longjmp",
        "exception-channel",
        "exception-channel",
        "non-local control transfer",
        re.compile(r"\b(?:setjmp|longjmp)\s*\("),
    ),
    (
        "c.language.error_channel",
        "errno",
        "error-channel",
        "error-channel",
        "ambient errno error channel",
        re.compile(r"\berrno\b"),
    ),
    (
        "c.library.threads",
        "pthread_create",
        "scheduling-boundary",
        "scheduling-boundary",
        "thread scheduling boundary",
        re.compile(r"\bpthread_create\s*\("),
    ),
]


@dataclass(frozen=True)
class BoundaryRow:
    language: str
    surface: str
    operation: str
    boundary: str
    capability: str
    status: str
    occurrences: int
    repos: int
    note: str
    source: str
    top_repos: tuple[tuple[str, int], ...] = ()

    def as_json(self) -> dict[str, Any]:
        family = obligation_family(self.boundary, self.capability, self.language, self.surface)
        secondary = secondary_families(self.boundary, family)
        return {
            "language": self.language,
            "surface": self.surface,
            "operation": self.operation,
            "occurrences": self.occurrences,
            "repos": self.repos,
            "status": self.status,
            "boundary": self.boundary,
            "capability": self.capability,
            "obligation_family": family,
            "secondary_obligation_families": secondary,
            "actionability": actionability(self.status, family, self.boundary),
            "note": self.note,
            "source": self.source,
            "top_repos": [
                {"repo": repo, "occurrences": occurrences}
                for repo, occurrences in self.top_repos
            ],
        }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", default=DEFAULT_MANIFEST)
    parser.add_argument("--repos-root", default=DEFAULT_REPOS_ROOT)
    parser.add_argument("--output", default=DEFAULT_OUTPUT)
    parser.add_argument("--generated-on", default=DEFAULT_GENERATED_ON)
    parser.add_argument("--corpus-priority", default="bench/recall_loss/corpus-priority-census-2026-06-28.v1.json")
    for language, path in DEFAULT_REPORTS.items():
        parser.add_argument(f"--{language}-report", default=path)
    return parser.parse_args()


def load_json(path: str | Path) -> dict[str, Any]:
    return json.loads(Path(path).read_text())


def manifest_repos(manifest: Path) -> dict[str, list[str]]:
    data = load_json(manifest)
    by_language: dict[str, list[str]] = defaultdict(list)
    for repo in data.get("repositories", []):
        by_language[repo["primary_language"]].append(repo["id"])
    return dict(by_language)


def report_arg(args: argparse.Namespace, language: str) -> str:
    return getattr(args, f"{language.replace('-', '_')}_report")


def rows_from_operations(language: str, report_path: str) -> tuple[list[BoundaryRow], dict[str, Any]]:
    report = load_json(report_path)
    rows: list[BoundaryRow] = []
    for row in report.get("operations", []):
        rows.append(
            BoundaryRow(
                language=language,
                surface=row["surface"],
                operation=row["operation"],
                boundary=row["boundary"],
                capability=row.get("capability", capability_from_boundary(row["boundary"])),
                status=row["status"],
                occurrences=int(row["occurrences"]),
                repos=int(row.get("repos", 0)),
                note=row.get("note", ""),
                source=report_path,
                top_repos=tuple(
                    (item["repo"], int(item["occurrences"]))
                    for item in row.get("top_repos", [])[:8]
                ),
            )
        )
    return rows, report


def rows_from_python(report_path: str) -> tuple[list[BoundaryRow], dict[str, Any]]:
    report = load_json(report_path)
    rows: list[BoundaryRow] = []
    for row in report.get("calls", []):
        module = row["module"]
        function = row["function"]
        boundary = row["boundary"]
        rows.append(
            BoundaryRow(
                language="python",
                surface=f"{module}.{function}",
                operation=function,
                boundary=boundary,
                capability=capability_from_boundary(boundary),
                status=row["status"],
                occurrences=int(row["occurrences"]),
                repos=int(row.get("repos", 0)),
                note=row.get("note", ""),
                source=report_path,
                top_repos=tuple(
                    (item["repo"], int(item["occurrences"]))
                    for item in row.get("top_repos", [])[:8]
                ),
            )
        )
    return rows, report


def rows_from_methods(language: str, report_path: str) -> tuple[list[BoundaryRow], dict[str, Any]]:
    report = load_json(report_path)
    rows: list[BoundaryRow] = []
    for row in report.get("methods", []):
        namespace = row["namespace"]
        method = row["method"]
        boundary = row["boundary"]
        rows.append(
            BoundaryRow(
                language=language,
                surface=f"{namespace}.{method}",
                operation=method,
                boundary=boundary,
                capability=capability_from_boundary(boundary),
                status=row["status"],
                occurrences=int(row["occurrences"]),
                repos=int(row.get("repos", 0)),
                note=row.get("note", ""),
                source=report_path,
            )
        )
    return rows, report


def ruby_rows(repos: list[str], repos_root: Path) -> tuple[list[BoundaryRow], dict[str, Any]]:
    counter: Counter[tuple[str, str, str, str, str]] = Counter()
    repo_counter: dict[tuple[str, str, str, str, str], Counter[str]] = defaultdict(Counter)
    scanned_files = 0
    for repo in repos:
        for path in corpus_files(repos_root / repo, {".rb"}):
            scanned_files += 1
            text = read_masked(path, language="ruby")
            for match in RUBY_METHOD_RE.finditer(text):
                method = match.group("method")
                boundary, capability, note = RUBY_METHOD_BOUNDARIES[method]
                key = (f"ruby.enumerable.{method}", method, boundary, capability, note)
                counter[key] += 1
                repo_counter[key][repo] += 1
            for name, boundary, capability, note, regex in [
                ("raise", "exception-channel", "exception-channel", "raise exception", RUBY_RAISE_RE),
                ("rescue", "exception-channel", "exception-channel", "rescue exception channel", RUBY_RESCUE_RE),
                (
                    "Thread/Fiber",
                    "scheduling-boundary",
                    "scheduling-boundary",
                    "Ruby thread/fiber scheduling boundary",
                    RUBY_THREAD_RE,
                ),
            ]:
                for _ in regex.finditer(text):
                    key = (f"ruby.runtime.{name}", name, boundary, capability, note)
                    counter[key] += 1
                    repo_counter[key][repo] += 1
    rows = [
        BoundaryRow(
            language="ruby",
            surface=surface,
            operation=operation,
            boundary=boundary,
            capability=capability,
            status="unsupported",
            occurrences=count,
            repos=len(repo_counter[key]),
            note=note,
            source="scripts/cross-language-boundary-census.py:ruby_scan",
            top_repos=tuple(repo_counter[key].most_common(8)),
        )
        for key, count in sorted(counter.items(), key=lambda item: (-item[1], item[0]))
        for surface, operation, boundary, capability, note in [key]
    ]
    return rows, {"scanned_ruby_repos": len(repos), "scanned_ruby_files": scanned_files}


def c_rows(repos: list[str], repos_root: Path) -> tuple[list[BoundaryRow], dict[str, Any]]:
    counter: Counter[tuple[str, str, str, str, str]] = Counter()
    repo_counter: dict[tuple[str, str, str, str, str], Counter[str]] = defaultdict(Counter)
    scanned_files = 0
    for repo in repos:
        for path in corpus_files(repos_root / repo, {".c", ".h"}):
            scanned_files += 1
            text = read_masked(path, language="c")
            for surface, operation, boundary, capability, note, regex in C_PATTERNS:
                matches = len(regex.findall(text))
                if matches == 0:
                    continue
                key = (surface, operation, boundary, capability, note)
                counter[key] += matches
                repo_counter[key][repo] += matches
    rows = [
        BoundaryRow(
            language="c",
            surface=surface,
            operation=operation,
            boundary=boundary,
            capability=capability,
            status="unsupported",
            occurrences=count,
            repos=len(repo_counter[key]),
            note=note,
            source="scripts/cross-language-boundary-census.py:c_scan",
            top_repos=tuple(repo_counter[key].most_common(8)),
        )
        for key, count in sorted(counter.items(), key=lambda item: (-item[1], item[0]))
        for surface, operation, boundary, capability, note in [key]
    ]
    return rows, {"scanned_c_repos": len(repos), "scanned_c_files": scanned_files}


def corpus_files(root: Path, suffixes: set[str]) -> list[Path]:
    if not root.exists():
        return []
    files: list[Path] = []
    for path in root.rglob("*"):
        if not path.is_file() or path.suffix not in suffixes:
            continue
        if any(part in SKIP_DIRS for part in path.parts):
            continue
        files.append(path)
    return files


def read_masked(path: Path, language: str) -> str:
    try:
        text = path.read_text(errors="ignore")
    except OSError:
        return ""
    if language == "ruby":
        return mask_ruby_comments_and_strings(text)
    return mask_c_comments_and_strings(text)


def mask_ruby_comments_and_strings(text: str) -> str:
    chars = list(text)
    i = 0
    while i < len(chars):
        ch = chars[i]
        if ch == "#":
            i = mask_until_newline(chars, i)
            continue
        if ch in {"'", '"'}:
            i = mask_quoted(chars, i, ch)
            continue
        i += 1
    return "".join(chars)


def mask_c_comments_and_strings(text: str) -> str:
    chars = list(text)
    i = 0
    while i < len(chars):
        ch = chars[i]
        nxt = chars[i + 1] if i + 1 < len(chars) else ""
        if ch == "/" and nxt == "/":
            i = mask_until_newline(chars, i)
            continue
        if ch == "/" and nxt == "*":
            i = mask_until_block_comment_end(chars, i)
            continue
        if ch in {"'", '"'}:
            i = mask_quoted(chars, i, ch)
            continue
        i += 1
    return "".join(chars)


def mask_until_newline(chars: list[str], start: int) -> int:
    i = start
    while i < len(chars) and chars[i] != "\n":
        chars[i] = " "
        i += 1
    return i


def mask_until_block_comment_end(chars: list[str], start: int) -> int:
    i = start
    while i < len(chars):
        if chars[i] != "\n":
            chars[i] = " "
        if i + 1 < len(chars) and chars[i] == "*" and chars[i + 1] == "/":
            chars[i + 1] = " "
            return i + 2
        i += 1
    return i


def mask_quoted(chars: list[str], start: int, quote: str) -> int:
    i = start
    chars[i] = " "
    i += 1
    escaped = False
    while i < len(chars):
        ch = chars[i]
        if ch != "\n":
            chars[i] = " "
        if escaped:
            escaped = False
        elif ch == "\\":
            escaped = True
        elif ch == quote:
            return i + 1
        i += 1
    return i


def capability_from_boundary(boundary: str) -> str:
    if "callback" in boundary or boundary in {"hof-lambda", "terminal-hof", "predicate-iterator"}:
        return "callback-demand-effect"
    if "mutation" in boundary or "effect" in boundary:
        return "effect-preserving-contracts"
    if "option" in boundary or "result" in boundary or "default" in boundary:
        return "option-result-channel"
    if "promise" in boundary or "async" in boundary or "await" in boundary:
        return "scheduling-boundary"
    if "iterator" in boundary or "materializer" in boundary or "view" in boundary:
        return "iterator-hof-materialization"
    if "proof" in boundary or "receiver" in boundary or "property" in boundary:
        return "receiver-domain-evidence"
    if "ordering" in boundary:
        return "ordering-semantics"
    if "allocation" in boundary or "lifetime" in boundary:
        return "allocation-lifetime"
    return "boundary-classification"


def obligation_family(boundary: str, capability: str, language: str, surface: str) -> str:
    text = f"{boundary} {capability} {surface}".lower()
    if "executor" in text:
        return "executor-callback"
    if "rejection" in text or "promise-error-channel" in text:
        return "rejection-channel"
    if "exception" in text or "nonlocal_jump" in text or "raise" in text or "rescue" in text:
        return "exception-channel"
    if "error-channel" in text:
        return "success-error-result-channel"
    if (
        "await" in text
        or "async" in text
        or "promise-combinator" in text
        or "scheduling" in text
        or "thread" in text
        or "pthread" in text
    ):
        return "scheduling-boundary"
    if "mutation" in text or "receiver-mutating" in text:
        return "receiver-mutation"
    if "reduction-callback" in text:
        return "reduction-callback"
    if "effect-callback" in text:
        return "effect-only-callback"
    if "callback" in text or "hof-lambda" in text or "terminal-hof" in text:
        if language in {"rust", "python"} or "lazy" in text or "iterator" in text:
            return "lazy-callback"
        if language == "js-ts" and ("array-hof" in text or "array-bool" in text):
            return "eager-callback"
        return "synchronous-callback"
    if "option" in text or "result" in text or "default" in text:
        return "success-error-result-channel"
    if (
        "iterator" in text
        or "materializer" in text
        or "view" in text
        or "lifecycle" in text
        or "allocation" in text
        or "copy-result" in text
        or "factory" in text
    ):
        return "lifecycle-materialization-boundary"
    if "proof" in text or "receiver" in text or "property" in text or "selector" in text:
        return "ambiguous-selector-boundary"
    return "other-boundary"


def secondary_families(boundary: str, primary: str) -> list[str]:
    secondary: list[str] = []
    if "mutation-callback" in boundary and primary != "effect-only-callback":
        secondary.append("effect-only-callback")
    if "ordering-callback" in boundary and primary != "synchronous-callback":
        secondary.append("synchronous-callback")
    if "option-producing-hof" in boundary and primary != "success-error-result-channel":
        secondary.append("success-error-result-channel")
    if "promise" in boundary and primary != "scheduling-boundary":
        secondary.append("scheduling-boundary")
    return secondary


def actionability(status: str, family: str, boundary: str) -> str:
    if status == "supported":
        return "existing-supported"
    if status == "supported-partial":
        return "actionable-with-existing-contract"
    if "unknown" in boundary:
        return "requires-classification"
    if family in {
        "scheduling-boundary",
        "exception-channel",
        "rejection-channel",
        "executor-callback",
    }:
        return "reporting-only-until-kernel-obligations"
    return "closed-boundary"


def count_rows(rows: list[BoundaryRow], key_fn) -> list[dict[str, Any]]:
    counts: Counter[Any] = Counter()
    repos: dict[Any, set[str]] = defaultdict(set)
    for row in rows:
        key = key_fn(row)
        counts[key] += row.occurrences
        if row.repos:
            repos[key].add(row.language)
    result = []
    for key, occurrences in sorted(counts.items(), key=lambda item: (-item[1], str(item[0]))):
        item: dict[str, Any]
        if isinstance(key, tuple):
            item = {f"key_{index}": value for index, value in enumerate(key)}
        else:
            item = {"key": key}
        item["occurrences"] = occurrences
        result.append(item)
    return result


def language_summaries(rows: list[BoundaryRow], reports: dict[str, dict[str, Any]], scan_meta: dict[str, Any]) -> list[dict[str, Any]]:
    by_language: dict[str, list[BoundaryRow]] = defaultdict(list)
    for row in rows:
        by_language[row.language].append(row)
    summaries: list[dict[str, Any]] = []
    for language in sorted(by_language):
        language_rows = by_language[language]
        status_counts = Counter()
        family_counts = Counter()
        for row in language_rows:
            status_counts[row.status] += row.occurrences
            family_counts[obligation_family(row.boundary, row.capability, row.language, row.surface)] += row.occurrences
        summary: dict[str, Any] = {
            "language": language,
            "occurrences": sum(row.occurrences for row in language_rows),
            "surfaces": len(language_rows),
            "status_counts": dict(sorted(status_counts.items())),
            "obligation_family_counts": dict(sorted(family_counts.items())),
        }
        report = reports.get(language)
        if report is not None:
            summary["source_report_kind"] = report.get("report_kind")
            summary["source_totals"] = report.get("totals")
        if language == "ruby":
            summary["scan_meta"] = {
                "scanned_repos": scan_meta.get("scanned_ruby_repos", 0),
                "scanned_files": scan_meta.get("scanned_ruby_files", 0),
            }
        if language == "c":
            summary["scan_meta"] = {
                "scanned_repos": scan_meta.get("scanned_c_repos", 0),
                "scanned_files": scan_meta.get("scanned_c_files", 0),
            }
        summaries.append(summary)
    return summaries


def taxonomy(rows: list[BoundaryRow]) -> list[dict[str, Any]]:
    descriptions = {
        "synchronous-callback": "Callback/block is invoked in the surrounding synchronous evaluation but still needs identity, cardinality, and effect obligations.",
        "lazy-callback": "Callback is delayed until iterator/view consumption, so callback errors/effects cannot be treated as eager.",
        "eager-callback": "Callback is invoked eagerly by a collection/HOF surface and may expose callback errors/effects immediately.",
        "effect-only-callback": "Callback result is ignored or the surrounding API is effect-oriented; exact value admission must not consume the callback result.",
        "reduction-callback": "Callback participates in accumulator/result state and needs accumulator demand/effect semantics.",
        "executor-callback": "Callback/executor runs as part of constructing a protocol object such as a Promise.",
        "success-error-result-channel": "Surface carries success/empty/error/result channels that cannot be collapsed into a plain value.",
        "exception-channel": "Surface can throw, rescue, or non-locally transfer control.",
        "rejection-channel": "Surface carries Promise/Future-style rejection or error continuation behavior.",
        "scheduling-boundary": "Surface crosses a task/thread/microtask/async protocol boundary.",
        "cancellation-early-exit-boundary": "Surface may cancel, stop early, or short-circuit control; currently represented by adjacent callback/channel buckets when observed.",
        "lifecycle-materialization-boundary": "Surface creates, views, materializes, consumes, or aliases iterator/stream/collection lifecycle state.",
        "receiver-mutation": "Surface mutates a receiver/place or exposes mutation-sensitive effects.",
        "ambiguous-selector-boundary": "Selector/property spelling is visible but receiver/symbol/domain proof is missing.",
        "other-boundary": "Classified but outside the initial #594 obligation families.",
    }
    counts = Counter(obligation_family(row.boundary, row.capability, row.language, row.surface) for row in rows for _ in range(row.occurrences))
    return [
        {
            "family": family,
            "description": description,
            "occurrences": counts.get(family, 0),
            "present": counts.get(family, 0) > 0,
        }
        for family, description in descriptions.items()
    ]


def ranked_matrix(rows: list[BoundaryRow]) -> list[dict[str, Any]]:
    grouped: dict[tuple[str, str], list[BoundaryRow]] = defaultdict(list)
    for row in rows:
        family = obligation_family(row.boundary, row.capability, row.language, row.surface)
        grouped[(family, row.language)].append(row)
    matrix: list[dict[str, Any]] = []
    for (family, language), group in grouped.items():
        occurrences = sum(row.occurrences for row in group)
        top = sorted(group, key=lambda row: (-row.occurrences, row.surface, row.operation))[:5]
        matrix.append(
            {
                "obligation_family": family,
                "language": language,
                "occurrences": occurrences,
                "surfaces": len(group),
                "top_surfaces": [
                    {
                        "surface": row.surface,
                        "operation": row.operation,
                        "boundary": row.boundary,
                        "occurrences": row.occurrences,
                        "repos": row.repos,
                        "status": row.status,
                    }
                    for row in top
                ],
            }
        )
    return sorted(matrix, key=lambda row: (-row["occurrences"], row["obligation_family"], row["language"]))


def top_surfaces(rows: list[BoundaryRow], limit: int = 25) -> list[dict[str, Any]]:
    return [row.as_json() for row in sorted(rows, key=lambda row: (-row.occurrences, row.language, row.surface, row.operation))[:limit]]


def top_recommendations(rows: list[BoundaryRow], corpus_priority: dict[str, Any] | None) -> list[dict[str, Any]]:
    family_counts = Counter(obligation_family(row.boundary, row.capability, row.language, row.surface) for row in rows for _ in range(row.occurrences))
    return [
        {
            "rank": 1,
            "work": "Design minimal scheduling/channel/callback obligation vocabulary before producer work",
            "why": "The largest observed boundary is JS/TS Promise async/scheduling at 29,094 occurrences, but it is explicitly unsafe to admit without shared obligations.",
            "issue": 596,
            "expected_semantic_admission_delta": 0,
        },
        {
            "rank": 2,
            "work": "Move broad recall-loss buckets into structured scheduling/channel/callback sub-buckets",
            "why": "The full corpus priority census has mutation/effect, unsupported-runtime, and HOF demand/effect buckets that should become measurable before any widening.",
            "issue": 597,
            "expected_semantic_admission_delta": 0,
        },
        {
            "rank": 3,
            "work": "Build cross-language hard negatives for the top observed obligation families",
            "why": "Receiver mutation, callback timing, scheduling, and error/rejection channels are exactly the false-merge risks exposed by the census.",
            "issue": 598,
            "expected_semantic_admission_delta": 0,
        },
        {
            "rank": 4,
            "work": "Start producer work with callback demand/effect reporting, not broad async admission",
            "why": (
                f"Callback-shaped obligations total {sum(count for family, count in family_counts.items() if 'callback' in family):,} "
                "occurrences across several languages and reuse existing HOF demand/effect concepts."
            ),
            "issue": 599,
            "expected_semantic_admission_delta": 0,
        },
        {
            "rank": 5,
            "work": "Keep Promise/Future/async/channel convergence closed until channel and scheduling evidence exists",
            "why": "High frequency is pricing evidence only; scheduling, exception, rejection, aggregate result, and callback-effect obligations are missing.",
            "issue": 600,
            "expected_semantic_admission_delta": 0,
        },
    ]


def main() -> int:
    args = parse_args()
    repos_by_language = manifest_repos(Path(args.manifest))
    rows: list[BoundaryRow] = []
    source_reports: dict[str, dict[str, Any]] = {}

    for language in ["js-ts", "rust", "swift"]:
        language_rows, report = rows_from_operations(language, report_arg(args, language))
        rows.extend(language_rows)
        source_reports[language] = report
    python_rows, python_report = rows_from_python(report_arg(args, "python"))
    rows.extend(python_rows)
    source_reports["python"] = python_report
    for language in ["go", "java"]:
        language_rows, report = rows_from_methods(language, report_arg(args, language))
        rows.extend(language_rows)
        source_reports[language] = report

    repos_root = Path(args.repos_root)
    ruby_scan_rows, ruby_meta = ruby_rows(repos_by_language.get("Ruby", []), repos_root)
    c_scan_rows, c_meta = c_rows(repos_by_language.get("C", []), repos_root)
    rows.extend(ruby_scan_rows)
    rows.extend(c_scan_rows)
    scan_meta = {**ruby_meta, **c_meta}

    corpus_priority = load_json(args.corpus_priority) if Path(args.corpus_priority).exists() else None
    report = {
        "report_kind": "cross-language-boundary-census",
        "schema_version": 1,
        "issue": 595,
        "parent_issue": 594,
        "generated_on": args.generated_on,
        "manifest": args.manifest,
        "repos_root": args.repos_root,
        "source_policy": "source prevalence is pricing evidence only; it never proves semantic admission",
        "source_reports": [
            {"language": language, "path": report_arg(args, language)}
            for language in ["go", "java", "js-ts", "python", "rust", "swift"]
        ]
        + [
            {"language": "ruby", "path": "scripts/cross-language-boundary-census.py:ruby_scan"},
            {"language": "c", "path": "scripts/cross-language-boundary-census.py:c_scan"},
        ],
        "hard_gate": {
            "semantic_admission_delta": 0,
            "false_merges": 0,
            "canon_preservation_violations": 0,
            "evidence": "reporting/source-prevalence-only; no semantic code path is changed by this census",
        },
        "corpus_recall_loss_baseline": corpus_priority.get("summary") if corpus_priority else None,
        "corpus_recall_loss_top_reasons": corpus_priority.get("recall_loss_top_reasons") if corpus_priority else [],
        "language_summaries": language_summaries(rows, source_reports, scan_meta),
        "obligation_taxonomy": taxonomy(rows),
        "obligation_family_counts": [
            {"family": row["key"], "occurrences": row["occurrences"]}
            for row in count_rows(rows, lambda item: obligation_family(item.boundary, item.capability, item.language, item.surface))
        ],
        "language_obligation_matrix": ranked_matrix(rows),
        "status_counts": [
            {"status": row["key"], "occurrences": row["occurrences"]}
            for row in count_rows(rows, lambda item: item.status)
        ],
        "top_surfaces": top_surfaces(rows),
        "recommendations": top_recommendations(rows, corpus_priority),
        "repro_commands": [
            "python3 scripts/cross-language-boundary-census.py --output target/cross-language-boundary-census-594.v1.json",
            "cp target/cross-language-boundary-census-594.v1.json bench/recall_loss/cross-language-boundary-census-594-2026-06-28.v1.json",
        ],
    }

    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
