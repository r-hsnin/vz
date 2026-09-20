//! vz — CLI BI tool with smart visualization and terminal presentation.
//!
//! This library crate re-exports the public modules for use by benchmarks and tests.

pub mod chart;
pub mod cli;
pub mod diagnostics;
pub mod diff;
pub mod directory;
pub mod explore;
pub mod filter;
pub mod infer;
pub mod info;
pub mod insights;
pub mod loader;
pub mod oneshot;
pub mod output;
pub mod pipeline;
pub mod present;
pub mod render;
pub mod sparkline;
pub mod theme;
pub mod util;
pub mod watch;

mod app;
pub use app::{apply_output_shorthands, run};

#[cfg(test)]
mod test_helpers;
