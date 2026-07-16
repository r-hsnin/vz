//! Sparkline output for diff mode (both categorical and temporal).

use std::path::Path;

use crate::diff::{DiffResult, DiffTimeSeries};
use crate::sparkline;

/// Print categorical diff as sparkline: `Δ revenue  ▁▃▅▇  (+45%)`
pub(super) fn print_diff_spark(diff: &DiffResult) {
    let deltas: Vec<f64> = diff.entries.iter().map(|e| e.delta).collect();
    let spark = sparkline::sparkline_from_values(&deltas);

    let overall = match diff.overall_pct {
        Some(pct) if pct > 0.0 => format!("(+{:.0}%)", pct),
        Some(pct) if pct < 0.0 => format!("({:.0}%)", pct),
        Some(_) => "(0%)".to_string(),
        None => String::new(),
    };

    println!("Δ {}  {}  {}", diff.y_column, spark, overall);
}

/// Print temporal diff as sparkline.
pub(super) fn print_diff_line_spark(ts: &DiffTimeSeries, before_path: &Path, after_path: &Path) {
    let before_name = before_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("before");
    let after_name = after_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("after");

    let before_values: Vec<f64> = ts.before.iter().map(|(_, y)| *y).collect();
    let after_values: Vec<f64> = ts.after.iter().map(|(_, y)| *y).collect();

    let before_spark = sparkline::sparkline_from_values(&before_values);
    let after_spark = sparkline::sparkline_from_values(&after_values);

    let overall = match ts.overall_pct {
        Some(pct) if pct > 0.0 => format!("(+{:.0}%)", pct),
        Some(pct) if pct < 0.0 => format!("({:.0}%)", pct),
        Some(_) => "(0%)".to_string(),
        None => String::new(),
    };

    println!("{}  {}", before_name, before_spark);
    println!("{}  {}  {}", after_name, after_spark, overall);
}
