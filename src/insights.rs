//! Plain-language insights: the takeaway for non-engineers.
//!
//! `vz data.csv` renders a chart plus a terse technical summary line. This
//! module translates the same parsed numbers into 0–3 plain sentences:
//! overall movement, extremes, and the biggest single move (line/scatter);
//! the leader and its share (bar); where values cluster (histogram); the
//! hottest cell (heatmap).
//!
//! Pure logic, no IO — callers decide where to print (oneshot stderr,
//! JSON `insights`). All numbers go through `util::parse_number`, so `$100`
//! means 100 here exactly as on every other path.

use crate::chart::data_builder;
use crate::chart::selector::AggFunction;
use crate::chart::selector::ChartType;
use crate::render::format_number;

/// Everything the engine needs; mirrors the resolved oneshot query.
pub struct InsightRequest<'a> {
    pub chart_type: ChartType,
    pub x_column: &'a str,
    pub y_column: Option<&'a str>,
    pub color_column: Option<&'a str>,
    pub headers: &'a [String],
    pub rows: &'a [Vec<String>],
    pub agg: AggFunction,
    pub bins: Option<usize>,
}

/// Build 0–3 plain-language takeaways for the rendered chart.
/// Empty when there is nothing meaningful to say (< 2 points, no parseable
/// values, unknown columns).
pub fn build_insights(req: &InsightRequest<'_>) -> Vec<String> {
    match req.chart_type {
        ChartType::Line | ChartType::Scatter => line_insights(req),
        ChartType::Bar => bar_insights(req),
        ChartType::Histogram => histogram_insights(req),
        ChartType::Heatmap => heatmap_insights(req),
    }
}

/// Single shared value stream for a Y column: (x_label, value) in row order.
/// Non-finite / non-numeric cells are skipped, same as every chart path.
fn y_stream(req: &InsightRequest<'_>) -> Option<(usize, Vec<(String, f64)>)> {
    let y_idx = req
        .y_column
        .and_then(|y| data_builder::column_index(req.headers, y))?;
    let x_idx = data_builder::column_index(req.headers, req.x_column)?;
    let pts: Vec<(String, f64)> = req
        .rows
        .iter()
        .filter_map(|r| {
            let x = r.get(x_idx).cloned().unwrap_or_default();
            let y = r
                .get(y_idx)
                .and_then(|v| crate::util::parse_number(v))
                .filter(|v| v.is_finite())?;
            Some((x, y))
        })
        .collect();
    Some((y_idx, pts))
}

/// Overall movement in plain words + percent. `rose`/`fell` only outside the
/// ±5% stable band that `util::trend_from_slice` uses; `None` inside it.
fn movement_phrase(first: f64, last: f64) -> Option<(String, String)> {
    let label = crate::util::trend_from_slice(&[first, last])?;
    let pct = ((last - first) / first.abs()) * 100.0;
    let arrow = label.chars().next().unwrap_or('→');
    match arrow {
        // Sign is redundant with the verb: "rose 80%", "fell 50%".
        '↑' => Some(("rose".to_string(), format!("{pct:.0}%"))),
        '↓' => Some(("fell".to_string(), format!("{pct:.0}%"))),
        _ => None,
    }
}

/// Line/scatter: overall move, extremes, biggest single move.
/// `-c` grouping is honored: per-group peaks/moves are reported when they
/// beat the ungrouped story, so multi-plan files don't read as one jumbled line.
fn line_insights(req: &InsightRequest<'_>) -> Vec<String> {
    let y = req.y_column.unwrap_or("value");
    let Some((_, pts)) = y_stream(req) else {
        return vec![];
    };
    if pts.len() < 2 {
        return vec![];
    }
    let mut out = Vec::new();
    let first = pts[0].1;
    let last = pts[pts.len() - 1].1;
    if let Some((verb, pct)) = movement_phrase(first, last) {
        let from = format_number(first);
        let to = format_number(last);
        out.push(format!("{y} {verb} {pct} overall ({from} → {to})."));
    }
    let (min_i, max_i) = {
        let mut min_i = 0;
        let mut max_i = 0;
        for (i, &(_, v)) in pts.iter().enumerate() {
            if v < pts[min_i].1 {
                min_i = i;
            }
            if v > pts[max_i].1 {
                max_i = i;
            }
        }
        (min_i, max_i)
    };
    if min_i != max_i {
        out.push(format!(
            "Peaked at {} ({}) and bottomed at {} ({}).",
            format_number(pts[max_i].1),
            pts[max_i].0,
            format_number(pts[min_i].1),
            pts[min_i].0,
        ));
    }
    if let Some(jump) = biggest_jump(&pts).filter(|_| req.color_column.is_none()) {
        out.push(jump);
    }
    if let Some(grouped) = grouped_line_insight(req) {
        out.push(grouped);
    }
    out.truncate(3);
    out
}

/// One grouped takeaway for `-c` overlays: which group moved the most
/// end-to-end (by absolute percent), e.g. "Enterprise rose 71% (280k → 480k),
/// the fastest of 3 plans." Silent without a color column or < 2 groups.
fn grouped_line_insight(req: &InsightRequest<'_>) -> Option<String> {
    let color = req.color_column?;
    let y_idx = req
        .y_column
        .and_then(|y| data_builder::column_index(req.headers, y))?;
    let color_idx = data_builder::column_index(req.headers, color)?;
    let mut groups: Vec<(String, Vec<f64>)> = Vec::new();
    for row in req.rows {
        let name = row.get(color_idx).cloned().unwrap_or_default();
        if name.is_empty() {
            continue;
        }
        let v = match row.get(y_idx).and_then(|s| crate::util::parse_number(s)) {
            Some(v) if v.is_finite() => v,
            _ => continue,
        };
        match groups.iter_mut().find(|(n, _)| n == &name) {
            Some((_, vals)) => vals.push(v),
            None => groups.push((name, vec![v])),
        }
    }
    if groups.len() < 2 {
        return None;
    }
    let mut best: Option<(String, String, String, f64, f64, f64)> = None;
    for (name, vals) in &groups {
        if vals.len() < 2 {
            continue;
        }
        let (first, last) = (vals[0], vals[vals.len() - 1]);
        let Some((verb, pct)) = movement_phrase(first, last) else {
            continue;
        };
        let score = ((last - first) / first.abs()).abs();
        if best.as_ref().is_none_or(|b| score > b.5) {
            best = Some((name.clone(), verb, pct, first, last, score));
        }
    }
    let (name, verb, pct, first, last, _) = best?;
    Some(format!(
        "{name} {verb} {pct} ({} → {}), the fastest of {} {}.",
        format_number(first),
        format_number(last),
        groups.len(),
        pluralize(req.color_column.unwrap_or("group"), groups.len()),
    ))
}

/// Naive plural: "plan" → "plans". Enough for column-name echoes.
fn pluralize(word: &str, n: usize) -> String {
    if n == 1 || word.ends_with('s') {
        word.to_string()
    } else {
        format!("{word}s")
    }
}

/// Biggest consecutive move (by absolute delta) in row order.
fn biggest_jump(pts: &[(String, f64)]) -> Option<String> {
    if pts.len() < 2 {
        return None;
    }
    let (mut best_i, mut best_delta) = (1, 0.0_f64);
    for i in 1..pts.len() {
        let delta = (pts[i].1 - pts[i - 1].1).abs();
        if delta > best_delta {
            best_delta = delta;
            best_i = i;
        }
    }
    if best_delta <= f64::EPSILON {
        return None;
    }
    let from = &pts[best_i - 1];
    let to = &pts[best_i];
    let verb = if to.1 > from.1 { "jumped" } else { "dropped" };
    Some(format!(
        "Biggest move: {verb} from {} ({}) to {} ({}).",
        from.0,
        format_number(from.1),
        to.0,
        format_number(to.1),
    ))
}

/// Bar: leader and share, runner-up gap. Aggregates with the same `agg`
/// the chart renders with, so `Tokyo leads` matches the tallest bar.
fn bar_insights(req: &InsightRequest<'_>) -> Vec<String> {
    let y = req.y_column.unwrap_or("value");
    let x_idx = match data_builder::column_index(req.headers, req.x_column) {
        Some(i) => i,
        None => return vec![],
    };
    let y_idx = match req
        .y_column
        .and_then(|c| data_builder::column_index(req.headers, c))
    {
        Some(i) => i,
        None => return vec![],
    };
    let (bar, _) =
        data_builder::aggregate_bar(req.rows, x_idx, y_idx, None, String::new(), req.agg);
    if bar.labels.len() < 2 || bar.values.iter().all(|v| *v == 0.0) {
        return vec![];
    }
    let mut order: Vec<usize> = (0..bar.values.len()).collect();
    order.sort_by(|&a, &b| {
        bar.values[b]
            .partial_cmp(&bar.values[a])
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let top = order[0];
    let second = order[1];
    let total: f64 = bar.values.iter().sum();
    let mut out = Vec::new();
    let mut lead = format!(
        "{} leads with {}.",
        bar.labels[top],
        format_number(bar.values[top])
    );
    if total.abs() > f64::EPSILON && req.agg == AggFunction::Sum {
        let share = bar.values[top] / total * 100.0;
        lead = format!(
            "{} leads with {} ({share:.0}% of total).",
            bar.labels[top],
            format_number(bar.values[top])
        );
    }
    let _ = y;
    out.push(lead);
    if bar.values[second].abs() > f64::EPSILON {
        let gap = (bar.values[top] - bar.values[second]) / bar.values[second].abs() * 100.0;
        let gap_txt = if gap < 10.0 {
            format!("{gap:.1}%")
        } else {
            format!("{gap:.0}%")
        };
        out.push(format!(
            "{} is next at {}, {gap_txt} behind {}.",
            bar.labels[second],
            format_number(bar.values[second]),
            bar.labels[top]
        ));
    }
    out.truncate(2);
    out
}

/// Histogram: where values cluster (densest bin) + range.
fn histogram_insights(req: &InsightRequest<'_>) -> Vec<String> {
    let col_idx = req
        .y_column
        .and_then(|c| data_builder::column_index(req.headers, c))
        .or_else(|| data_builder::column_index(req.headers, req.x_column));
    let Some(idx) = col_idx else {
        return vec![];
    };
    let hist = data_builder::build_histogram(req.rows, idx, None, String::new(), req.bins);
    if hist.values.len() < 2 {
        return vec![];
    }
    let bins = crate::render::compute_bins(&hist.values, hist.bin_count);
    if bins.is_empty() {
        return vec![];
    }
    let total: usize = bins.iter().map(|(_, _, c)| c).sum();
    if total == 0 {
        return vec![];
    }
    let (bi, &(s, e, c)) = bins
        .iter()
        .enumerate()
        .max_by_key(|(_, b)| b.2)
        .unwrap_or((0, &bins[0]));
    let _ = bi;
    let name = req.y_column.unwrap_or(req.x_column);
    let share = c as f64 / total as f64 * 100.0;
    let (min, max) = crate::util::min_max(&hist.values).unwrap_or((s, e));
    vec![
        format!(
            "Most {name} values sit between {} and {} ({c} of {total}, {share:.0}%).",
            format_number(s),
            format_number(e)
        ),
        format!(
            "{name} spans {} to {}.",
            format_number(min),
            format_number(max)
        ),
    ]
}

/// Heatmap: the hottest cell.
fn heatmap_insights(req: &InsightRequest<'_>) -> Vec<String> {
    let row_idx = match data_builder::column_index(req.headers, req.x_column) {
        Some(i) => i,
        None => return vec![],
    };
    let col_idx = match req
        .y_column
        .and_then(|c| data_builder::column_index(req.headers, c))
    {
        Some(i) => i,
        None => return vec![],
    };
    let heat = data_builder::build_heatmap_data(req.rows, row_idx, col_idx, None);
    if heat.max_count == 0 {
        return vec![];
    }
    let mut best = (0, 0);
    for (ri, row) in heat.counts.iter().enumerate() {
        for (ci, &c) in row.iter().enumerate() {
            if c > heat.counts[best.0][best.1] {
                best = (ri, ci);
            }
        }
    }
    let total: usize = heat.counts.iter().flatten().sum();
    let cell = heat.counts[best.0][best.1];
    let row = heat.row_labels.get(best.0).cloned().unwrap_or_default();
    let col = heat.col_labels.get(best.1).cloned().unwrap_or_default();
    let mut s = format!("Most common combo: {row} × {col} ({cell} rows");
    if total > 0 {
        let share = cell as f64 / total as f64 * 100.0;
        s.push_str(&format!(", {share:.0}% of all rows"));
    }
    s.push_str(").");
    vec![s]
}

/// Diff takeaway: net movement + biggest winner/loser, in plain words.
/// Each entry is (label, before, after, pct_change).
/// Ranks by absolute percent (what a non-engineer feels), falling back to
/// absolute delta when no percents exist (all-from-zero diffs).
pub fn diff_insights(entries: &[(String, f64, f64, Option<f64>)]) -> Vec<String> {
    if entries.is_empty() {
        return vec![];
    }
    let mut out = Vec::new();
    let ranked: Vec<&(String, f64, f64, Option<f64>)> = {
        let mut v: Vec<_> = entries.iter().collect();
        v.sort_by(|a, b| {
            rank_key(b)
                .partial_cmp(&rank_key(a))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        v
    };
    if let Some((label, before, after, pct)) = ranked.first().map(|r| (r.0.clone(), r.1, r.2, r.3))
    {
        let verb = if after > before { "grew" } else { "shrank" };
        let mut s = format!(
            "Biggest change: {label} {verb} from {} to {}",
            format_number(before),
            format_number(after)
        );
        if let Some(p) = pct {
            s.push_str(&format!(" ({p:+.0}%)"));
        }
        s.push('.');
        out.push(s);
    }
    let winners = entries.iter().filter(|(_, b, a, _)| *a > *b).count();
    let losers = entries.iter().filter(|(_, b, a, _)| *a < *b).count();
    if winners > 0 && losers > 0 {
        out.push(format!("{winners} improved, {losers} declined."));
    } else if winners > 0 && entries.len() > 1 {
        out.push(format!("All {winners} categories improved."));
    } else if losers > 0 && entries.len() > 1 {
        out.push(format!("All {losers} categories declined."));
    }
    out.truncate(2);
    out
}

/// Rank key for diff entries: absolute percent first (felt change), then
/// absolute delta so novel categories still order sensibly.
fn rank_key(e: &(String, f64, f64, Option<f64>)) -> (f64, f64) {
    (e.3.map(|p| p.abs()).unwrap_or(-1.0), (e.2 - e.1).abs())
}

/// Print takeaways to stderr after the summary line (oneshot default).
/// Silent when there is nothing to say. Honors NO_COLOR like the summary.
pub fn print_insights(req: &InsightRequest<'_>) {
    for line in &build_insights(req) {
        if crate::oneshot::ansi::should_colorize_stderr() {
            eprintln!("\x1b[2m💡 {}\x1b[0m", line);
        } else {
            eprintln!("💡 {line}");
        }
    }
}

/// JSON projection of the same takeaways (for `-o json` agents).
pub fn insights_json(req: &InsightRequest<'_>) -> Vec<String> {
    build_insights(req)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headers(cols: &[&str]) -> Vec<String> {
        cols.iter().map(|s| s.to_string()).collect()
    }

    fn rows(data: &[&[&str]]) -> Vec<Vec<String>> {
        data.iter()
            .map(|r| r.iter().map(|s| s.to_string()).collect())
            .collect()
    }

    fn req<'a>(
        chart_type: ChartType,
        x: &'a str,
        y: Option<&'a str>,
        headers: &'a [String],
        rows: &'a [Vec<String>],
    ) -> InsightRequest<'a> {
        InsightRequest {
            chart_type,
            x_column: x,
            y_column: y,
            color_column: None,
            headers,
            rows,
            agg: AggFunction::Sum,
            bins: None,
        }
    }

    fn req_colored<'a>(
        chart_type: ChartType,
        x: &'a str,
        y: Option<&'a str>,
        color: &'a str,
        headers: &'a [String],
        rows: &'a [Vec<String>],
    ) -> InsightRequest<'a> {
        InsightRequest {
            chart_type,
            x_column: x,
            y_column: y,
            color_column: Some(color),
            headers,
            rows,
            agg: AggFunction::Sum,
            bins: None,
        }
    }

    fn sales() -> (Vec<String>, Vec<Vec<String>>) {
        let h = headers(&["date", "city", "revenue", "profit"]);
        let r = rows(&[
            &["2024-01-01", "Tokyo", "1000", "200"],
            &["2024-02-01", "Osaka", "1500", "350"],
            &["2024-03-01", "Tokyo", "1200", "280"],
            &["2024-04-01", "Nagoya", "800", "150"],
            &["2024-05-01", "Tokyo", "2000", "500"],
            &["2024-06-01", "Osaka", "1800", "400"],
        ]);
        (h, r)
    }

    #[test]
    fn red_step_line_reports_growth() {
        let (h, r) = sales();
        let out = build_insights(&req(ChartType::Line, "date", Some("revenue"), &h, &r));
        assert!(
            out.iter()
                .any(|s| s.contains("rose 80%") && s.contains("revenue")),
            "{out:?}"
        );
        assert!(out.iter().any(|s| s.contains("Peaked")), "{out:?}");
        assert!(out.iter().any(|s| s.contains("Biggest move")), "{out:?}");
    }

    #[test]
    fn red_step_bar_names_leader() {
        let (h, r) = sales();
        let out = build_insights(&req(ChartType::Bar, "city", Some("revenue"), &h, &r));
        assert!(out.iter().any(|s| s.contains("Tokyo leads")), "{out:?}");
        // Tokyo = 1000+1200+2000 = 4200 of 8300 → 51%.
        assert!(out.iter().any(|s| s.contains("51%")), "{out:?}");
    }

    #[test]
    fn line_stable_produces_no_overall_sentence() {
        let h = headers(&["date", "revenue"]);
        let r = rows(&[&["2024-01", "100"], &["2024-02", "103"]]);
        let out = build_insights(&req(ChartType::Line, "date", Some("revenue"), &h, &r));
        assert!(
            !out.iter().any(|s| s.contains("rose") || s.contains("fell")),
            "stable band must stay silent: {out:?}"
        );
    }

    #[test]
    fn line_single_point_is_empty() {
        let h = headers(&["date", "revenue"]);
        let r = rows(&[&["2024-01", "100"]]);
        assert!(build_insights(&req(ChartType::Line, "date", Some("revenue"), &h, &r)).is_empty());
    }

    #[test]
    fn line_all_unparseable_is_empty() {
        let h = headers(&["date", "revenue"]);
        let r = rows(&[&["2024-01", "N/A"], &["2024-02", "missing"]]);
        assert!(build_insights(&req(ChartType::Line, "date", Some("revenue"), &h, &r)).is_empty());
    }

    #[test]
    fn line_handles_formatted_numbers() {
        let h = headers(&["date", "revenue"]);
        let r = rows(&[&["2024-01", "$1,000"], &["2024-02", "$2,000"]]);
        let out = build_insights(&req(ChartType::Line, "date", Some("revenue"), &h, &r));
        assert!(out.iter().any(|s| s.contains("rose 100%")), "{out:?}");
    }

    #[test]
    fn line_handles_zero_start_without_panic() {
        let h = headers(&["date", "revenue"]);
        let r = rows(&[&["2024-01", "0"], &["2024-02", "100"]]);
        let out = build_insights(&req(ChartType::Line, "date", Some("revenue"), &h, &r));
        assert!(
            !out.iter().any(|s| s.contains("rose") || s.contains("fell")),
            "near-zero start has no percent: {out:?}"
        );
    }

    #[test]
    fn bar_single_category_is_empty() {
        let h = headers(&["city", "revenue"]);
        let r = rows(&[&["Tokyo", "100"], &["Tokyo", "200"]]);
        assert!(build_insights(&req(ChartType::Bar, "city", Some("revenue"), &h, &r)).is_empty());
    }

    #[test]
    fn bar_unknown_column_is_empty() {
        let (h, r) = sales();
        assert!(build_insights(&req(ChartType::Bar, "city", Some("nope"), &h, &r)).is_empty());
        assert!(build_insights(&req(ChartType::Bar, "nope", Some("revenue"), &h, &r)).is_empty());
    }

    #[test]
    fn bar_mean_agg_still_names_leader() {
        let (h, r) = sales();
        let mut q = req(ChartType::Bar, "city", Some("revenue"), &h, &r);
        q.agg = AggFunction::Mean;
        // Osaka mean (1500+1800)/2=1650 > Tokyo mean (1000+1200+2000)/3=1400.
        let out = build_insights(&q);
        assert!(out.iter().any(|s| s.contains("Osaka leads")), "{out:?}");
        // Gap is 250/1400 = 17.9% → shown rounded ("18%"), precise only for close races.
        assert!(out.iter().any(|s| s.contains("18% behind")), "{out:?}");
    }

    #[test]
    fn histogram_names_densest_bin() {
        let h = headers(&["age"]);
        let r = rows(&[&["10"], &["11"], &["12"], &["50"], &["51"], &["90"]]);
        let out = build_insights(&req(ChartType::Histogram, "age", None, &h, &r));
        assert!(out.iter().any(|s| s.contains("Most age values")), "{out:?}");
        assert!(out.iter().any(|s| s.contains("spans")), "{out:?}");
    }

    #[test]
    fn heatmap_names_hottest_cell() {
        let h = headers(&["dept", "status"]);
        let r = rows(&[
            &["Engineering", "Active"],
            &["Engineering", "Active"],
            &["Marketing", "Active"],
        ]);
        let out = build_insights(&req(ChartType::Heatmap, "dept", Some("status"), &h, &r));
        assert!(
            out.iter().any(|s| s.contains("Engineering × Active")),
            "{out:?}"
        );
    }

    #[test]
    fn diff_ranks_by_absolute_percent() {
        let entries = vec![
            ("A".to_string(), 100.0, 110.0, Some(10.0)),
            ("B".to_string(), 1000.0, 800.0, Some(-20.0)),
        ];
        let out = diff_insights(&entries);
        assert!(
            out.iter().any(|s| s.contains("B") && s.contains("shrank")),
            "{out:?}"
        );
        assert!(out.iter().any(|s| s.contains("1 improved")), "{out:?}");
    }

    #[test]
    fn diff_ranks_absolute_delta_without_percents() {
        let entries = vec![
            ("A".to_string(), 0.0, 50.0, None),
            ("B".to_string(), 0.0, 500.0, None),
        ];
        let out = diff_insights(&entries);
        assert!(out.iter().any(|s| s.contains("B")), "{out:?}");
        assert!(!out[0].contains('%'), "no percent to show: {out:?}");
    }

    #[test]
    fn diff_empty_is_empty() {
        assert!(diff_insights(&[]).is_empty());
    }

    #[test]
    fn diff_single_entry_has_no_tally() {
        let entries = vec![("A".to_string(), 100.0, 150.0, Some(50.0))];
        let out = diff_insights(&entries);
        assert_eq!(out.len(), 1, "{out:?}");
        assert!(out[0].contains("grew"), "{out:?}");
        assert!(out[0].contains("+50%"), "{out:?}");
    }

    #[test]
    fn grouped_line_reports_fastest_group() {
        // Interleaved multi-plan rows in file order (like saas_revenue.csv).
        let h = headers(&["month", "plan", "mrr"]);
        let r = rows(&[
            &["2024-01", "Starter", "45000"],
            &["2024-01", "Pro", "120000"],
            &["2024-02", "Starter", "48200"],
            &["2024-02", "Pro", "128000"],
        ]);
        let out = build_insights(&req_colored(
            ChartType::Line,
            "month",
            Some("mrr"),
            "plan",
            &h,
            &r,
        ));
        assert!(
            out.iter().any(|s| s.contains("fastest of 2 plans")),
            "{out:?}"
        );
        // No jumbled cross-group jump when -c is set.
        assert!(!out.iter().any(|s| s.contains("Biggest move")), "{out:?}");
    }

    #[test]
    fn grouped_line_silent_without_color() {
        let (h, r) = sales();
        let out = build_insights(&req(ChartType::Line, "date", Some("revenue"), &h, &r));
        assert!(!out.iter().any(|s| s.contains("fastest")), "{out:?}");
        assert!(out.iter().any(|s| s.contains("Biggest move")), "{out:?}");
    }
}
