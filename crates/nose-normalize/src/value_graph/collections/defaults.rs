use super::super::*;

impl<'a> Builder<'a> {
    pub(in crate::value_graph) fn is_empty_value(&mut self, coll: ValueId) -> ValueId {
        let coll = self.param_domain_value(coll);
        let len = self.mk(ValOp::Call(builtin_tag(Builtin::Len)), vec![coll]);
        let zero = self.int_const(0);
        self.mk(ValOp::Bin(Op::Eq as u32), vec![len, zero])
    }

    pub(in crate::value_graph) fn map_default_pattern(
        &mut self,
        cond: ValueId,
        then_v: ValueId,
        else_v: ValueId,
    ) -> Option<ValueId> {
        let (key, map, negated) = self
            .own_property_condition(cond)
            .or_else(|| self.membership_condition(cond))?;
        let default = if negated {
            if !self.map_lookup_value_matches(else_v, map, key) {
                return None;
            }
            then_v
        } else {
            if !self.map_lookup_value_matches(then_v, map, key) {
                return None;
            }
            else_v
        };
        Some(self.mk(
            ValOp::Call(builtin_tag(Builtin::GetOrDefault)),
            vec![map, key, default],
        ))
    }

    pub(in crate::value_graph) fn value_default_pattern(
        &mut self,
        cond: ValueId,
        then_v: ValueId,
        else_v: ValueId,
    ) -> Option<ValueId> {
        if self.is_bottom_value(then_v) || self.is_bottom_value(else_v) {
            return None;
        }
        let (value, present) = self.null_condition(cond)?;
        let default = if present {
            if !self.value_branch_returns_value(then_v, value) {
                return None;
            }
            else_v
        } else {
            if !self.value_branch_returns_value(else_v, value) {
                return None;
            }
            then_v
        };
        Some(self.mk_nullish_map_default(value, default))
    }

    fn value_default_call(&self, value: ValueId) -> Option<(ValueId, ValueId)> {
        let node = &self.nodes[value as usize];
        if matches!(node.op, ValOp::Call(tag) if tag == builtin_tag(Builtin::ValueOrDefault))
            && node.args.len() == 2
        {
            Some((node.args[0], node.args[1]))
        } else {
            None
        }
    }

    fn value_branch_returns_value(&self, branch: ValueId, value: ValueId) -> bool {
        branch == value
            || self
                .value_default_call(branch)
                .is_some_and(|(inner_value, _)| inner_value == value)
    }

    fn mk_value_default(&mut self, value: ValueId, default: ValueId) -> ValueId {
        if self
            .value_default_call(value)
            .is_some_and(|(_, inner_default)| inner_default == default)
        {
            return value;
        }
        self.mk(
            ValOp::Call(builtin_tag(Builtin::ValueOrDefault)),
            vec![value, default],
        )
    }

    /// A NULL-guarded map default (`m.get(k) ?? d`, `m.get(k) == null ? d : …`) is the faithful
    /// nullish coalesce — it replaces BOTH absent (`undefined`) AND a stored `null` with the
    /// default. Model it as `ValueOrDefault`; do NOT upgrade it to `GetOrDefault(map, key, d)`,
    /// the absence-only fold ("d iff the key is absent, else the stored value — null included").
    /// The two diverge on a present key whose value is null, and the map's value-type nullability
    /// is erased from the IL, so the upgrade cannot be proven sound — it false-merged the coalesce
    /// form with the genuine presence-default `m.has(k) ? m.get(k) : d` (#410, experiments §CT).
    /// The provable-absence forms (`m.has(k)`, `k in m`, Python `d.get(k, d)`) fold to
    /// `GetOrDefault` through `map_presence_condition`, a separate path this never touches.
    pub(in crate::value_graph) fn mk_nullish_map_default(
        &mut self,
        value: ValueId,
        default: ValueId,
    ) -> ValueId {
        self.mk_value_default(value, default)
    }

    pub(in crate::value_graph) fn null_condition(&self, cond: ValueId) -> Option<(ValueId, bool)> {
        let node = &self.nodes[cond as usize];
        if node.args.len() == 2 {
            if matches!(node.op, ValOp::Bin(o) if o == Op::Eq as u32) {
                if self.is_null_value(node.args[0]) {
                    return Some((node.args[1], false));
                }
                if self.is_null_value(node.args[1]) {
                    return Some((node.args[0], false));
                }
            }
            if matches!(node.op, ValOp::Bin(o) if o == Op::Ne as u32) {
                if self.is_null_value(node.args[0]) {
                    return Some((node.args[1], true));
                }
                if self.is_null_value(node.args[1]) {
                    return Some((node.args[0], true));
                }
            }
        }
        None
    }

    pub(in crate::value_graph) fn is_null_value(&self, value: ValueId) -> bool {
        matches!(
            self.nodes[value as usize].op,
            ValOp::Const {
                kind: ConstKind::Null,
                ..
            }
        )
    }

    fn membership_condition(&self, cond: ValueId) -> Option<(ValueId, ValueId, bool)> {
        let node = &self.nodes[cond as usize];
        if matches!(node.op, ValOp::Bin(o) if o == Op::In as u32) && node.args.len() == 2 {
            return Some((node.args[0], node.args[1], false));
        }
        if matches!(node.op, ValOp::Un(o) if o == Op::Not as u32) && node.args.len() == 1 {
            let inner = &self.nodes[node.args[0] as usize];
            if matches!(inner.op, ValOp::Bin(o) if o == Op::In as u32) && inner.args.len() == 2 {
                return Some((inner.args[0], inner.args[1], true));
            }
        }
        None
    }

    pub(in crate::value_graph) fn map_presence_condition(
        &self,
        cond: ValueId,
    ) -> Option<(ValueId, ValueId, bool)> {
        let (key, map, negated) = self
            .own_property_condition(cond)
            .or_else(|| self.membership_condition(cond))?;
        Some((key, map, !negated))
    }

    pub(in crate::value_graph) fn static_literal_membership_predicate(
        &mut self,
        pred: ValueId,
    ) -> Option<(ValueId, ValueId)> {
        let node = &self.nodes[pred as usize];
        if !matches!(node.op, ValOp::Bin(o) if o == Op::Eq as u32) || node.args.len() != 2 {
            return None;
        }
        let left = node.args[0];
        let right = node.args[1];
        if let Some(collection) = self.static_literal_elem_collection(left) {
            return Some((right, collection));
        }
        if let Some(collection) = self.static_literal_elem_collection(right) {
            return Some((left, collection));
        }
        None
    }

    pub(in crate::value_graph) fn static_literal_absence_predicate(
        &mut self,
        pred: ValueId,
    ) -> Option<(ValueId, ValueId)> {
        let node = &self.nodes[pred as usize];
        if !matches!(node.op, ValOp::Bin(o) if o == Op::Ne as u32) || node.args.len() != 2 {
            return None;
        }
        let left = node.args[0];
        let right = node.args[1];
        if let Some(collection) = self.static_literal_elem_collection(left) {
            return Some((right, collection));
        }
        if let Some(collection) = self.static_literal_elem_collection(right) {
            return Some((left, collection));
        }
        None
    }

    fn static_literal_elem_collection(&mut self, value: ValueId) -> Option<ValueId> {
        let node = &self.nodes[value as usize];
        if !matches!(node.op, ValOp::Elem(_)) || node.args.len() != 1 {
            return None;
        }
        let collection = node.args[0];
        self.is_static_membership_collection(collection)
            .then(|| self.canonical_membership_collection_value(collection))
    }

    fn is_static_membership_collection(&self, value: ValueId) -> bool {
        let node = &self.nodes[value as usize];
        matches!(node.op, ValOp::Seq(SEQ_VALUE_COLLECTION)) && !node.args.is_empty()
    }

    fn own_property_condition(&self, cond: ValueId) -> Option<(ValueId, ValueId, bool)> {
        let parse = |value: ValueId| {
            let node = &self.nodes[value as usize];
            if matches!(node.op, ValOp::Seq(SEQ_VALUE_OWN_PROPERTY_GUARD)) && node.args.len() == 4 {
                let map = node.args[0];
                if !matches!(self.nodes[map as usize].op, ValOp::Seq(SEQ_VALUE_MAP)) {
                    return None;
                }
                return Some((node.args[1], map, false));
            }
            None
        };
        let node = &self.nodes[cond as usize];
        if let Some(parts) = parse(cond) {
            return Some(parts);
        }
        if matches!(node.op, ValOp::Un(o) if o == Op::Not as u32) && node.args.len() == 1 {
            if let Some((key, map, _)) = parse(node.args[0]) {
                return Some((key, map, true));
            }
        }
        None
    }

    pub(in crate::value_graph) fn map_lookup_value_matches(
        &mut self,
        value: ValueId,
        map: ValueId,
        key: ValueId,
    ) -> bool {
        let node = &self.nodes[value as usize];
        if matches!(node.op, ValOp::Index) && node.args.as_slice() == [map, key] {
            return true;
        }
        if !matches!(node.op, ValOp::Call(0)) || node.args.len() != 2 {
            return false;
        }
        let args = node.args.clone();
        if args[1] != key {
            return false;
        }
        let callee = &self.nodes[args[0] as usize];
        let ValOp::Field(method) = callee.op else {
            return false;
        };
        if callee.args.len() != 1 {
            return false;
        }
        if admitted_map_get_at_call_span(
            self.il,
            self.interner,
            self.library_api_span_call(value, args[0], Some(callee.args[0]), 1),
            method,
        )
        .is_none()
        {
            return false;
        }
        let receiver = callee.args[0];
        receiver == map
            || self
                .proven_map_value(receiver)
                .is_some_and(|candidate| candidate == map)
    }

    pub(in crate::value_graph) fn eval_membership_collection(
        &mut self,
        collection: NodeId,
        env: &FxHashMap<u32, ValueId>,
    ) -> ValueId {
        if self.il.kind(collection) == NodeKind::Seq {
            if self
                .seq_surface(collection)
                .is_some_and(|contract| contract.membership_collection)
            {
                let kids = self.il.children(collection).to_vec();
                let mut items: Vec<ValueId> = kids.iter().map(|&k| self.eval(k, env)).collect();
                items.sort_by_key(|&v| (self.vhash[v as usize], v));
                items.dedup();
                return self.mk(ValOp::Seq(SEQ_VALUE_COLLECTION), items);
            }
            let value = self.eval(collection, env);
            return self.canonical_membership_collection_value(value);
        }
        let value = self.eval(collection, env);
        if let Some(collection) = self
            .proven_collection_value(value)
            .or_else(|| self.proven_local_collection_binding_value(collection, env))
        {
            return self.canonical_membership_collection_value(collection);
        }
        if self.is_collection_param_expr(collection) {
            return self.mk(ValOp::CollectionParam, vec![value]);
        }
        self.canonical_membership_collection_value(value)
    }

    pub(in crate::value_graph) fn canonical_membership_collection_value(
        &mut self,
        value: ValueId,
    ) -> ValueId {
        let node = &self.nodes[value as usize];
        if !matches!(node.op, ValOp::Seq(SEQ_VALUE_COLLECTION)) {
            return value;
        }
        let mut items = node.args.clone();
        items.sort_by_key(|&v| (self.vhash[v as usize], v));
        items.dedup();
        self.mk(ValOp::Seq(SEQ_VALUE_COLLECTION), items)
    }

    pub(in crate::value_graph) fn is_static_non_float_collection_expr(
        &self,
        collection: NodeId,
    ) -> bool {
        if self.il.kind(collection) != NodeKind::Seq {
            return false;
        }
        if !self
            .seq_surface(collection)
            .is_some_and(|contract| contract.membership_collection)
        {
            return false;
        }
        let kids = self.il.children(collection);
        !kids.is_empty()
            && kids.iter().all(|&kid| {
                self.il.kind(kid) == NodeKind::Lit
                    && matches!(
                        self.il.node(kid).payload,
                        Payload::LitInt(_)
                            | Payload::LitBool(_)
                            | Payload::LitStr(_)
                            | Payload::Lit(LitClass::Null)
                    )
            })
    }

    pub(in crate::value_graph) fn eval_map_lookup_collection(
        &mut self,
        collection: NodeId,
        env: &FxHashMap<u32, ValueId>,
    ) -> ValueId {
        let value = self.eval(collection, env);
        self.proven_map_value(value).unwrap_or(value)
    }
}
