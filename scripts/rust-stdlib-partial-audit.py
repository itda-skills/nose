#!/usr/bin/env python3
"""Audit Rust stdlib partial-coverage prevalence in the pinned corpus.

This is a lexical pricing report, not semantic proof. It masks comments and
strings before counting common Rust stdlib-shaped constructors, factories, map
lookups, iterator adapters, and mutation/order boundaries. The goal is to split
large already-covered-partial surfaces into reusable semantic-kernel capability
buckets before adding more API rows.
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
DEFAULT_OUTPUT = "target/rust-stdlib-partial-audit.v2.json"
HIGH_VOLUME_PROCESSING_THRESHOLD = 5000

SKIP_DIRS = {
    ".git",
    ".gradle",
    "build",
    "dist",
    "node_modules",
    "target",
    "vendor",
}

METHOD_RE = re.compile(
    r"\.\s*(?P<method>[A-Za-z_][A-Za-z0-9_]*)\s*(?:::<[^;\n(){}]*>)?\s*\("
)


@dataclass(frozen=True)
class LexicalPattern:
    surface: str
    operation: str
    regex: str
    status: str
    boundary: str
    capability: str
    note: str

    def compile(self) -> re.Pattern[str]:
        return re.compile(self.regex)


LEXICAL_PATTERNS = [
    LexicalPattern(
        "rust.stdlib.option",
        "Some",
        r"\b(?:Option::|std::option::Option::|core::option::Option::)?Some\s*\(",
        "supported",
        "admitted-option-constructor",
        "option-result-channel",
        "Option Some constructor has pack-proven occurrence support.",
    ),
    LexicalPattern(
        "rust.stdlib.option",
        "None",
        r"\b(?:Option::|std::option::Option::|core::option::Option::)?None\b",
        "supported",
        "admitted-option-constructor",
        "option-result-channel",
        "Option None sentinel has pack-proven occurrence support.",
    ),
    LexicalPattern(
        "rust.stdlib.result",
        "Ok",
        r"\b(?:Result::|std::result::Result::|core::result::Result::)?Ok\s*\(",
        "supported",
        "admitted-result-constructor",
        "option-result-channel",
        "Result Ok constructor has pack-proven occurrence support.",
    ),
    LexicalPattern(
        "rust.stdlib.result",
        "Err",
        r"\b(?:Result::|std::result::Result::|core::result::Result::)?Err\s*\(",
        "supported",
        "admitted-result-constructor",
        "option-result-channel",
        "Result Err constructor has pack-proven occurrence support.",
    ),
    LexicalPattern(
        "rust.stdlib.vec",
        "vec!",
        r"\bvec!\s*[\[(]",
        "supported",
        "admitted-vec-factory",
        "collection-factory",
        "vec! macro factories have source-syntax and pack-proven occurrence support.",
    ),
    LexicalPattern(
        "rust.stdlib.vec",
        "Vec::new",
        r"\b(?:std::vec::|alloc::vec::)?Vec::new\s*\(",
        "supported",
        "admitted-vec-factory",
        "collection-factory",
        "Vec::new has pack-proven occurrence support.",
    ),
    LexicalPattern(
        "rust.stdlib.vec",
        "Vec::with_capacity",
        r"\b(?:std::vec::|alloc::vec::)?Vec::with_capacity\s*\(",
        "unsupported",
        "allocation-lifetime",
        "allocation-lifetime",
        "Capacity allocation is not a pure collection value contract.",
    ),
    LexicalPattern(
        "rust.stdlib.vec",
        "Vec::from",
        r"\b(?:std::vec::|alloc::vec::)?Vec::from\s*\(",
        "unsupported",
        "vec-from-domain-proof",
        "collection-factory",
        "Vec::from needs source/domain proof distinct from Vec::new and vec!.",
    ),
    LexicalPattern(
        "rust.stdlib.collection_factories",
        "HashSet::from",
        r"\b(?:std::collections::)?HashSet::from\s*\(",
        "supported-partial",
        "std-collection-factory-proof",
        "collection-factory",
        "HashSet::from is admitted only with std/import path and exact-safe arguments.",
    ),
    LexicalPattern(
        "rust.stdlib.collection_factories",
        "BTreeSet::from",
        r"\b(?:std::collections::)?BTreeSet::from\s*\(",
        "supported-partial",
        "std-collection-factory-proof",
        "collection-factory",
        "BTreeSet::from is admitted only with std/import path and exact-safe arguments.",
    ),
    LexicalPattern(
        "rust.stdlib.collection_factories",
        "VecDeque::from",
        r"\b(?:std::collections::)?VecDeque::from\s*\(",
        "supported-partial",
        "std-collection-factory-proof",
        "collection-factory",
        "VecDeque::from is admitted only with std/import path and exact-safe arguments.",
    ),
    LexicalPattern(
        "rust.stdlib.map_factories",
        "HashMap::from",
        r"\b(?:std::collections::)?HashMap::from\s*\(",
        "supported-partial",
        "std-map-factory-proof",
        "map-factory",
        "HashMap::from is admitted only with std/import path and exact-safe entry tuples.",
    ),
    LexicalPattern(
        "rust.stdlib.map_factories",
        "BTreeMap::from",
        r"\b(?:std::collections::)?BTreeMap::from\s*\(",
        "supported-partial",
        "std-map-factory-proof",
        "map-factory",
        "BTreeMap::from is admitted only with std/import path and exact-safe entry tuples.",
    ),
    LexicalPattern(
        "rust.stdlib.map_set_factories",
        "HashMap::new",
        r"\b(?:std::collections::)?HashMap::new\s*\(",
        "unsupported",
        "empty-map-factory",
        "map-factory",
        "Empty map factory semantics are not currently a dedicated admitted row.",
    ),
    LexicalPattern(
        "rust.stdlib.map_set_factories",
        "BTreeMap::new",
        r"\b(?:std::collections::)?BTreeMap::new\s*\(",
        "unsupported",
        "empty-map-factory",
        "map-factory",
        "Empty map factory semantics are not currently a dedicated admitted row.",
    ),
    LexicalPattern(
        "rust.stdlib.map_set_factories",
        "HashSet::new",
        r"\b(?:std::collections::)?HashSet::new\s*\(",
        "unsupported",
        "empty-set-factory",
        "collection-factory",
        "Empty set factory semantics are not currently a dedicated admitted row.",
    ),
    LexicalPattern(
        "rust.stdlib.map_set_factories",
        "BTreeSet::new",
        r"\b(?:std::collections::)?BTreeSet::new\s*\(",
        "unsupported",
        "empty-set-factory",
        "collection-factory",
        "Empty set factory semantics are not currently a dedicated admitted row.",
    ),
]

SUPPORTED_PARTIAL_METHODS: dict[str, tuple[str, str, str, str]] = {
    "all": (
        "rust.stdlib.iterators",
        "terminal-hof",
        "iterator-hof-materialization",
        "terminal iterator HOF needs receiver and callback proof",
    ),
    "and_then": (
        "rust.stdlib.option",
        "option-callback",
        "option-result-channel",
        "Option and_then is admitted only with exact Option receiver and callback proof",
    ),
    "any": (
        "rust.stdlib.iterators",
        "terminal-hof",
        "iterator-hof-materialization",
        "terminal iterator HOF needs receiver and callback proof",
    ),
    "cloned": (
        "rust.stdlib.iterators",
        "iterator-adapter-domain",
        "iterator-hof-materialization",
        "iterator identity adapter result-domain proof",
    ),
    "collect": (
        "rust.stdlib.iterators",
        "type-directed-materializer",
        "iterator-hof-materialization",
        "collect remains type-directed; it does not assert collection result domain alone",
    ),
    "contains": (
        "rust.stdlib.membership",
        "collection-membership-proof",
        "collection-membership",
        "collection membership needs exact receiver proof",
    ),
    "contains_key": (
        "rust.stdlib.membership",
        "map-membership-proof",
        "map-membership",
        "map membership needs exact map receiver proof",
    ),
    "copied": (
        "rust.stdlib.iterators",
        "iterator-adapter-domain",
        "iterator-hof-materialization",
        "iterator identity adapter result-domain proof",
    ),
    "filter": (
        "rust.stdlib.iterators",
        "hof-callback-proof",
        "iterator-hof-materialization",
        "iterator HOF needs receiver, callback, and demand/effect proof",
    ),
    "filter_map": (
        "rust.stdlib.iterators",
        "hof-callback-proof",
        "iterator-hof-materialization",
        "iterator HOF needs receiver, callback, and demand/effect proof",
    ),
    "flat_map": (
        "rust.stdlib.iterators",
        "hof-callback-proof",
        "iterator-hof-materialization",
        "iterator HOF needs receiver, callback, and demand/effect proof",
    ),
    "get": (
        "rust.stdlib.map_get",
        "map-get-proof",
        "map-lookup",
        "map get needs exact map receiver proof and downstream result use",
    ),
    "into_iter": (
        "rust.stdlib.iterators",
        "iterator-adapter-domain",
        "iterator-hof-materialization",
        "iterator identity adapter result-domain proof",
    ),
    "is_err": (
        "rust.stdlib.result",
        "result-predicate-proof",
        "option-result-channel",
        "Result predicate needs exact Result receiver proof",
    ),
    "is_ok": (
        "rust.stdlib.result",
        "result-predicate-proof",
        "option-result-channel",
        "Result predicate needs exact Result receiver proof",
    ),
    "is_none": (
        "rust.stdlib.option",
        "option-predicate-proof",
        "option-result-channel",
        "Option absence predicate has generic method-call support with exact Option receiver proof",
    ),
    "is_some": (
        "rust.stdlib.option",
        "option-predicate-proof",
        "option-result-channel",
        "Option predicate needs exact Option receiver proof",
    ),
    "iter": (
        "rust.stdlib.iterators",
        "iterator-adapter-domain",
        "iterator-hof-materialization",
        "iterator identity adapter result-domain proof",
    ),
    "iter_mut": (
        "rust.stdlib.iterators",
        "iterator-adapter-domain",
        "iterator-hof-materialization",
        "iterator identity adapter result-domain proof",
    ),
    "map": (
        "rust.stdlib.iterators",
        "hof-callback-proof",
        "iterator-hof-materialization",
        "iterator HOF needs receiver, callback, and demand/effect proof",
    ),
    "count": (
        "rust.stdlib.iterators",
        "iterator-terminal-count",
        "iterator-hof-materialization",
        "terminal iterator count has method-call support with explicit protocol receiver proof",
    ),
    "extend": (
        "rust.stdlib.mutation",
        "mutation-effect",
        "effect-preserving-contracts",
        "receiver mutation evidence exists; exact value admission stays closed after mutation",
    ),
    "insert": (
        "rust.stdlib.mutation",
        "mutation-effect",
        "effect-preserving-contracts",
        "receiver mutation evidence exists; exact value admission stays closed after mutation",
    ),
    "push": (
        "rust.stdlib.mutation",
        "mutation-effect",
        "effect-preserving-contracts",
        "builder append and receiver mutation evidence exist under effect-gated proof",
    ),
    "remove": (
        "rust.stdlib.mutation",
        "mutation-effect",
        "effect-preserving-contracts",
        "receiver mutation evidence exists; exact value admission stays closed after mutation",
    ),
    "sort": (
        "rust.stdlib.ordering",
        "mutation-effect",
        "effect-preserving-contracts",
        "receiver mutation evidence exists; sortedness semantics remain closed",
    ),
    "sort_by": (
        "rust.stdlib.ordering",
        "mutation-callback",
        "effect-preserving-contracts",
        "receiver mutation evidence exists; comparator semantics remain closed",
    ),
    "sort_by_key": (
        "rust.stdlib.ordering",
        "mutation-callback",
        "effect-preserving-contracts",
        "receiver mutation evidence exists; key/comparator semantics remain closed",
    ),
    "sort_unstable": (
        "rust.stdlib.ordering",
        "mutation-effect",
        "effect-preserving-contracts",
        "receiver mutation evidence exists; sortedness semantics remain closed",
    ),
    "to_vec": (
        "rust.stdlib.iterators",
        "collection-result-domain",
        "collection-factory",
        "to_vec can materialize collection result-domain evidence with receiver proof",
    ),
    "unwrap_or": (
        "rust.stdlib.option",
        "map-get-or-option-default",
        "option-result-channel",
        "unwrap_or is admitted only through exact Option or nested map-get proof",
    ),
    "unwrap_or_else": (
        "rust.stdlib.option",
        "map-get-or-option-default",
        "option-result-channel",
        "unwrap_or_else is admitted only through exact Option or nested map-get proof",
    ),
}

PROCESSING_DECISIONS: dict[str, dict[str, Any]] = {
    "rust-iterator-domain-proof": {
        "sequence": 2,
        "status": "processed-existing-contract",
        "semantic_admission_delta": 0,
        "strictness_effect": "unchanged",
        "decision": (
            "Keep iter/into_iter/iter_mut/copied/cloned/to_vec on the existing "
            "IteratorIdentityAdapter row and require receiver-domain proof."
        ),
        "closed_boundary": (
            "collect remains type-directed and does not assert collection result "
            "domain without caller-selected result-type proof."
        ),
        "next_metric": (
            "Track how many iterator adapter calls materialize Domain(Iterator) or "
            "Domain(Collection) evidence, and how many fail receiver-domain proof."
        ),
    },
    "rust-option-result-channel": {
        "sequence": 3,
        "status": "processed-contract-alignment",
        "semantic_admission_delta": 0,
        "strictness_effect": "unchanged",
        "decision": (
            "Align the audit with existing Option/Result channel contracts: "
            "is_some/is_none/is_ok/is_err/and_then/default helpers stay exact-receiver gated."
        ),
        "closed_boundary": (
            "Option-producing iterator APIs such as find still require callback and "
            "result-channel proof before admission."
        ),
        "next_metric": (
            "Split channel misses by exact Option receiver, exact Result receiver, "
            "default argument, callback, and option-producing iterator source."
        ),
    },
    "rust-hof-callback-proof": {
        "sequence": 4,
        "status": "processed-existing-contract",
        "semantic_admission_delta": 0,
        "strictness_effect": "unchanged",
        "decision": (
            "Keep Rust iterator HOFs on the sequence-HOF protocol row with explicit "
            "protocol receiver and callback demand/effect proof."
        ),
        "closed_boundary": (
            "Selector-only map/filter/any/all evidence, eager callback assumptions, "
            "and missing terminal proof remain closed."
        ),
        "next_metric": (
            "Track HOF misses by receiver proof, callback identity, callback effect, "
            "and terminal materialization proof."
        ),
    },
    "rust-iterator-view-lifecycle": {
        "sequence": 6,
        "status": "processed-boundary-split",
        "semantic_admission_delta": 0,
        "strictness_effect": "unchanged",
        "decision": (
            "Separate type-directed materializers, slice views, index views, zip views, "
            "terminal count, chain, and partition instead of admitting a broad iterator-view API."
        ),
        "closed_boundary": (
            "Views with lifecycle/cardinality obligations remain closed until the kernel "
            "can represent one-shot consumption, ordering, and result shape."
        ),
        "next_metric": (
            "Track each view subtype separately and require a fixture before moving any "
            "subtype from closed boundary to admitted contract."
        ),
    },
    "rust-mutation-effect-contracts": {
        "sequence": 7,
        "status": "processed-effect-contract-alignment",
        "semantic_admission_delta": 0,
        "strictness_effect": "stricter",
        "decision": (
            "Treat collection/map mutations as effect evidence, not pure value APIs; "
            "add the missing Rust sort_by_key receiver-mutation row."
        ),
        "closed_boundary": (
            "Mutating calls still do not produce exact value equivalence; they only "
            "mark receiver mutation so later strict receiver use can close safely."
        ),
        "next_metric": (
            "Track mutation/effect misses by covered receiver-mutation rows, builder "
            "append rows, callback-effect rows, and unsupported ordering callbacks."
        ),
    },
}

UNSUPPORTED_METHODS: dict[str, tuple[str, str, str, str]] = {
    "binary_search": (
        "rust.stdlib.ordering",
        "ordering-precondition",
        "collection-ordering",
        "binary search requires sorted-order preconditions",
    ),
    "binary_search_by": (
        "rust.stdlib.ordering",
        "ordering-callback",
        "collection-ordering",
        "binary search requires sorted-order and comparator callback proof",
    ),
    "binary_search_by_key": (
        "rust.stdlib.ordering",
        "ordering-callback",
        "collection-ordering",
        "binary search requires sorted-order and key callback proof",
    ),
    "chain": (
        "rust.stdlib.iterators",
        "iterator-view-lifecycle",
        "iterator-hof-materialization",
        "iterator chaining lifecycle and shape are not modeled",
    ),
    "count": (
        "rust.stdlib.iterators",
        "iterator-terminal-count",
        "iterator-hof-materialization",
        "terminal iterator count needs cardinality/source proof",
    ),
    "enumerate": (
        "rust.stdlib.iterators",
        "iterator-index-view",
        "iterator-hof-materialization",
        "index-producing iterator view is not modeled",
    ),
    "extend": (
        "rust.stdlib.mutation",
        "mutation-effect",
        "effect-preserving-contracts",
        "collection extension mutates the receiver",
    ),
    "find": (
        "rust.stdlib.iterators",
        "option-producing-hof",
        "option-result-channel",
        "find returns an Option and needs callback/result-channel proof",
    ),
    "fold": (
        "rust.stdlib.iterators",
        "reduction-callback",
        "iterator-hof-materialization",
        "callback reduction needs accumulator demand/effect semantics",
    ),
    "for_each": (
        "rust.stdlib.iterators",
        "effect-callback",
        "effect-preserving-contracts",
        "for_each is effect-oriented and callback-driven",
    ),
    "insert": (
        "rust.stdlib.mutation",
        "mutation-effect",
        "effect-preserving-contracts",
        "insert mutates collection or map state",
    ),
    "inspect": (
        "rust.stdlib.iterators",
        "effect-callback",
        "effect-preserving-contracts",
        "inspect is callback/effect-oriented",
    ),
    "is_none": (
        "rust.stdlib.option",
        "option-absence-predicate",
        "option-result-channel",
        "Option absence predicate is not currently an admitted row",
    ),
    "max": (
        "rust.stdlib.ordering",
        "ordering-reduction",
        "collection-ordering",
        "ordering reduction is not modeled",
    ),
    "min": (
        "rust.stdlib.ordering",
        "ordering-reduction",
        "collection-ordering",
        "ordering reduction is not modeled",
    ),
    "partition": (
        "rust.stdlib.iterators",
        "partition-materializer",
        "iterator-hof-materialization",
        "partition materializes two collections and needs result-domain proof",
    ),
    "position": (
        "rust.stdlib.iterators",
        "option-producing-hof",
        "option-result-channel",
        "position returns an Option index and needs callback/result-channel proof",
    ),
    "push": (
        "rust.stdlib.mutation",
        "mutation-effect",
        "effect-preserving-contracts",
        "push mutates collection state",
    ),
    "reduce": (
        "rust.stdlib.iterators",
        "reduction-callback",
        "iterator-hof-materialization",
        "callback reduction needs accumulator demand/effect semantics",
    ),
    "remove": (
        "rust.stdlib.mutation",
        "mutation-effect",
        "effect-preserving-contracts",
        "remove mutates collection or map state",
    ),
    "rev": (
        "rust.stdlib.iterators",
        "ordering-view",
        "collection-ordering",
        "reverse iterator view needs sequence/order proof",
    ),
    "skip": (
        "rust.stdlib.iterators",
        "iterator-slice-view",
        "iterator-hof-materialization",
        "iterator slicing view is not modeled",
    ),
    "sort": (
        "rust.stdlib.ordering",
        "mutation-effect",
        "collection-ordering",
        "in-place sort mutates collection order",
    ),
    "sort_by": (
        "rust.stdlib.ordering",
        "mutation-callback",
        "collection-ordering",
        "in-place sort with comparator callback",
    ),
    "sort_by_key": (
        "rust.stdlib.ordering",
        "mutation-callback",
        "collection-ordering",
        "in-place sort with key callback",
    ),
    "sort_unstable": (
        "rust.stdlib.ordering",
        "mutation-effect",
        "collection-ordering",
        "in-place unstable sort mutates collection order",
    ),
    "sum": (
        "rust.stdlib.iterators",
        "numeric-reduction",
        "numeric-scalar-methods",
        "numeric reduction needs domain and overflow semantics",
    ),
    "take": (
        "rust.stdlib.iterators",
        "iterator-slice-view",
        "iterator-hof-materialization",
        "iterator slicing view is not modeled",
    ),
    "zip": (
        "rust.stdlib.iterators",
        "iterator-zip-view",
        "iterator-hof-materialization",
        "zip view shape and lifecycle are not modeled",
    ),
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", default=DEFAULT_MANIFEST)
    parser.add_argument("--repos-root", default=DEFAULT_REPOS_ROOT)
    parser.add_argument("--output", default=DEFAULT_OUTPUT)
    return parser.parse_args()


def rust_repos(manifest: Path) -> list[str]:
    data = json.loads(manifest.read_text())
    return [
        repo["id"]
        for repo in data.get("repositories", [])
        if repo.get("primary_language") == "Rust"
    ]


def rust_files(root: Path) -> list[Path]:
    if not root.exists():
        return []
    files: list[Path] = []
    for path in root.rglob("*.rs"):
        if any(part in SKIP_DIRS for part in path.parts):
            continue
        files.append(path)
    return files


def mask_comments_and_strings(text: str) -> str:
    chars = list(text)
    i = 0
    block_depth = 0
    while i < len(chars):
        if block_depth > 0:
            if text.startswith("/*", i):
                chars[i : i + 2] = "  "
                block_depth += 1
                i += 2
            elif text.startswith("*/", i):
                chars[i : i + 2] = "  "
                block_depth -= 1
                i += 2
            else:
                if chars[i] != "\n":
                    chars[i] = " "
                i += 1
            continue
        if text.startswith("//", i):
            while i < len(chars) and chars[i] != "\n":
                chars[i] = " "
                i += 1
            continue
        if text.startswith("/*", i):
            chars[i : i + 2] = "  "
            block_depth = 1
            i += 2
            continue
        raw_len = raw_string_prefix_len(text, i)
        if raw_len is not None:
            prefix_len, hashes = raw_len
            j = i + prefix_len
            terminator = '"' + ("#" * hashes)
            end = text.find(terminator, j)
            end = len(chars) - 1 if end == -1 else end + len(terminator) - 1
            while i <= end and i < len(chars):
                if chars[i] != "\n":
                    chars[i] = " "
                i += 1
            continue
        if chars[i] == '"':
            i = mask_quoted(chars, i, '"')
            continue
        if chars[i] == "'":
            char_end = char_literal_end(text, i)
            if char_end is not None:
                while i <= char_end and i < len(chars):
                    if chars[i] != "\n":
                        chars[i] = " "
                    i += 1
                continue
            i += 1
            continue
        i += 1
    return "".join(chars)


def raw_string_prefix_len(text: str, index: int) -> tuple[int, int] | None:
    if text[index] not in {"r", "b"}:
        return None
    j = index
    if text.startswith("br", j) or text.startswith("rb", j):
        j += 2
    elif text[j] == "r":
        j += 1
    else:
        return None
    hashes = 0
    while j < len(text) and text[j] == "#":
        hashes += 1
        j += 1
    if j < len(text) and text[j] == '"':
        return j - index + 1, hashes
    return None


def mask_quoted(chars: list[str], index: int, quote: str) -> int:
    chars[index] = " "
    i = index + 1
    while i < len(chars):
        ch = chars[i]
        if ch != "\n":
            chars[i] = " "
        if ch == "\\":
            i += 2
            continue
        if ch == quote:
            return i + 1
        i += 1
    return i


def char_literal_end(text: str, index: int) -> int | None:
    j = index + 1
    if j >= len(text) or text[j] == "\n":
        return None
    if text[j] == "\\":
        j += 2
    else:
        j += 1
    if j < len(text) and text[j] == "'":
        return j
    return None


def classify_method(method: str) -> tuple[str, str, str, str, str, str]:
    partial = SUPPORTED_PARTIAL_METHODS.get(method)
    if partial is not None:
        surface, boundary, capability, note = partial
        return surface, method, "supported-partial", boundary, capability, note
    unsupported = UNSUPPORTED_METHODS.get(method)
    if unsupported is not None:
        surface, boundary, capability, note = unsupported
        return surface, method, "unsupported", boundary, capability, note
    return (
        "rust.stdlib.unclassified_methods",
        method,
        "unknown",
        "unknown-boundary",
        "unknown",
        "observed method is not classified by this audit",
    )


def next_work_group(status: str, boundary: str, capability: str) -> tuple[str, str, str] | None:
    if status == "supported":
        return None
    if boundary in {"iterator-adapter-domain", "collection-result-domain"}:
        return (
            "rust-iterator-domain-proof",
            "iterator adapter/result-domain proof",
            "Keep widening receiver-domain evidence instead of selector-only iterator admission.",
        )
    if boundary in {"hof-callback-proof", "terminal-hof"}:
        return (
            "rust-hof-callback-proof",
            "iterator HOF callback and demand/effect proof",
            "Map/filter/terminal HOFs need reusable callback identity and effect profiles.",
        )
    if boundary in {
        "iterator-index-view",
        "iterator-terminal-count",
        "iterator-slice-view",
        "iterator-view-lifecycle",
        "iterator-zip-view",
        "partition-materializer",
        "type-directed-materializer",
    }:
        return (
            "rust-iterator-view-lifecycle",
            "iterator view lifecycle and shape contracts",
            "Iterator views need lifecycle/cardinality policies before exact admission.",
        )
    if boundary in {
        "mutation-callback",
        "mutation-effect",
    } or capability == "effect-preserving-contracts":
        return (
            "rust-mutation-effect-contracts",
            "mutation/effect contracts",
            "In-place collection/map changes need place/effect summaries before admission.",
        )
    if "ordering" in boundary or capability == "collection-ordering":
        return (
            "rust-ordering-semantics",
            "ordering and sortedness preconditions",
            "Ordering APIs need comparator/key obligations and sortedness proof.",
        )
    if boundary in {"map-get-proof", "map-membership-proof", "collection-membership-proof"}:
        return (
            "rust-receiver-domain-proof",
            "receiver-domain proof",
            "Membership and map lookup should consume reusable collection/map receiver proof.",
        )
    if boundary in {
        "map-get-or-option-default",
        "option-absence-predicate",
        "option-callback",
        "option-predicate-proof",
        "option-producing-hof",
        "result-predicate-proof",
    }:
        return (
            "rust-option-result-channel",
            "Option/Result channel proof",
            "Option/Result helpers need exact receiver and callback/default obligations.",
        )
    if "factory" in boundary:
        return (
            "rust-factory-domain-proof",
            "collection/map factory domain proof",
            "Factories need source/path proof, result-domain proof, and exact-safe arguments.",
        )
    if boundary == "vec-from-domain-proof":
        return (
            "rust-factory-domain-proof",
            "collection/map factory domain proof",
            "Factories need source/path proof, result-domain proof, and exact-safe arguments.",
        )
    if boundary == "allocation-lifetime":
        return (
            "rust-allocation-lifetime",
            "allocation/lifetime boundary",
            "Capacity and allocation APIs should remain closed until lifetime/effect contracts exist.",
        )
    if "reduction" in boundary:
        return (
            "rust-reduction-contracts",
            "reduction contracts",
            "Reductions need accumulator, numeric, or callback demand/effect semantics.",
        )
    return (
        "rust-unclassified-boundary",
        "unclassified boundary",
        "Observed rows should stay closed until classified by capability.",
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
    compiled = [(pattern, pattern.compile()) for pattern in LEXICAL_PATTERNS]
    rows: Counter[tuple[str, str, str, str, str, str]] = Counter()
    row_repos: dict[tuple[str, str, str, str, str, str], Counter[str]] = defaultdict(Counter)
    scanned_files = 0

    for repo in rust_repos(Path(args.manifest)):
        for path in rust_files(repos_root / repo):
            scanned_files += 1
            try:
                raw_text = path.read_text(errors="ignore")
            except OSError:
                continue
            text = mask_comments_and_strings(raw_text)
            for pattern, regex in compiled:
                count = len(regex.findall(text))
                if count == 0:
                    continue
                key = (
                    pattern.surface,
                    pattern.operation,
                    pattern.status,
                    pattern.boundary,
                    pattern.capability,
                    pattern.note,
                )
                rows[key] += count
                row_repos[key][repo] += count
            for match in METHOD_RE.finditer(text):
                method = match.group("method")
                if method not in SUPPORTED_PARTIAL_METHODS and method not in UNSUPPORTED_METHODS:
                    continue
                surface, operation, status, boundary, capability, note = classify_method(method)
                key = (surface, operation, status, boundary, capability, note)
                rows[key] += 1
                row_repos[key][repo] += 1

    report_rows: list[dict[str, Any]] = []
    for key, occurrences in sorted(rows.items(), key=lambda item: (-item[1], item[0])):
        surface, operation, status, boundary, capability, note = key
        report_rows.append(
            {
                "surface": surface,
                "operation": operation,
                "occurrences": occurrences,
                "repos": len(row_repos[key]),
                "status": status,
                "boundary": boundary,
                "capability": capability,
                "note": note,
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
        surface, operation, status, boundary, capability, _note = key
        group = next_work_group(status, boundary, capability)
        if group is None:
            continue
        next_counts[group] += occurrences
        next_surfaces[group][f"{surface}.{operation}:{boundary}"] += occurrences
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
        "report_kind": "rust-stdlib-partial-audit",
        "schema_version": 2,
        "manifest": args.manifest,
        "repos_root": args.repos_root,
        "scanned_rust_repos": len(rust_repos(Path(args.manifest))),
        "scanned_rust_files": scanned_files,
        "totals": {
            "occurrences": sum(rows.values()),
            "supported_occurrences": status_counts.get("supported", 0),
            "supported_partial_occurrences": status_counts.get("supported-partial", 0),
            "unsupported_occurrences": status_counts.get("unsupported", 0),
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
