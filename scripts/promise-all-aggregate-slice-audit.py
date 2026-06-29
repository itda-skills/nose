#!/usr/bin/env python3
"""Build the #602 Promise.all literal aggregate recovery artifact."""

from __future__ import annotations

import argparse
import importlib.util
import json
import re
import sys
from collections import Counter
from pathlib import Path
from typing import Any


DEFAULT_MANIFEST = "bench/goldens/corpus.json"
DEFAULT_REPOS_ROOT = "bench/repos"
DEFAULT_OUTPUT = "target/promise-all-literal-aggregate-recovery.v1.json"
DEFAULT_GENERATED_ON = "2026-06-29"

PATTERNS = {
    "promise_all": re.compile(r"\bPromise\s*\.\s*all\s*(?:<[^;\n(){}]*>)?\s*\("),
    "literal_array": re.compile(r"\bPromise\s*\.\s*all\s*(?:<[^;\n(){}]*>)?\s*\(\s*\["),
    "literal_array_with_direct_resolve_seed": re.compile(
        r"\bPromise\s*\.\s*all\s*(?:<[^;\n(){}]*>)?\s*\(\s*\[[^\]\n]{0,240}"
        r"\bPromise\s*\.\s*resolve\s*\(",
        re.S,
    ),
}


def load_boundary_audit_module() -> Any:
    path = Path(__file__).with_name("scheduling-lifecycle-boundary-audit.py")
    spec = importlib.util.spec_from_file_location("scheduling_lifecycle_boundary_audit", path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", default=DEFAULT_MANIFEST)
    parser.add_argument("--repos-root", default=DEFAULT_REPOS_ROOT)
    parser.add_argument("--recall-loss-report", required=True)
    parser.add_argument("--output", default=DEFAULT_OUTPUT)
    parser.add_argument("--generated-on", default=DEFAULT_GENERATED_ON)
    return parser.parse_args()


def count_corpus(args: argparse.Namespace, audit: Any) -> dict[str, Any]:
    repos = audit.load_repos(Path(args.manifest))
    counts = Counter()
    repo_counts: dict[str, Counter[str]] = {key: Counter() for key in PATTERNS}
    file_counts: dict[str, Counter[str]] = {key: Counter() for key in PATTERNS}
    for repo in repos:
        repo_id = repo["id"]
        repo_root = Path(args.repos_root) / repo_id
        for path in audit.source_files(repo_root):
            if audit.language_for_path(path) != "javascript-typescript":
                continue
            masked = audit.mask_comments_and_strings(path.read_text(errors="ignore"))
            rel = f"{repo_id}/{path.relative_to(repo_root)}"
            for key, pattern in PATTERNS.items():
                count = sum(1 for _ in pattern.finditer(masked))
                if count:
                    counts[key] += count
                    repo_counts[key][repo_id] += count
                    file_counts[key][rel] += count

    return {
        "repos_in_manifest": len(repos),
        "counts": {key: counts[key] for key in PATTERNS},
        "surfaces": [
            surface_summary(key, counts[key], repo_counts[key], file_counts[key])
            for key in PATTERNS
        ],
    }


def surface_summary(
    key: str,
    occurrences: int,
    by_repo: Counter[str],
    by_file: Counter[str],
) -> dict[str, Any]:
    labels = {
        "promise_all": "Promise.all",
        "literal_array": "Promise.all literal array argument",
        "literal_array_with_direct_resolve_seed": "Promise.all literal array with direct Promise.resolve seed",
    }
    return {
        "surface": key,
        "operation": labels[key],
        "occurrences": occurrences,
        "repos": len(by_repo),
        "top_repos": [
            {"repo": repo, "occurrences": count}
            for repo, count in by_repo.most_common(8)
        ],
        "top_files": [
            {"path": path, "occurrences": count}
            for path, count in by_file.most_common(8)
        ],
    }


def recall_loss_summary(path: str) -> dict[str, Any]:
    report_path = Path(path)
    report = json.loads(report_path.read_text())
    relevant = [
        item
        for item in report.get("by_obligation", [])
        if item.get("obligation_subreason", "").startswith("promise-aggregate")
    ]
    return {
        "report": path,
        "summary": report.get("summary", {}),
        "soundness_gate": report.get("soundness_gate", {}),
        "promise_aggregate_obligations": relevant,
    }


def build_report(args: argparse.Namespace) -> dict[str, Any]:
    audit = load_boundary_audit_module()
    corpus = count_corpus(args, audit)
    return {
        "report_kind": "promise-all-literal-aggregate-recovery",
        "schema_version": 1,
        "generated_on": args.generated_on,
        "tracking_issue": {
            "number": 602,
            "url": "https://github.com/corca-ai/nose/issues/602",
        },
        "opened_exact_slice": {
            "capability": "fulfilled-only Promise aggregate result channel",
            "operation": "Promise.all",
            "admitted": [
                "unshadowed global Promise.all",
                "one literal array argument",
                "every element recovers to a fulfilled Promise boundary",
                "result remains behind a Promise boundary with an ordered sequence payload",
            ],
            "closed": [
                "dynamic iterables",
                "rejected elements",
                "Promise.race",
                "Promise.allSettled",
                "Promise.any",
                "new Promise executor timing",
                "thenable assimilation",
                "sync array equivalence",
            ],
        },
        "policy": {
            "opened_exact_admission": True,
            "selector_only_admission": False,
            "raw_source_snippets_included": False,
            "semantic_admission_delta": "narrow Promise.all fulfilled-literal aggregate only",
        },
        "corpus_pricing": corpus,
        "expected_recall_delta": {
            "pinned_120_repo_direct_safe_seed_occurrences": corpus["counts"].get(
                "literal_array_with_direct_resolve_seed", 0
            ),
            "literal_array_boundary_occurrences": corpus["counts"].get("literal_array", 0),
            "note": "The opened exact subset is intentionally smaller than the literal-array boundary count because every element must already recover as fulfilled Promise evidence.",
        },
        "hard_negative_inventory": [
            "Promise.all with any rejected element stays closed",
            "Promise.all over a dynamic iterable stays closed",
            "Promise.all does not converge with Promise.race",
            "Promise.all does not converge with Promise.allSettled",
            "Promise.all does not converge with synchronous arrays",
        ],
        "current_recall_loss": recall_loss_summary(args.recall_loss_report),
        "regenerate": [
            "cargo run -q -p nose-cli -- verify crates --max-violations 0 --recall-loss-report target/recall-loss.issue-602-promise-all.crates.json",
            "python3 scripts/promise-all-aggregate-slice-audit.py --recall-loss-report target/recall-loss.issue-602-promise-all.crates.json --output target/promise-all-literal-aggregate-recovery.v1.json",
        ],
    }


def main() -> None:
    args = parse_args()
    report = build_report(args)
    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")


if __name__ == "__main__":
    main()
