use nose_il::UnitKind;
use std::time::Instant;

pub(super) struct UnitTimer {
    sample_enabled: bool,
    summary_enabled: bool,
    summary: UnitTimingSummary,
}

impl UnitTimer {
    pub(super) fn new() -> Self {
        let sample_enabled = std::env::var_os("NOSE_TIME_UNITS").is_some();
        let summary_enabled = std::env::var_os("NOSE_TIME_UNIT_SUMMARY").is_some();
        Self {
            sample_enabled,
            summary_enabled,
            summary: UnitTimingSummary::default(),
        }
    }

    pub(super) fn start(&self) -> Option<Instant> {
        (self.sample_enabled || self.summary_enabled).then(Instant::now)
    }

    pub(super) fn elapsed(start: Option<Instant>) -> Option<f64> {
        start.map(|t| t.elapsed().as_secs_f64() * 1e3)
    }

    pub(super) fn report_skip(&mut self, sample: UnitTimingSkipSample<'_>) {
        let Some(start) = sample.start else {
            return;
        };
        let total_ms = start.elapsed().as_secs_f64() * 1e3;
        self.summary.record_skip(
            sample.kind,
            sample.tokens,
            total_ms,
            sample.pre_ms,
            sample.safe_ms,
            sample.value_ms,
        );
        if self.sample_enabled && total_ms >= 10.0 {
            let ms = |value: Option<f64>| {
                value
                    .map(|value| format!("{value:.1}ms"))
                    .unwrap_or_else(|| "-".to_string())
            };
            eprintln!(
                "  [unit] skip {:?} {}:{}-{} tokens={} pre={} safe={} value={} total={:.1}ms",
                sample.kind,
                sample.path,
                sample.start_line,
                sample.end_line,
                sample.tokens,
                ms(sample.pre_ms),
                ms(sample.safe_ms),
                ms(sample.value_ms),
                total_ms,
            );
        }
    }

    pub(super) fn report_keep(&mut self, sample: UnitTimingSample<'_>) {
        let (Some(start), Some(pre_ms), Some(safe_ms), Some(value_ms), Some(feature_start)) = (
            sample.start,
            sample.pre_ms,
            sample.safe_ms,
            sample.value_ms,
            sample.feature_start,
        ) else {
            return;
        };
        let feature_ms = feature_start.elapsed().as_secs_f64() * 1e3;
        let total_ms = start.elapsed().as_secs_f64() * 1e3;
        self.summary.record_keep(
            sample.kind,
            sample.tokens,
            sample.value_atoms,
            total_ms,
            pre_ms,
            safe_ms,
            value_ms,
            feature_ms,
        );
        if self.sample_enabled && total_ms >= 10.0 {
            eprintln!(
                "  [unit] keep {:?} {} {}:{}-{} tokens={} value_atoms={} pre={pre_ms:.1}ms safe={safe_ms:.1}ms value={value_ms:.1}ms features={feature_ms:.1}ms total={total_ms:.1}ms",
                sample.kind,
                sample.name,
                sample.path,
                sample.start_line,
                sample.end_line,
                sample.tokens,
                sample.value_atoms,
            );
        }
    }

    pub(super) fn report_summary(&self, path: &str) {
        if self.summary_enabled {
            self.summary.report(path);
        }
    }
}

#[derive(Clone, Copy, Default)]
struct UnitTimingBucket {
    seen: usize,
    kept: usize,
    skipped: usize,
    tokens: usize,
    value_atoms: usize,
    total_ms: f64,
    pre_ms: f64,
    safe_ms: f64,
    value_ms: f64,
    feature_ms: f64,
}

#[derive(Default)]
struct UnitTimingSummary {
    buckets: [UnitTimingBucket; 4],
}

impl UnitTimingSummary {
    fn bucket_mut(&mut self, kind: &UnitKind) -> &mut UnitTimingBucket {
        &mut self.buckets[unit_kind_index(kind)]
    }

    fn record_skip(
        &mut self,
        kind: &UnitKind,
        tokens: usize,
        total_ms: f64,
        pre_ms: Option<f64>,
        safe_ms: Option<f64>,
        value_ms: Option<f64>,
    ) {
        let bucket = self.bucket_mut(kind);
        bucket.seen += 1;
        bucket.skipped += 1;
        bucket.tokens += tokens;
        bucket.total_ms += total_ms;
        bucket.pre_ms += pre_ms.unwrap_or(0.0);
        bucket.safe_ms += safe_ms.unwrap_or(0.0);
        bucket.value_ms += value_ms.unwrap_or(0.0);
    }

    #[allow(clippy::too_many_arguments)]
    fn record_keep(
        &mut self,
        kind: &UnitKind,
        tokens: usize,
        value_atoms: usize,
        total_ms: f64,
        pre_ms: f64,
        safe_ms: f64,
        value_ms: f64,
        feature_ms: f64,
    ) {
        let bucket = self.bucket_mut(kind);
        bucket.seen += 1;
        bucket.kept += 1;
        bucket.tokens += tokens;
        bucket.value_atoms += value_atoms;
        bucket.total_ms += total_ms;
        bucket.pre_ms += pre_ms;
        bucket.safe_ms += safe_ms;
        bucket.value_ms += value_ms;
        bucket.feature_ms += feature_ms;
    }

    fn report(&self, path: &str) {
        let total_ms: f64 = self.buckets.iter().map(|bucket| bucket.total_ms).sum();
        if total_ms < 10.0 {
            return;
        }
        for (kind, bucket) in [
            (UnitKind::Function, self.buckets[0]),
            (UnitKind::Method, self.buckets[1]),
            (UnitKind::Class, self.buckets[2]),
            (UnitKind::Block, self.buckets[3]),
        ] {
            if bucket.seen == 0 {
                continue;
            }
            eprintln!(
                "  [unit-summary] {:?} {} seen={} kept={} skipped={} tokens={} value_atoms={} total={:.1}ms pre={:.1}ms safe={:.1}ms value={:.1}ms features={:.1}ms",
                kind,
                path,
                bucket.seen,
                bucket.kept,
                bucket.skipped,
                bucket.tokens,
                bucket.value_atoms,
                bucket.total_ms,
                bucket.pre_ms,
                bucket.safe_ms,
                bucket.value_ms,
                bucket.feature_ms,
            );
        }
    }
}

fn unit_kind_index(kind: &UnitKind) -> usize {
    match kind {
        UnitKind::Function => 0,
        UnitKind::Method => 1,
        UnitKind::Class => 2,
        UnitKind::Block => 3,
    }
}

pub(super) struct UnitTimingSample<'a> {
    pub(super) start: Option<Instant>,
    pub(super) feature_start: Option<Instant>,
    pub(super) kind: &'a UnitKind,
    pub(super) name: &'a str,
    pub(super) path: &'a str,
    pub(super) start_line: u32,
    pub(super) end_line: u32,
    pub(super) tokens: usize,
    pub(super) value_atoms: usize,
    pub(super) pre_ms: Option<f64>,
    pub(super) safe_ms: Option<f64>,
    pub(super) value_ms: Option<f64>,
}

pub(super) struct UnitTimingSkipSample<'a> {
    pub(super) start: Option<Instant>,
    pub(super) kind: &'a UnitKind,
    pub(super) path: &'a str,
    pub(super) start_line: u32,
    pub(super) end_line: u32,
    pub(super) tokens: usize,
    pub(super) pre_ms: Option<f64>,
    pub(super) safe_ms: Option<f64>,
    pub(super) value_ms: Option<f64>,
}
