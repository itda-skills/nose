use super::*;

#[test]
fn value_domain_inference_treats_retained_float_literals_as_numeric() {
    assert_eq!(
        inferred_domains_for_added_literal(Payload::LitInt(3)),
        vec![ValueDomain::Number]
    );
    assert_eq!(
        inferred_domains_for_added_literal(Payload::LitFloat(0xBEEF)),
        vec![ValueDomain::Number]
    );
}

#[test]
fn value_domain_inference_does_not_treat_python_repetition_as_numeric_proof() {
    assert_eq!(
        inferred_domains_for_multiplied_literal(Lang::Python),
        vec![ValueDomain::Unknown],
        "Python `*` can be string/list repetition, so untyped operands must fail closed"
    );
    assert_eq!(
        inferred_domains_for_multiplied_literal(Lang::Ruby),
        vec![ValueDomain::Unknown],
        "Ruby `*` can be repetition, so untyped operands must fail closed"
    );
    assert_eq!(
        inferred_domains_for_multiplied_literal(Lang::Rust),
        vec![ValueDomain::Number],
        "Rust `*` remains a strict numeric operator for the first-party language profile"
    );
}

fn inferred_domains_for_multiplied_literal(lang: Lang) -> Vec<ValueDomain> {
    let sp = Span::synthetic(FileId(0));
    let mut b = IlBuilder::new(FileId(0));
    let param = b.add(NodeKind::Param, Payload::Cid(0), sp, &[]);
    let varx = b.add(NodeKind::Var, Payload::Cid(0), sp, &[]);
    let lit = b.add(NodeKind::Lit, Payload::LitInt(2), sp, &[]);
    let mul = b.add(NodeKind::BinOp, Payload::Op(Op::Mul), sp, &[varx, lit]);
    let ret = b.add(NodeKind::Return, Payload::None, sp, &[mul]);
    let func = b.add(NodeKind::Func, Payload::None, sp, &[param, ret]);
    let il = b.finish(
        func,
        FileMeta {
            path: "t".into(),
            lang,
        },
        Vec::new(),
        Vec::new(),
    );
    semantics(lang)
        .operators()
        .infer_param_value_domains(&il, func)
}

#[test]
fn first_party_profile_wraps_each_language() {
    for &lang in ALL_LANGS {
        let profile = semantics(lang);
        assert_eq!(profile.lang(), lang);
        let expected_pack_id = if lang == Lang::C {
            C_LANGUAGE_PACK_ID
        } else {
            FIRST_PARTY_PACK_ID
        };
        assert_eq!(profile.pack_id(), expected_pack_id);
        assert_eq!(profile.trust(), PackTrust::DefaultFirstParty);
    }
}

#[test]
fn domain_evidence_preserves_param_semantic_boundaries() {
    assert_eq!(
        domain_evidence_from_param_semantic(ParamSemantic::Array),
        DomainEvidence::Array
    );
    assert_eq!(
        domain_evidence_from_param_semantic(ParamSemantic::Boolean),
        DomainEvidence::Boolean
    );
    assert_eq!(
        domain_evidence_from_param_semantic(ParamSemantic::Float),
        DomainEvidence::Float
    );
    assert_eq!(
        domain_evidence_from_param_semantic(ParamSemantic::FutureLike),
        DomainEvidence::FutureLike
    );
    assert_eq!(
        domain_evidence_from_param_semantic(ParamSemantic::Iterable),
        DomainEvidence::Iterable
    );
    assert_eq!(
        domain_evidence_from_param_semantic(ParamSemantic::Iterator),
        DomainEvidence::Iterator
    );
    assert_eq!(
        domain_evidence_from_param_semantic(ParamSemantic::Record),
        DomainEvidence::Record
    );
    assert_eq!(
        domain_evidence_from_param_semantic(ParamSemantic::Result),
        DomainEvidence::Result
    );
}

#[test]
fn domain_evidence_predicates_match_domain_families() {
    assert!(DomainEvidence::Array.is_array_collection_or_set());
    assert!(DomainEvidence::Boolean.is_boolean());
    assert!(DomainEvidence::Collection.is_array_or_collection());
    assert!(DomainEvidence::Set.is_collection_or_set());
    assert!(DomainEvidence::Float.is_float());
    assert!(DomainEvidence::FutureLike.is_future_like());
    assert!(DomainEvidence::PromiseLike.is_future_like());
    assert!(DomainEvidence::Iterable.is_iterable_or_iterator());
    assert!(DomainEvidence::Iterator.is_iterable_or_iterator());
    assert!(DomainEvidence::Map.is_map());
    assert!(DomainEvidence::Nominal {
        type_hash: stable_symbol_hash("pkg.Widget")
    }
    .is_nominal(stable_symbol_hash("pkg.Widget")));
    assert!(DomainEvidence::Option.is_option());
    assert!(DomainEvidence::PromiseLike.is_promise_like());
    assert!(DomainEvidence::Record.is_record());
    assert!(DomainEvidence::Result.is_result());
}

#[test]
fn scalar_domain_evidence_predicates_keep_numeric_axes_separate() {
    assert!(DomainEvidence::String.is_string());
    assert!(DomainEvidence::ByteArray.is_byte_array());
    assert!(DomainEvidence::Integer.is_integer());
    assert!(DomainEvidence::Number.is_integer_or_number());
    assert!(DomainEvidence::Float.is_integer_or_number());
    assert!(DomainEvidence::Integer.is_integer_or_number());
    assert!(!DomainEvidence::Number.is_integer());
    assert!(!DomainEvidence::Array.is_collection_or_set());
    assert!(!DomainEvidence::Set.is_array_or_collection());
}

#[test]
fn domain_requirements_accept_matching_evidence() {
    assert!(DomainRequirement::Boolean.accepts(DomainEvidence::Boolean));
    assert!(DomainRequirement::CollectionOrMap.accepts(DomainEvidence::Array));
    assert!(DomainRequirement::CollectionOrMap.accepts(DomainEvidence::Map));
    assert!(DomainRequirement::Float.accepts(DomainEvidence::Float));
    assert!(DomainRequirement::FutureLike.accepts(DomainEvidence::FutureLike));
    assert!(DomainRequirement::FutureLike.accepts(DomainEvidence::PromiseLike));
    assert!(DomainRequirement::Iterable.accepts(DomainEvidence::Iterable));
    assert!(DomainRequirement::Iterator.accepts(DomainEvidence::Iterator));
    assert!(DomainRequirement::IterableOrIterator.accepts(DomainEvidence::Iterable));
    assert!(DomainRequirement::IterableOrIterator.accepts(DomainEvidence::Iterator));
    assert!(DomainRequirement::Nominal {
        type_hash: stable_symbol_hash("pkg.Widget")
    }
    .accepts(DomainEvidence::Nominal {
        type_hash: stable_symbol_hash("pkg.Widget")
    }));
}

#[test]
fn domain_requirements_reject_mismatches_and_map_value_domains() {
    assert!(DomainRequirement::Number.accepts(DomainEvidence::Float));
    assert!(DomainRequirement::Record.accepts(DomainEvidence::Record));
    assert!(DomainRequirement::Result.accepts(DomainEvidence::Result));
    assert!(DomainRequirement::SetOrMap.accepts(DomainEvidence::Set));
    assert!(DomainRequirement::SetOrMap.accepts(DomainEvidence::Map));
    assert!(!DomainRequirement::SetOrMap.accepts(DomainEvidence::Collection));
    assert!(DomainRequirement::PromiseLike.accepts(DomainEvidence::PromiseLike));
    assert!(!DomainRequirement::PromiseLike.accepts(DomainEvidence::String));
    assert_eq!(
        ValueDomain::from_domain_evidence(DomainEvidence::Boolean),
        Some(ValueDomain::Boolean)
    );
    assert_eq!(
        ValueDomain::from_domain_evidence(DomainEvidence::Float),
        Some(ValueDomain::Number)
    );
    assert_eq!(
        ValueDomain::from_domain_evidence(DomainEvidence::PromiseLike),
        None
    );
}
