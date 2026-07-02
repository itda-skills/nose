use super::*;
use crate::test_helpers::raw_names;
use nose_il::SourceProtocolKind;

fn il_with_interner(src: &str) -> (Il, Interner) {
    let interner = Interner::default();
    let il = lower(FileId(0), "t.cs", src.as_bytes(), &interner).expect("lower csharp");
    (il, interner)
}

fn expect_csharp_protocol_boundary(src: &str, tag: &str, protocol: SourceProtocolKind) {
    let (il, interner) = il_with_interner(src);
    crate::test_helpers::expect_raw_protocol_boundary(&il, &interner, tag, protocol);
}

fn expect_no_csharp_boundary(src: &str, tag: &str, surface: &str) {
    let (il, interner) = il_with_interner(src);
    let raw = raw_names(&il, &interner);
    assert!(
        !raw.iter().any(|name| name == tag),
        "{surface} must not carry the {tag} boundary: {raw:?}"
    );
}

#[test]
fn await_preserves_source_backed_scheduling_boundary() {
    expect_csharp_protocol_boundary(
        "class C {\n  async Task M() { var x = await F(); }\n}\n",
        "await",
        SourceProtocolKind::Await,
    );
}

#[test]
fn yield_return_preserves_source_backed_generator_boundary() {
    expect_csharp_protocol_boundary(
        "class C {\n  IEnumerable<int> M() { yield return 1; }\n}\n",
        "yield",
        SourceProtocolKind::Yield,
    );
}

#[test]
fn async_method_preserves_async_function_boundary() {
    expect_csharp_protocol_boundary(
        "class C {\n  async Task M() { Work(); }\n}\n",
        "async_function",
        SourceProtocolKind::AsyncFunction,
    );
}

#[test]
fn sync_method_forms_no_async_function_boundary() {
    expect_no_csharp_boundary(
        "class C {\n  Task M() { Work(); return Task.CompletedTask; }\n}\n",
        "async_function",
        "a synchronous method",
    );
}

#[test]
fn async_lambda_preserves_async_function_boundary() {
    expect_csharp_protocol_boundary(
        "class C {\n  void M() { Run(async () => await F()); }\n}\n",
        "async_function",
        SourceProtocolKind::AsyncFunction,
    );
}

#[test]
fn await_foreach_preserves_async_iteration_boundary() {
    expect_csharp_protocol_boundary(
        "class C {\n  async Task M(IAsyncEnumerable<int> xs) {\n    await foreach (var x in xs) { Use(x); }\n  }\n}\n",
        "async_for",
        SourceProtocolKind::AsyncIteration,
    );
}

#[test]
fn sync_foreach_forms_no_async_iteration_boundary() {
    expect_no_csharp_boundary(
        "class C {\n  void M(IEnumerable<int> xs) {\n    foreach (var x in xs) { Use(x); }\n  }\n}\n",
        "async_for",
        "a synchronous foreach",
    );
}

#[test]
fn await_using_preserves_async_context_boundary() {
    expect_csharp_protocol_boundary(
        "class C {\n  async Task M() {\n    await using (var r = Open()) { Use(r); }\n  }\n}\n",
        "async_with",
        SourceProtocolKind::AsyncContext,
    );
}

#[test]
fn sync_using_forms_no_async_context_boundary() {
    expect_no_csharp_boundary(
        "class C {\n  void M() {\n    using (var r = Open()) { Use(r); }\n  }\n}\n",
        "async_with",
        "a synchronous using",
    );
}
