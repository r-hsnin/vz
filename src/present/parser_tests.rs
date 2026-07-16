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
    let content = "# Slide\n\n| Left | Center | Right |\n|:-----|:------:|------:|\n| a | b | c |";
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
