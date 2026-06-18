use nose_normalize::{ValueDag, VgSinkKind};

/// A value DAG with the two derived per-node facts the alignment needs: tree weight
/// (subtree size, for ranking a hole's mass) and whether a node feeds an ordered
/// effect sink (so a hole there is behaviorally load-bearing).
pub(super) struct Dag<'a> {
    pub(super) dag: &'a ValueDag,
    pub(super) weight: Vec<u32>,
    pub(super) effectish: Vec<bool>,
}

impl<'a> Dag<'a> {
    pub(super) fn new(dag: &'a ValueDag) -> Self {
        let n = dag.nodes.len();
        // Args reference earlier indices (hash-consed), so one forward pass suffices.
        let mut weight = vec![0u32; n];
        for i in 0..n {
            let mut w: u64 = 1;
            for &a in &dag.nodes[i].args {
                w += u64::from(weight[a as usize]);
            }
            weight[i] = u32::try_from(w).unwrap_or(u32::MAX);
        }
        let mut effectish = vec![false; n];
        let mut stack: Vec<u32> = dag
            .sinks
            .iter()
            .filter(|s| {
                matches!(
                    s.kind,
                    VgSinkKind::Effect | VgSinkKind::Break | VgSinkKind::Throw
                )
            })
            .map(|s| s.value)
            .collect();
        while let Some(v) = stack.pop() {
            if effectish[v as usize] {
                continue;
            }
            effectish[v as usize] = true;
            stack.extend(dag.nodes[v as usize].args.iter().copied());
        }
        Dag {
            dag,
            weight,
            effectish,
        }
    }
}
