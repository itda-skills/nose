//! Stage 2 — verification / ranking + commonness evidence.
//!
//! The top discriminator from the survey is IDF-weighted cosine: IDF down-weights the ubiquitous
//! grams that topically-related (sibling) sections share, which is **similarity-measurement
//! correctness**, not a worthiness judgment. We also expose **commonness** (how ubiquitous the
//! shared content is) as an orthogonal evidence field the user can filter on — we never silently
//! suppress real duplicates (boilerplate copies are true duplicates, surfaced with high
//! commonness). See epic #435 design-principle alignment.

use crate::fingerprint::Fingerprint;
use rustc_hash::FxHashMap;

struct WeightedFingerprint {
    weights: Vec<f64>,
    norm: f64,
}

pub(crate) struct PairStats {
    pub(crate) cosine: f64,
    pub(crate) containment: f64,
    pub(crate) shared: usize,
}

/// Corpus document-frequency model over the char-gram shingle space.
pub struct CorpusModel {
    n: usize,
    idf: FxHashMap<u64, f64>,
    df: FxHashMap<u64, u32>,
    weighted: Vec<WeightedFingerprint>,
}

impl CorpusModel {
    /// Fit DF/IDF over all unit fingerprints (shingles are per-unit de-duplicated sets, so this
    /// is document-frequency). IDF uses the standard smoothed form.
    pub fn fit(fps: &[Fingerprint]) -> CorpusModel {
        let n = fps.len();
        let mut df: FxHashMap<u64, u32> = FxHashMap::default();
        for fp in fps {
            for &g in &fp.shingles {
                *df.entry(g).or_default() += 1;
            }
        }
        let nf = n as f64;
        let idf: FxHashMap<u64, f64> = df
            .iter()
            .map(|(&g, &d)| {
                let d = d as f64;
                (g, ((nf - d + 0.5) / (d + 0.5) + 1.0).ln())
            })
            .collect();
        let unseen = ((n as f64) + 1.0).ln();
        let weighted = fps
            .iter()
            .map(|fp| {
                let weights: Vec<f64> = fp
                    .shingles
                    .iter()
                    .map(|&g| idf.get(&g).copied().unwrap_or(unseen))
                    .collect();
                let norm = weights.iter().map(|w| w.powi(2)).sum::<f64>().sqrt();
                WeightedFingerprint { weights, norm }
            })
            .collect();
        CorpusModel {
            n,
            idf,
            df,
            weighted,
        }
    }

    fn idf(&self, g: u64) -> f64 {
        self.idf
            .get(&g)
            .copied()
            .unwrap_or_else(|| ((self.n as f64) + 1.0).ln())
    }

    /// IDF-weighted cosine over the two shingle sets (binary tf). The survey's top relation
    /// discriminator and best topical-false-positive resistance.
    pub fn tfidf_cosine(&self, a: &Fingerprint, b: &Fingerprint) -> f64 {
        if a.shingles.is_empty() || b.shingles.is_empty() {
            return 0.0;
        }
        let (na, nb) = (self.norm(&a.shingles), self.norm(&b.shingles));
        if na == 0.0 || nb == 0.0 {
            return 0.0;
        }
        self.dot_slow(a, b) / (na * nb)
    }

    /// IDF-weighted cosine for fingerprints from this model's input slice. The per-unit vector
    /// norms and weights are cached during `fit`, avoiding a full shingle scan for every
    /// candidate pair.
    pub fn tfidf_cosine_at(&self, fps: &[Fingerprint], i: usize, j: usize) -> f64 {
        self.pair_stats_at(fps, i, j).cosine
    }

    /// Relation ingredients for two fingerprints from this model's input slice.
    /// Candidate verification touches many pairs in large documentation repos; computing
    /// cosine, containment, and shared-gram substance in one sorted-set pass avoids scanning
    /// the same long shingle vectors three times.
    pub(crate) fn pair_stats_at(&self, fps: &[Fingerprint], i: usize, j: usize) -> PairStats {
        let (a, b) = (&fps[i], &fps[j]);
        if a.shingles.is_empty() || b.shingles.is_empty() {
            return PairStats {
                cosine: 0.0,
                containment: 0.0,
                shared: 0,
            };
        }
        let (wa, wb) = (&self.weighted[i], &self.weighted[j]);
        let mut dot = 0.0;
        let mut shared = 0usize;
        let (mut ai, mut bi) = (0, 0);
        while ai < a.shingles.len() && bi < b.shingles.len() {
            match a.shingles[ai].cmp(&b.shingles[bi]) {
                std::cmp::Ordering::Less => ai += 1,
                std::cmp::Ordering::Greater => bi += 1,
                std::cmp::Ordering::Equal => {
                    dot += wa.weights[ai] * wb.weights[bi];
                    shared += 1;
                    ai += 1;
                    bi += 1;
                }
            }
        }
        let cosine = if wa.norm == 0.0 || wb.norm == 0.0 {
            0.0
        } else {
            dot / (wa.norm * wb.norm)
        };
        PairStats {
            cosine,
            containment: shared as f64 / a.shingles.len().min(b.shingles.len()) as f64,
            shared,
        }
    }

    fn norm(&self, s: &[u64]) -> f64 {
        s.iter().map(|&g| self.idf(g).powi(2)).sum::<f64>().sqrt()
    }

    fn dot_slow(&self, a: &Fingerprint, b: &Fingerprint) -> f64 {
        let mut dot = 0.0;
        let (mut i, mut j) = (0, 0);
        while i < a.shingles.len() && j < b.shingles.len() {
            match a.shingles[i].cmp(&b.shingles[j]) {
                std::cmp::Ordering::Less => i += 1,
                std::cmp::Ordering::Greater => j += 1,
                std::cmp::Ordering::Equal => {
                    dot += self.idf(a.shingles[i]).powi(2);
                    i += 1;
                    j += 1;
                }
            }
        }
        dot
    }

    /// Commonness of the content shared by `a` and `b`: mean document-frequency fraction of the
    /// shared grams, in 0..=1. High ⇒ the overlap is ubiquitous boilerplate (license/CoC/badges);
    /// low ⇒ distinctive shared content. Orthogonal evidence, NOT a suppression decision.
    pub fn commonness(&self, a: &Fingerprint, b: &Fingerprint) -> f64 {
        if self.n == 0 {
            return 0.0;
        }
        let (mut i, mut j) = (0, 0);
        let (mut sum, mut cnt) = (0.0_f64, 0usize);
        while i < a.shingles.len() && j < b.shingles.len() {
            match a.shingles[i].cmp(&b.shingles[j]) {
                std::cmp::Ordering::Less => i += 1,
                std::cmp::Ordering::Greater => j += 1,
                std::cmp::Ordering::Equal => {
                    let d = *self.df.get(&a.shingles[i]).unwrap_or(&1) as f64;
                    sum += d / self.n as f64;
                    cnt += 1;
                    i += 1;
                    j += 1;
                }
            }
        }
        if cnt == 0 {
            0.0
        } else {
            sum / cnt as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unit::split_units;

    fn fps(texts: &[&str]) -> Vec<Fingerprint> {
        texts
            .iter()
            .map(|t| Fingerprint::of(&split_units("t.md", t)[0]))
            .collect()
    }

    #[test]
    fn tfidf_high_for_near_dup_low_for_unrelated() {
        let f = fps(&[
            "the quick brown fox jumps over the lazy dog in the morning light today",
            "the quick brown fox leaps over a lazy dog in the morning light today",
            "database indexes accelerate lookups across very large partitioned tables",
        ]);
        let m = CorpusModel::fit(&f);
        let near = m.tfidf_cosine(&f[0], &f[1]);
        let far = m.tfidf_cosine(&f[0], &f[2]);
        assert!((near - m.tfidf_cosine_at(&f, 0, 1)).abs() < 1e-12);
        assert!((far - m.tfidf_cosine_at(&f, 0, 2)).abs() < 1e-12);
        // The property that matters is discrimination; absolute value is noisy on a 3-doc corpus.
        assert!(near > 0.4, "near={near}");
        assert!(far < near * 0.5, "far={far} near={near}");
    }

    #[test]
    fn commonness_higher_for_ubiquitous_overlap() {
        // "installation" boilerplate repeated across many docs vs a distinctive pair.
        let boiler = "installation run the standard install command then verify the version";
        let mut texts: Vec<String> = (0..8).map(|_| boiler.to_string()).collect();
        texts.push("a highly distinctive sentence about quantum entanglement experiments".into());
        texts.push("a highly distinctive sentence about quantum entanglement experiments".into());
        let refs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();
        let f = fps(&refs);
        let m = CorpusModel::fit(&f);
        let common = m.commonness(&f[0], &f[1]); // boilerplate pair
        let distinct = m.commonness(&f[8], &f[9]); // distinctive pair
        assert!(common > distinct, "common={common} distinct={distinct}");
    }
}
