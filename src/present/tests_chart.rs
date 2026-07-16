use super::*;

#[test]
fn test_load_chart_data_with_color_column() {
    let block = ChartBlock {
        source: "sales.csv".to_string(),
        chart_type: Some(ChartType::Line),
        x_col: Some("date".to_string()),
        y_col: Some("revenue".to_string()),
        color_col: Some("city".to_string()),
        title: Some("Multi-Series".to_string()),
        filter: vec![],
        sort: None,
        agg: None,
        top: None,
        bins: None,
        height: None,
        diff: None,
    };

    let base_dir = std::path::Path::new("fixtures");
    let result = super::load_chart_data(&block, base_dir, &crate::theme::Theme::dark());
    assert!(
        result.is_ok(),
        "multi-series should work: {:?}",
        result.err()
    );

    if let Ok(crate::render::ChartData::Line(config)) = result {
        // Should have multiple series (one per city)
        assert!(
            config.series.len() > 1,
            "Expected multi-series, got {}",
            config.series.len()
        );
        // Check series names
        let names: Vec<&str> = config.series.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"Tokyo"));
        assert!(names.contains(&"Osaka"));
    } else {
        panic!("Expected Line chart data");
    }
}

#[test]
fn test_parse_chart_block_with_color() {
    let lines = vec![
        "source: data.csv".to_string(),
        "x: month".to_string(),
        "y: revenue".to_string(),
        "color: region".to_string(),
        "type: line".to_string(),
        "title: By Region".to_string(),
    ];
    let block = super::parser::parse_chart_block(&lines);
    assert_eq!(block.color_col, Some("region".to_string()));
    assert_eq!(block.title, Some("By Region".to_string()));
}

#[test]
fn test_load_chart_data_json_source() {
    let block = ChartBlock {
        source: "scores.json".to_string(),
        chart_type: Some(ChartType::Bar),
        x_col: Some("name".to_string()),
        y_col: Some("score".to_string()),
        color_col: None,
        title: Some("Scores".to_string()),
        filter: vec![],
        sort: None,
        agg: None,
        top: None,
        bins: None,
        height: None,
        diff: None,
    };

    let base_dir = std::path::Path::new("fixtures");
    let result = super::load_chart_data(&block, base_dir, &crate::theme::Theme::dark());
    assert!(
        result.is_ok(),
        "JSON source should work via loader: {:?}",
        result.err()
    );

    if let Ok(crate::render::ChartData::Bar(data)) = result {
        assert_eq!(data.labels, vec!["Alice", "Bob", "Charlie"]);
        assert_eq!(data.values, vec![85.0, 92.0, 78.0]);
    } else {
        panic!("Expected Bar chart data from JSON source");
    }
}

#[test]
fn test_load_chart_data_infers_type_when_not_specified() {
    // departments.csv has 2 categorical columns → should infer Heatmap
    let block = ChartBlock {
        source: "departments.csv".to_string(),
        chart_type: None, // No explicit type
        x_col: None,
        y_col: None,
        color_col: None,
        title: None,
        filter: vec![],
        sort: None,
        agg: None,
        top: None,
        bins: None,
        height: None,
        diff: None,
    };

    let base_dir = std::path::Path::new("fixtures");
    let result = super::load_chart_data(&block, base_dir, &crate::theme::Theme::dark());
    assert!(result.is_ok(), "Should load: {:?}", result.err());
    assert!(
        matches!(result.unwrap(), crate::render::ChartData::Heatmap(_)),
        "Cat×Cat data should infer Heatmap"
    );
}

#[test]
fn test_load_chart_data_infers_line_for_temporal() {
    // sales.csv has temporal x + quantitative y → should infer Line
    let block = ChartBlock {
        source: "sales.csv".to_string(),
        chart_type: None,
        x_col: None,
        y_col: None,
        color_col: None,
        title: None,
        filter: vec![],
        sort: None,
        agg: None,
        top: None,
        bins: None,
        height: None,
        diff: None,
    };

    let base_dir = std::path::Path::new("fixtures");
    let result = super::load_chart_data(&block, base_dir, &crate::theme::Theme::dark());
    assert!(result.is_ok(), "Should load: {:?}", result.err());
    assert!(
        matches!(result.unwrap(), crate::render::ChartData::Line(_)),
        "Temporal×Quant data should infer Line"
    );
}

#[test]
fn test_jump_to_slide_with_digits() {
    let pres = Presentation {
        slides: (0..10)
            .map(|i| Slide {
                title: Some(format!("Slide {}", i + 1)),
                content: vec![],
            })
            .collect(),
    };
    let mut app = PresentApp::new(pres, std::path::PathBuf::new(), crate::theme::Theme::dark());
    assert_eq!(app.current_slide, 0);

    // Type "5" then Enter → jump to slide 5 (index 4)
    app.handle_key(KeyCode::Char('5'));
    app.handle_key(KeyCode::Enter);
    assert_eq!(
        app.current_slide, 4,
        "Should jump to slide 5 (0-indexed: 4)"
    );
}

#[test]
fn test_jump_to_slide_clamped_to_max() {
    let pres = Presentation {
        slides: (0..3)
            .map(|i| Slide {
                title: Some(format!("Slide {}", i + 1)),
                content: vec![],
            })
            .collect(),
    };
    let mut app = PresentApp::new(pres, std::path::PathBuf::new(), crate::theme::Theme::dark());

    // Type "99" then Enter → clamped to last slide (index 2)
    app.handle_key(KeyCode::Char('9'));
    app.handle_key(KeyCode::Char('9'));
    app.handle_key(KeyCode::Enter);
    assert_eq!(app.current_slide, 2, "Should be clamped to last slide");
}

#[test]
fn test_jump_to_slide_zero_goes_to_first() {
    let pres = Presentation {
        slides: (0..5)
            .map(|i| Slide {
                title: Some(format!("Slide {}", i + 1)),
                content: vec![],
            })
            .collect(),
    };
    let mut app = PresentApp::new(pres, std::path::PathBuf::new(), crate::theme::Theme::dark());
    app.current_slide = 3;

    // Type "0" then Enter → clamped to 0 (first slide)
    app.handle_key(KeyCode::Char('0'));
    app.handle_key(KeyCode::Enter);
    assert_eq!(app.current_slide, 0, "Slide 0 (or 1) should go to first");
}

#[test]
fn test_jump_escape_clears_buffer() {
    let pres = Presentation {
        slides: (0..10)
            .map(|i| Slide {
                title: Some(format!("Slide {}", i + 1)),
                content: vec![],
            })
            .collect(),
    };
    let mut app = PresentApp::new(pres, std::path::PathBuf::new(), crate::theme::Theme::dark());
    app.current_slide = 2;

    // Type "7" then Escape → should NOT jump, stay at 2
    app.handle_key(KeyCode::Char('7'));
    // Now the buffer has "7", but 'q' should quit, not jump
    // Actually Esc should clear buffer without quitting (when buffer is non-empty)
    app.handle_key(KeyCode::Esc);
    assert_eq!(app.current_slide, 2, "Escape should cancel jump");
    assert!(
        !app.should_quit,
        "Escape with pending input should not quit"
    );
}

#[test]
fn test_parse_inline_spans_plain_text() {
    let spans = parse_inline_spans("hello world");
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].content, "hello world");
}

#[test]
fn test_parse_inline_spans_bold() {
    let spans = parse_inline_spans("before **bold** after");
    assert_eq!(spans.len(), 3);
    assert_eq!(spans[0].content, "before ");
    assert_eq!(spans[1].content, "bold");
    assert!(spans[1].style.add_modifier.contains(Modifier::BOLD));
    assert_eq!(spans[2].content, " after");
}

#[test]
fn test_parse_inline_spans_italic() {
    let spans = parse_inline_spans("some *italic* text");
    assert_eq!(spans.len(), 3);
    assert_eq!(spans[1].content, "italic");
    assert!(spans[1].style.add_modifier.contains(Modifier::ITALIC));
}

#[test]
fn test_parse_inline_spans_code() {
    use ratatui::style::Color;
    let spans = parse_inline_spans("use `code` here");
    assert_eq!(spans.len(), 3);
    assert_eq!(spans[1].content, "code");
    assert_eq!(spans[1].style.fg, Some(Color::Yellow));
}

#[test]
fn test_parse_inline_spans_empty_string() {
    let spans = parse_inline_spans("");
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].content, "");
}

#[test]
fn test_parse_inline_spans_unclosed_bold() {
    // "**" without a closing pair — the italic matcher picks up the two *'s
    let spans = parse_inline_spans("hello **world");
    assert!(!spans.is_empty());
    // All original text content is preserved (markers consumed as formatting)
    let reconstructed: String = spans.iter().map(|s| s.content.as_ref()).collect();
    assert_eq!(reconstructed, "hello world");
}

#[test]
fn test_parse_inline_spans_unclosed_italic() {
    // Single "*" with no closing — treated as plain text
    let spans = parse_inline_spans("hello * world");
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].content, "hello * world");
}

#[test]
fn test_parse_inline_spans_empty_bold_markers() {
    // Adjacent "****" — opening "**" and immediately closing "**" with empty content
    let spans = parse_inline_spans("before **** after");
    assert!(
        spans
            .iter()
            .any(|s| s.style.add_modifier.contains(Modifier::BOLD))
    );
    let reconstructed: String = spans.iter().map(|s| s.content.as_ref()).collect();
    assert_eq!(reconstructed, "before  after");
}
