//! Terminal frame player: render `ChartData` frames with in-place redraw.
//!
//! Thin IO layer over [`super::frames`]: cursor control, per-frame delay,
//! flush accounting. Rendering itself stays in `oneshot`/`render`.

use std::io::Write;
use std::time::Duration;

use ratatui::{buffer::Buffer, layout::Rect};

use crate::render::ChartData;

/// Play pre-built frames to `writer`, redrawing in place.
///
/// The first frame renders normally; each subsequent frame moves the cursor
/// up by the area height and re-renders, so only the chart region flickers.
/// Returns the number of frames written.
///
/// `frame_delay` is the pause between frames (derived from `--fps`);
/// `render` converts one `ChartData` frame to text lines (injected so unit
/// tests can run without a real terminal).
pub fn play_frames<W: Write>(
    writer: &mut W,
    frames: &[ChartData],
    area: Rect,
    frame_delay: Duration,
    render: impl Fn(&ChartData, Rect) -> Vec<String>,
) -> anyhow::Result<usize> {
    if frames.is_empty() {
        return Ok(0);
    }
    // Hide cursor during playback; always restore on exit paths below.
    write!(writer, "\x1b[?25l")?;
    let mut written = 0usize;
    for (i, frame) in frames.iter().enumerate() {
        if i > 0 {
            // Move cursor up by the chart height, then clear each line.
            write!(writer, "\x1b[{}A", area.height)?;
            for _ in 0..area.height {
                write!(writer, "\x1b[2K\r\n")?;
            }
            write!(writer, "\x1b[{}A", area.height)?;
        }
        for line in render(frame, area) {
            writeln!(writer, "{line}")?;
        }
        writer.flush()?;
        written += 1;
        if i + 1 < frames.len() && !frame_delay.is_zero() {
            std::thread::sleep(frame_delay);
        }
    }
    write!(writer, "\x1b[?25h")?;
    writer.flush()?;
    Ok(written)
}

/// Render one `ChartData` frame into ANSI text lines for the player.
pub fn render_frame_lines(frame: &ChartData, area: Rect) -> Vec<String> {
    let mut buf = Buffer::empty(area);
    crate::render::render_chart_data(frame, area, &mut buf);
    let area = buf.area;
    (area.y..area.y + area.height)
        .map(|y| {
            let mut line = String::new();
            for x in area.x..area.x + area.width {
                line.push_str(buf[(x, y)].symbol());
            }
            line.trim_end().to_string()
        })
        .collect()
}
