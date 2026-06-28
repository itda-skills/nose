#!/usr/bin/env python3
"""Audit Java Arrays/Collections stdlib prevalence in the pinned corpus.

This is a lexical pricing report, not semantic proof. It answers which
`java.util.Arrays` and `java.util.Collections` methods are still worth modeling
after the currently admitted collection/map factory and Arrays.stream rows.
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
DEFAULT_OUTPUT = "target/java-arrays-collections-audit.v1.json"

SKIP_DIRS = {
    ".git",
    ".gradle",
    "build",
    "dist",
    "node_modules",
    "out",
    "target",
}

CALL_RE = re.compile(r"\b(Arrays|Collections)\s*\.\s*([A-Za-z_][A-Za-z0-9_]*)\s*\(")
STATIC_IMPORT_RE = re.compile(
    r"^\s*import\s+static\s+java\.util\.(Arrays|Collections)\.([A-Za-z_][A-Za-z0-9_]*|\*)\s*;",
    re.MULTILINE,
)


SUPPORTED: dict[tuple[str, str], tuple[str, str]] = {
    ("Arrays", "asList"): (
        "supported-partial",
        "collection factory; single-argument calls stay closed unless the argument has array-domain proof",
    ),
    ("Arrays", "stream"): (
        "supported",
        "static collection adapter with java.util.Arrays import/shadow proof",
    ),
    ("Collections", "emptyList"): ("supported", "empty collection factory"),
    ("Collections", "emptySet"): ("supported", "empty collection factory"),
    ("Collections", "singleton"): ("supported", "single-element collection factory"),
    ("Collections", "singletonList"): ("supported", "single-element collection factory"),
    ("Collections", "emptyMap"): ("supported", "empty map factory"),
    ("Collections", "singletonMap"): ("supported", "single-entry map factory"),
}

BOUNDARY_BY_METHOD: dict[str, tuple[str, str]] = {
    "addAll": ("mutation-effect", "mutates the target collection"),
    "binarySearch": ("ordering-precondition", "requires sorted-order preconditions"),
    "checkedCollection": ("wrapper-aliasing", "runtime type-checking wrapper"),
    "checkedList": ("wrapper-aliasing", "runtime type-checking wrapper"),
    "checkedMap": ("wrapper-aliasing", "runtime type-checking wrapper"),
    "checkedNavigableMap": ("wrapper-aliasing", "runtime type-checking wrapper"),
    "checkedNavigableSet": ("wrapper-aliasing", "runtime type-checking wrapper"),
    "checkedQueue": ("wrapper-aliasing", "runtime type-checking wrapper"),
    "checkedSet": ("wrapper-aliasing", "runtime type-checking wrapper"),
    "checkedSortedMap": ("wrapper-aliasing", "runtime type-checking wrapper"),
    "checkedSortedSet": ("wrapper-aliasing", "runtime type-checking wrapper"),
    "compare": ("lexicographic-array-compare", "array comparison needs ordering laws"),
    "compareUnsigned": (
        "lexicographic-array-compare",
        "array comparison needs ordering and unsigned lane laws",
    ),
    "copy": ("mutation-effect", "mutates destination collection"),
    "copyOf": ("copy-result-domain", "value-preserving copy result needs array/list domain proof"),
    "copyOfRange": ("copy-result-domain", "range copy changes shape and bounds obligations"),
    "deepEquals": ("deep-equality", "nested array equality needs value law and null handling"),
    "deepHashCode": ("representation-hash", "hash contract is not value equality"),
    "deepToString": ("representation-string", "debug representation is not value equality"),
    "disjoint": ("membership-reduction", "cross-collection predicate needs element-domain proof"),
    "emptyIterator": ("iterator-identity", "iterator sentinel is not a collection value"),
    "emptyListIterator": ("iterator-identity", "iterator sentinel is not a collection value"),
    "emptyNavigableMap": ("sorted-map-domain", "sorted/navigable map domain not modeled"),
    "emptyNavigableSet": ("sorted-set-domain", "sorted/navigable set domain not modeled"),
    "emptySortedMap": ("sorted-map-domain", "sorted map domain not modeled"),
    "emptySortedSet": ("sorted-set-domain", "sorted set domain not modeled"),
    "enumeration": ("legacy-iterator", "Enumeration adapter not modeled"),
    "equals": ("array-equality", "array content equality needs a dedicated value law"),
    "fill": ("mutation-effect", "mutates the target array/list"),
    "frequency": ("count-reduction", "element multiplicity reduction not modeled"),
    "hashCode": ("representation-hash", "hash contract is not value equality"),
    "list": ("legacy-iterator", "Enumeration-to-list adapter not modeled"),
    "lastIndexOfSubList": ("sequence-search", "subsequence search not modeled"),
    "max": ("ordering-reduction", "requires comparator/order proof"),
    "min": ("ordering-reduction", "requires comparator/order proof"),
    "mismatch": ("array-difference-search", "array mismatch search not modeled"),
    "nCopies": ("repeated-collection", "multiplicity-producing collection factory not modeled"),
    "newSetFromMap": ("wrapper-aliasing", "set backed by mutable map aliases input"),
    "parallelPrefix": ("mutation-effect", "mutates array with callback/operator effects"),
    "parallelSetAll": ("mutation-effect", "mutates array with callback effects"),
    "parallelSort": ("mutation-effect", "in-place ordering mutation"),
    "replaceAll": ("mutation-effect", "mutates list with callback effects"),
    "reverse": ("mutation-effect", "in-place order mutation"),
    "reverseOrder": ("comparator-factory", "comparator identity not modeled"),
    "rotate": ("mutation-effect", "in-place order mutation"),
    "setAll": ("mutation-effect", "mutates array with callback effects"),
    "shuffle": ("random-effect", "randomized in-place order mutation"),
    "sort": ("mutation-effect", "in-place ordering mutation"),
    "spliterator": ("iterator-identity", "spliterator adapter not modeled"),
    "swap": ("mutation-effect", "in-place order mutation"),
    "synchronizedCollection": ("wrapper-aliasing", "synchronization wrapper"),
    "synchronizedList": ("wrapper-aliasing", "synchronization wrapper"),
    "synchronizedMap": ("wrapper-aliasing", "synchronization wrapper"),
    "synchronizedNavigableMap": ("wrapper-aliasing", "synchronization wrapper"),
    "synchronizedNavigableSet": ("wrapper-aliasing", "synchronization wrapper"),
    "synchronizedSet": ("wrapper-aliasing", "synchronization wrapper"),
    "toString": ("representation-string", "debug representation is not value equality"),
    "unmodifiableCollection": ("wrapper-aliasing", "read-only wrapper aliases input"),
    "unmodifiableList": ("wrapper-aliasing", "read-only wrapper aliases input"),
    "unmodifiableMap": ("wrapper-aliasing", "read-only wrapper aliases input"),
    "unmodifiableNavigableMap": ("wrapper-aliasing", "read-only wrapper aliases input"),
    "unmodifiableNavigableSet": ("wrapper-aliasing", "read-only wrapper aliases input"),
    "unmodifiableSet": ("wrapper-aliasing", "read-only wrapper aliases input"),
    "unmodifiableSortedMap": ("wrapper-aliasing", "read-only wrapper aliases input"),
    "unmodifiableSortedSet": ("wrapper-aliasing", "read-only wrapper aliases input"),
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", default=DEFAULT_MANIFEST)
    parser.add_argument("--repos-root", default=DEFAULT_REPOS_ROOT)
    parser.add_argument("--output", default=DEFAULT_OUTPUT)
    return parser.parse_args()


def java_repos(manifest: Path) -> list[str]:
    data = json.loads(manifest.read_text())
    return [
        repo["id"]
        for repo in data.get("repositories", [])
        if repo.get("primary_language") == "Java"
    ]


def java_files(root: Path) -> list[Path]:
    if not root.exists():
        return []
    files: list[Path] = []
    for path in root.rglob("*.java"):
        if any(part in SKIP_DIRS for part in path.parts):
            continue
        files.append(path)
    return files


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
    static_import_counts: Counter[tuple[str, str]] = Counter()
    scanned_files = 0

    for repo in java_repos(Path(args.manifest)):
        for path in java_files(repos_root / repo):
            scanned_files += 1
            try:
                text = path.read_text(errors="ignore")
            except OSError:
                continue
            for namespace, method in CALL_RE.findall(text):
                key = (namespace, method)
                method_counts[key] += 1
                method_repos[key].add(repo)
            for namespace, method in STATIC_IMPORT_RE.findall(text):
                static_import_counts[(namespace, method)] += 1

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
        "report_kind": "java-arrays-collections-audit",
        "manifest": args.manifest,
        "repos_root": args.repos_root,
        "scanned_java_repos": len(java_repos(Path(args.manifest))),
        "scanned_java_files": scanned_files,
        "totals": {
            "occurrences": sum(method_counts.values()),
            "supported_or_partial_occurrences": sum(
                row["occurrences"]
                for row in methods
                if row["status"] in {"supported", "supported-partial"}
            ),
            "unsupported_occurrences": sum(
                row["occurrences"] for row in methods if row["status"] == "unsupported"
            ),
            "unknown_boundary_occurrences": boundary_counts.get("unknown-boundary", 0),
        },
        "status_counts": dict(sorted(status_counts.items())),
        "boundary_counts": dict(sorted(boundary_counts.items())),
        "static_imports": [
            {
                "namespace": namespace,
                "method": method,
                "occurrences": count,
            }
            for (namespace, method), count in sorted(
                static_import_counts.items(), key=lambda item: (-item[1], item[0])
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
