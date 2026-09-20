//! Table output mode: print data as formatted text tables.

use anyhow::Result;

use crate::chart;
use crate::chart::data_builder;
use crate::chart::selector::{AggFunction, ChartType, SortOrder};
use crate::infer::types::Schema;
use crate::oneshot;
use crate::render::format_number;

/// Maximum data rows printed before truncation (mirrors the JSON 100-row cap).
pub const TABLE_ROW_LIMIT: usize = 100;

/// Resolved inputs for table output (no CLI dependency).
pub struct TableParams {
    pub chart_type_override: Option<crate::cli::ChartTypeArg>,
    pub agg: AggFunction,
    pub sort: Option<SortOrder>,
    pub limit: Option<usize>,
    pub sort_flag: Option<SortOrder>,
}

/// Print data as a formatted text table, respecting chart type for aggregation.
pub fn print_table(
    recommendation: &chart::selector::ChartRecommendation,
    headers: &[String],
    rows: &[Vec<String>],
    params: &TableParams,
    schema: &Schema,
) -> Result<()> {
    let x_idx = data_builder::column_index(headers, &recommendation.x_column);
    let y_idx = recommendation
        .y_column
        .as_ref()
        .and_then(|y| data_builder::column_index(headers, y));

    let chart_type = oneshot::resolve_chart_type(recommendation, params.chart_type_override);

    // For bar charts, show aggregated data
    if chart_type == chart::selector::ChartType::Bar
        && let (Some(xi), Some(yi)) = (x_idx, y_idx)
    {
        let agg = params.agg;
        let y_label = recommendation.y_column.as_deref().unwrap_or("value");
        let (mut bar_data, _) =
            data_builder::aggregate_bar(rows, xi, yi, None, y_label.to_string(), agg);
        data_builder::sort_bar_data(&mut bar_data, params.sort);
        data_builder::truncate_bar_data(&mut bar_data, params.limit);
        print_two_col_values(
            &recommendation.x_column,
            &agg_header(y_label, agg),
            &bar_data.labels,
            &bar_data.values,
        );
        return Ok(());
    }

    // For other chart types: show all columns (users expect full data view).
    // sort/top only apply to bar charts — warn instead of silently ignoring.
    warn_non_bar_limits(chart_type, params.limit, params.sort_flag);
    print_all_columns(headers, rows, params.limit.or(Some(TABLE_ROW_LIMIT)));
    let _ = schema;
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
        println!("{:<col_w$}  {:>val_w$}", label, format_number(*value));
    }
}

/// Header for an aggregated column: `revenue` for sum, `mean(revenue)` otherwise.
pub fn agg_header(y_label: &str, agg: AggFunction) -> String {
    match agg {
        AggFunction::Sum => y_label.to_string(),
        AggFunction::Mean => format!("mean({y_label})"),
        AggFunction::Count => format!("count({y_label})"),
        AggFunction::Max => format!("max({y_label})"),
        AggFunction::Min => format!("min({y_label})"),
    }
}

/// Warn when row-limiting flags are used with a chart type that ignores them.
fn warn_non_bar_limits(chart_type: ChartType, limit: Option<usize>, sort_flag: Option<SortOrder>) {
    if limit.is_some() && !matches!(chart_type, ChartType::Bar) {
        eprintln!(
            "warning: --top/--tail has no effect on {} tables (only applies to bar charts); showing first {} rows",
            chart_type, TABLE_ROW_LIMIT
        );
    } else if matches!(sort_flag, Some(SortOrder::Desc) | Some(SortOrder::Asc))
        && !matches!(chart_type, ChartType::Bar)
    {
        eprintln!(
            "warning: --sort has no effect on {} tables (only applies to bar charts)",
            chart_type
        );
    }
}

/// Print all columns as a table, truncated to `limit` rows with a count footer.
pub fn print_all_columns(headers: &[String], rows: &[Vec<String>], limit: Option<usize>) {
    let total = rows.len();
    let shown: &[Vec<String>] = match limit {
        Some(n) => &rows[..rows.len().min(n)],
        None => rows,
    };
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
    for row in shown {
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
    if shown.len() < total {
        eprintln!(
            "info: showing {}/{} rows (use --top N, --sample N, or -o json for full data)",
            shown.len(),
            total
        );
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
    use crate::chart::selector::ChartType;
    use crate::test_helpers::make_recommendation;

    fn table_params() -> TableParams {
        TableParams {
            chart_type_override: None,
            agg: AggFunction::Sum,
            sort: None,
            limit: None,
            sort_flag: None,
        }
    }

    #[test]
    fn test_print_table_bar_aggregates_without_cli() {
        let rec = make_recommendation(ChartType::Bar, "city", Some("revenue"), None);
        let headers = vec!["city".to_string(), "revenue".to_string()];
        let rows = vec![
            vec!["Tokyo".to_string(), "1000".to_string()],
            vec!["Tokyo".to_string(), "2000".to_string()],
            vec!["Osaka".to_string(), "1500".to_string()],
        ];
        let schema = crate::test_helpers::make_schema(&[
            ("city", crate::infer::types::DataType::Categorical),
            ("revenue", crate::infer::types::DataType::Quantitative),
        ]);
        assert!(print_table(&rec, &headers, &rows, &table_params(), &schema).is_ok());
    }

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
