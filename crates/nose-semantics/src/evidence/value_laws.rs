use super::*;

/// Coarse value domain used by proof-gated value-graph and recursion laws.
///
/// This is deliberately not a general type system. It records only the semantic
/// domains that current first-party laws need in order to avoid known false
/// merges, such as numeric arithmetic versus string/list concatenation and
/// boolean logic versus short-circuit value selection. Unknown is fail-closed
/// for laws that require a positive proof.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ValueDomain {
    Number,
    Boolean,
    String,
    Sequence,
    Unknown,
}

impl ValueDomain {
    pub fn join(self, other: ValueDomain) -> ValueDomain {
        if self == other {
            self
        } else {
            ValueDomain::Unknown
        }
    }

    pub fn is_concat_like(self) -> bool {
        matches!(self, ValueDomain::String | ValueDomain::Sequence)
    }

    pub(crate) fn is_known(self) -> bool {
        self != ValueDomain::Unknown
    }

    pub fn from_domain_evidence(domain: DomainEvidence) -> Option<ValueDomain> {
        if domain.is_integer_or_number() {
            Some(ValueDomain::Number)
        } else if domain.is_boolean() {
            Some(ValueDomain::Boolean)
        } else if domain.is_string() {
            Some(ValueDomain::String)
        } else if domain.is_array_collection_or_set() {
            Some(ValueDomain::Sequence)
        } else {
            None
        }
    }
}

#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, serde::Serialize, serde::Deserialize,
)]
pub enum ValueLaw {
    AddCommutativity,
    AddAssociativity,
    NumericNegationInvolution,
    NumericBitwiseIdempotence,
    BooleanIdempotence,
    BooleanCommutativity,
    BooleanAssociativity,
    NumericFactorDistribution,
    StructuralNumericFold,
    IntegerClampOrderedMinMax,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ValueDomainRequirement {
    NumericOperands,
    BooleanOperands,
    NoConcatOperands,
}

impl ValueDomainRequirement {
    pub fn accepts(self, domains: impl IntoIterator<Item = ValueDomain>) -> bool {
        match self {
            ValueDomainRequirement::NumericOperands => domains
                .into_iter()
                .all(|domain| domain == ValueDomain::Number),
            ValueDomainRequirement::BooleanOperands => domains
                .into_iter()
                .all(|domain| domain == ValueDomain::Boolean),
            ValueDomainRequirement::NoConcatOperands => {
                domains.into_iter().all(|domain| !domain.is_concat_like())
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ValueDomainEvidence {
    Literal,
    SequenceSurface,
    DomainRecord,
    StrictOperatorUse,
    ModeledOperatorResult,
    ModeledBuiltinResult,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ValueLawContract {
    pub law: ValueLaw,
    pub requirement: ValueDomainRequirement,
    pub channel: ChannelEligibility,
    pub evidence: ValueDomainEvidence,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ValueLawProofStatus {
    Proven,
    Covered,
    Missing,
    EmpiricalOnly,
    RejectedCounterexample,
}

impl ValueLawProofStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            ValueLawProofStatus::Proven => "proven",
            ValueLawProofStatus::Covered => "covered",
            ValueLawProofStatus::Missing => "missing",
            ValueLawProofStatus::EmpiricalOnly => "empirical-only",
            ValueLawProofStatus::RejectedCounterexample => "rejected-counterexample",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ValueLawRegistryEntry {
    pub law: ValueLaw,
    pub pack_id: &'static str,
    pub law_id: &'static str,
    pub channel: ChannelEligibility,
    pub proof_status: ValueLawProofStatus,
    pub proof_obligation_id: &'static str,
    pub requirements: &'static [&'static str],
    pub conformance_refs: &'static [&'static str],
}

#[derive(
    Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct ValueLawProvenance {
    pub pack_id: String,
    pub pack_hash: String,
    pub law_id: String,
    pub channel: String,
    pub proof_status: String,
    pub proof_obligation_id: String,
}

const FACTOR_DISTRIBUTE_REQUIREMENTS: &[&str] = &[
    "ValueDomain.Number for distributed operands and shared factor",
    "numeric domain comes from a strict numeric operator surface or explicit domain evidence",
];
const FACTOR_DISTRIBUTE_CONFORMANCE: &[&str] = &[
    "factor-distribute-numeric-positive",
    "factor-distribute-string-repetition-hard-negative",
];

const CLAMP_REQUIREMENTS: &[&str] = &[
    "ValueDomain.Integer-compatible operands",
    "lo <= hi bound-order proof",
    "unique min/max or clamp-surface candidate",
];
const CLAMP_CONFORMANCE: &[&str] = &[
    "clamp-ordered-minmax-positive",
    "clamp-unproven-bound-hard-negative",
    "clamp-swapped-bound-hard-negative",
    "clamp-float-hard-negative",
];

static PACK_FACING_VALUE_LAWS: &[ValueLawRegistryEntry] = &[
    ValueLawRegistryEntry {
        law: ValueLaw::NumericFactorDistribution,
        pack_id: VALUE_GRAPH_LAW_PACK_ID,
        law_id: "value-graph.factor-distribute.numeric-common-factor",
        channel: ChannelEligibility::ExactProven,
        proof_status: ValueLawProofStatus::Proven,
        proof_obligation_id: "normalize.value_graph.factor_distribute",
        requirements: FACTOR_DISTRIBUTE_REQUIREMENTS,
        conformance_refs: FACTOR_DISTRIBUTE_CONFORMANCE,
    },
    ValueLawRegistryEntry {
        law: ValueLaw::IntegerClampOrderedMinMax,
        pack_id: VALUE_GRAPH_LAW_PACK_ID,
        law_id: "value-graph.clamp.integer-ordered-minmax",
        channel: ChannelEligibility::ExactProven,
        proof_status: ValueLawProofStatus::Proven,
        proof_obligation_id: "normalize.value_graph.clamp",
        requirements: CLAMP_REQUIREMENTS,
        conformance_refs: CLAMP_CONFORMANCE,
    },
];

pub fn pack_facing_value_laws() -> &'static [ValueLawRegistryEntry] {
    PACK_FACING_VALUE_LAWS
}

pub fn pack_facing_value_law(law: ValueLaw) -> Option<&'static ValueLawRegistryEntry> {
    PACK_FACING_VALUE_LAWS.iter().find(|entry| entry.law == law)
}

impl ValueLawRegistryEntry {
    pub fn provenance(self) -> ValueLawProvenance {
        ValueLawProvenance {
            pack_id: self.pack_id.to_string(),
            pack_hash: format!("{:016x}", semantic_pack_hash(self.pack_id)),
            law_id: self.law_id.to_string(),
            channel: self.channel.as_str().to_string(),
            proof_status: self.proof_status.as_str().to_string(),
            proof_obligation_id: self.proof_obligation_id.to_string(),
        }
    }
}

pub fn value_law_provenance(law: ValueLaw) -> Option<ValueLawProvenance> {
    pack_facing_value_law(law).map(|entry| entry.provenance())
}

pub(crate) fn strict_numeric_operand_operator(op: Op) -> bool {
    matches!(
        op,
        Op::Sub
            | Op::Mul
            | Op::Div
            | Op::FloorDiv
            | Op::Mod
            | Op::Pow
            | Op::BitAnd
            | Op::BitOr
            | Op::BitXor
            | Op::Shl
            | Op::Shr
    )
}
