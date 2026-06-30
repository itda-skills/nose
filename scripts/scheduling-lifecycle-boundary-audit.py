#!/usr/bin/env python3
"""Build the #602 scheduling, aggregate, cancellation, and lifecycle audit.

This is a lexical source-prevalence report, not semantic proof. It prices the
remaining #602 boundary surfaces across the pinned 120-repo corpus and can attach
the current local recall-loss rollups when a report is provided.
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
DEFAULT_OUTPUT = "target/scheduling-lifecycle-boundary-audit-602.v1.json"
DEFAULT_GENERATED_ON = "2026-06-29"

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

LANG_EXTS = {
    "c": {".c", ".h", ".cc", ".cpp", ".hpp"},
    "go": {".go"},
    "java": {".java"},
    "javascript-typescript": {
        ".cjs",
        ".cts",
        ".js",
        ".jsx",
        ".mjs",
        ".mts",
        ".svelte",
        ".ts",
        ".tsx",
        ".vue",
    },
    "python": {".py"},
    "ruby": {".rb"},
    "rust": {".rs"},
    "swift": {".swift"},
}


@dataclass(frozen=True)
class Pattern:
    language: str
    surface: str
    operation: str
    obligation_family: str
    obligation_subreason: str
    boundary: str
    note: str
    regex: re.Pattern[str]
    status: str = "closed-boundary"


PATTERNS: tuple[Pattern, ...] = (
    Pattern("javascript-typescript", "js-ts.async.await", "await", "scheduling-boundary", "async-await-scheduling-contract-missing", "await scheduling", "await is scheduling and thenable assimilation, not sync value equivalence", re.compile(r"\bawait\b")),
    Pattern("javascript-typescript", "js-ts.async.function", "async function", "scheduling-boundary", "async-function-scheduling-contract-missing", "async function scheduling", "async functions have scheduling and rejection boundaries even without explicit await", re.compile(r"\basync\s+(?:function\b|[A-Za-z_$]|\([^)]*\)\s*=>)")),
    Pattern("javascript-typescript", "js-ts.promise.executor", "new Promise", "executor-callback", "promise-executor-timing-contract-missing", "executor timing", "new Promise needs executor timing, resolve/reject callback, and throw-to-rejection contracts", re.compile(r"\bnew\s+Promise\s*(?:<[^;\n(){}]*>)?\s*\(")),
    Pattern("javascript-typescript", "js-ts.promise.aggregate", "Promise.all", "success-error-result-channel", "promise-aggregate-all-fulfilled-contract-missing", "all-fulfilled aggregate", "Promise.all needs ordered all-fulfilled value and first rejection semantics", re.compile(r"\bPromise\s*\.\s*all\s*(?:<[^;\n(){}]*>)?\s*\(")),
    Pattern("javascript-typescript", "js-ts.promise.aggregate", "Promise.race", "cancellation-liveness-boundary", "promise-aggregate-first-settled-contract-missing", "first-settled aggregate", "Promise.race needs first-settled ordering and liveness proof", re.compile(r"\bPromise\s*\.\s*race\s*(?:<[^;\n(){}]*>)?\s*\(")),
    Pattern("javascript-typescript", "js-ts.promise.aggregate", "Promise.allSettled", "success-error-result-channel", "promise-aggregate-all-settled-contract-missing", "all-settled aggregate", "Promise.allSettled needs settled-record channel and shape proof", re.compile(r"\bPromise\s*\.\s*allSettled\s*(?:<[^;\n(){}]*>)?\s*\(")),
    Pattern("javascript-typescript", "js-ts.promise.aggregate", "Promise.any", "success-error-result-channel", "promise-aggregate-first-fulfilled-contract-missing", "first-fulfilled aggregate", "Promise.any needs first-fulfilled and AggregateError rejection proof", re.compile(r"\bPromise\s*\.\s*any\s*(?:<[^;\n(){}]*>)?\s*\(")),
    Pattern("javascript-typescript", "js-ts.promise.scheduler", "scheduler.wait", "scheduling-boundary", "scheduler-wait-timing-contract-missing", "scheduler wait", "scheduler.wait has timer and AbortSignal rejection boundaries", re.compile(r"\bscheduler\s*\.\s*wait\s*\(")),
    Pattern("javascript-typescript", "js-ts.promise.scheduler", "scheduler.yield", "scheduling-boundary", "scheduler-yield-microtask-order-contract-missing", "scheduler yield", "scheduler.yield needs microtask/order proof", re.compile(r"\bscheduler\s*\.\s*yield\s*\(")),
    Pattern("javascript-typescript", "js-ts.promise.interval", "setInterval", "lifecycle-materialization-boundary", "interval-async-iteration-lifecycle-contract-missing", "interval lifecycle", "setInterval and timers interval streams have repeated emission, cancellation, and liveness semantics", re.compile(r"(?<![A-Za-z0-9_$])setInterval\s*\(")),
    Pattern("javascript-typescript", "js-ts.cancellation.abort", "AbortController/AbortSignal", "cancellation-liveness-boundary", "abort-signal-cancellation-contract-missing", "cancellation signal", "AbortSignal/AbortController can change scheduling and rejection outcomes", re.compile(r"\b(?:AbortController|AbortSignal)\b")),
    Pattern("python", "python.async.await", "await", "scheduling-boundary", "async-await-scheduling-contract-missing", "await scheduling", "Python await has coroutine scheduling and exception channel semantics", re.compile(r"\bawait\b")),
    Pattern("python", "python.async.function", "async def", "scheduling-boundary", "async-function-scheduling-contract-missing", "async function scheduling", "async def creates coroutine protocol boundaries", re.compile(r"\basync\s+def\b")),
    Pattern("python", "python.async.iteration", "async for/with", "lifecycle-materialization-boundary", "python-async-iterator-lifecycle-contract-missing", "async iterator lifecycle", "async for/with needs awaitable lifecycle and cleanup proof", re.compile(r"\basync\s+(?:for|with)\b")),
    Pattern("python", "python.asyncio.task", "asyncio.create_task/ensure_future", "scheduling-boundary", "task-spawn-scheduling-contract-missing", "asyncio task spawn", "asyncio task APIs create scheduler, cancellation, and handle lifecycle boundaries", re.compile(r"\basyncio\s*\.\s*(?:create_task|ensure_future)\s*\(")),
    Pattern("python", "python.asyncio.sleep", "asyncio.sleep", "scheduling-boundary", "timer-scheduling-contract-missing", "asyncio timer", "asyncio sleep creates a timer-backed scheduling boundary", re.compile(r"\basyncio\s*\.\s*sleep\s*\(")),
    Pattern("python", "python.asyncio.gather", "asyncio.gather", "success-error-result-channel", "async-aggregate-all-completion-contract-missing", "asyncio all-completion aggregate", "asyncio gather needs all-completion, result-channel, cancellation, and exception semantics", re.compile(r"\basyncio\s*\.\s*gather\s*\(")),
    Pattern("python", "python.asyncio.wait", "asyncio.wait", "success-error-result-channel", "async-aggregate-completion-contract-missing", "asyncio completion aggregate", "asyncio wait needs completion-selection, result-channel, cancellation, and exception semantics", re.compile(r"\basyncio\s*\.\s*wait\s*\(")),
    Pattern("python", "python.generator.yield", "yield", "lifecycle-materialization-boundary", "generator-yield-lifecycle-contract-missing", "generator lifecycle", "yield has suspension and iterator lifecycle semantics", re.compile(r"\byield(?:\s+from)?\b")),
    Pattern("rust", "rust.async.await", ".await", "scheduling-boundary", "async-await-scheduling-contract-missing", "future await", "Rust .await polls a Future and must keep wake/scheduling effects explicit", re.compile(r"\.\s*await\b")),
    Pattern("rust", "rust.async.function", "async fn", "scheduling-boundary", "async-function-scheduling-contract-missing", "async function scheduling", "async fn creates a suspended async function boundary", re.compile(r"\basync\s+fn\b")),
    Pattern("rust", "rust.async.block", "async block", "scheduling-boundary", "async-block-scheduling-contract-missing", "async block construction", "async blocks create suspended async boundaries", re.compile(r"\basync\s+(?:move\b|\{)")),
    Pattern("rust", "rust.async.spawn", "tokio/async-std spawn", "scheduling-boundary", "task-spawn-scheduling-contract-missing", "task spawn", "async spawn APIs introduce scheduler, cancellation, and join-handle boundaries", re.compile(r"\b(?:tokio(?:::task)?|async_std::task)\s*::\s*spawn(?:_blocking)?\s*\(")),
    Pattern("rust", "rust.async.join", "tokio/futures/futures_util join/try_join", "success-error-result-channel", "async-aggregate-all-completion-contract-missing", "future all-completion aggregate", "qualified runtime join style macros need all-completion result-channel proof", re.compile(r"\b(?:tokio|futures|futures_util)::(?:join|try_join)!\s*\(")),
    Pattern("rust", "rust.async.select", "tokio/futures/futures_util select", "cancellation-liveness-boundary", "async-aggregate-first-completion-contract-missing", "future first-completion aggregate", "qualified runtime select style macros need first-completion, cancellation, and result-channel proof", re.compile(r"\b(?:tokio|futures|futures_util)::select!\s*\(")),
    Pattern("go", "go.concurrent.goroutine", "go statement", "scheduling-boundary", "goroutine-scheduling-contract-missing", "goroutine scheduling", "go statements spawn concurrent execution", re.compile(r"\bgo\s+[A-Za-z_{(]")),
    Pattern("go", "go.concurrent.defer", "defer statement", "lifecycle-materialization-boundary", "defer-lifecycle-ordering-contract-missing", "defer lifecycle", "defer has scope-exit ordering and panic interaction semantics", re.compile(r"\bdefer\s+")),
    Pattern("go", "go.channel.send_receive", "channel send/receive", "channel-boundary", "channel-send-receive-protocol-contract-missing", "channel protocol", "channel send/receive has blocking and synchronization semantics", re.compile(r"<-")),
    Pattern("go", "go.channel.select", "select", "channel-boundary", "channel-select-protocol-contract-missing", "channel select", "select has readiness, default, and scheduling semantics", re.compile(r"\bselect\s*\{")),
    Pattern("java", "java.future.completable", "CompletableFuture", "success-error-result-channel", "java-completable-future-channel-contract-missing", "future channel", "CompletableFuture needs success/error channel and scheduling proof", re.compile(r"\bCompletableFuture\b")),
    Pattern("java", "java.future.executor", "Executor/Future", "scheduling-boundary", "java-executor-scheduling-contract-missing", "executor scheduling", "Executor/Future APIs introduce scheduler and lifecycle boundaries", re.compile(r"\b(?:ExecutorService|Executor|Future|ScheduledFuture)\b")),
    Pattern("java", "java.stream.lifecycle", "stream/parallelStream", "lifecycle-materialization-boundary", "java-stream-lifecycle-contract-missing", "stream lifecycle", "Java streams need lazy/eager lifecycle and terminal materialization proof", re.compile(r"\.\s*(?:stream|parallelStream)\s*\(")),
    Pattern("swift", "swift.async.await", "await", "scheduling-boundary", "async-await-scheduling-contract-missing", "await scheduling", "Swift await has task scheduling and actor/lifetime boundaries", re.compile(r"\bawait\b")),
    Pattern("swift", "swift.async.function", "async", "scheduling-boundary", "async-function-scheduling-contract-missing", "async function scheduling", "Swift async surfaces create task/future-like protocol boundaries", re.compile(r"\basync\b")),
    Pattern("swift", "swift.error.throws", "throws/try", "exception-channel", "swift-throws-exception-channel-contract-missing", "throws channel", "Swift throws/try is an explicit error channel", re.compile(r"\b(?:throws|try)\b")),
    Pattern("swift", "swift.task.spawn", "Task", "scheduling-boundary", "task-spawn-scheduling-contract-missing", "task scheduling", "Task APIs introduce scheduler, cancellation, and handle lifecycle boundaries", re.compile(r"\bTask(?:\s*\.\s*detached)?\s*\{")),
    Pattern("ruby", "ruby.thread.fiber", "Thread/Fiber", "scheduling-boundary", "ruby-thread-fiber-scheduling-contract-missing", "thread/fiber scheduling", "Thread/Fiber APIs create scheduler and lifecycle boundaries", re.compile(r"\b(?:Thread|Fiber)\s*\.\s*(?:new|schedule)\b")),
    Pattern("ruby", "ruby.exception", "raise/rescue", "exception-channel", "ruby-exception-channel-contract-missing", "exception channel", "raise/rescue changes error channels and non-local control", re.compile(r"\b(?:raise|rescue)\b")),
    Pattern("ruby", "ruby.generator.yield", "yield", "callback-demand-effect", "ruby-yield-callback-demand-effect-contract-missing", "block callback", "Ruby yield invokes a block with demand/effect obligations", re.compile(r"\byield\b")),
    Pattern("c", "c.thread.pthread", "pthread_create", "scheduling-boundary", "c-pthread-scheduling-contract-missing", "thread scheduling", "pthread_create introduces thread scheduling and lifetime boundaries", re.compile(r"\bpthread_create\s*\(")),
    Pattern("c", "c.nonlocal_jump", "setjmp/longjmp", "exception-channel", "c-nonlocal-jump-contract-missing", "non-local jump", "setjmp/longjmp is non-local control flow, not ordinary return", re.compile(r"\b(?:setjmp|longjmp)\s*\(")),
)


PYTHON_ASYNCIO_ALIAS_TASK = Pattern(
    "python",
    "python.asyncio.alias.task",
    "import asyncio as alias; alias.create_task/ensure_future",
    "scheduling-boundary",
    "task-spawn-scheduling-contract-missing",
    "asyncio alias task spawn",
    "import-backed asyncio aliases create the same scheduler, cancellation, and handle lifecycle boundaries as asyncio.*",
    re.compile(r"(?!x)x"),
    "reporting-supported-closed-boundary",
)
PYTHON_ASYNCIO_ALIAS_SLEEP = Pattern(
    "python",
    "python.asyncio.alias.sleep",
    "import asyncio as alias; alias.sleep",
    "scheduling-boundary",
    "timer-scheduling-contract-missing",
    "asyncio alias timer",
    "import-backed asyncio aliases create timer-backed scheduling boundaries",
    re.compile(r"(?!x)x"),
    "reporting-supported-closed-boundary",
)
PYTHON_ASYNCIO_ALIAS_GATHER = Pattern(
    "python",
    "python.asyncio.alias.gather",
    "import asyncio as alias; alias.gather",
    "success-error-result-channel",
    "async-aggregate-all-completion-contract-missing",
    "asyncio alias all-completion aggregate",
    "import-backed asyncio aliases need all-completion, result-channel, cancellation, and exception semantics",
    re.compile(r"(?!x)x"),
    "reporting-supported-closed-boundary",
)
PYTHON_ASYNCIO_ALIAS_WAIT = Pattern(
    "python",
    "python.asyncio.alias.wait",
    "import asyncio as alias; alias.wait",
    "success-error-result-channel",
    "async-aggregate-completion-contract-missing",
    "asyncio alias completion aggregate",
    "import-backed asyncio aliases need completion-selection, result-channel, cancellation, and exception semantics",
    re.compile(r"(?!x)x"),
    "reporting-supported-closed-boundary",
)
PYTHON_ASYNCIO_IMPORTED_TASK = Pattern(
    "python",
    "python.asyncio.imported.task",
    "from asyncio import create_task/ensure_future; binding",
    "scheduling-boundary",
    "task-spawn-scheduling-contract-missing",
    "imported asyncio task spawn",
    "import-backed asyncio task bindings create the same scheduler, cancellation, and handle lifecycle boundaries as asyncio.*",
    re.compile(r"(?!x)x"),
    "reporting-supported-closed-boundary",
)
PYTHON_ASYNCIO_IMPORTED_SLEEP = Pattern(
    "python",
    "python.asyncio.imported.sleep",
    "from asyncio import sleep; binding",
    "scheduling-boundary",
    "timer-scheduling-contract-missing",
    "imported asyncio timer",
    "import-backed asyncio sleep bindings create timer-backed scheduling boundaries",
    re.compile(r"(?!x)x"),
    "reporting-supported-closed-boundary",
)
PYTHON_ASYNCIO_IMPORTED_GATHER = Pattern(
    "python",
    "python.asyncio.imported.gather",
    "from asyncio import gather; binding",
    "success-error-result-channel",
    "async-aggregate-all-completion-contract-missing",
    "imported asyncio all-completion aggregate",
    "import-backed asyncio gather bindings need all-completion, result-channel, cancellation, and exception semantics",
    re.compile(r"(?!x)x"),
    "reporting-supported-closed-boundary",
)
PYTHON_ASYNCIO_IMPORTED_WAIT = Pattern(
    "python",
    "python.asyncio.imported.wait",
    "from asyncio import wait; binding",
    "success-error-result-channel",
    "async-aggregate-completion-contract-missing",
    "imported asyncio completion aggregate",
    "import-backed asyncio wait bindings need completion-selection, result-channel, cancellation, and exception semantics",
    re.compile(r"(?!x)x"),
    "reporting-supported-closed-boundary",
)
RUST_IMPORTED_ASYNC_SPAWN = Pattern(
    "rust",
    "rust.async.spawn.imported",
    "use runtime::spawn; spawn",
    "scheduling-boundary",
    "task-spawn-scheduling-contract-missing",
    "imported task spawn",
    "import-backed tokio/async-std spawn bindings introduce scheduler, cancellation, and join-handle boundaries",
    re.compile(r"(?!x)x"),
    "reporting-supported-closed-boundary",
)
RUST_IMPORTED_ASYNC_JOIN = Pattern(
    "rust",
    "rust.async.join.imported",
    "use runtime::join; join!",
    "success-error-result-channel",
    "async-aggregate-all-completion-contract-missing",
    "imported future all-completion aggregate",
    "import-backed tokio/futures/futures_util join-style macros need all-completion result-channel proof",
    re.compile(r"(?!x)x"),
    "reporting-supported-closed-boundary",
)
RUST_IMPORTED_ASYNC_SELECT = Pattern(
    "rust",
    "rust.async.select.imported",
    "use runtime::select; select!",
    "cancellation-liveness-boundary",
    "async-aggregate-first-completion-contract-missing",
    "imported future first-completion aggregate",
    "import-backed tokio/futures/futures_util select macros need first-completion, cancellation, and result-channel proof",
    re.compile(r"(?!x)x"),
    "reporting-supported-closed-boundary",
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", default=DEFAULT_MANIFEST)
    parser.add_argument("--repos-root", default=DEFAULT_REPOS_ROOT)
    parser.add_argument("--output", default=DEFAULT_OUTPUT)
    parser.add_argument("--generated-on", default=DEFAULT_GENERATED_ON)
    parser.add_argument("--recall-loss-report", default=None)
    return parser.parse_args()


def load_repos(manifest: Path) -> list[dict[str, Any]]:
    return json.loads(manifest.read_text()).get("repositories", [])


def language_for_path(path: Path) -> str | None:
    for language, exts in LANG_EXTS.items():
        if path.suffix in exts:
            return language
    return None


def source_files(repo_root: Path) -> list[Path]:
    if not repo_root.exists():
        return []
    files: list[Path] = []
    for path in repo_root.rglob("*"):
        if any(part in SKIP_DIRS for part in path.parts):
            continue
        if path.is_file() and language_for_path(path):
            files.append(path)
    return files


def mask_comments_and_strings(text: str) -> str:
    chars = list(text)
    i = 0
    while i < len(chars):
        ch = chars[i]
        nxt = chars[i + 1] if i + 1 < len(chars) else ""
        if ch in {"'", '"', "`"}:
            quote = ch
            chars[i] = " "
            i += 1
            escaped = False
            while i < len(chars):
                cur = chars[i]
                if cur == "\n" and quote != "`":
                    break
                chars[i] = "\n" if cur == "\n" else " "
                if cur == quote and not escaped:
                    i += 1
                    break
                escaped = cur == "\\" and not escaped
                if cur != "\\":
                    escaped = False
                i += 1
            continue
        if ch == "/" and nxt == "/":
            while i < len(chars) and chars[i] != "\n":
                chars[i] = " "
                i += 1
            continue
        if ch == "/" and nxt == "*":
            chars[i] = chars[i + 1] = " "
            i += 2
            while i + 1 < len(chars) and not (chars[i] == "*" and chars[i + 1] == "/"):
                chars[i] = "\n" if chars[i] == "\n" else " "
                i += 1
            if i + 1 < len(chars):
                chars[i] = chars[i + 1] = " "
                i += 2
            continue
        if ch == "#":
            while i < len(chars) and chars[i] != "\n":
                chars[i] = " "
                i += 1
            continue
        i += 1
    return "".join(chars)


def count_file(text: str, language: str) -> dict[Pattern, int]:
    masked = mask_comments_and_strings(text)
    counts: dict[Pattern, int] = {}
    for pattern in PATTERNS:
        if pattern.language != language:
            continue
        count = sum(1 for _ in pattern.regex.finditer(masked))
        if count:
            counts[pattern] = count
    if language == "python":
        counts.update(python_asyncio_alias_counts(masked))
        counts.update(python_asyncio_imported_counts(masked))
    elif language == "rust":
        counts.update(rust_imported_async_runtime_counts(masked))
    return counts


def python_asyncio_alias_counts(text: str) -> dict[Pattern, int]:
    aliases = python_asyncio_aliases(text)
    if not aliases:
        return {}
    counts: dict[Pattern, int] = {}
    count_by_methods(
        counts,
        PYTHON_ASYNCIO_ALIAS_TASK,
        text,
        aliases,
        ("create_task", "ensure_future"),
        ".",
    )
    count_by_methods(counts, PYTHON_ASYNCIO_ALIAS_SLEEP, text, aliases, ("sleep",), ".")
    count_by_methods(counts, PYTHON_ASYNCIO_ALIAS_GATHER, text, aliases, ("gather",), ".")
    count_by_methods(counts, PYTHON_ASYNCIO_ALIAS_WAIT, text, aliases, ("wait",), ".")
    return counts


def python_asyncio_aliases(text: str) -> set[str]:
    aliases: set[str] = set()
    for match in re.finditer(r"(?m)^\s*import\s+([^\n]+)", text):
        for part in match.group(1).split(","):
            pieces = part.strip().split()
            if len(pieces) == 3 and pieces[0] == "asyncio" and pieces[1] == "as":
                alias = pieces[2]
                if re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", alias) and alias != "asyncio":
                    aliases.add(alias)
    return aliases


def python_asyncio_imported_counts(text: str) -> dict[Pattern, int]:
    bindings = python_asyncio_imported_bindings(text)
    if not bindings:
        return {}
    counts: dict[Pattern, int] = {}
    count_bindings(
        counts,
        PYTHON_ASYNCIO_IMPORTED_TASK,
        text,
        bindings_for_python(bindings, ("create_task", "ensure_future")),
        "(",
    )
    count_bindings(
        counts,
        PYTHON_ASYNCIO_IMPORTED_SLEEP,
        text,
        bindings_for_python(bindings, ("sleep",)),
        "(",
    )
    count_bindings(
        counts,
        PYTHON_ASYNCIO_IMPORTED_GATHER,
        text,
        bindings_for_python(bindings, ("gather",)),
        "(",
    )
    count_bindings(
        counts,
        PYTHON_ASYNCIO_IMPORTED_WAIT,
        text,
        bindings_for_python(bindings, ("wait",)),
        "(",
    )
    return counts


def python_asyncio_imported_bindings(text: str) -> dict[str, set[str]]:
    bindings: dict[str, set[str]] = defaultdict(set)
    for match in re.finditer(r"(?m)^\s*from\s+asyncio\s+import\s+([^\n]+)", text):
        for part in match.group(1).split(","):
            parsed = python_imported_name(part)
            if not parsed:
                continue
            exported, local = parsed
            if exported in {"create_task", "ensure_future", "sleep", "gather", "wait"}:
                bindings[exported].add(local)
    return bindings


def python_imported_name(part: str) -> tuple[str, str] | None:
    pieces = part.strip().split()
    if len(pieces) == 1:
        exported = local = pieces[0]
    elif len(pieces) == 3 and pieces[1] == "as":
        exported = pieces[0]
        local = pieces[2]
    else:
        return None
    if re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", exported) and re.fullmatch(
        r"[A-Za-z_][A-Za-z0-9_]*",
        local,
    ):
        return exported, local
    return None


def bindings_for_python(bindings: dict[str, set[str]], targets: tuple[str, ...]) -> set[str]:
    out: set[str] = set()
    for target in targets:
        out.update(bindings.get(target, set()))
    return out


def rust_imported_async_runtime_counts(text: str) -> dict[Pattern, int]:
    bindings = rust_imported_runtime_bindings(text)
    if not bindings:
        return {}
    counts: dict[Pattern, int] = {}
    spawn_bindings = bindings_for(
        bindings,
        {
            ("tokio", "spawn"),
            ("tokio::task", "spawn"),
            ("tokio::task", "spawn_blocking"),
            ("async_std::task", "spawn"),
            ("async_std::task", "spawn_blocking"),
        },
    )
    join_bindings = bindings_for(
        bindings,
        {
            ("tokio", "join"),
            ("tokio", "try_join"),
            ("futures", "join"),
            ("futures", "try_join"),
            ("futures_util", "join"),
            ("futures_util", "try_join"),
        },
    )
    select_bindings = bindings_for(
        bindings,
        {
            ("tokio", "select"),
            ("futures", "select"),
            ("futures_util", "select"),
        },
    )
    count_bindings(counts, RUST_IMPORTED_ASYNC_SPAWN, text, spawn_bindings, "(")
    count_bindings(counts, RUST_IMPORTED_ASYNC_JOIN, text, join_bindings, "!")
    count_bindings(counts, RUST_IMPORTED_ASYNC_SELECT, text, select_bindings, "!")
    return counts


def count_by_methods(
    counts: dict[Pattern, int],
    pattern: Pattern,
    text: str,
    names: set[str],
    methods: tuple[str, ...],
    separator: str,
) -> None:
    method_pattern = "|".join(re.escape(method) for method in methods)
    total = 0
    for name in names:
        total += len(
            re.findall(
                rf"\b{re.escape(name)}\s*{re.escape(separator)}\s*(?:{method_pattern})\s*\(",
                text,
            )
        )
    if total:
        counts[pattern] = total


def count_bindings(
    counts: dict[Pattern, int],
    pattern: Pattern,
    text: str,
    bindings: set[str],
    call_marker: str,
) -> None:
    total = 0
    for binding in bindings:
        if call_marker == "!":
            total += len(re.findall(rf"\b{re.escape(binding)}\s*!\s*\(", text))
        else:
            total += len(re.findall(rf"\b{re.escape(binding)}\s*\(", text))
    if total:
        counts[pattern] = total


def bindings_for(
    bindings: dict[tuple[str, str], set[str]],
    targets: set[tuple[str, str]],
) -> set[str]:
    out: set[str] = set()
    for target in targets:
        out.update(bindings.get(target, set()))
    return out


def rust_imported_runtime_bindings(text: str) -> dict[tuple[str, str], set[str]]:
    bindings: dict[tuple[str, str], set[str]] = defaultdict(set)
    for match in re.finditer(r"\buse\s+([^;]+);", text, re.DOTALL):
        body = " ".join(match.group(1).split())
        if "*" in body:
            continue
        for module, exported, local in rust_use_bindings(body):
            if rust_runtime_import_target(module, exported):
                bindings[(module, exported)].add(local)
    return bindings


def rust_use_bindings(body: str) -> list[tuple[str, str, str]]:
    if "{" in body or "}" in body:
        return rust_brace_use_bindings(body)
    parsed = parse_rust_use_item(body)
    if not parsed:
        return []
    path, local = parsed
    split = split_rust_path_for_binding(path)
    if not split:
        return []
    module, exported = split
    return [(module, exported, local or exported)]


def rust_brace_use_bindings(body: str) -> list[tuple[str, str, str]]:
    open_idx = body.find("{")
    close_idx = body.rfind("}")
    if open_idx < 0 or close_idx <= open_idx:
        return []
    items = body[open_idx + 1 : close_idx]
    if "{" in items or "}" in items or body[close_idx + 1 :].strip():
        return []
    prefix = body[:open_idx].strip().removesuffix("::").strip()
    if not prefix or prefix.startswith(("self", "super")):
        return []
    bindings: list[tuple[str, str, str]] = []
    for raw_item in items.split(","):
        parsed = parse_rust_use_item(raw_item)
        if not parsed:
            continue
        path, local = parsed
        if path == "self":
            continue
        split = split_rust_path_tail(path)
        if not split:
            continue
        path_prefix, exported = split
        module = f"{prefix}::{path_prefix}" if path_prefix else prefix
        bindings.append((module, exported, local or exported))
    return bindings


def parse_rust_use_item(item: str) -> tuple[str, str | None] | None:
    item = item.strip()
    if not item or any(token in item for token in ("{", "}", "*")):
        return None
    if " as " in item:
        path, local = item.split(" as ", 1)
        path = path.strip()
        local = local.strip()
        if not path or not local:
            return None
        return path, local
    return item, None


def split_rust_path_for_binding(path: str) -> tuple[str, str] | None:
    if "::" not in path:
        return None
    module, exported = path.rsplit("::", 1)
    module = module.strip()
    exported = exported.strip()
    if not module or not exported or module.startswith(("self", "super")):
        return None
    return module, exported


def split_rust_path_tail(path: str) -> tuple[str | None, str] | None:
    path = path.strip()
    if not path:
        return None
    if "::" not in path:
        return None, path
    path_prefix, exported = path.rsplit("::", 1)
    if not exported:
        return None
    return path_prefix, exported


def rust_runtime_import_target(module: str, exported: str) -> bool:
    return (module, exported) in {
        ("tokio", "spawn"),
        ("tokio::task", "spawn"),
        ("tokio::task", "spawn_blocking"),
        ("async_std::task", "spawn"),
        ("async_std::task", "spawn_blocking"),
        ("tokio", "join"),
        ("tokio", "try_join"),
        ("futures", "join"),
        ("futures", "try_join"),
        ("futures_util", "join"),
        ("futures_util", "try_join"),
        ("tokio", "select"),
        ("futures", "select"),
        ("futures_util", "select"),
    }


def summarize(args: argparse.Namespace) -> dict[str, Any]:
    repos = load_repos(Path(args.manifest))
    by_pattern: dict[Pattern, Counter[str]] = defaultdict(Counter)
    file_counts: dict[Pattern, Counter[str]] = defaultdict(Counter)
    language_counts: Counter[str] = Counter()
    family_counts: Counter[str] = Counter()

    for repo in repos:
        repo_id = repo["id"]
        root = Path(args.repos_root) / repo_id
        for path in source_files(root):
            language = language_for_path(path)
            if language is None:
                continue
            try:
                text = path.read_text(errors="ignore")
            except OSError:
                continue
            rel = str(path.relative_to(root))
            for pattern, count in count_file(text, language).items():
                by_pattern[pattern][repo_id] += count
                file_counts[pattern][f"{repo_id}/{rel}"] += count
                language_counts[language] += count
                family_counts[pattern.obligation_family] += count

    surfaces = []
    for pattern, repo_counts in by_pattern.items():
        occurrences = sum(repo_counts.values())
        if occurrences == 0:
            continue
        surfaces.append(
            {
                "language": pattern.language,
                "surface": pattern.surface,
                "operation": pattern.operation,
                "status": pattern.status,
                "boundary": pattern.boundary,
                "obligation_family": pattern.obligation_family,
                "obligation_subreason": pattern.obligation_subreason,
                "occurrences": occurrences,
                "repos": len(repo_counts),
                "top_repos": [
                    {"repo": repo, "occurrences": count}
                    for repo, count in repo_counts.most_common(8)
                ],
                "top_files": [
                    {"path": path, "occurrences": count}
                    for path, count in file_counts[pattern].most_common(8)
                ],
                "note": pattern.note,
            }
        )
    surfaces.sort(key=lambda item: (-item["occurrences"], item["language"], item["operation"]))

    total_occurrences = sum(language_counts.values())
    report: dict[str, Any] = {
        "report_kind": "scheduling-lifecycle-boundary-audit",
        "schema_version": 1,
        "generated_on": args.generated_on,
        "tracking_issue": {
            "number": 602,
            "url": "https://github.com/corca-ai/nose/issues/602",
        },
        "policy": {
            "semantic_admission_delta": 0,
            "source_prevalence_only": True,
            "raw_source_snippets_included": False,
            "note": "Counts price boundary surfaces; they do not prove exact semantics.",
        },
        "summary": {
            "repos_in_manifest": len(repos),
            "total_source_prevalence": total_occurrences,
            "languages": dict(sorted(language_counts.items())),
            "obligation_families": dict(sorted(family_counts.items())),
        },
        "surfaces": surfaces,
        "recommended_order": recommended_order(surfaces),
        "hard_negative_inventory": hard_negative_inventory(),
        "current_recall_loss": current_recall_loss(args.recall_loss_report),
        "regenerate": [regenerate_command(args)],
    }
    return report


def regenerate_command(args: argparse.Namespace) -> str:
    parts = ["python3", "scripts/scheduling-lifecycle-boundary-audit.py"]
    if args.manifest != DEFAULT_MANIFEST:
        parts.extend(["--manifest", args.manifest])
    if args.repos_root != DEFAULT_REPOS_ROOT:
        parts.extend(["--repos-root", args.repos_root])
    if args.recall_loss_report:
        parts.extend(["--recall-loss-report", args.recall_loss_report])
    if args.output != DEFAULT_OUTPUT:
        parts.extend(["--output", args.output])
    if args.generated_on != DEFAULT_GENERATED_ON:
        parts.extend(["--generated-on", args.generated_on])
    return " ".join(parts)


def recommended_order(surfaces: list[dict[str, Any]]) -> list[dict[str, Any]]:
    priority = {
        "promise-aggregate-all-fulfilled-contract-missing": 1,
        "promise-aggregate-first-settled-contract-missing": 2,
        "promise-executor-timing-contract-missing": 3,
        "abort-signal-cancellation-contract-missing": 4,
        "interval-async-iteration-lifecycle-contract-missing": 5,
        "goroutine-scheduling-contract-missing": 6,
        "task-spawn-scheduling-contract-missing": 7,
        "async-aggregate-all-completion-contract-missing": 8,
        "async-aggregate-first-completion-contract-missing": 9,
        "async-aggregate-completion-contract-missing": 10,
        "java-completable-future-channel-contract-missing": 11,
    }
    surface_priority = {
        "swift.async.await": 12,
    }
    candidates = [
        item
        for item in surfaces
        if item["obligation_subreason"] in priority
        or item["surface"] in surface_priority
    ]

    def item_priority(item: dict[str, Any]) -> int:
        subreason = item["obligation_subreason"]
        if subreason in priority:
            return priority[subreason]
        return surface_priority[item["surface"]]

    candidates.sort(
        key=lambda item: (
            item_priority(item),
            -item["occurrences"],
            item["language"],
        )
    )
    return [
        {
            "rank": idx + 1,
            "language": item["language"],
            "operation": item["operation"],
            "obligation_family": item["obligation_family"],
            "obligation_subreason": item["obligation_subreason"],
            "occurrences": item["occurrences"],
            "repos": item["repos"],
            "why": recommended_reason(item),
            "next_action": "reporting-only split and hard-negative expansion before exact admission",
        }
        for idx, item in enumerate(candidates[:10])
    ]


def recommended_reason(item: dict[str, Any]) -> str:
    subreason = item["obligation_subreason"]
    if subreason.startswith("promise-aggregate"):
        return "Promise aggregate semantics are already corpus-priced and have adjacent first/all-settled hard negatives."
    if subreason == "promise-executor-timing-contract-missing":
        return "new Promise is high-risk because timing, resolve/reject callback, and throw-to-rejection behavior interact."
    if "abort" in subreason:
        return "Cancellation appears across JS/TS scheduling APIs and must stay separate from fulfillment/rejection recovery."
    if "interval" in subreason:
        return "Repeated emission/liveness needs lifecycle proof before interval streams can be compared exactly."
    return "High-prevalence boundary surface with reusable obligation vocabulary."


def hard_negative_inventory() -> list[dict[str, Any]]:
    return [
        {
            "class": "thenable assimilation and custom Promise-like receivers",
            "evidence": "crates/nose-cli/tests/cli/semantic_boundaries.rs::query_mode_semantic_rejects_unproven_js_promise_protocol_convergence",
            "status": "mapped-existing",
        },
        {
            "class": "first-settled versus all-settled aggregate semantics",
            "evidence": "crates/nose-cli/tests/cli/semantic_boundaries.rs::query_mode_semantic_rejects_unproven_js_promise_protocol_convergence",
            "status": "expanded-this-slice",
        },
        {
            "class": "executor callback timing and thrown executor errors",
            "evidence": "crates/nose-cli/src/verify_admission/runtime_boundary.rs::promise_constructor_missing_evidence_splits_executor_obligations",
            "status": "reporting-only-this-slice",
        },
        {
            "class": "scheduler/microtask ordering versus synchronous evaluation",
            "evidence": "crates/nose-cli/src/verify_admission/runtime_boundary.rs::scheduler_and_interval_calls_report_timing_and_lifecycle_obligations",
            "status": "reporting-only-this-slice",
        },
        {
            "class": "interval stream liveness/cardinality",
            "evidence": "crates/nose-cli/tests/cli/commands/recall_loss_report.rs::recall_loss_report_splits_promise_protocol_boundaries",
            "status": "reporting-only-this-slice",
        },
        {
            "class": "cross-language lifecycle one-shot/reusable/materialized distinctions",
            "evidence": "docs/scheduling-channel-callback-obligations-594.md",
            "status": "mapped-doc-policy",
        },
        {
            "class": "Python imported asyncio bindings shadowed by parameters, assignments, nested imports, or project-local asyncio modules",
            "evidence": "crates/nose-cli/src/verify_admission/runtime_boundary/tests/async_runtime/imported_bindings.rs::non_js_async_runtime_imported_bindings_reject_local_shadows and ::non_js_async_runtime_context_rejects_project_local_imported_bindings",
            "status": "expanded-this-slice",
        },
        {
            "class": "Rust brace/direct-imported runtime bindings shadowed by parameters, lets, local macros, block scopes, other modules, or project-local runtime roots",
            "evidence": "crates/nose-cli/src/verify_admission/runtime_boundary/tests/async_runtime/imported_bindings.rs::non_js_async_runtime_imported_bindings_reject_rust_shadows_and_scopes and ::non_js_async_runtime_context_rejects_project_local_imported_bindings",
            "status": "expanded-this-slice",
        },
    ]


def current_recall_loss(path: str | None) -> dict[str, Any] | None:
    if not path:
        return None
    report_path = Path(path)
    if not report_path.exists():
        return {"report": path, "status": "missing"}
    report = json.loads(report_path.read_text())
    relevant_interpretable = relevant_recall_loss_obligations(report.get("by_obligation", []))
    oracle = report.get("oracle_exclusions", {})
    relevant_oracle_exclusions = relevant_recall_loss_obligations(
        oracle.get("by_obligation", [])
    )
    return {
        "report": path,
        "summary": report.get("summary", {}),
        "soundness_gate": report.get("soundness_gate", {}),
        "relevant_obligations": relevant_interpretable,
        "relevant_oracle_exclusion_obligations": relevant_oracle_exclusions,
    }


def relevant_recall_loss_obligations(items: list[dict[str, Any]]) -> list[dict[str, Any]]:
    relevant = []
    for item in items:
        family = item.get("obligation_family", "")
        subreason = item.get("obligation_subreason", "")
        if family in {
            "scheduling-boundary",
            "channel-boundary",
            "executor-callback",
            "success-error-result-channel",
            "cancellation-liveness-boundary",
            "lifecycle-materialization-boundary",
            "exception-channel",
        } or any(key in subreason for key in ("promise", "scheduler", "channel", "goroutine", "defer", "interval")):
            relevant.append(item)
    return relevant


def main() -> None:
    args = parse_args()
    report = summarize(args)
    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")


if __name__ == "__main__":
    main()
