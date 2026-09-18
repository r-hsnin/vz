//! Bar chart rendering for categorical diff output.

use std::path::Path;

use crate::cli::Cli;
use crate::diff::DiffResult;
use crate::render::format_number;
use crate::util::path_label;

use super::apply_sort_and_limit;

/// Default chart width for diff bar rendering when terminal width is unavailable.
const DEFAULT_DIFF_BAR_WIDTH: usize = 60;

/// Print the diff summary line: `Diff │ x=col │ y=col │ before vs after │ Δ net +N% │ N entries`
pub(super) fn print_diff_summary(diff: &DiffResult, before_path: &Path, after_path: &Path) {
    let before_name = path_label(before_path);
    let after_name = path_label(after_path);

    let overall = match diff.overall_pct {
        Some(pct) if pct > 0.0 => format!(" │ Δ net +{:.0}%", pct),
        Some(pct) if pct < 0.0 => format!(" │ Δ net {:.0}%", pct),
        Some(_) => " │ Δ net 0%".to_string(),
        None => String::new(),
    };

    println!(
        "Diff │ x={} │ y={} │ {} vs {}{} │ {} entries",
        diff.x_column,
        diff.y_column,
        before_name,
        after_name,
        overall,
        diff.entries.len(),
    );

    for line in crate::insights::diff_insights(
        &diff
            .entries
            .iter()
            .map(|e| (e.label.clone(), e.before, e.after, e.pct_change))
            .collect::<Vec<_>>(),
    ) {
        eprintln!("💡 {line}");
    }
}

/// Print a diff-aware bar chart with ▲/▼ direction markers.
pub(super) fn print_diff_bar(cli: &Cli, diff: &DiffResult) {
    let entries = apply_sort_and_limit(cli, &diff.entries);

    if entries.is_empty() {
        return;
    }

    let label_width = entries.iter().map(|e| e.label.len()).max().unwrap_or(8);
    let bar_width: usize = cli
        .width
        .map(|w| w as usize)
        .unwrap_or(DEFAULT_DIFF_BAR_WIDTH)
        .saturating_sub(label_width + 40);
    let bar_width = bar_width.max(10);

    // Animated morph (TTY only): bars interpolate before → after with the
    // scale pinned to the final max; numbers stay final so the last frame is
    // byte-identical to the static render.
    let motion = crate::anim::MotionConfig::new(cli.motion, cli.fps, cli.frames);
    let no_color = std::env::var("NO_COLOR").is_ok_and(|v| !v.is_empty());
    if crate::anim::should_animate(
        &motion,
        std::io::IsTerminal::is_terminal(&std::io::stdout()),
        no_color,
    ) && let Some(crate::anim::Effect::Morph) = crate::anim::resolve_diff_effect(cli.motion)
    {
        let n = motion.frames as usize;
        let frames: Vec<Vec<String>> = (0..n)
            .map(|i| {
                if i + 1 == n {
                    return diff_bar_lines(&entries, None, label_width, bar_width);
                }
                let t = i as f64 / (n - 1) as f64;
                let displayed: Vec<f64> = entries
                    .iter()
                    .map(|e| crate::anim::morph_value(e.before, e.after, t))
                    .collect();
                diff_bar_lines(&entries, Some(&displayed), label_width, bar_width)
            })
            .collect();
        let delay = std::time::Duration::from_secs_f64(1.0 / f64::from(motion.fps));
        if crate::anim::player::play_text_frames(&mut std::io::stdout().lock(), &frames, delay)
            .is_ok()
        {
            return;
        }
        // Fall through to static on player error — never fail the command.
    }

    for line in diff_bar_lines(&entries, None, label_width, bar_width) {
        println!("{line}");
    }
}

/// Build diff bar text lines. `displayed` overrides the bar lengths
/// (interpolated morph values); `None` renders the final `after` values.
/// Direction markers and `before → after` numbers always use the final values.
pub(super) fn diff_bar_lines(
    entries: &[crate::diff::DiffEntry],
    displayed: Option<&[f64]>,
    label_width: usize,
    bar_width: usize,
) -> Vec<String> {
    // Pin the scale to the final max so intermediate frames grow/shrink
    // instead of rescaling.
    let max_abs = entries
        .iter()
        .map(|e| e.after.abs())
        .fold(0.0_f64, f64::max);

    entries
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let shown = displayed
                .and_then(|d| d.get(i))
                .copied()
                .unwrap_or(entry.after);
            let bar_len = if max_abs > 0.0 {
                ((shown.abs() / max_abs) * bar_width as f64).round() as usize
            } else {
                0
            };
            let bar = "█".repeat(bar_len);

            let direction = if entry.delta > 0.0 {
                "▲"
            } else if entry.delta < 0.0 {
                "▼"
            } else {
                "─"
            };

            let change_str = match entry.pct_change {
                Some(pct) if pct > 0.0 => format!("{} +{:.0}%", direction, pct),
                Some(pct) if pct < 0.0 => format!("{} {:.0}%", direction, pct),
                Some(_) => format!("{} 0%", direction),
                None if entry.delta > 0.0 => {
                    format!("{} +{}", direction, format_number(entry.delta))
                }
                None => direction.to_string(),
            };

            format!(
                "  {:width$}  {}  {} → {}  {}",
                entry.label,
                bar,
                format_number(entry.before),
                format_number(entry.after),
                change_str,
                width = label_width,
            )
        })
        .collect()
}
