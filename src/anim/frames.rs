//! Pure frame interpolation: final `ChartData` → animated `Vec<ChartData>`.
//!
//! No IO here — every function is deterministic and unit-testable.
//! The last frame is always the input chart unchanged.

use crate::anim::{Effect, ease_out_cubic};
use crate::render::ChartData;

/// Build animation frames for `final_chart`.
///
/// Returns `vec![final_chart.clone()]` (single final frame) when `effect` is
/// `None` or `frame_count < 2`. Grow frames scale bar values by
/// `ease_out_cubic(t)`; draw frames reveal the first `k` points of each
/// series (all series stay in sync); build frames reveal a prefix of histogram
/// values with the count axis pinned; wipe frames reveal heatmap columns
/// left-to-right with the color scale pinned.
pub fn build_frames(
    final_chart: &ChartData,
    effect: Option<Effect>,
    frame_count: usize,
) -> Vec<ChartData> {
    let Some(effect) = effect else {
        return vec![final_chart.clone()];
    };
    if frame_count < 2 {
        return vec![final_chart.clone()];
    }
    match (effect, final_chart) {
        (Effect::Grow, ChartData::Bar(data)) => {
            let final_max = data.values.iter().copied().fold(0.0_f64, f64::max);
            (0..frame_count)
                .map(|i| {
                    // Last frame is the untouched final chart (byte-identical
                    // to the non-animated render); its own max already pins
                    // the axis, so no hint is needed.
                    if i + 1 == frame_count {
                        return ChartData::Bar(data.clone());
                    }
                    let e = ease_out_cubic(i as f64 / (frame_count - 1) as f64);
                    let mut d = data.clone();
                    d.values = data.values.iter().map(|v| v * e).collect();
                    d.y_max_hint = Some(final_max);
                    ChartData::Bar(d)
                })
                .collect()
        }
        (Effect::Draw, ChartData::Line(config)) => {
            let counts: Vec<usize> = (0..frame_count)
                .map(|i| {
                    let t = i as f64 / (frame_count - 1) as f64;
                    let max_len = config
                        .series
                        .iter()
                        .map(|s| s.data.len())
                        .max()
                        .unwrap_or(0);
                    ((max_len as f64 * t).ceil() as usize)
                        .max(1)
                        .min(max_len.max(1))
                })
                .collect();
            counts
                .into_iter()
                .map(|k| {
                    let mut c = config.clone();
                    for s in &mut c.series {
                        s.data.truncate(k.min(s.data.len()));
                    }
                    ChartData::Line(c)
                })
                .collect()
        }
        (Effect::Draw, ChartData::Scatter(config)) => {
            let counts: Vec<usize> = (0..frame_count)
                .map(|i| {
                    let t = i as f64 / (frame_count - 1) as f64;
                    let max_len = config
                        .series
                        .iter()
                        .map(|s| s.data.len())
                        .max()
                        .unwrap_or(0);
                    ((max_len as f64 * t).ceil() as usize)
                        .max(1)
                        .min(max_len.max(1))
                })
                .collect();
            counts
                .into_iter()
                .map(|k| {
                    let mut c = config.clone();
                    for s in &mut c.series {
                        s.data.truncate(k.min(s.data.len()));
                    }
                    ChartData::Scatter(c)
                })
                .collect()
        }
        (Effect::Build, ChartData::Histogram(data)) => {
            let n = data.values.len();
            if n == 0 {
                return vec![final_chart.clone()];
            }
            let final_max = crate::render::compute_bins(&data.values, data.bin_count)
                .iter()
                .map(|(_, _, c)| *c)
                .max()
                .unwrap_or(0);
            (0..frame_count)
                .map(|i| {
                    if i + 1 == frame_count {
                        return ChartData::Histogram(data.clone());
                    }
                    let t = i as f64 / (frame_count - 1) as f64;
                    let k = ((n as f64 * t).ceil() as usize).max(1).min(n);
                    let mut d = data.clone();
                    d.values.truncate(k);
                    d.max_count_hint = Some(final_max);
                    ChartData::Histogram(d)
                })
                .collect()
        }
        (Effect::Wipe, ChartData::Heatmap(data)) => {
            let ncols = data.col_labels.len();
            if ncols == 0 {
                return vec![final_chart.clone()];
            }
            (0..frame_count)
                .map(|i| {
                    if i + 1 == frame_count {
                        return ChartData::Heatmap(data.clone());
                    }
                    let t = i as f64 / (frame_count - 1) as f64;
                    let k = ((ncols as f64 * t).ceil() as usize).max(1).min(ncols);
                    let mut d = data.clone();
                    for row in &mut d.counts {
                        for (j, v) in row.iter_mut().enumerate() {
                            if j >= k {
                                *v = 0;
                            }
                        }
                    }
                    ChartData::Heatmap(d)
                })
                .collect()
        }
        // Effect/chart mismatch (e.g. grow on a line): final frame only.
        _ => vec![final_chart.clone()],
    }
}
