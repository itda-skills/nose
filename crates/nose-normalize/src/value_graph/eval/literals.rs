use super::super::*;

impl<'a> Builder<'a> {
    pub(super) fn eval_var_expr(
        &mut self,
        expr: NodeId,
        payload: Payload,
        env: &FxHashMap<u32, ValueId>,
    ) -> ValueId {
        match payload {
            Payload::Cid(c) => env
                .get(&c)
                .copied()
                .unwrap_or_else(|| self.mk(ValOp::Input(c), vec![])),
            // A free variable (global / un-canonicalized callee) kept its name in
            // alpha — give it a STABLE identity keyed by that name (high-bit range,
            // clear of positional cids), so `foo(x)` ≠ `bar(x)` while two uses of
            // `foo` agree. Without this, distinct globals collapsed to one cid.
            Payload::Name(s) => {
                if let Some(&v) = self.global_env.get(&s) {
                    return v;
                }
                let name = self.interner.resolve(s);
                if self.is_rust_option_none_node(expr) {
                    return self.null_const();
                }
                if let Some(contract) = nullish_global_contract(self.il.meta.lang, name) {
                    if !contract.requires_unshadowed
                        || asserted_unshadowed_global_symbol(self.il, expr, contract.name)
                    {
                        return self.null_const();
                    }
                }
                self.mk(ValOp::Input(self.free_name_key(s)), vec![])
            }
            _ => self.fresh_opaque(),
        }
    }
    pub(super) fn eval_lit_expr(&mut self, payload: Payload) -> ValueId {
        // Behavior-defining constants are distinct values: `0` ≠ `1`, `"p"` ≠ `"q"`,
        // `true` ≠ `false`, `3.14` ≠ `2.71`. The kind is carried EXPLICITLY and `bits`
        // holds the FULL value/hash, so nothing wraps a class boundary or truncates
        // (coevo series 8). An abstract `Lit(class)` (value not retained) collapses to
        // `bits = 0` of its kind — all such literals of one class converge, by design.
        let (kind, bits) = match payload {
            Payload::LitInt(v) => (ConstKind::Int, v as u64),
            Payload::LitStr(h) => (ConstKind::Str, h),
            Payload::LitFloat(h) => (ConstKind::Float, h),
            Payload::LitBool(b) => (ConstKind::Bool, b as u64),
            Payload::Lit(LitClass::Int) => (ConstKind::Int, 0),
            Payload::Lit(LitClass::Float) => (ConstKind::Float, 0),
            Payload::Lit(LitClass::Str) => (ConstKind::Str, 0),
            Payload::Lit(LitClass::Bool) => (ConstKind::Bool, 0),
            Payload::Lit(LitClass::Null) => (ConstKind::Null, 0),
            _ => (ConstKind::Sentinel, 0),
        };
        self.mk_const(kind, bits)
    }
}
