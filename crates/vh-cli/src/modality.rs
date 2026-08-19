//! `AUTHORITY_CANNOT_LIFT_MODALITY_V1`
//!
//! Authority and epistemic modality are orthogonal. A human merge, operator
//! grant, or external confirmation may change `Authority` and must preserve
//! `Modality`. Promoting modality requires a witness whose kind matches the
//! exact adjacent step and whose revision equals the claim scope.
//!
//! This module is a pure evaluator. It does not read git, GitHub, or the
//! filesystem, and it cannot mint a `Proven` standing.

use std::fmt;

/// Epistemic standing of a claim. Ordered only so adjacent steps are obvious;
/// the order is not a license to skip.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Modality {
    Proposed,
    Documented,
    Implemented,
    Observed,
    Replayed,
    Proven,
}

/// Who ratified the claim as a social/legal act. Never a substitute for
/// observation, replay, or implementation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Authority {
    Unratified,
    HumanMerged,
    OperatorAuthorized,
    ExternalConfirmed,
}

/// What a promotion witness is allowed to attest.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WitnessKind {
    DocumentArtifact,
    Implementation,
    EngineObservation,
    StandaloneReplay,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Witness {
    pub kind: WitnessKind,
    pub revision: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Claim {
    pub value: String,
    pub scope_revision: String,
    pub modality: Modality,
    pub authority: Authority,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reject {
    MissingWitness,
    MissingImplementationWitness,
    WitnessKindMismatch {
        needed: WitnessKind,
        got: WitnessKind,
    },
    RevisionMismatch,
    NoSuchPromotion,
    ProvenNotConstructible,
}

impl Reject {
    pub const fn code(self) -> &'static str {
        match self {
            Self::MissingWitness => "MISSING_WITNESS",
            Self::MissingImplementationWitness => "MISSING_IMPLEMENTATION_WITNESS",
            Self::WitnessKindMismatch { .. } => "WITNESS_KIND_MISMATCH",
            Self::RevisionMismatch => "REVISION_MISMATCH",
            Self::NoSuchPromotion => "NO_SUCH_PROMOTION",
            Self::ProvenNotConstructible => "PROVEN_NOT_CONSTRUCTIBLE",
        }
    }
}

impl fmt::Display for Reject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

const PR91_SCOPE: &str = "3510883c9a4c0e8f7b2d1a6c5e4f3b2a1c0d9e8f";

/// Adjacent step required to move `from` to `to`. `Proven` has no v1 witness.
pub fn required_witness(from: Modality, to: Modality) -> Result<WitnessKind, Reject> {
    use Modality::*;
    use WitnessKind::*;
    match (from, to) {
        (Proposed, Documented) => Ok(DocumentArtifact),
        (Documented, Implemented) => Ok(Implementation),
        (Implemented, Observed) => Ok(EngineObservation),
        (Observed, Replayed) => Ok(StandaloneReplay),
        (_, Proven) => Err(Reject::ProvenNotConstructible),
        _ => Err(Reject::NoSuchPromotion),
    }
}

/// Raise modality by one adjacent step when `witness` matches. Authority is
/// copied, never inferred from the witness.
pub fn promote(claim: &Claim, to: Modality, witness: Option<&Witness>) -> Result<Claim, Reject> {
    let needed = required_witness(claim.modality, to)?;
    let Some(witness) = witness else {
        return Err(if needed == WitnessKind::Implementation {
            Reject::MissingImplementationWitness
        } else {
            Reject::MissingWitness
        });
    };
    if witness.kind != needed {
        return Err(Reject::WitnessKindMismatch {
            needed,
            got: witness.kind,
        });
    }
    if witness.revision != claim.scope_revision {
        return Err(Reject::RevisionMismatch);
    }
    Ok(Claim {
        modality: to,
        ..claim.clone()
    })
}

/// Change authority only. Modality is invariant.
pub fn change_authority(claim: &Claim, to: Authority) -> Claim {
    Claim {
        authority: to,
        ..claim.clone()
    }
}

/// The PR 91 / issue 90 controller: merged documentation is not implementation.
pub fn pr91_controller_claim() -> Claim {
    Claim {
        value: "issue 90 bridge contract".to_string(),
        scope_revision: PR91_SCOPE.to_string(),
        modality: Modality::Documented,
        authority: Authority::HumanMerged,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn documented() -> Claim {
        pr91_controller_claim()
    }

    fn impl_witness(rev: &str) -> Witness {
        Witness {
            kind: WitnessKind::Implementation,
            revision: rev.to_string(),
        }
    }

    #[test]
    fn authority_cannot_lift_modality_pr91_fixture() {
        let claim = documented();
        let err = promote(&claim, Modality::Implemented, None).unwrap_err();
        assert_eq!(err, Reject::MissingImplementationWitness);
        assert_eq!(err.code(), "MISSING_IMPLEMENTATION_WITNESS");
        assert_eq!(claim.modality, Modality::Documented);
        assert_eq!(claim.authority, Authority::HumanMerged);
    }

    #[test]
    fn authority_cannot_lift_modality_change_authority_preserves_m() {
        let claim = documented();
        for next in [
            Authority::Unratified,
            Authority::HumanMerged,
            Authority::OperatorAuthorized,
            Authority::ExternalConfirmed,
        ] {
            let out = change_authority(&claim, next);
            assert_eq!(out.modality, Modality::Documented, "{next:?}");
            assert_eq!(out.authority, next);
            assert_eq!(out.scope_revision, claim.scope_revision);
        }
    }

    #[test]
    fn authority_cannot_lift_modality_document_artifact_is_not_impl() {
        let claim = documented();
        let witness = Witness {
            kind: WitnessKind::DocumentArtifact,
            revision: claim.scope_revision.clone(),
        };
        let err = promote(&claim, Modality::Implemented, Some(&witness)).unwrap_err();
        assert_eq!(
            err,
            Reject::WitnessKindMismatch {
                needed: WitnessKind::Implementation,
                got: WitnessKind::DocumentArtifact,
            }
        );
    }

    #[test]
    fn authority_cannot_lift_modality_impl_witness_keeps_authority() {
        let claim = documented();
        let out = promote(
            &claim,
            Modality::Implemented,
            Some(&impl_witness(&claim.scope_revision)),
        )
        .unwrap();
        assert_eq!(out.modality, Modality::Implemented);
        assert_eq!(out.authority, Authority::HumanMerged);
    }

    #[test]
    fn authority_cannot_lift_modality_revision_must_match() {
        let claim = documented();
        let err = promote(
            &claim,
            Modality::Implemented,
            Some(&impl_witness("deadbeefdeadbeefdeadbeefdeadbeefdeadbeef")),
        )
        .unwrap_err();
        assert_eq!(err, Reject::RevisionMismatch);
    }

    #[test]
    fn authority_cannot_lift_modality_no_skipping() {
        let claim = documented();
        let err = promote(
            &claim,
            Modality::Observed,
            Some(&Witness {
                kind: WitnessKind::EngineObservation,
                revision: claim.scope_revision.clone(),
            }),
        )
        .unwrap_err();
        assert_eq!(err, Reject::NoSuchPromotion);
    }

    #[test]
    fn authority_cannot_lift_modality_no_same_or_lower() {
        let claim = documented();
        assert_eq!(
            promote(&claim, Modality::Documented, None).unwrap_err(),
            Reject::NoSuchPromotion
        );
        assert_eq!(
            promote(&claim, Modality::Proposed, None).unwrap_err(),
            Reject::NoSuchPromotion
        );
    }

    #[test]
    fn authority_cannot_lift_modality_proven_not_constructible() {
        let claim = Claim {
            value: "replayed run".into(),
            scope_revision: "aa".repeat(20),
            modality: Modality::Replayed,
            authority: Authority::ExternalConfirmed,
        };
        let witness = Witness {
            kind: WitnessKind::StandaloneReplay,
            revision: claim.scope_revision.clone(),
        };
        let err = promote(&claim, Modality::Proven, Some(&witness)).unwrap_err();
        assert_eq!(err, Reject::ProvenNotConstructible);
    }

    #[test]
    fn authority_cannot_lift_modality_adjacent_ladder() {
        let mut claim = Claim {
            value: "ladder".into(),
            scope_revision: "bb".repeat(20),
            modality: Modality::Proposed,
            authority: Authority::Unratified,
        };
        let steps = [
            (Modality::Documented, WitnessKind::DocumentArtifact),
            (Modality::Implemented, WitnessKind::Implementation),
            (Modality::Observed, WitnessKind::EngineObservation),
            (Modality::Replayed, WitnessKind::StandaloneReplay),
        ];
        for (to, kind) in steps {
            claim = promote(
                &claim,
                to,
                Some(&Witness {
                    kind,
                    revision: claim.scope_revision.clone(),
                }),
            )
            .unwrap();
            assert_eq!(claim.modality, to);
            assert_eq!(claim.authority, Authority::Unratified);
        }
    }

    #[test]
    fn authority_cannot_lift_modality_missing_non_impl_witness() {
        let claim = Claim {
            value: "impl".into(),
            scope_revision: "cc".repeat(20),
            modality: Modality::Implemented,
            authority: Authority::OperatorAuthorized,
        };
        let err = promote(&claim, Modality::Observed, None).unwrap_err();
        assert_eq!(err, Reject::MissingWitness);
    }

    #[test]
    fn authority_cannot_lift_modality_closed_table() {
        use Modality::*;
        let all = [
            Proposed,
            Documented,
            Implemented,
            Observed,
            Replayed,
            Proven,
        ];
        for from in all {
            for to in all {
                let got = required_witness(from, to);
                match (from, to) {
                    (Proposed, Documented) => {
                        assert_eq!(got, Ok(WitnessKind::DocumentArtifact));
                    }
                    (Documented, Implemented) => {
                        assert_eq!(got, Ok(WitnessKind::Implementation));
                    }
                    (Implemented, Observed) => {
                        assert_eq!(got, Ok(WitnessKind::EngineObservation));
                    }
                    (Observed, Replayed) => {
                        assert_eq!(got, Ok(WitnessKind::StandaloneReplay));
                    }
                    (_, Proven) => assert_eq!(got, Err(Reject::ProvenNotConstructible)),
                    _ => assert_eq!(got, Err(Reject::NoSuchPromotion)),
                }
            }
        }
    }
}
