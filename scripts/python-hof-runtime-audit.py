#!/usr/bin/env python3
"""Audit Python HOF/runtime attribution prevalence in the pinned corpus.

The report uses Python's AST to separate:

- bare builtin/HOF calls that need unshadowed runtime attribution;
- calls shadowed by local parameters, assignments, definitions, or imports;
- `itertools`/`functools` calls through simple module aliases or direct imports.

This is a pricing report, not semantic proof.
"""

from __future__ import annotations

import argparse
import ast
import json
import warnings
from collections import Counter, defaultdict
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any


warnings.filterwarnings("ignore", category=SyntaxWarning)


DEFAULT_MANIFEST = "bench/goldens/corpus.json"
DEFAULT_REPOS_ROOT = "bench/repos"
DEFAULT_OUTPUT = "target/python-hof-runtime-audit.v3.json"
HIGH_VOLUME_PROCESSING_THRESHOLD = 5000

SKIP_DIRS = {
    ".git",
    ".mypy_cache",
    ".pytest_cache",
    ".tox",
    ".venv",
    "__pycache__",
    "build",
    "dist",
    "node_modules",
    "site-packages",
    "target",
    "venv",
}

BUILTIN_TARGETS = {
    "all",
    "any",
    "enumerate",
    "filter",
    "frozenset",
    "list",
    "map",
    "max",
    "min",
    "range",
    "reversed",
    "set",
    "sorted",
    "sum",
    "tuple",
    "zip",
}

SUPPORTED_BUILTINS: dict[str, tuple[str, str, str]] = {
    "all": ("supported", "terminal-reduction", "terminal iterator builtin"),
    "any": ("supported", "terminal-reduction", "terminal iterator builtin"),
    "enumerate": ("supported", "lazy-iterator-producer", "lazy iterator producer"),
    "filter": ("supported-partial", "hof-lambda", "HOF requires lambda and iterable-source proof"),
    "frozenset": (
        "supported-partial",
        "materializer",
        "materializes admitted lazy iterator producers distinctly",
    ),
    "list": (
        "supported-partial",
        "materializer",
        "materializes admitted lazy iterator producers",
    ),
    "map": ("supported-partial", "hof-lambda", "HOF requires lambda and iterable-source proof"),
    "range": ("supported", "range-producer", "range builtin"),
    "set": (
        "supported-partial",
        "materializer",
        "materializes admitted lazy iterator producers distinctly",
    ),
    "sum": ("supported", "reduction", "sum builtin over one iterable"),
    "tuple": (
        "supported-partial",
        "materializer",
        "materializes admitted lazy iterator producers distinctly",
    ),
    "zip": ("supported", "lazy-iterator-producer", "lazy iterator producer"),
}

UNSUPPORTED_BUILTINS: dict[str, tuple[str, str]] = {
    "max": ("ordering-reduction", "ordering reduction is not the iterator-HOF capability"),
    "min": ("ordering-reduction", "ordering reduction is not the iterator-HOF capability"),
    "reversed": ("ordering-view", "reverse-order iterator needs ordering/sequence proof"),
    "sorted": ("ordering-materializer", "sorted materializer needs ordering/key semantics"),
}

MODULE_METHOD_BOUNDARIES: dict[tuple[str, str], tuple[str, str]] = {
    ("functools", "cache"): ("decorator-runtime", "decorator/runtime cache semantics"),
    ("functools", "cached_property"): ("descriptor-runtime", "descriptor/cache semantics"),
    ("functools", "cmp_to_key"): ("ordering-callback", "comparator adapter"),
    ("functools", "lru_cache"): ("decorator-runtime", "decorator/runtime cache semantics"),
    ("functools", "partial"): ("callable-binding", "partial application identity"),
    ("functools", "partialmethod"): ("callable-binding", "partial method identity"),
    ("functools", "reduce"): ("reduction-callback", "callback reduction"),
    ("functools", "singledispatch"): ("dispatch-runtime", "runtime dispatch registry"),
    ("functools", "total_ordering"): ("class-decorator", "generated ordering methods"),
    ("functools", "update_wrapper"): ("decorator-runtime", "decorator metadata/runtime wrapper"),
    ("functools", "wraps"): ("decorator-runtime", "decorator metadata/runtime wrapper"),
    ("itertools", "accumulate"): ("stateful-iterator", "stateful reduction iterator"),
    ("itertools", "batched"): ("chunk-iterator", "chunking iterator"),
    ("itertools", "chain"): ("iterator-composition", "iterator concatenation"),
    ("itertools", "combinations"): ("combinatoric-iterator", "combinatoric iterator"),
    ("itertools", "combinations_with_replacement"): (
        "combinatoric-iterator",
        "combinatoric iterator",
    ),
    ("itertools", "compress"): ("predicate-iterator", "selector-driven filtering"),
    ("itertools", "count"): ("infinite-iterator", "infinite iterator"),
    ("itertools", "cycle"): ("infinite-iterator", "infinite iterator"),
    ("itertools", "dropwhile"): ("predicate-iterator", "callback predicate iterator"),
    ("itertools", "filterfalse"): ("predicate-iterator", "negated predicate iterator"),
    ("itertools", "groupby"): ("stateful-iterator", "stateful grouping iterator"),
    ("itertools", "islice"): ("slice-iterator", "iterator slicing"),
    ("itertools", "pairwise"): ("window-iterator", "sliding window iterator"),
    ("itertools", "permutations"): ("combinatoric-iterator", "combinatoric iterator"),
    ("itertools", "product"): ("combinatoric-iterator", "cartesian product iterator"),
    ("itertools", "repeat"): ("repeat-iterator", "repeat iterator"),
    ("itertools", "starmap"): ("hof-callback", "callback mapping over unpacked tuples"),
    ("itertools", "takewhile"): ("predicate-iterator", "callback predicate iterator"),
    ("itertools", "tee"): ("iterator-lifecycle", "shared iterator buffering/lifecycle"),
    ("itertools", "zip_longest"): ("zip-variant", "zip with fill value"),
}

PROCESSING_DECISIONS: dict[str, dict[str, Any]] = {
    "python-materializer-domain-proof": {
        "sequence": 1,
        "status": "processed-boundary-split",
        "semantic_admission_delta": 0,
        "strictness_effect": "unchanged",
        "decision": (
            "Keep Python materializers gated by existing LibraryApi occurrence, "
            "unshadowed builtin proof, source-iterator provenance, and result-domain proof."
        ),
        "closed_boundary": (
            "Lexical list/set/tuple/frozenset frequency alone is not semantic proof; "
            "shadowed names, non-iterator inputs, and missing materializer evidence stay closed."
        ),
        "next_metric": (
            "Measure admitted materializer calls by result domain and the remaining "
            "source-iterator-provenance misses in recall-loss reports."
        ),
        "subgroups": [
            {
                "surface": "builtins.list",
                "result_domain": "Collection",
                "admission_policy": "unshadowed builtin plus admitted source iterator",
            },
            {
                "surface": "builtins.set",
                "result_domain": "Set",
                "admission_policy": "unshadowed builtin plus admitted source iterator",
            },
            {
                "surface": "builtins.tuple",
                "result_domain": "Collection",
                "admission_policy": "unshadowed builtin plus admitted source iterator",
            },
            {
                "surface": "builtins.frozenset",
                "result_domain": "Set",
                "admission_policy": "unshadowed builtin plus admitted source iterator",
            },
        ],
    },
}


@dataclass
class ScopeInfo:
    bound: set[str] = field(default_factory=set)
    module_aliases: dict[str, str] = field(default_factory=dict)
    direct_imports: dict[str, tuple[str, str]] = field(default_factory=dict)


class ScopeBindingCollector(ast.NodeVisitor):
    def __init__(self) -> None:
        self.info = ScopeInfo()

    def visit_FunctionDef(self, node: ast.FunctionDef) -> None:
        self.info.bound.add(node.name)

    def visit_AsyncFunctionDef(self, node: ast.AsyncFunctionDef) -> None:
        self.info.bound.add(node.name)

    def visit_ClassDef(self, node: ast.ClassDef) -> None:
        self.info.bound.add(node.name)

    def visit_Lambda(self, node: ast.Lambda) -> None:
        self._bind_args(node.args)

    def visit_arguments(self, node: ast.arguments) -> None:
        self._bind_args(node)

    def visit_Import(self, node: ast.Import) -> None:
        for alias in node.names:
            local = alias.asname or alias.name.split(".", 1)[0]
            self.info.bound.add(local)
            if alias.name in {"itertools", "functools"}:
                self.info.module_aliases[local] = alias.name

    def visit_ImportFrom(self, node: ast.ImportFrom) -> None:
        if node.module not in {"itertools", "functools"}:
            for alias in node.names:
                self.info.bound.add(alias.asname or alias.name)
            return
        for alias in node.names:
            if alias.name == "*":
                continue
            local = alias.asname or alias.name
            self.info.bound.add(local)
            self.info.direct_imports[local] = (node.module, alias.name)

    def visit_Assign(self, node: ast.Assign) -> None:
        for target in node.targets:
            self._bind_target(target)

    def visit_AnnAssign(self, node: ast.AnnAssign) -> None:
        self._bind_target(node.target)

    def visit_AugAssign(self, node: ast.AugAssign) -> None:
        self._bind_target(node.target)

    def visit_For(self, node: ast.For) -> None:
        self._bind_target(node.target)

    def visit_AsyncFor(self, node: ast.AsyncFor) -> None:
        self._bind_target(node.target)

    def visit_With(self, node: ast.With) -> None:
        for item in node.items:
            if item.optional_vars is not None:
                self._bind_target(item.optional_vars)

    def visit_AsyncWith(self, node: ast.AsyncWith) -> None:
        for item in node.items:
            if item.optional_vars is not None:
                self._bind_target(item.optional_vars)

    def visit_ExceptHandler(self, node: ast.ExceptHandler) -> None:
        if node.name:
            self.info.bound.add(node.name)
        for stmt in node.body:
            self.visit(stmt)

    def visit_If(self, node: ast.If) -> None:
        for stmt in node.body:
            self.visit(stmt)
        for stmt in node.orelse:
            self.visit(stmt)

    def visit_While(self, node: ast.While) -> None:
        for stmt in node.body:
            self.visit(stmt)
        for stmt in node.orelse:
            self.visit(stmt)

    def visit_Try(self, node: ast.Try) -> None:
        for stmt in node.body:
            self.visit(stmt)
        for handler in node.handlers:
            self.visit(handler)
        for stmt in node.orelse:
            self.visit(stmt)
        for stmt in node.finalbody:
            self.visit(stmt)

    def visit_Match(self, node: ast.Match) -> None:
        for case in node.cases:
            self._bind_pattern(case.pattern)
            for stmt in case.body:
                self.visit(stmt)

    def visit_NamedExpr(self, node: ast.NamedExpr) -> None:
        self._bind_target(node.target)

    def _bind_args(self, node: ast.arguments) -> None:
        args = list(node.posonlyargs) + list(node.args) + list(node.kwonlyargs)
        if node.vararg:
            args.append(node.vararg)
        if node.kwarg:
            args.append(node.kwarg)
        for arg in args:
            self.info.bound.add(arg.arg)

    def _bind_target(self, target: ast.AST) -> None:
        if isinstance(target, ast.Name):
            self.info.bound.add(target.id)
        elif isinstance(target, (ast.Tuple, ast.List)):
            for element in target.elts:
                self._bind_target(element)

    def _bind_pattern(self, pattern: ast.AST) -> None:
        if isinstance(pattern, ast.MatchAs):
            if pattern.name:
                self.info.bound.add(pattern.name)
            if pattern.pattern is not None:
                self._bind_pattern(pattern.pattern)
        elif isinstance(pattern, ast.MatchStar):
            if pattern.name:
                self.info.bound.add(pattern.name)
        elif isinstance(pattern, ast.MatchMapping):
            if pattern.rest:
                self.info.bound.add(pattern.rest)
            for subpattern in pattern.patterns:
                self._bind_pattern(subpattern)
        elif isinstance(pattern, ast.MatchClass):
            for subpattern in pattern.patterns:
                self._bind_pattern(subpattern)
            for subpattern in pattern.kwd_patterns:
                self._bind_pattern(subpattern)
        elif isinstance(pattern, (ast.MatchSequence, ast.MatchOr)):
            for subpattern in pattern.patterns:
                self._bind_pattern(subpattern)


class AuditVisitor(ast.NodeVisitor):
    def __init__(self, repo: str, file_path: Path) -> None:
        self.repo = repo
        self.file_path = file_path
        self.scopes: list[ScopeInfo] = []
        self.calls: list[tuple[str, str, str, str | None]] = []

    def visit_Module(self, node: ast.Module) -> None:
        self._with_scope(node.body, None)

    def visit_FunctionDef(self, node: ast.FunctionDef) -> None:
        self._visit_function_prelude(node)
        self._with_scope(node.body, node.args)

    def visit_AsyncFunctionDef(self, node: ast.AsyncFunctionDef) -> None:
        self._visit_function_prelude(node)
        self._with_scope(node.body, node.args)

    def visit_Lambda(self, node: ast.Lambda) -> None:
        self._with_scope([node.body], node.args)

    def visit_ClassDef(self, node: ast.ClassDef) -> None:
        for decorator in node.decorator_list:
            self.visit(decorator)
        for base in node.bases:
            self.visit(base)
        self._with_scope(node.body, None)

    def visit_Call(self, node: ast.Call) -> None:
        self._record_call(node)
        self.generic_visit(node)

    def _visit_function_prelude(self, node: ast.FunctionDef | ast.AsyncFunctionDef) -> None:
        for decorator in node.decorator_list:
            self.visit(decorator)
        for default in node.args.defaults:
            self.visit(default)
        for default in node.args.kw_defaults:
            if default is not None:
                self.visit(default)
        if node.returns is not None:
            self.visit(node.returns)

    def _with_scope(self, body: list[ast.stmt] | list[ast.expr], args: ast.arguments | None) -> None:
        collector = ScopeBindingCollector()
        if args is not None:
            collector.visit(args)
        for stmt in body:
            if isinstance(stmt, ast.stmt):
                collector.visit(stmt)
        self.scopes.append(collector.info)
        for stmt in body:
            self.visit(stmt)
        self.scopes.pop()

    def _record_call(self, node: ast.Call) -> None:
        func = node.func
        if isinstance(func, ast.Name):
            imported = self._direct_import(func.id)
            if imported is not None:
                module, name = imported
                self.calls.append((module, name, "direct-import", None))
                return
            if func.id in BUILTIN_TARGETS:
                shadow = self._shadow_reason(func.id)
                self.calls.append(("builtins", func.id, "bare", shadow))
        elif isinstance(func, ast.Attribute) and isinstance(func.value, ast.Name):
            module = self._module_alias(func.value.id)
            if module in {"itertools", "functools"}:
                self.calls.append((module, func.attr, "module-attribute", None))

    def _shadow_reason(self, name: str) -> str | None:
        for scope in reversed(self.scopes):
            if name in scope.direct_imports:
                return "direct-import"
            if name in scope.bound:
                return "local-binding"
        return None

    def _direct_import(self, name: str) -> tuple[str, str] | None:
        for scope in reversed(self.scopes):
            if name in scope.direct_imports:
                return scope.direct_imports[name]
            if name in scope.bound:
                return None
        return None

    def _module_alias(self, name: str) -> str | None:
        for scope in reversed(self.scopes):
            if name in scope.module_aliases:
                return scope.module_aliases[name]
            if name in scope.bound:
                return None
        return None


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", default=DEFAULT_MANIFEST)
    parser.add_argument("--repos-root", default=DEFAULT_REPOS_ROOT)
    parser.add_argument("--output", default=DEFAULT_OUTPUT)
    return parser.parse_args()


def python_repos(manifest: Path) -> list[str]:
    data = json.loads(manifest.read_text())
    return [
        repo["id"]
        for repo in data.get("repositories", [])
        if repo.get("primary_language") == "Python"
    ]


def python_files(root: Path) -> list[Path]:
    if not root.exists():
        return []
    files: list[Path] = []
    for path in root.rglob("*.py"):
        if any(part in SKIP_DIRS for part in path.parts):
            continue
        files.append(path)
    return files


def classify(module: str, name: str, shadow: str | None) -> tuple[str, str, str]:
    if shadow is not None:
        return (
            "runtime-attribution-required",
            shadow,
            "bare builtin spelling is shadowed in lexical scope",
        )
    if module == "builtins":
        supported = SUPPORTED_BUILTINS.get(name)
        if supported:
            return supported
        boundary, note = UNSUPPORTED_BUILTINS.get(
            name,
            ("unknown-builtin-boundary", "observed builtin not classified by this audit"),
        )
        return "unsupported", boundary, note
    boundary, note = MODULE_METHOD_BOUNDARIES.get(
        (module, name),
        ("unknown-module-boundary", "observed module function not classified by this audit"),
    )
    return "unsupported", boundary, note


def next_work_group(
    module: str,
    name: str,
    status: str,
    boundary: str,
) -> tuple[str, str, str] | None:
    if status == "runtime-attribution-required":
        return (
            "python-runtime-attribution",
            "builtin runtime attribution",
            "Do not widen builtin semantics until these lexical shadows stay explicit.",
        )
    if status == "supported-partial" and boundary == "hof-lambda":
        return (
            "python-hof-lambda-proof",
            "HOF callback and iterable-source proof",
            "Existing partial support needs callback identity, demand, and iterable-source proof.",
        )
    if status == "supported-partial" and boundary == "materializer":
        return (
            "python-materializer-domain-proof",
            "collection materializer domain proof",
            "Existing partial support needs source iterator provenance and result-domain proof.",
        )
    if boundary in {
        "ordering-callback",
        "ordering-materializer",
        "ordering-reduction",
        "ordering-view",
    }:
        return (
            "python-ordering-semantics",
            "ordering/key/comparator semantics",
            "Sorted, reversed, min, max, and comparator adapters need ordering obligations.",
        )
    if boundary in {"reduction-callback", "hof-callback"}:
        return (
            "python-callback-reduction",
            "callback reduction contracts",
            "Callback reductions need demand/effect and accumulator semantics.",
        )
    if boundary in {"callable-binding", "decorator-runtime", "descriptor-runtime"}:
        return (
            "python-callable-runtime",
            "callable/decorator runtime identity",
            "Callable wrappers and decorators are runtime identity surfaces, not pure value factories.",
        )
    if boundary in {"class-decorator", "dispatch-runtime"}:
        return (
            "python-dynamic-dispatch-runtime",
            "dynamic dispatch/class decorator runtime",
            "Runtime registry or generated-method behavior should remain fail-closed.",
        )
    if boundary in {"combinatoric-iterator"}:
        return (
            "python-combinatoric-iterators",
            "combinatoric iterator shape contracts",
            "Product, permutations, and combinations need iterator shape and cardinality contracts.",
        )
    if module == "itertools":
        return (
            "python-itertools-lifecycle",
            "iterator lifecycle and view contracts",
            "Iterator views need lifecycle, state, and materialization policies before admission.",
        )
    if name in SUPPORTED_BUILTINS:
        return None
    return (
        "python-unclassified-runtime-boundary",
        "unclassified runtime boundary",
        "Observed calls should stay closed until the audit classifies their capability.",
    )


def processed_high_volume_groups(ranked_next_work: list[dict[str, Any]]) -> list[dict[str, Any]]:
    processed: list[dict[str, Any]] = []
    for group in ranked_next_work:
        if group["occurrences"] < HIGH_VOLUME_PROCESSING_THRESHOLD:
            continue
        decision = PROCESSING_DECISIONS.get(group["id"])
        if decision is None:
            decision = {
                "status": "unprocessed-high-volume-group",
                "semantic_admission_delta": 0,
                "strictness_effect": "unchanged",
                "decision": "No processing decision is recorded for this high-volume group yet.",
                "next_metric": "Add a processing decision before using this group for implementation.",
            }
        processed.append(
            {
                "id": group["id"],
                "capability": group["capability"],
                "occurrences": group["occurrences"],
                "repos": group["repos"],
                "top_surfaces": group["top_surfaces"],
                **decision,
            }
        )
    return sorted(processed, key=lambda group: group.get("sequence", 999))


def main() -> int:
    args = parse_args()
    repos_root = Path(args.repos_root)
    call_counts: Counter[tuple[str, str, str, str, str]] = Counter()
    call_repos: dict[tuple[str, str, str, str, str], set[str]] = defaultdict(set)
    call_repo_counts: dict[tuple[str, str, str, str, str], Counter[str]] = defaultdict(Counter)
    call_shape_counts: dict[tuple[str, str, str, str, str], Counter[str]] = defaultdict(Counter)
    next_work_counts: Counter[tuple[str, str, str]] = Counter()
    next_work_repos: dict[tuple[str, str, str], set[str]] = defaultdict(set)
    next_work_surfaces: dict[tuple[str, str, str], Counter[str]] = defaultdict(Counter)
    parse_errors: list[dict[str, str]] = []
    scanned_files = 0

    for repo in python_repos(Path(args.manifest)):
        for path in python_files(repos_root / repo):
            scanned_files += 1
            try:
                text = path.read_text(errors="ignore")
                tree = ast.parse(text, filename=str(path))
            except (OSError, SyntaxError) as exc:
                parse_errors.append({"repo": repo, "file": str(path), "error": str(exc)})
                continue
            visitor = AuditVisitor(repo, path)
            try:
                visitor.visit(tree)
            except RecursionError as exc:
                parse_errors.append({"repo": repo, "file": str(path), "error": str(exc)})
                continue
            for module, name, call_shape, shadow in visitor.calls:
                status, boundary, note = classify(module, name, shadow)
                key = (module, name, status, boundary, note)
                call_counts[key] += 1
                call_repos[key].add(repo)
                call_repo_counts[key][repo] += 1
                call_shape_counts[key][call_shape] += 1
                group = next_work_group(module, name, status, boundary)
                if group is not None:
                    next_work_counts[group] += 1
                    next_work_repos[group].add(repo)
                    next_work_surfaces[group][f"{module}.{name}:{boundary}"] += 1

    rows: list[dict[str, Any]] = []
    for key, occurrences in sorted(call_counts.items(), key=lambda item: (-item[1], item[0])):
        module, name, status, boundary, note = key
        rows.append(
            {
                "module": module,
                "function": name,
                "occurrences": occurrences,
                "repos": len(call_repos[key]),
                "status": status,
                "boundary": boundary,
                "note": note,
                "call_shape_counts": dict(sorted(call_shape_counts[key].items())),
                "top_repos": [
                    {"repo": repo, "occurrences": count}
                    for repo, count in call_repo_counts[key].most_common(8)
                ],
            }
        )

    status_counts = Counter(row["status"] for row in rows for _ in range(row["occurrences"]))
    boundary_counts = Counter(row["boundary"] for row in rows for _ in range(row["occurrences"]))
    module_counts = Counter(row["module"] for row in rows for _ in range(row["occurrences"]))
    ranked_next_work = [
        {
            "id": group_id,
            "capability": capability,
            "occurrences": count,
            "repos": len(next_work_repos[group]),
            "policy": policy,
            "top_surfaces": [
                {"surface": surface, "occurrences": surface_count}
                for surface, surface_count in next_work_surfaces[group].most_common(8)
            ],
        }
        for group, count in sorted(
            next_work_counts.items(), key=lambda item: (-item[1], item[0][0])
        )
        for group_id, capability, policy in [group]
    ]
    report = {
        "report_kind": "python-hof-runtime-audit",
        "schema_version": 3,
        "manifest": args.manifest,
        "repos_root": args.repos_root,
        "scanned_python_repos": len(python_repos(Path(args.manifest))),
        "scanned_python_files": scanned_files,
        "parse_error_count": len(parse_errors),
        "totals": {
            "occurrences": sum(call_counts.values()),
            "supported_or_partial_occurrences": sum(
                row["occurrences"]
                for row in rows
                if row["status"] in {"supported", "supported-partial"}
            ),
            "unsupported_occurrences": sum(
                row["occurrences"] for row in rows if row["status"] == "unsupported"
            ),
            "runtime_attribution_required_occurrences": status_counts.get(
                "runtime-attribution-required", 0
            ),
            "unknown_boundary_occurrences": boundary_counts.get("unknown-module-boundary", 0)
            + boundary_counts.get("unknown-builtin-boundary", 0),
        },
        "status_counts": dict(sorted(status_counts.items())),
        "module_counts": dict(sorted(module_counts.items())),
        "boundary_counts": dict(sorted(boundary_counts.items())),
        "ranked_next_work": ranked_next_work,
        "processing_threshold_occurrences": HIGH_VOLUME_PROCESSING_THRESHOLD,
        "processed_high_volume_groups": processed_high_volume_groups(ranked_next_work),
        "calls": rows,
        "parse_errors": parse_errors[:50],
    }
    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=False) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
