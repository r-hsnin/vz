//! Markdown table output mode: emit data as GitHub-Flavored Markdown tables.

use anyhow::Result;

use crate::chart;
use crate::chart::data_builder;
use crate::cli;
use crate::oneshot;

/// Print data as a Markdown table, respecting chart type for aggregation.
pub fn print_markdown(
    recommendation: &chart::selector::ChartRecommendation,
    headers: &[String],
    rows: &[Vec<String>],
    cli: &cli::Cli,
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
        let agg = cli.agg.unwrap_or(cli::AggFunction::Sum);
        let y_label = recommendation.y_column.as_deref().unwrap_or("value");
        let (mut bar_data, _) =
            data_builder::aggregate_bar(rows, xi, yi, None, y_label.to_string(), agg);
        crate::oneshot::builders::sort_bar_data(&mut bar_data, cli.effective_sort());
        crate::oneshot::builders::truncate_bar_data(&mut bar_data, cli.top.or(cli.tail));
        print_markdown_two_col(
            &recommendation.x_column,
            y_label,
            &bar_data.labels,
            &bar_data.values,
        );
        return Ok(());
    }

    // For other chart types: show all columns
    print_markdown_all(headers, rows);
    Ok(())
}

/// Render a two-column aggregated table as Markdown.
fn print_markdown_two_col(x_label: &str, y_label: &str, labels: &[String], values: &[f64]) {
    println!("| {} | {} |", x_label, y_label);
    println!("|---|---|");
    for (label, value) in labels.iter().zip(values.iter()) {
        println!("| {} | {} |", label, format_value(*value));
    }
}

/// Render all columns as a Markdown table.
fn print_markdown_all(headers: &[String], rows: &[Vec<String>]) {
    // Header row
    let header_line: String = headers
        .iter()
        .map(|h| format!(" {} ", h))
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
    for row in rows {
        let cells: Vec<String> = headers
            .iter()
            .enumerate()
            .map(|(i, _)| format!(" {} ", row.get(i).map_or("", |v| v.as_str())))
            .collect();
        println!("|{}|", cells.join("|"));
    }
}

/// Format a numeric value for display (remove trailing zeros for integers).
fn format_value(val: f64) -> String {
    if val == val.trunc() && val.abs() < 1_000_000.0 {
        format!("{:.0}", val)
    } else {
        format!("{:.2}", val)
    }
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
        assert_eq!(format_value(3.75), "3.75");
    }

    #[test]
    fn test_format_value_large() {
        assert_eq!(format_value(1_500_000.0), "1500000.00");
    }

    #[test]
    fn test_markdown_two_col_format() {
        // Capture by running the function in a test — we just verify format_value works
        // The actual output is tested via integration tests
        assert_eq!(format_value(1000.0), "1000");
        assert_eq!(format_value(1500.5), "1500.50");
    }
}
