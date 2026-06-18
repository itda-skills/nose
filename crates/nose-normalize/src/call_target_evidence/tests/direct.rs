use super::support::*;

#[test]
fn emits_direct_function_call_target_for_unique_unshadowed_function() {
    let interner = Interner::new();
    let (mut il, func, call) = function_with_call(&interner, "f", "f", false);
    run(&mut il, &interner);
    assert!(direct_function_call_target_at_call(&il, call, func));
}

#[test]
fn does_not_emit_when_local_binder_shadows_function_name() {
    let interner = Interner::new();
    let f = interner.intern("f");
    let mut b = IlBuilder::new(FileId(0));
    let param = b.add(NodeKind::Param, Payload::Name(f), sp(1), &[]);
    let callee = b.add(NodeKind::Var, Payload::Name(f), sp(2), &[]);
    let call = b.add(NodeKind::Call, Payload::None, sp(3), &[callee]);
    let ret = b.add(NodeKind::Return, Payload::None, sp(4), &[call]);
    let body = b.add(NodeKind::Block, Payload::None, sp(5), &[ret]);
    let func = b.add(NodeKind::Func, Payload::None, sp(6), &[param, body]);
    let module = b.add(NodeKind::Module, Payload::None, sp(7), &[func]);
    let mut il = b.finish(
        module,
        FileMeta {
            path: "t".into(),
            lang: Lang::Python,
        },
        vec![Unit {
            root: func,
            kind: UnitKind::Function,
            name: Some(f),
            origin: Default::default(),
        }],
        Vec::new(),
    );

    run(&mut il, &interner);
    assert!(!direct_function_call_target_at_call(&il, call, func));
}

#[test]
fn does_not_emit_for_duplicate_function_names() {
    let interner = Interner::new();
    let (mut il, func, call) = function_with_call(&interner, "f", "f", true);
    run(&mut il, &interner);
    assert!(!direct_function_call_target_at_call(&il, call, func));
}

#[test]
fn does_not_emit_for_method_bare_call() {
    let interner = Interner::new();
    let method_sym = interner.intern("fac");
    let mut b = IlBuilder::new(FileId(0));
    let callee = b.add(NodeKind::Var, Payload::Name(method_sym), sp(20), &[]);
    let call = b.add(NodeKind::Call, Payload::None, sp(21), &[callee]);
    let ret = b.add(NodeKind::Return, Payload::None, sp(22), &[call]);
    let body = b.add(NodeKind::Block, Payload::None, sp(23), &[ret]);
    let method = b.add(NodeKind::Func, Payload::None, sp(24), &[body]);
    let module = b.add(NodeKind::Module, Payload::None, sp(25), &[method]);
    let mut il = b.finish(
        module,
        FileMeta {
            path: "t".into(),
            lang: Lang::Java,
        },
        vec![Unit {
            root: method,
            kind: UnitKind::Method,
            name: Some(method_sym),
            origin: Default::default(),
        }],
        Vec::new(),
    );

    run(&mut il, &interner);
    assert!(!direct_function_call_target_at_call(&il, call, method));
}

#[test]
fn does_not_emit_for_nested_function_not_visible_as_top_level() {
    let interner = Interner::new();
    let f = interner.intern("f");
    let mut b = IlBuilder::new(FileId(0));
    let nested_body = b.add(NodeKind::Block, Payload::None, sp(1), &[]);
    let nested = b.add(NodeKind::Func, Payload::None, sp(2), &[nested_body]);
    let callee = b.add(NodeKind::Var, Payload::Name(f), sp(3), &[]);
    let call = b.add(NodeKind::Call, Payload::None, sp(4), &[callee]);
    let ret = b.add(NodeKind::Return, Payload::None, sp(5), &[call]);
    let outer_body = b.add(NodeKind::Block, Payload::None, sp(6), &[nested, ret]);
    let outer = b.add(NodeKind::Func, Payload::None, sp(7), &[outer_body]);
    let module = b.add(NodeKind::Module, Payload::None, sp(8), &[outer]);
    let mut il = b.finish(
        module,
        FileMeta {
            path: "t".into(),
            lang: Lang::Python,
        },
        vec![Unit {
            root: nested,
            kind: UnitKind::Function,
            name: Some(f),
            origin: Default::default(),
        }],
        Vec::new(),
    );

    run(&mut il, &interner);
    assert!(!direct_function_call_target_at_call(&il, call, nested));
}

#[test]
fn does_not_emit_when_enclosing_scope_binds_function_name() {
    let interner = Interner::new();
    let f = interner.intern("f");
    let g = interner.intern("g");
    let mut b = IlBuilder::new(FileId(0));

    let target_body = b.add(NodeKind::Block, Payload::None, sp(1), &[]);
    let target = b.add(NodeKind::Func, Payload::None, sp(2), &[target_body]);

    let shadow_lhs = b.add(NodeKind::Var, Payload::Name(f), sp(3), &[]);
    let shadow_rhs = b.add(NodeKind::Lit, Payload::LitInt(1), sp(4), &[]);
    let shadow = b.add(
        NodeKind::Assign,
        Payload::None,
        sp(5),
        &[shadow_lhs, shadow_rhs],
    );
    let callee = b.add(NodeKind::Var, Payload::Name(f), sp(6), &[]);
    let call = b.add(NodeKind::Call, Payload::None, sp(7), &[callee]);
    let inner_ret = b.add(NodeKind::Return, Payload::None, sp(8), &[call]);
    let inner_body = b.add(NodeKind::Block, Payload::None, sp(9), &[inner_ret]);
    let inner = b.add(NodeKind::Func, Payload::None, sp(10), &[inner_body]);
    let outer_body = b.add(NodeKind::Block, Payload::None, sp(11), &[shadow, inner]);
    let outer = b.add(NodeKind::Func, Payload::None, sp(12), &[outer_body]);
    let module = b.add(NodeKind::Module, Payload::None, sp(13), &[target, outer]);
    let mut il = b.finish(
        module,
        FileMeta {
            path: "t".into(),
            lang: Lang::Python,
        },
        vec![
            Unit {
                root: target,
                kind: UnitKind::Function,
                name: Some(f),
                origin: Default::default(),
            },
            Unit {
                root: outer,
                kind: UnitKind::Function,
                name: Some(g),
                origin: Default::default(),
            },
        ],
        Vec::new(),
    );

    run(&mut il, &interner);
    assert!(!direct_function_call_target_at_call(&il, call, target));
}
