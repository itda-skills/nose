use super::super::*;

impl<'a> Builder<'a> {
    /// Whether `target` appears anywhere in `v`'s value subgraph (DAG-safe).
    pub(in crate::value_graph) fn references(&self, v: ValueId, target: ValueId) -> bool {
        let mut stack = vec![v];
        let mut seen = FxHashSet::default();
        while let Some(x) = stack.pop() {
            if x == target {
                return true;
            }
            if seen.insert(x) {
                stack.extend(self.nodes[x as usize].args.iter().copied());
            }
        }
        false
    }
    pub(in crate::value_graph) fn references_cached(
        &self,
        v: ValueId,
        target: ValueId,
        cache: &mut ReductionCache,
    ) -> bool {
        let key = (v, target);
        if let Some(&cached) = cache.references.get(&key) {
            return cached;
        }
        let result = self.references(v, target);
        cache.references.insert(key, result);
        result
    }
    pub(in crate::value_graph) fn references_any_cached(
        &self,
        v: ValueId,
        targets: &[ValueId],
        cache: &mut ReductionCache,
    ) -> bool {
        targets
            .iter()
            .copied()
            .any(|target| self.references_cached(v, target, cache))
    }
}
