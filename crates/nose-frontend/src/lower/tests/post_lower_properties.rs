use super::*;

#[test]
fn post_lowering_emits_property_and_rust_option_occurrences() {
    let interner = Interner::new();
    assert_ts_length_property_occurrences(&interner);
    assert_rust_option_occurrences(&interner);
}

fn assert_ts_length_property_occurrences(interner: &Interner) {
    let ts = lower_fixture(
        "t.ts",
        b"function f(xs: number[]) { return xs.length; }\n",
        Lang::TypeScript,
        interner,
    );
    let property_contract = library_property_builtin_contract(Lang::TypeScript, "length").unwrap();
    let length_field = named_node_span(&ts, interner, NodeKind::Field, "length")
        .expect("length field should be lowered");
    let property_api = library_api_evidence_ids_at_node(
        &ts.evidence,
        length_field,
        NodeKind::Field,
        library_api_contract_id_hash(property_contract.id),
        library_api_callee_contract_hash(property_contract.callee),
        0,
    );
    assert_eq!(
        property_api.len(),
        1,
        "typed exact-collection property access should carry LibraryApi occurrence evidence"
    );
    let ts_filter_length = lower_fixture(
            "t.ts",
            b"function f(value: string) { return [\"red\", \"blue\"].filter((item: string) => item === value).length >= 1; }\n",
            Lang::TypeScript,
            interner,
        );
    let filter_length_field =
        named_node_span(&ts_filter_length, interner, NodeKind::Field, "length")
            .expect("filter length field should be lowered");
    let filter_length_api = library_api_evidence_ids_at_node(
        &ts_filter_length.evidence,
        filter_length_field,
        NodeKind::Field,
        library_api_contract_id_hash(property_contract.id),
        library_api_callee_contract_hash(property_contract.callee),
        0,
    );
    assert_eq!(
        filter_length_api.len(),
        1,
        "HOF result property access should carry LibraryApi occurrence evidence"
    );
}

fn assert_rust_option_occurrences(interner: &Interner) {
    let rust_some = lower_fixture(
        "t.rs",
        b"fn f(x: i32) -> Option<i32> { Some(x) }\n",
        Lang::Rust,
        interner,
    );
    let some_contract =
        library_rust_option_some_constructor_contract(Lang::Rust, "Some", 1).unwrap();
    let some_call = call_span_with_callee_named(&rust_some, interner, "Some")
        .expect("Some call should be lowered");
    let some_api = library_api_evidence_ids_at(
        &rust_some.evidence,
        some_call,
        library_api_contract_id_hash(some_contract.id),
        library_api_callee_contract_hash(some_contract.callee),
        1,
    );
    assert_eq!(some_api.len(), 1);
    let some_records =
        contract_api_records(&rust_some.evidence, some_contract.id, some_contract.callee);
    assert_rust_option_record_provenance(some_records[0]);
    assert!(result_domain_depends_on_api(
        &rust_some.evidence,
        some_call,
        DomainEvidence::Option,
        &some_api,
    ));

    let rust_some_pattern = lower_fixture(
            "t.rs",
            b"pub fn f(value: Option<i32>) -> bool { if let Some(_) = value { true } else { false } }\n",
            Lang::Rust,
            interner,
        );
    let some_pattern_var = named_node_span(&rust_some_pattern, interner, NodeKind::Var, "Some")
        .expect("Some pattern var should be preserved");
    let some_pattern_api = library_api_evidence_ids_at_node(
        &rust_some_pattern.evidence,
        some_pattern_var,
        NodeKind::Var,
        library_api_contract_id_hash(some_contract.id),
        library_api_callee_contract_hash(some_contract.callee),
        1,
    );
    assert_eq!(some_pattern_api.len(), 1);
    let some_pattern_records = contract_api_records(
        &rust_some_pattern.evidence,
        some_contract.id,
        some_contract.callee,
    );
    assert_rust_option_record_provenance(some_pattern_records[0]);
    assert!(
        !result_domain_depends_on_api_at_node(
            &rust_some_pattern.evidence,
            some_pattern_var,
            NodeKind::Var,
            DomainEvidence::Option,
            &some_pattern_api,
        ),
        "pattern occurrence identity must not become a constructor result domain"
    );

    let rust_none = lower_fixture(
        "t.rs",
        b"fn f() -> Option<i32> { None }\n",
        Lang::Rust,
        interner,
    );
    let none_contract = library_rust_option_none_sentinel_contract(Lang::Rust, "None").unwrap();
    let none_var = named_node_span(&rust_none, interner, NodeKind::Var, "None")
        .expect("None var should be lowered");
    let none_api = library_api_evidence_ids_at_node(
        &rust_none.evidence,
        none_var,
        NodeKind::Var,
        library_api_contract_id_hash(none_contract.id),
        library_api_callee_contract_hash(none_contract.callee),
        0,
    );
    assert_eq!(none_api.len(), 1);
    let none_records =
        contract_api_records(&rust_none.evidence, none_contract.id, none_contract.callee);
    assert_rust_option_record_provenance(none_records[0]);
    assert!(result_domain_depends_on_api_at_node(
        &rust_none.evidence,
        none_var,
        NodeKind::Var,
        DomainEvidence::Option,
        &none_api,
    ));

    let shadowed_some = lower_fixture(
        "t.rs",
        b"fn Some(x: i32) -> Option<i32> { None }\nfn f(x: i32) -> Option<i32> { Some(x) }\n",
        Lang::Rust,
        interner,
    );
    assert_eq!(
        contract_api_count(
            &shadowed_some.evidence,
            some_contract.id,
            some_contract.callee
        ),
        0,
        "local Rust Some item must close the std Option constructor occurrence"
    );
}

fn assert_rust_option_record_provenance(record: &EvidenceRecord) {
    assert_eq!(
        record.provenance.pack_hash,
        Some(stable_symbol_hash(
            nose_semantics::RUST_STDLIB_OPTION_PACK_ID
        ))
    );
    assert_eq!(
        record.provenance.rule_hash,
        Some(stable_symbol_hash(RUST_STDLIB_OPTION_PRODUCER_ID))
    );
}
