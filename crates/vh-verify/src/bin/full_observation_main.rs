#![forbid(unsafe_code)]

//! Boundary binary for the V1 full-observation soak: replays the
//! verifier-owned SimRuntime reference (8 universes x 125 replays =
//! 1,000 Tier-1 replays) and prints the NORMALIZED full-observation
//! artifact to stdout. No arguments, no clock, no filesystem, no
//! environment — provenance is the CI workflow's separate artifact, so
//! this output is byte-comparable across the OS matrix.

use vh_verify::full_observation::{
    full_observation_artifact, SIM_REFERENCE_REPLAYS_PER_UNIVERSE, SIM_REFERENCE_UNIVERSES,
};

#[derive(Debug)]
struct SoakFailed;

fn main() -> Result<(), SoakFailed> {
    match full_observation_artifact(SIM_REFERENCE_UNIVERSES, SIM_REFERENCE_REPLAYS_PER_UNIVERSE) {
        Ok(artifact) => {
            print!("{artifact}");
            Ok(())
        }
        Err(error) => {
            println!("full-observation: verdict=ERROR {error}");
            eprintln!("V1 full-observation soak failed: {error}");
            Err(SoakFailed)
        }
    }
}
