#!/usr/bin/env python3
"""Audit Go sort/slices/maps stdlib prevalence in the pinned corpus.

This scanner resolves simple Go import aliases before counting package calls.
It is still a lexical pricing report, not semantic proof: local shadowing,
build tags, and generated files are not full Go type-checking.
"""

from __future__ import annotations

import argparse
import json
import re
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any


DEFAULT_MANIFEST = "bench/goldens/corpus.json"
DEFAULT_REPOS_ROOT = "bench/repos"
DEFAULT_OUTPUT = "target/go-stdlib-collections-audit.v1.json"

SKIP_DIRS = {
    ".git",
    "dist",
    "node_modules",
    "target",
    "vendor",
}

TARGET_IMPORTS = {"sort", "slices", "maps"}
CALL_RE = re.compile(r"\b([A-Za-z_][A-Za-z0-9_]*)\s*\.\s*([A-Za-z_][A-Za-z0-9_]*)\s*\(")
SINGLE_IMPORT_RE = re.compile(
    r'^\s*import\s+(?:(?P<alias>[A-Za-z_][A-Za-z0-9_]*|\.)\s+)?"(?P<path>sort|slices|maps)"',
    re.MULTILINE,
)
IMPORT_BLOCK_RE = re.compile(r"^\s*import\s*\((?P<body>.*?)^\s*\)", re.MULTILINE | re.DOTALL)
BLOCK_IMPORT_RE = re.compile(
    r'^\s*(?:(?P<alias>[A-Za-z_][A-Za-z0-9_]*|\.)\s+)?"(?P<path>sort|slices|maps)"',
    re.MULTILINE,
)


SUPPORTED: dict[tuple[str, str], tuple[str, str]] = {
    ("slices", "Contains"): (
        "supported",
        "namespace-call collection membership with imported package proof",
    )
}

BOUNDARY_BY_METHOD: dict[str, tuple[str, str]] = {
    "All": ("iterator-view", "iterator sequence view is not modeled"),
    "BinarySearch": ("ordering-precondition", "requires sorted-order preconditions"),
    "BinarySearchFunc": (
        "ordering-callback",
        "requires sorted-order preconditions and comparator callback proof",
    ),
    "Chunk": ("iterator-view", "chunk iterator is not modeled"),
    "Clone": ("copy-result-domain", "copy result and alias/lifetime proof not modeled"),
    "Clip": ("allocation-lifetime", "capacity/lifetime effect not modeled"),
    "Collect": ("iterator-collection-factory", "iterator collection factory not modeled"),
    "Compact": ("mutation-effect", "in-place duplicate compaction"),
    "CompactFunc": ("mutation-callback", "in-place compaction with equality callback"),
    "Compare": ("ordering-reduction", "lexicographic ordering comparison"),
    "CompareFunc": ("ordering-callback", "lexicographic comparison with callback"),
    "Concat": ("copy-result-domain", "concatenating slice factory not modeled"),
    "ContainsFunc": ("membership-callback", "membership predicate with callback"),
    "Copy": ("mutation-effect", "mutates destination map"),
    "Delete": ("mutation-effect", "slice deletion changes shape"),
    "DeleteFunc": ("mutation-callback", "map deletion with predicate callback"),
    "Equal": ("collection-equality", "collection/map equality law not modeled"),
    "EqualFunc": ("equality-callback", "equality with callback not modeled"),
    "Float64s": ("mutation-effect", "in-place ordering mutation"),
    "Float64sAreSorted": ("ordering-predicate", "sortedness predicate not modeled"),
    "Grow": ("allocation-lifetime", "capacity/lifetime effect not modeled"),
    "Index": ("membership-index", "index search result is not a boolean membership proof"),
    "IndexFunc": ("membership-callback", "index search with callback"),
    "Insert": ("mutation-effect", "insertion changes slice/map shape"),
    "Ints": ("mutation-effect", "in-place ordering mutation"),
    "IntsAreSorted": ("ordering-predicate", "sortedness predicate not modeled"),
    "IsSorted": ("ordering-predicate", "sortedness predicate not modeled"),
    "IsSortedFunc": ("ordering-callback", "sortedness predicate with callback"),
    "Keys": ("iterator-view", "map key iterator/view not modeled"),
    "Max": ("ordering-reduction", "ordering reduction not modeled"),
    "MaxFunc": ("ordering-callback", "ordering reduction with callback"),
    "Min": ("ordering-reduction", "ordering reduction not modeled"),
    "MinFunc": ("ordering-callback", "ordering reduction with callback"),
    "Repeat": ("copy-result-domain", "repeated slice construction not modeled"),
    "Replace": ("mutation-effect", "replacement changes slice shape"),
    "Reverse": ("mutation-effect", "in-place order mutation"),
    "Search": ("ordering-precondition", "binary search requires monotonic predicate"),
    "SearchFloat64s": ("ordering-precondition", "requires sorted-order preconditions"),
    "SearchInts": ("ordering-precondition", "requires sorted-order preconditions"),
    "SearchStrings": ("ordering-precondition", "requires sorted-order preconditions"),
    "Slice": ("mutation-callback", "in-place sort with less callback"),
    "SliceIsSorted": ("ordering-callback", "sortedness predicate with callback"),
    "SliceStable": ("mutation-callback", "stable in-place sort with less callback"),
    "Sort": ("mutation-effect", "in-place ordering mutation"),
    "SortFunc": ("mutation-callback", "in-place sort with comparator callback"),
    "SortStableFunc": ("mutation-callback", "stable in-place sort with comparator callback"),
    "Sorted": ("iterator-collection-factory", "iterator-to-sorted-slice factory not modeled"),
    "SortedFunc": ("iterator-collection-factory", "iterator sort with comparator callback"),
    "SortedStableFunc": (
        "iterator-collection-factory",
        "stable iterator sort with comparator callback",
    ),
    "Stable": ("mutation-effect", "stable in-place ordering mutation"),
    "Strings": ("mutation-effect", "in-place ordering mutation"),
    "StringsAreSorted": ("ordering-predicate", "sortedness predicate not modeled"),
    "StringSlice": ("sort-interface-adapter", "sort.Interface adapter type conversion not modeled"),
    "Values": ("iterator-view", "map value iterator/view not modeled"),
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", default=DEFAULT_MANIFEST)
    parser.add_argument("--repos-root", default=DEFAULT_REPOS_ROOT)
    parser.add_argument("--output", default=DEFAULT_OUTPUT)
    return parser.parse_args()


def go_repos(manifest: Path) -> list[str]:
    data = json.loads(manifest.read_text())
    return [
        repo["id"]
        for repo in data.get("repositories", [])
        if repo.get("primary_language") == "Go"
    ]


def go_files(root: Path) -> list[Path]:
    if not root.exists():
        return []
    files: list[Path] = []
    for path in root.rglob("*.go"):
        if any(part in SKIP_DIRS for part in path.parts):
            continue
        files.append(path)
    return files


def import_aliases(text: str) -> dict[str, str]:
    aliases: dict[str, str] = {}
    for match in SINGLE_IMPORT_RE.finditer(text):
        add_alias(aliases, match.group("path"), match.group("alias"))
    for block in IMPORT_BLOCK_RE.finditer(text):
        for match in BLOCK_IMPORT_RE.finditer(block.group("body")):
            add_alias(aliases, match.group("path"), match.group("alias"))
    return aliases


def add_alias(aliases: dict[str, str], path: str, alias: str | None) -> None:
    if alias == ".":
        return
    local = alias or path
    aliases[local] = path


def classify(namespace: str, method: str) -> tuple[str, str, str]:
    supported = SUPPORTED.get((namespace, method))
    if supported:
        status, note = supported
        return status, "admitted", note
    boundary, note = BOUNDARY_BY_METHOD.get(
        method,
        ("unknown-boundary", "observed but not classified by this audit"),
    )
    return "unsupported", boundary, note


def main() -> int:
    args = parse_args()
    repos_root = Path(args.repos_root)
    method_counts: Counter[tuple[str, str]] = Counter()
    method_repos: dict[tuple[str, str], set[str]] = defaultdict(set)
    alias_counts: Counter[tuple[str, str]] = Counter()
    scanned_files = 0

    for repo in go_repos(Path(args.manifest)):
        for path in go_files(repos_root / repo):
            scanned_files += 1
            try:
                text = path.read_text(errors="ignore")
            except OSError:
                continue
            aliases = import_aliases(text)
            for local, namespace in aliases.items():
                if local != namespace:
                    alias_counts[(namespace, local)] += 1
            for receiver, method in CALL_RE.findall(text):
                namespace = aliases.get(receiver)
                if namespace not in TARGET_IMPORTS:
                    continue
                key = (namespace, method)
                method_counts[key] += 1
                method_repos[key].add(repo)

    methods: list[dict[str, Any]] = []
    for (namespace, method), occurrences in sorted(
        method_counts.items(), key=lambda item: (-item[1], item[0][0], item[0][1])
    ):
        status, boundary, note = classify(namespace, method)
        methods.append(
            {
                "namespace": namespace,
                "method": method,
                "occurrences": occurrences,
                "repos": len(method_repos[(namespace, method)]),
                "status": status,
                "boundary": boundary,
                "note": note,
            }
        )

    status_counts = Counter(row["status"] for row in methods for _ in range(row["occurrences"]))
    boundary_counts = Counter(row["boundary"] for row in methods for _ in range(row["occurrences"]))
    report = {
        "report_kind": "go-stdlib-collections-audit",
        "manifest": args.manifest,
        "repos_root": args.repos_root,
        "scanned_go_repos": len(go_repos(Path(args.manifest))),
        "scanned_go_files": scanned_files,
        "totals": {
            "occurrences": sum(method_counts.values()),
            "supported_occurrences": sum(
                row["occurrences"] for row in methods if row["status"] == "supported"
            ),
            "unsupported_occurrences": sum(
                row["occurrences"] for row in methods if row["status"] == "unsupported"
            ),
            "unknown_boundary_occurrences": boundary_counts.get("unknown-boundary", 0),
        },
        "status_counts": dict(sorted(status_counts.items())),
        "boundary_counts": dict(sorted(boundary_counts.items())),
        "import_aliases": [
            {
                "namespace": namespace,
                "alias": alias,
                "files": count,
            }
            for (namespace, alias), count in sorted(
                alias_counts.items(), key=lambda item: (-item[1], item[0])
            )
        ],
        "methods": methods,
    }
    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=False) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
