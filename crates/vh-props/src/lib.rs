//! vh-props — the property system.
//!
//! Three assertion classes:
//!
//! * `always(name, cond)` — an invariant. One violation in one universe is a
//!   finding.
//! * `sometimes(name)` — a reachability assertion, evaluated across the whole
//!   multiverse: if NO universe ever hits it, the property fails. This is the
//!   Antithesis-style check that catches vibe-code's signature failure mode —
//!   error paths that are dead code.
//! * [`EndStateOracle`] — a typed post-run assertion over the workload's
//!   DECLARED end state (TigerBeetle-VOPR-style): the runner evaluates each
//!   oracle once after the workload returns and records exactly one entry
//!   named `oracle:<name>` into the always transcript. Oracles READ state;
//!   they never record trace events — a re-expressed check must leave the
//!   frozen trace identity untouched.
//!
//! Uses BTreeMap everywhere — never a hash-ordered map: iteration order is
//! part of the deterministic surface.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;

/// The workload's declared end state: deterministic key/value pairs
/// accumulated during the run for post-run oracle judgment.
pub type EndState = BTreeMap<String, String>;

/// A typed post-run assertion over the declared [`EndState`].
///
/// `check` returns `Ok(())` (recorded as a PASSING `oracle:<name>`
/// transcript entry) or `Err(detail)` (an always-failure named
/// `oracle:<name>` carrying the detail). Plain `fn` pointers keep the
/// oracle out of data equality — an oracle's identity is its NAME, which
/// property contracts require by name.
#[derive(Debug, Clone, Copy)]
pub struct EndStateOracle {
    pub name: &'static str,
    pub check: fn(&EndState) -> Result<(), String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlwaysFailure {
    pub name: String,
    pub detail: String,
}

/// One entry in the assertion transcript: every `always` evaluation is
/// recorded, PASSING ones included, in invocation order. Without this, a
/// replay that silently skips a passing invariant is observably equal to
/// one that evaluated it (PR #1 hardening-loop BLOCKER).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlwaysCheck {
    pub name: String,
    pub passed: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Properties {
    always_checks: Vec<AlwaysCheck>,
    always_failures: Vec<AlwaysFailure>,
    sometimes: BTreeMap<String, bool>,
}

impl Properties {
    pub fn new() -> Self {
        Self::default()
    }

    /// Check an invariant. `detail` is only rendered on failure. Every
    /// evaluation — pass or fail — enters the assertion transcript.
    pub fn always<F: FnOnce() -> String>(&mut self, name: &str, condition: bool, detail: F) {
        self.always_checks.push(AlwaysCheck {
            name: name.to_string(),
            passed: condition,
        });
        if !condition {
            self.always_failures.push(AlwaysFailure {
                name: name.to_string(),
                detail: detail(),
            });
        }
    }

    /// The full assertion transcript in invocation order.
    pub fn always_checks(&self) -> &[AlwaysCheck] {
        &self.always_checks
    }

    /// Declare a sometimes-assertion without hitting it. Declare every
    /// sometimes up front so an unreached one is visible, not absent.
    pub fn declare_sometimes(&mut self, name: &str) {
        self.sometimes.entry(name.to_string()).or_insert(false);
    }

    /// Mark a sometimes-assertion as reached in this universe.
    ///
    /// Fail-closed declaration discipline: hitting an undeclared name used
    /// to fabricate a reached property out of thin air, so a typo'd or
    /// undeclared hit could never surface as an unreached finding (PR #1
    /// hardening-loop-2 GAP). Declaration must precede the hit.
    pub fn sometimes(&mut self, name: &str) {
        assert!(
            self.sometimes.contains_key(name),
            "sometimes property must be declared before it is hit: {name}"
        );
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
    fn always_records_failures_and_full_transcript() {
        let mut p = Properties::new();
        p.always("ok", true, || unreachable!("detail must be lazy"));
        p.always("broken", false, || "x was 3".to_string());
        assert_eq!(p.always_failures().len(), 1);
        assert_eq!(p.always_failures()[0].name, "broken");
        // The transcript records BOTH evaluations, in order.
        assert_eq!(p.always_checks().len(), 2);
        assert_eq!(p.always_checks()[0].name, "ok");
        assert!(p.always_checks()[0].passed);
        assert_eq!(p.always_checks()[1].name, "broken");
        assert!(!p.always_checks()[1].passed);
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

    /// Negative regression (hardening-loop-2 GAP): the pre-repair kernel
    /// inserted an undeclared hit as `true`, silently fabricating a reached
    /// property that no declaration ever announced.
    #[test]
    #[should_panic(expected = "sometimes property must be declared before it is hit")]
    fn undeclared_sometimes_hit_fails_closed() {
        let mut p = Properties::new();
        p.sometimes("never_declared");
    }

    /// Redeclaring after a hit must not erase the reached state.
    #[test]
    fn redeclaration_does_not_erase_a_hit() {
        let mut p = Properties::new();
        p.declare_sometimes("reached");
        p.sometimes("reached");
        p.declare_sometimes("reached");
        assert_eq!(p.sometimes_map().get("reached"), Some(&true));
    }
}
