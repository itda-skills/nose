use super::*;

fn ops(src: &str) -> Vec<Op> {
    let interner = Interner::new();
    lower(FileId(0), "t.go", src.as_bytes(), &interner)
        .expect("lower")
        .nodes
        .iter()
        .filter(|n| matches!(n.kind, NodeKind::BinOp | NodeKind::UnOp))
        .filter_map(|n| match n.payload {
            Payload::Op(op) => Some(op),
            _ => None,
        })
        .collect()
}

fn switch_case_rhs_ints(src: &str) -> Vec<i64> {
    let interner = Interner::new();
    let il = lower(FileId(0), "t.go", src.as_bytes(), &interner).expect("lower");
    il.nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| node.kind == NodeKind::BinOp && node.payload == Payload::Op(Op::Eq))
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

#[test]
fn switch_cases_compare_scrutinee_to_all_case_labels() {
    let src =
        "package main\nfunc f(x int) int { switch x { case 1, 2: return 3; default: return 4 } }\n";
    assert_eq!(switch_case_rhs_ints(src), vec![1, 2]);
}

#[test]
fn block_switch_select_bodies_survive_statement_list_wrapper() {
    // go 0.25 wraps block / switch-case / select-case statements in a single
    // `statement_list` node (go 0.23 listed them directly). `stmt_children` must
    // flatten that wrapper, else every nested statement is orphaned to Raw — the
    // go-0.25 Raw-ratio blowup (0.7% → 29%) this migration fixes. Each op below
    // lives inside one of the three wrapper sites.
    let block = ops("package main\nfunc f(a int, b int) int { c := a + b; return c }\n");
    assert!(
        block.contains(&Op::Add),
        "block-body op orphaned, got {block:?}"
    );

    let sw = ops(
        "package main\nfunc f(a int, b int, x int) int { switch x { case 1: return a - b; default: return a * b } }\n",
    );
    assert!(
        sw.contains(&Op::Sub) && sw.contains(&Op::Mul),
        "switch-case-body ops orphaned, got {sw:?}"
    );

    let sel = ops(
        "package main\nfunc f(ch chan int, a int, b int) int { select { case <-ch: return a / b; default: return a + b } }\n",
    );
    assert!(
        sel.contains(&Op::Div) && sel.contains(&Op::Add),
        "select-case-body ops orphaned, got {sel:?}"
    );
}

#[test]
fn bit_clear_is_not_plain_bitand() {
    // Go's `a &^ b` is AND-NOT (`a & ^b`): it must desugar to a `BitAnd` over a
    // `BitNot` of the right operand, NOT collapse to a plain `a & b` (different bits,
    // and a false merge with real `&`).
    let clear = ops("package main\nfunc f(a int, b int) int { return a &^ b }\n");
    assert!(
        clear.contains(&Op::BitNot),
        "`a &^ b` must introduce BitNot, got {clear:?}"
    );
    assert!(
        clear.contains(&Op::BitAnd),
        "`a &^ b` must keep BitAnd, got {clear:?}"
    );

    // Plain `a & b` must NOT introduce a BitNot — the two operators stay distinct.
    let and = ops("package main\nfunc f(a int, b int) int { return a & b }\n");
    assert!(
        !and.contains(&Op::BitNot),
        "`a & b` must not introduce BitNot, got {and:?}"
    );
}

#[test]
fn go_defer_and_channel_operations_preserve_source_backed_protocol_boundaries() {
    let interner = Interner::new();
    let il = lower(
        FileId(0),
        "t.go",
        b"package main\nfunc f(ch chan int, x int) int { go record(x); defer record(x); ch <- x; return <-ch }\n",
        &interner,
    )
    .expect("lower");

    crate::test_helpers::expect_raw_protocol_boundary(
        &il,
        &interner,
        "go",
        SourceProtocolKind::GoRoutine,
    );
    crate::test_helpers::expect_raw_protocol_boundary(
        &il,
        &interner,
        "defer",
        SourceProtocolKind::Defer,
    );
    crate::test_helpers::expect_raw_protocol_boundary(
        &il,
        &interner,
        "channel_send",
        SourceProtocolKind::ChannelSend,
    );
    crate::test_helpers::expect_raw_protocol_boundary(
        &il,
        &interner,
        "channel_receive",
        SourceProtocolKind::ChannelReceive,
    );
}

#[test]
fn select_statement_preserves_source_backed_protocol_boundary() {
    let interner = Interner::new();
    let il = lower(
        FileId(0),
        "t.go",
        b"package main\nfunc f(ch chan int) { select { case v := <-ch: _ = v; default: return } }\n",
        &interner,
    )
    .expect("lower");

    crate::test_helpers::expect_raw_protocol_boundary(
        &il,
        &interner,
        "select",
        SourceProtocolKind::ChannelSelect,
    );
    crate::test_helpers::expect_raw_protocol_boundary(
        &il,
        &interner,
        "select_case",
        SourceProtocolKind::ChannelSelectCase,
    );
    crate::test_helpers::expect_raw_protocol_boundary(
        &il,
        &interner,
        "select_default",
        SourceProtocolKind::ChannelSelectDefault,
    );
    crate::test_helpers::expect_raw_protocol_boundary(
        &il,
        &interner,
        "channel_receive",
        SourceProtocolKind::ChannelReceive,
    );
}

#[test]
fn comma_ok_receive_preserves_value_and_status_protocol_boundaries() {
    let interner = Interner::new();
    let il = lower(
        FileId(0),
        "t.go",
        b"package main\nfunc f(ch chan int) bool { v, ok := <-ch; _ = v; return ok }\n",
        &interner,
    )
    .expect("lower");

    crate::test_helpers::expect_raw_protocol_boundary(
        &il,
        &interner,
        "channel_receive",
        SourceProtocolKind::ChannelReceive,
    );
    crate::test_helpers::expect_raw_protocol_boundary(
        &il,
        &interner,
        "channel_receive_status",
        SourceProtocolKind::ChannelReceive,
    );
}
