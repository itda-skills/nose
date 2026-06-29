#!/usr/bin/env python3
"""Build the #602 Promise executor readiness artifact.

This is a reporting-only audit for `new Promise(...)` executor surfaces. It
prices inline executor shapes and hard-negative boundaries before any exact
constructor settlement recovery is opened.
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
DEFAULT_OUTPUT = "target/promise-executor-boundary-audit.v1.json"
DEFAULT_GENERATED_ON = "2026-06-30"

PROMISE_CONSTRUCTOR = re.compile(r"\bnew\s+Promise\s*(?:<[^;\n(){}]*>)?\s*\(")
IDENT = re.compile(r"\A[A-Za-z_$][A-Za-z0-9_$]*\Z")
NUMERIC_LITERAL = re.compile(
    r"\A[+-]?(?:(?:\d+(?:\.\d*)?)|(?:\.\d+))(?:[eE][+-]?\d+)?n?\Z"
)
NULLISH_BOOL_LITERALS = {"true", "false", "null", "undefined"}
ASYNC_OR_TIMER = re.compile(
    r"\b(?:await|setTimeout|setImmediate|setInterval|queueMicrotask|"
    r"requestAnimationFrame|process\s*\.\s*nextTick|scheduler\s*\.\s*"
    r"(?:wait|yield)|AbortController|AbortSignal|addEventListener)\b"
)
CALL_NAME = re.compile(r"(?<![A-Za-z0-9_$])([A-Za-z_$][A-Za-z0-9_$]*)\s*\(")
IGNORED_CALL_WORDS = {
    "if",
    "for",
    "while",
    "switch",
    "catch",
    "function",
    "return",
    "throw",
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
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", default=DEFAULT_MANIFEST)
    parser.add_argument("--repos-root", default=DEFAULT_REPOS_ROOT)
    parser.add_argument("--recall-loss-report", required=True)
    parser.add_argument("--output", default=DEFAULT_OUTPUT)
    parser.add_argument("--generated-on", default=DEFAULT_GENERATED_ON)
    return parser.parse_args()


def find_matching(text: str, open_index: int, open_ch: str, close_ch: str) -> int | None:
    if open_index >= len(text) or text[open_index] != open_ch:
        return None
    depth = 0
    for index in range(open_index, len(text)):
        ch = text[index]
        if ch == open_ch:
            depth += 1
        elif ch == close_ch:
            depth -= 1
            if depth == 0:
                return index
    return None


def split_top_level_spans(text: str, start: int, end: int) -> list[tuple[int, int]]:
    spans: list[tuple[int, int]] = []
    current = start
    depth = 0
    for index in range(start, end):
        ch = text[index]
        if ch in "([{":
            depth += 1
        elif ch in ")]}":
            depth = max(0, depth - 1)
        elif ch == "," and depth == 0:
            if text[current:index].strip():
                spans.append((current, index))
            current = index + 1
    if text[current:end].strip():
        spans.append((current, end))
    return spans


def find_top_level_arrow(text: str, start: int, end: int) -> int | None:
    depth = 0
    index = start
    while index + 1 < end:
        ch = text[index]
        if ch in "([{":
            depth += 1
        elif ch in ")]}":
            depth = max(0, depth - 1)
        elif ch == "=" and text[index + 1] == ">" and depth == 0:
            return index
        index += 1
    return None


def trim_span(text: str, start: int, end: int) -> tuple[int, int]:
    while start < end and text[start].isspace():
        start += 1
    while end > start and text[end - 1].isspace():
        end -= 1
    return start, end


def parse_params(masked: str, start: int, end: int) -> list[str]:
    start, end = trim_span(masked, start, end)
    if start >= end:
        return []
    if masked[start] == "(":
        close = find_matching(masked, start, "(", ")")
        if close is None or close > end:
            return []
        spans = split_top_level_spans(masked, start + 1, close)
        params = []
        for param_start, param_end in spans:
            text = masked[param_start:param_end].strip()
            text = text.split("=", 1)[0].strip()
            text = text.lstrip("...").strip()
            name = re.split(r"[:\s]", text, maxsplit=1)[0]
            if IDENT.match(name):
                params.append(name)
        return params
    text = masked[start:end].strip()
    return [text] if IDENT.match(text) else []


def parse_arrow_executor(masked: str, start: int, end: int) -> dict[str, Any] | None:
    arrow = find_top_level_arrow(masked, start, end)
    if arrow is None:
        return None
    params = parse_params(masked, start, arrow)
    body_start, body_end = trim_span(masked, arrow + 2, end)
    body_kind = "expression"
    if body_start < body_end and masked[body_start] == "{":
        close = find_matching(masked, body_start, "{", "}")
        if close is not None and close <= body_end:
            body_start += 1
            body_end = close
            body_kind = "block"
    return {"kind": "arrow", "params": params, "body": (body_start, body_end), "body_kind": body_kind}


def parse_function_executor(masked: str, start: int, end: int) -> dict[str, Any] | None:
    text = masked[start:end]
    rel = text.find("function")
    if rel < 0:
        return None
    fn = start + rel
    params_open = masked.find("(", fn, end)
    if params_open < 0:
        return None
    params_close = find_matching(masked, params_open, "(", ")")
    if params_close is None or params_close > end:
        return None
    body_open = masked.find("{", params_close, end)
    if body_open < 0:
        return None
    body_close = find_matching(masked, body_open, "{", "}")
    if body_close is None or body_close > end:
        return None
    return {
        "kind": "function",
        "params": parse_params(masked, params_open, params_close + 1),
        "body": (body_open + 1, body_close),
        "body_kind": "block",
    }


def is_string_literal(original: str) -> bool:
    text = original.strip()
    if len(text) < 2:
        return False
    return text[0] in {"'", '"'} and text[-1] == text[0] and "\n" not in text


def classify_payload(masked: str, original: str, start: int, end: int) -> str:
    masked_text = masked[start:end].strip()
    original_text = original[start:end].strip()
    if not original_text:
        return "empty"
    if masked_text in NULLISH_BOOL_LITERALS or NUMERIC_LITERAL.match(masked_text):
        return "scalar_non_thenable"
    if is_string_literal(original_text):
        return "scalar_non_thenable"
    if masked_text.startswith("{") or masked_text.startswith("function") or "=>" in masked_text:
        return "object_or_callable_thenable_risk"
    if re.match(r"\A[A-Za-z_$][A-Za-z0-9_$]*\s*\(", masked_text):
        return "call_result_thenable_risk"
    if IDENT.match(masked_text):
        return "identifier_possible_thenable"
    return "other_possible_thenable"


def call_argument_span(masked: str, call_match: re.Match[str]) -> tuple[int, int] | None:
    open_index = masked.find("(", call_match.start(), call_match.end())
    if open_index < 0:
        return None
    close = find_matching(masked, open_index, "(", ")")
    if close is None:
        return None
    args = split_top_level_spans(masked, open_index + 1, close)
    return args[0] if args else (open_index + 1, close)


def settlement_calls(masked: str, original: str, body: tuple[int, int], name: str) -> list[dict[str, Any]]:
    calls: list[dict[str, Any]] = []
    if not name:
        return calls
    pattern = re.compile(rf"(?<![A-Za-z0-9_$]){re.escape(name)}\s*\(")
    for match in pattern.finditer(masked, body[0], body[1]):
        arg_span = call_argument_span(masked, match)
        payload_kind = "missing"
        if arg_span is not None:
            payload_kind = classify_payload(masked, original, arg_span[0], arg_span[1])
        calls.append({"span": (match.start(), match.end()), "arg_span": arg_span, "payload": payload_kind})
    return calls


def body_has_extra_calls(masked: str, body: tuple[int, int], settlement_names: set[str]) -> bool:
    for match in CALL_NAME.finditer(masked, body[0], body[1]):
        name = match.group(1)
        if name in settlement_names or name in IGNORED_CALL_WORDS:
            continue
        return True
    return False


def is_exact_single_statement(masked: str, body: tuple[int, int], call: dict[str, Any]) -> bool:
    call_start = call["span"][0]
    open_index = masked.find("(", call_start, call["span"][1])
    close = find_matching(masked, open_index, "(", ")") if open_index >= 0 else None
    if close is None:
        return False
    prefix = masked[body[0]:call_start].strip()
    suffix = masked[close + 1 : body[1]].strip()
    return prefix in {"", "return"} and suffix in {"", ";"}


def record_metric(
    metric: str,
    repo_id: str,
    rel: str,
    counts: Counter[str],
    repo_counts: dict[str, Counter[str]],
    file_counts: dict[str, Counter[str]],
) -> None:
    counts[metric] += 1
    repo_counts[metric][repo_id] += 1
    file_counts[metric][rel] += 1


class CounterMap(dict[str, Counter[str]]):
    def __missing__(self, key: str) -> Counter[str]:
        counter: Counter[str] = Counter()
        self[key] = counter
        return counter


def surface_summary(
    metric: str,
    occurrences: int,
    by_repo: Counter[str],
    by_file: Counter[str],
) -> dict[str, Any]:
    return {
        "surface": f"js-ts.promise.executor.{metric}",
        "operation": "new Promise",
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


def classify_executor_occurrence(
    masked: str,
    original: str,
    arg_span: tuple[int, int],
) -> set[str]:
    metrics: set[str] = set()
    start, end = trim_span(masked, arg_span[0], arg_span[1])
    executor = parse_arrow_executor(masked, start, end)
    if executor is None:
        executor = parse_function_executor(masked, start, end)

    if executor is None:
        text = masked[start:end].strip()
        if IDENT.match(text):
            metrics.add("identifier_executor_closed_boundary")
        else:
            metrics.add("non_inline_or_unclassified_executor_closed_boundary")
        return metrics

    metrics.add(f"inline_{executor['kind']}_executor")
    if executor["body_kind"] == "expression":
        metrics.add("expression_body_executor")
    else:
        metrics.add("block_body_executor")

    params = executor["params"]
    resolve_name = params[0] if len(params) >= 1 else ""
    reject_name = params[1] if len(params) >= 2 else ""
    body = executor["body"]
    resolve_calls = settlement_calls(masked, original, body, resolve_name)
    reject_calls = settlement_calls(masked, original, body, reject_name)
    settlement_count = len(resolve_calls) + len(reject_calls)
    settlement_names = {name for name in (resolve_name, reject_name) if name}

    if resolve_calls:
        metrics.add("resolve_callback_call_present")
    if reject_calls:
        metrics.add("reject_callback_call_present")
    if settlement_count == 0:
        metrics.add("no_settlement_call_found_closed_boundary")
    if settlement_count > 1:
        metrics.add("multiple_settlement_calls_closed_boundary")
    if resolve_calls and reject_calls:
        metrics.add("mixed_resolve_reject_calls_closed_boundary")

    body_text = masked[body[0] : body[1]]
    if re.search(r"\bthrow\b", body_text):
        metrics.add("throw_to_rejection_closed_boundary")
    if ASYNC_OR_TIMER.search(body_text):
        metrics.add("async_or_timer_inside_executor_closed_boundary")
    if body_has_extra_calls(masked, body, settlement_names):
        metrics.add("side_effect_call_inside_executor_closed_boundary")

    for call in resolve_calls:
        metrics.add(f"resolve_payload_{call['payload']}")
        if call["payload"] != "scalar_non_thenable":
            metrics.add("resolve_thenable_assimilation_closed_boundary")
    for call in reject_calls:
        metrics.add(f"reject_payload_{call['payload']}")

    safe_single_resolve = (
        len(resolve_calls) == 1
        and not reject_calls
        and resolve_calls[0]["payload"] == "scalar_non_thenable"
        and is_exact_single_statement(masked, body, resolve_calls[0])
        and "throw_to_rejection_closed_boundary" not in metrics
        and "async_or_timer_inside_executor_closed_boundary" not in metrics
        and "side_effect_call_inside_executor_closed_boundary" not in metrics
    )
    if safe_single_resolve:
        metrics.add("direct_single_resolve_scalar_upper_bound")

    direct_single_reject = (
        len(reject_calls) == 1
        and not resolve_calls
        and reject_calls[0]["payload"] == "scalar_non_thenable"
        and is_exact_single_statement(masked, body, reject_calls[0])
        and "throw_to_rejection_closed_boundary" not in metrics
        and "async_or_timer_inside_executor_closed_boundary" not in metrics
        and "side_effect_call_inside_executor_closed_boundary" not in metrics
    )
    if direct_single_reject:
        metrics.add("direct_single_reject_scalar_upper_bound")

    return metrics


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
            for match in PROMISE_CONSTRUCTOR.finditer(masked):
                record_metric("all_constructor_calls", repo_id, rel, counts, repo_counts, file_counts)
                close = find_matching(masked, match.end() - 1, "(", ")")
                if close is None:
                    record_metric(
                        "unbalanced_constructor_call_closed_boundary",
                        repo_id,
                        rel,
                        counts,
                        repo_counts,
                        file_counts,
                    )
                    continue
                constructor_args = split_top_level_spans(masked, match.end(), close)
                if not constructor_args:
                    record_metric(
                        "missing_executor_closed_boundary",
                        repo_id,
                        rel,
                        counts,
                        repo_counts,
                        file_counts,
                    )
                    continue
                for metric in classify_executor_occurrence(masked, original, constructor_args[0]):
                    record_metric(metric, repo_id, rel, counts, repo_counts, file_counts)
                if len(constructor_args) > 1:
                    record_metric(
                        "extra_constructor_args_closed_boundary",
                        repo_id,
                        rel,
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


def recall_loss_summary(path: str) -> dict[str, Any]:
    report = json.loads(Path(path).read_text())
    relevant = [
        item
        for item in report.get("by_obligation", [])
        if item.get("obligation_family") == "executor-callback"
        or item.get("obligation_subreason", "").startswith("promise-executor")
    ]
    return {
        "report": path,
        "summary": report.get("summary", {}),
        "soundness_gate": report.get("soundness_gate", {}),
        "executor_obligations": relevant,
    }


def build_report(args: argparse.Namespace) -> dict[str, Any]:
    audit = load_boundary_audit_module()
    corpus = count_corpus(args, audit)
    counts = corpus["counts"]
    return {
        "report_kind": "promise-executor-boundary-audit",
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
                "all `new Promise(...)` constructor settlement recovery",
                "executor timing and synchronous construction effects",
                "resolve/reject callback identity and single-settlement precedence",
                "throw-to-rejection and throw-after-settlement ordering",
                "possible thenable assimilation from resolved payloads",
                "executor body side effects and timer/scheduler callbacks",
                "sync payload equivalence",
            ],
        },
        "policy": {
            "opened_exact_admission": False,
            "selector_only_admission": False,
            "raw_source_snippets_included": False,
            "semantic_admission_delta": 0,
            "note": "This artifact prices executor shapes and hard negatives before exact constructor admission.",
        },
        "corpus_pricing": corpus,
        "expected_recall_delta": {
            "this_slice": 0,
            "direct_single_resolve_scalar_upper_bound": counts.get(
                "direct_single_resolve_scalar_upper_bound", 0
            ),
            "direct_single_reject_scalar_upper_bound": counts.get(
                "direct_single_reject_scalar_upper_bound", 0
            ),
            "note": "Upper bounds are lexical only. A future exact slice still needs dependency-backed constructor evidence, executor timing proof, callback identity proof, settlement precedence, throw-to-rejection modeling, and non-thenable payload proof.",
        },
        "hard_negative_inventory": [
            "new Promise(resolve => resolve(value)) must not converge with a synchronous payload",
            "multiple settlement calls must not recover the later settlement",
            "throw after a prior settlement must not overwrite the settled channel",
            "timer/scheduler settlement inside the executor must not converge with direct Promise.resolve",
            "resolved non-scalar or untyped payloads stay closed behind possible thenable assimilation",
            "identifier or otherwise non-inline executors stay closed without callback body evidence",
        ],
        "next_exact_admission_requirements": [
            "unshadowed global `Promise` constructor occurrence proof",
            "inline executor body with explicit resolve/reject parameter identity",
            "single settlement call whose observed channel is unambiguous",
            "throw-to-rejection and throw-after-settlement precedence represented in the value graph",
            "executor callback effects reported separately from Promise settlement value",
            "resolved payload proven non-thenable-safe before fulfillment recovery",
            "result preserved behind a Promise boundary, never erased to the sync payload",
            "focused hard negatives and local `crates` gate before opening any exact constructor subset",
        ],
        "current_recall_loss": recall_loss_summary(args.recall_loss_report),
        "regenerate": [
            "cargo run -q -p nose-cli -- verify crates --max-violations 0 --recall-loss-report target/recall-loss.issue-602-promise-executor.crates.json",
            "python3 scripts/promise-executor-slice-audit.py --recall-loss-report target/recall-loss.issue-602-promise-executor.crates.json --output target/promise-executor-boundary-audit.v1.json",
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
