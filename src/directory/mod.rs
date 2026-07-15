//! Directory mode: combine multiple data files from a directory.
//!
//! Scans a directory for data files, validates schema compatibility,
//! concatenates rows, and injects a `_source` column with each file's stem.

#[allow(unused)]
pub mod combiner;
#[allow(unused)]
pub mod scanner;
#[cfg(test)]
mod tests;

use std::path::Path;

use anyhow::Result;

use crate::cli::Cli;

/// Run directory mode: scan, combine, and render data from a directory.
#[allow(unused)]
pub fn run_directory(cli: &Cli, dir: &Path) -> Result<()> {
    todo!()
}
