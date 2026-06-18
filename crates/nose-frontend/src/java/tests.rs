use super::*;

fn raw_kinds(src: &str) -> Vec<String> {
    let interner = Interner::new();
    lower(FileId(0), "T.java", src.as_bytes(), &interner)
        .expect("lower")
        .nodes
        .iter()
        .filter(|n| n.kind == NodeKind::Raw)
        .filter_map(|n| match n.payload {
            Payload::Name(s) => Some(interner.resolve(s).to_string()),
            _ => None,
        })
        .collect()
}

#[test]
fn local_record_and_annotation_declarations_do_not_fall_to_raw() {
    // Local type declarations are type metadata in this IL. They should follow the
    // same class-like lowering path as top-level declarations instead of surfacing
    // as opaque statement Raw nodes.
    let raw = raw_kinds(
        "class C { void f(){ record Pair(int a, int b) {} @interface Local { String value(); } } }",
    );
    assert!(
        raw.is_empty(),
        "local type declarations should be erased/lowered, got {raw:?}"
    );
}

fn unary_ops(src: &str) -> Vec<Op> {
    let interner = Interner::new();
    lower(FileId(0), "T.java", src.as_bytes(), &interner)
        .expect("lower")
        .nodes
        .iter()
        .filter(|n| n.kind == NodeKind::UnOp)
        .filter_map(|n| match n.payload {
            Payload::Op(op) => Some(op),
            _ => None,
        })
        .collect()
}

#[test]
fn unary_operators_lower_to_distinct_ops() {
    // `+x` must be Pos and `~x` BitNot, not both collapsed onto Neg.
    let ops = unary_ops(
        "class C { int f(int x){ return +x + -x + ~x; } boolean g(boolean b){ return !b; } }",
    );
    assert!(ops.contains(&Op::Pos), "unary + → Op::Pos, got {ops:?}");
    assert!(ops.contains(&Op::Neg), "unary - → Op::Neg, got {ops:?}");
    assert!(
        ops.contains(&Op::BitNot),
        "unary ~ → Op::BitNot, got {ops:?}"
    );
    assert!(ops.contains(&Op::Not), "unary ! → Op::Not, got {ops:?}");
}

fn binops(src: &str) -> Vec<Op> {
    let interner = Interner::new();
    lower(FileId(0), "T.java", src.as_bytes(), &interner)
        .expect("lower")
        .nodes
        .iter()
        .filter(|n| n.kind == NodeKind::BinOp)
        .filter_map(|n| match n.payload {
            Payload::Op(op) => Some(op),
            _ => None,
        })
        .collect()
}

fn switch_case_rhs_ints(src: &str) -> Vec<i64> {
    let interner = Interner::new();
    let il = lower(FileId(0), "T.java", src.as_bytes(), &interner).expect("lower");
    il.nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.kind == NodeKind::BinOp && n.payload == Payload::Op(Op::Eq))
        .filter_map(|(idx, _)| {
            let kids = il.children(NodeId(idx as u32));
            match kids {
                [_, rhs] => match il.node(*rhs).payload {
                    Payload::LitInt(value) => Some(value),
                    _ => None,
                },
                _ => None,
            }
        })
        .collect()
}

fn raw_names(src: &str) -> Vec<String> {
    let interner = Interner::new();
    let il = lower(FileId(0), "T.java", src.as_bytes(), &interner).expect("lower");
    il.nodes
        .iter()
        .filter(|node| node.kind == NodeKind::Raw)
        .filter_map(|node| match node.payload {
            Payload::Name(sym) => Some(interner.resolve(sym).to_string()),
            _ => None,
        })
        .collect()
}

fn switch_expression_branch_ints(src: &str) -> Vec<i64> {
    let interner = Interner::new();
    let il = lower(FileId(0), "T.java", src.as_bytes(), &interner).expect("lower");
    il.nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| node.kind == NodeKind::If)
        .find_map(|(idx, _)| {
            let kids = il.children(NodeId(idx as u32));
            match kids {
                [_, then_expr, else_expr] => {
                    match (il.node(*then_expr).payload, il.node(*else_expr).payload) {
                        (Payload::LitInt(then_value), Payload::LitInt(else_value)) => {
                            Some(vec![then_value, else_value])
                        }
                        _ => None,
                    }
                }
                _ => None,
            }
        })
        .unwrap_or_default()
}

fn switch_case_lhs_names(src: &str) -> Vec<String> {
    let interner = Interner::new();
    let il = lower(FileId(0), "T.java", src.as_bytes(), &interner).expect("lower");
    il.nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| node.kind == NodeKind::BinOp && node.payload == Payload::Op(Op::Eq))
        .filter_map(|(idx, _)| {
            let kids = il.children(NodeId(idx as u32));
            match kids {
                [lhs, _] => match il.node(*lhs).payload {
                    Payload::Name(sym) => Some(interner.resolve(sym).to_string()),
                    _ => None,
                },
                _ => None,
            }
        })
        .collect()
}

fn expr_stmt_ints(src: &str) -> Vec<i64> {
    let interner = Interner::new();
    let il = lower(FileId(0), "T.java", src.as_bytes(), &interner).expect("lower");
    il.nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.kind == NodeKind::ExprStmt)
        .filter_map(|(idx, _)| {
            let kids = il.children(NodeId(idx as u32));
            match kids {
                [expr] => match il.node(*expr).payload {
                    Payload::LitInt(value) => Some(value),
                    _ => None,
                },
                _ => None,
            }
        })
        .collect()
}

#[test]
fn switch_cases_compare_scrutinee_to_case_literals() {
    let src = "class C { int f(int x){ switch(x){ case 7: return 1; case 8: return 2; default: return 3; } } }";
    assert_eq!(switch_case_rhs_ints(src), vec![7, 8]);
    assert!(
        expr_stmt_ints(src).is_empty(),
        "case labels should not remain as stray expression statements"
    );
}

#[test]
fn switch_expression_rules_lower_to_expression_if_chain() {
    let src = "class C { int f(int x){ return switch (x) { case 1 -> 2; default -> 3; }; } }";
    assert_eq!(switch_case_rhs_ints(src), vec![1]);
    assert_eq!(switch_case_lhs_names(src), vec!["x"]);
    assert_eq!(switch_expression_branch_ints(src), vec![2, 3]);
    let raw = raw_names(src);
    assert!(
        !raw.iter()
            .any(|name| matches!(name.as_str(), "switch_expression" | "switch_rule")),
        "switch expression rules should lower without Raw nodes: {raw:?}"
    );
}

#[test]
fn switch_expression_yield_blocks_lower_to_branch_values() {
    let src = "class C { int f(int x){ return switch (x) { case 1 -> { yield 2; } default -> { yield 3; } }; } }";
    assert_eq!(switch_case_rhs_ints(src), vec![1]);
    assert_eq!(switch_expression_branch_ints(src), vec![2, 3]);
    let raw = raw_names(src);
    assert!(
        !raw.iter().any(|name| name == "yield_statement"),
        "switch expression yield blocks should lower without Raw yield_statement: {raw:?}"
    );
}

#[test]
fn postfix_increment_with_nested_decrement_in_operand() {
    // `a[i--]++` desugars with the OUTER op being increment (`+ 1`); a substring
    // `--` check misread the nested `i--` and flipped it to decrement.
    let ops = binops("class C { void f(){ int[] a = new int[10]; int i = 0; a[i--]++; } }");
    assert!(
        ops.contains(&Op::Add),
        "outer `++` must lower to Op::Add despite the nested `i--`, got {ops:?}"
    );
}
