use super::*;

#[test]
fn python_lowering_emits_library_api_for_aliased_imported_collection_factory() {
    let interner = Interner::new();
    let il = crate::lower_source(
        FileId(0),
        "alias.py",
        b"from collections import deque as Values\n\n\
def f(value, other):\n    return Values([\"red\", \"blue\"]).__contains__(value)\n",
        Lang::Python,
        &interner,
    )
    .expect("python lowering should succeed");
    let contract = nose_semantics::library_imported_collection_factory_contract(
        Lang::Python,
        "collections",
        "deque",
    )
    .expect("deque contract");

    assert_eq!(
        library_api_evidence_count_in_records(
            &il.evidence,
            library_api_contract_id_hash(contract.id),
            library_api_callee_contract_hash(contract.callee),
        ),
        1
    );
}
