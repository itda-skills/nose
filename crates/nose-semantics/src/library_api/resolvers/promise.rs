use super::*;

pub fn admitted_promise_then_at_call(
    il: &Il,
    interner: &Interner,
    call: NodeId,
) -> Option<AdmittedLibraryApiCall<LibraryPromiseThenContract>> {
    admitted_receiver_method_contract_call(il, interner, call, library_promise_then_contract)
}

pub fn admitted_promise_catch_at_call(
    il: &Il,
    interner: &Interner,
    call: NodeId,
) -> Option<AdmittedLibraryApiCall<LibraryPromiseCatchContract>> {
    admitted_receiver_method_contract_call(il, interner, call, library_promise_catch_contract)
}

pub fn admitted_promise_finally_at_call(
    il: &Il,
    interner: &Interner,
    call: NodeId,
) -> Option<AdmittedLibraryApiCall<LibraryPromiseFinallyContract>> {
    admitted_receiver_method_contract_call(il, interner, call, library_promise_finally_contract)
}

pub fn admitted_promise_resolve_at_call(
    il: &Il,
    interner: &Interner,
    call: NodeId,
) -> Option<AdmittedLibraryApiCall<LibraryPromiseFactoryContract>> {
    admitted_named_receiver_method_call(il, interner, call, |receiver_name, method, arg_count| {
        library_promise_resolve_contract(il.meta.lang, receiver_name, method, arg_count)
    })
}

pub fn admitted_promise_aggregate_at_call(
    il: &Il,
    interner: &Interner,
    call: NodeId,
) -> Option<AdmittedLibraryApiCall<LibraryPromiseAggregateContract>> {
    admitted_named_receiver_method_call(il, interner, call, |receiver_name, method, arg_count| {
        library_promise_aggregate_contract(il.meta.lang, receiver_name, method, arg_count)
    })
}
