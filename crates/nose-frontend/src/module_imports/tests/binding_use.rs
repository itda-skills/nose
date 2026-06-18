use super::super::bindings::BindingUseIndex;
use super::support::module_with_binding_method;

#[test]
fn module_binding_push_marks_export_unsafe() {
    let (il, interner, lookup, assign) = module_with_binding_method("push");
    let binding_uses = BindingUseIndex::new(&il, &interner);
    assert!(
        binding_uses.exported_binding_unsafe(&il, lookup, assign),
        "exported literal bindings mutated through push must not be imported as immutable"
    );
}

#[test]
fn module_binding_get_is_not_a_mutation() {
    let (il, interner, lookup, assign) = module_with_binding_method("get");
    let binding_uses = BindingUseIndex::new(&il, &interner);
    assert!(
        !binding_uses.binding_mutated(&il, lookup, assign),
        "read-only lookup methods should not block immutable import replacement"
    );
}
