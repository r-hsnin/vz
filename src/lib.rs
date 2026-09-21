//! vz — CLI BI tool with smart visualization and terminal presentation.
//!
//! The library surface is the future `vz-core` body: data-plane modules
//! (`loader`, `infer`, `filter`, `chart`, `util`, `theme`, …) plus the
//! binary entry points (`run`, [`Cli`](cli::Cli)). Everything else is an
//! app-plane detail and stays `pub(crate)` or private.

pub mod chart;
pub mod cli;
pub mod filter;
pub mod infer;
pub mod loader;
pub mod theme;
pub mod util;

pub(crate) mod diagnostics;
pub(crate) mod diff;
pub(crate) mod directory;
pub(crate) mod explore;
pub(crate) mod info;
pub(crate) mod insights;
pub(crate) mod oneshot;
pub(crate) mod output;
pub(crate) mod pipeline;
pub(crate) mod present;
pub(crate) mod render;
pub(crate) mod sparkline;
pub(crate) mod watch;

mod app;
pub use app::{apply_output_shorthands, run};

/// Re-exported for benchmarks: the load → infer measurement path.
pub use pipeline::infer_from_data;

#[cfg(test)]
mod test_helpers;
