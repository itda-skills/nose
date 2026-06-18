use super::*;

fn match_case_rhs_ints(src: &str) -> Vec<i64> {
    let interner = Interner::new();
    let il = lower(FileId(0), "t.rs", src.as_bytes(), &interner).expect("lower");
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

fn lower_rust(src: &str) -> (Interner, Il) {
    let interner = Interner::new();
    let il = lower(FileId(0), "t.rs", src.as_bytes(), &interner).expect("lower");
    (interner, il)
}

fn raw_names(il: &Il, interner: &Interner) -> Vec<String> {
    il.nodes
        .iter()
        .filter(|node| node.kind == NodeKind::Raw)
        .filter_map(|node| match node.payload {
            Payload::Name(sym) => Some(interner.resolve(sym).to_string()),
            _ => None,
        })
        .collect()
}

fn unit_names(src: &str) -> Vec<(UnitKind, String)> {
    let (interner, il) = lower_rust(src);
    il.units
        .iter()
        .map(|unit| {
            (
                unit.kind,
                unit.name
                    .map(|name| interner.resolve(name).to_string())
                    .unwrap_or_else(|| "-".to_string()),
            )
        })
        .collect()
}

fn binop_ops(il: &Il) -> Vec<Op> {
    il.nodes
        .iter()
        .filter(|node| node.kind == NodeKind::BinOp)
        .filter_map(|node| match node.payload {
            Payload::Op(op) => Some(op),
            _ => None,
        })
        .collect()
}

fn source_range_count(il: &Il, kind: SourceRangeKind) -> usize {
    il.evidence
        .iter()
        .filter(|record| {
            matches!(
                record.kind,
                nose_il::EvidenceKind::Source(SourceFactKind::Range(actual))
                    if actual == kind
            )
        })
        .count()
}

fn source_pattern_count(il: &Il, kind: SourcePatternKind) -> usize {
    il.evidence
        .iter()
        .filter(|record| {
            matches!(
                record.kind,
                nose_il::EvidenceKind::Source(SourceFactKind::Pattern(actual))
                    if actual == kind
            )
        })
        .count()
}

#[test]
fn match_cases_compare_scrutinee_to_literal_patterns() {
    let src = "fn f(x: i32) -> i32 { match x { 7 => 1, 8 => 2, _ => 3 } }";
    assert_eq!(match_case_rhs_ints(src), vec![7, 8]);
}

#[test]
fn guarded_match_combines_pattern_and_guard() {
    let src = "fn f(x: i32, ok: bool) -> i32 { match x { 7 | 8 if ok => 1, _ => 2 } }";
    let (interner, il) = lower_rust(src);

    assert_eq!(match_case_rhs_ints(src), vec![7, 8]);
    assert!(il
        .nodes
        .iter()
        .any(|node| node.kind == NodeKind::BinOp && node.payload == Payload::Op(Op::And)));
    assert!(il.nodes.iter().any(|node| match node.payload {
        Payload::Name(sym) => interner.resolve(sym) == "ok",
        _ => false,
    }));
}

#[test]
fn panic_macro_lowers_to_throw() {
    let src = "fn f(x: i32) -> i32 { if x < 0 { panic!(); } x }";
    let (_, il) = lower_rust(src);

    assert!(
        il.nodes.iter().any(|node| node.kind == NodeKind::Throw),
        "panic! should lower to a Throw node so guard clauses are path-narrowing exits"
    );
}

#[test]
fn rust_item_shadow_scan_handles_visibility_and_qualifiers() {
    assert!(rust_item_declares_name(
        "pub(crate) struct Some<T>(T);",
        "Some"
    ));
    assert!(rust_item_declares_name(
        "pub const None: Option<i32> = Some(0);",
        "None"
    ));
    assert!(rust_item_declares_name(
        "pub const fn Some(value: i32) -> Option<i32> { None }",
        "Some"
    ));
    assert!(!rust_item_declares_name(
        "if let Some(_) = value { true } else { false }",
        "Some"
    ));
}

#[test]
fn macro_rules_arm_bodies_become_block_units_without_raw_token_trees() {
    let src = r#"
macro_rules! sample {
($arg:expr) => {{
    let value = $arg;
    if value > 0 {
        panic!("bad");
    }
    value
}};
($other:expr) => {{
    let value = $other;
    if value > 1 {
        panic!("worse");
    }
    value
}};
}
"#;
    let units = unit_names(src);
    assert!(
        units.contains(&(UnitKind::Block, "sample:arm0".to_string())),
        "first macro_rules! arm should be a block unit: {units:?}"
    );
    assert!(
        units.contains(&(UnitKind::Block, "sample:arm1".to_string())),
        "second macro_rules! arm should be a block unit: {units:?}"
    );

    let (interner, il) = lower_rust(src);
    let raw = raw_names(&il, &interner);
    assert!(
        !raw.iter().any(|name| name == "token_tree"),
        "macro_rules! arm extraction should not emit Raw token_tree nodes: {raw:?}"
    );
}

#[test]
fn if_let_option_patterns_preserve_pattern_surface() {
    let (interner, il) = lower_rust(
        "pub fn f(value: Option<i32>) -> bool { if let None = value { true } else { false } }",
    );

    assert!(il.nodes.iter().any(|node| {
        node.kind == NodeKind::Var
            && matches!(node.payload, Payload::Name(sym) if interner.resolve(sym) == "None")
    }));
    assert!(il
        .nodes
        .iter()
        .any(|node| node.kind == NodeKind::BinOp && node.payload == Payload::Op(Op::Eq)));

    let (shadowed_interner, shadowed) = lower_rust(
        "pub const None: Option<i32> = Some(0);\npub fn f(value: Option<i32>) -> bool { if let None = value { true } else { false } }",
    );
    assert!(shadowed.nodes.iter().any(|node| {
        node.kind == NodeKind::Var
            && matches!(node.payload, Payload::Name(sym) if shadowed_interner.resolve(sym) == "None")
    }));
}

#[test]
fn range_expressions_emit_distinct_source_range_evidence() {
    let (_, half_open) = lower_rust("fn f(n: usize) { for i in 0..n { let _ = i; } }");
    assert_eq!(
        source_range_count(&half_open, SourceRangeKind::RustHalfOpenRangeExpression),
        1
    );
    assert_eq!(
        source_range_count(&half_open, SourceRangeKind::RustInclusiveRangeExpression),
        0
    );

    let (_, inclusive) = lower_rust("fn f(n: usize) { for i in 0..=n { let _ = i; } }");
    assert_eq!(
        source_range_count(&inclusive, SourceRangeKind::RustInclusiveRangeExpression),
        1
    );
    assert_eq!(
        source_range_count(&inclusive, SourceRangeKind::RustHalfOpenRangeExpression),
        0
    );
}

#[test]
fn tuple_struct_single_wildcard_pattern_emits_source_pattern_evidence() {
    let (_, wildcard) = lower_rust(
        "pub fn f(value: Option<i32>) -> bool { if let Some(_) = value { true } else { false } }",
    );
    assert_eq!(
        source_pattern_count(
            &wildcard,
            SourcePatternKind::RustTupleStructSingleWildcardPattern
        ),
        1
    );

    let (_, binding) = lower_rust(
        "pub fn f(value: Option<i32>) -> bool { if let Some(x) = value { x > 0 } else { false } }",
    );
    assert_eq!(
        source_pattern_count(
            &binding,
            SourcePatternKind::RustTupleStructSingleWildcardPattern
        ),
        0
    );
}

#[test]
fn match_range_pattern_lowers_to_bounds() {
    let src = "fn f(x: i32) -> i32 { match x { 1..=3 => 7, _ => 0 } }";
    let (interner, il) = lower_rust(src);

    let raw = raw_names(&il, &interner);
    assert!(
        !raw.iter().any(|name| name == "range_pattern"),
        "range match pattern should lower without Raw range_pattern: {raw:?}"
    );
    let ops = binop_ops(&il);
    assert!(
        ops.contains(&Op::Ge) && ops.contains(&Op::Le) && ops.contains(&Op::And),
        "inclusive range pattern should lower to lower/upper bound checks, got {ops:?}"
    );
}

#[test]
fn match_tuple_pattern_lowers_without_raw() {
    let src = "fn f(x: (i32, i32)) -> i32 { match x { (1, 2) => 7, _ => 0 } }";
    let (interner, il) = lower_rust(src);

    let raw = raw_names(&il, &interner);
    assert!(
        !raw.iter().any(|name| name == "tuple_pattern"),
        "tuple match pattern should lower without Raw tuple_pattern: {raw:?}"
    );
}

#[test]
fn match_slice_pattern_lowers_without_raw() {
    let src = "fn f(x: [i32; 2]) -> i32 { match x { [1, 2] => 7, _ => 0 } }";
    let (interner, il) = lower_rust(src);

    let raw = raw_names(&il, &interner);
    assert!(
        !raw.iter().any(|name| name == "slice_pattern"),
        "slice match pattern should lower without Raw slice_pattern: {raw:?}"
    );
}

#[test]
fn match_reference_pattern_lowers_without_raw() {
    let src = "fn f(x: &i32) -> i32 { match x { &1 => 7, _ => 0 } }";
    let (interner, il) = lower_rust(src);

    let raw = raw_names(&il, &interner);
    assert!(
        !raw.iter().any(|name| name == "reference_pattern"),
        "reference match pattern should lower without Raw reference_pattern: {raw:?}"
    );
}

#[test]
fn match_negative_literal_pattern_lowers_without_raw() {
    let src = "fn f(x: i32) -> i32 { match x { -1 => 7, _ => 0 } }";
    let (interner, il) = lower_rust(src);

    let raw = raw_names(&il, &interner);
    assert!(
        !raw.iter().any(|name| name == "negative_literal"),
        "negative literal match pattern should lower without Raw negative_literal: {raw:?}"
    );
    assert!(match_case_rhs_ints(src).contains(&-1));
}

#[test]
fn match_typed_integer_literal_pattern_retains_value() {
    let src = "fn f(x: i32) -> i32 { match x { 1i32 => 7, _ => 0 } }";
    assert!(
        match_case_rhs_ints(src).contains(&1),
        "typed integer match patterns should retain their numeric value"
    );
}

#[test]
fn async_blocks_preserve_source_backed_protocol_boundaries() {
    let interner = Interner::new();
    let il = lower(
        FileId(0),
        "t.rs",
        b"pub async fn f(x: i32) -> i32 { async move { x + 1 }.await }\n",
        &interner,
    )
    .expect("lower");

    crate::test_helpers::expect_raw_protocol_boundary(
        &il,
        &interner,
        "async_block",
        SourceProtocolKind::AsyncBlock,
    );
    crate::test_helpers::expect_raw_protocol_boundary(
        &il,
        &interner,
        "await",
        SourceProtocolKind::Await,
    );

    let ops: Vec<_> = il
        .nodes
        .iter()
        .filter_map(|node| match node.payload {
            Payload::Op(op) => Some(op),
            _ => None,
        })
        .collect();
    assert!(ops.contains(&Op::Add), "async block body was lost: {ops:?}");
}

#[test]
fn try_expression_preserves_source_backed_protocol_boundary() {
    let interner = Interner::new();
    let il = lower(
        FileId(0),
        "t.rs",
        b"fn f(x: Result<i32, E>) -> Result<i32, E> { Ok(x? + 1) }\n",
        &interner,
    )
    .expect("lower");

    crate::test_helpers::expect_raw_protocol_boundary(
        &il,
        &interner,
        "try",
        SourceProtocolKind::TryPropagation,
    );
}
