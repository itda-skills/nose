use super::*;

fn nodes(src: &str) -> Vec<nose_il::Node> {
    let interner = Interner::new();
    lower(FileId(0), "t.rb", src.as_bytes(), &interner)
        .expect("lower")
        .nodes
}

fn unit_names(src: &str) -> Vec<(UnitKind, String)> {
    let interner = Interner::new();
    let il = lower(FileId(0), "t.rb", src.as_bytes(), &interner).expect("lower");
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

fn unary_ops(src: &str) -> Vec<Op> {
    nodes(src)
        .iter()
        .filter(|n| n.kind == NodeKind::UnOp)
        .filter_map(|n| match n.payload {
            Payload::Op(op) => Some(op),
            _ => None,
        })
        .collect()
}

fn binary_ops(src: &str) -> Vec<Op> {
    nodes(src)
        .iter()
        .filter(|n| n.kind == NodeKind::BinOp)
        .filter_map(|n| match n.payload {
            Payload::Op(op) => Some(op),
            _ => None,
        })
        .collect()
}

fn expr_stmt_ints(src: &str) -> Vec<i64> {
    let interner = Interner::new();
    let il = lower(FileId(0), "t.rb", src.as_bytes(), &interner).expect("lower");
    il.nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| node.kind == NodeKind::ExprStmt)
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
fn unary_operators_lower_to_distinct_ops() {
    let ops = unary_ops("x = +5\ny = -5\nz = !a\nw = ~5\n");
    assert!(ops.contains(&Op::Pos), "unary + → Op::Pos, got {ops:?}");
    assert!(ops.contains(&Op::Neg), "unary - → Op::Neg, got {ops:?}");
    assert!(ops.contains(&Op::Not), "unary ! → Op::Not, got {ops:?}");
    assert!(
        ops.contains(&Op::BitNot),
        "unary ~ → Op::BitNot, got {ops:?}"
    );
}

#[test]
fn keyword_not_lowers_to_not() {
    assert_eq!(unary_ops("y = not a\n"), vec![Op::Not]);
}

#[test]
fn case_when_compares_scrutinee_against_pattern() {
    // `case x when 7 ...` must lower a comparison of the scrutinee against the
    // pattern `7`; previously the pattern was dropped (cond was `x == x`), so the
    // literal 7 never appeared in the IL.
    let has_seven = nodes("case x\nwhen 7\n  y\nend\n")
        .iter()
        .any(|n| matches!(n.payload, Payload::LitInt(7)));
    assert!(
        has_seven,
        "the `when 7` pattern literal must appear in the lowered IL"
    );
}

#[test]
fn scrutinee_less_case_uses_when_condition_directly() {
    let ops = binary_ops("case\nwhen x > 0\n  y\nelse\n  z\nend\n");
    assert!(
        ops.contains(&Op::Gt),
        "scrutinee-less case should keep the when predicate, got {ops:?}"
    );
    assert!(
        !ops.contains(&Op::Eq),
        "scrutinee-less case should not compare an empty scrutinee, got {ops:?}"
    );
}

#[test]
fn case_else_body_is_preserved() {
    let mut ints = expr_stmt_ints("case\nwhen x > 0\n  1\nelse\n  2\nend\n");
    ints.sort_unstable();
    assert_eq!(ints, vec![1, 2]);
}

#[test]
fn test_dsl_block_calls_are_units() {
    let units = unit_names(
        r#"
test 'renders table' do
  assert_equal 1, result
end

RSpec.describe 'Widget' do
  it 'renders value' do
expect(result).to eq(1)
  end
end

items.each do |item|
  puts item
end
"#,
    );
    assert!(
        units.contains(&(UnitKind::Block, "test:renders table".to_string())),
        "Minitest-style test blocks should be block units: {units:?}"
    );
    assert!(
        units.contains(&(UnitKind::Block, "describe:Widget".to_string())),
        "RSpec describe blocks should be block units: {units:?}"
    );
    assert!(
        units.contains(&(UnitKind::Block, "it:renders value".to_string())),
        "RSpec it blocks should be block units: {units:?}"
    );
    assert!(
        !units
            .iter()
            .any(|(kind, name)| *kind == UnitKind::Block && name.starts_with("each:")),
        "generic Ruby block calls must not become DSL units: {units:?}"
    );
}
