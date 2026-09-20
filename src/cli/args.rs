use super::Cli;
use crate::chart::selector::SortOrder;

use super::OutputFormat;

/// Resolved, Cli-free inputs for the single-file render pipeline.
///
/// Built once in the app plane (`Cli::to_pipeline_params`); downstream
/// pipeline/output code takes this instead of `&Cli`. The `query` field is
/// the Phase 2 `Query` seam (`to_query`); the rest are the other `Cli`
/// resolutions the pipeline needs (sort/limit/output/flags/theme/…).
#[derive(Debug, Clone)]
pub struct PipelineParams {
    pub query: crate::chart::Query,
    pub filters: Vec<String>,
    pub sample: Option<usize>,
    pub info: bool,
    pub all_y: bool,
    pub output: Option<OutputFormat>,
    pub sort: Option<SortOrder>,
    pub sort_flag: Option<SortOrder>,
    pub limit: Option<usize>,
    pub bins: Option<usize>,
    pub title: Option<String>,
    pub labels: bool,
    pub width: Option<u16>,
    pub height: Option<u16>,
    pub theme: Option<super::ThemeArg>,
}

impl Cli {
    /// Compute the effective sort order, considering --top (implies desc) and --tail (implies asc).
    pub fn effective_sort(&self) -> Option<SortOrder> {
        if let Some(s) = self.sort {
            return Some(s.to_sort_order());
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

    /// Convert CLI flags into the Cli-free [`Query`](crate::chart::Query).
    /// Single conversion point for the recommendation seam (app plane only).
    pub fn to_query(&self) -> crate::chart::Query {
        crate::chart::Query {
            x_col: self.x_col.clone(),
            y_col: self.y_col.clone(),
            chart_type: self.chart_type,
            color_col: self.color_col.clone(),
            agg: self.agg.map(|a| a.to_agg_function()),
        }
    }

    /// Convert CLI flags into [`PipelineParams`]: the `Query` seam plus the
    /// other resolved render inputs. Sole funnel for `Cli → pipeline`.
    pub fn to_pipeline_params(&self) -> PipelineParams {
        let query = self.to_query();
        PipelineParams {
            query,
            filters: self.filter.clone(),
            sample: self.sample,
            info: self.info,
            all_y: self.all_y,
            output: self.output,
            sort: self.effective_sort(),
            sort_flag: self.sort.map(|s| s.to_sort_order()),
            limit: self.top.or(self.tail),
            bins: self.bins,
            title: self.title.clone(),
            labels: self.labels,
            width: self.width,
            height: self.height,
            theme: self.theme,
        }
    }

    /// Convert CLI flags into [`DirectoryParams`]: the `PipelineParams`
    /// seam plus the directory scan/combine/catalog inputs. Sole funnel
    /// for `Cli → directory`.
    pub fn to_directory_params(&self) -> DirectoryParams {
        DirectoryParams {
            pipeline: self.to_pipeline_params(),
            glob_pattern: self.glob.clone(),
            recurse: self.recurse,
            catalog: self.catalog,
            no_header: self.no_header,
            no_limit: self.no_limit,
        }
    }

    /// Convert CLI flags into [`DiffParams`]: the `Query` seam plus the
    /// other resolved diff inputs. Sole funnel for `Cli → diff`.
    pub fn to_diff_params(&self) -> DiffParams {
        DiffParams {
            query: self.to_query(),
            no_header: self.no_header,
            format: super::format_override(self),
            output: self.output,
            sort: self.effective_sort(),
            limit: self.top.or(self.tail),
            width: self.width,
            height: self.height,
            title: self.title.clone(),
            theme: self.theme,
        }
    }
}

/// Resolved, Cli-free inputs for directory mode (multi-file combine).
///
/// Built once in the app plane (`Cli::to_directory_params`); downstream
/// directory code takes this instead of `&Cli`. The `pipeline` field is
/// the Phase 2-4 seam (`to_pipeline_params`); the rest are the directory
/// resolutions (scan/combine/catalog flags).
#[derive(Debug, Clone)]
pub struct DirectoryParams {
    pub pipeline: PipelineParams,
    pub glob_pattern: Option<String>,
    pub recurse: bool,
    pub catalog: bool,
    pub no_header: bool,
    pub no_limit: bool,
}

/// Resolved, Cli-free inputs for diff mode (two-file comparison).
///
/// Built once in the app plane (`Cli::to_diff_params`); downstream
/// diff/schema/render code takes this instead of `&Cli`. The `query` field
/// is the Phase 2 `Query` seam (`to_query`); `--where`/`--agg`/`--color`
/// have no effect in diff mode, so their warnings stay in the
/// `run_diff_from_cli` adapter and never reach this struct.
#[derive(Debug, Clone)]
pub struct DiffParams {
    pub query: crate::chart::Query,
    pub no_header: bool,
    pub format: Option<crate::loader::InputFormat>,
    pub output: Option<OutputFormat>,
    pub sort: Option<SortOrder>,
    pub limit: Option<usize>,
    pub width: Option<u16>,
    pub height: Option<u16>,
    pub title: Option<String>,
    pub theme: Option<super::ThemeArg>,
}
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
    use clap::{CommandFactory, Parser};
    use std::path::PathBuf;

    #[test]
    fn test_after_help_shows_examples_legend_and_stream_split() {
        let after = Cli::command().get_after_help().unwrap().to_string();
        assert!(after.contains("Examples:"), "{after}");
        assert!(after.contains("vz sales.csv -x city"), "{after}");
        assert!(after.contains("→ stable"), "{after}");
        assert!(
            after.contains("summary and warnings go to stderr"),
            "{after}"
        );
    }

    #[test]
    fn test_to_pipeline_params_resolves_render_inputs() {
        let cli = Cli::try_parse_from(["vz", "data.csv", "--top", "3", "--agg", "mean"]).unwrap();
        let params = cli.to_pipeline_params();
        assert_eq!(params.query.x_col, None);
        assert_eq!(
            params.query.agg,
            Some(crate::chart::selector::AggFunction::Mean)
        );
        assert_eq!(params.sort, Some(SortOrder::Desc));
        assert_eq!(params.limit, Some(3));
        assert_eq!(params.sample, None);
        assert!(!params.info);
    }

    #[test]
    fn test_to_directory_params_resolves_scan_inputs() {
        let cli =
            Cli::try_parse_from(["vz", "dir/", "--glob", "sales_*", "--recurse", "--no-limit"])
                .unwrap();
        let params = cli.to_directory_params();
        assert_eq!(params.glob_pattern.as_deref(), Some("sales_*"));
        assert!(params.recurse);
        assert!(params.no_limit);
        assert!(!params.catalog);
        assert!(!params.no_header);
        assert_eq!(params.pipeline.query.x_col, None);
    }

    #[test]
    fn test_to_diff_params_resolves_render_inputs() {
        let cli =
            Cli::try_parse_from(["vz", "a.csv", "b.csv", "--top", "2", "-x", "city"]).unwrap();
        let params = cli.to_diff_params();
        assert_eq!(params.query.x_col.as_deref(), Some("city"));
        assert_eq!(params.sort, Some(SortOrder::Desc));
        assert_eq!(params.limit, Some(2));
        assert!(!params.no_header);
    }

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
