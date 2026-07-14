use crate::chart::selector::ChartType;

use super::{ChartBlock, Presentation, Slide, SlideElement};

/// Parse a markdown file into a Presentation.
pub fn parse_presentation(content: &str) -> Presentation {
    let mut ctx = ParseContext::new();
    for line in content.lines() {
        ctx.process_line(line);
    }
    ctx.finalize()
}

/// Internal parser state for building a Presentation from markdown.
struct ParseContext {
    slides: Vec<Slide>,
    current_title: Option<String>,
    current_elements: Vec<SlideElement>,
    in_chart_block: bool,
    chart_lines: Vec<String>,
    in_code_block: bool,
    code_language: Option<String>,
    code_lines: Vec<String>,
    text_buffer: String,
    table_lines: Vec<String>,
}

impl ParseContext {
    fn new() -> Self {
        Self {
            slides: Vec::new(),
            current_title: None,
            current_elements: Vec::new(),
            in_chart_block: false,
            chart_lines: Vec::new(),
            in_code_block: false,
            code_language: None,
            code_lines: Vec::new(),
            text_buffer: String::new(),
            table_lines: Vec::new(),
        }
    }

    fn process_line(&mut self, line: &str) {
        if self.try_chart_content(line) {
            return;
        }
        if self.try_code_content(line) {
            return;
        }
        if self.try_separator(line) {
            return;
        }
        if self.try_fenced_block_start(line) {
            return;
        }
        if self.try_table_line(line) {
            return;
        }
        if self.try_heading(line) {
            return;
        }
        if self.try_subheading(line) {
            return;
        }
        if self.try_numbered_list(line) {
            return;
        }
        if self.try_bullet(line) {
            return;
        }
        self.accumulate_text(line);
    }

    fn try_code_content(&mut self, line: &str) -> bool {
        if !self.in_code_block {
            return false;
        }
        if line.trim() == "```" {
            self.in_code_block = false;
            let content = self.code_lines.join("\n");
            self.current_elements.push(SlideElement::Code {
                language: self.code_language.take(),
                content,
            });
            self.code_lines.clear();
        } else {
            self.code_lines.push(line.to_string());
        }
        true
    }

    fn try_chart_content(&mut self, line: &str) -> bool {
        if !self.in_chart_block {
            return false;
        }
        if line.trim() == "```" {
            self.in_chart_block = false;
            let chart = parse_chart_block(&self.chart_lines);
            self.current_elements.push(SlideElement::Chart(chart));
            self.chart_lines.clear();
        } else {
            self.chart_lines.push(line.to_string());
        }
        true
    }

    fn try_separator(&mut self, line: &str) -> bool {
        if line.trim() != "---" {
            return false;
        }
        self.flush_table();
        self.flush_text();
        self.push_slide_if_nonempty();
        true
    }

    fn try_fenced_block_start(&mut self, line: &str) -> bool {
        let trimmed = line.trim();
        if !trimmed.starts_with("```") {
            return false;
        }
        self.flush_table();
        self.flush_text();
        let lang = trimmed.strip_prefix("```").unwrap_or("").trim();
        if lang == "chart" {
            self.in_chart_block = true;
            self.chart_lines.clear();
        } else {
            self.in_code_block = true;
            self.code_language = if lang.is_empty() {
                None
            } else {
                Some(lang.to_string())
            };
            self.code_lines.clear();
        }
        true
    }

    fn try_table_line(&mut self, line: &str) -> bool {
        let trimmed = line.trim();
        if trimmed.starts_with('|') && trimmed.ends_with('|') && trimmed.len() >= 3 {
            self.flush_text();
            self.table_lines.push(trimmed.to_string());
            return true;
        }
        // Not a table line — flush any accumulated table
        self.flush_table();
        false
    }

    fn flush_table(&mut self) {
        if self.table_lines.is_empty() {
            return;
        }
        // Need at least header + separator (2 lines) to be a valid table
        if self.table_lines.len() >= 2 && is_separator_row(&self.table_lines[1]) {
            let headers = parse_table_row(&self.table_lines[0]);
            let rows: Vec<Vec<String>> = self.table_lines[2..]
                .iter()
                .map(|l| parse_table_row(l))
                .collect();
            self.current_elements
                .push(SlideElement::Table { headers, rows });
        } else {
            // Not a valid table — push as text
            for line in &self.table_lines {
                self.current_elements.push(SlideElement::Text(line.clone()));
            }
        }
        self.table_lines.clear();
    }

    fn try_heading(&mut self, line: &str) -> bool {
        if !line.starts_with("# ") {
            return false;
        }
        self.flush_table();
        self.flush_text();
        self.push_slide_if_nonempty();
        self.current_title = Some(line.trim_start_matches("# ").to_string());
        true
    }

    fn try_subheading(&mut self, line: &str) -> bool {
        let (level, prefix) = if line.starts_with("### ") {
            (3, "### ")
        } else if line.starts_with("## ") {
            (2, "## ")
        } else {
            return false;
        };
        self.flush_text();
        let text = line.strip_prefix(prefix).unwrap_or("").to_string();
        self.current_elements
            .push(SlideElement::Heading { level, text });
        true
    }

    fn try_numbered_list(&mut self, line: &str) -> bool {
        // Match "1. text", "2. text", etc.
        let trimmed = line.trim_start();
        let digit_end = trimmed.find(|c: char| !c.is_ascii_digit());
        if let Some(pos) = digit_end
            && pos > 0
            && trimmed[pos..].starts_with(". ")
        {
            self.flush_text();
            let item_text = trimmed[pos + 2..].to_string();
            if let Some(SlideElement::OrderedList(items)) = self.current_elements.last_mut() {
                items.push(item_text);
            } else {
                self.current_elements
                    .push(SlideElement::OrderedList(vec![item_text]));
            }
            return true;
        }
        false
    }

    fn try_bullet(&mut self, line: &str) -> bool {
        if !line.starts_with("- ") && !line.starts_with("* ") {
            return false;
        }
        self.flush_text();
        let bullet_text = line[2..].to_string();
        if let Some(SlideElement::Bullets(bullets)) = self.current_elements.last_mut() {
            bullets.push(bullet_text);
        } else {
            self.current_elements
                .push(SlideElement::Bullets(vec![bullet_text]));
        }
        true
    }

    fn accumulate_text(&mut self, line: &str) {
        if !line.trim().is_empty() {
            if !self.text_buffer.is_empty() {
                self.text_buffer.push(' ');
            }
            self.text_buffer.push_str(line.trim());
        } else if !self.text_buffer.is_empty() {
            self.flush_text();
        }
    }

    fn flush_text(&mut self) {
        if !self.text_buffer.is_empty() {
            self.current_elements
                .push(SlideElement::Text(std::mem::take(&mut self.text_buffer)));
        }
    }

    fn push_slide_if_nonempty(&mut self) {
        if self.current_title.is_some() || !self.current_elements.is_empty() {
            self.slides.push(Slide {
                title: self.current_title.take(),
                content: std::mem::take(&mut self.current_elements),
            });
        }
    }

    fn finalize(mut self) -> Presentation {
        self.flush_table();
        self.flush_text();
        self.push_slide_if_nonempty();
        Presentation {
            slides: self.slides,
        }
    }
}

/// Check if a table line is a separator row (e.g., `|---|---|`).
fn is_separator_row(line: &str) -> bool {
    let inner = line.trim_matches('|');
    inner.split('|').all(|cell| {
        let trimmed = cell.trim();
        !trimmed.is_empty() && trimmed.chars().all(|c| c == '-' || c == ':' || c == ' ')
    })
}

/// Parse a GFM table row into cells.
fn parse_table_row(line: &str) -> Vec<String> {
    line.trim_matches('|')
        .split('|')
        .map(|cell| cell.trim().to_string())
        .collect()
}

/// Parse chart block key-value pairs.
pub(crate) fn parse_chart_block(lines: &[String]) -> ChartBlock {
    let mut source = String::new();
    let mut chart_type = None;
    let mut x_col = None;
    let mut y_col = None;
    let mut color_col = None;
    let mut title = None;
    let mut filter = Vec::new();
    let mut sort = None;
    let mut agg = None;
    let mut top = None;
    let mut bins = None;
    let mut height = None;

    for line in lines {
        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim();
            let value = value.trim().to_string();
            match key {
                "source" => source = value,
                "type" => {
                    chart_type = match value.to_lowercase().as_str() {
                        "line" => Some(ChartType::Line),
                        "bar" => Some(ChartType::Bar),
                        "scatter" => Some(ChartType::Scatter),
                        "histogram" => Some(ChartType::Histogram),
                        "heatmap" => Some(ChartType::Heatmap),
                        _ => None,
                    }
                }
                "x" => x_col = Some(value),
                "y" => y_col = Some(value),
                "color" => color_col = Some(value),
                "title" => title = Some(value),
                "where" => filter.push(value),
                "sort" => {
                    sort = match value.to_lowercase().as_str() {
                        "desc" => Some(crate::cli::SortOrder::Desc),
                        "asc" => Some(crate::cli::SortOrder::Asc),
                        _ => None,
                    }
                }
                "agg" => {
                    agg = match value.to_lowercase().as_str() {
                        "sum" => Some(crate::cli::AggFunction::Sum),
                        "mean" => Some(crate::cli::AggFunction::Mean),
                        "count" => Some(crate::cli::AggFunction::Count),
                        "max" => Some(crate::cli::AggFunction::Max),
                        "min" => Some(crate::cli::AggFunction::Min),
                        _ => None,
                    }
                }
                "top" => {
                    top = value.parse::<usize>().ok();
                }
                "bins" => {
                    bins = value.parse::<usize>().ok();
                }
                "height" => {
                    height = value.parse::<u16>().ok();
                }
                _ => {}
            }
        }
    }

    ChartBlock {
        source,
        chart_type,
        x_col,
        y_col,
        color_col,
        title,
        filter,
        sort,
        agg,
        top,
        bins,
        height,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unclosed_chart_block_ignored() {
        // A chart block that never closes should not produce a Chart element
        let content = "# Slide\n\n```chart\nsource: data.csv\nx: month\n";
        let pres = parse_presentation(content);
        assert_eq!(pres.slides.len(), 1);
        // The chart_lines accumulate but finalize doesn't push them
        for elem in &pres.slides[0].content {
            assert!(
                !matches!(elem, SlideElement::Chart(_)),
                "Unclosed chart block should not produce a Chart element"
            );
        }
    }

    #[test]
    fn test_empty_slide_between_separators() {
        let content = "# First\n\nHello\n\n---\n\n---\n\n# Third\n\nWorld";
        let pres = parse_presentation(content);
        // First slide has content, then empty separator creates nothing, third slide has content
        assert!(pres.slides.len() >= 2);
        // First slide should have title "First"
        assert_eq!(pres.slides[0].title.as_deref(), Some("First"));
    }

    #[test]
    fn test_bullets_interleaved_with_text() {
        let content = "# Mixed\n\nSome text\n\n- bullet 1\n- bullet 2\n\nMore text";
        let pres = parse_presentation(content);
        assert_eq!(pres.slides.len(), 1);
        let elems = &pres.slides[0].content;
        // Should have: Text, Bullets, Text
        assert_eq!(elems.len(), 3, "Expected 3 elements, got {:?}", elems);
        assert!(matches!(&elems[0], SlideElement::Text(t) if t == "Some text"));
        assert!(matches!(&elems[1], SlideElement::Bullets(b) if b.len() == 2));
        assert!(matches!(&elems[2], SlideElement::Text(t) if t == "More text"));
    }

    #[test]
    fn test_chart_block_with_unknown_keys() {
        let lines = vec![
            "source: data.csv".into(),
            "type: bar".into(),
            "unknown_key: value".into(),
            "x: month".into(),
        ];
        let chart = parse_chart_block(&lines);
        assert_eq!(chart.source, "data.csv");
        assert_eq!(chart.chart_type, Some(ChartType::Bar));
        assert_eq!(chart.x_col, Some("month".into()));
    }

    #[test]
    fn test_chart_block_invalid_type() {
        let lines = vec!["source: data.csv".into(), "type: sparkline".into()];
        let chart = parse_chart_block(&lines);
        assert_eq!(chart.source, "data.csv");
        assert_eq!(chart.chart_type, None); // Unknown type → None
    }

    #[test]
    fn test_chart_block_bins_parsed() {
        let lines = vec![
            "source: data.csv".into(),
            "type: histogram".into(),
            "bins: 20".into(),
        ];
        let chart = parse_chart_block(&lines);
        assert_eq!(chart.bins, Some(20));
    }

    #[test]
    fn test_chart_block_bins_invalid_ignored() {
        let lines = vec!["source: data.csv".into(), "bins: not_a_number".into()];
        let chart = parse_chart_block(&lines);
        assert_eq!(chart.bins, None);
    }

    #[test]
    fn test_chart_block_height_parsed() {
        let lines = vec![
            "source: data.csv".into(),
            "type: line".into(),
            "height: 20".into(),
        ];
        let chart = parse_chart_block(&lines);
        assert_eq!(chart.height, Some(20));
    }

    #[test]
    fn test_chart_block_height_invalid_ignored() {
        let lines = vec!["source: data.csv".into(), "height: abc".into()];
        let chart = parse_chart_block(&lines);
        assert_eq!(chart.height, None);
    }

    #[test]
    fn test_multiple_headings_create_slides() {
        let content = "# One\n\nText1\n\n# Two\n\nText2\n\n# Three\n\nText3";
        let pres = parse_presentation(content);
        assert_eq!(pres.slides.len(), 3);
        assert_eq!(pres.slides[0].title.as_deref(), Some("One"));
        assert_eq!(pres.slides[1].title.as_deref(), Some("Two"));
        assert_eq!(pres.slides[2].title.as_deref(), Some("Three"));
    }

    #[test]
    fn test_code_block_parsed_as_code_element() {
        let content = "# Demo\n\n```python\ndef hello():\n    print('hi')\n```\n";
        let pres = parse_presentation(content);
        assert_eq!(pres.slides.len(), 1);
        let elems = &pres.slides[0].content;
        assert_eq!(elems.len(), 1, "Expected 1 element, got {:?}", elems);
        match &elems[0] {
            SlideElement::Code { language, content } => {
                assert_eq!(language.as_deref(), Some("python"));
                assert!(content.contains("def hello()"));
                assert!(content.contains("print('hi')"));
            }
            other => panic!("Expected Code element, got {:?}", other),
        }
    }

    #[test]
    fn test_code_block_without_language() {
        let content = "# Slide\n\n```\nplain code\n```\n";
        let pres = parse_presentation(content);
        let elems = &pres.slides[0].content;
        match &elems[0] {
            SlideElement::Code { language, content } => {
                assert_eq!(language.as_deref(), None);
                assert_eq!(content.trim(), "plain code");
            }
            other => panic!("Expected Code element, got {:?}", other),
        }
    }

    #[test]
    fn test_code_block_interleaved_with_text() {
        let content = "# Slide\n\nBefore\n\n```bash\necho hi\n```\n\nAfter";
        let pres = parse_presentation(content);
        let elems = &pres.slides[0].content;
        assert_eq!(elems.len(), 3, "Expected 3 elements, got {:?}", elems);
        assert!(matches!(&elems[0], SlideElement::Text(t) if t == "Before"));
        assert!(matches!(&elems[1], SlideElement::Code { .. }));
        assert!(matches!(&elems[2], SlideElement::Text(t) if t == "After"));
    }

    #[test]
    fn test_chart_block_still_works_alongside_code() {
        let content = "# Slide\n\n```chart\nsource: sales.csv\nx: month\ny: revenue\n```\n\n```rust\nfn main() {}\n```\n";
        let pres = parse_presentation(content);
        let elems = &pres.slides[0].content;
        assert_eq!(elems.len(), 2, "Expected 2 elements, got {:?}", elems);
        assert!(matches!(&elems[0], SlideElement::Chart(_)));
        assert!(matches!(&elems[1], SlideElement::Code { .. }));
    }

    #[test]
    fn test_table_basic_parsing() {
        let content = "# Data\n\n| Name | Age |\n|------|-----|\n| Alice | 30 |\n| Bob | 25 |";
        let pres = parse_presentation(content);
        let elems = &pres.slides[0].content;
        assert_eq!(elems.len(), 1, "Expected 1 table element, got {:?}", elems);
        match &elems[0] {
            SlideElement::Table { headers, rows } => {
                assert_eq!(headers, &["Name", "Age"]);
                assert_eq!(rows.len(), 2);
                assert_eq!(rows[0], &["Alice", "30"]);
                assert_eq!(rows[1], &["Bob", "25"]);
            }
            other => panic!("Expected Table element, got {:?}", other),
        }
    }

    #[test]
    fn test_table_with_alignment_markers() {
        let content =
            "# Slide\n\n| Left | Center | Right |\n|:-----|:------:|------:|\n| a | b | c |";
        let pres = parse_presentation(content);
        let elems = &pres.slides[0].content;
        match &elems[0] {
            SlideElement::Table { headers, rows } => {
                assert_eq!(headers, &["Left", "Center", "Right"]);
                assert_eq!(rows.len(), 1);
                assert_eq!(rows[0], &["a", "b", "c"]);
            }
            other => panic!("Expected Table element, got {:?}", other),
        }
    }

    #[test]
    fn test_table_interleaved_with_text() {
        let content = "# Slide\n\nBefore\n\n| A | B |\n|---|---|\n| 1 | 2 |\n\nAfter";
        let pres = parse_presentation(content);
        let elems = &pres.slides[0].content;
        assert_eq!(elems.len(), 3, "Expected 3 elements, got {:?}", elems);
        assert!(matches!(&elems[0], SlideElement::Text(t) if t == "Before"));
        assert!(matches!(&elems[1], SlideElement::Table { .. }));
        assert!(matches!(&elems[2], SlideElement::Text(t) if t == "After"));
    }

    #[test]
    fn test_table_without_separator_treated_as_text() {
        // A pipe line without a separator row is not a valid table
        let content = "# Slide\n\n| Not | A | Table |\n| just | pipes | here |";
        let pres = parse_presentation(content);
        let elems = &pres.slides[0].content;
        // Should produce Text elements, not a Table
        assert!(
            !elems
                .iter()
                .any(|e| matches!(e, SlideElement::Table { .. })),
            "Should not produce a Table without separator row, got {:?}",
            elems
        );
    }

    #[test]
    fn test_table_header_only_no_rows() {
        let content = "# Slide\n\n| Col1 | Col2 |\n|------|------|";
        let pres = parse_presentation(content);
        let elems = &pres.slides[0].content;
        match &elems[0] {
            SlideElement::Table { headers, rows } => {
                assert_eq!(headers, &["Col1", "Col2"]);
                assert!(rows.is_empty());
            }
            other => panic!("Expected Table element, got {:?}", other),
        }
    }
}

#[test]
fn test_subheading_h2_parsed() {
    let content = "# Main Title\n\n## Section One\n\nContent here";
    let pres = parse_presentation(content);
    assert_eq!(pres.slides.len(), 1);
    let elems = &pres.slides[0].content;
    // ## should NOT create a new slide, but should be a Heading element
    assert!(
        elems
            .iter()
            .any(|e| matches!(e, SlideElement::Heading { level: 2, .. })),
        "Expected h2 heading element, got {:?}",
        elems
    );
}

#[test]
fn test_subheading_h3_parsed() {
    let content = "# Title\n\n### Sub-sub\n\nText";
    let pres = parse_presentation(content);
    let elems = &pres.slides[0].content;
    assert!(
        elems
            .iter()
            .any(|e| matches!(e, SlideElement::Heading { level: 3, .. })),
        "Expected h3 heading element, got {:?}",
        elems
    );
}

#[test]
fn test_numbered_list_parsed() {
    let content = "# Steps\n\n1. First step\n2. Second step\n3. Third step";
    let pres = parse_presentation(content);
    let elems = &pres.slides[0].content;
    assert!(
        elems
            .iter()
            .any(|e| matches!(e, SlideElement::OrderedList(_))),
        "Expected OrderedList element, got {:?}",
        elems
    );
    if let Some(SlideElement::OrderedList(items)) = elems
        .iter()
        .find(|e| matches!(e, SlideElement::OrderedList(_)))
    {
        assert_eq!(items.len(), 3);
        assert_eq!(items[0], "First step");
        assert_eq!(items[2], "Third step");
    }
}

#[test]
fn test_numbered_list_interleaved_with_bullets() {
    let content = "# Mixed\n\n1. ordered\n2. items\n\n- unordered\n- bullets";
    let pres = parse_presentation(content);
    let elems = &pres.slides[0].content;
    assert_eq!(elems.len(), 2, "Expected 2 list elements, got {:?}", elems);
    assert!(matches!(&elems[0], SlideElement::OrderedList(v) if v.len() == 2));
    assert!(matches!(&elems[1], SlideElement::Bullets(v) if v.len() == 2));
}

#[test]
fn test_inline_bold_parsed() {
    use crate::present::parse_inline_spans;
    let spans = parse_inline_spans("Hello **world** end");
    assert_eq!(spans.len(), 3);
    assert_eq!(spans[0].content, "Hello ");
    assert_eq!(spans[1].content, "world");
    assert!(
        spans[1]
            .style
            .add_modifier
            .contains(ratatui::style::Modifier::BOLD)
    );
    assert_eq!(spans[2].content, " end");
}

#[test]
fn test_inline_italic_parsed() {
    use crate::present::parse_inline_spans;
    let spans = parse_inline_spans("Hello *world* end");
    assert_eq!(spans.len(), 3);
    assert_eq!(spans[0].content, "Hello ");
    assert_eq!(spans[1].content, "world");
    assert!(
        spans[1]
            .style
            .add_modifier
            .contains(ratatui::style::Modifier::ITALIC)
    );
    assert_eq!(spans[2].content, " end");
}

#[test]
fn test_inline_no_formatting() {
    use crate::present::parse_inline_spans;
    let spans = parse_inline_spans("plain text");
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].content, "plain text");
}

#[test]
fn test_inline_bold_and_italic_mixed() {
    use crate::present::parse_inline_spans;
    let spans = parse_inline_spans("**bold** then *italic*");
    assert_eq!(spans.len(), 3);
    assert_eq!(spans[0].content, "bold");
    assert!(
        spans[0]
            .style
            .add_modifier
            .contains(ratatui::style::Modifier::BOLD)
    );
    assert_eq!(spans[1].content, " then ");
    assert_eq!(spans[2].content, "italic");
    assert!(
        spans[2]
            .style
            .add_modifier
            .contains(ratatui::style::Modifier::ITALIC)
    );
}
