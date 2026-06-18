use super::*;

#[test]
fn value_graph_skips_try_handler_after_normal_return() {
    let i = Interner::new();
    let try_return =
        "def f():\n    try:\n        return 1\n    except Exception:\n        return 7\n";
    let plain_return = "def f():\n    return 1\n";
    assert_eq!(
        value_fp(&i, try_return, Lang::Python),
        value_fp(&i, plain_return, Lang::Python),
        "a try handler should not contribute when the try body already returned normally"
    );
}

#[test]
fn value_graph_runs_try_handler_after_bare_throw() {
    let i = Interner::new();
    let try_throw = "function f() { try { throw \"x\"; } catch (err) { return 7; } }";
    let plain_return = "function f() { return 7; }";
    assert_eq!(
        value_fp(&i, try_throw, Lang::JavaScript),
        value_fp(&i, plain_return, Lang::JavaScript),
        "a side-effect-free throw body should be replaced by the simple catch handler"
    );
}

#[test]
fn value_graph_runs_try_handler_after_pure_throw_prefix() {
    let i = Interner::new();
    let try_throw = "function f() { try { 1 + 2; throw \"x\"; } catch (err) { return 7; } }";
    let plain_return = "function f() { return 7; }";
    assert_eq!(
        value_fp(&i, try_throw, Lang::JavaScript),
        value_fp(&i, plain_return, Lang::JavaScript),
        "pure statements before a throw should not block the simple catch handler"
    );
}

#[test]
fn value_graph_keeps_try_throw_prefix_effects() {
    let i = Interner::new();
    let effect_then_throw = "def f():\n    try:\n        print(1)\n        raise Exception()\n    except Exception:\n        return 7\n";
    let plain_return = "def f():\n    return 7\n";
    assert_ne!(
        value_fp(&i, effect_then_throw, Lang::Python),
        value_fp(&i, plain_return, Lang::Python),
        "observable effects before a throw must not be discarded with the try body"
    );
}

#[test]
fn value_graph_runs_try_handler_after_static_expr_err() {
    let i = Interner::new();
    let try_err = "def f():\n    try:\n        1 / 0\n    except Exception:\n        return 7\n";
    let plain_return = "def f():\n    return 7\n";
    assert_eq!(
        value_fp(&i, try_err, Lang::Python),
        value_fp(&i, plain_return, Lang::Python),
        "a statically visible expression error should run the simple catch handler"
    );
}

#[test]
fn value_graph_runs_try_handler_after_static_return_err() {
    let i = Interner::new();
    let try_err =
        "def f():\n    try:\n        return 1 / 0\n    except Exception:\n        return 7\n";
    let plain_return = "def f():\n    return 7\n";
    assert_eq!(
        value_fp(&i, try_err, Lang::Python),
        value_fp(&i, plain_return, Lang::Python),
        "a statically visible return expression error is not a normal try-body return"
    );
}

#[test]
fn value_graph_runs_try_handler_after_static_ternary_condition_err() {
    let i = Interner::new();
    let try_err =
        "def f():\n    try:\n        return 1 if 1 / 0 else 2\n    except Exception:\n        return 7\n";
    let plain_return = "def f():\n    return 7\n";
    assert_eq!(
        value_fp(&i, try_err, Lang::Python),
        value_fp(&i, plain_return, Lang::Python),
        "a statically visible ternary condition error should run the simple catch handler"
    );
}

#[test]
fn value_graph_runs_try_handler_after_static_selected_ternary_branch_err() {
    let i = Interner::new();
    let try_err =
        "def f():\n    try:\n        return 1 / 0 if True else 2\n    except Exception:\n        return 7\n";
    let plain_return = "def f():\n    return 7\n";
    assert_eq!(
        value_fp(&i, try_err, Lang::Python),
        value_fp(&i, plain_return, Lang::Python),
        "a statically selected ternary branch error should run the simple catch handler"
    );
}

#[test]
fn value_graph_runs_try_handler_after_static_pow_err() {
    let i = Interner::new();
    let try_err =
        "def f():\n    try:\n        return 2 ** -1\n    except Exception:\n        return 7\n";
    let plain_return = "def f():\n    return 7\n";
    assert_eq!(
        value_fp(&i, try_err, Lang::Python),
        value_fp(&i, plain_return, Lang::Python),
        "a statically visible pow exponent error should run the simple catch handler"
    );
}

#[test]
fn value_graph_runs_try_handler_after_static_unary_operand_err() {
    let i = Interner::new();
    let try_err =
        "def f():\n    try:\n        return -(1 / 0)\n    except Exception:\n        return 7\n";
    let plain_return = "def f():\n    return 7\n";
    assert_eq!(
        value_fp(&i, try_err, Lang::Python),
        value_fp(&i, plain_return, Lang::Python),
        "a statically visible unary operand error should run the simple catch handler"
    );
}

#[test]
fn value_graph_runs_try_handler_after_static_binop_left_err() {
    let i = Interner::new();
    let try_err =
        "def f():\n    try:\n        return (1 / 0) + print(1)\n    except Exception:\n        return 7\n";
    let plain_return = "def f():\n    return 7\n";
    assert_eq!(
        value_fp(&i, try_err, Lang::Python),
        value_fp(&i, plain_return, Lang::Python),
        "a statically visible binary left operand error should run the simple catch handler"
    );
}

#[test]
fn value_graph_keeps_try_binop_left_effects_before_static_op_err() {
    let i = Interner::new();
    let effect_then_err =
        "def f():\n    try:\n        return print(1) / 0\n    except Exception:\n        return 7\n";
    let plain_return = "def f():\n    return 7\n";
    assert_ne!(
        value_fp(&i, effect_then_err, Lang::Python),
        value_fp(&i, plain_return, Lang::Python),
        "observable left operand effects before a binary op error must not be discarded"
    );
}

#[test]
fn value_graph_runs_try_handler_after_static_index_base_err() {
    let i = Interner::new();
    let try_err =
        "def f():\n    try:\n        return (1 / 0)[print(1)]\n    except Exception:\n        return 7\n";
    let plain_return = "def f():\n    return 7\n";
    assert_eq!(
        value_fp(&i, try_err, Lang::Python),
        value_fp(&i, plain_return, Lang::Python),
        "a statically visible index base error should run the simple catch handler"
    );
}

#[test]
fn value_graph_runs_try_handler_after_static_field_receiver_err() {
    let i = Interner::new();
    let try_err =
        "def f():\n    try:\n        return (1 / 0).x\n    except Exception:\n        return 7\n";
    let plain_return = "def f():\n    return 7\n";
    assert_eq!(
        value_fp(&i, try_err, Lang::Python),
        value_fp(&i, plain_return, Lang::Python),
        "a statically visible field receiver error should run the simple catch handler"
    );
}

#[test]
fn value_graph_runs_try_handler_after_static_field_assignment_receiver_err() {
    let i = Interner::new();
    let try_err =
        "def f():\n    try:\n        (1 / 0).x = 7\n    except Exception:\n        return 7\n";
    let plain_return = "def f():\n    return 7\n";
    assert_eq!(
        value_fp(&i, try_err, Lang::Python),
        value_fp(&i, plain_return, Lang::Python),
        "a static field assignment receiver error should run the simple catch handler"
    );
}

#[test]
fn value_graph_keeps_try_index_base_effects_before_static_index_err() {
    let i = Interner::new();
    let effect_then_err =
        "def f():\n    try:\n        return print(1)[1 / 0]\n    except Exception:\n        return 7\n";
    let plain_return = "def f():\n    return 7\n";
    assert_ne!(
        value_fp(&i, effect_then_err, Lang::Python),
        value_fp(&i, plain_return, Lang::Python),
        "observable base effects before an index error must not be discarded"
    );
}

#[test]
fn value_graph_runs_try_handler_after_static_index_assignment_target_err() {
    let i = Interner::new();
    let try_err =
        "def f(xs):\n    try:\n        xs[1 / 0] = 2\n    except Exception:\n        return 7\n";
    let plain_return = "def f(xs):\n    return 7\n";
    assert_eq!(
        value_fp(&i, try_err, Lang::Python),
        value_fp(&i, plain_return, Lang::Python),
        "a static index assignment target error should run the simple catch handler"
    );
}

#[test]
fn value_graph_runs_try_handler_after_static_index_assignment_base_err() {
    let i = Interner::new();
    let try_err =
        "def f():\n    try:\n        (1 / 0)[print(1)] = 2\n    except Exception:\n        return 7\n";
    let plain_return = "def f():\n    return 7\n";
    assert_eq!(
        value_fp(&i, try_err, Lang::Python),
        value_fp(&i, plain_return, Lang::Python),
        "a static index assignment base error should run the simple catch handler before subscript effects"
    );
}

#[test]
fn value_graph_keeps_try_index_assignment_rhs_effects_before_target_err() {
    let i = Interner::new();
    let effect_then_err =
        "def f(xs):\n    try:\n        xs[1 / 0] = print(1)\n    except Exception:\n        return 7\n";
    let plain_return = "def f(xs):\n    return 7\n";
    assert_ne!(
        value_fp(&i, effect_then_err, Lang::Python),
        value_fp(&i, plain_return, Lang::Python),
        "observable RHS effects before an index assignment target error must not be discarded"
    );
}

#[test]
fn value_graph_runs_try_handler_after_static_seq_item_err() {
    let i = Interner::new();
    let try_err =
        "def f():\n    try:\n        return [1 / 0]\n    except Exception:\n        return 7\n";
    let plain_return = "def f():\n    return 7\n";
    assert_eq!(
        value_fp(&i, try_err, Lang::Python),
        value_fp(&i, plain_return, Lang::Python),
        "a statically visible sequence item error should run the simple catch handler"
    );
}

#[test]
fn value_graph_runs_try_handler_after_first_static_seq_item_err() {
    let i = Interner::new();
    let try_err =
        "def f():\n    try:\n        return [1 / 0, print(1)]\n    except Exception:\n        return 7\n";
    let plain_return = "def f():\n    return 7\n";
    assert_eq!(
        value_fp(&i, try_err, Lang::Python),
        value_fp(&i, plain_return, Lang::Python),
        "a first sequence item error should run the simple catch handler before later effects"
    );
}

#[test]
fn value_graph_keeps_try_seq_item_effects_before_static_err() {
    let i = Interner::new();
    let effect_then_err =
        "def f():\n    try:\n        return [print(1), 1 / 0]\n    except Exception:\n        return 7\n";
    let plain_return = "def f():\n    return 7\n";
    assert_ne!(
        value_fp(&i, effect_then_err, Lang::Python),
        value_fp(&i, plain_return, Lang::Python),
        "observable sequence item effects before an error must not be discarded"
    );
}

#[test]
fn value_graph_runs_try_handler_after_static_hof_lambda_err() {
    let i = Interner::new();
    let try_err = "def f():\n    try:\n        return [1 / 0 for x in [1]]\n    except Exception:\n        return 7\n";
    let plain_return = "def f():\n    return 7\n";
    assert_eq!(
        value_fp(&i, try_err, Lang::Python),
        value_fp(&i, plain_return, Lang::Python),
        "a statically visible HoF lambda error should run the simple catch handler"
    );
}

#[test]
fn value_graph_skips_try_handler_for_empty_static_hof_lambda_err() {
    let i = Interner::new();
    let empty_map =
        "def f():\n    try:\n        return [1 / 0 for x in []]\n    except Exception:\n        return 7\n";
    let plain_return = "def f():\n    return 7\n";
    assert_ne!(
        value_fp(&i, empty_map, Lang::Python),
        value_fp(&i, plain_return, Lang::Python),
        "a static lambda error is not observable when a known-empty collection skips it"
    );
}

#[test]
fn value_graph_runs_try_handler_after_static_reduce_lambda_err() {
    let i = Interner::new();
    let try_err =
        "import functools\n\ndef f():\n    try:\n        return functools.reduce(lambda a, x: 1 / 0, [1], 0)\n    except Exception:\n        return 7\n";
    let plain_return = "def f():\n    return 7\n";
    assert_eq!(
        value_fp(&i, try_err, Lang::Python),
        value_fp(&i, plain_return, Lang::Python),
        "a statically visible reduce lambda error should run the simple catch handler"
    );
}

#[test]
fn value_graph_skips_try_handler_for_empty_static_reduce_lambda_err() {
    let i = Interner::new();
    let empty_reduce =
        "import functools\n\ndef f():\n    try:\n        return functools.reduce(lambda a, x: 1 / 0, [], 0)\n    except Exception:\n        return 7\n";
    let plain_return = "def f():\n    return 7\n";
    assert_ne!(
        value_fp(&i, empty_reduce, Lang::Python),
        value_fp(&i, plain_return, Lang::Python),
        "a static reduce lambda error is not observable when a known-empty collection skips it"
    );
}

#[test]
fn value_graph_runs_try_handler_after_static_builtin_arg_err() {
    let i = Interner::new();
    let try_err =
        "def f():\n    try:\n        print(1 / 0)\n    except Exception:\n        return 7\n";
    let plain_return = "def f():\n    return 7\n";
    assert_eq!(
        value_fp(&i, try_err, Lang::Python),
        value_fp(&i, plain_return, Lang::Python),
        "a statically visible eager builtin argument error should run the simple catch handler"
    );
}

#[test]
fn value_graph_runs_try_handler_after_static_range_step_err() {
    let i = Interner::new();
    let try_err =
        "def f():\n    try:\n        return range(1, 5, 0)\n    except Exception:\n        return 7\n";
    let plain_return = "def f():\n    return 7\n";
    assert_eq!(
        value_fp(&i, try_err, Lang::Python),
        value_fp(&i, plain_return, Lang::Python),
        "a statically visible range zero-step error should run the simple catch handler"
    );
}

#[test]
fn value_graph_runs_try_handler_after_static_opaque_call_arg_err() {
    let i = Interner::new();
    let try_err =
        "def f():\n    try:\n        unknown(1 / 0)\n    except Exception:\n        return 7\n";
    let plain_return = "def f():\n    return 7\n";
    assert_eq!(
        value_fp(&i, try_err, Lang::Python),
        value_fp(&i, plain_return, Lang::Python),
        "a statically visible opaque call argument error should run the simple catch handler"
    );
}

#[test]
fn value_graph_keeps_try_opaque_call_arg_prefix_effects() {
    let i = Interner::new();
    let effect_then_err =
        "def f():\n    try:\n        unknown(print(1), 1 / 0)\n    except Exception:\n        return 7\n";
    let plain_return = "def f():\n    return 7\n";
    assert_ne!(
        value_fp(&i, effect_then_err, Lang::Python),
        value_fp(&i, plain_return, Lang::Python),
        "observable argument effects before a runtime error must not be discarded"
    );
}

#[test]
fn value_graph_keeps_try_static_expr_err_prefix_effects() {
    let i = Interner::new();
    let effect_then_err = "def f():\n    try:\n        print(1)\n        1 / 0\n    except Exception:\n        return 7\n";
    let plain_return = "def f():\n    return 7\n";
    assert_ne!(
        value_fp(&i, effect_then_err, Lang::Python),
        value_fp(&i, plain_return, Lang::Python),
        "observable effects before a runtime error must not be discarded with the try body"
    );
}
