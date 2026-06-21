use super::*;

pub(super) fn assert_group() {
    let laws = builtin_pack_descriptor(VALUE_GRAPH_LAW_PACK_ID).expect("value law descriptor");
    assert_eq!(laws.kind, SemanticPackKind::LawPack);
    assert_eq!(laws.counts().value_laws, pack_facing_value_laws().len());
    assert_eq!(
        laws.value_law_ids(),
        pack_facing_value_laws()
            .iter()
            .map(|law| law.law_id)
            .collect::<Vec<_>>()
    );
    assert!(laws
        .conformance_refs()
        .contains(&"clamp-float-hard-negative"));
}
