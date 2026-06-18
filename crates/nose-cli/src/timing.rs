/// Time a named CLI-side stage under `NOSE_TIME` (the in-pipeline detector stages
/// report themselves; this covers post-detection CLI work — lowering, the graded-witness
/// enrichment — that is otherwise invisible).
pub(crate) fn time_stage<T>(label: &str, f: impl FnOnce() -> T) -> T {
    if std::env::var_os("NOSE_TIME").is_none() {
        return f();
    }
    let t0 = std::time::Instant::now();
    let out = f();
    eprintln!(
        "  [time] {:<12} {:>7.1}ms",
        label,
        t0.elapsed().as_secs_f64() * 1e3
    );
    out
}

/// Run the corpus discover+parse+lower step, printing its wall time under
/// `NOSE_TIME` (the in-pipeline stages report themselves; this covers the
/// frontend, which usually dominates and is otherwise invisible).
pub(crate) fn time_lower<T>(f: impl FnOnce() -> T) -> T {
    time_stage("lower", f)
}
