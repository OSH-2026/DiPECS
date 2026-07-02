//! aios-cli — internal tooling (replay / evaluate / golden-trace).
//!
//! The library entry exposes pipeline modules so integration tests can drive
//! them without spawning the binary.

pub mod android_bridge;
pub mod next_app_benchmark;
pub mod replay;
