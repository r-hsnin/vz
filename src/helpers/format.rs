use crate::cli::{self, Cli};
use crate::loader;

/// Convert CLI format argument to loader InputFormat.
pub(crate) fn format_override(cli: &Cli) -> Option<loader::InputFormat> {
    cli.format.map(|f| match f {
        cli::InputFormatArg::Csv => loader::InputFormat::Csv,
        cli::InputFormatArg::Tsv => loader::InputFormat::Tsv,
        cli::InputFormatArg::Json => loader::InputFormat::Json,
        cli::InputFormatArg::Ndjson => loader::InputFormat::Ndjson,
        cli::InputFormatArg::Space => loader::InputFormat::Space,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Cli;
    use crate::loader::InputFormat;
    use clap::Parser;

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
