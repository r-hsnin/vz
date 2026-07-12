//! Summary line rendering for oneshot mode.

use crate::chart::selector::{ChartRecommendation, ChartType};
use crate::cli::AggFunction;
use crate::render::format_number_pub;

use super::ansi;

/// Context for building a summary line.
pub struct SummaryContext<'a> {
    pub recommendation: &'a ChartRecommendation,
    pub chart_type: ChartType,
    pub headers: &'a [String],
    pub rows: &'a [Vec<String>],
    pub extra_y_columns: &'a [(String, Option<String>)],
    pub agg: AggFunction,
    pub agg_stats: Option<(f64, f64)>,
    pub skipped_rows: usize,
}

/// Print a one-line summary before the chart.
pub fn print_summary(ctx: &SummaryContext<'_>) {
    let parts = build_summary_parts(ctx);
    format_and_print_parts(&parts);
}

/// Build the summary parts vector (pure logic, no IO).
pub fn build_summary_parts(ctx: &SummaryContext<'_>) -> Vec<String> {
    let mut parts = vec![format!("{:?}", ctx.chart_type)];
    parts.push(format!("x={}", ctx.recommendation.x_column));

    if let Some(ref y) = ctx.recommendation.y_column {
        let y_idx = ctx.headers.iter().position(|h| h == y);
        let y_part = format_y_part(y, ctx.agg, ctx.agg_stats, ctx.rows, y_idx, ctx.chart_type);
        parts.push(y_part);
        // Add trend annotation for line/scatter
        if ctx.chart_type != ChartType::Bar
            && let Some(idx) = y_idx
            && let Some(trend) = trend_annotation(ctx.rows, idx)
        {
            parts.push(trend);
        }
    }

    if !ctx.extra_y_columns.is_empty() {
        let names: Vec<&str> = ctx
            .extra_y_columns
            .iter()
            .map(|(col, label)| label.as_deref().unwrap_or(col.as_str()))
            .collect();
        parts.push(format!("y+={}", names.join(",")));
    }
    if let Some(ref c) = ctx.recommendation.color_column {
        parts.push(color_legend_hint(c, ctx.headers, ctx.rows));
    }
    parts.push(if ctx.skipped_rows > 0 {
        format!("{} rows ({} skipped)", ctx.rows.len(), ctx.skipped_rows)
    } else {
        format!("{} rows", ctx.rows.len())
    });

    let extra_names: Vec<&str> = ctx
        .extra_y_columns
        .iter()
        .map(|(col, _)| col.as_str())
        .collect();
    if let Some(hint) =
        unused_columns_hint_with_extra(ctx.recommendation, ctx.headers, &extra_names)
    {
        parts.push(hint);
    }
    parts
}

/// Format the Y-axis display part including range and sparkline.
fn format_y_part(
    y: &str,
    agg: AggFunction,
    agg_stats: Option<(f64, f64)>,
    rows: &[Vec<String>],
    y_idx: Option<usize>,
    chart_type: ChartType,
) -> String {
    let y_display = if agg == AggFunction::Sum {
        y.to_string()
    } else {
        format!("{}({})", agg_label(agg), y)
    };

    let mut result = if agg == AggFunction::Sum {
        let stats = agg_stats.or_else(|| y_idx.and_then(|idx| compute_y_stats(rows, idx)));
        if let Some((min, max)) = stats {
            format!(
                "y={} ({}–{})",
                y_display,
                format_number_pub(min),
                format_number_pub(max)
            )
        } else {
            format!("y={}", y_display)
        }
    } else {
        format!("y={}", y_display)
    };

    // Append sparkline (skip for bar charts)
    if chart_type != ChartType::Bar
        && let Some(idx) = y_idx
        && let Some(spark) = sparkline(rows, idx)
    {
        result.push(' ');
        result.push_str(&spark);
    }

    result
}

/// Compute trend annotation for line/scatter charts.
/// Returns arrow + percentage change from first to last value.
fn trend_annotation(rows: &[Vec<String>], y_idx: usize) -> Option<String> {
    let values: Vec<f64> = rows
        .iter()
        .filter_map(|r| r.get(y_idx)?.parse::<f64>().ok())
        .collect();
    if values.len() < 2 {
        return None;
    }
    let first = values[0];
    let last = *values.last()?;
    if first.abs() < f64::EPSILON {
        return None;
    }
    let pct = ((last - first) / first) * 100.0;
    if pct > 5.0 {
        Some(format!("↑ {:+.0}%", pct))
    } else if pct < -5.0 {
        Some(format!("↓ {:+.0}%", pct))
    } else {
        Some("→ stable".to_string())
    }
}

/// Generate a sparkline string from numeric values.
/// Maps values to 8 Unicode block characters (▁▂▃▄▅▆▇█).
/// Samples to at most 8 points for compact display.
fn sparkline(rows: &[Vec<String>], y_idx: usize) -> Option<String> {
    let values: Vec<f64> = rows
        .iter()
        .filter_map(|r| r.get(y_idx)?.parse::<f64>().ok())
        .collect();
    if values.len() < 2 {
        return None;
    }
    let sampled = crate::sparkline::sample_values(&values, 8);
    Some(crate::sparkline::sparkline_from_values(&sampled))
}

/// Format and print parts with optional ANSI coloring.
fn format_and_print_parts(parts: &[String]) {
    let max_width = summary_max_width();
    let colorize = ansi::should_colorize();
    let sep = if colorize { " │ " } else { " | " };

    let is_hint = |s: &str| s.starts_with('+') || s.contains("try ");
    let emit = |line: &str| {
        if colorize {
            eprintln!("\x1b[90m{}\x1b[0m", line);
        } else {
            eprintln!("{}", line);
        }
    };

    let last = parts.last();
    if let Some(hint) = last.filter(|h| is_hint(h)) {
        let main_line = parts[..parts.len() - 1].join(sep);
        let full = format!("{}{}{}", main_line, sep, hint);
        if full.chars().count() <= max_width {
            emit(&full);
        } else {
            emit(&truncate_to_width(&main_line, max_width));
            emit(&format!(
                "  {}",
                truncate_to_width(hint, max_width.saturating_sub(2))
            ));
        }
    } else {
        let full = parts.join(sep);
        emit(&truncate_to_width(&full, max_width));
    }
}

/// Get maximum width for summary line (matches chart width).
fn summary_max_width() -> usize {
    if !std::io::IsTerminal::is_terminal(&std::io::stderr()) {
        return 120; // generous width for piped stderr
    }
    crossterm::terminal::size()
        .map(|(w, _)| w as usize)
        .unwrap_or(80)
}

/// Truncate a string to fit within max_width characters, adding "…" if truncated.
fn truncate_to_width(s: &str, max_width: usize) -> String {
    // Count display characters (simplified: treat each char as width 1)
    let char_count = s.chars().count();
    if char_count <= max_width {
        return s.to_string();
    }
    if max_width <= 1 {
        return "…".to_string();
    }
    let truncated: String = s.chars().take(max_width - 1).collect();
    format!("{}…", truncated)
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

#[test]
fn test_build_summary_parts_basic() {
    let rec = ChartRecommendation {
        chart_type: ChartType::Line,
        x_column: "date".to_string(),
        y_column: Some("revenue".to_string()),
        color_column: None,
    };
    let headers = vec!["date".to_string(), "revenue".to_string()];
    let rows = vec![
        vec!["2024-01".to_string(), "1000".to_string()],
        vec!["2024-02".to_string(), "2000".to_string()],
    ];
    let parts = build_summary_parts(&SummaryContext {
        recommendation: &rec,
        chart_type: ChartType::Line,
        headers: &headers,
        rows: &rows,
        extra_y_columns: &[],
        agg: AggFunction::Sum,
        agg_stats: None,
        skipped_rows: 0,
    });
    assert_eq!(parts[0], "Line");
    assert_eq!(parts[1], "x=date");
    assert!(parts[2].starts_with("y=revenue"));
    assert!(parts[2].contains("1.0k"));
    assert!(parts[2].contains("2.0k"));
    assert!(parts.iter().any(|p| p.contains("2 rows")));
}

#[test]
fn test_build_summary_parts_with_agg_stats() {
    let rec = ChartRecommendation {
        chart_type: ChartType::Bar,
        x_column: "city".to_string(),
        y_column: Some("revenue".to_string()),
        color_column: None,
    };
    let headers = vec!["city".to_string(), "revenue".to_string()];
    let rows = vec![
        vec!["Tokyo".to_string(), "1000".to_string()],
        vec!["Tokyo".to_string(), "2000".to_string()],
    ];
    // With agg_stats override, summary should use provided stats
    let parts = build_summary_parts(&SummaryContext {
        recommendation: &rec,
        chart_type: ChartType::Bar,
        headers: &headers,
        rows: &rows,
        extra_y_columns: &[],
        agg: AggFunction::Sum,
        agg_stats: Some((800.0, 4200.0)),
        skipped_rows: 0,
    });
    assert!(
        parts[2].contains("4.2k"),
        "Expected 4.2k in parts: {:?}",
        parts
    );
}

#[test]
fn test_build_summary_parts_non_sum_agg() {
    let rec = ChartRecommendation {
        chart_type: ChartType::Bar,
        x_column: "city".to_string(),
        y_column: Some("revenue".to_string()),
        color_column: None,
    };
    let headers = vec!["city".to_string(), "revenue".to_string()];
    let rows = vec![vec!["Tokyo".to_string(), "1000".to_string()]];
    let parts = build_summary_parts(&SummaryContext {
        recommendation: &rec,
        chart_type: ChartType::Bar,
        headers: &headers,
        rows: &rows,
        extra_y_columns: &[],
        agg: AggFunction::Mean,
        agg_stats: None,
        skipped_rows: 0,
    });
    // Should show mean(revenue) without range (since range is misleading for non-sum)
    assert!(
        parts[2].contains("mean(revenue)"),
        "Expected mean(revenue): {:?}",
        parts
    );
    assert!(
        !parts[2].contains('–'),
        "Should not contain range for non-sum agg"
    );
}

#[test]
fn test_build_summary_parts_extra_y() {
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
    let rows = vec![vec![
        "2024-01".to_string(),
        "1000".to_string(),
        "200".to_string(),
    ]];
    let extra = vec![("profit".to_string(), None)];
    let parts = build_summary_parts(&SummaryContext {
        recommendation: &rec,
        chart_type: ChartType::Line,
        headers: &headers,
        rows: &rows,
        extra_y_columns: &extra,
        agg: AggFunction::Sum,
        agg_stats: None,
        skipped_rows: 0,
    });
    assert!(
        parts.iter().any(|p| p.contains("y+=profit")),
        "Expected y+=profit: {:?}",
        parts
    );
}

#[test]
fn test_sparkline_basic() {
    let rows = vec![
        vec!["a".to_string(), "1".to_string()],
        vec!["b".to_string(), "5".to_string()],
        vec!["c".to_string(), "3".to_string()],
        vec!["d".to_string(), "10".to_string()],
    ];
    let spark = sparkline(&rows, 1).unwrap();
    assert_eq!(spark.chars().count(), 4);
    // First value (1) should be lowest block, last (10) should be highest
    let chars: Vec<char> = spark.chars().collect();
    assert_eq!(chars[0], '▁'); // min value
    assert_eq!(chars[3], '█'); // max value
}

#[test]
fn test_sparkline_single_value_returns_none() {
    let rows = vec![vec!["a".to_string(), "5".to_string()]];
    assert!(sparkline(&rows, 1).is_none());
}

#[test]
fn test_sparkline_constant_values() {
    let rows = vec![
        vec!["a".to_string(), "5".to_string()],
        vec!["b".to_string(), "5".to_string()],
        vec!["c".to_string(), "5".to_string()],
    ];
    let spark = sparkline(&rows, 1).unwrap();
    // All same value → all middle blocks
    assert!(spark.chars().all(|c| c == '▄'));
}

#[test]
fn test_trend_annotation_uptrend() {
    let rows = vec![
        vec!["a".to_string(), "100".to_string()],
        vec!["b".to_string(), "200".to_string()],
    ];
    let trend = trend_annotation(&rows, 1).unwrap();
    assert!(
        trend.contains('↑'),
        "Expected ↑ for uptrend, got: {}",
        trend
    );
    assert!(trend.contains("+100%"), "Expected +100%, got: {}", trend);
}

#[test]
fn test_trend_annotation_downtrend() {
    let rows = vec![
        vec!["a".to_string(), "100".to_string()],
        vec!["b".to_string(), "50".to_string()],
    ];
    let trend = trend_annotation(&rows, 1).unwrap();
    assert!(
        trend.contains('↓'),
        "Expected ↓ for downtrend, got: {}",
        trend
    );
}

#[test]
fn test_trend_annotation_stable() {
    let rows = vec![
        vec!["a".to_string(), "100".to_string()],
        vec!["b".to_string(), "103".to_string()],
    ];
    let trend = trend_annotation(&rows, 1).unwrap();
    assert!(trend.contains('→'), "Expected → for stable, got: {}", trend);
    assert!(
        trend.contains("stable"),
        "Expected 'stable', got: {}",
        trend
    );
}

#[test]
fn test_trend_annotation_single_row_returns_none() {
    let rows = vec![vec!["a".to_string(), "100".to_string()]];
    assert!(trend_annotation(&rows, 1).is_none());
}

#[test]
fn test_truncate_to_width_short_string() {
    assert_eq!(truncate_to_width("hello", 80), "hello");
}

#[test]
fn test_truncate_to_width_exact() {
    assert_eq!(truncate_to_width("12345", 5), "12345");
}

#[test]
fn test_truncate_to_width_overflow() {
    let result = truncate_to_width("abcdefghij", 6);
    assert_eq!(result, "abcde…");
    assert_eq!(result.chars().count(), 6);
}

#[test]
fn test_truncate_to_width_one() {
    assert_eq!(truncate_to_width("hello", 1), "…");
}

#[test]
fn test_build_summary_parts_shows_skipped_rows() {
    let rec = ChartRecommendation {
        chart_type: ChartType::Line,
        x_column: "date".to_string(),
        y_column: Some("revenue".to_string()),
        color_column: None,
    };
    let headers = vec!["date".to_string(), "revenue".to_string()];
    let rows = vec![
        vec!["2024-01".to_string(), "1000".to_string()],
        vec!["2024-02".to_string(), "N/A".to_string()],
        vec!["2024-03".to_string(), "2000".to_string()],
    ];
    let parts = build_summary_parts(&SummaryContext {
        recommendation: &rec,
        chart_type: ChartType::Line,
        headers: &headers,
        rows: &rows,
        extra_y_columns: &[],
        agg: AggFunction::Sum,
        agg_stats: None,
        skipped_rows: 1,
    });
    assert!(
        parts.iter().any(|p| p.contains("3 rows (1 skipped)")),
        "Expected '3 rows (1 skipped)' in parts: {:?}",
        parts
    );
}

#[test]
fn test_truncate_to_width_fits() {
    assert_eq!(truncate_to_width("hello", 10), "hello");
}

#[test]
fn test_truncate_to_width_truncates() {
    let result = truncate_to_width("hello world", 6);
    assert_eq!(result, "hello…");
}

#[test]
fn test_truncate_to_width_min() {
    assert_eq!(truncate_to_width("abc", 1), "…");
}
