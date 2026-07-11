//! Summary line rendering for oneshot mode.

use crate::chart::selector::{ChartRecommendation, ChartType};
use crate::cli::AggFunction;
use crate::render::format_number_pub;

use super::ansi;

/// Print a one-line summary before the chart.
pub fn print_summary(
    recommendation: &ChartRecommendation,
    chart_type: ChartType,
    headers: &[String],
    rows: &[Vec<String>],
    extra_y_columns: &[(String, Option<String>)],
    agg: AggFunction,
) {
    let mut parts = vec![format!("{:?}", chart_type)];
    parts.push(format!("x={}", recommendation.x_column));
    if let Some(ref y) = recommendation.y_column {
        let y_display = if agg == AggFunction::Sum {
            y.clone()
        } else {
            format!("{}({})", agg_label(agg), y)
        };
        // Only show raw Y range when using default aggregation (Sum).
        // For other agg functions, the raw range is misleading.
        if agg == AggFunction::Sum {
            let y_idx = headers.iter().position(|h| h == y);
            let stats = y_idx.and_then(|idx| compute_y_stats(rows, idx));
            if let Some((min, max)) = stats {
                parts.push(format!(
                    "y={} ({}–{})",
                    y_display,
                    format_number_pub(min),
                    format_number_pub(max)
                ));
            } else {
                parts.push(format!("y={}", y_display));
            }
        } else {
            parts.push(format!("y={}", y_display));
        }
    }
    // Show extra Y columns
    if !extra_y_columns.is_empty() {
        let names: Vec<&str> = extra_y_columns
            .iter()
            .map(|(col, label)| label.as_deref().unwrap_or(col.as_str()))
            .collect();
        parts.push(format!("y+={}", names.join(",")));
    }
    if let Some(ref c) = recommendation.color_column {
        let legend = color_legend_hint(c, headers, rows);
        parts.push(legend);
    }
    parts.push(format!("{} rows", rows.len()));

    let extra_names: Vec<&str> = extra_y_columns
        .iter()
        .map(|(col, _)| col.as_str())
        .collect();
    if let Some(hint) = unused_columns_hint_with_extra(recommendation, headers, &extra_names) {
        parts.push(hint);
    }

    if ansi::should_colorize() {
        // Main info in dim gray, actionable hints in brighter yellow
        let main_parts: Vec<&String> = parts.iter().take(parts.len().saturating_sub(1)).collect();
        if let Some(hint) = parts.last() {
            if hint.starts_with('+') || hint.contains("try ") {
                // The last part is an actionable hint — highlight it
                let main_line = main_parts
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(" │ ");
                eprintln!("\x1b[90m{} │ \x1b[33m{}\x1b[0m", main_line, hint);
            } else {
                eprintln!("\x1b[90m{}\x1b[0m", parts.join(" │ "));
            }
        } else {
            eprintln!("\x1b[90m{}\x1b[0m", parts.join(" │ "));
        }
    } else {
        eprintln!("{}", parts.join(" │ "));
    }
}

/// Color name for ANSI display corresponding to SERIES_COLORS palette.
const COLOR_NAMES: &[&str] = &["cyan", "yellow", "green", "magenta", "red", "blue"];

/// Format a color legend for the summary line, e.g. `color=city [Tokyo=cyan, Osaka=yellow]`.
pub fn color_legend_hint(color_col: &str, headers: &[String], rows: &[Vec<String>]) -> String {
    let color_idx = headers.iter().position(|h| h == color_col);
    let unique_values: Vec<String> = match color_idx {
        Some(idx) => {
            let mut seen: Vec<String> = Vec::new();
            for row in rows {
                let val = match row.get(idx).filter(|v| !v.is_empty()) {
                    Some(v) => v,
                    None => continue,
                };
                if !seen.contains(val) {
                    seen.push(val.clone());
                }
            }
            seen
        }
        None => Vec::new(),
    };

    if unique_values.is_empty() {
        return format!("color={}", color_col);
    }

    let mappings: Vec<String> = unique_values
        .iter()
        .enumerate()
        .take(COLOR_NAMES.len())
        .map(|(i, name)| format!("{}={}", name, COLOR_NAMES[i]))
        .collect();

    let suffix = if unique_values.len() > COLOR_NAMES.len() {
        format!(" +{}", unique_values.len() - COLOR_NAMES.len())
    } else {
        String::new()
    };

    format!("color={} [{}{}]", color_col, mappings.join(", "), suffix)
}

/// Format unused columns hint, also excluding extra Y columns from the "unused" set.
pub fn unused_columns_hint_with_extra(
    recommendation: &ChartRecommendation,
    headers: &[String],
    extra_y_names: &[&str],
) -> Option<String> {
    let mut used: Vec<&str> = [
        Some(recommendation.x_column.as_str()),
        recommendation.y_column.as_deref(),
        recommendation.color_column.as_deref(),
    ]
    .into_iter()
    .flatten()
    .collect();
    used.extend(extra_y_names);
    let unused: Vec<&str> = headers
        .iter()
        .map(|h| h.as_str())
        .filter(|h| !used.contains(h))
        .collect();
    if unused.is_empty() {
        return None;
    }
    let names = if unused.len() <= 3 {
        unused.join(", ")
    } else {
        format!("{}, {}…", unused[0], unused[1])
    };
    let suggestion = if unused.len() == 1 {
        let y_name = recommendation.y_column.as_deref().unwrap_or("");
        if y_name.is_empty() {
            format!(" (try -y {})", unused[0])
        } else {
            format!(" (try -y {},{} or -c {})", y_name, unused[0], unused[0])
        }
    } else {
        String::new()
    };
    Some(format!("+{}: {}{}", unused.len(), names, suggestion))
}

/// Shorthand for tests: unused_columns_hint without extra Y.
#[cfg(test)]
pub fn unused_columns_hint(
    recommendation: &ChartRecommendation,
    headers: &[String],
) -> Option<String> {
    unused_columns_hint_with_extra(recommendation, headers, &[])
}

/// Compute min and max of Y values.
fn compute_y_stats(rows: &[Vec<String>], y_idx: usize) -> Option<(f64, f64)> {
    let values: Vec<f64> = rows
        .iter()
        .filter_map(|row| {
            row.get(y_idx)
                .and_then(|v| v.replace(',', "").parse::<f64>().ok())
        })
        .collect();

    if values.is_empty() {
        return None;
    }

    let min = values.iter().copied().fold(f64::INFINITY, f64::min);
    let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    Some((min, max))
}

/// Human-readable label for aggregation function.
fn agg_label(agg: AggFunction) -> &'static str {
    match agg {
        AggFunction::Sum => "sum",
        AggFunction::Mean => "mean",
        AggFunction::Count => "count",
        AggFunction::Max => "max",
        AggFunction::Min => "min",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_legend_hint_basic() {
        let headers = vec!["city".to_string(), "revenue".to_string()];
        let rows = vec![
            vec!["Tokyo".to_string(), "1000".to_string()],
            vec!["Osaka".to_string(), "2000".to_string()],
        ];
        let result = color_legend_hint("city", &headers, &rows);
        assert!(result.contains("Tokyo=cyan"));
        assert!(result.contains("Osaka=yellow"));
    }

    #[test]
    fn test_color_legend_hint_missing_column() {
        let headers = vec!["x".to_string()];
        let rows = vec![vec!["a".to_string()]];
        let result = color_legend_hint("missing", &headers, &rows);
        assert_eq!(result, "color=missing");
    }

    #[test]
    fn test_unused_columns_hint_none_when_all_used() {
        let rec = ChartRecommendation {
            chart_type: ChartType::Line,
            x_column: "date".to_string(),
            y_column: Some("revenue".to_string()),
            color_column: Some("city".to_string()),
        };
        let headers = vec![
            "date".to_string(),
            "revenue".to_string(),
            "city".to_string(),
        ];
        assert_eq!(unused_columns_hint(&rec, &headers), None);
    }

    #[test]
    fn test_unused_columns_hint_shows_unused() {
        let rec = ChartRecommendation {
            chart_type: ChartType::Line,
            x_column: "date".to_string(),
            y_column: Some("revenue".to_string()),
            color_column: None,
        };
        let headers = vec![
            "date".to_string(),
            "revenue".to_string(),
            "city".to_string(),
            "profit".to_string(),
        ];
        let hint = unused_columns_hint(&rec, &headers).unwrap();
        assert!(hint.contains("+2"));
        assert!(hint.contains("city"));
        assert!(hint.contains("profit"));
    }

    #[test]
    fn test_unused_columns_hint_truncates_many() {
        let rec = ChartRecommendation {
            chart_type: ChartType::Line,
            x_column: "x".to_string(),
            y_column: None,
            color_column: None,
        };
        let headers = vec![
            "x".to_string(),
            "a".to_string(),
            "b".to_string(),
            "c".to_string(),
            "d".to_string(),
        ];
        let hint = unused_columns_hint(&rec, &headers).unwrap();
        assert!(hint.contains("+4"));
        assert!(hint.contains("a"));
        assert!(hint.contains("b"));
        assert!(hint.contains('…'));
    }

    #[test]
    fn test_unused_columns_hint_single_suggests_command() {
        let rec = ChartRecommendation {
            chart_type: ChartType::Line,
            x_column: "date".to_string(),
            y_column: Some("revenue".to_string()),
            color_column: None,
        };
        let headers = vec![
            "date".to_string(),
            "revenue".to_string(),
            "profit".to_string(),
        ];
        let hint = unused_columns_hint(&rec, &headers).unwrap();
        assert!(hint.contains("+1: profit"));
        assert!(hint.contains("-y revenue,profit"));
    }

    #[test]
    fn test_compute_y_stats_basic() {
        let rows = vec![
            vec!["2024-01".to_string(), "100".to_string()],
            vec!["2024-02".to_string(), "200".to_string()],
            vec!["2024-03".to_string(), "300".to_string()],
        ];
        let (min, max) = compute_y_stats(&rows, 1).unwrap();
        assert!((min - 100.0).abs() < f64::EPSILON);
        assert!((max - 300.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_compute_y_stats_empty() {
        let rows: Vec<Vec<String>> = vec![];
        assert_eq!(compute_y_stats(&rows, 0), None);
    }

    #[test]
    fn test_agg_label_display() {
        assert_eq!(agg_label(AggFunction::Sum), "sum");
        assert_eq!(agg_label(AggFunction::Mean), "mean");
        assert_eq!(agg_label(AggFunction::Count), "count");
        assert_eq!(agg_label(AggFunction::Max), "max");
        assert_eq!(agg_label(AggFunction::Min), "min");
    }
}
