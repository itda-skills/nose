use super::*;

/// Run the whole-source span through the AST-facts classifier — the same
/// path `declaration_run_span` takes, minus the file I/O.
fn ast_classifies(ext: &str, src: &str) -> bool {
    let Some(facts) = nose_frontend::declaration_facts(ext, src) else {
        return false;
    };
    let all: Vec<String> = src.lines().map(str::to_string).collect();
    let end = all.len().max(1) as u32;
    span_is_declarations(&facts, &all, 1, end)
}

#[test]
fn declaration_spans_classify_per_language() {
    let yes: &[(&str, &str)] = &[
        ("ts", "import { a } from './a';\nimport { b } from './b';"),
        ("ts", "import {\n  a,\n  b,\n} from './ab';"),
        ("ts", "export { a } from './a';\nexport * from './b';"),
        ("ts", "const fs = require('fs');"),
        ("py", "import os\nfrom typing import (\n    Any,\n)"),
        (
            "go",
            "package main\n\nimport (\n\t\"fmt\"\n\talias \"net/http\"\n)",
        ),
        (
            "rs",
            "use std::fmt;\npub use crate::x::{\n    A,\n};\nmod wiring;",
        ),
        (
            "java",
            "package com.x;\nimport java.util.List;\nimport static java.util.Map.entry;",
        ),
        ("c", "#include <stdio.h>\n#include \"x.h\"\n#pragma once"),
        ("rb", "require 'json'\nrequire_relative 'x'"),
        // S2-C3 coverage rows: shapes the code supports but no test locked.
        ("rs", "pub(crate) use crate::x::Y;"),
        ("go", "import http \"net/http\""),
        ("py", "from os import path"),
        ("rb", "require('json')"),
        ("c", "#include<stdio.h>"),
        ("ts", "import{a} from './a';"),
        // ASI: a multi-line import may close without a semicolon.
        ("ts", "import {\n  a,\n} from './ab'"),
        // The closer may carry the final import names (corpus re-price
        // regression in series 2: bare-`)` leaked real Python imports).
        ("py", "from typing import (\n    Any,\n    Mapping)"),
        // S4-C5 coverage adoptions (supported kinds with no locked row).
        ("rs", "extern crate serde;\nextern crate serde_json;"),
        ("go", "package main"),
        ("rb", "require_relative './helpers'"),
        // S3-C5 coverage adoptions.
        ("go", "import (\n\t. \"fmt\"\n\t_ \"encoding/json\"\n)"),
        ("rs", "use std::{\n    io::{self, Read},\n};"),
        ("ts", "import {\n  $ref,\n} from './x';"),
        ("ts", "export {\n  a,\n  b,\n} from './lib';"),
        ("ts", "const $lib = require('lib');"),
        ("py", "from typing import (\n    Dict as D,\n)"),
        ("py", "from x import *"),
        // Corpus re-price regressions (series 3): inert trailing comments
        // and single-line parenthesized name lists are real wiring.
        ("py", "import os  # noqa"),
        ("py", "from os import path  # comment"),
        ("py", "from x import (a, b)"),
    ];
    for (ext, src) in yes {
        assert!(
            ast_classifies(ext, src),
            "should classify as declarations: {src}"
        );
    }
}

#[test]
fn declaration_spans_fail_open_per_language() {
    // Fail-open: anything not provably a declaration keeps the family on
    // its ranked surface — misclassifying a real finding is the error
    // class this filter must never make.
    let no: &[(&str, &str)] = &[
        ("ts", "import { a } from './a';\nexport const x = a;"),
        ("ts", "import {\n  a,"),
        ("py", "import os\nx = os.environ"),
        ("go", "import (\n\t\"fmt\""),
        ("rs", "use std::fmt;\nfn main() {}"),
        ("java", "import java.util.List;\nclass X {}"),
        ("c", "#include <stdio.h>\n#define MAX 4"),
        ("rb", "require 'json'\nputs 'hi'"),
        ("py", ""),
        // C1 claim-violation packets: a single LINE mixing a declaration
        // with executable code must never classify (the "provably no
        // extraction exists" claim breaks if real code rides along).
        ("ts", "import { a } from './a'; doEvil();"),
        ("ts", "var a = require('a'), b = compute();"),
        ("go", "import \"fmt\"; func main() { hack() }"),
        ("rb", "require 'json'; system('x')"),
        ("py", "from x import y; z = 1"),
        ("java", "import java.util.List; int x = 1;"),
        ("rs", "use std::fmt; let x = 1;"),
        ("c", "#includeevil <x.h>"),
        // C5 boundary re-attack on the C1 defense itself.
        ("ts", "import { a } from './a';;"),
        ("rb", "require 'x' if expensive_check()"),
        // S2-C1 blind-attacker packets: open-block interiors and closers
        // were unvalidated (tree-sitter error tolerance voids any "the
        // file parsed, so interiors are specifiers" assumption).
        ("rb", "require 'fs' + 1"),
        ("c", "#include <stdio.h> int x = 1;"),
        ("ts", "import {\n  a,\n} || x();"),
        ("go", "import (\n\t\"fmt\"\n\tos.Exit(1))"),
        ("rs", "use std::{\ninvalid;\n};"),
        // S3-C1 blind-attacker packets: from-clause sources, Python name
        // lists, and Java paths smuggled expressions through shape checks.
        ("ts", "import { x } from Math.max(\"a\", \"b\");"),
        ("ts", "export { x } from path.join(\"c\", \"d\");"),
        ("py", "from x import max(\"a\", \"b\")"),
        ("java", "import java.util.x + y;"),
        ("java", "package com.example.x + y;"),
        // S3-C5 boundary re-attacks on the strict closers.
        ("rs", "use std::{\n  A,\n}x;"),
        ("go", "import (\n\tfunc() \"x\"\n)"),
        // S4-C1: call-shaped declaration entries whose binding/block
        // smuggles execution past the node-kind whitelist.
        (
            "js",
            "const { boom = stealCreditCards() } = require('lit');",
        ),
        ("js", "const { [exfiltrate()]: grabbed } = require('lit');"),
        (
            "ts",
            "const { boom = stealCreditCards() } = require('lit');",
        ),
        ("rb", "require('socket') { launch_missiles }"),
    ];
    for (ext, src) in no {
        assert!(!ast_classifies(ext, src), "must fail open on: {src:?}");
    }
}

#[test]
fn declaration_spans_inert_destructure_still_classifies() {
    // The S4-C1 fix must not over-reject: a plain destructuring require
    // executes nothing and stays wiring.
    assert!(ast_classifies(
        "js",
        "const { boom, fizz } = require('lit');"
    ));
    assert!(ast_classifies("py", "\u{feff}import os")); // BOM-tolerant
}
