//! vh-props — the property system.
//!
//! Two assertion classes (the third, end-state oracles, lands in Phase 2):
//!
//! * `always(name, cond)` — an invariant. One violation in one universe is a
//!   finding.
//! * `sometimes(name)` — a reachability assertion, evaluated across the whole
//!   multiverse: if NO universe ever hits it, the property fails. This is the
//!   Antithesis-style check that catches vibe-code's signature failure mode —
//!   error paths that are dead code.
//!
//! Uses BTreeMap everywhere — never a hash-ordered map: iteration order is
//! part of the deterministic surface.

use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlwaysFailure {
    pub name: String,
    pub detail: String,
}

#[derive(Debug, Clone, Default)]
pub struct Properties {
    always_failures: Vec<AlwaysFailure>,
    sometimes: BTreeMap<String, bool>,
}

impl Properties {
    pub fn new() -> Self {
        Self::default()
    }

    /// Check an invariant. `detail` is only rendered on failure.
    pub fn always<F: FnOnce() -> String>(&mut self, name: &str, condition: bool, detail: F) {
        if !condition {
            self.always_failures.push(AlwaysFailure {
                name: name.to_string(),
                detail: detail(),
            });
        }
    }

    /// Declare a sometimes-assertion without hitting it. Declare every
    /// sometimes up front so an unreached one is visible, not absent.
    pub fn declare_sometimes(&mut self, name: &str) {
        self.sometimes.entry(name.to_string()).or_insert(false);
    }

    /// Mark a sometimes-assertion as reached in this universe.
    pub fn sometimes(&mut self, name: &str) {
        self.sometimes.insert(name.to_string(), true);
    }

    pub fn always_failures(&self) -> &[AlwaysFailure] {
        &self.always_failures
    }

    pub fn sometimes_map(&self) -> &BTreeMap<String, bool> {
        &self.sometimes
    }
}

/// Multiverse-level merge: always-failures accumulate (tagged by universe),
/// a sometimes passes if ANY universe reached it.
#[derive(Debug, Clone, Default)]
pub struct MergedProperties {
    pub always_failures: Vec<(u64, AlwaysFailure)>,
    pub sometimes: BTreeMap<String, bool>,
}

impl MergedProperties {
    pub fn absorb(&mut self, universe_id: u64, props: &Properties) {
        for f in props.always_failures() {
            self.always_failures.push((universe_id, f.clone()));
        }
        for (name, hit) in props.sometimes_map() {
            let entry = self.sometimes.entry(name.clone()).or_insert(false);
            *entry = *entry || *hit;
        }
    }

    pub fn unreached_sometimes(&self) -> Vec<&str> {
        self.sometimes
            .iter()
            .filter(|(_, hit)| !**hit)
            .map(|(name, _)| name.as_str())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn always_records_failures_only() {
        let mut p = Properties::new();
        p.always("ok", true, || unreachable!("detail must be lazy"));
        p.always("broken", false, || "x was 3".to_string());
        assert_eq!(p.always_failures().len(), 1);
        assert_eq!(p.always_failures()[0].name, "broken");
    }

    #[test]
    fn sometimes_merges_across_universes() {
        let mut a = Properties::new();
        a.declare_sometimes("crash_seen");
        let mut b = Properties::new();
        b.declare_sometimes("crash_seen");
        b.sometimes("crash_seen");

        let mut merged = MergedProperties::default();
        merged.absorb(0, &a);
        merged.absorb(1, &b);
        assert!(merged.sometimes["crash_seen"]);
        assert!(merged.unreached_sometimes().is_empty());
    }

    #[test]
    fn unreached_sometimes_is_a_finding() {
        let mut a = Properties::new();
        a.declare_sometimes("error_path_taken");
        let mut merged = MergedProperties::default();
        merged.absorb(0, &a);
        assert_eq!(merged.unreached_sometimes(), vec!["error_path_taken"]);
    }
}
