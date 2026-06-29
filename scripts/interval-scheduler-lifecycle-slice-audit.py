#!/usr/bin/env python3
"""Build the #602 interval/scheduler liveness artifact.

This is a reporting-only audit for JS/TS timer, interval, scheduler, and
microtask surfaces. It prices lifecycle and ordering boundaries before any
exact scheduling or interval semantics are opened.
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
DEFAULT_OUTPUT = "target/interval-scheduler-lifecycle-boundary-audit.v1.json"
DEFAULT_GENERATED_ON = "2026-06-30"

SET_TIMEOUT_CALL = re.compile(r"(?<![A-Za-z0-9_$])setTimeout\s*\(")
SET_IMMEDIATE_CALL = re.compile(r"(?<![A-Za-z0-9_$])setImmediate\s*\(")
SET_INTERVAL_CALL = re.compile(r"(?<![A-Za-z0-9_$])setInterval\s*\(")
MEMBER_SET_INTERVAL_CALL = re.compile(r"\.\s*setInterval\s*\(")
CLEAR_INTERVAL_CALL = re.compile(r"(?<![A-Za-z0-9_$])clearInterval\s*\(")
CLEAR_TIMEOUT_CALL = re.compile(r"(?<![A-Za-z0-9_$])clearTimeout\s*\(")
QUEUE_MICROTASK_CALL = re.compile(r"(?<![A-Za-z0-9_$])queueMicrotask\s*\(")
REQUEST_ANIMATION_FRAME_CALL = re.compile(
    r"(?<![A-Za-z0-9_$])requestAnimationFrame\s*\("
)
CANCEL_ANIMATION_FRAME_CALL = re.compile(r"(?<![A-Za-z0-9_$])cancelAnimationFrame\s*\(")
SCHEDULER_WAIT_CALL = re.compile(r"\bscheduler\s*\.\s*wait\s*\(")
SCHEDULER_YIELD_CALL = re.compile(r"\bscheduler\s*\.\s*yield\s*\(")
TIMERS_PROMISES_MODULE = re.compile(r"['\"](?:node:)?timers/promises['\"]")
AWAIT_PREFIX = re.compile(r"\bawait\s+$")
SIGNAL_OPTION = re.compile(r"(?<![A-Za-z0-9_$])signal\s*:")
NEW_PROMISE = re.compile(r"\bnew\s+Promise\s*(?:<[^;\n(){}]*>)?\s*\(")


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


def call_is_awaited(masked: str, match: re.Match[str]) -> bool:
    prefix = masked[max(0, match.start() - 32) : match.start()]
    return AWAIT_PREFIX.search(prefix) is not None


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
    signal_metric: str | None,
    awaited_metric: str | None,
    repo_id: str,
    rel: str,
    counts: Counter[str],
    repo_counts: dict[str, Counter[str]],
    file_counts: dict[str, Counter[str]],
) -> None:
    for match in pattern.finditer(masked):
        record_metric(all_metric, repo_id, rel, 1, counts, repo_counts, file_counts)
        if signal_metric is not None and call_has_signal_option(masked, match):
            record_metric(signal_metric, repo_id, rel, 1, counts, repo_counts, file_counts)
        if awaited_metric is not None and call_is_awaited(masked, match):
            record_metric(awaited_metric, repo_id, rel, 1, counts, repo_counts, file_counts)


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
                "timers_promises_module_mentions": len(
                    TIMERS_PROMISES_MODULE.findall(original)
                ),
                "member_set_interval_calls": len(MEMBER_SET_INTERVAL_CALL.findall(masked)),
                "clear_interval_calls": len(CLEAR_INTERVAL_CALL.findall(masked)),
                "clear_timeout_calls": len(CLEAR_TIMEOUT_CALL.findall(masked)),
                "queue_microtask_calls": len(QUEUE_MICROTASK_CALL.findall(masked)),
                "request_animation_frame_calls": len(
                    REQUEST_ANIMATION_FRAME_CALL.findall(masked)
                ),
                "cancel_animation_frame_calls": len(
                    CANCEL_ANIMATION_FRAME_CALL.findall(masked)
                ),
            }
            for metric, count in simple_metrics.items():
                record_metric(metric, repo_id, rel, count, counts, repo_counts, file_counts)

            record_call_metrics(
                masked,
                SET_TIMEOUT_CALL,
                "set_timeout_calls",
                "set_timeout_calls_with_signal_option",
                "awaited_set_timeout_calls",
                repo_id,
                rel,
                counts,
                repo_counts,
                file_counts,
            )
            record_call_metrics(
                masked,
                SET_IMMEDIATE_CALL,
                "set_immediate_calls",
                "set_immediate_calls_with_signal_option",
                "awaited_set_immediate_calls",
                repo_id,
                rel,
                counts,
                repo_counts,
                file_counts,
            )
            record_call_metrics(
                masked,
                SET_INTERVAL_CALL,
                "set_interval_calls",
                "set_interval_calls_with_signal_option",
                "awaited_set_interval_calls",
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
                "awaited_scheduler_wait_calls",
                repo_id,
                rel,
                counts,
                repo_counts,
                file_counts,
            )
            record_call_metrics(
                masked,
                SCHEDULER_YIELD_CALL,
                "scheduler_yield_calls",
                None,
                "awaited_scheduler_yield_calls",
                repo_id,
                rel,
                counts,
                repo_counts,
                file_counts,
            )

            if SET_INTERVAL_CALL.search(masked) and CLEAR_INTERVAL_CALL.search(masked):
                record_metric(
                    "set_interval_clear_interval_pair_files",
                    repo_id,
                    rel,
                    1,
                    counts,
                    repo_counts,
                    file_counts,
                )
            if SET_INTERVAL_CALL.search(masked) and not CLEAR_INTERVAL_CALL.search(masked):
                record_metric(
                    "set_interval_without_clear_interval_files",
                    repo_id,
                    rel,
                    1,
                    counts,
                    repo_counts,
                    file_counts,
                )
            if NEW_PROMISE.search(masked) and (
                SET_TIMEOUT_CALL.search(masked)
                or SET_IMMEDIATE_CALL.search(masked)
                or SET_INTERVAL_CALL.search(masked)
            ):
                record_metric(
                    "new_promise_timer_pair_files",
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
        "surface": f"js-ts.scheduling.lifecycle.{metric}",
        "operation": "timers/scheduler/interval/microtask",
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
        if item.get("obligation_family")
        in {
            "scheduling-boundary",
            "cancellation-liveness-boundary",
            "lifecycle-materialization-boundary",
        }
        or item.get("obligation_subreason", "").startswith(
            ("timer-", "scheduler-", "interval-")
        )
    ]
    return {
        "report": path,
        "summary": report.get("summary", {}),
        "soundness_gate": report.get("soundness_gate", {}),
        "scheduling_lifecycle_obligations": relevant,
    }


def build_report(args: argparse.Namespace) -> dict[str, Any]:
    audit = load_boundary_audit_module()
    corpus = count_corpus(args, audit)
    counts = corpus["counts"]
    return {
        "report_kind": "interval-scheduler-lifecycle-boundary-audit",
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
                "global timer scheduling",
                "scheduler.wait timing and cancellation/liveness",
                "scheduler.yield microtask ordering",
                "setInterval repeated-emission lifecycle",
                "clearInterval interval cancellation lifecycle",
                "clearTimeout and cancelAnimationFrame one-shot cancellation lifecycle",
                "queueMicrotask and animation-frame callback ordering",
                "sync value or direct callback equivalence for scheduled operations",
            ],
        },
        "policy": {
            "opened_exact_admission": False,
            "selector_only_admission": False,
            "raw_source_snippets_included": False,
            "semantic_admission_delta": 0,
            "note": "This artifact prices interval/scheduler liveness surfaces and hard negatives before exact scheduling admission.",
        },
        "corpus_pricing": corpus,
        "expected_recall_delta": {
            "this_slice": 0,
            "set_interval_calls": counts.get("set_interval_calls", 0),
            "scheduler_wait_calls": counts.get("scheduler_wait_calls", 0),
            "scheduler_yield_calls": counts.get("scheduler_yield_calls", 0),
            "queue_microtask_calls": counts.get("queue_microtask_calls", 0),
            "set_timeout_calls": counts.get("set_timeout_calls", 0),
            "note": "Counts are lexical pricing only. Exact admission still needs ordering, cancellation, cardinality, and lifecycle proof.",
        },
        "hard_negative_inventory": [
            "setInterval must not converge with one-shot setTimeout",
            "clearInterval must not converge with clearTimeout",
            "scheduler.yield must not converge with Promise.resolve",
            "scheduler.wait must not converge with global setTimeout or sync payloads",
            "queueMicrotask must not converge with direct callback invocation",
            "requestAnimationFrame must not converge with setTimeout timing",
        ],
        "next_exact_admission_requirements": [
            "source/API occurrence proof for the scheduled API",
            "callback identity and callback demand/effect profile",
            "microtask, task, timer, and animation-frame ordering model",
            "interval cardinality and cancellation lifecycle proof",
            "AbortSignal and clear/cancel interaction proof where options or handles appear",
            "result remains behind a protocol boundary rather than becoming a sync value",
            "hard negatives for one-shot versus repeated emission and direct versus deferred callbacks",
        ],
        "current_recall_loss": recall_loss_summary(args.recall_loss_report),
        "regenerate": [
            "cargo run -q -p nose-cli -- verify crates --max-violations 0 --recall-loss-report target/recall-loss.issue-602-interval-scheduler.crates.json",
            "python3 scripts/interval-scheduler-lifecycle-slice-audit.py --recall-loss-report target/recall-loss.issue-602-interval-scheduler.crates.json --output target/interval-scheduler-lifecycle-boundary-audit.v1.json",
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
