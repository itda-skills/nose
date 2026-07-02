use crate::verify_admission::AdmissionContext;
use nose_il::{Interner, NodeId, NodeKind};

/// Map C# `System.Threading.Tasks` runtime calls to the shared scheduling
/// obligation vocabulary (reporting-only — a runtime boundary is still a
/// strict-exact rejection). Mirrors the Swift structured-concurrency mapping:
/// `Task.Run`/`Task.Factory.StartNew` spawn, `Task.Delay` is the `Task.sleep`
/// twin, `Task.Yield` re-schedules, and `Task.WhenAll`/`Task.WhenAny` are the
/// all/first aggregates. Attribution requires the `Task` root to be unshadowed
/// by any C# definition visible in the scanned corpus (fail-closed).
pub(super) fn push_csharp_async_runtime_call_missing_evidence(
    il: &nose_il::Il,
    interner: &Interner,
    callee: NodeId,
    callee_path: &str,
    context: &AdmissionContext,
    labels: &mut Vec<&'static str>,
) -> bool {
    match callee_path {
        "Task.Run" | "Task.Factory.StartNew"
            if csharp_task_runtime_root_unshadowed(il, interner, callee, context) =>
        {
            super::push_task_spawn_missing_evidence(labels);
            true
        }
        "Task.Delay" if csharp_task_runtime_root_unshadowed(il, interner, callee, context) => {
            super::super::push_unique(labels, "timer-scheduling-contract");
            super::super::push_unique(labels, "task-cancellation-liveness-contract");
            true
        }
        "Task.Yield" if csharp_task_runtime_root_unshadowed(il, interner, callee, context) => {
            super::super::push_unique(labels, "task-yield-scheduling-contract");
            true
        }
        "Task.WhenAll" if csharp_task_runtime_root_unshadowed(il, interner, callee, context) => {
            super::push_async_aggregate_all_missing_evidence(labels);
            true
        }
        "Task.WhenAny" if csharp_task_runtime_root_unshadowed(il, interner, callee, context) => {
            super::push_async_aggregate_first_missing_evidence(labels);
            true
        }
        _ => false,
    }
}

/// The base receiver of a (possibly chained) field access: `Task.Factory.StartNew`
/// → the `Task` variable node.
fn csharp_callee_root(il: &nose_il::Il, callee: NodeId) -> NodeId {
    let mut root = callee;
    while il.kind(root) == NodeKind::Field {
        let Some(receiver) = super::super::method_receiver(il, root) else {
            break;
        };
        root = receiver;
    }
    root
}

fn csharp_task_runtime_root_unshadowed(
    il: &nose_il::Il,
    interner: &Interner,
    callee: NodeId,
    context: &AdmissionContext,
) -> bool {
    let root = csharp_callee_root(il, callee);
    il.kind(root) == NodeKind::Var
        && super::super::node_defines_name(il, interner, root, "Task")
        && !context.csharp_name_is_visible("Task")
        && !nose_semantics::file_defines_name_visible_at(il, interner, "Task", il.node(root).span)
}
