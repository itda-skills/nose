use super::*;

#[test]
fn post_lowering_emits_free_name_and_require_library_api_occurrences() {
    let interner = Interner::new();
    assert_python_free_name_occurrences(&interner);
    assert_go_and_rust_free_name_occurrences(&interner);
    assert_ruby_require_occurrences(&interner);
}

fn assert_python_free_name_occurrences(interner: &Interner) {
    for factory in ["list", "set", "frozenset", "tuple"] {
        let src = format!("def f(values):\n    return {factory}(values)\n");
        let py = lower_fixture("builtin.py", src.as_bytes(), Lang::Python, interner);
        let py_contract =
            library_free_name_collection_factory_contract(Lang::Python, factory).unwrap();
        assert_eq!(
            contract_api_count(&py.evidence, py_contract.id, py_contract.callee),
            1
        );
        let py_api_records = contract_api_records(&py.evidence, py_contract.id, py_contract.callee);
        assert_eq!(
            py_api_records[0].provenance.pack_hash,
            Some(stable_symbol_hash(
                nose_semantics::PYTHON_BUILTIN_COLLECTION_FACTORY_PACK_ID
            ))
        );
        assert_eq!(
            py_api_records[0].provenance.rule_hash,
            Some(stable_symbol_hash(
                PYTHON_BUILTIN_COLLECTION_FACTORY_PRODUCER_ID
            ))
        );
    }

    let py_contract = library_free_name_collection_factory_contract(Lang::Python, "list").unwrap();

    let shadowed_py = lower_fixture(
        "shadowed.py",
        b"def f(list, values):\n    return list(values)\n",
        Lang::Python,
        interner,
    );
    assert_eq!(
        contract_api_count(&shadowed_py.evidence, py_contract.id, py_contract.callee),
        0
    );

    let wildcard_py = lower_fixture(
        "wildcard.py",
        b"from custom import *\n\ndef f(values):\n    return list(values)\n",
        Lang::Python,
        interner,
    );
    assert!(wildcard_py.evidence.iter().any(|record| matches!(
        record.kind,
        EvidenceKind::Import(ImportEvidenceKind::Wildcard { module_hash })
            if module_hash == stable_symbol_hash("custom")
    )));
    assert_eq!(
        contract_api_count(&wildcard_py.evidence, py_contract.id, py_contract.callee),
        0
    );

    let py_len = lower_fixture(
        "len.py",
        b"def f(values):\n    return len(values)\n",
        Lang::Python,
        interner,
    );
    let py_len_contract = library_free_function_builtin_contract(Lang::Python, "len", 1).unwrap();
    assert_eq!(
        contract_api_count(&py_len.evidence, py_len_contract.id, py_len_contract.callee),
        1
    );

    let shadowed_py_len = lower_fixture(
        "shadowed_len.py",
        b"def f(len, values):\n    return len(values)\n",
        Lang::Python,
        interner,
    );
    assert_eq!(
        contract_api_count(
            &shadowed_py_len.evidence,
            py_len_contract.id,
            py_len_contract.callee
        ),
        0
    );

    let wildcard_py_len = lower_fixture(
        "wildcard_len.py",
        b"from custom import *\n\ndef f(values):\n    return len(values)\n",
        Lang::Python,
        interner,
    );
    assert_eq!(
        contract_api_count(
            &wildcard_py_len.evidence,
            py_len_contract.id,
            py_len_contract.callee
        ),
        0
    );
}

fn assert_go_and_rust_free_name_occurrences(interner: &Interner) {
    let go = lower_fixture(
            "builtin.go",
            b"package p\nfunc f(xs []int, x int) int { return len(xs) }\nfunc g(xs []int, x int) []int { return append(xs, x) }\n",
            Lang::Go,
            interner,
        );
    let go_len_contract = library_free_function_builtin_contract(Lang::Go, "len", 1).unwrap();
    assert_eq!(
        contract_api_count(&go.evidence, go_len_contract.id, go_len_contract.callee),
        1
    );
    let go_append_contract = library_free_function_builtin_contract(Lang::Go, "append", 2).unwrap();
    assert_eq!(
        contract_api_count(
            &go.evidence,
            go_append_contract.id,
            go_append_contract.callee
        ),
        1
    );

    let rust = lower_fixture(
        "vec.rs",
        b"fn f() { let xs = Vec::new(); }",
        Lang::Rust,
        interner,
    );
    let rust_contract = library_rust_vec_new_factory_contract(Lang::Rust, "Vec::new").unwrap();
    assert_eq!(
        contract_api_count(&rust.evidence, rust_contract.id, rust_contract.callee),
        1
    );

    let rust_macro = lower_fixture(
        "vec_macro.rs",
        b"fn f() { let xs = vec![1, 2]; }",
        Lang::Rust,
        interner,
    );
    let rust_macro_contract = library_rust_vec_macro_factory_contract(Lang::Rust, "vec").unwrap();
    assert_eq!(
        contract_api_count(
            &rust_macro.evidence,
            rust_macro_contract.id,
            rust_macro_contract.callee
        ),
        1
    );

    let rust_function_call = lower_fixture(
        "vec_function.rs",
        b"fn f(vec: fn(i32) -> Vec<i32>) { let xs = vec(1); }",
        Lang::Rust,
        interner,
    );
    assert_eq!(
        contract_api_count(
            &rust_function_call.evidence,
            rust_macro_contract.id,
            rust_macro_contract.callee
        ),
        0
    );

    let rust_shadowed_macro = lower_fixture(
            "vec_shadowed_macro.rs",
            b"macro_rules! vec { ($($x:expr),*) => { custom_vec![$($x),*] }; }\nfn f() { let xs = vec![1, 2]; }",
            Lang::Rust,
            interner,
        );
    assert_eq!(
        contract_api_count(
            &rust_shadowed_macro.evidence,
            rust_macro_contract.id,
            rust_macro_contract.callee
        ),
        0
    );
}

fn assert_ruby_require_occurrences(interner: &Interner) {
    let ruby = lower_fixture(
        "set.rb",
        b"require \"set\"\n\ndef f(values)\n  Set.new(values)\nend\n",
        Lang::Ruby,
        interner,
    );
    let ruby_contract = library_ruby_set_factory_contract(Lang::Ruby, "Set", "new", 1).unwrap();
    assert_eq!(
        contract_api_count(&ruby.evidence, ruby_contract.id, ruby_contract.callee),
        1
    );

    let missing_require = lower_fixture(
        "set_missing_require.rb",
        b"def f(values)\n  Set.new(values)\nend\n",
        Lang::Ruby,
        interner,
    );
    assert_eq!(
        contract_api_count(
            &missing_require.evidence,
            ruby_contract.id,
            ruby_contract.callee
        ),
        0
    );

    let late_require = lower_fixture(
        "set_late_require.rb",
        b"def f(values)\n  Set.new(values)\nend\n\nrequire \"set\"\n",
        Lang::Ruby,
        interner,
    );
    assert_eq!(
        contract_api_count(
            &late_require.evidence,
            ruby_contract.id,
            ruby_contract.callee
        ),
        0
    );

    let shadowed_require = lower_fixture(
            "set_shadowed_require.rb",
            b"def require(name)\n  name\nend\n\nrequire \"set\"\n\ndef f(values)\n  Set.new(values)\nend\n",
            Lang::Ruby,
            interner,
        );
    assert_eq!(
        contract_api_count(
            &shadowed_require.evidence,
            ruby_contract.id,
            ruby_contract.callee
        ),
        0
    );
}
