use super::super::*;

impl<'a> Builder<'a> {
    pub(super) fn eval_hof_expr(
        &mut self,
        expr: NodeId,
        payload: Payload,
        env: &FxHashMap<u32, ValueId>,
    ) -> ValueId {
        let kind = match payload {
            Payload::HoF(h) => h,
            _ => return self.source_salted_opaque(expr, 0x484F_465F),
        };
        let Some(admission) = self.hof_value_admission(expr, kind) else {
            return self.source_salted_opaque(expr, 0x484F_465F);
        };
        self.eval_hof_value(
            expr,
            kind,
            env,
            admission == HofAdmission::SourceComprehension,
        )
    }
    pub(super) fn eval_seq_expr(
        &mut self,
        expr: NodeId,
        payload: Payload,
        env: &FxHashMap<u32, ValueId>,
    ) -> ValueId {
        if let Some(value) = self.import_fact_value(expr) {
            return value;
        }
        let kids = self.il.children(expr).to_vec();
        let a: Vec<ValueId> = kids.iter().map(|&k| self.eval(k, env)).collect();
        if matches!(payload, Payload::Builtin(Builtin::DictEntry)) {
            return self.dict_entry(a);
        }
        if let Some(map) = self.proven_go_literal_zero_map_seq(expr, &a) {
            return map;
        }
        self.mk(ValOp::Seq(self.seq_tag(expr)), a)
    }
    pub(super) fn eval_if_expr(&mut self, expr: NodeId, env: &FxHashMap<u32, ValueId>) -> ValueId {
        // Ternary / expression-if. Rust closures lower `if c { x } else { y }`
        // branches as Blocks whose trailing expression is the branch value, so
        // evaluate branches with the same implicit-return rule used for lambdas.
        let kids = self.il.children(expr).to_vec();
        let mut a = Vec::new();
        if let Some(&cond) = kids.first() {
            a.push(self.eval(cond, env));
        }
        for &branch in kids.iter().skip(1).take(2) {
            let mut branch_env = env.clone();
            let value = self
                .eval_block_return(branch, &mut branch_env)
                .unwrap_or_else(|| self.eval(branch, env));
            a.push(value);
        }
        // abs/min/max idiom recognition happens in `mk(Phi, …)` so it applies to
        // both the ternary and the equivalent if/else-assign form uniformly.
        self.mk(ValOp::Phi, a)
    }
}
