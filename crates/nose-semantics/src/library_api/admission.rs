use super::*;

pub fn library_api_contract_evidence_for_call(
    il: &Il,
    interner: &Interner,
    node: NodeId,
    id: LibraryApiContractId,
    callee: LibraryApiCalleeContract,
    arg_count: usize,
) -> LibraryApiEvidenceStatus {
    if il.kind(node) != NodeKind::Call || arg_count > u16::MAX as usize {
        return LibraryApiEvidenceStatus::Rejected;
    }
    let expected = LibraryApiEvidenceKind::Contract {
        contract_hash: library_api_contract_id_hash(id),
        callee_hash: library_api_callee_contract_hash(callee),
        arity: arg_count as u16,
    };
    let span = il.node(node).span;
    let mut saw_library_api_evidence = false;
    let mut admitted = false;
    for record in il.evidence_anchored_at(span) {
        if record.anchor != EvidenceAnchor::node(span, NodeKind::Call) {
            continue;
        }
        let EvidenceKind::LibraryApi(api) = record.kind else {
            continue;
        };
        saw_library_api_evidence = true;
        if record.status != EvidenceStatus::Asserted
            || api != expected
            || !library_api_record_provenance_matches_contract(id, callee, record)
            || !il.evidence_dependencies_asserted(record)
            || !library_api_callee_shape_matches(il, interner, node, callee)
            || !library_api_dependencies_match_callee(il, interner, node, callee, record)
        {
            return LibraryApiEvidenceStatus::Rejected;
        }
        admitted = true;
    }
    if admitted {
        LibraryApiEvidenceStatus::Admitted
    } else if saw_library_api_evidence {
        LibraryApiEvidenceStatus::Rejected
    } else {
        LibraryApiEvidenceStatus::Missing
    }
}

pub fn library_api_contract_evidence_for_node(
    il: &Il,
    interner: &Interner,
    node: NodeId,
    id: LibraryApiContractId,
    callee: LibraryApiCalleeContract,
    arg_count: usize,
) -> LibraryApiEvidenceStatus {
    if arg_count > u16::MAX as usize {
        return LibraryApiEvidenceStatus::Rejected;
    }
    let expected = LibraryApiEvidenceKind::Contract {
        contract_hash: library_api_contract_id_hash(id),
        callee_hash: library_api_callee_contract_hash(callee),
        arity: arg_count as u16,
    };
    let anchor = EvidenceAnchor::node(il.node(node).span, il.kind(node));
    let mut saw_library_api_evidence = false;
    let mut admitted = false;
    for record in il.evidence_anchored_at(anchor.span()) {
        if record.anchor != anchor {
            continue;
        }
        let EvidenceKind::LibraryApi(api) = record.kind else {
            continue;
        };
        saw_library_api_evidence = true;
        if record.status != EvidenceStatus::Asserted
            || api != expected
            || !library_api_record_provenance_matches_contract(id, callee, record)
            || !il.evidence_dependencies_asserted(record)
            || !library_api_node_callee_shape_matches(il, interner, node, callee)
            || !library_api_dependencies_match_callee_node(il, interner, node, callee, record)
        {
            return LibraryApiEvidenceStatus::Rejected;
        }
        admitted = true;
    }
    if admitted {
        LibraryApiEvidenceStatus::Admitted
    } else if saw_library_api_evidence {
        LibraryApiEvidenceStatus::Rejected
    } else {
        LibraryApiEvidenceStatus::Missing
    }
}

pub fn library_api_contract_evidence_at_call_span(
    il: &Il,
    interner: &Interner,
    query: LibraryApiSpanEvidenceQuery,
) -> LibraryApiEvidenceStatus {
    let Some(span) = query.call_span else {
        return LibraryApiEvidenceStatus::Missing;
    };
    if query.arg_count > u16::MAX as usize {
        return LibraryApiEvidenceStatus::Rejected;
    }
    let expected = LibraryApiEvidenceKind::Contract {
        contract_hash: library_api_contract_id_hash(query.id),
        callee_hash: library_api_callee_contract_hash(query.callee),
        arity: query.arg_count as u16,
    };
    let source_call = node_at_span_with_kind(il, span, NodeKind::Call);
    let mut saw_library_api_evidence = false;
    let mut admitted = false;
    for record in il.evidence_anchored_at(span) {
        if record.anchor != EvidenceAnchor::node(span, NodeKind::Call) {
            continue;
        }
        let EvidenceKind::LibraryApi(api) = record.kind else {
            continue;
        };
        saw_library_api_evidence = true;
        let source_call_matches = source_call.is_some_and(|node| {
            library_api_source_call_spans_match_query(
                il,
                node,
                query.callee_span,
                query.receiver_span,
            ) && library_api_callee_shape_matches(il, interner, node, query.callee)
                && library_api_dependencies_match_callee(il, interner, node, query.callee, record)
        });
        let span_query_matches = library_api_dependencies_match_callee_at_span(
            il,
            interner,
            span,
            query.callee_span,
            query.receiver_span,
            query.callee,
            record,
        );
        if record.status != EvidenceStatus::Asserted
            || api != expected
            || !library_api_record_provenance_matches_contract(query.id, query.callee, record)
            || !il.evidence_dependencies_asserted(record)
            || (!source_call_matches && !span_query_matches)
        {
            return LibraryApiEvidenceStatus::Rejected;
        }
        admitted = true;
    }
    if admitted {
        LibraryApiEvidenceStatus::Admitted
    } else if saw_library_api_evidence {
        LibraryApiEvidenceStatus::Rejected
    } else {
        LibraryApiEvidenceStatus::Missing
    }
}

pub(in crate::library_api) fn library_api_record_provenance_matches_contract(
    id: LibraryApiContractId,
    callee: LibraryApiCalleeContract,
    record: &EvidenceRecord,
) -> bool {
    match id {
        LibraryApiContractId::PythonBuiltinCollectionFactory => {
            record.provenance.emitter == EvidenceEmitter::FirstParty
                && record.provenance.pack_hash
                    == Some(stable_symbol_hash(
                        PYTHON_BUILTIN_COLLECTION_FACTORY_PACK_ID,
                    ))
                && record.provenance.rule_hash
                    == Some(stable_symbol_hash(
                        PYTHON_BUILTIN_COLLECTION_FACTORY_PRODUCER_ID,
                    ))
        }
        LibraryApiContractId::PythonImportedCollectionFactory => {
            record.provenance.emitter == EvidenceEmitter::FirstParty
                && record.provenance.pack_hash
                    == Some(stable_symbol_hash(PYTHON_STDLIB_COLLECTION_FACTORY_PACK_ID))
                && record.provenance.rule_hash
                    == Some(stable_symbol_hash(
                        PYTHON_STDLIB_COLLECTION_FACTORY_PRODUCER_ID,
                    ))
        }
        LibraryApiContractId::ImportedNamespaceFunction(
            ImportedNamespaceFunctionSemantic::ProductReduction {
                op: Op::Mul,
                identity: 1,
            },
        ) => {
            record.provenance.emitter == EvidenceEmitter::FirstParty
                && record.provenance.pack_hash
                    == Some(stable_symbol_hash(PYTHON_STDLIB_MATH_PACK_ID))
                && record.provenance.rule_hash
                    == Some(stable_symbol_hash(PYTHON_STDLIB_MATH_PRODUCER_ID))
        }
        LibraryApiContractId::PromiseFactory(PromiseFactoryKind::Resolve)
        | LibraryApiContractId::PromiseThen => {
            record.provenance.emitter == EvidenceEmitter::FirstParty
                && record.provenance.pack_hash
                    == Some(stable_symbol_hash(JS_LIKE_BUILTIN_PROMISE_PACK_ID))
                && record.provenance.rule_hash
                    == Some(stable_symbol_hash(JS_LIKE_BUILTIN_PROMISE_PRODUCER_ID))
        }
        LibraryApiContractId::MapKeyViewWrapper | LibraryApiContractId::JsArrayIsArray => {
            record.provenance.emitter == EvidenceEmitter::FirstParty
                && record.provenance.pack_hash
                    == Some(stable_symbol_hash(JS_LIKE_BUILTIN_ARRAY_PACK_ID))
                && record.provenance.rule_hash
                    == Some(stable_symbol_hash(JS_LIKE_BUILTIN_ARRAY_PRODUCER_ID))
        }
        LibraryApiContractId::JsBooleanCoercion => {
            record.provenance.emitter == EvidenceEmitter::FirstParty
                && record.provenance.pack_hash
                    == Some(stable_symbol_hash(JS_LIKE_BUILTIN_BOOLEAN_PACK_ID))
                && record.provenance.rule_hash
                    == Some(stable_symbol_hash(JS_LIKE_BUILTIN_BOOLEAN_PRODUCER_ID))
        }
        LibraryApiContractId::RegexTest => {
            record.provenance.emitter == EvidenceEmitter::FirstParty
                && record.provenance.pack_hash
                    == Some(stable_symbol_hash(JS_LIKE_BUILTIN_REGEX_PACK_ID))
                && record.provenance.rule_hash
                    == Some(stable_symbol_hash(JS_LIKE_BUILTIN_REGEX_PRODUCER_ID))
        }
        LibraryApiContractId::JsLikeStaticIndexMembership(_) => {
            record.provenance.emitter == EvidenceEmitter::FirstParty
                && record.provenance.pack_hash
                    == Some(stable_symbol_hash(
                        JS_LIKE_BUILTIN_STATIC_INDEX_MEMBERSHIP_PACK_ID,
                    ))
                && record.provenance.rule_hash
                    == Some(stable_symbol_hash(
                        JS_LIKE_BUILTIN_STATIC_INDEX_MEMBERSHIP_PRODUCER_ID,
                    ))
        }
        LibraryApiContractId::JsLikeSetConstructor | LibraryApiContractId::JsLikeMapConstructor => {
            record.provenance.emitter == EvidenceEmitter::FirstParty
                && record.provenance.pack_hash
                    == Some(stable_symbol_hash(
                        JS_LIKE_BUILTIN_COLLECTION_CONSTRUCTOR_PACK_ID,
                    ))
                && record.provenance.rule_hash
                    == Some(stable_symbol_hash(
                        JS_LIKE_BUILTIN_COLLECTION_CONSTRUCTOR_PRODUCER_ID,
                    ))
        }
        LibraryApiContractId::RustVecMacroFactory | LibraryApiContractId::RustVecNewFactory => {
            record.provenance.emitter == EvidenceEmitter::FirstParty
                && record.provenance.pack_hash == Some(stable_symbol_hash(RUST_STDLIB_VEC_PACK_ID))
                && record.provenance.rule_hash
                    == Some(stable_symbol_hash(RUST_STDLIB_VEC_PRODUCER_ID))
        }
        LibraryApiContractId::RustOptionSomeConstructor
        | LibraryApiContractId::RustOptionNoneSentinel
        | LibraryApiContractId::RustOptionAndThen => {
            record.provenance.emitter == EvidenceEmitter::FirstParty
                && record.provenance.pack_hash
                    == Some(stable_symbol_hash(RUST_STDLIB_OPTION_PACK_ID))
                && record.provenance.rule_hash
                    == Some(stable_symbol_hash(RUST_STDLIB_OPTION_PRODUCER_ID))
        }
        LibraryApiContractId::ScalarIntegerMethod(_)
            if matches!(
                callee,
                LibraryApiCalleeContract::Method {
                    receiver: MethodReceiverContract::ExactInteger,
                    ..
                }
            ) =>
        {
            record.provenance.emitter == EvidenceEmitter::FirstParty
                && record.provenance.pack_hash
                    == Some(stable_symbol_hash(RUST_STDLIB_INTEGER_METHOD_PACK_ID))
                && record.provenance.rule_hash
                    == Some(stable_symbol_hash(RUST_STDLIB_INTEGER_METHOD_PRODUCER_ID))
        }
        LibraryApiContractId::ScalarIntegerMethod(_)
            if matches!(
                callee,
                LibraryApiCalleeContract::Method {
                    receiver: MethodReceiverContract::UnshadowedGlobal("Math"),
                    ..
                }
            ) =>
        {
            record.provenance.emitter == EvidenceEmitter::FirstParty
                && record.provenance.pack_hash == Some(stable_symbol_hash(JAVA_STDLIB_MATH_PACK_ID))
                && record.provenance.rule_hash
                    == Some(stable_symbol_hash(JAVA_STDLIB_MATH_PRODUCER_ID))
        }
        LibraryApiContractId::IteratorIdentityAdapter => {
            record.provenance.emitter == EvidenceEmitter::FirstParty
                && record.provenance.pack_hash
                    == Some(stable_symbol_hash(ITERATOR_IDENTITY_ADAPTER_PACK_ID))
                && record.provenance.rule_hash
                    == Some(stable_symbol_hash(ITERATOR_IDENTITY_ADAPTER_PRODUCER_ID))
        }
        LibraryApiContractId::RustStdCollectionFactory => {
            record.provenance.emitter == EvidenceEmitter::FirstParty
                && record.provenance.pack_hash
                    == Some(stable_symbol_hash(RUST_STDLIB_COLLECTION_FACTORY_PACK_ID))
                && record.provenance.rule_hash
                    == Some(stable_symbol_hash(
                        RUST_STDLIB_COLLECTION_FACTORY_PRODUCER_ID,
                    ))
        }
        LibraryApiContractId::RustStdMapFactory => {
            record.provenance.emitter == EvidenceEmitter::FirstParty
                && record.provenance.pack_hash
                    == Some(stable_symbol_hash(RUST_STDLIB_MAP_FACTORY_PACK_ID))
                && record.provenance.rule_hash
                    == Some(stable_symbol_hash(RUST_STDLIB_MAP_FACTORY_PRODUCER_ID))
        }
        LibraryApiContractId::JavaMapFactory(_) => {
            record.provenance.emitter == EvidenceEmitter::FirstParty
                && record.provenance.pack_hash
                    == Some(stable_symbol_hash(JAVA_STDLIB_MAP_FACTORY_PACK_ID))
                && record.provenance.rule_hash
                    == Some(stable_symbol_hash(JAVA_STDLIB_MAP_FACTORY_PRODUCER_ID))
        }
        LibraryApiContractId::JavaMapEntryFactory => {
            record.provenance.emitter == EvidenceEmitter::FirstParty
                && record.provenance.pack_hash
                    == Some(stable_symbol_hash(JAVA_STDLIB_MAP_ENTRY_PACK_ID))
                && record.provenance.rule_hash
                    == Some(stable_symbol_hash(JAVA_STDLIB_MAP_ENTRY_PRODUCER_ID))
        }
        LibraryApiContractId::JavaCollectionFactory(_) => {
            record.provenance.emitter == EvidenceEmitter::FirstParty
                && record.provenance.pack_hash
                    == Some(stable_symbol_hash(JAVA_STDLIB_COLLECTION_FACTORY_PACK_ID))
                && record.provenance.rule_hash
                    == Some(stable_symbol_hash(
                        JAVA_STDLIB_COLLECTION_FACTORY_PRODUCER_ID,
                    ))
        }
        LibraryApiContractId::JavaCollectionConstructor(_) => {
            record.provenance.emitter == EvidenceEmitter::FirstParty
                && record.provenance.pack_hash
                    == Some(stable_symbol_hash(
                        JAVA_STDLIB_COLLECTION_CONSTRUCTOR_PACK_ID,
                    ))
                && record.provenance.rule_hash
                    == Some(stable_symbol_hash(
                        JAVA_STDLIB_COLLECTION_CONSTRUCTOR_PRODUCER_ID,
                    ))
        }
        LibraryApiContractId::StaticCollectionAdapter => {
            record.provenance.emitter == EvidenceEmitter::FirstParty
                && record.provenance.pack_hash
                    == Some(stable_symbol_hash(
                        JAVA_STDLIB_STATIC_COLLECTION_ADAPTER_PACK_ID,
                    ))
                && record.provenance.rule_hash
                    == Some(stable_symbol_hash(
                        JAVA_STDLIB_STATIC_COLLECTION_ADAPTER_PRODUCER_ID,
                    ))
        }
        LibraryApiContractId::RubySetFactory => {
            record.provenance.emitter == EvidenceEmitter::FirstParty
                && record.provenance.pack_hash == Some(stable_symbol_hash(RUBY_STDLIB_SET_PACK_ID))
                && record.provenance.rule_hash
                    == Some(stable_symbol_hash(RUBY_STDLIB_SET_PRODUCER_ID))
        }
        _ => true,
    }
}

fn library_api_source_call_spans_match_query(
    il: &Il,
    source_call: NodeId,
    callee_span: Option<Span>,
    receiver_span: Option<Span>,
) -> bool {
    let Some(&callee) = il.children(source_call).first() else {
        return false;
    };
    if callee_span.is_some_and(|span| il.node(callee).span != span) {
        return false;
    }
    if let Some(span) = receiver_span {
        let Some(&receiver) = il.children(callee).first() else {
            return false;
        };
        if il.node(receiver).span != span {
            return false;
        }
    }
    true
}
