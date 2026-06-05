#![warn(clippy::pedantic)]
#![warn(clippy::unwrap_used)]
#![warn(clippy::expect_used)]
#![warn(clippy::todo)]
#![allow(clippy::module_name_repetitions)]

//! Lab4 measurement and data utilities.
//!
//! The crate keeps experiment automation small and explicit: prompt validation,
//! `llama.cpp` command execution, JSONL records, storage timing, and summary
//! statistics. It does not hide model paths, Ceph paths, or RPC endpoints.

pub mod command;
pub mod env;
pub mod error;
pub mod jsonl;
pub mod llama;
pub mod prompt;
pub mod stats;
pub mod storage;
