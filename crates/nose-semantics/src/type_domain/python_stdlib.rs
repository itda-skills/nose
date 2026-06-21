use nose_il::DomainEvidence;

pub const PYTHON_STDLIB_TYPE_DOMAIN_PACK_ID: &str = "nose.python.stdlib.type_domain";
pub const PYTHON_STDLIB_TYPE_DOMAIN_PRODUCER_ID: &str = "python.stdlib.type-domain-alias-domain";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct BuiltinTypeDomainAliasContract {
    pub pack_id: &'static str,
    pub producer_id: &'static str,
    pub contract_id: &'static str,
    pub module: &'static str,
    pub exported: &'static str,
    pub domain: DomainEvidence,
    pub positive_fixture: &'static str,
    pub hard_negative_fixture: &'static str,
}

pub type FirstPartyTypeDomainAliasContract = BuiltinTypeDomainAliasContract;

pub const PYTHON_STDLIB_TYPE_DOMAIN_ALIAS_CONTRACTS: &[BuiltinTypeDomainAliasContract] = &[
    python_stdlib_type_domain_alias_contract(
        "typing",
        "Dict",
        DomainEvidence::Map,
        "python-typing-dict-domain-positive",
        "python-typing-alias-rebound-hard-negative",
    ),
    python_stdlib_type_domain_alias_contract(
        "typing",
        "Mapping",
        DomainEvidence::Map,
        "python-typing-mapping-domain-positive",
        "python-typing-alias-rebound-hard-negative",
    ),
    python_stdlib_type_domain_alias_contract(
        "typing",
        "MutableMapping",
        DomainEvidence::Map,
        "python-typing-mutablemapping-domain-positive",
        "python-typing-alias-rebound-hard-negative",
    ),
    python_stdlib_type_domain_alias_contract(
        "collections.abc",
        "Mapping",
        DomainEvidence::Map,
        "python-collections-abc-mapping-domain-positive",
        "python-typing-alias-rebound-hard-negative",
    ),
    python_stdlib_type_domain_alias_contract(
        "collections.abc",
        "MutableMapping",
        DomainEvidence::Map,
        "python-collections-abc-mutablemapping-domain-positive",
        "python-typing-alias-rebound-hard-negative",
    ),
    python_stdlib_type_domain_alias_contract(
        "typing",
        "Iterable",
        DomainEvidence::Iterable,
        "python-typing-iterable-domain-positive",
        "python-typing-alias-rebound-hard-negative",
    ),
    python_stdlib_type_domain_alias_contract(
        "typing",
        "AsyncIterable",
        DomainEvidence::Iterable,
        "python-typing-asynciterable-domain-positive",
        "python-typing-alias-rebound-hard-negative",
    ),
    python_stdlib_type_domain_alias_contract(
        "collections.abc",
        "Iterable",
        DomainEvidence::Iterable,
        "python-collections-abc-iterable-domain-positive",
        "python-typing-alias-rebound-hard-negative",
    ),
    python_stdlib_type_domain_alias_contract(
        "collections.abc",
        "AsyncIterable",
        DomainEvidence::Iterable,
        "python-collections-abc-asynciterable-domain-positive",
        "python-typing-alias-rebound-hard-negative",
    ),
    python_stdlib_type_domain_alias_contract(
        "typing",
        "Iterator",
        DomainEvidence::Iterator,
        "python-typing-iterator-domain-positive",
        "python-typing-alias-rebound-hard-negative",
    ),
    python_stdlib_type_domain_alias_contract(
        "typing",
        "AsyncIterator",
        DomainEvidence::Iterator,
        "python-typing-asynciterator-domain-positive",
        "python-typing-alias-rebound-hard-negative",
    ),
    python_stdlib_type_domain_alias_contract(
        "collections.abc",
        "Iterator",
        DomainEvidence::Iterator,
        "python-collections-abc-iterator-domain-positive",
        "python-typing-alias-rebound-hard-negative",
    ),
    python_stdlib_type_domain_alias_contract(
        "collections.abc",
        "AsyncIterator",
        DomainEvidence::Iterator,
        "python-collections-abc-asynciterator-domain-positive",
        "python-typing-alias-rebound-hard-negative",
    ),
    python_stdlib_type_domain_alias_contract(
        "typing",
        "FrozenSet",
        DomainEvidence::Set,
        "python-typing-frozenset-domain-positive",
        "python-typing-alias-rebound-hard-negative",
    ),
    python_stdlib_type_domain_alias_contract(
        "typing",
        "MutableSet",
        DomainEvidence::Set,
        "python-typing-mutableset-domain-positive",
        "python-typing-alias-rebound-hard-negative",
    ),
    python_stdlib_type_domain_alias_contract(
        "typing",
        "Set",
        DomainEvidence::Set,
        "python-typing-set-domain-positive",
        "python-typing-alias-rebound-hard-negative",
    ),
    python_stdlib_type_domain_alias_contract(
        "collections.abc",
        "MutableSet",
        DomainEvidence::Set,
        "python-collections-abc-mutableset-domain-positive",
        "python-typing-alias-rebound-hard-negative",
    ),
    python_stdlib_type_domain_alias_contract(
        "collections.abc",
        "Set",
        DomainEvidence::Set,
        "python-collections-abc-set-domain-positive",
        "python-typing-alias-rebound-hard-negative",
    ),
    python_stdlib_type_domain_alias_contract(
        "typing",
        "Optional",
        DomainEvidence::Option,
        "python-typing-optional-domain-positive",
        "python-typing-alias-rebound-hard-negative",
    ),
    python_stdlib_type_domain_alias_contract(
        "typing",
        "TypedDict",
        DomainEvidence::Record,
        "python-typing-typeddict-domain-positive",
        "python-typing-alias-rebound-hard-negative",
    ),
    python_stdlib_type_domain_alias_contract(
        "typing",
        "Awaitable",
        DomainEvidence::FutureLike,
        "python-typing-awaitable-domain-positive",
        "python-typing-alias-rebound-hard-negative",
    ),
    python_stdlib_type_domain_alias_contract(
        "typing",
        "Coroutine",
        DomainEvidence::FutureLike,
        "python-typing-coroutine-domain-positive",
        "python-typing-alias-rebound-hard-negative",
    ),
    python_stdlib_type_domain_alias_contract(
        "collections.abc",
        "Awaitable",
        DomainEvidence::FutureLike,
        "python-collections-abc-awaitable-domain-positive",
        "python-typing-alias-rebound-hard-negative",
    ),
    python_stdlib_type_domain_alias_contract(
        "collections.abc",
        "Coroutine",
        DomainEvidence::FutureLike,
        "python-collections-abc-coroutine-domain-positive",
        "python-typing-alias-rebound-hard-negative",
    ),
    python_stdlib_type_domain_alias_contract(
        "asyncio",
        "Future",
        DomainEvidence::FutureLike,
        "python-asyncio-future-domain-positive",
        "python-typing-alias-rebound-hard-negative",
    ),
    python_stdlib_type_domain_alias_contract(
        "typing",
        "Collection",
        DomainEvidence::Collection,
        "python-typing-collection-domain-positive",
        "python-typing-alias-rebound-hard-negative",
    ),
    python_stdlib_type_domain_alias_contract(
        "typing",
        "Container",
        DomainEvidence::Collection,
        "python-typing-container-domain-positive",
        "python-typing-alias-rebound-hard-negative",
    ),
    python_stdlib_type_domain_alias_contract(
        "typing",
        "Deque",
        DomainEvidence::Collection,
        "python-typing-deque-domain-positive",
        "python-typing-alias-rebound-hard-negative",
    ),
    python_stdlib_type_domain_alias_contract(
        "typing",
        "List",
        DomainEvidence::Collection,
        "python-typing-list-domain-positive",
        "python-typing-alias-rebound-hard-negative",
    ),
    python_stdlib_type_domain_alias_contract(
        "typing",
        "MutableSequence",
        DomainEvidence::Collection,
        "python-typing-mutablesequence-domain-positive",
        "python-typing-alias-rebound-hard-negative",
    ),
    python_stdlib_type_domain_alias_contract(
        "typing",
        "Sequence",
        DomainEvidence::Collection,
        "python-typing-sequence-domain-positive",
        "python-typing-alias-rebound-hard-negative",
    ),
    python_stdlib_type_domain_alias_contract(
        "typing",
        "Tuple",
        DomainEvidence::Collection,
        "python-typing-tuple-domain-positive",
        "python-typing-alias-rebound-hard-negative",
    ),
    python_stdlib_type_domain_alias_contract(
        "collections.abc",
        "Collection",
        DomainEvidence::Collection,
        "python-collections-abc-collection-domain-positive",
        "python-typing-alias-rebound-hard-negative",
    ),
    python_stdlib_type_domain_alias_contract(
        "collections.abc",
        "Container",
        DomainEvidence::Collection,
        "python-collections-abc-container-domain-positive",
        "python-typing-alias-rebound-hard-negative",
    ),
    python_stdlib_type_domain_alias_contract(
        "collections.abc",
        "MutableSequence",
        DomainEvidence::Collection,
        "python-collections-abc-mutablesequence-domain-positive",
        "python-typing-alias-rebound-hard-negative",
    ),
    python_stdlib_type_domain_alias_contract(
        "collections.abc",
        "Sequence",
        DomainEvidence::Collection,
        "python-collections-abc-sequence-domain-positive",
        "python-typing-alias-rebound-hard-negative",
    ),
];

const fn python_stdlib_type_domain_alias_contract(
    module: &'static str,
    exported: &'static str,
    domain: DomainEvidence,
    positive_fixture: &'static str,
    hard_negative_fixture: &'static str,
) -> BuiltinTypeDomainAliasContract {
    BuiltinTypeDomainAliasContract {
        pack_id: PYTHON_STDLIB_TYPE_DOMAIN_PACK_ID,
        producer_id: PYTHON_STDLIB_TYPE_DOMAIN_PRODUCER_ID,
        contract_id: "python.stdlib.type-domain-alias.contract",
        module,
        exported,
        domain,
        positive_fixture,
        hard_negative_fixture,
    }
}

pub fn python_stdlib_type_domain_contract(
    module: &str,
    exported: &str,
) -> Option<&'static BuiltinTypeDomainAliasContract> {
    let module = module.trim();
    let exported = exported.trim();
    PYTHON_STDLIB_TYPE_DOMAIN_ALIAS_CONTRACTS
        .iter()
        .find(|row| row.module == module && row.exported == exported)
}

pub fn python_stdlib_type_domain(module: &str, exported: &str) -> Option<DomainEvidence> {
    python_stdlib_type_domain_contract(module, exported).map(|row| row.domain)
}
