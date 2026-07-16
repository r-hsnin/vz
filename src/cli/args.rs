use super::{Cli, SortOrder};

impl Cli {
    /// Compute the effective sort order, considering --top (implies desc) and --tail (implies asc).
    pub fn effective_sort(&self) -> Option<SortOrder> {
        if self.sort.is_some() {
            return self.sort;
        }
        if self.top.is_some() {
            return Some(SortOrder::Desc);
        }
        if self.tail.is_some() {
            return Some(SortOrder::Asc);
        }
        None
    }

    /// Resolve diff file pair: returns (before, after) if diff mode is active.
    pub fn diff_pair(&self) -> Option<(std::path::PathBuf, std::path::PathBuf)> {
        if self.file.len() == 2 {
            Some((self.file[0].clone(), self.file[1].clone()))
        } else if self.file.len() == 1 {
            self.diff_file
                .as_ref()
                .map(|d| (self.file[0].clone(), d.clone()))
        } else {
            None
        }
    }

    /// Get the primary input file (first positional argument).
    pub fn primary_file(&self) -> Option<&std::path::Path> {
        self.file.first().map(|p| p.as_path())
    }
}

/// Parse a column spec that may include a label override.
/// "revenue" → ("revenue", None)
/// "revenue:Revenue (USD)" → ("revenue", Some("Revenue (USD)"))
pub fn parse_column_spec(spec: &str) -> (&str, Option<&str>) {
    match spec.split_once(':') {
        Some((col, label)) => (col, Some(label)),
        None => (spec, None),
    }
}

/// Parse a comma-separated list of Y column specs.
/// "revenue,profit" → [("revenue", None), ("profit", None)]
/// "revenue:Rev,profit:Prof" → [("revenue", Some("Rev")), ("profit", Some("Prof"))]
pub fn parse_multi_y_specs(spec: &str) -> Vec<(&str, Option<&str>)> {
    spec.split(',')
        .map(|s| parse_column_spec(s.trim()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Cli;
    use clap::Parser;
    use std::path::PathBuf;

    #[test]
    fn test_effective_sort_explicit_sort_takes_priority() {
        let cli = Cli::try_parse_from(["vz", "data.csv", "--sort", "asc", "--top", "5"]).unwrap();
        assert_eq!(cli.effective_sort(), Some(SortOrder::Asc));
    }

    #[test]
    fn test_effective_sort_top_implies_desc() {
        let cli = Cli::try_parse_from(["vz", "data.csv", "--top", "3"]).unwrap();
        assert_eq!(cli.effective_sort(), Some(SortOrder::Desc));
    }

    #[test]
    fn test_effective_sort_tail_implies_asc() {
        let cli = Cli::try_parse_from(["vz", "data.csv", "--tail", "3"]).unwrap();
        assert_eq!(cli.effective_sort(), Some(SortOrder::Asc));
    }

    #[test]
    fn test_effective_sort_none_by_default() {
        let cli = Cli::try_parse_from(["vz", "data.csv"]).unwrap();
        assert_eq!(cli.effective_sort(), None);
    }

    #[test]
    fn test_parse_column_spec_simple() {
        let (col, label) = parse_column_spec("revenue");
        assert_eq!(col, "revenue");
        assert_eq!(label, None);
    }

    #[test]
    fn test_parse_column_spec_with_label() {
        let (col, label) = parse_column_spec("revenue:Revenue (USD)");
        assert_eq!(col, "revenue");
        assert_eq!(label, Some("Revenue (USD)"));
    }

    #[test]
    fn test_parse_column_spec_colon_in_label() {
        let (col, label) = parse_column_spec("time:Time (HH:MM)");
        assert_eq!(col, "time");
        assert_eq!(label, Some("Time (HH:MM)"));
    }

    #[test]
    fn test_cli_parse_default() {
        let cli = Cli::try_parse_from(["vz", "data.csv"]).unwrap();
        assert_eq!(cli.file, vec![PathBuf::from("data.csv")]);
        assert_eq!(cli.command, None);
    }

    #[test]
    fn test_cli_parse_with_axes() {
        let cli = Cli::try_parse_from(["vz", "data.csv", "-x", "month", "-y", "revenue"]).unwrap();
        assert_eq!(cli.x_col, Some("month".to_string()));
        assert_eq!(cli.y_col, Some("revenue".to_string()));
    }

    #[test]
    fn test_cli_parse_with_type() {
        use crate::cli::ChartTypeArg;
        let cli = Cli::try_parse_from(["vz", "data.csv", "-t", "bar"]).unwrap();
        assert_eq!(cli.chart_type, Some(ChartTypeArg::Bar));
    }

    #[test]
    fn test_cli_parse_explore_subcommand() {
        use crate::cli::Command;
        let cli = Cli::try_parse_from(["vz", "explore", "data.csv"]).unwrap();
        match cli.command {
            Some(Command::Explore { ref file, .. }) => {
                assert_eq!(file, &vec![PathBuf::from("data.csv")]);
            }
            _ => panic!("Expected Explore command"),
        }
    }

    #[test]
    fn test_cli_parse_explore_with_where() {
        use crate::cli::Command;
        let cli = Cli::try_parse_from([
            "vz",
            "explore",
            "data.csv",
            "--where",
            "city=Tokyo",
            "-w",
            "revenue>100",
        ])
        .unwrap();
        match cli.command {
            Some(Command::Explore { ref filter, .. }) => {
                assert_eq!(filter, &["city=Tokyo", "revenue>100"]);
            }
            _ => panic!("Expected Explore command"),
        }
    }

    #[test]
    fn test_cli_parse_present_subcommand() {
        use crate::cli::Command;
        let cli = Cli::try_parse_from(["vz", "present", "slides.md"]).unwrap();
        match cli.command {
            Some(Command::Present { ref file }) => {
                assert_eq!(file, &PathBuf::from("slides.md"));
            }
            _ => panic!("Expected Present command"),
        }
    }

    #[test]
    fn test_cli_parse_width_height() {
        let cli = Cli::try_parse_from(["vz", "data.csv", "-W", "60", "-H", "15"]).unwrap();
        assert_eq!(cli.width, Some(60));
        assert_eq!(cli.height, Some(15));
    }

    #[test]
    fn test_cli_parse_width_height_long_form() {
        let cli =
            Cli::try_parse_from(["vz", "data.csv", "--width", "100", "--height", "30"]).unwrap();
        assert_eq!(cli.width, Some(100));
        assert_eq!(cli.height, Some(30));
    }

    #[test]
    fn test_parse_multi_y_specs_single() {
        let specs = parse_multi_y_specs("revenue");
        assert_eq!(specs, vec![("revenue", None)]);
    }

    #[test]
    fn test_parse_multi_y_specs_multiple() {
        let specs = parse_multi_y_specs("revenue,profit");
        assert_eq!(specs, vec![("revenue", None), ("profit", None)]);
    }

    #[test]
    fn test_parse_multi_y_specs_with_labels() {
        let specs = parse_multi_y_specs("revenue:Rev,profit:Prof");
        assert_eq!(
            specs,
            vec![("revenue", Some("Rev")), ("profit", Some("Prof"))]
        );
    }

    #[test]
    fn test_parse_multi_y_specs_mixed() {
        let specs = parse_multi_y_specs("revenue:Revenue (USD),profit");
        assert_eq!(
            specs,
            vec![("revenue", Some("Revenue (USD)")), ("profit", None)]
        );
    }

    #[test]
    fn test_cli_parse_format_flag() {
        use crate::cli::InputFormatArg;
        let cli = Cli::try_parse_from(["vz", "data.csv", "-f", "tsv"]).unwrap();
        assert_eq!(cli.format, Some(InputFormatArg::Tsv));
    }

    #[test]
    fn test_cli_parse_format_long() {
        use crate::cli::InputFormatArg;
        let cli = Cli::try_parse_from(["vz", "-", "--format", "ndjson"]).unwrap();
        assert_eq!(cli.format, Some(InputFormatArg::Ndjson));
    }

    #[test]
    fn test_cli_parse_bins_flag() {
        let cli = Cli::try_parse_from(["vz", "data.csv", "--bins", "20"]).unwrap();
        assert_eq!(cli.bins, Some(20));
    }

    #[test]
    fn test_cli_parse_bins_flag_not_set() {
        let cli = Cli::try_parse_from(["vz", "data.csv"]).unwrap();
        assert_eq!(cli.bins, None);
    }
}
