mod args;
mod types;

pub use args::{parse_column_spec, parse_multi_y_specs};
pub use types::{AggFunction, ChartTypeArg, InputFormatArg, OutputFormat, SortOrder, ThemeArg};

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// vz — CLI BI tool with smart visualization and terminal presentation.
#[derive(Parser, Debug, Clone)]
#[command(name = "vz", version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Input file(s) (CSV, TSV, JSON, NDJSON). Use "-" for stdin. Provide two files for diff mode.
    #[arg(value_name = "FILE", num_args = 0..=2)]
    pub file: Vec<PathBuf>,

    /// Second file for comparison (diff mode). Alternative to `vz <file1> <file2>`.
    #[arg(long = "diff", value_name = "FILE2")]
    pub diff_file: Option<PathBuf>,

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
    #[arg(long = "svg", conflicts_with_all = ["output", "json", "spark", "markdown", "html"])]
    pub svg: bool,

    /// Shorthand for --output markdown (Markdown table export).
    #[arg(long = "markdown", conflicts_with_all = ["output", "json", "spark", "svg", "html"])]
    pub markdown: bool,

    /// Shorthand for --output html (self-contained HTML page with embedded chart).
    #[arg(long = "html", conflicts_with_all = ["output", "json", "spark", "svg", "markdown"])]
    pub html: bool,

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

    /// Disable automatic row limit for directory mode (load all rows regardless of size).
    #[arg(long = "no-limit")]
    pub no_limit: bool,
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
