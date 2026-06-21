use super::*;

#[test]
fn operator_law_contracts_preserve_comparison_gates() {
    for &lang in ALL_LANGS {
        let profile = semantics(lang);
        assert_eq!(
            profile
                .operators()
                .comparison_law(ComparisonLaw::LatticeStrictAbsorbsNonstrict)
                .is_some(),
            matches!(lang, Lang::C | Lang::Go | Lang::Java)
        );
        assert_eq!(
            profile
                .operators()
                .comparison_law(ComparisonLaw::LatticeLeNeToLt),
            Some(OperatorLawContract {
                law: ComparisonLaw::LatticeLeNeToLt,
                channel: ChannelEligibility::ExactProven,
                evidence: OperatorEvidence::ModeledIlOperator,
            })
        );
    }
}

#[test]
fn comparison_transform_contracts_carry_outputs_and_operand_swaps() {
    let ops = semantics(Lang::Python).operators();
    assert_eq!(
        ops.comparison_direction(Op::Gt),
        Some(ComparisonTransformContract {
            law: ComparisonLaw::DirectionCanon,
            input: Op::Gt,
            output: Op::Lt,
            swap_operands: true,
            channel: ChannelEligibility::ExactProven,
            evidence: OperatorEvidence::ModeledIlOperator,
        })
    );
    assert_eq!(
        ops.comparison_complement(Op::Lt)
            .map(|contract| (contract.output, contract.swap_operands)),
        Some((Op::Ge, false))
    );
    assert_eq!(
        ops.canonical_negated_comparison(Op::Lt)
            .map(|contract| (contract.output, contract.swap_operands)),
        Some((Op::Le, true))
    );
    assert_eq!(ops.comparison_direction(Op::Eq), None);
}

#[test]
fn cardinality_threshold_contracts_name_existing_operator_shapes() {
    let ops = semantics(Lang::JavaScript).operators();
    assert_eq!(
        ops.zero_cardinality_equality(Op::Eq),
        Some(CardinalityThresholdContract {
            threshold: CardinalityThreshold::Zero,
            predicate: CardinalityPredicate::Empty,
            channel: ChannelEligibility::ExactProven,
            evidence: OperatorEvidence::StaticCardinalityThreshold,
        })
    );
    assert_eq!(ops.zero_cardinality_equality(Op::Gt), None);
    assert_eq!(
        ops.cardinality_threshold(
            Op::Gt,
            false,
            CardinalityThreshold::Zero,
            CardinalityPredicate::NonEmpty,
        )
        .map(|contract| contract.predicate),
        Some(CardinalityPredicate::NonEmpty)
    );
    assert_eq!(
        ops.cardinality_threshold(
            Op::Eq,
            false,
            CardinalityThreshold::One,
            CardinalityPredicate::NonEmpty,
        ),
        None
    );
}

#[test]
fn membership_operator_contract_is_language_scoped() {
    assert_eq!(
        semantics(Lang::Python)
            .operators()
            .membership_operator(Op::In),
        Some(MembershipOperatorContract {
            operator: Op::In,
            receiver: MembershipOperatorReceiverContract::ExactCollectionOrMap,
            channel: ChannelEligibility::ExactProven,
            evidence: OperatorEvidence::ModeledIlOperator,
        })
    );
    assert_eq!(
        semantics(Lang::JavaScript)
            .operators()
            .membership_operator(Op::In),
        None
    );
    assert_eq!(
        semantics(Lang::Python)
            .operators()
            .membership_operator(Op::Eq),
        None
    );
}

#[test]
fn static_index_membership_contracts_are_js_like_and_threshold_constrained() {
    assert_eq!(
        static_index_membership_contract(Lang::JavaScript, "indexOf", 1),
        Some(StaticIndexMembershipContract {
            method: "indexOf",
            kind: StaticIndexMembershipKind::IndexOf,
            receiver: StaticIndexMembershipReceiverContract::StaticNonFloatLiteralCollection,
        })
    );
    assert_eq!(
        static_index_membership_contract(Lang::TypeScript, "findIndex", 1),
        Some(StaticIndexMembershipContract {
            method: "findIndex",
            kind: StaticIndexMembershipKind::FindIndex,
            receiver: StaticIndexMembershipReceiverContract::StaticNonFloatLiteralCollection,
        })
    );
    assert_eq!(
        static_index_membership_contract(Lang::Python, "indexOf", 1),
        None
    );
    assert_eq!(
        static_index_membership_contract(Lang::JavaScript, "indexOf", 2),
        None
    );
    assert_eq!(
        static_index_membership_contract(Lang::JavaScript, "includes", 1),
        None
    );
    assert_eq!(
        semantics(Lang::JavaScript)
            .operators()
            .static_index_membership_threshold(Op::Ne, false, IndexMembershipThreshold::MinusOne)
            .map(|contract| contract.evidence),
        Some(OperatorEvidence::JsLikeStaticIndexMembershipThreshold)
    );
    assert!(semantics(Lang::TypeScript)
        .operators()
        .static_index_membership_threshold(Op::Le, true, IndexMembershipThreshold::Zero)
        .is_some());
    assert!(semantics(Lang::Python)
        .operators()
        .static_index_membership_threshold(Op::Ne, false, IndexMembershipThreshold::MinusOne)
        .is_none());
    assert!(semantics(Lang::JavaScript)
        .operators()
        .static_index_membership_threshold(Op::Eq, false, IndexMembershipThreshold::MinusOne)
        .is_none());
}

#[test]
fn imported_namespace_function_contracts_carry_module_and_receiver_proof() {
    assert_eq!(
        imported_namespace_function_contract(Lang::Python, "prod", 1),
        Some(ImportedNamespaceFunctionContract {
            module: "math",
            function: "prod",
            receiver: MethodReceiverContract::ImportedNamespace("math"),
            semantic: ImportedNamespaceFunctionSemantic::ProductReduction {
                op: Op::Mul,
                identity: 1,
            },
        })
    );
    assert_eq!(
        imported_namespace_function_contract(Lang::Python, "prod", 2)
            .map(|contract| contract.semantic),
        Some(ImportedNamespaceFunctionSemantic::ProductReduction {
            op: Op::Mul,
            identity: 1,
        })
    );
    assert_eq!(
        imported_namespace_function_contract(Lang::JavaScript, "prod", 1),
        None
    );
    assert_eq!(
        imported_namespace_function_contract(Lang::Python, "prod", 3),
        None
    );
    assert_eq!(
        imported_namespace_function_contract(Lang::Python, "sum", 1),
        None
    );
}

#[test]
fn nullish_global_contracts_are_js_like_and_unshadowed() {
    assert_eq!(
        nullish_global_contract(Lang::JavaScript, "undefined"),
        Some(NullishGlobalContract {
            name: "undefined",
            requires_unshadowed: true,
        })
    );
    assert_eq!(
        nullish_global_contract(Lang::TypeScript, "undefined"),
        Some(NullishGlobalContract {
            name: "undefined",
            requires_unshadowed: true,
        })
    );
    assert_eq!(nullish_global_contract(Lang::Python, "undefined"), None);
    assert_eq!(nullish_global_contract(Lang::JavaScript, "null"), None);
}

#[test]
fn builder_append_contracts_are_language_and_arity_constrained() {
    let rust_push = builder_append_method_contract(Lang::Rust, "push", 1)
        .expect("rust push builder append contract");
    assert_eq!(rust_push.pack_id, RUST_LANGUAGE_PACK_ID);
    assert_eq!(rust_push.effect, EffectEvidenceKind::BuilderAppendCall);
    assert_eq!(
        rust_push.receiver,
        MethodEffectReceiverContract::ActiveCollectionBuilder
    );
    assert!(builder_append_method_contract(Lang::Rust, "push", 2).is_none());
    assert!(builder_append_method_contract(Lang::Java, "add", 1).is_some());
    assert_eq!(
        builder_append_method_contract(Lang::Java, "add", 1)
            .expect("java add builder append contract")
            .pack_id,
        JAVA_LANGUAGE_PACK_ID
    );
    assert_eq!(
        builder_append_method_contract(Lang::JavaScript, "push", 1)
            .expect("javascript push builder append contract")
            .pack_id,
        JS_TS_LANGUAGE_PACK_ID
    );
    assert_eq!(
        builder_append_method_contract(Lang::TypeScript, "push", 1)
            .expect("typescript push builder append contract")
            .pack_id,
        JS_TS_LANGUAGE_PACK_ID
    );
    assert_eq!(
        builder_append_method_contract(Lang::Python, "append", 1)
            .expect("python append builder append contract")
            .pack_id,
        PYTHON_LANGUAGE_PACK_ID
    );
    assert!(builder_append_method_contract(Lang::Ruby, "push", 1).is_none());
}

#[test]
fn unproven_membership_like_guard_is_negative_api_policy() {
    let guard = unproven_membership_like_method_contract(Lang::TypeScript, "includes", 1)
        .expect("includes guard");
    assert_eq!(guard.pack_id, FIRST_PARTY_PACK_ID);
    assert_eq!(guard.id, ApiGuardContractId::UnprovenMembershipLikeCall);
    assert_eq!(guard.lang, Lang::TypeScript);
    assert_eq!(guard.method, "includes");
    assert_eq!(guard.arg_count, 1);
    assert_eq!(guard.channel, ChannelEligibility::ExactProven);
    assert!(unproven_membership_like_method_contract(Lang::Java, "containsKey", 1).is_some());
    assert!(unproven_membership_like_method_contract(Lang::Python, "custom", 1).is_none());
}

#[test]
fn map_builder_index_write_contracts_are_language_scoped() {
    let contract =
        map_builder_index_write_contract(Lang::Python).expect("python map builder contract");
    assert_eq!(contract.pack_id, PYTHON_LANGUAGE_PACK_ID);
    assert_eq!(contract.id, IndexWriteContractId::MapBuilderEntryWrite);
    assert_eq!(
        contract.receiver,
        IndexWriteReceiverContract::ActiveMapBuilder
    );
    assert_eq!(contract.required_effect, EffectEvidenceKind::BindingWrite);
    assert_eq!(contract.channel, ChannelEligibility::ExactProven);
    assert!(map_builder_index_write_contract(Lang::Ruby).is_none());
    assert!(map_builder_index_write_contract(Lang::JavaScript).is_none());
}
