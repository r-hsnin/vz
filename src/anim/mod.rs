//! Animated chart playback: frame interpolation (pure) + terminal player (IO).
//!
//! MVP scope: bar `grow` (values scale 0 → final with the axis pinned to the
//! final max) and line/scatter `draw` (points reveal left-to-right, reusing
//! the final `ChartConfig` axes). Everything else renders its final frame.

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

/// Resolve the concrete effect for a chart type.
/// `auto` maps bar → grow, line/scatter → draw; histogram/heatmap have no
/// effect yet and render the final frame immediately.
pub fn resolve_effect(motion: MotionArg, chart_type: ChartType) -> Option<Effect> {
    match motion {
        MotionArg::Off => None,
        MotionArg::Grow => (chart_type == ChartType::Bar).then_some(Effect::Grow),
        MotionArg::Draw => {
            matches!(chart_type, ChartType::Line | ChartType::Scatter).then_some(Effect::Draw)
        }
        MotionArg::Auto => match chart_type {
            ChartType::Bar => Some(Effect::Grow),
            ChartType::Line | ChartType::Scatter => Some(Effect::Draw),
            _ => None,
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
}

/// Cubic ease-out: fast start, gentle landing (t ∈ [0, 1]).
pub fn ease_out_cubic(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
