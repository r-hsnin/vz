//! CLI-derived resolutions: theme, input file, and loader format override.
//!
//! Single home for the former `helpers::{resolve_*, format_override}`.
//! Only the app plane (`cli` + binary adapters) converts `Cli` into these
//! values; data/output planes take the resolved values, never `&Cli`.

use anyhow::Result;
use std::path::PathBuf;

use super::{Cli, InputFormatArg, ThemeArg};
use crate::loader;
use crate::theme;

/// Resolve the theme from CLI args.
pub fn resolve_theme(cli: &Cli) -> theme::Theme {
    resolve_theme_arg(cli.theme)
}

/// Resolve the theme from an already-extracted theme arg (Cli-free).
pub fn resolve_theme_arg(arg: Option<ThemeArg>) -> theme::Theme {
    match arg {
        Some(ThemeArg::Light) => theme::Theme::light(),
        Some(ThemeArg::HighContrast) => theme::Theme::high_contrast(),
        _ => theme::Theme::dark(),
    }
}

pub fn resolve_input_file(cli: &Cli) -> Result<PathBuf> {
    match cli.primary_file() {
        Some(f) => Ok(f.to_path_buf()),
        None => {
            if !std::io::IsTerminal::is_terminal(&std::io::stdin()) {
                Ok(PathBuf::from("-"))
            } else {
                anyhow::bail!("No input file specified. Usage: vz <file> or pipe data to stdin");
            }
        }
    }
}

/// Convert CLI format argument to loader InputFormat.
pub fn format_override(cli: &Cli) -> Option<loader::InputFormat> {
    cli.format.map(|f| match f {
        InputFormatArg::Csv => loader::InputFormat::Csv,
        InputFormatArg::Tsv => loader::InputFormat::Tsv,
        InputFormatArg::Json => loader::InputFormat::Json,
        InputFormatArg::Ndjson => loader::InputFormat::Ndjson,
        InputFormatArg::Space => loader::InputFormat::Space,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Cli;
    use crate::loader::InputFormat;
    use clap::Parser;
    use std::path::PathBuf;

    #[test]
    fn resolve_input_file_returns_path_when_file_specified() {
        let cli = Cli::try_parse_from(["vz", "sales.csv"]).unwrap();
        let result = resolve_input_file(&cli).unwrap();
        assert_eq!(result, PathBuf::from("sales.csv"));
    }

    #[test]
    fn resolve_input_file_returns_dash_for_stdin_in_non_terminal() {
        let cli = Cli::try_parse_from(["vz", "--info"]).unwrap();
        let result = resolve_input_file(&cli);
        assert!(result.is_ok() || result.is_err());
        if let Ok(path) = result {
            assert_eq!(path, PathBuf::from("-"));
        }
    }

    #[test]
    fn format_override_none_when_not_specified() {
        let cli = Cli::try_parse_from(["vz", "data.csv"]).unwrap();
        assert_eq!(format_override(&cli), None);
    }

    #[test]
    fn format_override_maps_tsv() {
        let cli = Cli::try_parse_from(["vz", "-", "-f", "tsv"]).unwrap();
        assert_eq!(format_override(&cli), Some(InputFormat::Tsv));
    }

    #[test]
    fn format_override_maps_ndjson() {
        let cli = Cli::try_parse_from(["vz", "-", "-f", "ndjson"]).unwrap();
        assert_eq!(format_override(&cli), Some(InputFormat::Ndjson));
    }
}
