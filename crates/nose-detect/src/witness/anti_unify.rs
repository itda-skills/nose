use super::{dag::Dag, model::MAX_DEPTH};
use nose_normalize::{bin_is_commutative, VgOp, VG_PROTOCOL_AWAIT};
use rustc_hash::{FxHashMap, FxHashSet};

/// `(node, on_a)`; `node == NONE` marks an absent side (a one-sided hole).
pub(super) const NONE: u32 = u32::MAX;

pub(super) struct Au<'a> {
    a: &'a Dag<'a>,
    b: &'a Dag<'a>,
    visited: FxHashSet<(u32, u32)>,
    /// `(a_node, b_node)`; either may be [`NONE`] for a one-sided hole.
    pub(super) holes: Vec<(u32, u32)>,
    hole_seen: FxHashSet<(u32, u32)>,
    matched_a: FxHashSet<u32>,
    matched_b: FxHashSet<u32>,
    /// Set when recursion hit [`MAX_DEPTH`] — the witness fails closed.
    pub(super) truncated: bool,
    /// Set when the alignment saw an `await` wrapper on exactly one side (an async↔sync
    /// twin). The family is then a *transformation* (`async-mirror`), never
    /// `equal_modulo_holes` — async code is not behaviorally equal to its sync twin.
    pub(super) async_mirror: bool,
}

impl<'a> Au<'a> {
    pub(super) fn new(a: &'a Dag<'a>, b: &'a Dag<'a>) -> Self {
        Au {
            a,
            b,
            visited: FxHashSet::default(),
            holes: Vec::new(),
            hole_seen: FxHashSet::default(),
            matched_a: FxHashSet::default(),
            matched_b: FxHashSet::default(),
            truncated: false,
            async_mirror: false,
        }
    }

    fn mark(matched: &mut FxHashSet<u32>, nodes: &[nose_normalize::VgNode], root: u32) {
        let mut stack = vec![root];
        while let Some(v) = stack.pop() {
            if !matched.insert(v) {
                continue;
            }
            stack.extend(nodes[v as usize].args.iter().copied());
        }
    }

    fn mark_subtree(&mut self, x: u32, y: u32) {
        Self::mark(&mut self.matched_a, &self.a.dag.nodes, x);
        Self::mark(&mut self.matched_b, &self.b.dag.nodes, y);
    }

    fn hole(&mut self, x: u32, y: u32) {
        if self.hole_seen.insert((x, y)) {
            self.holes.push((x, y));
        }
    }

    fn one_sided(&mut self, node: u32, on_a: bool) {
        let key = if on_a { (node, NONE) } else { (NONE, node) };
        if self.hole_seen.insert(key) {
            self.holes.push(key);
        }
    }

    /// Flatten a commutative-operator chain rooted at `root` (same `key`) into its leaf
    /// operands, so two chains can be matched as multisets rather than positionally.
    fn flatten(dag: &Dag<'_>, root: u32, key: u64, out: &mut Vec<u32>) {
        let mut stack = vec![root];
        while let Some(v) = stack.pop() {
            let n = &dag.dag.nodes[v as usize];
            if n.op == VgOp::Bin && n.key == key {
                for &a in n.args.iter().rev() {
                    stack.push(a);
                }
            } else {
                out.push(v);
            }
        }
    }

    pub(super) fn unify(&mut self, x: u32, y: u32, depth: u32) {
        if depth > MAX_DEPTH {
            self.truncated = true;
            return;
        }
        if !self.visited.insert((x, y)) {
            return;
        }
        // Snapshot the two nodes' identity into scalars so no `&self.a/b` borrow is held across
        // the `&mut self` recursive calls below (the async-mirror gate recurses mid-node).
        let (x_hash, x_op, x_key, x_arity, x_arg0) = {
            let nx = &self.a.dag.nodes[x as usize];
            (
                nx.hash,
                nx.op,
                nx.key,
                nx.args.len(),
                nx.args.first().copied(),
            )
        };
        let (y_hash, y_op, y_key, y_arity, y_arg0) = {
            let ny = &self.b.dag.nodes[y as usize];
            (
                ny.hash,
                ny.op,
                ny.key,
                ny.args.len(),
                ny.args.first().copied(),
            )
        };
        if x_hash == y_hash {
            self.mark_subtree(x, y);
            return;
        }
        // async-mirror: `await e` (kept as `Opaque(VG_PROTOCOL_AWAIT,[e])` in the witness build)
        // aligns with the bare operand on the sync-twin side. Recurse into the operand so the
        // alignment propagates downstream, and record the await itself as a one-sided hole. This
        // never fires when BOTH sides await (both wrappers fall through to the same-op recurse).
        let x_await = x_op == VgOp::Opaque && x_key == VG_PROTOCOL_AWAIT && x_arity == 1;
        let y_await = y_op == VgOp::Opaque && y_key == VG_PROTOCOL_AWAIT && y_arity == 1;
        if x_await != y_await {
            self.async_mirror = true;
            if x_await {
                self.one_sided(x, true);
                self.unify(x_arg0.unwrap_or(x), y, depth + 1);
            } else {
                self.one_sided(y, false);
                self.unify(x, y_arg0.unwrap_or(y), depth + 1);
            }
            return;
        }
        if x_op != y_op || x_key != y_key {
            self.hole(x, y);
            return;
        }
        // Same op and key.
        if x_op == VgOp::Bin && bin_is_commutative(x_key) {
            self.unify_commutative(x, y, depth);
            return;
        }
        if x_arity != y_arity {
            self.hole(x, y);
            return;
        }
        self.matched_a.insert(x);
        self.matched_b.insert(y);
        // Recurse on each aligned argument pair. Index rather than clone the two arg
        // vecs: this is the per-node hot path, and the immutable borrow of each node is
        // released before the `&mut self` recursive call.
        let arity = self.a.dag.nodes[x as usize].args.len();
        for i in 0..arity {
            let cx = self.a.dag.nodes[x as usize].args[i];
            let cy = self.b.dag.nodes[y as usize].args[i];
            self.unify(cx, cy, depth + 1);
        }
    }

    /// Align a commutative chain: pair identical leaves by hash multiset, recurse on
    /// the hash-sorted leftovers, and count arity gaps as one-sided holes.
    fn unify_commutative(&mut self, x: u32, y: u32, depth: u32) {
        let key = self.a.dag.nodes[x as usize].key;
        let (mut la, mut lb) = (Vec::new(), Vec::new());
        Self::flatten(self.a, x, key, &mut la);
        Self::flatten(self.b, y, key, &mut lb);
        self.matched_a.insert(x);
        self.matched_b.insert(y);
        let mut by_hash: FxHashMap<u64, Vec<u32>> = FxHashMap::default();
        for &l in &lb {
            by_hash
                .entry(self.b.dag.nodes[l as usize].hash)
                .or_default()
                .push(l);
        }
        let mut rest_a: Vec<u32> = Vec::new();
        for &l in &la {
            let h = self.a.dag.nodes[l as usize].hash;
            if let Some(m) = by_hash.get_mut(&h).and_then(Vec::pop) {
                self.mark_subtree(l, m);
            } else {
                rest_a.push(l);
            }
        }
        let mut rest_b: Vec<u32> = by_hash.into_values().flatten().collect();
        rest_a.sort_unstable_by_key(|&l| self.a.dag.nodes[l as usize].hash);
        rest_b.sort_unstable_by_key(|&l| self.b.dag.nodes[l as usize].hash);
        let common = rest_a.len().min(rest_b.len());
        for i in 0..common {
            self.unify(rest_a[i], rest_b[i], depth + 1);
        }
        for &l in &rest_a[common..] {
            self.one_sided(l, true);
        }
        for &l in &rest_b[common..] {
            self.one_sided(l, false);
        }
    }
}
