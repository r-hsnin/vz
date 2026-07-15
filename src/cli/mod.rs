use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

/// vz — CLI BI tool with smart visualization and terminal presentation.
#[derive(Parser, Debug, Clone)]
#[command(name = "vz", version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Input file (CSV, TSV, JSON, NDJSON). Use "-" for stdin.
    #[arg(value_name = "FILE")]
    pub file: Option<PathBuf>,

    /// Column to use for X axis.
    #[arg(short = 'x', long = "x-col")]
    pub x_col: Option<String>,

    /// Column(s) for Y axis. Comma-separated for multi-series. Supports "col:Label" override.
    #[arg(short = 'y', long = "y-col")]
    pub y_col: Option<String>,

    /// Override chart type (line, bar, scatter, histogram, heatmap).
    #[arg(short = 't', long = "type", value_enum)]
    pub chart_type: Option<ChartTypeArg>,

    /// Color/group-by column.
    #[arg(short = 'c', long = "color")]
    pub color_col: Option<String>,

    /// Chart width in columns (default: terminal width).
    #[arg(short = 'W', long = "width")]
    pub width: Option<u16>,

    /// Chart height in rows (default: 24).
    #[arg(short = 'H', long = "height")]
    pub height: Option<u16>,

    /// Show column metadata (types, unique values) without rendering a chart.
    #[arg(short = 'I', long = "info")]
    pub info: bool,

    /// Treat first row as data (no header row). Auto-detected if first row is all-numeric.
    #[arg(long = "no-header")]
    pub no_header: bool,

    /// Sort bar chart values: desc, asc, or none (default: none).
    #[arg(long = "sort", value_enum)]
    pub sort: Option<SortOrder>,

    /// Force input format (auto-detected if not specified).
    #[arg(short = 'f', long = "format", value_enum)]
    pub format: Option<InputFormatArg>,

    /// Filter rows: col=value, col!=value, col>value, col>=value, col<value, col<=value (repeatable).
    #[arg(short = 'w', long = "where", value_name = "FILTER")]
    pub filter: Vec<String>,

    /// Show only the top N categories (by Y value, descending). Implies --sort desc.
    #[arg(long = "top", value_name = "N", conflicts_with = "tail")]
    pub top: Option<usize>,

    /// Show only the bottom N categories (by Y value, ascending). Implies --sort asc.
    #[arg(long = "tail", value_name = "N")]
    pub tail: Option<usize>,

    /// Aggregation function for bar charts: sum (default), mean, count, max, min.
    #[arg(long = "agg", value_enum)]
    pub agg: Option<AggFunction>,

    /// Custom chart title (overrides the auto-generated title).
    #[arg(long = "title")]
    pub title: Option<String>,

    /// Output format: text (default), json, table, spark, svg, markdown.
    #[arg(short = 'o', long = "output", value_enum)]
    pub output: Option<OutputFormat>,

    /// Shorthand for --output json (machine-readable JSON output).
    #[arg(long = "json", conflicts_with = "output")]
    pub json: bool,

    /// Shorthand for --output spark (single-line sparkline).
    #[arg(long = "spark", conflicts_with_all = ["output", "json"])]
    pub spark: bool,

    /// Shorthand for --output svg (SVG image export).
    #[arg(long = "svg", conflicts_with_all = ["output", "json", "spark", "markdown"])]
    pub svg: bool,

    /// Shorthand for --output markdown (Markdown table export).
    #[arg(long = "markdown", conflicts_with_all = ["output", "json", "spark", "svg"])]
    pub markdown: bool,

    /// Sample at most N rows from the data (systematic sampling for large datasets).
    #[arg(long = "sample", value_name = "N")]
    pub sample: Option<usize>,

    /// Plot all quantitative columns as multi-series (overlay all numeric Y columns).
    #[arg(short = 'Y', long = "all-y")]
    pub all_y: bool,

    /// Show value labels with percentages on bar chart bars.
    #[arg(long = "labels")]
    pub labels: bool,

    /// Watch the input file for changes and re-render automatically.
    #[arg(long = "watch")]
    pub watch: bool,

    /// Color theme: dark (default), light, or high-contrast.
    #[arg(long = "theme", value_enum)]
    pub theme: Option<ThemeArg>,

    /// Number of bins for histogram charts (default: 10).
    #[arg(long = "bins", value_name = "N")]
    pub bins: Option<usize>,

    /// Glob pattern to filter files in directory mode (e.g. "sales_*.csv").
    #[arg(long = "glob", value_name = "PATTERN")]
    pub glob: Option<String>,

    /// Recursively scan subdirectories in directory mode (excludes hidden directories).
    #[arg(short = 'R', long = "recurse")]
    pub recurse: bool,

    /// Show schema catalog of files in a directory (columns, row counts, format per file).
    #[arg(long = "catalog")]
    pub catalog: bool,
}

#[derive(Subcommand, Debug, PartialEq, Clone)]
pub enum Command {
    /// Interactive exploration mode (TUI).
    Explore {
        /// Input file.
        #[arg(value_name = "FILE")]
        file: PathBuf,
        /// Filter rows: e.g. --where "city=Tokyo" (same syntax as oneshot).
        #[arg(short = 'w', long = "where", value_name = "FILTER")]
        filter: Vec<String>,
    },
    /// Presentation mode with markdown slides.
    Present {
        /// Markdown file with chart blocks.
        #[arg(value_name = "FILE")]
        file: PathBuf,
    },
    /// Generate shell completion scripts.
    Completions {
        /// Shell to generate completions for.
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
}

/// Sort order for bar chart values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SortOrder {
    /// Sort by value descending (highest first).
    Desc,
    /// Sort by value ascending (lowest first).
    Asc,
    /// Keep original order.
    None,
}

/// Aggregation function for bar charts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum AggFunction {
    /// Sum of values per category (default).
    Sum,
    /// Arithmetic mean per category.
    Mean,
    /// Count of rows per category.
    Count,
    /// Maximum value per category.
    Max,
    /// Minimum value per category.
    Min,
}

/// Input format for data files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum InputFormatArg {
    /// Comma-separated values.
    Csv,
    /// Tab-separated values.
    Tsv,
    /// JSON array of objects.
    Json,
    /// Newline-delimited JSON (one object per line).
    Ndjson,
}

/// Chart type for the -t/--type flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ChartTypeArg {
    /// Line chart (for temporal × quantitative data).
    Line,
    /// Bar chart (for categorical × quantitative data).
    Bar,
    /// Scatter plot (for quantitative × quantitative data).
    Scatter,
    /// Histogram (distribution of a single quantitative column).
    Histogram,
    /// Heatmap (for categorical × categorical data).
    Heatmap,
}

impl ChartTypeArg {
    /// Convert CLI argument to internal ChartType.
    pub fn to_chart_type(self) -> crate::chart::selector::ChartType {
        use crate::chart::selector::ChartType;
        match self {
            Self::Line => ChartType::Line,
            Self::Bar => ChartType::Bar,
            Self::Scatter => ChartType::Scatter,
            Self::Histogram => ChartType::Histogram,
            Self::Heatmap => ChartType::Heatmap,
        }
    }
}

/// Output format for results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    /// Human-readable text with ANSI charts (default).
    Text,
    /// Machine-readable JSON for agent integration.
    Json,
    /// Formatted text table of aggregated data.
    Table,
    /// Single-line sparkline for pipeline embedding.
    Spark,
    /// SVG image output (monospace text rendering).
    Svg,
    /// Markdown table for documentation embedding.
    Markdown,
}

/// Color theme preset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ThemeArg {
    /// Optimized for dark terminal backgrounds (default).
    Dark,
    /// Optimized for light/white terminal backgrounds.
    Light,
    /// Maximum visibility, colorblind-friendly.
    HighContrast,
}

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
        assert_eq!(cli.file, Some(PathBuf::from("data.csv")));
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
        let cli = Cli::try_parse_from(["vz", "data.csv", "-t", "bar"]).unwrap();
        assert_eq!(cli.chart_type, Some(ChartTypeArg::Bar));
    }

    #[test]
    fn test_cli_parse_explore_subcommand() {
        let cli = Cli::try_parse_from(["vz", "explore", "data.csv"]).unwrap();
        match cli.command {
            Some(Command::Explore { ref file, .. }) => {
                assert_eq!(file, &PathBuf::from("data.csv"));
            }
            _ => panic!("Expected Explore command"),
        }
    }

    #[test]
    fn test_cli_parse_explore_with_where() {
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
        let cli = Cli::try_parse_from(["vz", "data.csv", "-f", "tsv"]).unwrap();
        assert_eq!(cli.format, Some(InputFormatArg::Tsv));
    }

    #[test]
    fn test_cli_parse_format_long() {
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
