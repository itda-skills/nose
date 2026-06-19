use super::super::*;

impl<'a> Builder<'a> {
    pub(super) fn order_bin_operands(&mut self, op: &mut ValOp, args: &mut [ValueId]) {
        let ValOp::Bin(opc) = *op else { return };
        // Canonicalize comparison DIRECTION: `a > b` ≡ `b < a`, `a >= b` ≡ `b <= a`.
        // Reduce the >/>= family to </<= with swapped operands so a guard converges
        // however it was written or negated (`0 < v`, `v > 0`, `!(v <= 0)` all become
        // one node). Language-agnostic and sound (total order). This is what lets a
        // `reduce(λa,v: a+v if v>0 else a, …)` fold converge with its loop, whose
        // guard may lower to the mirror comparison.
        if args.len() == 2 && self.comparison_law_enabled(ComparisonLaw::DirectionCanon) {
            if opc == Op::Gt as u32 {
                *op = ValOp::Bin(Op::Lt as u32);
                args.swap(0, 1);
            } else if opc == Op::Ge as u32 {
                *op = ValOp::Bin(Op::Le as u32);
                args.swap(0, 1);
            }
        }
        // Canonicalize commutative operands by structural hash. The commute is gated per op
        // (`ac_chain_commutes`): `+` is held ordered when an operand is PROVEN string/list
        // (concat, `s + x` ≠ `x + s`, #283-C) and `*` when it could be a Ruby string/list
        // repetition (`"ab" * 3` ≠ `3 * "ab"`, series 9) — both distinguished by the
        // free-monoid oracle. Unknown operands keep commuting (optimistic, oracle-checked),
        // so the common untyped numeric case is unaffected; `& | ^`, `==`/`!=`, min/max all
        // Err-or-symmetric regardless of order and stay safe.
        if let ValOp::Bin(o) = *op {
            if is_commutative(o)
                && args.len() == 2
                && self.ac_chain_commutes(o, args, ValueLaw::AddCommutativity)
                && self.reorder_safe(args[0])
                && self.reorder_safe(args[1])
                && self.vhash[args[0] as usize] > self.vhash[args[1] as usize]
            {
                args.swap(0, 1);
            }
        }
    }
    /// Effect-free ⇒ safe to reorder past a sibling operand. A subtree with a
    /// call/HOF/lambda/opaque node can carry an observable effect whose order the
    /// interpreter tracks, so it is held in place (coevo §CE / §AS).
    pub(in crate::value_graph) fn reorder_safe(&mut self, v: ValueId) -> bool {
        if let Some(&safe) = self.reorder_safe_cache.get(&v) {
            return safe;
        }
        let mut seen = FxHashSet::default();
        let mut stack = vec![v];
        let mut safe = true;
        while let Some(n) = stack.pop() {
            if !seen.insert(n) {
                continue;
            }
            match self.nodes[n as usize].op {
                ValOp::Call(_)
                | ValOp::Hof(_)
                | ValOp::Lambda(_)
                | ValOp::Loop(_)
                | ValOp::Recurrence(_)
                | ValOp::Opaque(_) => {
                    safe = false;
                    break;
                }
                _ => {}
            }
            stack.extend(self.nodes[n as usize].args.iter().copied());
        }
        self.reorder_safe_cache.insert(v, safe);
        safe
    }
}
