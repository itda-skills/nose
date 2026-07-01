use super::*;

#[test]
fn async_functions_and_blocks_preserve_source_backed_protocol_boundaries() {
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
        "async_function",
        SourceProtocolKind::AsyncFunction,
    );
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
fn async_closure_preserves_source_backed_protocol_boundary() {
    let interner = Interner::new();
    let il = lower(
        FileId(0),
        "t.rs",
        b"fn f() { let cb = async move |x: i32| x + 1; }\n",
        &interner,
    )
    .expect("lower");

    crate::test_helpers::expect_raw_protocol_boundary(
        &il,
        &interner,
        "async_function",
        SourceProtocolKind::AsyncFunction,
    );

    let ops: Vec<_> = il
        .nodes
        .iter()
        .filter_map(|node| match node.payload {
            Payload::Op(op) => Some(op),
            _ => None,
        })
        .collect();
    assert!(
        ops.contains(&Op::Add),
        "async closure body was lost: {ops:?}"
    );
}

#[test]
fn sync_closure_does_not_create_async_protocol_boundary() {
    let (interner, il) = lower_rust("fn f() { let cb = |x: i32| x + 1; }\n");

    let raw = raw_names(&il, &interner);
    assert!(
        !raw.iter().any(|name| name == "async_function"),
        "sync closure must not create an async_function protocol boundary: {raw:?}"
    );
}
