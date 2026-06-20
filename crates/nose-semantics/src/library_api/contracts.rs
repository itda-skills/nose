//! Library API contract identities, callee coordinates, and result wrappers.
//!
//! This module names the semantic rows. Occurrence evidence admission and
//! dependency validation stay in the parent module.

use super::contract_keys::{library_api_callee_contract_key, library_api_contract_id_key};
use crate::{
    AsyncReceiverContract, FreeFunctionBuiltinContract, ImportedNamespaceFunctionContract,
    ImportedNamespaceFunctionSemantic, IteratorAdapterReceiverContract,
    IteratorIdentityAdapterContract, JavaCollectionConstructorKind, JavaCollectionFactoryKind,
    JavaMapFactoryKind, MapGetContract, MapKeyViewContract, MapKeyViewKind,
    MapKeyViewWrapperContract, MethodCallContract, MethodReceiverContract, MethodSemanticContract,
    PromiseFactoryContract, PromiseFactoryKind, PromiseThenContract, RegexTestContract,
    ScalarIntegerMethod, ScalarIntegerMethodContract, StaticCollectionAdapterContract,
    StaticGlobalFunctionContract, StaticGlobalMethodContract, StaticIndexMembershipContract,
    StaticIndexMembershipKind, StaticIndexMembershipReceiverContract,
};
use nose_il::{stable_symbol_hash, Builtin, DomainEvidence, Lang, SourceFactKind, Span};

pub const PYTHON_BUILTIN_COLLECTION_FACTORY_PACK_ID: &str =
    "nose.python.builtins.collection_factories";
pub const PYTHON_BUILTIN_COLLECTION_FACTORY_PRODUCER_ID: &str =
    "python.builtins.collection-factory-api";
pub const PYTHON_BUILTIN_COLLECTION_FACTORY_CONTRACT_ID: &str = "python.builtin.collection_factory";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LibraryApiContractId {
    PropertyBuiltin(Builtin),
    PythonBuiltinCollectionFactory,
    PythonImportedCollectionFactory,
    FreeFunctionBuiltin(Builtin),
    RustOptionSomeConstructor,
    RustOptionNoneSentinel,
    RustOptionAndThen,
    ScalarIntegerMethod(ScalarIntegerMethod),
    RustStdCollectionFactory,
    RustStdMapFactory,
    RustVecMacroFactory,
    RustVecNewFactory,
    JavaCollectionFactory(JavaCollectionFactoryKind),
    JavaCollectionConstructor(JavaCollectionConstructorKind),
    JavaMapFactory(JavaMapFactoryKind),
    JavaMapEntryFactory,
    RubySetFactory,
    JsLikeSetConstructor,
    JsLikeMapConstructor,
    MapKeyView(MapKeyViewKind),
    MapKeyViewWrapper,
    MapGet,
    JsArrayIsArray,
    JsBooleanCoercion,
    RegexTest,
    JsLikeStaticIndexMembership(StaticIndexMembershipKind),
    ImportedNamespaceFunction(ImportedNamespaceFunctionSemantic),
    PromiseFactory(PromiseFactoryKind),
    PromiseThen,
    IteratorIdentityAdapter,
    StaticCollectionAdapter,
    MethodCall(MethodSemanticContract),
}

pub fn library_api_contract_id_hash(id: LibraryApiContractId) -> u64 {
    stable_symbol_hash(&library_api_contract_id_key(id))
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LibraryApiShadowPolicy {
    None,
    SameName,
    RustStdRootForStdPath,
    ExplicitRoot(&'static str),
}

pub fn library_api_free_name_shadow_safe(
    lang: Lang,
    name: &str,
    policy: LibraryApiShadowPolicy,
    defines_name: impl Fn(&str) -> bool,
) -> bool {
    match policy {
        LibraryApiShadowPolicy::None => true,
        LibraryApiShadowPolicy::SameName => !defines_name(name),
        LibraryApiShadowPolicy::RustStdRootForStdPath => {
            !(lang == Lang::Rust && name.starts_with("std::") && defines_name("std"))
        }
        LibraryApiShadowPolicy::ExplicitRoot(root) => !defines_name(root),
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LibraryApiCalleeContract {
    FreeName {
        name: &'static str,
        shadow: LibraryApiShadowPolicy,
    },
    RustMacro {
        name: &'static str,
        shadow: LibraryApiShadowPolicy,
    },
    ImportedBinding {
        module: &'static str,
        exported: &'static str,
    },
    JavaUtilStaticMember {
        receiver: &'static str,
        method: &'static str,
    },
    JavaUtilConstructor {
        simple_type: &'static str,
        qualified_type: &'static str,
        module: &'static str,
        requires_import_for_simple_type: bool,
        requires_no_local_type_shadow: bool,
    },
    RubyRequireStaticMember {
        receiver: &'static str,
        method: &'static str,
        required_module: &'static str,
        shadow_root: &'static str,
    },
    JsGlobalConstructor {
        receiver: &'static str,
        requires_unshadowed_global: bool,
    },
    Method {
        method: &'static str,
        receiver: MethodReceiverContract,
    },
    StaticGlobalMethod {
        receiver: &'static str,
        method: &'static str,
        qualified_path: &'static str,
        requires_unshadowed_receiver: bool,
    },
    StaticGlobalFunction {
        function: &'static str,
        requires_unshadowed_function: bool,
    },
    RegexLiteralMethod {
        method: &'static str,
        required_receiver_fact: SourceFactKind,
    },
    Property {
        property: &'static str,
        receiver: MethodReceiverContract,
    },
    StaticIndexMembershipMethod {
        method: &'static str,
        receiver: StaticIndexMembershipReceiverContract,
    },
    ImportedNamespaceFunction {
        module: &'static str,
        function: &'static str,
    },
    AsyncMethod {
        method: &'static str,
        receiver: AsyncReceiverContract,
    },
    IteratorAdapterMethod {
        method: &'static str,
        receiver: IteratorAdapterReceiverContract,
    },
}

pub fn library_api_callee_contract_hash(callee: LibraryApiCalleeContract) -> u64 {
    stable_symbol_hash(&library_api_callee_contract_key(callee))
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LibraryCollectionFactoryResult {
    SequenceArgument,
    VariadicElements { single_arg_spreads_array: bool },
    StaticNonFloatSequenceArgument,
    EmptySequence,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LibraryCollectionFactoryContract {
    pub pack_id: &'static str,
    pub id: LibraryApiContractId,
    pub callee: LibraryApiCalleeContract,
    pub result: LibraryCollectionFactoryResult,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LibraryMapFactoryResult {
    EntrySequence { entry_seq_tag: u64 },
    JavaFactory { kind: JavaMapFactoryKind },
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LibraryMapFactoryContract {
    pub id: LibraryApiContractId,
    pub callee: LibraryApiCalleeContract,
    pub result: LibraryMapFactoryResult,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LibraryMapEntryFactoryContract {
    pub id: LibraryApiContractId,
    pub callee: LibraryApiCalleeContract,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LibraryMapKeyViewContract {
    pub id: LibraryApiContractId,
    pub callee: LibraryApiCalleeContract,
    pub result: MapKeyViewContract,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LibraryMapKeyViewWrapperContract {
    pub id: LibraryApiContractId,
    pub callee: LibraryApiCalleeContract,
    pub result: MapKeyViewWrapperContract,
}

pub fn library_collection_factory_result_domain(
    contract: LibraryCollectionFactoryContract,
) -> DomainEvidence {
    match contract.id {
        LibraryApiContractId::PythonBuiltinCollectionFactory => match contract.callee {
            LibraryApiCalleeContract::FreeName {
                name: "set" | "frozenset",
                ..
            } => DomainEvidence::Set,
            _ => DomainEvidence::Collection,
        },
        LibraryApiContractId::RustStdCollectionFactory => match contract.callee {
            LibraryApiCalleeContract::FreeName {
                name: "std::collections::HashSet::from" | "std::collections::BTreeSet::from",
                ..
            } => DomainEvidence::Set,
            _ => DomainEvidence::Collection,
        },
        LibraryApiContractId::JavaCollectionFactory(JavaCollectionFactoryKind::SetOf)
        | LibraryApiContractId::RubySetFactory
        | LibraryApiContractId::JsLikeSetConstructor => DomainEvidence::Set,
        _ => DomainEvidence::Collection,
    }
}

pub fn library_collection_factory_result_domain_for_arity(
    contract: LibraryCollectionFactoryContract,
    arg_count: usize,
) -> Option<DomainEvidence> {
    match contract.id {
        LibraryApiContractId::JavaCollectionFactory(JavaCollectionFactoryKind::ArraysAsList)
            if arg_count == 1 =>
        {
            None
        }
        _ => Some(library_collection_factory_result_domain(contract)),
    }
}

pub fn library_map_factory_result_domain(_contract: LibraryMapFactoryContract) -> DomainEvidence {
    DomainEvidence::Map
}

pub fn library_map_key_view_wrapper_result_domain(
    _contract: LibraryMapKeyViewWrapperContract,
) -> DomainEvidence {
    DomainEvidence::Array
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LibraryMapGetContract {
    pub id: LibraryApiContractId,
    pub callee: LibraryApiCalleeContract,
    pub result: MapGetContract,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LibraryStaticGlobalMethodContract {
    pub id: LibraryApiContractId,
    pub callee: LibraryApiCalleeContract,
    pub result: StaticGlobalMethodContract,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LibraryStaticGlobalFunctionContract {
    pub id: LibraryApiContractId,
    pub callee: LibraryApiCalleeContract,
    pub result: StaticGlobalFunctionContract,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LibraryRegexTestContract {
    pub id: LibraryApiContractId,
    pub callee: LibraryApiCalleeContract,
    pub result: RegexTestContract,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LibraryStaticIndexMembershipContract {
    pub id: LibraryApiContractId,
    pub callee: LibraryApiCalleeContract,
    pub result: StaticIndexMembershipContract,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LibraryImportedNamespaceFunctionContract {
    pub id: LibraryApiContractId,
    pub callee: LibraryApiCalleeContract,
    pub result: ImportedNamespaceFunctionContract,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LibraryPromiseFactoryContract {
    pub id: LibraryApiContractId,
    pub callee: LibraryApiCalleeContract,
    pub result: PromiseFactoryContract,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LibraryPromiseThenContract {
    pub id: LibraryApiContractId,
    pub callee: LibraryApiCalleeContract,
    pub result: PromiseThenContract,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LibraryIteratorIdentityAdapterContract {
    pub id: LibraryApiContractId,
    pub callee: LibraryApiCalleeContract,
    pub result: IteratorIdentityAdapterContract,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LibraryStaticCollectionAdapterContract {
    pub id: LibraryApiContractId,
    pub callee: LibraryApiCalleeContract,
    pub result: StaticCollectionAdapterContract,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LibraryMethodCallContract {
    pub id: LibraryApiContractId,
    pub callee: LibraryApiCalleeContract,
    pub result: MethodCallContract,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LibraryPropertyBuiltinContract {
    pub id: LibraryApiContractId,
    pub callee: LibraryApiCalleeContract,
    pub result: Builtin,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LibraryScalarIntegerMethodContract {
    pub id: LibraryApiContractId,
    pub callee: LibraryApiCalleeContract,
    pub result: ScalarIntegerMethodContract,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LibraryRustOptionConstructorContract {
    pub id: LibraryApiContractId,
    pub callee: LibraryApiCalleeContract,
    pub result_domain: DomainEvidence,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LibraryRustOptionSentinelContract {
    pub id: LibraryApiContractId,
    pub callee: LibraryApiCalleeContract,
    pub result_domain: DomainEvidence,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct RustOptionAndThenContract {
    pub receiver: MethodReceiverContract,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LibraryRustOptionAndThenContract {
    pub id: LibraryApiContractId,
    pub callee: LibraryApiCalleeContract,
    pub result: RustOptionAndThenContract,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LibraryFreeFunctionBuiltinContract {
    pub id: LibraryApiContractId,
    pub callee: LibraryApiCalleeContract,
    pub result: FreeFunctionBuiltinContract,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LibraryReceiverMethodApiContract {
    pub id: LibraryApiContractId,
    pub callee: LibraryApiCalleeContract,
    pub rule: &'static str,
    pub result_domain: Option<DomainEvidence>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LibraryApiEvidenceStatus {
    Missing,
    Admitted,
    Rejected,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LibraryApiSpanEvidenceQuery {
    pub call_span: Option<Span>,
    pub callee_span: Option<Span>,
    pub receiver_span: Option<Span>,
    pub id: LibraryApiContractId,
    pub callee: LibraryApiCalleeContract,
    pub arg_count: usize,
}
