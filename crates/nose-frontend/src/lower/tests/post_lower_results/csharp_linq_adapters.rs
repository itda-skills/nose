use super::*;

#[test]
fn csharp_linq_select_where_emit_hof_contracts_with_result_domains() {
    let interner = Interner::new();
    let il = lower_fixture(
        "linq_chain.cs",
        b"class C { static object F(int[] xs) { return xs.Where(a => a > 3).Select(b => b * 2); } }",
        Lang::CSharp,
        &interner,
    );
    let where_contract = nose_semantics::library_method_call_contract(Lang::CSharp, "Where", 1)
        .expect("C# Where HOF contract");
    assert_eq!(
        contract_api_count(&il.evidence, where_contract.id, where_contract.callee),
        1,
        "`xs.Where(p)` over an array-typed parameter should carry the Filter contract"
    );
    let select_contract = nose_semantics::library_method_call_contract(Lang::CSharp, "Select", 1)
        .expect("C# Select HOF contract");
    assert_eq!(
        contract_api_count(&il.evidence, select_contract.id, select_contract.callee),
        1,
        "the chained `.Select(f)` should consume the Where result-domain receiver proof"
    );
}
