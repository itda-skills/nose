#!/usr/bin/env python3
"""Build the #602 Promise aggregate raw-input assimilation artifact.

This is a lexical pricing report for the narrow exact slice that treats
non-thenable-safe raw literal/scalar inputs as fulfilled aggregate elements for
already-admitted literal `Promise.all` and `Promise.allSettled` calls.
"""

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
DEFAULT_OUTPUT = "target/promise-aggregate-raw-input-recovery.v1.json"
DEFAULT_GENERATED_ON = "2026-06-29"

OPERATIONS = {
    "promise_all": {
        "operation": "Promise.all",
        "call": re.compile(r"\bPromise\s*\.\s*all\s*(?:<[^;\n(){}]*>)?\s*\("),
        "direct_seed": re.compile(r"\APromise\s*\.\s*resolve\s*\("),
    },
    "promise_all_settled": {
        "operation": "Promise.allSettled",
        "call": re.compile(r"\bPromise\s*\.\s*allSettled\s*(?:<[^;\n(){}]*>)?\s*\("),
        "direct_seed": re.compile(r"\APromise\s*\.\s*(?:resolve|reject)\s*\("),
    },
}

NUMERIC_LITERAL = re.compile(
    r"\A[+-]?(?:(?:\d+(?:\.\d*)?)|(?:\.\d+))(?:[eE][+-]?\d+)?n?\Z"
)
NULLISH_BOOL_LITERAL = {"true", "false", "null", "undefined"}


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
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", default=DEFAULT_MANIFEST)
    parser.add_argument("--repos-root", default=DEFAULT_REPOS_ROOT)
    parser.add_argument("--recall-loss-report", required=True)
    parser.add_argument("--output", default=DEFAULT_OUTPUT)
    parser.add_argument("--generated-on", default=DEFAULT_GENERATED_ON)
    return parser.parse_args()


def literal_array_element_spans(masked: str, call_end: int) -> list[tuple[int, int]] | None:
    index = call_end
    while index < len(masked) and masked[index].isspace():
        index += 1
    if index >= len(masked) or masked[index] != "[":
        return None

    spans: list[tuple[int, int]] = []
    start = index + 1
    depth = 0
    index += 1
    while index < len(masked):
        ch = masked[index]
        if ch in "([{":
            depth += 1
        elif ch in ")]}":
            if depth > 0:
                depth -= 1
            elif ch == "]":
                spans.append((start, index))
                return spans
            else:
                return None
        elif ch == "," and depth == 0:
            spans.append((start, index))
            start = index + 1
        index += 1
    return None


def is_string_literal(original: str) -> bool:
    text = original.strip()
    if len(text) < 2:
        return False
    quote = text[0]
    return quote in {"'", '"'} and text.endswith(quote) and "\n" not in text


def classify_element(masked: str, original: str, seed_pattern: re.Pattern[str]) -> str:
    masked_text = masked.strip()
    original_text = original.strip()
    if not original_text:
        return "empty"
    if masked_text in NULLISH_BOOL_LITERAL or NUMERIC_LITERAL.match(masked_text):
        return "raw_non_thenable"
    if is_string_literal(original_text):
        return "raw_non_thenable"
    if seed_pattern.match(masked_text):
        return "direct_promise_seed"
    if original_text.startswith("{") or original_text.startswith("function"):
        return "object_or_callable"
    return "other"


def count_corpus(args: argparse.Namespace, audit: Any) -> dict[str, Any]:
    repos = audit.load_repos(Path(args.manifest))
    counts = Counter()
    repo_counts: dict[str, Counter[str]] = CounterMap()
    file_counts: dict[str, Counter[str]] = CounterMap()

    for repo in repos:
        repo_id = repo["id"]
        repo_root = Path(args.repos_root) / repo_id
        for path in audit.source_files(repo_root):
            if audit.language_for_path(path) != "javascript-typescript":
                continue
            original = path.read_text(errors="ignore")
            masked = audit.mask_comments_and_strings(original)
            rel = f"{repo_id}/{path.relative_to(repo_root)}"
            for op_key, op in OPERATIONS.items():
                for match in op["call"].finditer(masked):
                    record_metric(op_key, "all_calls", repo_id, rel, counts, repo_counts, file_counts)
                    spans = literal_array_element_spans(masked, match.end())
                    if spans is None:
                        continue
                    record_metric(
                        op_key,
                        "literal_array_calls",
                        repo_id,
                        rel,
                        counts,
                        repo_counts,
                        file_counts,
                    )
                    classes = [
                        classify_element(masked[start:end], original[start:end], op["direct_seed"])
                        for start, end in spans
                    ]
                    if any(kind == "raw_non_thenable" for kind in classes):
                        record_metric(
                            op_key,
                            "literal_array_with_raw_non_thenable_element",
                            repo_id,
                            rel,
                            counts,
                            repo_counts,
                            file_counts,
                        )
                    if classes and all(
                        kind in {"raw_non_thenable", "direct_promise_seed"} for kind in classes
                    ):
                        record_metric(
                            op_key,
                            "literal_array_fully_direct_safe_candidate",
                            repo_id,
                            rel,
                            counts,
                            repo_counts,
                            file_counts,
                        )
                    if any(kind == "object_or_callable" for kind in classes):
                        record_metric(
                            op_key,
                            "literal_array_with_object_or_callable_element",
                            repo_id,
                            rel,
                            counts,
                            repo_counts,
                            file_counts,
                        )

    return {
        "repos_in_manifest": len(repos),
        "counts": dict(counts),
        "surfaces": [
            surface_summary(metric, counts[metric], repo_counts[metric], file_counts[metric])
            for metric in sorted(counts)
        ],
    }


class CounterMap(dict[str, Counter[str]]):
    def __missing__(self, key: str) -> Counter[str]:
        counter: Counter[str] = Counter()
        self[key] = counter
        return counter


def record_metric(
    op_key: str,
    metric: str,
    repo_id: str,
    rel: str,
    counts: Counter[str],
    repo_counts: dict[str, Counter[str]],
    file_counts: dict[str, Counter[str]],
) -> None:
    key = f"{op_key}.{metric}"
    counts[key] += 1
    repo_counts[key][repo_id] += 1
    file_counts[key][rel] += 1


def surface_summary(
    metric: str,
    occurrences: int,
    by_repo: Counter[str],
    by_file: Counter[str],
) -> dict[str, Any]:
    op_key, metric_key = metric.split(".", 1)
    return {
        "surface": metric,
        "operation": OPERATIONS[op_key]["operation"],
        "metric": metric_key.replace("_", " "),
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
        "report_kind": "promise-aggregate-raw-input-recovery",
        "schema_version": 1,
        "generated_on": args.generated_on,
        "tracking_issue": {
            "number": 602,
            "url": "https://github.com/corca-ai/nose/issues/602",
        },
        "opened_exact_slice": {
            "capability": "non-thenable-safe raw aggregate input assimilation",
            "operations": ["Promise.all", "Promise.allSettled"],
            "admitted": [
                "unshadowed global Promise aggregate call",
                "one literal array argument",
                "each element is either already recoverable as a Promise settlement or proves non-thenable-safe raw input",
                "raw inputs become fulfilled aggregate elements",
                "result remains behind the aggregate Promise boundary",
            ],
            "closed": [
                "dynamic iterables",
                "object/function inputs without non-thenable proof",
                "untyped raw variables because they may be thenables",
                "custom thenables",
                "Promise.race",
                "Promise.any",
                "new Promise executor timing",
                "sync array or sync settled-record equivalence",
            ],
        },
        "policy": {
            "opened_exact_admission": True,
            "selector_only_admission": False,
            "raw_source_snippets_included": False,
            "semantic_admission_delta": "narrow raw non-thenable input assimilation for already-admitted literal Promise aggregates",
        },
        "corpus_pricing": corpus,
        "expected_recall_delta": {
            "promise_all_literal_arrays_with_direct_raw_non_thenable_element": corpus[
                "counts"
            ].get("promise_all.literal_array_with_raw_non_thenable_element", 0),
            "promise_all_settled_literal_arrays_with_direct_raw_non_thenable_element": corpus[
                "counts"
            ].get("promise_all_settled.literal_array_with_raw_non_thenable_element", 0),
            "fully_lexical_direct_safe_candidates": corpus["counts"].get(
                "promise_all.literal_array_fully_direct_safe_candidate", 0
            )
            + corpus["counts"].get(
                "promise_all_settled.literal_array_fully_direct_safe_candidate", 0
            ),
            "note": "The lexical candidate count is an upper bound: exact admission still requires frontend static-global evidence and value-graph non-thenable proof for every element.",
        },
        "hard_negative_inventory": [
            "untyped raw aggregate inputs stay closed as possible thenables",
            "object/function raw aggregate inputs stay closed without stronger non-thenable proof",
            "dynamic iterable aggregate inputs stay closed",
            "raw input assimilation preserves Promise boundaries",
            "all-fulfilled and all-settled aggregate result channels stay distinct",
        ],
        "current_recall_loss": recall_loss_summary(args.recall_loss_report),
        "regenerate": [
            "cargo run -q -p nose-cli -- verify crates --max-violations 0 --recall-loss-report target/recall-loss.issue-602-promise-aggregate-raw-input.crates.json",
            "python3 scripts/promise-aggregate-raw-input-slice-audit.py --recall-loss-report target/recall-loss.issue-602-promise-aggregate-raw-input.crates.json --output target/promise-aggregate-raw-input-recovery.v1.json",
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
