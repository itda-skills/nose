use super::super::*;

impl<'a> Builder<'a> {
    pub(in crate::value_graph) fn mk(&mut self, mut op: ValOp, mut args: Vec<ValueId>) -> ValueId {
        // Opaque-fallback census (#391 prevalence probe; only when enabled). Record before any
        // canonicalization — attributes the opaque to the IL construct that is being evaluated.
        if matches!(op, ValOp::Opaque(_)) {
            if let (Some(census), Some(kind)) = (self.opaque_census.as_mut(), self.cur_il_kind) {
                *census.entry((kind, args.is_empty())).or_insert(0) += 1;
            }
        }
        self.commute_numeric_reduce_contrib(&op, &mut args);
        self.order_bin_operands(&mut op, &mut args);
        if let Some(v) = self.u16_byte_pack(&op, &args) {
            return v;
        }
        // Type-gated simplifications — now SOUND because the operand type is PROVEN (these
        // were the 17 false merges when applied untyped; they only hold on numbers/bools):
        //   -(-x) → x        when x : Num   (−(−x) = x; on a list it would Err ≠ x)
        //   x & x, x | x → x when x : Num   (idempotent integer bitwise)
        //   x && x, x || x → x when x : Bool (idempotent boolean)
        if let Some(v) = self.unary_canon(&op, &args) {
            return v;
        }
        if let Some(v) = self.bin_idempotence_and_factor(&op, &args) {
            return v;
        }
        if let Some(v) = self.bool_and_or_canon(&op, &mut args) {
            return v;
        }
        if let Some(v) = self.phi_select_idioms(&op, &args) {
            return v;
        }
        if let Some(v) = self.bool_chain_flatten(&op, &args) {
            return v;
        }
        if let Some(v) = self.ac_chain_canon(&op, &args) {
            return v;
        }
        let id = self.intern_node(op, args);
        rules::clamp::apply(self, id).unwrap_or(id)
    }
    /// Intern a value node by `(op, args)` (hash-consing), computing its structural hash
    /// and kernel value domain. The raw constructor used by `mk` after canonicalization
    /// does not itself canonicalize, so callers must pass already-canonical operands.
    pub(super) fn intern_node(&mut self, op: ValOp, args: Vec<ValueId>) -> ValueId {
        let key = (op.clone(), args.clone());
        if let Some(&id) = self.intern.get(&key) {
            return id;
        }
        let id = self.nodes.len() as ValueId;
        let mut h = op_tag(&op);
        for &a in &args {
            h = combine(h, self.vhash[a as usize]);
        }
        let ty = self.value_domain_of(&op, &args);
        self.nodes.push(ValNode { op, args });
        self.vhash.push(h);
        self.vty.push(ty);
        self.node_span.push(self.cur_span);
        self.intern.insert(key, id);
        id
    }
    pub(in crate::value_graph) fn fresh_opaque(&mut self) -> ValueId {
        let c = self.opaque_ctr;
        self.opaque_ctr += 1;
        self.mk(ValOp::Opaque(c as u64), vec![])
    }
}
