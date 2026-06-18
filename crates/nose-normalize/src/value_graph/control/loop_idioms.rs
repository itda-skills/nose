use super::super::*;

impl<'a> Builder<'a> {
    pub(in crate::value_graph) fn flag_break_reduction(
        &mut self,
        body: NodeId,
        cid: u32,
        init: ValueId,
        env: &FxHashMap<u32, ValueId>,
        index_vals: &FxHashSet<ValueId>,
    ) -> Option<ValueId> {
        let init_bool = self.bool_const(init)?;
        let (cond_node, assigned_bool) = self.flag_break_if(body, cid)?;
        if init_bool == assigned_bool {
            return None;
        }
        let mut cond = self.eval(cond_node, env);
        if !index_vals.is_empty() {
            let mut memo = FxHashMap::default();
            cond = self.rewrite_indices(cond, index_vals, &mut memo);
        }
        if !self.refs_elem(cond) {
            return None;
        }
        if !init_bool && assigned_bool {
            Some(self.mk(ValOp::Reduce(REDUCE_ANY), vec![cond]))
        } else {
            let pred = self.mk(ValOp::Un(Op::Not as u32), vec![cond]);
            Some(self.mk(ValOp::Reduce(REDUCE_ALL), vec![pred]))
        }
    }

    pub(in crate::value_graph) fn flat_map_builder_value(
        &mut self,
        value: ValueId,
        pattern_bindings: &[(u32, ValueId)],
    ) -> Option<ValueId> {
        let outer_elem = pattern_bindings.first()?.1;
        let op = self.nodes[value as usize].op.clone();
        match op {
            ValOp::Hof(k) if k == HoFKind::Map as u32 || k == HoFKind::FlatMap as u32 => {
                Some(self.mk(ValOp::Hof(HoFKind::FlatMap as u32), vec![outer_elem, value]))
            }
            _ => None,
        }
    }

    pub(in crate::value_graph) fn flag_break_if(
        &self,
        body: NodeId,
        cid: u32,
    ) -> Option<(NodeId, bool)> {
        let stmts = self.direct_block_statements(body);
        if stmts.len() != 1 || self.il.kind(stmts[0]) != NodeKind::If {
            return None;
        }
        let if_kids = self.il.children(stmts[0]);
        if if_kids.len() != 2 {
            return None;
        }
        let branch = self.direct_block_statements(if_kids[1]);
        if branch.len() != 2 || self.il.kind(branch[1]) != NodeKind::Break {
            return None;
        }
        Some((if_kids[0], self.flag_assignment(branch[0], cid)?))
    }

    pub(in crate::value_graph) fn ordered_string_concat_loop(
        &mut self,
        body: NodeId,
        cid: u32,
        init: ValueId,
        env: &FxHashMap<u32, ValueId>,
        index_vals: &FxHashSet<ValueId>,
    ) -> Option<ValueId> {
        if !self.is_empty_string_value(init) {
            return None;
        }
        let contrib_node = self.ordered_concat_contribution(body, cid)?;
        let mut contrib = self.eval(contrib_node, env);
        if !index_vals.is_empty() {
            let mut memo = FxHashMap::default();
            contrib = self.rewrite_indices(contrib, index_vals, &mut memo);
        }
        if !self.refs_elem(contrib) {
            return None;
        }
        let sep = self.empty_string_value();
        Some(self.mk(ValOp::Reduce(ORDERED_STRING_JOIN), vec![sep, contrib]))
    }

    pub(in crate::value_graph) fn ordered_concat_contribution(
        &self,
        body: NodeId,
        cid: u32,
    ) -> Option<NodeId> {
        let stmts = self.direct_block_statements(body);
        if stmts.len() != 1 || self.il.kind(stmts[0]) != NodeKind::Assign {
            return None;
        }
        let kids = self.il.children(stmts[0]);
        if kids.len() != 2 || !self.is_var_cid(kids[0], cid) {
            return None;
        }
        if self.il.kind(kids[1]) != NodeKind::BinOp
            || op_code(self.il.node(kids[1]).payload) != Op::Add as u32
        {
            return None;
        }
        let add = self.il.children(kids[1]);
        if add.len() != 2 || !self.is_var_cid(add[0], cid) {
            return None;
        }
        if mentioned_cids(self.il, add[1]).contains(&cid) {
            return None;
        }
        Some(add[1])
    }

    pub(in crate::value_graph) fn direct_block_statements(&self, node: NodeId) -> Vec<NodeId> {
        if self.il.kind(node) == NodeKind::Block {
            self.il.children(node).to_vec()
        } else {
            vec![node]
        }
    }

    pub(in crate::value_graph) fn flag_assignment(&self, stmt: NodeId, cid: u32) -> Option<bool> {
        if self.il.kind(stmt) != NodeKind::Assign {
            return None;
        }
        let kids = self.il.children(stmt);
        if kids.len() != 2 || !self.is_var_cid(kids[0], cid) {
            return None;
        }
        match self.il.node(kids[1]).payload {
            Payload::LitBool(value) => Some(value),
            _ => None,
        }
    }

    pub(in crate::value_graph) fn is_var_cid(&self, node: NodeId, cid: u32) -> bool {
        matches!(
            (self.il.kind(node), self.il.node(node).payload),
            (NodeKind::Var, Payload::Cid(c)) if c == cid
        )
    }

    pub(in crate::value_graph) fn loop_entry_condition_is_proven_false(
        &self,
        cond: NodeId,
        env: &FxHashMap<u32, ValueId>,
    ) -> bool {
        if self.condition_atom_is_proven_false(cond, env) {
            return true;
        }
        if self.il.kind(cond) != NodeKind::BinOp
            || op_code(self.il.node(cond).payload) != Op::And as u32
        {
            return false;
        }
        let kids = self.il.children(cond);
        kids.len() == 2 && self.condition_atom_is_proven_false(kids[0], env)
    }

    pub(in crate::value_graph) fn condition_atom_is_proven_false(
        &self,
        atom: NodeId,
        env: &FxHashMap<u32, ValueId>,
    ) -> bool {
        match self.il.node(atom).payload {
            Payload::LitBool(false) if self.il.kind(atom) == NodeKind::Lit => true,
            Payload::Cid(cid) if self.il.kind(atom) == NodeKind::Var => env
                .get(&cid)
                .and_then(|&v| self.bool_const(v))
                .is_some_and(|value| !value),
            Payload::Op(Op::Not) if self.il.kind(atom) == NodeKind::UnOp => {
                let kids = self.il.children(atom);
                if kids.len() != 1 {
                    return false;
                }
                match self.il.node(kids[0]).payload {
                    Payload::LitBool(true) if self.il.kind(kids[0]) == NodeKind::Lit => true,
                    Payload::Cid(cid) if self.il.kind(kids[0]) == NodeKind::Var => env
                        .get(&cid)
                        .and_then(|&v| self.bool_const(v))
                        .is_some_and(|value| value),
                    _ => false,
                }
            }
            _ => false,
        }
    }

    /// The iterable of a `while i < len(xs)`-style loop: from a comparison whose
    /// bound side is `len(iterable)`, return the `iterable` node. Requires the other
    /// side to reference an induction variable (so we don't misread `a < len(b)`).
    pub(in crate::value_graph) fn loop_iterable(
        &self,
        cond: NodeId,
        induction: &FxHashSet<u32>,
    ) -> Option<NodeId> {
        if self.il.kind(cond) != NodeKind::BinOp {
            return None;
        }
        let kids = self.il.children(cond).to_vec();
        if kids.len() != 2 {
            return None;
        }
        let mentions_ind = |n: NodeId| {
            matches!((self.il.kind(n), self.il.node(n).payload),
                (NodeKind::Var, Payload::Cid(c)) if induction.contains(&c))
        };
        if !kids.iter().any(|&k| mentions_ind(k)) {
            return None;
        }
        // The other operand is `len(iterable)` → a Len builtin Call with one arg.
        for &k in &kids {
            if self.il.kind(k) == NodeKind::Call
                && matches!(self.il.node(k).payload, Payload::Builtin(Builtin::Len))
                && self.admitted_builtin_call(k, Builtin::Len)
            {
                if let Some(&arg) = self.il.children(k).first() {
                    return Some(arg);
                }
            }
        }
        None
    }

    /// Conservative C-style pointer+length loop recognition:
    ///
    /// `while i < n { ... xs[i] ...; i += 1 }`
    ///
    /// Unlike `i < len(xs)`, the bound is not intrinsically tied to the collection.
    /// Therefore this only licenses the local `xs[i] -> Elem(xs)` rewrite and records a
    /// bound guard keyed by the normalized comparison and bound value. That lets
    /// C `for`/`while` spellings of the same `(ptr, len)` traversal converge without
    /// claiming the loop is automatically identical to a high-level full-collection
    /// traversal.
    pub(in crate::value_graph) fn indexed_bound_loop_iterable(
        &self,
        cond: NodeId,
        body: NodeId,
        induction: &FxHashSet<u32>,
    ) -> Option<(NodeId, NodeId, u32)> {
        if self.il.kind(cond) != NodeKind::BinOp {
            return None;
        }
        let cmp = op_code(self.il.node(cond).payload);
        let kids = self.il.children(cond);
        if kids.len() != 2 {
            return None;
        }

        let left_ind = self.direct_induction_cid(kids[0], induction);
        let right_ind = self.direct_induction_cid(kids[1], induction);
        let (cid, bound, normalized_cmp) = match (left_ind, right_ind) {
            (Some(cid), None) if !mentioned_cids(self.il, kids[1]).contains(&cid) => {
                (cid, kids[1], cmp)
            }
            (None, Some(cid)) if !mentioned_cids(self.il, kids[0]).contains(&cid) => {
                (cid, kids[0], reverse_cmp_code(self.il.meta.lang, cmp)?)
            }
            _ => return None,
        };
        if normalized_cmp != Op::Lt as u32 && normalized_cmp != Op::Le as u32 {
            return None;
        }

        let collection = self.indexed_collection_in_body(body, cid)?;
        Some((collection, bound, normalized_cmp))
    }

    pub(in crate::value_graph) fn direct_induction_cid(
        &self,
        node: NodeId,
        induction: &FxHashSet<u32>,
    ) -> Option<u32> {
        match (self.il.kind(node), self.il.node(node).payload) {
            (NodeKind::Var, Payload::Cid(c)) if induction.contains(&c) => Some(c),
            _ => None,
        }
    }

    pub(in crate::value_graph) fn indexed_collection_in_body(
        &self,
        node: NodeId,
        cid: u32,
    ) -> Option<NodeId> {
        if self.il.kind(node) == NodeKind::Index {
            let kids = self.il.children(node);
            if kids.len() == 2
                && matches!(
                    (self.il.kind(kids[1]), self.il.node(kids[1]).payload),
                    (NodeKind::Var, Payload::Cid(c)) if c == cid
                )
            {
                return Some(kids[0]);
            }
        }
        for &c in self.il.children(node) {
            if let Some(collection) = self.indexed_collection_in_body(c, cid) {
                return Some(collection);
            }
        }
        None
    }

    pub(in crate::value_graph) fn indexed_bound_guard(
        &mut self,
        cmp: u32,
        bound: ValueId,
    ) -> ValueId {
        let marker = self.int_const(0xC10C_0000);
        let cmp_value = self.int_const(0xC10C_1000u32.wrapping_add(cmp));
        self.mk(ValOp::Call(0), vec![marker, cmp_value, bound])
    }

    pub(in crate::value_graph) fn full_pointer_length_contract(
        &self,
        cmp: u32,
        collection: ValueId,
        bound: ValueId,
    ) -> bool {
        if cmp != Op::Lt as u32 {
            return false;
        }
        matches!(
            (self.input_key(collection), self.input_key(bound)),
            // Single pointer-length convention: `(xs, n)`.
            (Some(0), Some(1))
                // Two aligned pointer arrays with shared length: `(a, b, n)`.
                | (Some(0), Some(2))
                | (Some(1), Some(2))
        )
    }

    pub(in crate::value_graph) fn input_key(&self, value: ValueId) -> Option<u32> {
        match self.nodes[value as usize].op {
            ValOp::Input(key) => Some(key),
            _ => None,
        }
    }

    /// Rewrite every `Index(C, idx)` whose index is in `index_vals` to `Elem(C)`,
    /// throughout `val`'s subgraph (DAG-safe, memoized). This is what makes indexed
    /// iteration converge with value iteration: `xs[i]` (any collection, any index
    /// variable) becomes the canonical element of that collection.
    pub(in crate::value_graph) fn rewrite_indices(
        &mut self,
        val: ValueId,
        index_vals: &FxHashSet<ValueId>,
        memo: &mut FxHashMap<ValueId, ValueId>,
    ) -> ValueId {
        if let Some(&m) = memo.get(&val) {
            return m;
        }
        let (op, args) = {
            let n = &self.nodes[val as usize];
            (n.op.clone(), n.args.clone())
        };
        let new_args: Vec<ValueId> = args
            .iter()
            .map(|&a| self.rewrite_indices(a, index_vals, memo))
            .collect();
        // `C[idx]` with an index-role `idx` → `Elem(C)`.
        let r = if matches!(op, ValOp::Index)
            && new_args.len() == 2
            && index_vals.contains(&new_args[1])
        {
            self.elem(new_args[0])
        } else if new_args == args {
            val
        } else {
            self.mk(op, new_args)
        };
        memo.insert(val, r);
        r
    }
}
