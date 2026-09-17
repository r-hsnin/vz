//! Markdown table output mode: emit data as GitHub-Flavored Markdown tables.

use anyhow::Result;

use crate::chart;
use crate::chart::data_builder;
use crate::cli;
use crate::helpers::effective_agg;
use crate::infer::types::Schema;
use crate::oneshot;
use crate::output::table::agg_header;
use crate::render::format_number;

/// Maximum data rows printed before truncation (mirrors the JSON 100-row cap).
pub const MARKDOWN_ROW_LIMIT: usize = 100;

/// Print data as a Markdown table, respecting chart type for aggregation.
pub fn print_markdown(
    recommendation: &chart::selector::ChartRecommendation,
    headers: &[String],
    rows: &[Vec<String>],
    cli: &cli::Cli,
    schema: &Schema,
) -> Result<()> {
    let x_idx = data_builder::column_index(headers, &recommendation.x_column);
    let y_idx = recommendation
        .y_column
        .as_ref()
        .and_then(|y| data_builder::column_index(headers, y));

    let chart_type = oneshot::resolve_chart_type(recommendation, cli.chart_type);

    // For bar charts, show aggregated data
    if chart_type == chart::selector::ChartType::Bar
        && let (Some(xi), Some(yi)) = (x_idx, y_idx)
    {
        let agg = effective_agg(cli, recommendation, schema);
        let y_label = recommendation.y_column.as_deref().unwrap_or("value");
        let (mut bar_data, _) =
            data_builder::aggregate_bar(rows, xi, yi, None, y_label.to_string(), agg);
        crate::oneshot::builders::sort_bar_data(&mut bar_data, cli.effective_sort());
        crate::oneshot::builders::truncate_bar_data(&mut bar_data, cli.top.or(cli.tail));
        print_markdown_two_col(
            &recommendation.x_column,
            &agg_header(y_label, agg),
            &bar_data.labels,
            &bar_data.values,
        );
        return Ok(());
    }

    // For other chart types: show all columns with a row cap.
    // sort/top only apply to bar charts — warn instead of silently ignoring.
    warn_non_bar_limits(chart_type, cli);
    print_markdown_all(
        headers,
        rows,
        cli.top.or(cli.tail).or(Some(MARKDOWN_ROW_LIMIT)),
    );
    Ok(())
}

/// Warn when row-limiting flags are used with a chart type that ignores them.
fn warn_non_bar_limits(chart_type: chart::selector::ChartType, cli: &cli::Cli) {
    use crate::chart::selector::ChartType;
    if cli.top.or(cli.tail).is_some() && !matches!(chart_type, ChartType::Bar) {
        eprintln!(
            "warning: --top/--tail has no effect on {} tables (only applies to bar charts); showing first {} rows",
            chart_type, MARKDOWN_ROW_LIMIT
        );
    } else if matches!(
        cli.sort,
        Some(cli::SortOrder::Desc) | Some(cli::SortOrder::Asc)
    ) && !matches!(chart_type, ChartType::Bar)
    {
        eprintln!(
            "warning: --sort has no effect on {} tables (only applies to bar charts)",
            chart_type
        );
    }
}

/// Render a two-column aggregated table as Markdown.
fn print_markdown_two_col(x_label: &str, y_label: &str, labels: &[String], values: &[f64]) {
    println!("| {} | {} |", escape_cell(x_label), escape_cell(y_label));
    println!("|---|---|");
    for (label, value) in labels.iter().zip(values.iter()) {
        println!("| {} | {} |", escape_cell(label), format_value(*value));
    }
}

/// Render all columns as a Markdown table, truncated to `limit` rows.
fn print_markdown_all(headers: &[String], rows: &[Vec<String>], limit: Option<usize>) {
    let total = rows.len();
    let shown: &[Vec<String>] = match limit {
        Some(n) => &rows[..rows.len().min(n)],
        None => rows,
    };
    // Header row
    let header_line: String = headers
        .iter()
        .map(|h| format!(" {} ", escape_cell(h)))
        .collect::<Vec<_>>()
        .join("|");
    println!("|{}|", header_line);

    // Separator row
    let sep_line: String = headers
        .iter()
        .map(|_| "---".to_string())
        .collect::<Vec<_>>()
        .join("|");
    println!("|{}|", sep_line);

    // Data rows
    for row in shown {
        let cells: Vec<String> = headers
            .iter()
            .enumerate()
            .map(|(i, _)| format!(" {} ", escape_cell(row.get(i).map_or("", |v| v.as_str()))))
            .collect();
        println!("|{}|", cells.join("|"));
    }
    if shown.len() < total {
        eprintln!(
            "info: showing {}/{} rows (use --top N, --sample N, or -o json for full data)",
            shown.len(),
            total
        );
    }
}

/// Escape a cell so embedded pipes/newlines can't break the table structure.
fn escape_cell(cell: &str) -> String {
    cell.replace('|', "\\|")
        .replace("\r\n", "<br/>")
        .replace(['\r', '\n'], "<br/>")
}

/// Format a numeric value for display (remove trailing zeros for integers).
/// Uses the shared chart number formatter (4.2k/1.5M) for consistency.
fn format_value(val: f64) -> String {
    format_number(val)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_value_integer() {
        assert_eq!(format_value(42.0), "42");
        assert_eq!(format_value(0.0), "0");
    }

    #[test]
    fn test_format_value_decimal() {
        assert_eq!(format_value(3.75), "3.8");
    }

    #[test]
    fn test_format_value_large() {
        assert_eq!(format_value(1_500_000.0), "1.5M");
    }

    #[test]
    fn test_markdown_two_col_format() {
        // Capture by running the function in a test — we just verify format_value works
        // The actual output is tested via integration tests
        assert_eq!(format_value(1000.0), "1k");
        assert_eq!(format_value(1500.5), "1.5k");
    }

    #[test]
    fn test_escape_cell_pipe_and_newline() {
        assert_eq!(escape_cell("a|b"), "a\\|b");
        assert_eq!(escape_cell("a\nb"), "a<br/>b");
        assert_eq!(escape_cell("a\r\nb"), "a<br/>b");
        assert_eq!(escape_cell("plain"), "plain");
    }
}
