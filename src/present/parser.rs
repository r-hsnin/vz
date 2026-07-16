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
        if self.try_blockquote(line) {
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

    fn try_blockquote(&mut self, line: &str) -> bool {
        let text = if let Some(stripped) = line.strip_prefix("> ") {
            stripped.to_string()
        } else if line == ">" {
            String::new()
        } else {
            return false;
        };
        self.flush_text();
        if let Some(SlideElement::Blockquote(lines)) = self.current_elements.last_mut() {
            lines.push(text);
        } else {
            self.current_elements
                .push(SlideElement::Blockquote(vec![text]));
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
    let mut diff = None;

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
                "diff" => diff = Some(value),
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
        diff,
    }
}

#[cfg(test)]
#[path = "parser_tests.rs"]
mod tests;
