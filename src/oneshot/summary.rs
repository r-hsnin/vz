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
    pub series_colors: &'a [ratatui::style::Color],
}

/// Print a one-line summary before the chart.
pub fn print_summary(ctx: &SummaryContext<'_>) {
    let parts = build_summary_parts(ctx);
    format_and_print_parts(&parts);
}

/// Build the summary parts vector (pure logic, no IO).
pub fn build_summary_parts(ctx: &SummaryContext<'_>) -> Vec<String> {
    let mut parts = vec![format!("{}", ctx.chart_type)];
    parts.push(format!("x={}", ctx.recommendation.x_column));

    if let Some(ref y) = ctx.recommendation.y_column {
        let y_idx = crate::chart::data_builder::column_index(ctx.headers, y);
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
        parts.push(color_legend_hint(
            c,
            ctx.headers,
            ctx.rows,
            ctx.series_colors,
        ));
    }
    parts.push(if ctx.skipped_rows > 0 {
        format!(
            "{} {} ({} skipped)",
            ctx.rows.len(),
            if ctx.rows.len() == 1 { "row" } else { "rows" },
            ctx.skipped_rows
        )
    } else {
        let n = ctx.rows.len();
        format!("{} {}", n, if n == 1 { "row" } else { "rows" })
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
    let colorize = ansi::should_colorize_stderr();
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

/// Human-readable name for a ratatui Color value.
fn color_display_name(color: ratatui::style::Color) -> &'static str {
    use ratatui::style::Color;
    match color {
        Color::Cyan => "cyan",
        Color::Yellow => "yellow",
        Color::Green => "green",
        Color::Magenta => "magenta",
        Color::Red => "red",
        Color::Blue => "blue",
        Color::White => "white",
        Color::Black => "black",
        Color::Gray => "gray",
        Color::DarkGray => "darkgray",
        _ => "color",
    }
}

/// Format a color legend for the summary line, e.g. `color=city [Tokyo=cyan, Osaka=yellow]`.
pub fn color_legend_hint(
    color_col: &str,
    headers: &[String],
    rows: &[Vec<String>],
    series_colors: &[ratatui::style::Color],
) -> String {
    let color_idx = crate::chart::data_builder::column_index(headers, color_col);
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
        .take(series_colors.len())
        .map(|(i, name)| format!("{}={}", name, color_display_name(series_colors[i])))
        .collect();

    let suffix = if unique_values.len() > series_colors.len() {
        format!(" +{}", unique_values.len() - series_colors.len())
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

    crate::util::min_max(&values)
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
    include!("summary_tests.rs");
}
