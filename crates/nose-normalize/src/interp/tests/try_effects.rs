use super::*;

fn run_try(body_stmt: NodeId, handler_stmt: NodeId, mut b: IlBuilder, sp: Span) -> Value {
    let body = b.add(NodeKind::Block, Payload::None, sp, &[body_stmt]);
    let handler = b.add(NodeKind::Block, Payload::None, sp, &[handler_stmt]);
    let try_node = b.add(NodeKind::Try, Payload::None, sp, &[body, handler]);
    let block = b.add(NodeKind::Block, Payload::None, sp, &[try_node]);
    let func = b.add(NodeKind::Func, Payload::None, sp, &[block]);
    let il = b.finish(
        func,
        FileMeta {
            path: "t".into(),
            lang: Lang::Python,
        },
        Vec::new(),
        Vec::new(),
    );
    run_admitted_unit(il, func, &[]).expect("run_unit").ret
}

#[test]
fn try_handler_runs_on_throw_err() {
    let sp = Span::synthetic(FileId(0));
    let mut b = IlBuilder::new(FileId(0));
    let thrown = b.add(NodeKind::Lit, Payload::LitStr(0xBAD), sp, &[]);
    let throw = b.add(NodeKind::Throw, Payload::None, sp, &[thrown]);
    let seven = b.add(NodeKind::Lit, Payload::LitInt(7), sp, &[]);
    let handler_ret = b.add(NodeKind::Return, Payload::None, sp, &[seven]);
    assert_eq!(run_try(throw, handler_ret, b, sp), Value::Int(7));
}

#[test]
fn try_handler_is_skipped_on_normal_return() {
    let sp = Span::synthetic(FileId(0));
    let mut b = IlBuilder::new(FileId(0));
    let one = b.add(NodeKind::Lit, Payload::LitInt(1), sp, &[]);
    let body_ret = b.add(NodeKind::Return, Payload::None, sp, &[one]);
    let seven = b.add(NodeKind::Lit, Payload::LitInt(7), sp, &[]);
    let handler_ret = b.add(NodeKind::Return, Payload::None, sp, &[seven]);
    assert_eq!(run_try(body_ret, handler_ret, b, sp), Value::Int(1));
}

#[test]
fn try_handler_catches_return_expression_err() {
    let sp = Span::synthetic(FileId(0));
    let mut b = IlBuilder::new(FileId(0));
    let one = b.add(NodeKind::Lit, Payload::LitInt(1), sp, &[]);
    let zero = b.add(NodeKind::Lit, Payload::LitInt(0), sp, &[]);
    let div = b.add(NodeKind::BinOp, Payload::Op(Op::Div), sp, &[one, zero]);
    let body_ret = b.add(NodeKind::Return, Payload::None, sp, &[div]);
    let seven = b.add(NodeKind::Lit, Payload::LitInt(7), sp, &[]);
    let handler_ret = b.add(NodeKind::Return, Payload::None, sp, &[seven]);
    assert_eq!(run_try(body_ret, handler_ret, b, sp), Value::Int(7));
}

#[test]
fn try_handler_catches_assignment_expression_err() {
    let sp = Span::synthetic(FileId(0));
    let mut b = IlBuilder::new(FileId(0));
    let var = b.add(NodeKind::Var, Payload::Cid(0), sp, &[]);
    let one = b.add(NodeKind::Lit, Payload::LitInt(1), sp, &[]);
    let zero = b.add(NodeKind::Lit, Payload::LitInt(0), sp, &[]);
    let div = b.add(NodeKind::BinOp, Payload::Op(Op::Div), sp, &[one, zero]);
    let assign = b.add(NodeKind::Assign, Payload::None, sp, &[var, div]);
    let seven = b.add(NodeKind::Lit, Payload::LitInt(7), sp, &[]);
    let handler_ret = b.add(NodeKind::Return, Payload::None, sp, &[seven]);
    assert_eq!(run_try(assign, handler_ret, b, sp), Value::Int(7));
}

fn append_with_error_item_value() -> Value {
    let sp = Span::synthetic(FileId(0));
    let mut b = IlBuilder::new(FileId(0));
    let one = b.add(NodeKind::Lit, Payload::LitInt(1), sp, &[]);
    let list = b.add(NodeKind::Seq, Payload::None, sp, &[one]);
    let zero = b.add(NodeKind::Lit, Payload::LitInt(0), sp, &[]);
    let div = b.add(NodeKind::BinOp, Payload::Op(Op::Div), sp, &[one, zero]);
    let append = b.add(
        NodeKind::Call,
        Payload::Builtin(Builtin::Append),
        sp,
        &[list, div],
    );
    let ret = b.add(NodeKind::Return, Payload::None, sp, &[append]);
    let func = b.add(NodeKind::Func, Payload::None, sp, &[ret]);
    let il = b.finish(
        func,
        FileMeta {
            path: "t".into(),
            lang: Lang::Go,
        },
        Vec::new(),
        Vec::new(),
    );
    run_admitted_unit(il, func, &[]).expect("run_unit").ret
}

#[test]
fn value_append_propagates_error_items() {
    assert_eq!(append_with_error_item_value(), Value::Err);
}

fn statement_append_with_error_item_value() -> Value {
    let sp = Span::synthetic(FileId(0));
    let mut b = IlBuilder::new(FileId(0));
    let var = b.add(NodeKind::Var, Payload::Cid(0), sp, &[]);
    let empty = b.add(NodeKind::Seq, Payload::None, sp, &[]);
    let assign = b.add(NodeKind::Assign, Payload::None, sp, &[var, empty]);
    let one = b.add(NodeKind::Lit, Payload::LitInt(1), sp, &[]);
    let zero = b.add(NodeKind::Lit, Payload::LitInt(0), sp, &[]);
    let div = b.add(NodeKind::BinOp, Payload::Op(Op::Div), sp, &[one, zero]);
    let append = b.add(
        NodeKind::Call,
        Payload::Builtin(Builtin::Append),
        sp,
        &[var, div],
    );
    let append_stmt = b.add(NodeKind::ExprStmt, Payload::None, sp, &[append]);
    let ret = b.add(NodeKind::Return, Payload::None, sp, &[var]);
    let block = b.add(
        NodeKind::Block,
        Payload::None,
        sp,
        &[assign, append_stmt, ret],
    );
    let func = b.add(NodeKind::Func, Payload::None, sp, &[block]);
    let il = b.finish(
        func,
        FileMeta {
            path: "t".into(),
            lang: Lang::Python,
        },
        Vec::new(),
        Vec::new(),
    );
    run_admitted_unit(il, func, &[]).expect("run_unit").ret
}

#[test]
fn statement_append_propagates_error_items() {
    assert_eq!(statement_append_with_error_item_value(), Value::Err);
}

fn statement_append_on_error_target_with_effect_arg() -> Behavior {
    let sp = Span::synthetic(FileId(0));
    let mut b = IlBuilder::new(FileId(0));
    let param = b.add(NodeKind::Param, Payload::Cid(0), sp, &[]);
    let target = b.add(NodeKind::Var, Payload::Cid(0), sp, &[]);
    let one = b.add(NodeKind::Lit, Payload::LitInt(1), sp, &[]);
    let print = b.add(NodeKind::Call, Payload::Builtin(Builtin::Print), sp, &[one]);
    let append = b.add(
        NodeKind::Call,
        Payload::Builtin(Builtin::Append),
        sp,
        &[target, print],
    );
    let append_stmt = b.add(NodeKind::ExprStmt, Payload::None, sp, &[append]);
    let seven = b.add(NodeKind::Lit, Payload::LitInt(7), sp, &[]);
    let ret = b.add(NodeKind::Return, Payload::None, sp, &[seven]);
    let block = b.add(NodeKind::Block, Payload::None, sp, &[append_stmt, ret]);
    let func = b.add(NodeKind::Func, Payload::None, sp, &[param, block]);
    let il = b.finish(
        func,
        FileMeta {
            path: "t".into(),
            lang: Lang::Python,
        },
        Vec::new(),
        Vec::new(),
    );
    run_admitted_unit(il, func, &[Value::Err]).expect("run_unit")
}

#[test]
fn statement_append_checks_error_target_before_items() {
    let behavior = statement_append_on_error_target_with_effect_arg();
    assert_eq!(behavior.ret, Value::Err);
    assert!(behavior.effects.is_empty());
}

fn statement_append_on_error_expr_target_with_effect_arg() -> Behavior {
    let sp = Span::synthetic(FileId(0));
    let mut b = IlBuilder::new(FileId(0));
    let one = b.add(NodeKind::Lit, Payload::LitInt(1), sp, &[]);
    let zero = b.add(NodeKind::Lit, Payload::LitInt(0), sp, &[]);
    let target_err = b.add(NodeKind::BinOp, Payload::Op(Op::Div), sp, &[one, zero]);
    let print = b.add(NodeKind::Call, Payload::Builtin(Builtin::Print), sp, &[one]);
    let append = b.add(
        NodeKind::Call,
        Payload::Builtin(Builtin::Append),
        sp,
        &[target_err, print],
    );
    let append_stmt = b.add(NodeKind::ExprStmt, Payload::None, sp, &[append]);
    let seven = b.add(NodeKind::Lit, Payload::LitInt(7), sp, &[]);
    let ret = b.add(NodeKind::Return, Payload::None, sp, &[seven]);
    let block = b.add(NodeKind::Block, Payload::None, sp, &[append_stmt, ret]);
    let func = b.add(NodeKind::Func, Payload::None, sp, &[block]);
    let il = b.finish(
        func,
        FileMeta {
            path: "t".into(),
            lang: Lang::Python,
        },
        Vec::new(),
        Vec::new(),
    );
    run_admitted_unit(il, func, &[]).expect("run_unit")
}

#[test]
fn statement_append_checks_error_expr_target_before_items() {
    let behavior = statement_append_on_error_expr_target_with_effect_arg();
    assert_eq!(behavior.ret, Value::Err);
    assert!(behavior.effects.is_empty());
}

fn statement_append_on_error_field_receiver_with_effect_arg() -> Behavior {
    let sp = Span::synthetic(FileId(0));
    let mut b = IlBuilder::new(FileId(0));
    let interner = Interner::new();
    let field_name = interner.intern("x");
    let one = b.add(NodeKind::Lit, Payload::LitInt(1), sp, &[]);
    let zero = b.add(NodeKind::Lit, Payload::LitInt(0), sp, &[]);
    let receiver_err = b.add(NodeKind::BinOp, Payload::Op(Op::Div), sp, &[one, zero]);
    let target = b.add(
        NodeKind::Field,
        Payload::Name(field_name),
        sp,
        &[receiver_err],
    );
    let print = b.add(NodeKind::Call, Payload::Builtin(Builtin::Print), sp, &[one]);
    let append = b.add(
        NodeKind::Call,
        Payload::Builtin(Builtin::Append),
        sp,
        &[target, print],
    );
    let append_stmt = b.add(NodeKind::ExprStmt, Payload::None, sp, &[append]);
    let seven = b.add(NodeKind::Lit, Payload::LitInt(7), sp, &[]);
    let ret = b.add(NodeKind::Return, Payload::None, sp, &[seven]);
    let block = b.add(NodeKind::Block, Payload::None, sp, &[append_stmt, ret]);
    let func = b.add(NodeKind::Func, Payload::None, sp, &[block]);
    let il = b.finish(
        func,
        FileMeta {
            path: "t".into(),
            lang: Lang::Python,
        },
        Vec::new(),
        Vec::new(),
    );
    run_admitted_unit(il, func, &[]).expect("run_unit")
}

#[test]
fn statement_append_checks_error_field_receiver_before_items() {
    let behavior = statement_append_on_error_field_receiver_with_effect_arg();
    assert_eq!(behavior.ret, Value::Err);
    assert!(behavior.effects.is_empty());
}

fn index_on_error_base_with_effect_index() -> Behavior {
    let sp = Span::synthetic(FileId(0));
    let mut b = IlBuilder::new(FileId(0));
    let one = b.add(NodeKind::Lit, Payload::LitInt(1), sp, &[]);
    let zero = b.add(NodeKind::Lit, Payload::LitInt(0), sp, &[]);
    let base_err = b.add(NodeKind::BinOp, Payload::Op(Op::Div), sp, &[one, zero]);
    let print = b.add(NodeKind::Call, Payload::Builtin(Builtin::Print), sp, &[one]);
    let index = b.add(NodeKind::Index, Payload::None, sp, &[base_err, print]);
    let index_stmt = b.add(NodeKind::ExprStmt, Payload::None, sp, &[index]);
    let seven = b.add(NodeKind::Lit, Payload::LitInt(7), sp, &[]);
    let ret = b.add(NodeKind::Return, Payload::None, sp, &[seven]);
    let block = b.add(NodeKind::Block, Payload::None, sp, &[index_stmt, ret]);
    let func = b.add(NodeKind::Func, Payload::None, sp, &[block]);
    let il = b.finish(
        func,
        FileMeta {
            path: "t".into(),
            lang: Lang::Python,
        },
        Vec::new(),
        Vec::new(),
    );
    run_admitted_unit(il, func, &[]).expect("run_unit")
}

#[test]
fn index_checks_error_base_before_index_expr() {
    let behavior = index_on_error_base_with_effect_index();
    assert_eq!(behavior.ret, Value::Err);
    assert!(behavior.effects.is_empty());
}

fn binop_on_error_left_with_effect_right() -> Behavior {
    let sp = Span::synthetic(FileId(0));
    let mut b = IlBuilder::new(FileId(0));
    let one = b.add(NodeKind::Lit, Payload::LitInt(1), sp, &[]);
    let zero = b.add(NodeKind::Lit, Payload::LitInt(0), sp, &[]);
    let left_err = b.add(NodeKind::BinOp, Payload::Op(Op::Div), sp, &[one, zero]);
    let print = b.add(NodeKind::Call, Payload::Builtin(Builtin::Print), sp, &[one]);
    let add = b.add(
        NodeKind::BinOp,
        Payload::Op(Op::Add),
        sp,
        &[left_err, print],
    );
    let add_stmt = b.add(NodeKind::ExprStmt, Payload::None, sp, &[add]);
    let seven = b.add(NodeKind::Lit, Payload::LitInt(7), sp, &[]);
    let ret = b.add(NodeKind::Return, Payload::None, sp, &[seven]);
    let block = b.add(NodeKind::Block, Payload::None, sp, &[add_stmt, ret]);
    let func = b.add(NodeKind::Func, Payload::None, sp, &[block]);
    let il = b.finish(
        func,
        FileMeta {
            path: "t".into(),
            lang: Lang::Python,
        },
        Vec::new(),
        Vec::new(),
    );
    run_admitted_unit(il, func, &[]).expect("run_unit")
}

#[test]
fn binop_checks_error_left_before_right_expr() {
    let behavior = binop_on_error_left_with_effect_right();
    assert_eq!(behavior.ret, Value::Err);
    assert!(behavior.effects.is_empty());
}

#[test]
fn try_handler_catches_statement_append_item_err() {
    let sp = Span::synthetic(FileId(0));
    let mut b = IlBuilder::new(FileId(0));
    let var = b.add(NodeKind::Var, Payload::Cid(0), sp, &[]);
    let empty = b.add(NodeKind::Seq, Payload::None, sp, &[]);
    let assign = b.add(NodeKind::Assign, Payload::None, sp, &[var, empty]);
    let one = b.add(NodeKind::Lit, Payload::LitInt(1), sp, &[]);
    let zero = b.add(NodeKind::Lit, Payload::LitInt(0), sp, &[]);
    let div = b.add(NodeKind::BinOp, Payload::Op(Op::Div), sp, &[one, zero]);
    let append = b.add(
        NodeKind::Call,
        Payload::Builtin(Builtin::Append),
        sp,
        &[var, div],
    );
    let append_stmt = b.add(NodeKind::ExprStmt, Payload::None, sp, &[append]);
    let body = b.add(NodeKind::Block, Payload::None, sp, &[assign, append_stmt]);
    let seven = b.add(NodeKind::Lit, Payload::LitInt(7), sp, &[]);
    let handler_ret = b.add(NodeKind::Return, Payload::None, sp, &[seven]);
    let handler = b.add(NodeKind::Block, Payload::None, sp, &[handler_ret]);
    let try_node = b.add(NodeKind::Try, Payload::None, sp, &[body, handler]);
    let block = b.add(NodeKind::Block, Payload::None, sp, &[try_node]);
    let func = b.add(NodeKind::Func, Payload::None, sp, &[block]);
    let il = b.finish(
        func,
        FileMeta {
            path: "t".into(),
            lang: Lang::Python,
        },
        Vec::new(),
        Vec::new(),
    );
    assert_eq!(
        run_admitted_unit(il, func, &[]).expect("run_unit").ret,
        Value::Int(7)
    );
}

#[test]
fn expression_statement_err_stops_later_execution() {
    let sp = Span::synthetic(FileId(0));
    let mut b = IlBuilder::new(FileId(0));
    let one = b.add(NodeKind::Lit, Payload::LitInt(1), sp, &[]);
    let zero = b.add(NodeKind::Lit, Payload::LitInt(0), sp, &[]);
    let div = b.add(NodeKind::BinOp, Payload::Op(Op::Div), sp, &[one, zero]);
    let expr_stmt = b.add(NodeKind::ExprStmt, Payload::None, sp, &[div]);
    let later = b.add(NodeKind::Lit, Payload::LitInt(9), sp, &[]);
    let ret = b.add(NodeKind::Return, Payload::None, sp, &[later]);
    let block = b.add(NodeKind::Block, Payload::None, sp, &[expr_stmt, ret]);
    let func = b.add(NodeKind::Func, Payload::None, sp, &[block]);
    let il = b.finish(
        func,
        FileMeta {
            path: "t".into(),
            lang: Lang::Python,
        },
        Vec::new(),
        Vec::new(),
    );
    assert_eq!(
        run_admitted_unit(il, func, &[]).expect("run_unit").ret,
        Value::Err
    );
}

fn seq_with_error_item_value() -> Value {
    let sp = Span::synthetic(FileId(0));
    let mut b = IlBuilder::new(FileId(0));
    let one = b.add(NodeKind::Lit, Payload::LitInt(1), sp, &[]);
    let zero = b.add(NodeKind::Lit, Payload::LitInt(0), sp, &[]);
    let div = b.add(NodeKind::BinOp, Payload::Op(Op::Div), sp, &[one, zero]);
    let seq = b.add(NodeKind::Seq, Payload::None, sp, &[div]);
    let ret = b.add(NodeKind::Return, Payload::None, sp, &[seq]);
    let func = b.add(NodeKind::Func, Payload::None, sp, &[ret]);
    let il = b.finish(
        func,
        FileMeta {
            path: "t".into(),
            lang: Lang::Python,
        },
        Vec::new(),
        Vec::new(),
    );
    run_admitted_unit(il, func, &[]).expect("run_unit").ret
}

#[test]
fn seq_expression_propagates_error_items() {
    assert_eq!(seq_with_error_item_value(), Value::Err);
}
