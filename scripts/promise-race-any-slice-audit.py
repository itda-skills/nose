#!/usr/bin/env python3
"""Build the #602 Promise.race/Promise.any literal aggregate artifact.

This is a lexical pricing report for the narrow exact slice that admits
non-empty literal `Promise.race` aggregates when every input has closed
settlement/raw-input evidence, and literal `Promise.any` aggregates when every
input is closed and at least one input is fulfilled. All-rejected `Promise.any`
stays closed until AggregateError payloads are modeled.
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
DEFAULT_OUTPUT = "target/promise-race-any-literal-aggregate-recovery.v1.json"
DEFAULT_GENERATED_ON = "2026-06-30"

OPERATIONS = {
    "promise_race": {
        "operation": "Promise.race",
        "call": re.compile(r"\bPromise\s*\.\s*race\s*(?:<[^;\n(){}]*>)?\s*\("),
    },
    "promise_any": {
        "operation": "Promise.any",
        "call": re.compile(r"\bPromise\s*\.\s*any\s*(?:<[^;\n(){}]*>)?\s*\("),
    },
}

NUMERIC_LITERAL = re.compile(
    r"\A[+-]?(?:(?:\d+(?:\.\d*)?)|(?:\.\d+))(?:[eE][+-]?\d+)?n?\Z"
)
NULLISH_BOOL_LITERAL = {"true", "false", "null", "undefined"}
PROMISE_RESOLVE = re.compile(r"\APromise\s*\.\s*resolve\s*\(")
PROMISE_REJECT = re.compile(r"\APromise\s*\.\s*reject\s*\(")


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
                if masked[start:index].strip():
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


def classify_element(masked: str, original: str) -> str:
    masked_text = masked.strip()
    original_text = original.strip()
    if not original_text:
        return "empty"
    if masked_text in NULLISH_BOOL_LITERAL or NUMERIC_LITERAL.match(masked_text):
        return "raw_fulfilled"
    if is_string_literal(original_text):
        return "raw_fulfilled"
    if PROMISE_RESOLVE.match(masked_text):
        return "promise_fulfilled"
    if PROMISE_REJECT.match(masked_text):
        return "promise_rejected"
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
                    if not spans:
                        record_metric(
                            op_key,
                            "empty_literal_array_calls",
                            repo_id,
                            rel,
                            counts,
                            repo_counts,
                            file_counts,
                        )
                        continue
                    classes = [
                        classify_element(masked[start:end], original[start:end])
                        for start, end in spans
                    ]
                    has_fulfilled = any(
                        kind in {"raw_fulfilled", "promise_fulfilled"} for kind in classes
                    )
                    has_rejected = any(kind == "promise_rejected" for kind in classes)
                    all_closed = all(
                        kind in {"raw_fulfilled", "promise_fulfilled", "promise_rejected"}
                        for kind in classes
                    )
                    if any(kind == "raw_fulfilled" for kind in classes):
                        record_metric(
                            op_key,
                            "literal_array_with_raw_fulfilled_element",
                            repo_id,
                            rel,
                            counts,
                            repo_counts,
                            file_counts,
                        )
                    if has_rejected:
                        record_metric(
                            op_key,
                            "literal_array_with_rejected_seed",
                            repo_id,
                            rel,
                            counts,
                            repo_counts,
                            file_counts,
                        )
                    if all_closed:
                        record_metric(
                            op_key,
                            "literal_array_fully_closed_candidate",
                            repo_id,
                            rel,
                            counts,
                            repo_counts,
                            file_counts,
                        )
                    if op_key == "promise_any" and all_closed and has_fulfilled:
                        record_metric(
                            op_key,
                            "literal_array_fully_closed_with_fulfilled_candidate",
                            repo_id,
                            rel,
                            counts,
                            repo_counts,
                            file_counts,
                        )
                    if op_key == "promise_any" and all_closed and not has_fulfilled:
                        record_metric(
                            op_key,
                            "literal_array_all_rejected_closed_boundary",
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
    counts = corpus["counts"]
    return {
        "report_kind": "promise-race-any-literal-aggregate-recovery",
        "schema_version": 1,
        "generated_on": args.generated_on,
        "tracking_issue": {
            "number": 602,
            "url": "https://github.com/corca-ai/nose/issues/602",
        },
        "opened_exact_slice": {
            "capability": "first-observed literal aggregate settlement",
            "operations": ["Promise.race", "Promise.any"],
            "admitted": [
                "unshadowed global Promise aggregate call",
                "one literal array argument",
                "every element is recoverable as a Promise settlement or non-thenable-safe raw fulfilled input",
                "Promise.race returns the first literal element settlement",
                "Promise.any returns the first fulfilled literal element settlement when one exists",
                "result remains behind the aggregate Promise boundary",
            ],
            "closed": [
                "dynamic iterables",
                "empty Promise.race because it never settles",
                "all-rejected Promise.any until AggregateError payloads are modeled",
                "object/function inputs without non-thenable proof",
                "untyped raw variables because they may be thenables",
                "custom thenables",
                "new Promise executor timing",
                "cancellation/liveness and broad scheduler ordering",
                "sync value equivalence",
            ],
        },
        "policy": {
            "opened_exact_admission": True,
            "selector_only_admission": False,
            "raw_source_snippets_included": False,
            "semantic_admission_delta": "narrow first-settled/first-fulfilled recovery for closed literal Promise aggregates",
        },
        "corpus_pricing": corpus,
        "expected_recall_delta": {
            "promise_race_literal_arrays_fully_closed_candidates": counts.get(
                "promise_race.literal_array_fully_closed_candidate", 0
            ),
            "promise_any_literal_arrays_with_fulfilled_fully_closed_candidates": counts.get(
                "promise_any.literal_array_fully_closed_with_fulfilled_candidate", 0
            ),
            "promise_any_all_rejected_literal_arrays_kept_closed": counts.get(
                "promise_any.literal_array_all_rejected_closed_boundary", 0
            ),
            "note": "The lexical candidate count is an upper bound: exact admission still requires frontend static-global evidence and value-graph settlement/non-thenable proof for every element.",
        },
        "hard_negative_inventory": [
            "Promise.race preserves first-settled element order and channel",
            "Promise.race([]) stays closed as a never-settling aggregate",
            "Promise.any preserves first-fulfilled element order",
            "all-rejected Promise.any stays closed instead of converging with a Promise.reject array substitute",
            "possible thenables keep the whole first-observed aggregate closed",
            "dynamic iterable aggregate inputs stay closed",
            "first-observed aggregates preserve Promise boundaries and do not converge with sync values",
        ],
        "current_recall_loss": recall_loss_summary(args.recall_loss_report),
        "regenerate": [
            "cargo run -q -p nose-cli -- verify crates --max-violations 0 --recall-loss-report target/recall-loss.issue-602-promise-race-any.crates.json",
            "python3 scripts/promise-race-any-slice-audit.py --recall-loss-report target/recall-loss.issue-602-promise-race-any.crates.json --output target/promise-race-any-literal-aggregate-recovery.v1.json",
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
