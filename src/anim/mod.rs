//! Animated chart playback: frame interpolation (pure) + terminal player (IO).
//!
//! Bar `grow` (values scale 0 → final with the axis pinned to the final max),
//! line/scatter `draw` (points reveal left-to-right, reusing the final
//! `ChartConfig` axes), histogram `build` (values accumulate with the count
//! axis pinned), heatmap `wipe` (columns reveal left-to-right with the color
//! scale pinned), and diff `morph` (before → after interpolation).
//! Everything else renders its final frame.

pub mod frames;
pub mod player;

pub use frames::build_frames;
pub use player::play_frames;

use crate::chart::selector::ChartType;
use crate::cli::MotionArg;

/// Playback settings resolved from CLI flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MotionConfig {
    pub motion: MotionArg,
    pub fps: u32,
    pub frames: u32,
}

impl MotionConfig {
    /// Clamp user input to sane ranges (fps 1–30, frames 2–30).
    pub fn new(motion: MotionArg, fps: u32, frames: u32) -> Self {
        Self {
            motion,
            fps: fps.clamp(1, 30),
            frames: frames.clamp(2, 30),
        }
    }
}

/// Decide whether to animate: explicit `off` never animates, and animation
/// only runs on an interactive stdout TTY (piped output stays deterministic).
/// `NO_COLOR` also disables motion (accessibility: no unexpected movement).
pub fn should_animate(config: &MotionConfig, is_tty_stdout: bool, no_color: bool) -> bool {
    if config.motion == MotionArg::Off {
        return false;
    }
    if no_color {
        return false;
    }
    is_tty_stdout
}

/// Decide whether to animate an SVG/HTML export: only explicit `grow`/`draw`
/// opts in (file output is usually piped, so the TTY gate doesn't apply).
/// `NO_COLOR` still disables motion (accessibility). `auto`/`off` stay
/// byte-identical for deterministic snapshots.
pub fn should_animate_export(motion: MotionArg, no_color: bool) -> bool {
    if no_color {
        return false;
    }
    matches!(motion, MotionArg::Grow | MotionArg::Draw)
}

/// Resolve the concrete effect for a chart type.
/// `auto` maps bar → grow, line/scatter → draw, histogram → build,
/// heatmap → wipe. Explicit `grow` also covers histogram (bar-like) and
/// explicit `draw` also covers heatmap (left-to-right reveal).
pub fn resolve_effect(motion: MotionArg, chart_type: ChartType) -> Option<Effect> {
    match motion {
        MotionArg::Off => None,
        MotionArg::Grow => match chart_type {
            ChartType::Bar => Some(Effect::Grow),
            ChartType::Histogram => Some(Effect::Build),
            _ => None,
        },
        MotionArg::Draw => match chart_type {
            ChartType::Line | ChartType::Scatter => Some(Effect::Draw),
            ChartType::Heatmap => Some(Effect::Wipe),
            _ => None,
        },
        MotionArg::Auto => match chart_type {
            ChartType::Bar => Some(Effect::Grow),
            ChartType::Line | ChartType::Scatter => Some(Effect::Draw),
            ChartType::Histogram => Some(Effect::Build),
            ChartType::Heatmap => Some(Effect::Wipe),
        },
    }
}

/// Concrete animation effect applied to a chart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    /// Bar values scale 0 → final with the Y axis pinned to the final max.
    Grow,
    /// Line/scatter points reveal left-to-right.
    Draw,
    /// Histogram values accumulate (prefix reveal) with the count axis pinned.
    Build,
    /// Heatmap columns reveal left-to-right with the color scale pinned.
    Wipe,
    /// Diff bars morph before → after with easing.
    Morph,
}

/// Cubic ease-out: fast start, gentle landing (t ∈ [0, 1]).
pub fn ease_out_cubic(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

/// Interpolate a single value before → after with ease-out cubic.
/// `t = 0` yields `before`, `t = 1` yields `after`.
pub fn morph_value(before: f64, after: f64, t: f64) -> f64 {
    before + (after - before) * ease_out_cubic(t)
}

/// Resolve the diff-bar effect: auto/grow morph before → after, else static.
pub fn resolve_diff_effect(motion: MotionArg) -> Option<Effect> {
    match motion {
        MotionArg::Auto | MotionArg::Grow => Some(Effect::Morph),
        _ => None,
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
