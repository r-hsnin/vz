use clap::ValueEnum;

use crate::chart::selector::{AggFunction, SortOrder};

/// Sort order for bar chart values (CLI spelling; converts to [`SortOrder`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SortOrderArg {
    /// Sort by value descending (highest first).
    Desc,
    /// Sort by value ascending (lowest first).
    Asc,
    /// Keep original order.
    None,
}

impl SortOrderArg {
    pub fn to_sort_order(self) -> SortOrder {
        match self {
            Self::Desc => SortOrder::Desc,
            Self::Asc => SortOrder::Asc,
            Self::None => SortOrder::None,
        }
    }
}

/// Aggregation function for bar charts (CLI spelling; converts to
/// [`AggFunction`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum AggFunctionArg {
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

impl AggFunctionArg {
    pub fn to_agg_function(self) -> AggFunction {
        match self {
            Self::Sum => AggFunction::Sum,
            Self::Mean => AggFunction::Mean,
            Self::Count => AggFunction::Count,
            Self::Max => AggFunction::Max,
            Self::Min => AggFunction::Min,
        }
    }
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
    /// Fixed-width / space-aligned columns (e.g., kubectl, ps, df output).
    Space,
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
    /// Self-contained HTML page with embedded SVG chart and interactive tooltips.
    Html,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chart::selector::ChartType;

    #[test]
    fn chart_type_arg_converts_all_variants() {
        let cases = [
            (ChartTypeArg::Line, ChartType::Line),
            (ChartTypeArg::Bar, ChartType::Bar),
            (ChartTypeArg::Scatter, ChartType::Scatter),
            (ChartTypeArg::Histogram, ChartType::Histogram),
            (ChartTypeArg::Heatmap, ChartType::Heatmap),
        ];
        for (arg, expected) in cases {
            assert_eq!(arg.to_chart_type(), expected);
        }
    }
}
