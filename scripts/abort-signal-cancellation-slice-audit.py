#!/usr/bin/env python3
"""Build the #602 AbortSignal cancellation/liveness artifact.

This is a reporting-only audit for JS/TS cancellation surfaces. It prices
AbortController/AbortSignal construction, composition, signal options, and
scheduler/timer consumers before any exact cancellation semantics are opened.
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
DEFAULT_OUTPUT = "target/abort-signal-cancellation-boundary-audit.v1.json"
DEFAULT_GENERATED_ON = "2026-06-30"

ABORT_MENTION = re.compile(r"\b(?:AbortController|AbortSignal)\b")
ABORT_CONTROLLER_CONSTRUCTOR = re.compile(
    r"\bnew\s+AbortController\s*(?:<[^;\n(){}]*>)?\s*\("
)
ABORT_SELECTOR_CALL = re.compile(r"\.\s*abort\s*\(")
SIGNAL_PROPERTY_READ = re.compile(r"\.\s*signal\b")
ABORT_SIGNAL_ABORT = re.compile(r"\bAbortSignal\s*\.\s*abort\s*\(")
ABORT_SIGNAL_ANY = re.compile(r"\bAbortSignal\s*\.\s*any\s*\(")
ABORT_SIGNAL_TIMEOUT = re.compile(r"\bAbortSignal\s*\.\s*timeout\s*\(")
SIGNAL_OPTION = re.compile(r"(?<![A-Za-z0-9_$])signal\s*:")
FETCH_CALL = re.compile(r"(?<![A-Za-z0-9_$])fetch\s*\(")
ADD_EVENT_LISTENER_CALL = re.compile(r"\.\s*addEventListener\s*\(")
SCHEDULER_WAIT_CALL = re.compile(r"\bscheduler\s*\.\s*wait\s*\(")
TIMER_CALL = re.compile(
    r"(?<![A-Za-z0-9_$])(?:setTimeout|setImmediate|setInterval)\s*\("
)
PROMISE_STATIC_AGGREGATE = re.compile(
    r"\bPromise\s*\.\s*(?:race|any|all|allSettled)\s*(?:<[^;\n(){}]*>)?\s*\("
)


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


def find_matching(text: str, open_index: int) -> int | None:
    if open_index >= len(text) or text[open_index] != "(":
        return None
    depth = 0
    for index in range(open_index, len(text)):
        ch = text[index]
        if ch in "([{":
            depth += 1
        elif ch in ")]}":
            depth -= 1
            if depth == 0 and ch == ")":
                return index
            if depth < 0:
                return None
    return None


def call_span(masked: str, match: re.Match[str]) -> tuple[int, int] | None:
    open_index = masked.find("(", match.start(), match.end())
    if open_index < 0:
        return None
    close = find_matching(masked, open_index)
    if close is None:
        return None
    return open_index, close + 1


def call_has_signal_option(masked: str, match: re.Match[str]) -> bool:
    span = call_span(masked, match)
    if span is None:
        return False
    return SIGNAL_OPTION.search(masked, span[0], span[1]) is not None


class CounterMap(dict[str, Counter[str]]):
    def __missing__(self, key: str) -> Counter[str]:
        counter: Counter[str] = Counter()
        self[key] = counter
        return counter


def record_metric(
    metric: str,
    repo_id: str,
    rel: str,
    count: int,
    counts: Counter[str],
    repo_counts: dict[str, Counter[str]],
    file_counts: dict[str, Counter[str]],
) -> None:
    if count <= 0:
        return
    counts[metric] += count
    repo_counts[metric][repo_id] += count
    file_counts[metric][rel] += count


def record_call_metrics(
    masked: str,
    pattern: re.Pattern[str],
    all_metric: str,
    signal_metric: str,
    repo_id: str,
    rel: str,
    counts: Counter[str],
    repo_counts: dict[str, Counter[str]],
    file_counts: dict[str, Counter[str]],
) -> None:
    for match in pattern.finditer(masked):
        record_metric(all_metric, repo_id, rel, 1, counts, repo_counts, file_counts)
        if call_has_signal_option(masked, match):
            record_metric(signal_metric, repo_id, rel, 1, counts, repo_counts, file_counts)


def count_corpus(args: argparse.Namespace, audit: Any) -> dict[str, Any]:
    repos = audit.load_repos(Path(args.manifest))
    counts: Counter[str] = Counter()
    repo_counts: dict[str, Counter[str]] = CounterMap()
    file_counts: dict[str, Counter[str]] = CounterMap()

    for repo in repos:
        repo_id = repo["id"]
        repo_root = Path(args.repos_root) / repo_id
        for path in audit.source_files(repo_root):
            if audit.language_for_path(path) != "javascript-typescript":
                continue
            try:
                original = path.read_text(errors="ignore")
            except OSError:
                continue
            masked = audit.mask_comments_and_strings(original)
            rel = f"{repo_id}/{path.relative_to(repo_root)}"
            simple_metrics = {
                "all_abort_mentions": len(ABORT_MENTION.findall(masked)),
                "abort_controller_constructors": len(
                    ABORT_CONTROLLER_CONSTRUCTOR.findall(masked)
                ),
                "abort_selector_calls": len(ABORT_SELECTOR_CALL.findall(masked)),
                "signal_property_reads": len(SIGNAL_PROPERTY_READ.findall(masked)),
                "abort_signal_abort_calls": len(ABORT_SIGNAL_ABORT.findall(masked)),
                "abort_signal_any_calls": len(ABORT_SIGNAL_ANY.findall(masked)),
                "abort_signal_timeout_calls": len(ABORT_SIGNAL_TIMEOUT.findall(masked)),
                "signal_option_properties": len(SIGNAL_OPTION.findall(masked)),
                "promise_aggregate_calls": len(PROMISE_STATIC_AGGREGATE.findall(masked)),
            }
            for metric, count in simple_metrics.items():
                record_metric(metric, repo_id, rel, count, counts, repo_counts, file_counts)

            record_call_metrics(
                masked,
                FETCH_CALL,
                "fetch_calls",
                "fetch_calls_with_signal_option",
                repo_id,
                rel,
                counts,
                repo_counts,
                file_counts,
            )
            record_call_metrics(
                masked,
                ADD_EVENT_LISTENER_CALL,
                "add_event_listener_calls",
                "add_event_listener_calls_with_signal_option",
                repo_id,
                rel,
                counts,
                repo_counts,
                file_counts,
            )
            record_call_metrics(
                masked,
                SCHEDULER_WAIT_CALL,
                "scheduler_wait_calls",
                "scheduler_wait_calls_with_signal_option",
                repo_id,
                rel,
                counts,
                repo_counts,
                file_counts,
            )
            record_call_metrics(
                masked,
                TIMER_CALL,
                "timer_calls",
                "timer_calls_with_signal_option",
                repo_id,
                rel,
                counts,
                repo_counts,
                file_counts,
            )

            if (
                simple_metrics["abort_controller_constructors"] > 0
                and simple_metrics["signal_property_reads"] > 0
            ):
                record_metric(
                    "controller_signal_pair_files",
                    repo_id,
                    rel,
                    1,
                    counts,
                    repo_counts,
                    file_counts,
                )
            if (
                simple_metrics["abort_controller_constructors"] > 0
                and simple_metrics["abort_selector_calls"] > 0
            ):
                record_metric(
                    "controller_abort_lifecycle_pair_files",
                    repo_id,
                    rel,
                    1,
                    counts,
                    repo_counts,
                    file_counts,
                )

    return {
        "repos_in_manifest": len(repos),
        "counts": dict(sorted(counts.items())),
        "surfaces": [
            surface_summary(metric, counts[metric], repo_counts[metric], file_counts[metric])
            for metric in sorted(counts)
        ],
    }


def surface_summary(
    metric: str,
    occurrences: int,
    by_repo: Counter[str],
    by_file: Counter[str],
) -> dict[str, Any]:
    return {
        "surface": f"js-ts.cancellation.abort.{metric}",
        "operation": "AbortController/AbortSignal",
        "metric": metric.replace("_", " "),
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
    report = json.loads(Path(path).read_text())
    relevant = [
        item
        for item in report.get("by_obligation", [])
        if item.get("obligation_family") == "cancellation-liveness-boundary"
        or item.get("obligation_subreason", "").startswith("abort-")
    ]
    return {
        "report": path,
        "summary": report.get("summary", {}),
        "soundness_gate": report.get("soundness_gate", {}),
        "cancellation_liveness_obligations": relevant,
    }


def build_report(args: argparse.Namespace) -> dict[str, Any]:
    audit = load_boundary_audit_module()
    corpus = count_corpus(args, audit)
    counts = corpus["counts"]
    return {
        "report_kind": "abort-signal-cancellation-boundary-audit",
        "schema_version": 1,
        "generated_on": args.generated_on,
        "tracking_issue": {
            "number": 602,
            "url": "https://github.com/corca-ai/nose/issues/602",
        },
        "opened_exact_slice": {
            "capability": "none",
            "admitted": [],
            "closed": [
                "all AbortSignal cancellation and liveness behavior",
                "AbortController mutable signal state",
                "AbortSignal.timeout scheduling and timer-triggered cancellation",
                "AbortSignal.any composition and first-cancellation liveness",
                "signal-bearing timer/scheduler/fetch/event-listener option paths",
                "sync value or fulfilled-payload equivalence for cancelable operations",
            ],
        },
        "policy": {
            "opened_exact_admission": False,
            "selector_only_admission": False,
            "raw_source_snippets_included": False,
            "semantic_admission_delta": 0,
            "note": "This artifact prices cancellation/liveness surfaces and hard negatives before exact cancellation admission.",
        },
        "corpus_pricing": corpus,
        "expected_recall_delta": {
            "this_slice": 0,
            "all_abort_mentions": counts.get("all_abort_mentions", 0),
            "signal_option_properties": counts.get("signal_option_properties", 0),
            "timer_calls_with_signal_option": counts.get("timer_calls_with_signal_option", 0),
            "scheduler_wait_calls_with_signal_option": counts.get(
                "scheduler_wait_calls_with_signal_option", 0
            ),
            "note": "Counts are lexical pricing only. Future exact admission still needs cancellation state/liveness proof, settled-channel proof for affected APIs, and hard negatives for abort-before/abort-after ordering.",
        },
        "hard_negative_inventory": [
            "AbortSignal-bearing timer options must not recover as fulfilled payloads",
            "scheduler waits with signal options must not converge with uncancelable waits",
            "AbortSignal.timeout must not converge with a fresh non-timeout signal",
            "AbortSignal.any must not converge with first-signal identity",
            "controller abort/signal lifecycle state must remain effectful and closed",
            "fetch/event listener signal options stay closed until cancellation side effects are represented",
        ],
        "next_exact_admission_requirements": [
            "source/API occurrence proof for the cancelable API",
            "signal identity and lifecycle-state proof",
            "abort-before, abort-during, and abort-after ordering model",
            "fulfilled/rejected/aborted channel distinction",
            "timer/scheduler liveness and cleanup proof when time is involved",
            "no selector-only `.abort` or `.signal` admission",
            "result remains behind the protocol boundary instead of becoming a sync value",
        ],
        "current_recall_loss": recall_loss_summary(args.recall_loss_report),
        "regenerate": [
            "cargo run -q -p nose-cli -- verify crates --max-violations 0 --recall-loss-report target/recall-loss.issue-602-abort-signal.crates.json",
            "python3 scripts/abort-signal-cancellation-slice-audit.py --recall-loss-report target/recall-loss.issue-602-abort-signal.crates.json --output target/abort-signal-cancellation-boundary-audit.v1.json",
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
