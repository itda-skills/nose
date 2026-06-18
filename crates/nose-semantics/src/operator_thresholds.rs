//! Shared cardinality and index threshold helpers.

use super::*;

pub(super) fn threshold_excludes_floor(op: Op, value_on_right: bool) -> bool {
    op == Op::Ne || (!value_on_right && op == Op::Gt) || (value_on_right && op == Op::Lt)
}

pub(super) fn threshold_reaches_floor(op: Op, value_on_right: bool) -> bool {
    (!value_on_right && op == Op::Ge) || (value_on_right && op == Op::Le)
}

pub(super) fn threshold_at_or_below_floor(op: Op, value_on_right: bool) -> bool {
    op == Op::Eq || (!value_on_right && op == Op::Le) || (value_on_right && op == Op::Ge)
}

pub(super) fn threshold_below_floor(op: Op, value_on_right: bool) -> bool {
    (!value_on_right && op == Op::Lt) || (value_on_right && op == Op::Gt)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum IndexMembershipThreshold {
    MinusOne,
    Zero,
}

pub(super) fn index_membership_threshold_matches(
    op: Op,
    index_call_on_right: bool,
    threshold: IndexMembershipThreshold,
) -> bool {
    match threshold {
        IndexMembershipThreshold::MinusOne => threshold_excludes_floor(op, index_call_on_right),
        IndexMembershipThreshold::Zero => threshold_reaches_floor(op, index_call_on_right),
    }
}

pub fn index_membership_threshold_contract(
    op: Op,
    index_call_on_right: bool,
    threshold: IndexMembershipThreshold,
) -> bool {
    index_membership_threshold_matches(op, index_call_on_right, threshold)
}
