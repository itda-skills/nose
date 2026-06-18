use super::call_fixtures::*;
use super::support::*;

#[test]
fn free_name_builtin_requires_language_contract() {
    let (il, interner, call) = free_call_il(Lang::JavaScript, "len", false);
    assert!(matches!(canon_call(&il, &interner, call), CallCanon::None));
}

#[test]
fn free_name_builtin_requires_no_shadowing() {
    let (mut il, interner, call) = free_call_il(Lang::Python, "len", true);
    let _ = push_free_function_builtin_library_api_evidence(&mut il, &interner, call);
    assert!(matches!(canon_call(&il, &interner, call), CallCanon::None));
}

#[test]
fn python_unshadowed_builtin_requires_library_api_evidence() {
    let (mut il, interner, call) = free_call_il(Lang::Python, "len", false);
    assert!(matches!(canon_call(&il, &interner, call), CallCanon::None));

    let _ = push_free_function_builtin_library_api_evidence(&mut il, &interner, call);
    assert!(matches!(
        canon_call(&il, &interner, call),
        CallCanon::Builtin {
            op: Builtin::Len,
            arg_olds
        } if arg_olds.len() == 1
    ));
}

#[test]
fn method_hof_requires_exact_receiver() {
    let (il, interner, call) = method_call_il(Lang::JavaScript, "map", false);
    assert!(matches!(canon_call(&il, &interner, call), CallCanon::None));
}

#[test]
fn method_hof_allows_literal_sequence_receiver() {
    let (il, interner, call) = method_call_il(Lang::JavaScript, "map", true);
    assert!(matches!(
        canon_call(&il, &interner, call),
        CallCanon::HoF {
            kind: HoFKind::Map,
            ..
        }
    ));
}

#[test]
fn map_get_default_lambda_fallback_is_contract_controlled() {
    let (ruby, ruby_interner, ruby_call, ruby_fallback_value) =
        map_get_default_call_il(Lang::Ruby, "fetch", true, false);
    assert!(matches!(
        canon_call(&ruby, &ruby_interner, ruby_call),
        CallCanon::Builtin {
            op: Builtin::GetOrDefault,
            arg_olds
        } if arg_olds.len() == 3 && arg_olds[2] == ruby_fallback_value
    ));

    let (ruby_param, ruby_param_interner, ruby_param_call, _) =
        map_get_default_call_il(Lang::Ruby, "fetch", true, true);
    assert!(
        matches!(
            canon_call(&ruby_param, &ruby_param_interner, ruby_param_call),
            CallCanon::None
        ),
        "Ruby fetch block fallback must be zero-arg before exact canonicalization"
    );

    let (python, python_interner, python_call, python_fallback_value) =
        map_get_default_call_il(Lang::Python, "get", true, false);
    assert!(matches!(
        canon_call(&python, &python_interner, python_call),
        CallCanon::Builtin {
            op: Builtin::GetOrDefault,
            arg_olds
        } if arg_olds.len() == 3 && arg_olds[2] != python_fallback_value
    ));
}

#[test]
fn iterator_identity_adapter_requires_kernel_contract() {
    let (js, js_interner, js_iter) = method_call_no_arg_il(Lang::JavaScript, "iter", true);
    let js_domains = ReceiverDomainEvidenceIndex::new(&js, &js_interner);
    assert!(
        !exact_protocol_receiver(&js, &js_interner, &js_domains, js_iter),
        "a JS method named iter is not a Rust iterator adapter"
    );

    let (rust_bad, rust_bad_interner, rust_bad_iter) = method_call_il(Lang::Rust, "iter", true);
    let rust_bad_domains = ReceiverDomainEvidenceIndex::new(&rust_bad, &rust_bad_interner);
    assert!(
        !exact_protocol_receiver(
            &rust_bad,
            &rust_bad_interner,
            &rust_bad_domains,
            rust_bad_iter
        ),
        "Rust iter with unexpected arguments must not bypass the arity contract"
    );

    let (rust, rust_interner, rust_iter) = method_call_no_arg_il(Lang::Rust, "iter", true);
    let rust_domains = ReceiverDomainEvidenceIndex::new(&rust, &rust_interner);
    assert!(
        exact_protocol_receiver(&rust, &rust_interner, &rust_domains, rust_iter),
        "Rust iter stays admitted through iterator_identity_adapter_contract"
    );
}

#[test]
fn zip_protocol_pair_requires_kernel_contract() {
    let (js, js_interner, js_zip) = method_call_with_arg_il(Lang::JavaScript, "zip", true, true);
    let js_domains = ReceiverDomainEvidenceIndex::new(&js, &js_interner);
    assert!(
        !exact_protocol_receiver(&js, &js_interner, &js_domains, js_zip),
        "a JS method named zip is not a Rust zip protocol contract"
    );

    let (rust, rust_interner, rust_zip) = method_call_with_arg_il(Lang::Rust, "zip", true, true);
    let rust_domains = ReceiverDomainEvidenceIndex::new(&rust, &rust_interner);
    assert!(
        exact_protocol_receiver(&rust, &rust_interner, &rust_domains, rust_zip),
        "Rust zip stays admitted through method_call_contract"
    );
}

#[test]
fn method_bool_reduction_allows_typed_collection_receiver() {
    let (il, interner, call) =
        typed_method_call_il(Lang::TypeScript, "some", ParamSemantic::Collection, false);
    assert!(matches!(
        canon_call(&il, &interner, call),
        CallCanon::Builtin {
            op: Builtin::Any,
            arg_olds
        } if arg_olds.len() == 2
    ));
}

#[test]
fn method_bool_reduction_consumes_receiver_domain_evidence() {
    let (il, interner, call, _) = receiver_domain_method_call_il(DomainEvidence::Collection);
    assert!(matches!(
        canon_call(&il, &interner, call),
        CallCanon::Builtin {
            op: Builtin::Any,
            arg_olds
        } if arg_olds.len() == 2
    ));

    let (mut il, interner, call, receiver_span) =
        receiver_domain_method_call_il(DomainEvidence::Collection);
    il.evidence.push(evidence(
        next_evidence_id(&il),
        EvidenceAnchor::node(receiver_span, NodeKind::Var),
        EvidenceKind::Domain(DomainEvidence::Map),
        EvidenceStatus::Asserted,
    ));
    assert!(
        matches!(canon_call(&il, &interner, call), CallCanon::None),
        "conflicting receiver-domain evidence must not fall back to selector matching"
    );
}
