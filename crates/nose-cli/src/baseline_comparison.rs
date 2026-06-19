use crate::legacy_prelude::*;

#[derive(serde::Serialize)]
pub(super) struct BaselineSummary {
    pub(super) path: String,
    pub(super) mode: &'static str,
    pub(super) baseline_families: usize,
    pub(super) new_families: usize,
    pub(super) changed_families: usize,
    pub(super) unchanged_families: usize,
    pub(super) resolved_families: usize,
}

impl BaselineSummary {
    pub(super) fn line(&self) -> String {
        format!(
            "baseline: {} new · {} changed · {} unchanged · {} resolved",
            self.new_families,
            self.changed_families,
            self.unchanged_families,
            self.resolved_families
        )
    }
}

#[derive(Clone, Copy)]
pub(super) enum BaselineStatus {
    New,
    Changed,
}

impl BaselineStatus {
    pub(super) fn as_str(&self) -> &'static str {
        match self {
            BaselineStatus::New => "new",
            BaselineStatus::Changed => "changed",
        }
    }
}

pub(super) struct BaselineComparison {
    pub(super) summary: BaselineSummary,
    pub(super) statuses: std::collections::HashMap<u64, BaselineStatus>,
}

impl BaselineComparison {
    pub(super) fn new(
        path: &std::path::Path,
        baseline: &baseline::Baseline,
        families: &[nose_detect::RefactorFamily],
    ) -> Self {
        let current_keys: std::collections::HashSet<u64> =
            families.iter().map(baseline::family_key).collect();
        let unchanged_families = baseline.keys.intersection(&current_keys).count();

        let mut changed_current = std::collections::HashSet::new();
        let mut changed_baseline = std::collections::HashSet::new();
        for family in families {
            let key = baseline::family_key(family);
            if baseline.keys.contains(&key) {
                continue;
            }
            let current_members = baseline::member_keys(family);
            if baseline
                .entries
                .iter()
                .filter(|entry| !current_keys.contains(&entry.key))
                .any(|entry| {
                    !entry.members.is_empty()
                        && baseline::member_sets_overlap(&entry.members, &current_members)
                })
            {
                changed_current.insert(key);
                for entry in baseline
                    .entries
                    .iter()
                    .filter(|entry| !current_keys.contains(&entry.key))
                {
                    if !entry.members.is_empty()
                        && baseline::member_sets_overlap(&entry.members, &current_members)
                    {
                        changed_baseline.insert(entry.key);
                    }
                }
            }
        }

        let mut statuses = std::collections::HashMap::new();
        for family in families {
            let key = baseline::family_key(family);
            if baseline.keys.contains(&key) {
                continue;
            }
            let status = if changed_current.contains(&key) {
                BaselineStatus::Changed
            } else {
                BaselineStatus::New
            };
            statuses.insert(key, status);
        }

        let resolved_families = baseline
            .keys
            .iter()
            .filter(|key| !current_keys.contains(key) && !changed_baseline.contains(key))
            .count();
        let changed_families = changed_current.len();
        let new_families = statuses.len().saturating_sub(changed_families);
        BaselineComparison {
            summary: BaselineSummary {
                path: path.display().to_string(),
                mode: "new-only",
                baseline_families: baseline.keys.len(),
                new_families,
                changed_families,
                unchanged_families,
                resolved_families,
            },
            statuses,
        }
    }
}
