use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum ParamSemantic {
    Array,
    Boolean,
    ByteArray,
    Collection,
    Float,
    FutureLike,
    Integer,
    Iterable,
    Iterator,
    Map,
    Number,
    Option,
    Record,
    Result,
    Set,
    String,
}

/// Kernel-facing receiver/domain evidence recovered from source annotations,
/// inference, or semantic packs. Unknown is represented by absence, and
/// consumers must fail closed when evidence is missing or conflicting.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum DomainEvidence {
    Array,
    Boolean,
    ByteArray,
    Collection,
    Float,
    FutureLike,
    Integer,
    Iterable,
    Iterator,
    Map,
    Nominal { type_hash: u64 },
    Number,
    Option,
    PromiseLike,
    Record,
    Result,
    Set,
    String,
}

impl DomainEvidence {
    pub fn from_param_semantic(semantic: ParamSemantic) -> Self {
        match semantic {
            ParamSemantic::Array => DomainEvidence::Array,
            ParamSemantic::Boolean => DomainEvidence::Boolean,
            ParamSemantic::ByteArray => DomainEvidence::ByteArray,
            ParamSemantic::Collection => DomainEvidence::Collection,
            ParamSemantic::Float => DomainEvidence::Float,
            ParamSemantic::FutureLike => DomainEvidence::FutureLike,
            ParamSemantic::Integer => DomainEvidence::Integer,
            ParamSemantic::Iterable => DomainEvidence::Iterable,
            ParamSemantic::Iterator => DomainEvidence::Iterator,
            ParamSemantic::Map => DomainEvidence::Map,
            ParamSemantic::Number => DomainEvidence::Number,
            ParamSemantic::Option => DomainEvidence::Option,
            ParamSemantic::Record => DomainEvidence::Record,
            ParamSemantic::Result => DomainEvidence::Result,
            ParamSemantic::Set => DomainEvidence::Set,
            ParamSemantic::String => DomainEvidence::String,
        }
    }

    pub fn is_array(self) -> bool {
        self == DomainEvidence::Array
    }

    pub fn is_byte_array(self) -> bool {
        self == DomainEvidence::ByteArray
    }

    pub fn is_boolean(self) -> bool {
        self == DomainEvidence::Boolean
    }

    pub fn is_collection_or_set(self) -> bool {
        matches!(self, DomainEvidence::Collection | DomainEvidence::Set)
    }

    pub fn is_array_or_collection(self) -> bool {
        matches!(self, DomainEvidence::Array | DomainEvidence::Collection)
    }

    pub fn is_array_collection_or_set(self) -> bool {
        matches!(
            self,
            DomainEvidence::Array | DomainEvidence::Collection | DomainEvidence::Set
        )
    }

    pub fn is_set(self) -> bool {
        self == DomainEvidence::Set
    }

    pub fn is_float(self) -> bool {
        self == DomainEvidence::Float
    }

    pub fn is_future_like(self) -> bool {
        matches!(
            self,
            DomainEvidence::FutureLike | DomainEvidence::PromiseLike
        )
    }

    pub fn is_map(self) -> bool {
        self == DomainEvidence::Map
    }

    pub fn is_iterable(self) -> bool {
        self == DomainEvidence::Iterable
    }

    pub fn is_iterator(self) -> bool {
        self == DomainEvidence::Iterator
    }

    pub fn is_iterable_or_iterator(self) -> bool {
        matches!(self, DomainEvidence::Iterable | DomainEvidence::Iterator)
    }

    pub fn is_nominal(self, expected_hash: u64) -> bool {
        matches!(self, DomainEvidence::Nominal { type_hash } if type_hash == expected_hash)
    }

    pub fn is_option(self) -> bool {
        self == DomainEvidence::Option
    }

    pub fn is_promise_like(self) -> bool {
        self == DomainEvidence::PromiseLike
    }

    pub fn is_string(self) -> bool {
        self == DomainEvidence::String
    }

    pub fn is_record(self) -> bool {
        self == DomainEvidence::Record
    }

    pub fn is_result(self) -> bool {
        self == DomainEvidence::Result
    }

    pub fn is_integer(self) -> bool {
        self == DomainEvidence::Integer
    }

    pub fn is_integer_or_number(self) -> bool {
        matches!(
            self,
            DomainEvidence::Integer | DomainEvidence::Float | DomainEvidence::Number
        )
    }
}
