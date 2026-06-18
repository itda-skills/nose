use super::support::*;

pub(super) fn map_get_default_call_il(
    lang: Lang,
    method: &str,
    default_lambda: bool,
    lambda_param: bool,
) -> (Il, Interner, NodeId, NodeId) {
    let interner = Interner::new();
    let mut b = IlBuilder::new(FileId(0));
    let receiver = b.add(
        NodeKind::Seq,
        Payload::Name(interner.intern("hash")),
        sp(),
        &[],
    );
    let field = b.add(
        NodeKind::Field,
        Payload::Name(interner.intern(method)),
        sp(),
        &[receiver],
    );
    let key = b.add(NodeKind::Lit, Payload::LitStr(1), sp(), &[]);
    let fallback_value = b.add(NodeKind::Lit, Payload::LitInt(0), sp(), &[]);
    let fallback = if default_lambda {
        if lambda_param {
            let param = b.add(NodeKind::Param, Payload::Cid(0), sp(), &[]);
            b.add(
                NodeKind::Lambda,
                Payload::None,
                sp(),
                &[param, fallback_value],
            )
        } else {
            b.add(NodeKind::Lambda, Payload::None, sp(), &[fallback_value])
        }
    } else {
        fallback_value
    };
    let call = b.add(NodeKind::Call, Payload::None, sp(), &[field, key, fallback]);
    let root = b.add(NodeKind::Module, Payload::None, sp(), &[call]);
    let mut il = b.finish(
        root,
        FileMeta {
            path: "t".to_string(),
            lang,
        },
        Vec::new(),
        Vec::new(),
    );
    push_receiver_sequence_surface_evidence(&mut il, call, SequenceSurfaceKind::Map);
    let _ = push_receiver_method_library_api_evidence(&mut il, &interner, call);
    (il, interner, call, fallback_value)
}
