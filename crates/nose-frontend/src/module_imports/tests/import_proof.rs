use super::super::bindings::import_binding_key;
use super::support::{add_import_binding_evidence, coordinate_import_binding_assignment};
use nose_il::{stable_symbol_hash, EvidenceStatus, FileId, Lang};

#[test]
fn import_binding_key_requires_asserted_import_evidence() {
    let (mut il, _interner, span, assign, _rhs) =
        coordinate_import_binding_assignment(FileId(0), Lang::Java);
    assert_eq!(
        import_binding_key(&il, assign),
        None,
        "raw import coordinate Seqs must not prove import identity"
    );

    add_import_binding_evidence(&mut il, span, EvidenceStatus::Asserted);
    assert_eq!(
        import_binding_key(&il, assign),
        Some((stable_symbol_hash("java.util"), stable_symbol_hash("Map")))
    );
}

#[test]
fn import_binding_key_rejects_ambiguous_import_evidence_even_with_coordinates() {
    let (mut il, _interner, span, assign, _rhs) =
        coordinate_import_binding_assignment(FileId(0), Lang::Java);
    add_import_binding_evidence(&mut il, span, EvidenceStatus::Ambiguous);

    assert_eq!(
        import_binding_key(&il, assign),
        None,
        "ambiguous import evidence must close the imported literal rewrite"
    );
}
