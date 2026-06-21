use super::*;

fn rust_map_get_call_il() -> (Il, Interner, NodeId, NodeId, NodeId) {
    let interner = Interner::new();
    let mut b = IlBuilder::new(FileId(0));
    let receiver = b.add(NodeKind::Var, Payload::Cid(0), sp(72), &[]);
    let callee = b.add(
        NodeKind::Field,
        Payload::Name(interner.intern("get")),
        sp(73),
        &[receiver],
    );
    let key = b.add(NodeKind::Var, Payload::Cid(1), sp(74), &[]);
    let call = b.add(NodeKind::Call, Payload::None, sp(75), &[callee, key]);
    let root = b.add(NodeKind::Func, Payload::None, sp(76), &[call]);
    (
        finish_il(b, root, Lang::Rust),
        interner,
        call,
        callee,
        receiver,
    )
}

mod core;
mod imported_collections;
