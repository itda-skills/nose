#!/usr/bin/env python3
"""Audit JS/TS builtin and stdlib-like surface prevalence in the pinned corpus.

This is a lexical pricing report, not semantic proof. It masks comments and
strings before counting common JS/TS Array, Map/Set, Object-key, Promise, and
mutation surfaces. Simple receiver hints from local constructors, array
literals, and type annotations are recorded separately so selector-heavy names
like `get` and `map` do not look like proof by themselves.
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
DEFAULT_OUTPUT = "target/js-ts-stdlib-partial-audit.v1.json"
HIGH_VOLUME_PROCESSING_THRESHOLD = 5000

SKIP_DIRS = {
    ".git",
    ".next",
    ".nuxt",
    ".svelte-kit",
    "build",
    "coverage",
    "dist",
    "node_modules",
    "out",
    "target",
    "vendor",
}

JS_TS_EXTS = {".js", ".jsx", ".mjs", ".cjs", ".ts", ".tsx", ".mts", ".cts", ".vue", ".svelte"}

IDENT = r"[A-Za-z_$][A-Za-z0-9_$]*"

STATIC_CALL_RE = re.compile(
    rf"\b(?P<receiver>Array|Object|Promise)\s*\.\s*(?P<method>{IDENT})\s*(?:<[^;\n(){{}}]*>)?\s*\("
)
OBJECT_HASOWN_CALL_RE = re.compile(
    r"\bObject\s*\.\s*prototype\s*\.\s*hasOwnProperty\s*\.\s*call\s*\("
)
CONSTRUCTOR_RE = re.compile(
    rf"\bnew\s+(?P<type>Map|Set|WeakMap|WeakSet|Array|Promise)\s*(?:<[^;\n(){{}}]*>)?\s*\("
)
BARE_CALL_RE = re.compile(
    rf"(?<![.\w$])(?P<function>Map|Set|WeakMap|WeakSet|Array|Promise|Boolean)\s*(?:<[^;\n(){{}}]*>)?\s*\("
)
METHOD_CALL_RE = re.compile(
    rf"(?P<receiver>{IDENT})\s*(?:\?\.|\.)\s*(?P<method>{IDENT})\s*(?:<[^;\n(){{}}]*>)?\s*\("
)
PROPERTY_RE = re.compile(rf"(?P<receiver>{IDENT})\s*(?:\?\.|\.)\s*(?P<property>length|size)\b(?!\s*\()")
AWAIT_RE = re.compile(r"\bawait\b")
ASYNC_FUNCTION_RE = re.compile(r"\basync\s+(?:function\b|[A-Za-z_$]|\([^)]*\)\s*=>)")
YIELD_RE = re.compile(r"\byield\b")

LOCAL_ARRAY_LITERAL_RE = re.compile(
    rf"\b(?:const|let|var)\s+(?P<name>{IDENT})\s*(?::[^=;\n]+)?=\s*\["
)
LOCAL_CONSTRUCTOR_RE = re.compile(
    rf"\b(?:const|let|var)\s+(?P<name>{IDENT})\s*(?::[^=;\n]+)?=\s*new\s+(?P<type>Map|Set|WeakMap|WeakSet|Array|Promise)\b"
)
LOCAL_STATIC_FACTORY_RE = re.compile(
    rf"\b(?:const|let|var)\s+(?P<name>{IDENT})\s*(?::[^=;\n]+)?=\s*(?P<receiver>Array|Promise)\s*\.\s*(?P<method>from|resolve)\s*\("
)
LOCAL_TYPE_RE = re.compile(
    rf"\b(?:const|let|var|function\s+{IDENT}\s*\([^)]*)\s+(?P<name>{IDENT})\s*:\s*(?P<type>[^=;,\n)]+)"
)

METHOD_TARGETS = {
    "add",
    "catch",
    "clear",
    "concat",
    "copyWithin",
    "delete",
    "entries",
    "every",
    "fill",
    "filter",
    "finally",
    "find",
    "findIndex",
    "flat",
    "flatMap",
    "forEach",
    "get",
    "has",
    "includes",
    "indexOf",
    "join",
    "keys",
    "map",
    "pop",
    "push",
    "reduce",
    "reduceRight",
    "reverse",
    "set",
    "shift",
    "slice",
    "some",
    "sort",
    "splice",
    "then",
    "toReversed",
    "toSorted",
    "toSpliced",
    "unshift",
    "values",
    "with",
}

MUTATING_METHODS = {
    "add",
    "clear",
    "copyWithin",
    "delete",
    "fill",
    "pop",
    "push",
    "reverse",
    "set",
    "shift",
    "sort",
    "splice",
    "unshift",
}


@dataclass(frozen=True)
class OperationKey:
    surface: str
    operation: str
    status: str
    boundary: str
    capability: str
    note: str


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", default=DEFAULT_MANIFEST)
    parser.add_argument("--repos-root", default=DEFAULT_REPOS_ROOT)
    parser.add_argument("--output", default=DEFAULT_OUTPUT)
    return parser.parse_args()


def js_ts_repos(manifest: Path) -> list[str]:
    data = json.loads(manifest.read_text())
    return [
        repo["id"]
        for repo in data.get("repositories", [])
        if repo.get("primary_language") in {"JavaScript", "TypeScript"}
    ]


def js_ts_files(root: Path) -> list[Path]:
    if not root.exists():
        return []
    files: list[Path] = []
    for path in root.rglob("*"):
        if any(part in SKIP_DIRS for part in path.parts):
            continue
        if path.suffix in JS_TS_EXTS and path.is_file():
            files.append(path)
    return files


def mask_comments_and_strings(text: str) -> str:
    chars = list(text)
    i = 0
    while i < len(chars):
        ch = chars[i]
        nxt = chars[i + 1] if i + 1 < len(chars) else ""
        if ch == "/" and nxt == "/":
            i = mask_until_newline(chars, i)
            continue
        if ch == "/" and nxt == "*":
            i = mask_until_block_comment_end(chars, i)
            continue
        if ch in {"'", '"', "`"}:
            i = mask_quoted(chars, i, ch)
            continue
        i += 1
    return "".join(chars)


def mask_until_newline(chars: list[str], start: int) -> int:
    i = start
    while i < len(chars) and chars[i] != "\n":
        chars[i] = " "
        i += 1
    return i


def mask_until_block_comment_end(chars: list[str], start: int) -> int:
    i = start
    while i < len(chars):
        if chars[i] != "\n":
            chars[i] = " "
        if i + 1 < len(chars) and chars[i] == "*" and chars[i + 1] == "/":
            chars[i + 1] = " "
            return i + 2
        i += 1
    return i


def mask_quoted(chars: list[str], start: int, quote: str) -> int:
    i = start
    chars[i] = " "
    i += 1
    escaped = False
    while i < len(chars):
        ch = chars[i]
        if ch != "\n":
            chars[i] = " "
        if escaped:
            escaped = False
        elif ch == "\\":
            escaped = True
        elif ch == quote:
            return i + 1
        i += 1
    return i


def language_for_path(path: Path) -> str:
    if path.suffix in {".ts", ".tsx", ".mts", ".cts"}:
        return "typescript"
    if path.suffix in {".vue", ".svelte"}:
        return "embedded-js-ts"
    return "javascript"


def receiver_hints(text: str) -> dict[str, str]:
    hints: dict[str, str] = {}
    for match in LOCAL_ARRAY_LITERAL_RE.finditer(text):
        hints[match.group("name")] = "array-literal-binding"
    for match in LOCAL_CONSTRUCTOR_RE.finditer(text):
        hints[match.group("name")] = constructor_domain(match.group("type"))
    for match in LOCAL_STATIC_FACTORY_RE.finditer(text):
        receiver, method = match.group("receiver"), match.group("method")
        if receiver == "Array" and method == "from":
            hints[match.group("name")] = "array-factory-binding"
        elif receiver == "Promise" and method == "resolve":
            hints[match.group("name")] = "promise-factory-binding"
    for match in LOCAL_TYPE_RE.finditer(text):
        name = match.group("name")
        typ = match.group("type")
        hinted = type_domain_hint(typ)
        if hinted is not None:
            hints.setdefault(name, hinted)
    return hints


def constructor_domain(type_name: str) -> str:
    return {
        "Array": "array-constructor-binding",
        "Map": "map-constructor-binding",
        "Set": "set-constructor-binding",
        "WeakMap": "weak-map-constructor-binding",
        "WeakSet": "weak-set-constructor-binding",
        "Promise": "promise-constructor-binding",
    }[type_name]


def type_domain_hint(typ: str) -> str | None:
    normalized = re.sub(r"\s+", "", typ)
    if re.search(r"\b(?:Readonly)?(?:Map|WeakMap)<", normalized):
        return "map-type-annotation"
    if re.search(r"\b(?:Readonly)?(?:Set|WeakSet)<", normalized):
        return "set-type-annotation"
    if re.search(r"\b(?:Readonly)?Array<", normalized) or "[]" in normalized:
        return "array-type-annotation"
    if re.search(r"\bPromise<", normalized):
        return "promise-type-annotation"
    return None


def classify_static(receiver: str, method: str) -> OperationKey:
    if receiver == "Array" and method == "isArray":
        return OperationKey(
            "js_ts.builtins.array",
            "Array.isArray",
            "supported",
            "admitted-array-guard",
            "array-guard",
            "static global array guard is admitted with unshadowed Array proof",
        )
    if receiver == "Array" and method == "from":
        return OperationKey(
            "js_ts.builtins.array",
            "Array.from",
            "supported-partial",
            "array-from-domain-proof",
            "collection-materializer",
            "Array.from is admitted only with unshadowed Array and supported source proof",
        )
    if receiver == "Object" and method == "keys":
        return OperationKey(
            "js_ts.builtins.object_key_views",
            "Object.keys",
            "supported-partial",
            "object-key-view-proof",
            "map-key-view",
            "Object.keys is admitted only for static object/map-key proof",
        )
    if receiver == "Object" and method == "hasOwn":
        return OperationKey(
            "js_ts.builtins.own_property",
            "Object.hasOwn",
            "supported-partial",
            "own-property-guard-proof",
            "own-property-guard",
            "own-property guard needs unshadowed Object and matching object/key/default proof",
        )
    if receiver == "Promise" and method == "resolve":
        return OperationKey(
            "js_ts.builtins.promise",
            "Promise.resolve",
            "supported-partial",
            "promise-factory-proof",
            "promise-protocol",
            "Promise.resolve needs unshadowed Promise and safe settled-value proof",
        )
    boundary, capability, note = {
        ("Array", "of"): (
            "array-factory-shape",
            "collection-factory",
            "Array.of needs element/result-shape proof distinct from Array.from",
        ),
        ("Object", "values"): (
            "object-value-view",
            "map-value-view",
            "Object.values returns values, not key membership proof",
        ),
        ("Object", "entries"): (
            "object-entry-view",
            "map-entry-view",
            "Object.entries needs entry-shape and key/value proof",
        ),
        ("Object", "fromEntries"): (
            "object-from-entries",
            "map-factory",
            "Object.fromEntries needs entry-shape and duplicate-key policy",
        ),
        ("Object", "assign"): (
            "object-mutation-copy",
            "object-runtime-reflection",
            "Object.assign mutates/copies enumerable properties and needs alias/effect proof",
        ),
        ("Object", "create"): (
            "prototype-construction",
            "object-runtime-reflection",
            "Object.create depends on prototype identity and descriptor semantics",
        ),
        ("Object", "defineProperty"): (
            "property-descriptor-effect",
            "object-runtime-reflection",
            "Object.defineProperty mutates descriptors and needs effect/descriptor proof",
        ),
        ("Object", "defineProperties"): (
            "property-descriptor-effect",
            "object-runtime-reflection",
            "Object.defineProperties mutates descriptors and needs effect/descriptor proof",
        ),
        ("Object", "freeze"): (
            "object-mutability-state",
            "object-runtime-reflection",
            "Object.freeze changes mutability state, not plain value shape",
        ),
        ("Object", "isFrozen"): (
            "object-mutability-state",
            "object-runtime-reflection",
            "Object.isFrozen observes mutability state outside current exact object proof",
        ),
        ("Object", "is"): (
            "same-value-comparison",
            "object-runtime-reflection",
            "Object.is uses SameValue semantics distinct from ordinary equality",
        ),
        ("Object", "getPrototypeOf"): (
            "prototype-introspection",
            "object-runtime-reflection",
            "Object.getPrototypeOf observes prototype identity",
        ),
        ("Object", "setPrototypeOf"): (
            "prototype-mutation",
            "object-runtime-reflection",
            "Object.setPrototypeOf mutates prototype identity",
        ),
        ("Object", "getOwnPropertyDescriptor"): (
            "property-descriptor-introspection",
            "object-runtime-reflection",
            "Object.getOwnPropertyDescriptor observes descriptor metadata",
        ),
        ("Object", "getOwnPropertyDescriptors"): (
            "property-descriptor-introspection",
            "object-runtime-reflection",
            "Object.getOwnPropertyDescriptors observes descriptor metadata",
        ),
        ("Object", "getOwnPropertyNames"): (
            "own-property-name-view",
            "object-runtime-reflection",
            "Object.getOwnPropertyNames includes non-enumerable names outside Object.keys proof",
        ),
        ("Object", "getOwnPropertySymbols"): (
            "own-symbol-view",
            "object-runtime-reflection",
            "Object.getOwnPropertySymbols needs symbol-key proof",
        ),
        ("Object", "groupBy"): (
            "grouping-materializer",
            "collection-materializer",
            "Object.groupBy needs callback, grouping key, and result-shape contracts",
        ),
        ("Array", "fromAsync"): (
            "async-array-materializer",
            "promise-protocol",
            "Array.fromAsync needs async iterator, scheduling, and result-domain proof",
        ),
        ("Promise", "all"): (
            "promise-combinator",
            "promise-protocol",
            "Promise.all needs aggregate scheduling, rejection, and result-shape contracts",
        ),
        ("Promise", "allSettled"): (
            "promise-combinator",
            "promise-protocol",
            "Promise.allSettled needs settled-state and result-shape contracts",
        ),
        ("Promise", "any"): (
            "promise-combinator",
            "promise-protocol",
            "Promise.any needs scheduling and aggregate rejection semantics",
        ),
        ("Promise", "race"): (
            "promise-combinator",
            "promise-protocol",
            "Promise.race needs scheduling and first-settled semantics",
        ),
        ("Promise", "reject"): (
            "promise-rejection",
            "promise-protocol",
            "Promise.reject needs error-channel semantics",
        ),
    }.get(
        (receiver, method),
        static_fallback(receiver, method),
    )
    return OperationKey(f"js_ts.builtins.{receiver.lower()}", f"{receiver}.{method}", "unsupported", boundary, capability, note)


def static_fallback(receiver: str, method: str) -> tuple[str, str, str]:
    if receiver == "Object":
        return (
            "object-reflection-boundary",
            "object-runtime-reflection",
            f"Object.{method} is outside the current key-view and own-property guard contracts",
        )
    if receiver == "Array":
        return (
            "array-static-boundary",
            "collection-materializer",
            f"Array.{method} needs a focused array static API contract before admission",
        )
    return (
        "promise-static-boundary",
        "promise-protocol",
        f"Promise.{method} needs a focused Promise protocol contract before admission",
    )


def classify_constructor(type_name: str) -> OperationKey:
    if type_name in {"Map", "Set"}:
        return OperationKey(
            "js_ts.builtins.collection_constructors",
            f"new {type_name}",
            "supported-partial",
            "collection-constructor-proof",
            "collection-factory",
            f"new {type_name} is admitted only with construct syntax and unshadowed global proof",
        )
    boundary, capability, note = {
        "Array": (
            "array-constructor-shape",
            "collection-factory",
            "new Array has length-vs-elements ambiguity and needs shape proof",
        ),
        "Promise": (
            "promise-executor-boundary",
            "promise-protocol",
            "new Promise depends on executor demand, scheduling, and error effects",
        ),
        "WeakMap": (
            "weak-collection-lifetime",
            "collection-factory",
            "WeakMap keys have identity/lifetime obligations outside exact value semantics",
        ),
        "WeakSet": (
            "weak-collection-lifetime",
            "collection-factory",
            "WeakSet values have identity/lifetime obligations outside exact value semantics",
        ),
    }[type_name]
    return OperationKey("js_ts.builtins.collection_constructors", f"new {type_name}", "unsupported", boundary, capability, note)


def classify_bare_call(function: str) -> OperationKey:
    if function == "Boolean":
        return OperationKey(
            "js_ts.builtins.boolean",
            "Boolean",
            "supported",
            "admitted-boolean-coercion",
            "primitive-coercion",
            "Boolean(value) is admitted with unshadowed global proof",
        )
    return OperationKey(
        "js_ts.builtins.collection_constructors",
        function,
        "unsupported",
        "non-construct-call-boundary",
        "collection-factory" if function != "Promise" else "promise-protocol",
        "bare global call is not construct-syntax proof and may be shadowed",
    )


def classify_method(method: str) -> OperationKey | None:
    if method not in METHOD_TARGETS:
        return None
    if method in {"map", "filter", "flatMap"}:
        return OperationKey(
            "js_ts.builtins.array_hof",
            method,
            "supported-partial",
            "array-hof-proof",
            "iterator-hof-materialization",
            "Array HOF needs exact Array receiver, callback identity, and demand/effect proof",
        )
    if method in {"some", "every"}:
        return OperationKey(
            "js_ts.builtins.array_hof",
            method,
            "supported-partial",
            "array-bool-reduction-proof",
            "iterator-hof-materialization",
            "Array boolean reduction needs exact Array receiver and callback proof",
        )
    if method in {"includes", "has"}:
        return OperationKey(
            "js_ts.builtins.membership",
            method,
            "supported-partial",
            "receiver-membership-proof",
            "receiver-domain-proof",
            "membership call needs exact collection/set/map receiver proof",
        )
    if method == "get":
        return OperationKey(
            "js_ts.builtins.map_get",
            method,
            "supported-partial",
            "map-get-proof",
            "receiver-domain-proof",
            "Map.get needs exact Map receiver proof and default/channel handling downstream",
        )
    if method == "keys":
        return OperationKey(
            "js_ts.builtins.map_key_views",
            method,
            "supported-partial",
            "map-key-view-proof",
            "map-key-view",
            "Map.keys needs exact Map receiver proof and Array.from wrapper proof before collection use",
        )
    if method in {"indexOf", "findIndex"}:
        return OperationKey(
            "js_ts.builtins.static_index_membership",
            method,
            "supported-partial",
            "static-index-membership-proof",
            "receiver-domain-proof",
            "static index membership needs static non-float literal collection receiver proof",
        )
    if method == "then":
        return OperationKey(
            "js_ts.builtins.promise",
            method,
            "supported-partial",
            "promise-then-proof",
            "promise-protocol",
            "Promise.then needs exact PromiseLike receiver and supported settled-value proof",
        )
    if method == "test":
        return OperationKey(
            "js_ts.builtins.regex",
            method,
            "supported-partial",
            "regex-literal-proof",
            "regex-protocol",
            "regex test needs regex-literal receiver proof",
        )
    if method in MUTATING_METHODS:
        return OperationKey(
            "js_ts.builtins.mutation",
            method,
            "supported-partial",
            "mutation-effect",
            "effect-preserving-contracts",
            "receiver mutation evidence exists; exact value admission stays closed after mutation",
        )
    boundary, capability, note = {
        "reduce": (
            "reduction-callback",
            "iterator-hof-materialization",
            "reduction needs accumulator, callback demand/effect, and initial-value semantics",
        ),
        "reduceRight": (
            "reduction-callback",
            "iterator-hof-materialization",
            "right-fold reduction needs ordering and accumulator semantics",
        ),
        "forEach": (
            "effect-callback",
            "effect-preserving-contracts",
            "forEach is effect-oriented and needs callback effect proof",
        ),
        "find": (
            "option-producing-hof",
            "iterator-hof-materialization",
            "find needs option/result-channel semantics plus callback proof",
        ),
        "catch": (
            "promise-error-channel",
            "promise-protocol",
            "catch needs rejection-channel and callback demand/effect semantics",
        ),
        "finally": (
            "promise-finalizer-effect",
            "promise-protocol",
            "finally needs finalizer demand/effect and scheduling semantics",
        ),
        "values": (
            "map-value-view",
            "map-value-view",
            "values() is a value iterator/view, not key proof",
        ),
        "entries": (
            "entry-view",
            "map-entry-view",
            "entries() needs entry-shape and lifecycle contracts",
        ),
        "sort": (
            "ordering-mutation",
            "effect-preserving-contracts",
            "sort mutates and needs comparator/order obligations for value semantics",
        ),
        "toSorted": (
            "ordering-materializer",
            "collection-materializer",
            "toSorted copies and orders; it needs result-domain and comparator/order proof",
        ),
        "toReversed": (
            "ordering-materializer",
            "collection-materializer",
            "toReversed copies and reverses order; it needs result-domain/order proof",
        ),
        "toSpliced": (
            "copy-result-domain",
            "collection-materializer",
            "toSpliced copies with shape changes and needs result-domain proof",
        ),
        "with": (
            "copy-result-domain",
            "collection-materializer",
            "with copies with indexed replacement and needs result-domain proof",
        ),
        "slice": (
            "copy-result-domain",
            "collection-materializer",
            "slice returns a view/copy with bounds and result-domain obligations",
        ),
        "concat": (
            "copy-result-domain",
            "collection-materializer",
            "concat needs result-domain and flattening/array-like obligations",
        ),
        "flat": (
            "flatten-materializer",
            "collection-materializer",
            "flat needs depth and nested collection proof",
        ),
        "join": (
            "string-materializer",
            "string-transform",
            "join needs separator and element stringification semantics",
        ),
    }.get(
        method,
        (
            "unknown-boundary",
            "unclassified-method-api",
            "observed method is not classified by this audit",
        ),
    )
    return OperationKey("js_ts.builtins.methods", method, "unsupported", boundary, capability, note)


def classify_property(prop: str) -> OperationKey:
    if prop == "length":
        return OperationKey(
            "js_ts.builtins.cardinality",
            "length",
            "supported-partial",
            "length-property-proof",
            "cardinality-receiver-proof",
            "length is admitted only for exact array/string/property-builtin receiver proof",
        )
    return OperationKey(
        "js_ts.builtins.cardinality",
        "size",
        "unsupported",
        "map-set-size-cardinality",
        "cardinality-receiver-proof",
        "Map/Set size needs collection receiver and mutation/lifecycle proof",
    )


def next_work_group(status: str, boundary: str, capability: str) -> tuple[str, str, str] | None:
    if status == "supported":
        return None
    mapping = {
        "array-hof-proof": (
            "js-ts-array-hof-proof",
            "Array HOF receiver/callback proof",
            "Keep JS/TS Array HOFs exact Array receiver and callback-effect gated.",
        ),
        "array-bool-reduction-proof": (
            "js-ts-array-hof-proof",
            "Array HOF receiver/callback proof",
            "Keep JS/TS Array HOFs exact Array receiver and callback-effect gated.",
        ),
        "map-get-proof": (
            "js-ts-map-set-receiver-proof",
            "Map/Set receiver-domain proof",
            "Map/Set selectors need exact receiver proof, not selector admission.",
        ),
        "receiver-membership-proof": (
            "js-ts-map-set-receiver-proof",
            "Map/Set receiver-domain proof",
            "Map/Set selectors need exact receiver proof, not selector admission.",
        ),
        "map-key-view-proof": (
            "js-ts-map-set-receiver-proof",
            "Map/Set receiver-domain proof",
            "Map/Set selectors need exact receiver proof, not selector admission.",
        ),
        "collection-constructor-proof": (
            "js-ts-collection-constructor-proof",
            "collection constructor proof",
            "new Map/new Set need construct-syntax and unshadowed global proof.",
        ),
        "length-property-proof": (
            "js-ts-cardinality-receiver-proof",
            "cardinality receiver proof",
            "Length property reads need exact receiver proof and property-builtin evidence.",
        ),
        "map-set-size-cardinality": (
            "js-ts-cardinality-receiver-proof",
            "cardinality receiver proof",
            "Map/Set size needs receiver and mutation/lifecycle proof before admission.",
        ),
        "mutation-effect": (
            "js-ts-mutation-effect-contracts",
            "mutation/effect contracts",
            "Mutating JS/TS collection methods should remain effect evidence, not value APIs.",
        ),
        "promise-factory-proof": (
            "js-ts-promise-protocol-proof",
            "Promise protocol proof",
            "Promise factories need unshadowed global and settled-value proof.",
        ),
        "promise-then-proof": (
            "js-ts-promise-protocol-proof",
            "Promise protocol proof",
            "Promise.then needs PromiseLike receiver and demand/effect proof.",
        ),
        "array-from-domain-proof": (
            "js-ts-array-materializer-proof",
            "Array.from materializer/source proof",
            "Array.from needs source-domain and wrapper proof before exact collection use.",
        ),
        "object-key-view-proof": (
            "js-ts-object-key-view-proof",
            "Object key-view proof",
            "Object.keys needs static object key proof before collection membership use.",
        ),
        "own-property-guard-proof": (
            "js-ts-own-property-guard-proof",
            "own-property guard proof",
            "Own-property guards need object/key/default alignment before value admission.",
        ),
        "static-index-membership-proof": (
            "js-ts-static-index-membership-proof",
            "static index membership proof",
            "indexOf/findIndex need static literal receiver and threshold proof.",
        ),
    }
    if boundary in mapping:
        return mapping[boundary]
    if capability == "promise-protocol":
        return (
            "js-ts-promise-protocol-boundaries",
            "Promise async/scheduling boundaries",
            "Async combinators and error channels need scheduling, rejection, and demand/effect contracts.",
        )
    if capability == "collection-materializer":
        return (
            "js-ts-array-materializer-proof",
            "Array/copy materializer proof",
            "Copy/view materializers need result-domain, lifecycle, and shape contracts.",
        )
    if capability == "iterator-hof-materialization":
        return (
            "js-ts-array-hof-boundaries",
            "Array HOF/reduction boundaries",
            "Unsupported HOF/reduction methods need callback demand/effect and result-channel proof.",
        )
    if capability == "map-value-view":
        return (
            "js-ts-map-value-view-boundaries",
            "Map value-view proof",
            "Value views need lifecycle and value-domain contracts before admission.",
        )
    if capability == "map-entry-view":
        return (
            "js-ts-map-entry-view-boundaries",
            "Map entry-view proof",
            "Entry views need entry-shape and lifecycle contracts before admission.",
        )
    if capability == "object-runtime-reflection":
        return (
            "js-ts-object-reflection-boundaries",
            "Object runtime reflection boundaries",
            "Object reflection APIs need descriptor, prototype, mutability, and effect contracts.",
        )
    return (
        f"js-ts-{capability}",
        capability.replace("-", " "),
        "Observed JS/TS surface needs a focused processing decision before implementation.",
    )


PROCESSING_DECISIONS: dict[str, dict[str, Any]] = {
    "js-ts-array-hof-proof": {
        "sequence": 1,
        "status": "processed-existing-contract",
        "semantic_admission_delta": 0,
        "strictness_effect": "unchanged",
        "decision": (
            "Keep map/filter/flatMap/some/every on the existing JS/TS Array HOF rows "
            "with exact Array receiver proof and callback demand/effect proof."
        ),
        "closed_boundary": (
            "Selector-only HOF calls, framework collection methods, and non-lambda or "
            "effectful callbacks stay closed."
        ),
        "next_metric": (
            "Split misses by exact Array receiver proof, callback shape, callback effect, "
            "and terminal materialization proof."
        ),
    },
    "js-ts-map-set-receiver-proof": {
        "sequence": 2,
        "status": "processed-existing-contract",
        "semantic_admission_delta": 0,
        "strictness_effect": "unchanged",
        "decision": (
            "Keep get/has/includes/keys on existing receiver-domain contracts and use "
            "receiver hints only as pricing evidence, not semantic proof."
        ),
        "closed_boundary": (
            "Selector-only get/has/includes/keys, untyped framework objects, and missing "
            "Map/Set/Array receiver proof stay closed."
        ),
        "next_metric": (
            "Measure receiver-domain misses by constructor binding, type annotation, "
            "literal binding, imported snapshot, and unhinted selector."
        ),
    },
    "js-ts-mutation-effect-contracts": {
        "sequence": 3,
        "status": "processed-existing-contract",
        "semantic_admission_delta": 0,
        "strictness_effect": "unchanged",
        "decision": (
            "Treat JS/TS mutating Array/Map/Set methods as receiver-mutation or builder "
            "append effect evidence, not pure value APIs."
        ),
        "closed_boundary": (
            "Mutating calls do not produce exact value equivalence; they close later "
            "receiver assumptions unless a builder append path is explicitly proven."
        ),
        "next_metric": (
            "Track covered receiver-mutation rows, active builder append rows, and "
            "unsupported copy-on-write/order materializers separately."
        ),
    },
    "js-ts-promise-protocol-proof": {
        "sequence": 4,
        "status": "processed-existing-contract",
        "semantic_admission_delta": 0,
        "strictness_effect": "unchanged",
        "decision": (
            "Keep Promise.resolve and then on the existing Promise protocol rows with "
            "unshadowed global, PromiseLike receiver, and settled-value proof."
        ),
        "closed_boundary": (
            "Promise combinators, catch/finally, new Promise executors, unsupported "
            "thenables, and await/plain-value convergence stay closed."
        ),
        "next_metric": (
            "Split misses by Promise factory proof, PromiseLike receiver proof, "
            "continuation callback effect, scheduling, and rejection channel."
        ),
    },
    "js-ts-promise-protocol-boundaries": {
        "sequence": 4,
        "status": "processed-closed-boundary",
        "semantic_admission_delta": 0,
        "strictness_effect": "unchanged",
        "decision": (
            "Keep await, async functions, Promise combinators, new Promise, catch/finally, "
            "and unsupported thenables as explicit closed protocol boundaries."
        ),
        "closed_boundary": (
            "Frequency of async syntax is not Promise equivalence proof; scheduling, "
            "exception, cancellation, aggregate result, and callback effect obligations "
            "must be modeled before any convergence opens."
        ),
        "next_metric": (
            "Split future recall-loss misses by await/plain-value convergence, "
            "Promise combinator result shape, executor effect, rejection channel, and "
            "thenable receiver proof."
        ),
    },
    "js-ts-cardinality-receiver-proof": {
        "sequence": 5,
        "status": "processed-boundary-split",
        "semantic_admission_delta": 0,
        "strictness_effect": "unchanged",
        "decision": (
            "Keep .length on property-builtin receiver proof and leave .size closed "
            "until Map/Set cardinality and mutation/lifecycle proof exist."
        ),
        "closed_boundary": (
            "Selector-only length/size reads, custom size getters, and mutable Map/Set "
            "cardinality assumptions stay closed."
        ),
        "next_metric": (
            "Track cardinality misses by exact array/string receiver, Map/Set receiver, "
            "and mutable receiver evidence."
        ),
    },
}


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


def add_row(
    rows: Counter[OperationKey],
    row_repos: dict[OperationKey, Counter[str]],
    receiver_hint_counts: dict[OperationKey, Counter[str]],
    key: OperationKey,
    repo: str,
    receiver_hint: str | None = None,
) -> None:
    rows[key] += 1
    row_repos[key][repo] += 1
    if receiver_hint is not None:
        receiver_hint_counts[key][receiver_hint] += 1


def main() -> int:
    args = parse_args()
    manifest = Path(args.manifest)
    repos_root = Path(args.repos_root)
    repos = js_ts_repos(manifest)
    rows: Counter[OperationKey] = Counter()
    row_repos: dict[OperationKey, Counter[str]] = defaultdict(Counter)
    receiver_hint_counts: dict[OperationKey, Counter[str]] = defaultdict(Counter)
    language_counts: Counter[str] = Counter()
    scanned_files = 0

    for repo in repos:
        for path in js_ts_files(repos_root / repo):
            scanned_files += 1
            language_counts[language_for_path(path)] += 1
            try:
                raw_text = path.read_text(errors="ignore")
            except OSError:
                continue
            text = mask_comments_and_strings(raw_text)
            hints = receiver_hints(text)

            for match in STATIC_CALL_RE.finditer(text):
                add_row(rows, row_repos, receiver_hint_counts, classify_static(match.group("receiver"), match.group("method")), repo)
            for _match in OBJECT_HASOWN_CALL_RE.finditer(text):
                add_row(
                    rows,
                    row_repos,
                    receiver_hint_counts,
                    OperationKey(
                        "js_ts.builtins.own_property",
                        "Object.prototype.hasOwnProperty.call",
                        "supported-partial",
                        "own-property-guard-proof",
                        "own-property-guard",
                        "prototype own-property guard needs unshadowed Object and matching object/key/default proof",
                    ),
                    repo,
                )
            for match in CONSTRUCTOR_RE.finditer(text):
                add_row(rows, row_repos, receiver_hint_counts, classify_constructor(match.group("type")), repo)
            for match in BARE_CALL_RE.finditer(text):
                prefix = text[max(0, match.start() - 8) : match.start()]
                if re.search(r"\bnew\s+$", prefix):
                    continue
                add_row(rows, row_repos, receiver_hint_counts, classify_bare_call(match.group("function")), repo)
            for match in METHOD_CALL_RE.finditer(text):
                if match.group("receiver") in {"Array", "Object", "Promise"}:
                    continue
                method = match.group("method")
                key = classify_method(method)
                if key is None:
                    continue
                add_row(rows, row_repos, receiver_hint_counts, key, repo, hints.get(match.group("receiver"), "unhinted-receiver"))
            for match in PROPERTY_RE.finditer(text):
                add_row(
                    rows,
                    row_repos,
                    receiver_hint_counts,
                    classify_property(match.group("property")),
                    repo,
                    hints.get(match.group("receiver"), "unhinted-receiver"),
                )
            for _match in AWAIT_RE.finditer(text):
                add_row(
                    rows,
                    row_repos,
                    receiver_hint_counts,
                    OperationKey(
                        "js_ts.builtins.async",
                        "await",
                        "unsupported",
                        "await-protocol-boundary",
                        "promise-protocol",
                        "await needs async scheduling, exception, and effect contracts before convergence",
                    ),
                    repo,
                )
            for _match in ASYNC_FUNCTION_RE.finditer(text):
                add_row(
                    rows,
                    row_repos,
                    receiver_hint_counts,
                    OperationKey(
                        "js_ts.builtins.async",
                        "async-function",
                        "unsupported",
                        "async-function-boundary",
                        "promise-protocol",
                        "async functions produce Promise-like protocol surfaces with scheduling effects",
                    ),
                    repo,
                )
            for _match in YIELD_RE.finditer(text):
                add_row(
                    rows,
                    row_repos,
                    receiver_hint_counts,
                    OperationKey(
                        "js_ts.builtins.generator",
                        "yield",
                        "unsupported",
                        "generator-protocol-boundary",
                        "generator-protocol",
                        "yield needs generator demand and suspension semantics before convergence",
                    ),
                    repo,
                )

    report_rows: list[dict[str, Any]] = []
    for key, occurrences in sorted(rows.items(), key=lambda item: (-item[1], item[0].surface, item[0].operation)):
        report_rows.append(
            {
                "surface": key.surface,
                "operation": key.operation,
                "occurrences": occurrences,
                "repos": len(row_repos[key]),
                "status": key.status,
                "boundary": key.boundary,
                "capability": key.capability,
                "note": key.note,
                "receiver_hints": [
                    {"hint": hint, "occurrences": count}
                    for hint, count in receiver_hint_counts[key].most_common()
                ],
                "top_repos": [
                    {"repo": repo, "occurrences": count}
                    for repo, count in row_repos[key].most_common(8)
                ],
            }
        )

    status_counts = Counter(row["status"] for row in report_rows for _ in range(row["occurrences"]))
    boundary_counts = Counter(
        row["boundary"] for row in report_rows for _ in range(row["occurrences"])
    )
    surface_counts = Counter(row["surface"] for row in report_rows for _ in range(row["occurrences"]))
    next_counts: Counter[tuple[str, str, str]] = Counter()
    next_repos: dict[tuple[str, str, str], set[str]] = defaultdict(set)
    next_surfaces: dict[tuple[str, str, str], Counter[str]] = defaultdict(Counter)
    for key, occurrences in rows.items():
        group = next_work_group(key.status, key.boundary, key.capability)
        if group is None:
            continue
        next_counts[group] += occurrences
        next_surfaces[group][f"{key.surface}.{key.operation}:{key.boundary}"] += occurrences
        next_repos[group].update(row_repos[key].keys())

    ranked_next_work = [
        {
            "id": group_id,
            "capability": capability,
            "occurrences": count,
            "repos": len(next_repos[group]),
            "policy": policy,
            "top_surfaces": [
                {"surface": surface, "occurrences": surface_count}
                for surface, surface_count in next_surfaces[group].most_common(8)
            ],
        }
        for group, count in sorted(next_counts.items(), key=lambda item: (-item[1], item[0][0]))
        for group_id, capability, policy in [group]
    ]

    report = {
        "report_kind": "js-ts-stdlib-partial-audit",
        "schema_version": 1,
        "manifest": args.manifest,
        "repos_root": args.repos_root,
        "scanned_js_ts_repos": len(repos),
        "scanned_js_ts_files": scanned_files,
        "scanned_file_languages": dict(sorted(language_counts.items())),
        "totals": {
            "occurrences": sum(rows.values()),
            "supported_occurrences": sum(
                row["occurrences"] for row in report_rows if row["status"] == "supported"
            ),
            "supported_partial_occurrences": sum(
                row["occurrences"] for row in report_rows if row["status"] == "supported-partial"
            ),
            "unsupported_occurrences": sum(
                row["occurrences"] for row in report_rows if row["status"] == "unsupported"
            ),
            "unknown_boundary_occurrences": boundary_counts.get("unknown-boundary", 0),
        },
        "status_counts": dict(sorted(status_counts.items())),
        "surface_counts": dict(sorted(surface_counts.items())),
        "boundary_counts": dict(sorted(boundary_counts.items())),
        "ranked_next_work": ranked_next_work,
        "processing_threshold_occurrences": HIGH_VOLUME_PROCESSING_THRESHOLD,
        "processed_high_volume_groups": processed_high_volume_groups(ranked_next_work),
        "operations": report_rows,
    }

    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=False) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
