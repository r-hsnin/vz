//! HTML output: wraps SVG chart in a self-contained HTML page with interactive tooltips.

/// Render the chart as a self-contained HTML page with embedded SVG
/// and print to stdout.
pub fn print_html(
    recommendation: &crate::chart::selector::ChartRecommendation,
    headers: &[String],
    rows: &[Vec<String>],
    opts: &crate::oneshot::RenderOptions<'_>,
) -> anyhow::Result<()> {
    let bg = opts.theme.svg_background();
    let svg = crate::output::svg::render_chart_svg(recommendation, headers, rows, opts);
    let title = opts.title.as_deref().unwrap_or("vz chart");
    println!("{}", wrap_svg_in_html(&svg, title, bg));
    Ok(())
}

/// Wrap an SVG string in a complete, self-contained HTML5 document.
///
/// The resulting HTML includes:
/// - Responsive viewport meta
/// - Inline CSS for centering and dark/light background
/// - The SVG chart embedded inline
/// - Inline JavaScript (~50 lines) for hover tooltips on data points
///
/// No external CDN dependencies — works fully offline.
pub fn wrap_svg_in_html(svg: &str, title: &str, bg_color: &str) -> String {
    let escaped_title = html_escape(title);
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{escaped_title}</title>
<style>
body {{
  margin: 0;
  background: {bg_color};
  display: flex;
  justify-content: center;
  align-items: center;
  min-height: 100vh;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
}}
.chart-container {{
  position: relative;
}}
.tooltip {{
  position: absolute;
  display: none;
  background: rgba(0, 0, 0, 0.85);
  color: #fff;
  padding: 4px 8px;
  border-radius: 4px;
  font-size: 12px;
  pointer-events: none;
  white-space: nowrap;
  z-index: 10;
}}
svg {{
  max-width: 100%;
  height: auto;
}}
</style>
</head>
<body>
<div class="chart-container">
{svg}
<div class="tooltip" id="tooltip"></div>
</div>
<script>
(function() {{
  var tooltip = document.getElementById('tooltip');
  var container = document.querySelector('.chart-container');
  var svg = container.querySelector('svg');
  if (!svg) return;

  // Prefer real data marks first (<circle class="vz-point"> with
  // data-label/data-value + <title>); fall back to tspan/rect
  // heuristics only when no marks exist.
  if (svg.querySelector('circle.vz-point')) {{
    svg.querySelectorAll('circle.vz-point').forEach(function(el) {{
      el.style.cursor = 'pointer';
      el.style.pointerEvents = 'all';
      function labelFor(target) {{
        var title = target.querySelector('title');
        if (title && title.textContent.trim().length > 0) {{
          var series = target.getAttribute('data-series');
          var t = title.textContent.trim();
          return series ? series + ' — ' + t : t;
        }}
        var label = target.getAttribute('data-label') || '';
        var value = target.getAttribute('data-value') || '';
        var s = (label + (value ? ': ' + value : '')).trim();
        return s || 'data point';
      }}
      el.addEventListener('mouseenter', function(e) {{
        showTooltip(e, labelFor(el));
      }});
      el.addEventListener('mousemove', function(e) {{
        showTooltip(e, labelFor(el));
      }});
      el.addEventListener('mouseleave', hideTooltip);
    }});
  }} else {{
  // Attach hover listeners to tspan elements that contain visible text
  var tspans = svg.querySelectorAll('tspan');
  tspans.forEach(function(el) {{
    var text = el.textContent.trim();
    if (text.length > 0 && text.length < 40) {{
      el.style.cursor = 'pointer';
      el.addEventListener('mouseenter', function(e) {{
        showTooltip(e, text);
      }});
      el.addEventListener('mousemove', function(e) {{
        showTooltip(e, text);
      }});
      el.addEventListener('mouseleave', hideTooltip);
    }}
  }});

  // Also handle rect elements with non-background fills (chart bars)
  var rects = svg.querySelectorAll('rect');
  rects.forEach(function(el) {{
    var fill = el.getAttribute('fill');
    if (fill && fill !== '{bg_color}' && el.getAttribute('width') !== '100%') {{
      el.style.cursor = 'pointer';
      el.addEventListener('mouseenter', function(e) {{
        showTooltip(e, 'bar');
      }});
      el.addEventListener('mousemove', function(e) {{
        showTooltip(e, 'bar');
      }});
      el.addEventListener('mouseleave', hideTooltip);
    }}
  }});
  }}
}})();
</script>
</body>
</html>"#
    )
}

/// Escape HTML special characters in text content.
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 640 384"><rect width="100%" height="100%" fill="#1e1e1e"/><text><tspan>Hello</tspan></text></svg>"##;

    #[test]
    fn test_produces_valid_html5_structure() {
        let html = wrap_svg_in_html(SAMPLE_SVG, "Test Chart", "#1e1e1e");
        assert!(html.starts_with("<!DOCTYPE html>"));
        assert!(html.contains("<html"));
        assert!(html.contains("</html>"));
        assert!(html.contains("<head>"));
        assert!(html.contains("</head>"));
        assert!(html.contains("<body>"));
        assert!(html.contains("</body>"));
    }

    #[test]
    fn test_contains_original_svg() {
        let html = wrap_svg_in_html(SAMPLE_SVG, "Test", "#1e1e1e");
        assert!(html.contains(SAMPLE_SVG));
    }

    #[test]
    fn test_contains_inline_javascript() {
        let html = wrap_svg_in_html(SAMPLE_SVG, "Test", "#1e1e1e");
        assert!(html.contains("<script>"));
        assert!(html.contains("</script>"));
    }

    #[test]
    fn test_no_external_dependencies() {
        let html = wrap_svg_in_html(SAMPLE_SVG, "Test", "#1e1e1e");
        assert!(!html.contains("src=\"http"));
        assert!(!html.contains("src=\"//"));
        assert!(!html.contains("href=\"http"));
    }

    #[test]
    fn test_has_viewport_meta() {
        let html = wrap_svg_in_html(SAMPLE_SVG, "Test", "#1e1e1e");
        assert!(html.contains("viewport"));
    }

    #[test]
    fn test_has_charset_utf8() {
        let html = wrap_svg_in_html(SAMPLE_SVG, "Test", "#1e1e1e");
        assert!(html.contains("utf-8"));
    }

    #[test]
    fn test_custom_title() {
        let html = wrap_svg_in_html(SAMPLE_SVG, "Revenue by City", "#1e1e1e");
        assert!(html.contains("<title>Revenue by City</title>"));
    }

    #[test]
    fn test_title_html_escaped() {
        let html = wrap_svg_in_html(SAMPLE_SVG, "Sales <Q1> & \"Q2\"", "#1e1e1e");
        assert!(html.contains("&lt;Q1&gt;"));
        assert!(html.contains("&amp;"));
        assert!(html.contains("&quot;Q2&quot;"));
        assert!(!html.contains("<Q1>"));
    }

    #[test]
    fn test_background_color_applied() {
        let html = wrap_svg_in_html(SAMPLE_SVG, "Test", "#ffffff");
        assert!(html.contains("background: #ffffff"));
    }

    #[test]
    fn test_has_inline_styles() {
        let html = wrap_svg_in_html(SAMPLE_SVG, "Test", "#1e1e1e");
        assert!(html.contains("<style>"));
    }

    #[test]
    fn test_tooltip_hover_events() {
        let html = wrap_svg_in_html(SAMPLE_SVG, "Test", "#1e1e1e");
        assert!(html.contains("mouseenter"));
        assert!(html.contains("mouseleave"));
    }

    #[test]
    fn test_tooltip_prefers_vz_point_data() {
        // RED: tooltip must read real data (data-label/data-value/<title>),
        // not tspan glyph geometry. Must fail until the JS is rewritten.
        let svg_with_points = r##"<svg xmlns="http://www.w3.org/2000/svg"><g class="vz-data"><circle class="vz-point" cx="10" cy="10" r="7" data-label="Tokyo" data-value="4200"><title>Tokyo: 4.2k</title></circle></g><text><tspan>4k│████</tspan></text></svg>"##;
        let html = wrap_svg_in_html(svg_with_points, "Test", "#1e1e1e");
        assert!(
            html.contains("vz-point"),
            "JS must target .vz-point marks: {html}"
        );
        assert!(
            html.contains("data-label") || html.contains("getAttribute"),
            "JS must read data-label for tooltip text"
        );
        assert!(
            !html.contains("Math.round(w)"),
            "JS must not show rect geometry as tooltip: {html}"
        );
    }

    #[test]
    fn test_html_escape_function() {
        assert_eq!(html_escape("a < b & c"), "a &lt; b &amp; c");
        assert_eq!(html_escape("\"hi\""), "&quot;hi&quot;");
        assert_eq!(html_escape("normal text"), "normal text");
    }
}
