use super::*;

// Broad fixture matrix for ordered loop conditional-effect branch boundaries. The size is
// intentional until the fixture setup has a clearer table-builder abstraction.
#[allow(clippy::too_many_lines)]
#[test]
fn semantic_scan_reports_exact_safe_ordered_loop_conditional_effect_branch_fragments() {
    let dir = std::env::temp_dir().join(format!(
        "nose_ordered_loop_conditional_effect_branch_fragments_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    let fixtures = [
        (
            "loop_cond_append_a.ts",
            "function loopCondAppendLeft(enabled: boolean, xs: number[], y: number, out: number[]): void {\n  if (enabled) {\n    for (const x of xs) {\n      out.push(x + 1);\n    }\n    if (y > 0) {\n      out.push(y * y);\n    }\n  }\n  audit(enabled);\n}\n",
        ),
        (
            "loop_cond_append_b.ts",
            "function loopCondAppendRight(flag: boolean, ys: number[], b: number, dst: number[]): void {\n  if (flag) {\n    for (const a of ys) {\n      dst.push(a + 1);\n    }\n    if (b > 0) {\n      dst.push(b * b);\n    }\n  }\n  trace(flag);\n}\n",
        ),
        (
            "loop_cond_append_wrong_order.ts",
            "function loopCondAppendWrongOrder(flag: boolean, ys: number[], b: number, dst: number[]): void {\n  if (flag) {\n    if (b > 0) {\n      dst.push(b * b);\n    }\n    for (const a of ys) {\n      dst.push(a + 1);\n    }\n  }\n  trace(flag);\n}\n",
        ),
        (
            "loop_cond_append_wrong_guard.ts",
            "function loopCondAppendWrongGuard(flag: boolean, ys: number[], b: number, dst: number[]): void {\n  if (flag) {\n    for (const a of ys) {\n      dst.push(a + 1);\n    }\n    if (b >= 0) {\n      dst.push(b * b);\n    }\n  }\n  trace(flag);\n}\n",
        ),
        (
            "loop_cond_append_wrong_receiver.ts",
            "function loopCondAppendWrongReceiver(flag: boolean, ys: number[], b: number, dst: number[], other: number[]): void {\n  if (flag) {\n    for (const a of ys) {\n      dst.push(a + 1);\n    }\n    if (b > 0) {\n      other.push(b * b);\n    }\n  }\n  trace(flag);\n}\n",
        ),
        (
            "loop_cond_append_mutated.ts",
            "function loopCondAppendMutated(flag: boolean, ys: number[], b: number, dst: number[]): void {\n  dst.push(0);\n  if (flag) {\n    for (const a of ys) {\n      dst.push(a + 1);\n    }\n    if (b > 0) {\n      dst.push(b * b);\n    }\n  }\n  trace(flag);\n}\n",
        ),
        (
            "loop_cond_append_third.ts",
            "function loopCondAppendThird(flag: boolean, ys: number[], b: number, c: number, dst: number[]): void {\n  if (flag) {\n    for (const a of ys) {\n      dst.push(a + 1);\n    }\n    if (b > 0) {\n      dst.push(b * b);\n    }\n    dst.push(c + 3);\n  }\n  trace(flag);\n}\n",
        ),
        (
            "loop_cond_append_a.py",
            "def loop_cond_append_left(flag: bool, xs: list[int], y: int, out: list[int]):\n    if flag:\n        if y > 0:\n            out.append(y * y)\n        for x in xs:\n            value = x + 1\n            out.append(value)\n    audit(flag)\n",
        ),
        (
            "loop_cond_append_b.py",
            "def loop_cond_append_right(enabled: bool, ys: list[int], b: int, dst: list[int]):\n    if enabled:\n        if b > 0:\n            dst.append(b * b)\n        for a in ys:\n            item = 1 + a\n            dst.append(item)\n    trace(enabled)\n",
        ),
        (
            "loop_cond_append_wrong_order.py",
            "def loop_cond_append_wrong_order(flag: bool, ys: list[int], b: int, dst: list[int]):\n    if flag:\n        for a in ys:\n            item = 1 + a\n            dst.append(item)\n        if b > 0:\n            dst.append(b * b)\n    trace(flag)\n",
        ),
        (
            "loop_cond_append_wrong_temp.py",
            "def loop_cond_append_wrong_temp(flag: bool, ys: list[int], b: int, dst: list[int]):\n    if flag:\n        if b > 0:\n            dst.append(b * b)\n        for a in ys:\n            item = 2 + a\n            dst.append(item)\n    trace(flag)\n",
        ),
        (
            "loop_cond_index_a.go",
            "package p\nfunc loopCondIndexLeft(flag bool, xs []int, y int, out []int) {\n  if flag {\n    for i, x := range xs {\n      out[i] = x * x\n    }\n    if y > 0 {\n      out[0] = y + 1\n    }\n  }\n  audit(out)\n}\n",
        ),
        (
            "loop_cond_index_b.go",
            "package p\nfunc loopCondIndexRight(enabled bool, ys []int, b int, dst []int) {\n  if enabled {\n    for j, a := range ys {\n      dst[j] = a * a\n    }\n    if b > 0 {\n      dst[0] = 1 + b\n    }\n  }\n  trace(dst)\n}\n",
        ),
        (
            "loop_cond_index_wrong_index.go",
            "package p\nfunc loopCondIndexWrongIndex(flag bool, ys []int, b int, dst []int) {\n  if flag {\n    for j, a := range ys {\n      dst[j] = a * a\n    }\n    if b > 0 {\n      dst[1] = 1 + b\n    }\n  }\n  trace(dst)\n}\n",
        ),
        (
            "loop_cond_index_wrong_receiver.go",
            "package p\nfunc loopCondIndexWrongReceiver(flag bool, ys []int, b int, dst []int, other []int) {\n  if flag {\n    for j, a := range ys {\n      dst[j] = a * a\n    }\n    if b > 0 {\n      other[0] = 1 + b\n    }\n  }\n  trace(dst)\n}\n",
        ),
    ];
    for (name, src) in fixtures {
        fs::write(dir.join(name), src).unwrap();
    }

    let out = run(&[
        "scan",
        dir.to_str().unwrap(),
        "--mode",
        "semantic",
        "--min-lines",
        "100",
        "--min-size",
        "100",
        "--format",
        "json",
        "--top",
        "0",
    ]);
    let json = scan_json(&out);
    let families = scan_families(&json);

    let assert_branch_pair = |left: &str,
                              right: &str,
                              negative: &str,
                              start_line: u64,
                              end_line: u64| {
        let family = block_branch_pair_family(
            families, left, right, negative, start_line, end_line,
        )
        .unwrap_or_else(|| {
            panic!("missing ordered loop conditional-effect branch family {left}/{right}: {out}")
        });
        assert!(
                family["locations"]
                    .as_array()
                    .expect("locations")
                    .iter()
                    .all(|loc| loc["kind"] == "Block"),
                "ordered loop conditional-effect branch fragments should report as Block units: {family:?}"
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
            "ordered loop conditional-effect branch boundary must not merge {left}/{right}: {out}"
        );
    };

    assert_branch_pair(
        "loop_cond_append_a.ts",
        "loop_cond_append_b.ts",
        "loop_cond_append_wrong_order.ts",
        2,
        9,
    );
    assert_branch_pair(
        "loop_cond_append_a.py",
        "loop_cond_append_b.py",
        "loop_cond_append_wrong_temp.py",
        2,
        7,
    );
    assert_branch_pair(
        "loop_cond_index_a.go",
        "loop_cond_index_b.go",
        "loop_cond_index_wrong_index.go",
        3,
        10,
    );

    assert_no_branch_pair(
        "loop_cond_append_a.ts",
        "loop_cond_append_wrong_order.ts",
        2,
        9,
        2,
        9,
    );
    assert_no_branch_pair(
        "loop_cond_append_a.ts",
        "loop_cond_append_wrong_guard.ts",
        2,
        9,
        2,
        9,
    );
    assert_no_branch_pair(
        "loop_cond_append_a.ts",
        "loop_cond_append_wrong_receiver.ts",
        2,
        9,
        2,
        9,
    );
    assert_no_branch_pair(
        "loop_cond_append_a.ts",
        "loop_cond_append_mutated.ts",
        2,
        9,
        3,
        10,
    );
    assert_no_branch_pair(
        "loop_cond_append_a.ts",
        "loop_cond_append_third.ts",
        2,
        9,
        2,
        10,
    );
    assert_no_branch_pair(
        "loop_cond_append_a.py",
        "loop_cond_append_wrong_order.py",
        2,
        7,
        2,
        7,
    );
    assert_no_branch_pair(
        "loop_cond_append_a.py",
        "loop_cond_append_wrong_temp.py",
        2,
        7,
        2,
        7,
    );
    assert_no_branch_pair(
        "loop_cond_index_a.go",
        "loop_cond_index_wrong_index.go",
        3,
        10,
        3,
        10,
    );
    assert_no_branch_pair(
        "loop_cond_index_a.go",
        "loop_cond_index_wrong_receiver.go",
        3,
        10,
        3,
        10,
    );

    let _ = fs::remove_dir_all(&dir);
}

// Broad fixture matrix for ordered loop conditional mixed-effect branch boundaries. The size is
// intentional until the fixture setup has a clearer table-builder abstraction.
#[allow(clippy::too_many_lines)]
#[test]
fn semantic_scan_reports_exact_safe_ordered_loop_conditional_mixed_effect_branch_fragments() {
    let dir = std::env::temp_dir().join(format!(
        "nose_ordered_loop_conditional_mixed_effect_branch_fragments_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    let fixtures = [
        (
            "loop_cond_mixed_append_a.ts",
            "function loopCondMixedLeft(enabled: boolean, xs: number[], y: number, z: number, out: number[]): void {\n  if (enabled) {\n    for (const x of xs) {\n      out.push(x + 1);\n    }\n    if (y > 0) {\n      out.push(y * y);\n    }\n    out.push(z + 3);\n  }\n  audit(enabled);\n}\n",
        ),
        (
            "loop_cond_mixed_append_b.ts",
            "function loopCondMixedRight(flag: boolean, ys: number[], b: number, c: number, dst: number[]): void {\n  if (flag) {\n    for (const a of ys) {\n      dst.push(a + 1);\n    }\n    if (b > 0) {\n      dst.push(b * b);\n    }\n    dst.push(c + 3);\n  }\n  trace(flag);\n}\n",
        ),
        (
            "loop_cond_mixed_append_wrong_order.ts",
            "function loopCondMixedWrongOrder(flag: boolean, ys: number[], b: number, c: number, dst: number[]): void {\n  if (flag) {\n    if (b > 0) {\n      dst.push(b * b);\n    }\n    for (const a of ys) {\n      dst.push(a + 1);\n    }\n    dst.push(c + 3);\n  }\n  trace(flag);\n}\n",
        ),
        (
            "loop_cond_mixed_append_wrong_guard.ts",
            "function loopCondMixedWrongGuard(flag: boolean, ys: number[], b: number, c: number, dst: number[]): void {\n  if (flag) {\n    for (const a of ys) {\n      dst.push(a + 1);\n    }\n    if (b >= 0) {\n      dst.push(b * b);\n    }\n    dst.push(c + 3);\n  }\n  trace(flag);\n}\n",
        ),
        (
            "loop_cond_mixed_append_wrong_receiver.ts",
            "function loopCondMixedWrongReceiver(flag: boolean, ys: number[], b: number, c: number, dst: number[], other: number[]): void {\n  if (flag) {\n    for (const a of ys) {\n      dst.push(a + 1);\n    }\n    if (b > 0) {\n      dst.push(b * b);\n    }\n    other.push(c + 3);\n  }\n  trace(flag);\n}\n",
        ),
        (
            "loop_cond_mixed_append_mutated.ts",
            "function loopCondMixedMutated(flag: boolean, ys: number[], b: number, c: number, dst: number[]): void {\n  dst.push(0);\n  if (flag) {\n    for (const a of ys) {\n      dst.push(a + 1);\n    }\n    if (b > 0) {\n      dst.push(b * b);\n    }\n    dst.push(c + 3);\n  }\n  trace(flag);\n}\n",
        ),
        (
            "loop_cond_mixed_append_fourth.ts",
            "function loopCondMixedFourth(flag: boolean, ys: number[], b: number, c: number, d: number, dst: number[]): void {\n  if (flag) {\n    for (const a of ys) {\n      dst.push(a + 1);\n    }\n    if (b > 0) {\n      dst.push(b * b);\n    }\n    dst.push(c + 3);\n    dst.push(d + 4);\n  }\n  trace(flag);\n}\n",
        ),
        (
            "loop_cond_mixed_append_a.py",
            "def loop_cond_mixed_left(flag: bool, xs: list[int], y: int, z: int, out: list[int]):\n    if flag:\n        if y > 0:\n            out.append(y * y)\n        for x in xs:\n            value = x + 1\n            out.append(value)\n        out.append(z + 3)\n    audit(flag)\n",
        ),
        (
            "loop_cond_mixed_append_b.py",
            "def loop_cond_mixed_right(enabled: bool, ys: list[int], b: int, c: int, dst: list[int]):\n    if enabled:\n        if b > 0:\n            dst.append(b * b)\n        for a in ys:\n            item = 1 + a\n            dst.append(item)\n        dst.append(3 + c)\n    trace(enabled)\n",
        ),
        (
            "loop_cond_mixed_append_wrong_order.py",
            "def loop_cond_mixed_wrong_order(flag: bool, ys: list[int], b: int, c: int, dst: list[int]):\n    if flag:\n        for a in ys:\n            item = 1 + a\n            dst.append(item)\n        if b > 0:\n            dst.append(b * b)\n        dst.append(3 + c)\n    trace(flag)\n",
        ),
        (
            "loop_cond_mixed_append_wrong_temp.py",
            "def loop_cond_mixed_wrong_temp(flag: bool, ys: list[int], b: int, c: int, dst: list[int]):\n    if flag:\n        if b > 0:\n            dst.append(b * b)\n        for a in ys:\n            item = 2 + a\n            dst.append(item)\n        dst.append(3 + c)\n    trace(flag)\n",
        ),
        (
            "loop_cond_mixed_index_a.go",
            "package p\nfunc loopCondMixedIndexLeft(flag bool, xs []int, y int, z int, out []int) {\n  if flag {\n    for i, x := range xs {\n      out[i] = x * x\n    }\n    if y > 0 {\n      out[0] = y + 1\n    }\n    out[1] = z + 3\n  }\n  audit(out)\n}\n",
        ),
        (
            "loop_cond_mixed_index_b.go",
            "package p\nfunc loopCondMixedIndexRight(enabled bool, ys []int, b int, c int, dst []int) {\n  if enabled {\n    for j, a := range ys {\n      dst[j] = a * a\n    }\n    if b > 0 {\n      dst[0] = 1 + b\n    }\n    dst[1] = 3 + c\n  }\n  trace(dst)\n}\n",
        ),
        (
            "loop_cond_mixed_index_wrong_index.go",
            "package p\nfunc loopCondMixedIndexWrongIndex(flag bool, ys []int, b int, c int, dst []int) {\n  if flag {\n    for j, a := range ys {\n      dst[j] = a * a\n    }\n    if b > 0 {\n      dst[0] = 1 + b\n    }\n    dst[2] = 3 + c\n  }\n  trace(dst)\n}\n",
        ),
        (
            "loop_cond_mixed_index_wrong_receiver.go",
            "package p\nfunc loopCondMixedIndexWrongReceiver(flag bool, ys []int, b int, c int, dst []int, other []int) {\n  if flag {\n    for j, a := range ys {\n      dst[j] = a * a\n    }\n    if b > 0 {\n      dst[0] = 1 + b\n    }\n    other[1] = 3 + c\n  }\n  trace(dst)\n}\n",
        ),
    ];
    for (name, src) in fixtures {
        fs::write(dir.join(name), src).unwrap();
    }

    let out = run(&[
        "scan",
        dir.to_str().unwrap(),
        "--mode",
        "semantic",
        "--min-lines",
        "100",
        "--min-size",
        "100",
        "--format",
        "json",
        "--top",
        "0",
    ]);
    let json = scan_json(&out);
    let families = scan_families(&json);

    let assert_branch_pair = |left: &str,
                              right: &str,
                              negative: &str,
                              start_line: u64,
                              end_line: u64| {
        let family =
            block_branch_pair_family(families, left, right, negative, start_line, end_line)
                .unwrap_or_else(|| {
                    panic!(
                "missing ordered loop conditional mixed-effect branch family {left}/{right}: {out}"
            )
                });
        assert!(
                family["locations"]
                    .as_array()
                    .expect("locations")
                    .iter()
                    .all(|loc| loc["kind"] == "Block"),
                "ordered loop conditional mixed-effect branch fragments should report as Block units: {family:?}"
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
            "ordered loop conditional mixed-effect branch boundary must not merge {left}/{right}: {out}"
        );
    };

    assert_branch_pair(
        "loop_cond_mixed_append_a.ts",
        "loop_cond_mixed_append_b.ts",
        "loop_cond_mixed_append_wrong_order.ts",
        2,
        10,
    );
    assert_branch_pair(
        "loop_cond_mixed_append_a.py",
        "loop_cond_mixed_append_b.py",
        "loop_cond_mixed_append_wrong_temp.py",
        2,
        8,
    );
    assert_branch_pair(
        "loop_cond_mixed_index_a.go",
        "loop_cond_mixed_index_b.go",
        "loop_cond_mixed_index_wrong_index.go",
        3,
        11,
    );

    assert_no_branch_pair(
        "loop_cond_mixed_append_a.ts",
        "loop_cond_mixed_append_wrong_order.ts",
        2,
        10,
        2,
        10,
    );
    assert_no_branch_pair(
        "loop_cond_mixed_append_a.ts",
        "loop_cond_mixed_append_wrong_guard.ts",
        2,
        10,
        2,
        10,
    );
    assert_no_branch_pair(
        "loop_cond_mixed_append_a.ts",
        "loop_cond_mixed_append_wrong_receiver.ts",
        2,
        10,
        2,
        10,
    );
    assert_no_branch_pair(
        "loop_cond_mixed_append_a.ts",
        "loop_cond_mixed_append_mutated.ts",
        2,
        10,
        3,
        11,
    );
    assert_no_branch_pair(
        "loop_cond_mixed_append_a.ts",
        "loop_cond_mixed_append_fourth.ts",
        2,
        10,
        2,
        11,
    );
    assert_no_branch_pair(
        "loop_cond_mixed_append_a.py",
        "loop_cond_mixed_append_wrong_order.py",
        2,
        8,
        2,
        8,
    );
    assert_no_branch_pair(
        "loop_cond_mixed_append_a.py",
        "loop_cond_mixed_append_wrong_temp.py",
        2,
        8,
        2,
        8,
    );
    assert_no_branch_pair(
        "loop_cond_mixed_index_a.go",
        "loop_cond_mixed_index_wrong_index.go",
        3,
        11,
        3,
        11,
    );
    assert_no_branch_pair(
        "loop_cond_mixed_index_a.go",
        "loop_cond_mixed_index_wrong_receiver.go",
        3,
        11,
        3,
        11,
    );

    let _ = fs::remove_dir_all(&dir);
}
