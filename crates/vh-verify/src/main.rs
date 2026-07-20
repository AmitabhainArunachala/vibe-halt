#![forbid(unsafe_code)]

//! Boundary binary for the Tier-1 sequential replay soak.

use std::time::Instant;

use vh_verify::{
    format_error_receipt, format_receipt, replay_soak, ReplayPanicStage, ReplaySoakError,
};

#[cfg(feature = "ci-soak-200")]
const RUNS: usize = 200;
#[cfg(not(feature = "ci-soak-200"))]
const RUNS: usize = 1_000;

#[derive(Debug)]
struct SoakFailed;

fn catch_replay<T, F>(runs: usize, replay: F) -> Result<T, ReplaySoakError>
where
    F: FnOnce() -> Result<T, ReplaySoakError>,
{
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(replay)) {
        Ok(result) => result,
        Err(_) => Err(ReplaySoakError::Panicked {
            requested_runs: runs,
            stage: ReplayPanicStage::CliBoundary,
            run: None,
        }),
    }
}

fn main() -> Result<(), SoakFailed> {
    // Wall-clock throughput is boundary telemetry only; it never enters a
    // replay input, event, property, or trace hash.
    let started = Instant::now();
    let report = match catch_replay(RUNS, || replay_soak(RUNS)) {
        Ok(report) => report,
        Err(error) => {
            println!("{}", format_error_receipt(&error));
            eprintln!("Tier-1 replay soak failed: {error:?}");
            return Err(SoakFailed);
        }
    };
    let elapsed_nanos = started.elapsed().as_nanos().max(1);
    let universes_per_hour = (report.runs() as u128 * 3_600_000_000_000u128) / elapsed_nanos;

    println!("{}", format_receipt(&report, universes_per_hour));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panic_is_converted_to_a_frozen_error_receipt() {
        let error = catch_replay(7, || -> Result<(), ReplaySoakError> {
            panic!("panic-boundary fixture")
        })
        .expect_err("panic must fail closed");

        assert_eq!(error.code(), "panic");
        assert_eq!(error.requested_runs(), 7);
        assert_eq!(
            format_error_receipt(&error),
            "soak: receipt-schema=vh-verify-soak-v1 verdict=ERROR determinism-tier=Tier-1 evidence-grade=D0 root-seed=0x000000000000d1ce universe=0 workload=vh-verify-reference trace-format=v0 observable-schema=vh-verify-observable-v2 requested-runs=7 error-code=panic panic-stage=cli-boundary panic-run=boundary"
        );
    }
}
