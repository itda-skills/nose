#![allow(clippy::too_many_arguments, clippy::too_many_lines)]

use super::*;

mod admission_resolvers;
mod callee_sources;
mod canonical_builtin;
mod resolution;

fn library_api_record(
    id: u32,
    span: Span,
    contract_id: LibraryApiContractId,
    callee: LibraryApiCalleeContract,
    status: EvidenceStatus,
    dependencies: &[u32],
) -> EvidenceRecord {
    library_api_record_with_arity(id, span, contract_id, callee, 1, status, dependencies)
}

fn language_core_symbol_record(
    id: u32,
    anchor: EvidenceAnchor,
    symbol: SymbolEvidenceKind,
    status: EvidenceStatus,
    dependencies: &[u32],
    lang: Lang,
) -> EvidenceRecord {
    language_core_evidence_with_dependencies(
        id,
        anchor,
        EvidenceKind::Symbol(symbol),
        status,
        dependencies.iter().copied().map(EvidenceId).collect(),
        lang,
    )
}

fn library_api_record_with_provenance(
    id: u32,
    span: Span,
    contract_id: LibraryApiContractId,
    callee: LibraryApiCalleeContract,
    status: EvidenceStatus,
    dependencies: &[u32],
    pack_id: &str,
    rule: &str,
) -> EvidenceRecord {
    library_api_record_with_provenance_and_arity(
        id,
        span,
        contract_id,
        callee,
        1,
        status,
        dependencies,
        pack_id,
        rule,
    )
}

fn library_api_record_with_provenance_and_arity(
    id: u32,
    span: Span,
    contract_id: LibraryApiContractId,
    callee: LibraryApiCalleeContract,
    arity: u16,
    status: EvidenceStatus,
    dependencies: &[u32],
    pack_id: &str,
    rule: &str,
) -> EvidenceRecord {
    let mut record =
        library_api_record_with_arity(id, span, contract_id, callee, arity, status, dependencies);
    record.provenance.pack_hash = Some(stable_symbol_hash(pack_id));
    record.provenance.rule_hash = Some(stable_symbol_hash(rule));
    record
}

fn property_builtin_record(
    id: u32,
    span: Span,
    contract: LibraryPropertyBuiltinContract,
    status: EvidenceStatus,
    dependencies: &[u32],
) -> EvidenceRecord {
    library_api_record_with_provenance_and_arity(
        id,
        span,
        contract.id,
        contract.callee,
        0,
        status,
        dependencies,
        PROPERTY_BUILTIN_PROTOCOL_PACK_ID,
        PROPERTY_BUILTIN_PROTOCOL_PRODUCER_ID,
    )
}

fn python_builtin_collection_factory_record(
    id: u32,
    span: Span,
    contract: LibraryCollectionFactoryContract,
    status: EvidenceStatus,
    dependencies: &[u32],
) -> EvidenceRecord {
    library_api_record_with_provenance(
        id,
        span,
        contract.id,
        contract.callee,
        status,
        dependencies,
        PYTHON_BUILTIN_COLLECTION_FACTORY_PACK_ID,
        PYTHON_BUILTIN_COLLECTION_FACTORY_PRODUCER_ID,
    )
}

fn python_stdlib_collection_factory_record(
    id: u32,
    span: Span,
    contract: LibraryCollectionFactoryContract,
    status: EvidenceStatus,
    dependencies: &[u32],
) -> EvidenceRecord {
    library_api_record_with_provenance(
        id,
        span,
        contract.id,
        contract.callee,
        status,
        dependencies,
        PYTHON_STDLIB_COLLECTION_FACTORY_PACK_ID,
        PYTHON_STDLIB_COLLECTION_FACTORY_PRODUCER_ID,
    )
}

fn python_stdlib_math_record(
    id: u32,
    span: Span,
    contract: LibraryImportedNamespaceFunctionContract,
    arity: u16,
    status: EvidenceStatus,
    dependencies: &[u32],
) -> EvidenceRecord {
    library_api_record_with_provenance_and_arity(
        id,
        span,
        contract.id,
        contract.callee,
        arity,
        status,
        dependencies,
        PYTHON_STDLIB_MATH_PACK_ID,
        PYTHON_STDLIB_MATH_PRODUCER_ID,
    )
}

mod records;
pub(super) use records::*;
