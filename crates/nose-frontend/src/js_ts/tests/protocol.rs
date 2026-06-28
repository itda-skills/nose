use super::super::lower;
use nose_il::{FileId, Interner, Lang, SourceProtocolKind};

#[test]
fn await_expression_preserves_source_backed_async_boundary() {
    let interner = Interner::new();
    let il = lower(
        FileId(0),
        "t.js",
        b"async function f(x) { return await x + 1; }",
        Lang::JavaScript,
        &interner,
    )
    .expect("lower js");

    crate::test_helpers::expect_raw_protocol_boundary(
        &il,
        &interner,
        "await",
        SourceProtocolKind::Await,
    );
}

#[test]
fn async_function_preserves_source_backed_async_boundary() {
    let interner = Interner::new();
    let il = lower(
        FileId(0),
        "t.js",
        b"async function f(x) { return x + 1; }",
        Lang::JavaScript,
        &interner,
    )
    .expect("lower js");

    crate::test_helpers::expect_raw_protocol_boundary(
        &il,
        &interner,
        "async_function",
        SourceProtocolKind::AsyncFunction,
    );
}

#[test]
fn yield_expression_preserves_source_backed_protocol_boundary() {
    let interner = Interner::new();
    let il = lower(
        FileId(0),
        "t.js",
        b"function* f(x) { yield x + 1; }",
        Lang::JavaScript,
        &interner,
    )
    .expect("lower js");

    crate::test_helpers::expect_raw_protocol_boundary(
        &il,
        &interner,
        "yield",
        SourceProtocolKind::Yield,
    );
}
