use super::*;

// Broad fixture matrix for ordered conditional-effect branch boundaries. The size is
// intentional until the fixture setup has a clearer table-builder abstraction.
#[allow(clippy::too_many_lines)]
#[test]
fn semantic_query_reports_exact_safe_ordered_conditional_effect_branch_fragments() {
    let dir = std::env::temp_dir().join(format!(
        "nose_ordered_conditional_effect_branch_fragments_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    let fixtures = [
        (
            "cond_pair_append_a.ts",
            "function condPairAppendLeft(enabled: boolean, x: number, y: number, out: number[]): void {\n  if (enabled) {\n    if (x > 0) {\n      out.push(x + 1);\n    }\n    if (y > 0) {\n      out.push(y * y);\n    }\n  }\n  audit(enabled);\n}\n",
        ),
        (
            "cond_pair_append_b.ts",
            "function condPairAppendRight(flag: boolean, a: number, b: number, dst: number[]): void {\n  if (flag) {\n    if (0 < a) {\n      dst.push(1 + a);\n    }\n    if (b > 0) {\n      dst.push(b * b);\n    }\n  }\n  trace(flag);\n}\n",
        ),
        (
            "cond_pair_append_wrong_order.ts",
            "function condPairAppendWrongOrder(flag: boolean, a: number, b: number, dst: number[]): void {\n  if (flag) {\n    if (b > 0) {\n      dst.push(b * b);\n    }\n    if (0 < a) {\n      dst.push(1 + a);\n    }\n  }\n  trace(flag);\n}\n",
        ),
        (
            "cond_pair_append_wrong_receiver.ts",
            "function condPairAppendWrongReceiver(flag: boolean, a: number, b: number, dst: number[], other: number[]): void {\n  if (flag) {\n    if (0 < a) {\n      dst.push(1 + a);\n    }\n    if (b > 0) {\n      other.push(b * b);\n    }\n  }\n  trace(flag);\n}\n",
        ),
        (
            "cond_pair_append_mutated.ts",
            "function condPairAppendMutated(flag: boolean, a: number, b: number, dst: number[]): void {\n  dst.push(0);\n  if (flag) {\n    if (0 < a) {\n      dst.push(1 + a);\n    }\n    if (b > 0) {\n      dst.push(b * b);\n    }\n  }\n  trace(flag);\n}\n",
        ),
        (
            "cond_pair_append_third.ts",
            "function condPairAppendThird(flag: boolean, a: number, b: number, c: number, dst: number[]): void {\n  if (flag) {\n    if (0 < a) {\n      dst.push(1 + a);\n    }\n    if (b > 0) {\n      dst.push(b * b);\n    }\n    if (c > 0) {\n      dst.push(c + 3);\n    }\n  }\n  trace(flag);\n}\n",
        ),
        (
            "cond_pair_append_a.py",
            "def cond_pair_append_left(flag: bool, x: int, y: int, out: list[int]):\n    if flag:\n        if x > 0:\n            out.append(x + 1)\n        if y > 0:\n            out.append(y * y)\n    audit(flag)\n",
        ),
        (
            "cond_pair_append_b.py",
            "def cond_pair_append_right(enabled: bool, a: int, b: int, dst: list[int]):\n    if enabled:\n        if 0 < a:\n            dst.append(1 + a)\n        if b > 0:\n            dst.append(b * b)\n    trace(enabled)\n",
        ),
        (
            "cond_pair_append_wrong_guard.py",
            "def cond_pair_append_wrong_guard(flag: bool, a: int, b: int, dst: list[int]):\n    if flag:\n        if 0 < a:\n            dst.append(1 + a)\n        if b >= 0:\n            dst.append(b * b)\n    trace(flag)\n",
        ),
        (
            "cond_pair_append_wrong_order.py",
            "def cond_pair_append_wrong_order(flag: bool, a: int, b: int, dst: list[int]):\n    if flag:\n        if b > 0:\n            dst.append(b * b)\n        if 0 < a:\n            dst.append(1 + a)\n    trace(flag)\n",
        ),
        (
            "cond_pair_index_a.go",
            "package p\nfunc condPairIndexLeft(flag bool, x int, y int, out []int) {\n  if flag {\n    if x > 0 {\n      out[0] = x + 1\n    }\n    if y > 0 {\n      out[1] = y * y\n    }\n  }\n  audit(out)\n}\n",
        ),
        (
            "cond_pair_index_b.go",
            "package p\nfunc condPairIndexRight(enabled bool, a int, b int, dst []int) {\n  if enabled {\n    if 0 < a {\n      dst[0] = 1 + a\n    }\n    if b > 0 {\n      dst[1] = b * b\n    }\n  }\n  trace(dst)\n}\n",
        ),
        (
            "cond_pair_index_wrong_index.go",
            "package p\nfunc condPairIndexWrongIndex(flag bool, a int, b int, dst []int) {\n  if flag {\n    if 0 < a {\n      dst[0] = 1 + a\n    }\n    if b > 0 {\n      dst[2] = b * b\n    }\n  }\n  trace(dst)\n}\n",
        ),
        (
            "cond_pair_index_wrong_receiver.go",
            "package p\nfunc condPairIndexWrongReceiver(flag bool, a int, b int, dst []int, other []int) {\n  if flag {\n    if 0 < a {\n      dst[0] = 1 + a\n    }\n    if b > 0 {\n      other[1] = b * b\n    }\n  }\n  trace(dst)\n}\n",
        ),
    ];
    for (name, src) in fixtures {
        fs::write(dir.join(name), src).unwrap();
    }

    let out = run(&[
        "query",
        dir.to_str().unwrap(),
        "--mode",
        "semantic",
        "--min-lines",
        "100",
        "--min-size",
        "100",
        "--format",
        "json",
        "top=0",
    ]);
    let json = query_json(&out);
    let families = query_families(&json);

    let assert_branch_pair =
        |left: &str, right: &str, negative: &str, start_line: u64, end_line: u64| {
            let family = block_branch_pair_family(
                families, left, right, negative, start_line, end_line,
            )
            .unwrap_or_else(|| {
                panic!("missing ordered conditional-effect branch family {left}/{right}: {out}")
            });
            assert!(
            family["locations"]
                .as_array()
                .expect("locations")
                .iter()
                .all(|loc| loc["kind"] == "Block"),
            "ordered conditional-effect branch fragments should report as Block units: {family:?}"
        );
        };

    let assert_no_branch_pair = |left: &str,
                                 right: &str,
                                 left_start: u64,
                                 left_end: u64,
                                 right_start: u64,
                                 right_end: u64| {
        assert!(
            !families_pair_locations(
                families,
                (left, left_start, left_end),
                (right, right_start, right_end),
            ),
            "ordered conditional-effect branch boundary must not merge {left}/{right}: {out}"
        );
    };

    assert_branch_pair(
        "cond_pair_append_a.ts",
        "cond_pair_append_b.ts",
        "cond_pair_append_wrong_order.ts",
        2,
        9,
    );
    assert_branch_pair(
        "cond_pair_append_a.py",
        "cond_pair_append_b.py",
        "cond_pair_append_wrong_guard.py",
        2,
        6,
    );
    assert_branch_pair(
        "cond_pair_index_a.go",
        "cond_pair_index_b.go",
        "cond_pair_index_wrong_index.go",
        3,
        10,
    );

    assert_no_branch_pair(
        "cond_pair_append_a.ts",
        "cond_pair_append_wrong_order.ts",
        2,
        9,
        2,
        9,
    );
    assert_no_branch_pair(
        "cond_pair_append_a.ts",
        "cond_pair_append_wrong_receiver.ts",
        2,
        9,
        2,
        9,
    );
    assert_no_branch_pair(
        "cond_pair_append_a.ts",
        "cond_pair_append_mutated.ts",
        2,
        9,
        3,
        10,
    );
    assert_no_branch_pair(
        "cond_pair_append_a.ts",
        "cond_pair_append_third.ts",
        2,
        9,
        2,
        12,
    );
    assert_no_branch_pair(
        "cond_pair_append_a.py",
        "cond_pair_append_wrong_guard.py",
        2,
        6,
        2,
        6,
    );
    assert_no_branch_pair(
        "cond_pair_append_a.py",
        "cond_pair_append_wrong_order.py",
        2,
        6,
        2,
        6,
    );
    assert_no_branch_pair(
        "cond_pair_index_a.go",
        "cond_pair_index_wrong_index.go",
        3,
        10,
        3,
        10,
    );
    assert_no_branch_pair(
        "cond_pair_index_a.go",
        "cond_pair_index_wrong_receiver.go",
        3,
        10,
        3,
        10,
    );

    let _ = fs::remove_dir_all(&dir);
}

// Broad fixture matrix for ordered conditional mixed-effect branch boundaries. The size is
// intentional until the fixture setup has a clearer table-builder abstraction.
#[allow(clippy::too_many_lines)]
#[test]
fn semantic_query_reports_exact_safe_ordered_conditional_mixed_effect_branch_fragments() {
    let dir = std::env::temp_dir().join(format!(
        "nose_ordered_conditional_mixed_effect_branch_fragments_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    let fixtures = [
        (
            "cond_mixed_append_a.ts",
            "function condMixedAppendLeft(enabled: boolean, x: number, y: number, out: number[]): void {\n  if (enabled) {\n    if (x > 0) {\n      out.push(x + 1);\n    }\n    out.push(y * y);\n  }\n  audit(enabled);\n}\n",
        ),
        (
            "cond_mixed_append_b.ts",
            "function condMixedAppendRight(flag: boolean, a: number, b: number, dst: number[]): void {\n  if (flag) {\n    if (0 < a) {\n      dst.push(1 + a);\n    }\n    dst.push(b * b);\n  }\n  trace(flag);\n}\n",
        ),
        (
            "cond_mixed_append_wrong_order.ts",
            "function condMixedAppendWrongOrder(flag: boolean, a: number, b: number, dst: number[]): void {\n  if (flag) {\n    dst.push(b * b);\n    if (0 < a) {\n      dst.push(1 + a);\n    }\n  }\n  trace(flag);\n}\n",
        ),
        (
            "cond_mixed_append_wrong_guard.ts",
            "function condMixedAppendWrongGuard(flag: boolean, a: number, b: number, dst: number[]): void {\n  if (flag) {\n    if (0 <= a) {\n      dst.push(1 + a);\n    }\n    dst.push(b * b);\n  }\n  trace(flag);\n}\n",
        ),
        (
            "cond_mixed_append_wrong_receiver.ts",
            "function condMixedAppendWrongReceiver(flag: boolean, a: number, b: number, dst: number[], other: number[]): void {\n  if (flag) {\n    if (0 < a) {\n      dst.push(1 + a);\n    }\n    other.push(b * b);\n  }\n  trace(flag);\n}\n",
        ),
        (
            "cond_mixed_append_mutated.ts",
            "function condMixedAppendMutated(flag: boolean, a: number, b: number, dst: number[]): void {\n  dst.push(0);\n  if (flag) {\n    if (0 < a) {\n      dst.push(1 + a);\n    }\n    dst.push(b * b);\n  }\n  trace(flag);\n}\n",
        ),
        (
            "cond_mixed_append_third.ts",
            "function condMixedAppendThird(flag: boolean, a: number, b: number, c: number, dst: number[]): void {\n  if (flag) {\n    if (0 < a) {\n      dst.push(1 + a);\n    }\n    dst.push(b * b);\n    dst.push(c + 3);\n  }\n  trace(flag);\n}\n",
        ),
        (
            "cond_mixed_append_a.py",
            "def cond_mixed_append_left(flag: bool, x: int, y: int, out: list[int]):\n    if flag:\n        out.append(y * y)\n        if x > 0:\n            out.append(x + 1)\n    audit(flag)\n",
        ),
        (
            "cond_mixed_append_b.py",
            "def cond_mixed_append_right(enabled: bool, a: int, b: int, dst: list[int]):\n    if enabled:\n        dst.append(b * b)\n        if 0 < a:\n            dst.append(1 + a)\n    trace(enabled)\n",
        ),
        (
            "cond_mixed_append_wrong_order.py",
            "def cond_mixed_append_wrong_order(flag: bool, a: int, b: int, dst: list[int]):\n    if flag:\n        if 0 < a:\n            dst.append(1 + a)\n        dst.append(b * b)\n    trace(flag)\n",
        ),
        (
            "cond_mixed_append_wrong_guard.py",
            "def cond_mixed_append_wrong_guard(flag: bool, a: int, b: int, dst: list[int]):\n    if flag:\n        dst.append(b * b)\n        if 0 <= a:\n            dst.append(1 + a)\n    trace(flag)\n",
        ),
        (
            "cond_mixed_index_a.go",
            "package p\nfunc condMixedIndexLeft(flag bool, x int, y int, out []int) {\n  if flag {\n    if x > 0 {\n      out[0] = x + 1\n    }\n    out[1] = y * y\n  }\n  audit(out)\n}\n",
        ),
        (
            "cond_mixed_index_b.go",
            "package p\nfunc condMixedIndexRight(enabled bool, a int, b int, dst []int) {\n  if enabled {\n    if 0 < a {\n      dst[0] = 1 + a\n    }\n    dst[1] = b * b\n  }\n  trace(dst)\n}\n",
        ),
        (
            "cond_mixed_index_wrong_index.go",
            "package p\nfunc condMixedIndexWrongIndex(flag bool, a int, b int, dst []int) {\n  if flag {\n    if 0 < a {\n      dst[0] = 1 + a\n    }\n    dst[2] = b * b\n  }\n  trace(dst)\n}\n",
        ),
        (
            "cond_mixed_index_wrong_receiver.go",
            "package p\nfunc condMixedIndexWrongReceiver(flag bool, a int, b int, dst []int, other []int) {\n  if flag {\n    if 0 < a {\n      dst[0] = 1 + a\n    }\n    other[1] = b * b\n  }\n  trace(dst)\n}\n",
        ),
    ];
    for (name, src) in fixtures {
        fs::write(dir.join(name), src).unwrap();
    }

    let out = run(&[
        "query",
        dir.to_str().unwrap(),
        "--mode",
        "semantic",
        "--min-lines",
        "100",
        "--min-size",
        "100",
        "--format",
        "json",
        "top=0",
    ]);
    let json = query_json(&out);
    let families = query_families(&json);

    let assert_branch_pair = |left: &str,
                              right: &str,
                              negative: &str,
                              start_line: u64,
                              end_line: u64| {
        let family = block_branch_pair_family(
            families, left, right, negative, start_line, end_line,
        )
        .unwrap_or_else(|| {
            panic!("missing ordered conditional mixed-effect branch family {left}/{right}: {out}")
        });
        assert!(
                family["locations"]
                    .as_array()
                    .expect("locations")
                    .iter()
                    .all(|loc| loc["kind"] == "Block"),
                "ordered conditional mixed-effect branch fragments should report as Block units: {family:?}"
            );
    };

    let assert_no_branch_pair = |left: &str,
                                 right: &str,
                                 left_start: u64,
                                 left_end: u64,
                                 right_start: u64,
                                 right_end: u64| {
        assert!(
            !families_pair_locations(
                families,
                (left, left_start, left_end),
                (right, right_start, right_end),
            ),
            "ordered conditional mixed-effect branch boundary must not merge {left}/{right}: {out}"
        );
    };

    assert_branch_pair(
        "cond_mixed_append_a.ts",
        "cond_mixed_append_b.ts",
        "cond_mixed_append_wrong_order.ts",
        2,
        7,
    );
    assert_branch_pair(
        "cond_mixed_append_a.py",
        "cond_mixed_append_b.py",
        "cond_mixed_append_wrong_guard.py",
        2,
        5,
    );
    assert_branch_pair(
        "cond_mixed_index_a.go",
        "cond_mixed_index_b.go",
        "cond_mixed_index_wrong_index.go",
        3,
        8,
    );

    assert_no_branch_pair(
        "cond_mixed_append_a.ts",
        "cond_mixed_append_wrong_order.ts",
        2,
        7,
        2,
        7,
    );
    assert_no_branch_pair(
        "cond_mixed_append_a.ts",
        "cond_mixed_append_wrong_guard.ts",
        2,
        7,
        2,
        7,
    );
    assert_no_branch_pair(
        "cond_mixed_append_a.ts",
        "cond_mixed_append_wrong_receiver.ts",
        2,
        7,
        2,
        7,
    );
    assert_no_branch_pair(
        "cond_mixed_append_a.ts",
        "cond_mixed_append_mutated.ts",
        2,
        7,
        3,
        8,
    );
    assert_no_branch_pair(
        "cond_mixed_append_a.ts",
        "cond_mixed_append_third.ts",
        2,
        7,
        2,
        8,
    );
    assert_no_branch_pair(
        "cond_mixed_append_a.py",
        "cond_mixed_append_wrong_order.py",
        2,
        5,
        2,
        5,
    );
    assert_no_branch_pair(
        "cond_mixed_append_a.py",
        "cond_mixed_append_wrong_guard.py",
        2,
        5,
        2,
        5,
    );
    assert_no_branch_pair(
        "cond_mixed_index_a.go",
        "cond_mixed_index_wrong_index.go",
        3,
        8,
        3,
        8,
    );
    assert_no_branch_pair(
        "cond_mixed_index_a.go",
        "cond_mixed_index_wrong_receiver.go",
        3,
        8,
        3,
        8,
    );

    let _ = fs::remove_dir_all(&dir);
}
