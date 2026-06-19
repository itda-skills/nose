use super::*;

#[test]
fn semantic_scan_reports_exact_safe_return_fragments_under_opaque_functions() {
    let fixtures = [
        (
            "arith_a.py",
            "def arith_left(xs):\n    return (xs[0] + 1) * 2\n    audit(xs)\n",
        ),
        (
            "arith_b.py",
            "def arith_right(ys):\n    return 2 * (ys[0] + 1)\n    trace(ys)\n",
        ),
        (
            "arith_neg.py",
            "def arith_wrong(zs):\n    return (zs[0] + 2) * 2\n    audit(zs)\n",
        ),
        (
            "squares_a.py",
            "def squares_left(xs):\n    return xs[0] * xs[0] + xs[1] * xs[1]\n    audit(xs)\n",
        ),
        (
            "squares_b.py",
            "def squares_right(ys):\n    return ys[1] * ys[1] + ys[0] * ys[0]\n    trace(ys)\n",
        ),
        (
            "squares_neg.py",
            "def squares_wrong(zs):\n    return zs[0] * zs[0] - zs[1] * zs[1]\n    audit(zs)\n",
        ),
        (
            "product_a.py",
            "def product_left(xs):\n    return (xs[0] + xs[1]) * (xs[2] + 4)\n    audit(xs)\n",
        ),
        (
            "product_b.py",
            "def product_right(ys):\n    return (4 + ys[2]) * (ys[0] + ys[1])\n    trace(ys)\n",
        ),
        (
            "product_neg.py",
            "def product_wrong(zs):\n    return (zs[0] + zs[1]) * (zs[2] + 5)\n    audit(zs)\n",
        ),
    ];
    let (dir, out, families) = scan_fragment_fixtures("nose_exact_return_fragments", &fixtures);

    assert_block_pair_family(
        &families,
        &out,
        "arith_a.py",
        "arith_b.py",
        "arith_neg.py",
        "return",
    );
    assert_block_pair_family(
        &families,
        &out,
        "squares_a.py",
        "squares_b.py",
        "squares_neg.py",
        "return",
    );
    assert_block_pair_family(
        &families,
        &out,
        "product_a.py",
        "product_b.py",
        "product_neg.py",
        "return",
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn semantic_scan_reports_exact_safe_conditional_return_fragments_under_opaque_functions() {
    let fixtures = [
        (
            "square_guard_a.py",
            "def square_guard_left(xs):\n    if xs[0] > 0:\n        return xs[0] * xs[0]\n    audit(xs)\n",
        ),
        (
            "square_guard_b.py",
            "def square_guard_right(ys):\n    if 0 < ys[0]:\n        return ys[0] * ys[0]\n    trace(ys)\n",
        ),
        (
            "square_guard_neg.py",
            "def square_guard_wrong(zs):\n    if zs[0] > 1:\n        return zs[0] * zs[0]\n    audit(zs)\n",
        ),
        (
            "sum_guard_a.py",
            "def sum_guard_left(xs):\n    if xs[0] + xs[1] > 10:\n        return xs[0] + xs[1]\n    audit(xs)\n",
        ),
        (
            "sum_guard_b.py",
            "def sum_guard_right(ys):\n    if 10 < ys[0] + ys[1]:\n        return ys[0] + ys[1]\n    trace(ys)\n",
        ),
        (
            "sum_guard_neg.py",
            "def sum_guard_wrong(zs):\n    if zs[0] + zs[1] > 10:\n        return zs[0] - zs[1]\n    audit(zs)\n",
        ),
        (
            "both_guard_a.py",
            "def both_guard_left(xs):\n    if xs[0] > 0 and xs[1] > 0:\n        return xs[0] + xs[1]\n    audit(xs)\n",
        ),
        (
            "both_guard_b.py",
            "def both_guard_right(ys):\n    if ys[1] > 0 and ys[0] > 0:\n        return ys[0] + ys[1]\n    trace(ys)\n",
        ),
        (
            "both_guard_mutated.py",
            "def both_guard_mutated(zs):\n    zs.append(1)\n    if zs[0] > 0 and zs[1] > 0:\n        return zs[0] + zs[1]\n    audit(zs)\n",
        ),
    ];
    let (dir, out, families) = scan_fragment_fixtures("nose_exact_guard_fragments", &fixtures);

    assert_block_pair_family(
        &families,
        &out,
        "square_guard_a.py",
        "square_guard_b.py",
        "square_guard_neg.py",
        "conditional return",
    );
    assert_block_pair_family(
        &families,
        &out,
        "sum_guard_a.py",
        "sum_guard_b.py",
        "sum_guard_neg.py",
        "conditional return",
    );
    assert_block_pair_family(
        &families,
        &out,
        "both_guard_a.py",
        "both_guard_b.py",
        "both_guard_mutated.py",
        "conditional return",
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn semantic_scan_reports_exact_safe_conditional_throw_fragments_under_opaque_functions() {
    let fixtures = [
        (
            "square_throw_guard_a.ts",
            "function squareThrowLeft(xs: number[]) {\n  if (xs[0] > 0) {\n    throw xs[0] * xs[0];\n  }\n  audit(xs);\n}\n",
        ),
        (
            "square_throw_guard_b.ts",
            "function squareThrowRight(ys: number[]) {\n  if (0 < ys[0]) {\n    throw ys[0] * ys[0];\n  }\n  trace(ys);\n}\n",
        ),
        (
            "square_throw_guard_neg.ts",
            "function squareThrowWrong(zs: number[]) {\n  if (zs[0] > 1) {\n    throw zs[0] * zs[0];\n  }\n  audit(zs);\n}\n",
        ),
        (
            "sum_throw_guard_a.ts",
            "function sumThrowLeft(xs: number[]) {\n  if (xs[0] + xs[1] > 10) {\n    throw xs[0] + xs[1];\n  }\n  audit(xs);\n}\n",
        ),
        (
            "sum_throw_guard_b.ts",
            "function sumThrowRight(ys: number[]) {\n  if (10 < ys[0] + ys[1]) {\n    throw ys[0] + ys[1];\n  }\n  trace(ys);\n}\n",
        ),
        (
            "sum_throw_guard_neg.ts",
            "function sumThrowWrong(zs: number[]) {\n  if (zs[0] + zs[1] > 10) {\n    throw zs[0] - zs[1];\n  }\n  audit(zs);\n}\n",
        ),
        (
            "both_throw_guard_a.ts",
            "function bothThrowLeft(x: number, y: number) {\n  if (x > 0 && y > 0) {\n    throw x + y;\n  }\n  audit(x);\n}\n",
        ),
        (
            "both_throw_guard_b.ts",
            "function bothThrowRight(a: number, b: number) {\n  if (b > 0 && a > 0) {\n    throw a + b;\n  }\n  trace(a);\n}\n",
        ),
        (
            "both_throw_guard_mutated.ts",
            "function bothThrowMutated(z: number, w: number) {\n  z = z + 1;\n  if (z > 0 && w > 0) {\n    throw z + w;\n  }\n  audit(z);\n}\n",
        ),
    ];
    let (dir, out, families) =
        scan_fragment_fixtures("nose_exact_throw_guard_fragments", &fixtures);

    assert_block_pair_family(
        &families,
        &out,
        "square_throw_guard_a.ts",
        "square_throw_guard_b.ts",
        "square_throw_guard_neg.ts",
        "conditional throw",
    );
    assert_block_pair_family(
        &families,
        &out,
        "sum_throw_guard_a.ts",
        "sum_throw_guard_b.ts",
        "sum_throw_guard_neg.ts",
        "conditional throw",
    );
    assert_block_pair_family(
        &families,
        &out,
        "both_throw_guard_a.ts",
        "both_throw_guard_b.ts",
        "both_throw_guard_mutated.ts",
        "conditional throw",
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn semantic_scan_reports_exact_safe_empty_branch_conditional_exit_fragments_under_opaque_functions()
{
    let fixtures = [
        (
            "empty_else_return_a.ts",
            "function emptyElseReturnLeft(xs: number[]) {\n  if (xs[0] > 0) {\n    return xs[0] * xs[0];\n  } else {\n  }\n  audit(xs);\n}\n",
        ),
        (
            "empty_else_return_b.ts",
            "function emptyElseReturnRight(ys: number[]) {\n  if (0 < ys[0]) {\n    return ys[0] * ys[0];\n  } else {\n  }\n  trace(ys);\n}\n",
        ),
        (
            "empty_else_return_neg.ts",
            "function emptyElseReturnWrong(zs: number[]) {\n  if (zs[0] > 1) {\n    return zs[0] * zs[0];\n  } else {\n  }\n  audit(zs);\n}\n",
        ),
        (
            "empty_else_throw_a.ts",
            "function emptyElseThrowLeft(xs: number[]) {\n  if (xs[0] + xs[1] > 10) {\n    throw xs[0] + xs[1];\n  } else {\n  }\n  audit(xs);\n}\n",
        ),
        (
            "empty_else_throw_b.ts",
            "function emptyElseThrowRight(ys: number[]) {\n  if (10 < ys[0] + ys[1]) {\n    throw ys[0] + ys[1];\n  } else {\n  }\n  trace(ys);\n}\n",
        ),
        (
            "empty_else_throw_neg.ts",
            "function emptyElseThrowWrong(zs: number[]) {\n  if (zs[0] + zs[1] > 10) {\n    throw zs[0] - zs[1];\n  } else {\n  }\n  audit(zs);\n}\n",
        ),
        (
            "empty_then_throw_a.ts",
            "function emptyThenThrowLeft(x: number, y: number) {\n  if (x > 0 && y > 0) {\n  } else {\n    throw x + y;\n  }\n  audit(x);\n}\n",
        ),
        (
            "empty_then_throw_b.ts",
            "function emptyThenThrowRight(a: number, b: number) {\n  if (b > 0 && a > 0) {\n  } else {\n    throw a + b;\n  }\n  trace(a);\n}\n",
        ),
        (
            "empty_then_throw_mutated.ts",
            "function emptyThenThrowMutated(z: number, w: number) {\n  z = z + 1;\n  if (z > 0 && w > 0) {\n  } else {\n    throw z + w;\n  }\n  audit(z);\n}\n",
        ),
    ];
    let (dir, out, families) =
        scan_fragment_fixtures("nose_exact_empty_branch_fragments", &fixtures);

    assert_block_pair_family(
        &families,
        &out,
        "empty_else_return_a.ts",
        "empty_else_return_b.ts",
        "empty_else_return_neg.ts",
        "empty-branch conditional",
    );
    assert_block_pair_family(
        &families,
        &out,
        "empty_else_throw_a.ts",
        "empty_else_throw_b.ts",
        "empty_else_throw_neg.ts",
        "empty-branch conditional",
    );
    assert_block_pair_family(
        &families,
        &out,
        "empty_then_throw_a.ts",
        "empty_then_throw_b.ts",
        "empty_then_throw_mutated.ts",
        "empty-branch conditional",
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn semantic_scan_reports_exact_safe_conditional_bare_return_fragments_under_opaque_functions() {
    let fixtures = [
        (
            "bare_square_a.ts",
            "function bareSquareLeft(xs: number[]) {\n  if (xs[0] > 0) {\n    return;\n  }\n  audit(xs);\n}\n",
        ),
        (
            "bare_square_b.ts",
            "function bareSquareRight(ys: number[]) {\n  if (0 < ys[0]) {\n    return;\n  }\n  trace(ys);\n}\n",
        ),
        (
            "bare_square_neg.ts",
            "function bareSquareWrong(zs: number[]) {\n  if (zs[0] > 1) {\n    return;\n  }\n  audit(zs);\n}\n",
        ),
        (
            "bare_sum_a.ts",
            "function bareSumLeft(xs: number[]) {\n  if (xs[0] + xs[1] > 10) {\n    return;\n  }\n  audit(xs);\n}\n",
        ),
        (
            "bare_sum_b.ts",
            "function bareSumRight(ys: number[]) {\n  if (10 < ys[0] + ys[1]) {\n    return;\n  }\n  trace(ys);\n}\n",
        ),
        (
            "bare_sum_mutated.ts",
            "function bareSumMutated(zs: number[]) {\n  zs.push(1);\n  if (zs[0] + zs[1] > 10) {\n    return;\n  }\n  audit(zs);\n}\n",
        ),
        (
            "bare_else_a.ts",
            "function bareElseLeft(x: number, y: number) {\n  if (x > 0 && y > 0) {\n  } else {\n    return;\n  }\n  audit(x);\n}\n",
        ),
        (
            "bare_else_b.ts",
            "function bareElseRight(a: number, b: number) {\n  if (b > 0 && a > 0) {\n  } else {\n    return;\n  }\n  trace(a);\n}\n",
        ),
        (
            "bare_else_neg.ts",
            "function bareElseWrong(z: number, w: number) {\n  if (z > 0 && w > 1) {\n  } else {\n    return;\n  }\n  audit(z);\n}\n",
        ),
    ];
    let (dir, out, families) =
        scan_fragment_fixtures("nose_exact_bare_return_fragments", &fixtures);

    assert_block_pair_family(
        &families,
        &out,
        "bare_square_a.ts",
        "bare_square_b.ts",
        "bare_square_neg.ts",
        "conditional bare-return",
    );
    assert_block_pair_family(
        &families,
        &out,
        "bare_sum_a.ts",
        "bare_sum_b.ts",
        "bare_sum_mutated.ts",
        "conditional bare-return",
    );
    assert_block_pair_family(
        &families,
        &out,
        "bare_else_a.ts",
        "bare_else_b.ts",
        "bare_else_neg.ts",
        "conditional bare-return",
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn semantic_scan_reports_exact_safe_conditional_expr_effect_fragments_under_opaque_functions() {
    let fixtures = [
        (
            "push_square_a.ts",
            "function pushSquareLeft(xs: number[], out: number[]) {\n  if (xs[0] > 0) {\n    out.push(xs[0] * xs[0]);\n  }\n  audit(xs);\n}\n",
        ),
        (
            "push_square_b.ts",
            "function pushSquareRight(ys: number[], dst: number[]) {\n  if (0 < ys[0]) {\n    dst.push(ys[0] * ys[0]);\n  }\n  trace(ys);\n}\n",
        ),
        (
            "push_square_neg.ts",
            "function pushSquareWrong(zs: number[], out: number[]) {\n  if (zs[0] > 1) {\n    out.push(zs[0] * zs[0]);\n  }\n  audit(zs);\n}\n",
        ),
        (
            "push_sum_a.ts",
            "function pushSumLeft(xs: number[], out: number[]) {\n  if (xs[0] + xs[1] > 10) {\n    out.push(xs[0] + xs[1]);\n  }\n  audit(xs);\n}\n",
        ),
        (
            "push_sum_b.ts",
            "function pushSumRight(ys: number[], dst: number[]) {\n  if (10 < ys[0] + ys[1]) {\n    dst.push(ys[0] + ys[1]);\n  }\n  trace(ys);\n}\n",
        ),
        (
            "push_sum_neg.ts",
            "function pushSumWrong(zs: number[], out: number[]) {\n  if (zs[0] + zs[1] > 10) {\n    out.push(zs[0] - zs[1]);\n  }\n  audit(zs);\n}\n",
        ),
        (
            "push_else_a.ts",
            "function pushElseLeft(x: number, y: number, out: number[]) {\n  if (x > 0 && y > 0) {\n  } else {\n    out.push(x + y);\n  }\n  audit(x);\n}\n",
        ),
        (
            "push_else_b.ts",
            "function pushElseRight(a: number, b: number, dst: number[]) {\n  if (b > 0 && a > 0) {\n  } else {\n    dst.push(a + b);\n  }\n  trace(a);\n}\n",
        ),
        (
            "push_else_mutated.ts",
            "function pushElseMutated(z: number, w: number, out: number[]) {\n  out.push(0);\n  if (z > 0 && w > 0) {\n  } else {\n    out.push(z + w);\n  }\n  audit(z);\n}\n",
        ),
    ];
    let (dir, out, families) =
        scan_fragment_fixtures("nose_exact_expr_effect_fragments", &fixtures);

    assert_block_pair_family(
        &families,
        &out,
        "push_square_a.ts",
        "push_square_b.ts",
        "push_square_neg.ts",
        "conditional expression-effect",
    );
    assert_block_pair_family(
        &families,
        &out,
        "push_sum_a.ts",
        "push_sum_b.ts",
        "push_sum_neg.ts",
        "conditional expression-effect",
    );
    assert_block_pair_family(
        &families,
        &out,
        "push_else_a.ts",
        "push_else_b.ts",
        "push_else_mutated.ts",
        "conditional expression-effect",
    );
    let _ = fs::remove_dir_all(&dir);
}
