//! Table output mode: print data as formatted text tables.

use anyhow::Result;

use crate::chart;
use crate::chart::data_builder;
use crate::cli;
use crate::oneshot;

/// Print data as a formatted text table, respecting chart type for aggregation.
pub fn print_table(
    recommendation: &chart::selector::ChartRecommendation,
    headers: &[String],
    rows: &[Vec<String>],
    cli: &cli::Cli,
) -> Result<()> {
    let x_idx = headers.iter().position(|h| h == &recommendation.x_column);
    let y_idx = recommendation
        .y_column
        .as_ref()
        .and_then(|y| headers.iter().position(|h| h == y));

    let chart_type = oneshot::resolve_chart_type(recommendation, cli.chart_type);

    // For bar charts, show aggregated data
    if chart_type == chart::selector::ChartType::Bar
        && let (Some(xi), Some(yi)) = (x_idx, y_idx)
    {
        let agg = cli.agg.unwrap_or(cli::AggFunction::Sum);
        let y_label = recommendation.y_column.as_deref().unwrap_or("value");
        let (bar_data, _) =
            data_builder::aggregate_bar(rows, xi, yi, None, y_label.to_string(), agg);
        print_two_col_values(
            &recommendation.x_column,
            y_label,
            &bar_data.labels,
            &bar_data.values,
        );
        return Ok(());
    }

    // For other chart types: show raw x, y data
    match (x_idx, y_idx) {
        (Some(xi), Some(yi)) => {
            let x_label = &recommendation.x_column;
            let y_label = recommendation.y_column.as_deref().unwrap_or("value");
            print_xy_table(x_label, y_label, rows, xi, yi);
        }
        _ => print_all_columns(headers, rows),
    }
    Ok(())
}

/// Print a two-column table: labels + numeric values.
fn print_two_col_values(x_label: &str, y_label: &str, labels: &[String], values: &[f64]) {
    let col_w = labels
        .iter()
        .map(|l| l.len())
        .max()
        .unwrap_or(5)
        .max(x_label.len());
    let val_w = 12;
    println!("{:<col_w$}  {:>val_w$}", x_label, y_label);
    println!("{:-<col_w$}  {:-<val_w$}", "", "");
    for (label, value) in labels.iter().zip(values.iter()) {
        println!("{:<col_w$}  {:>val_w$.2}", label, value);
    }
}

/// Print a two-column table from raw row data.
fn print_xy_table(x_label: &str, y_label: &str, rows: &[Vec<String>], xi: usize, yi: usize) {
    let col_w = col_width(rows, xi, x_label.len());
    let val_w = col_width(rows, yi, y_label.len());
    println!("{:<col_w$}  {:>val_w$}", x_label, y_label);
    println!("{:-<col_w$}  {:-<val_w$}", "", "");
    for row in rows {
        let x_val = row.get(xi).map_or("", |v| v.as_str());
        let y_val = row.get(yi).map_or("", |v| v.as_str());
        println!("{:<col_w$}  {:>val_w$}", x_val, y_val);
    }
}

/// Print all columns as a table (fallback when x/y columns can't be determined).
pub fn print_all_columns(headers: &[String], rows: &[Vec<String>]) {
    let widths: Vec<usize> = headers
        .iter()
        .enumerate()
        .map(|(i, h)| col_width(rows, i, h.len()))
        .collect();
    for (i, h) in headers.iter().enumerate() {
        if i > 0 {
            print!("  ");
        }
        print!("{:<width$}", h, width = widths[i]);
    }
    println!();
    for w in &widths {
        print!("{:-<width$}  ", "", width = w);
    }
    println!();
    for row in rows {
        for (i, val) in row.iter().enumerate() {
            if i > 0 {
                print!("  ");
            }
            print!(
                "{:<width$}",
                val,
                width = widths.get(i).copied().unwrap_or(5)
            );
        }
        println!();
    }
}

/// Compute column display width from data.
fn col_width(rows: &[Vec<String>], idx: usize, min: usize) -> usize {
    rows.iter()
        .map(|r| r.get(idx).map_or(0, |v| v.len()))
        .max()
        .unwrap_or(min)
        .max(min)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_col_width_uses_max_data_length() {
        let rows = vec![
            vec!["ab".to_string(), "12345".to_string()],
            vec!["abc".to_string(), "1".to_string()],
        ];
        assert_eq!(col_width(&rows, 0, 2), 3); // "abc" is longest
        assert_eq!(col_width(&rows, 1, 2), 5); // "12345" is longest
    }

    #[test]
    fn test_col_width_respects_minimum() {
        let rows = vec![vec!["a".to_string()]];
        assert_eq!(col_width(&rows, 0, 10), 10);
    }

    #[test]
    fn test_col_width_empty_rows() {
        let rows: Vec<Vec<String>> = vec![];
        assert_eq!(col_width(&rows, 0, 5), 5);
    }

    #[test]
    fn test_col_width_missing_index() {
        let rows = vec![vec!["a".to_string()]];
        assert_eq!(col_width(&rows, 5, 3), 3);
    }
}
