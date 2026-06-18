use super::super::*;

impl<'a> Builder<'a> {
    pub(in crate::value_graph) fn eval_proven_integer_method_call(
        &mut self,
        call: NodeId,
        kids: &[NodeId],
        env: &FxHashMap<u32, ValueId>,
    ) -> Option<ValueId> {
        let admitted = admitted_scalar_integer_method_at_call(self.il, self.interner, call)?;
        let contract = admitted.contract;
        match contract.result.receiver {
            MethodReceiverContract::ExactInteger => {
                let receiver = admitted.receiver?;
                let receiver_value = self.eval_proven_integer_expr(receiver, env)?;
                match contract.result.semantic {
                    ScalarIntegerMethod::Abs => {
                        Some(self.mk(ValOp::Un(ABS_CODE), vec![receiver_value]))
                    }
                    ScalarIntegerMethod::Min => {
                        let rhs = self.eval(*kids.get(1)?, env);
                        self.eval_proven_integer_minmax_method_call(MIN_CODE, receiver_value, rhs)
                    }
                    ScalarIntegerMethod::Max => {
                        let rhs = self.eval(*kids.get(1)?, env);
                        self.eval_proven_integer_minmax_method_call(MAX_CODE, receiver_value, rhs)
                    }
                    ScalarIntegerMethod::Clamp => {
                        let lo = self.eval(*kids.get(1)?, env);
                        let hi = self.eval(*kids.get(2)?, env);
                        self.eval_proven_integer_clamp_method_call(receiver_value, lo, hi)
                    }
                }
            }
            MethodReceiverContract::UnshadowedGlobal("Math") if self.il.meta.lang == Lang::Java => {
                match contract.result.semantic {
                    ScalarIntegerMethod::Abs => {
                        let value = self.eval_proven_integer_expr(*kids.get(1)?, env)?;
                        Some(self.mk(ValOp::Un(ABS_CODE), vec![value]))
                    }
                    ScalarIntegerMethod::Min | ScalarIntegerMethod::Max => {
                        let left = self.eval_proven_integer_expr(*kids.get(1)?, env)?;
                        let right = self.eval_proven_integer_expr(*kids.get(2)?, env)?;
                        let op = match contract.result.semantic {
                            ScalarIntegerMethod::Min => MIN_CODE,
                            ScalarIntegerMethod::Max => MAX_CODE,
                            _ => unreachable!(),
                        };
                        Some(self.mk(ValOp::Bin(op), vec![left, right]))
                    }
                    ScalarIntegerMethod::Clamp => None,
                }
            }
            _ => None,
        }
    }
    fn eval_proven_integer_minmax_method_call(
        &mut self,
        op: u32,
        receiver: ValueId,
        rhs: ValueId,
    ) -> Option<ValueId> {
        if !self.is_integer_domain_value(rhs) {
            return None;
        }
        Some(self.mk(ValOp::Bin(op), vec![receiver, rhs]))
    }
    pub(in crate::value_graph) fn eval_proven_free_minmax_call(
        &mut self,
        expr: NodeId,
        kids: &[NodeId],
        env: &FxHashMap<u32, ValueId>,
    ) -> Option<ValueId> {
        if kids.len() != 3 {
            return None;
        }
        let admitted = admitted_free_function_builtin_at_call(self.il, self.interner, expr)?;
        let contract = admitted.contract;
        let op = match (contract.result.builtin, contract.result.args) {
            (Builtin::Min, BuiltinArgContract::All) => MIN_CODE,
            (Builtin::Max, BuiltinArgContract::All) => MAX_CODE,
            _ => return None,
        };
        let left = self.eval(kids[1], env);
        let right = self.eval(kids[2], env);
        if !self.is_integer_domain_value(left) || !self.is_integer_domain_value(right) {
            return None;
        }
        Some(self.mk(ValOp::Bin(op), vec![left, right]))
    }
    fn eval_proven_integer_clamp_method_call(
        &mut self,
        receiver: ValueId,
        lo: ValueId,
        hi: ValueId,
    ) -> Option<ValueId> {
        if !self.is_integer_domain_value(lo) || !self.is_integer_domain_value(hi) {
            return None;
        }
        self.proof_backed_clamp_value(receiver, lo, hi)
    }
    pub(in crate::value_graph) fn eval_proven_integer_expr(
        &mut self,
        expr: NodeId,
        env: &FxHashMap<u32, ValueId>,
    ) -> Option<ValueId> {
        let value = self.eval(expr, env);
        if self.il.kind(expr) == NodeKind::Var {
            return self.is_integer_param_expr(expr).then_some(value);
        }
        self.is_integer_domain_value(value).then_some(value)
    }
    pub(in crate::value_graph) fn is_integer_domain_value(&self, value: ValueId) -> bool {
        if self.int_const_value(value).is_some()
            || self.is_param_value(value, DomainEvidence::Integer)
        {
            return true;
        }
        let node = &self.nodes[value as usize];
        match node.op {
            ValOp::Un(op) if op == ABS_CODE && node.args.len() == 1 => {
                self.is_integer_domain_value(node.args[0])
            }
            ValOp::Bin(op) if (op == MIN_CODE || op == MAX_CODE) && node.args.len() == 2 => node
                .args
                .iter()
                .copied()
                .all(|arg| self.is_integer_domain_value(arg)),
            ValOp::Clamp if node.args.len() == 3 => node
                .args
                .iter()
                .copied()
                .all(|arg| self.is_integer_domain_value(arg)),
            _ => false,
        }
    }
}
