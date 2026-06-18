use super::*;

pub(super) fn receiver_only_args(
    op: Builtin,
    receiver: ProvenReceiver,
    direct: Option<NodeId>,
) -> CallCanon {
    if let ProvenReceiver::MapGet { map, key } = receiver {
        if op == Builtin::IsNotNull {
            return CallCanon::Builtin {
                op: Builtin::Contains,
                arg_olds: vec![key, map],
            };
        }
        return CallCanon::None;
    }
    let Some(direct) = direct else {
        return CallCanon::None;
    };
    CallCanon::Builtin {
        op,
        arg_olds: vec![direct],
    }
}

pub(super) fn map_get_default_or_zero_arg_lambda_args(
    old: &Il,
    op: Builtin,
    direct: Option<NodeId>,
    args: &[NodeId],
) -> CallCanon {
    let Some(base) = direct else {
        return CallCanon::None;
    };
    let fallback = if old.kind(args[1]) == NodeKind::Lambda {
        let Some(fallback) = zero_arg_lambda_body_value(old, args[1]) else {
            return CallCanon::None;
        };
        fallback
    } else {
        args[1]
    };
    CallCanon::Builtin {
        op,
        arg_olds: vec![base, args[0], fallback],
    }
}

pub(super) fn rust_map_get_or_option_default_args(
    op: Builtin,
    receiver: ProvenReceiver,
    args: &[NodeId],
) -> CallCanon {
    match receiver {
        ProvenReceiver::MapGet { map, key } => CallCanon::Builtin {
            op: Builtin::GetOrDefault,
            arg_olds: vec![map, key, args[0]],
        },
        ProvenReceiver::Direct(base) => CallCanon::Builtin {
            op,
            arg_olds: vec![base, args[0]],
        },
    }
}

pub(super) fn rust_option_default_lambda_args(
    old: &Il,
    op: Builtin,
    direct: Option<NodeId>,
    args: &[NodeId],
) -> CallCanon {
    let Some(base) = direct else {
        return CallCanon::None;
    };
    if let Some(fallback) = zero_arg_lambda_body(old, args[0]) {
        return CallCanon::Builtin {
            op,
            arg_olds: vec![base, fallback],
        };
    }
    CallCanon::None
}

pub(super) fn rust_option_map_or_identity_args(
    old: &Il,
    op: Builtin,
    direct: Option<NodeId>,
    args: &[NodeId],
) -> CallCanon {
    let Some(base) = direct else {
        return CallCanon::None;
    };
    if identity_lambda(old, args[1]) {
        return CallCanon::Builtin {
            op,
            arg_olds: vec![base, args[0]],
        };
    }
    CallCanon::None
}

pub(super) fn rust_zip_args(
    old: &Il,
    interner: &Interner,
    op: Builtin,
    direct: Option<NodeId>,
    args: &[NodeId],
) -> CallCanon {
    let Some(base) = direct else {
        return CallCanon::None;
    };
    CallCanon::Builtin {
        op,
        arg_olds: vec![
            unwrap_iter(old, interner, base),
            unwrap_iter(old, interner, args[0]),
        ],
    }
}

pub(super) fn fold_args(
    old: &Il,
    interner: &Interner,
    op: Builtin,
    direct: Option<NodeId>,
    args: &[NodeId],
) -> CallCanon {
    let Some(base) = direct else {
        return CallCanon::None;
    };
    let (fn_old, init_old) = fold_fn_init(old, args);
    let Some(fn_old) = fn_old else {
        return CallCanon::None;
    };
    let coll = unwrap_iter(old, interner, base);
    let mut a = vec![fn_old, coll];
    if let Some(init) = init_old {
        a.push(init);
    }
    CallCanon::Builtin { op, arg_olds: a }
}

pub(super) fn collection_reduction_args(
    old: &Il,
    interner: &Interner,
    op: Builtin,
    direct: Option<NodeId>,
) -> CallCanon {
    let Some(base) = direct else {
        return CallCanon::None;
    };
    if matches!(op, Builtin::Min | Builtin::Max) && old.kind(base) == NodeKind::Seq {
        let items = old.children(base);
        if items.len() == 2 {
            return CallCanon::Builtin {
                op,
                arg_olds: items.to_vec(),
            };
        }
    }
    CallCanon::Builtin {
        op,
        arg_olds: vec![unwrap_iter(old, interner, base)],
    }
}

/// Classify a fold's args into `(fn, init)`: the lambda is the function (whatever its
/// position — JS puts it first, Ruby/Rust last), the other arg (if any) is the seed.
pub(super) fn fold_fn_init(old: &Il, args: &[NodeId]) -> (Option<NodeId>, Option<NodeId>) {
    let mut fn_old = None;
    let mut init = None;
    for &a in args {
        if old.kind(a) == NodeKind::Lambda {
            fn_old = Some(a);
        } else {
            init = Some(a);
        }
    }
    (fn_old, init)
}

/// Peel a first-party identity iterator adapter to the underlying collection, so
/// `xs.iter().fold(..)` / `xs.stream().map(..)` iterate over `xs`. NOT `.keys()`/`.values()`,
/// which change *what* is iterated.
pub(super) fn unwrap_iter(old: &Il, interner: &Interner, node: NodeId) -> NodeId {
    if old.kind(node) == NodeKind::Call {
        let call_kids = old.children(node);
        if let Some(&callee) = call_kids.first() {
            if old.kind(callee) == NodeKind::Field {
                if let Some(arg) = static_collection_adapter_arg(old, interner, node, call_kids) {
                    return arg;
                }
                if let Some(base) = admitted_iterator_identity_adapter_at_call(old, interner, node)
                    .and_then(|admitted| admitted.receiver)
                {
                    return unwrap_iter(old, interner, base);
                }
            }
        }
    }
    node
}
