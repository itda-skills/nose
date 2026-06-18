use nose_il::{Op, SourceOperatorKind};

pub(super) fn js_bin_op(text: &str) -> Option<Op> {
    // JS `/` is TRUE (float) division (`7 / 2 == 3.5`) — distinct from the C-family
    // truncated `Op::Div` that `common_bin_op` returns and from floored `Op::FloorDiv`,
    // so it never shares a fingerprint with them (#283-D).
    if text == "/" {
        return Some(Op::TrueDiv);
    }
    // shared C-family set, plus JS's strict-equality, exponent, and type-test
    // operators. `>>>` (zero-fill shift) is deliberately UNMAPPED: collapsing it
    // onto `Shr` merged `-5 >> 1` (sign-extending, `-3`) with `-5 >>> 1`
    // (zero-filling, `2147483645`) — a false merge. The raw fallback keys it by
    // its own operator spelling instead.
    crate::lower::common_bin_op(text).or(match text {
        "===" => Some(Op::Eq),
        "!==" => Some(Op::Ne),
        // `x in obj` is a directional membership/key test — its own non-commutative op.
        // `instanceof` is structurally equality-shaped but carries a source-operator fact;
        // the value graph salts it so prototype-chain type membership never merges with equality.
        "in" => Some(Op::In),
        "instanceof" => Some(Op::Eq),
        _ => None,
    })
}

pub(super) fn js_source_operator(text: &str) -> Option<SourceOperatorKind> {
    match text {
        "===" => Some(SourceOperatorKind::StrictEquality),
        "!==" => Some(SourceOperatorKind::StrictInequality),
        "==" => Some(SourceOperatorKind::LooseEquality),
        "!=" => Some(SourceOperatorKind::LooseInequality),
        "instanceof" => Some(SourceOperatorKind::TypeMembership),
        _ => None,
    }
}
