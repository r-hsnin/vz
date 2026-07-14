use super::*;

#[test]
fn test_parse_simple_presentation() {
    let content = r#"# Hello World

Welcome to vz.

---

# Slide 2

- Point one
- Point two
- Point three
"#;
    let pres = parse_presentation(content);
    assert_eq!(pres.slides.len(), 2);
    assert_eq!(pres.slides[0].title, Some("Hello World".to_string()));
    assert_eq!(pres.slides[1].title, Some("Slide 2".to_string()));
}

#[test]
fn test_parse_bullets() {
    let content = "# Lists\n\n- Alpha\n- Beta\n- Gamma\n";
    let pres = parse_presentation(content);
    assert_eq!(pres.slides.len(), 1);
    match &pres.slides[0].content[0] {
        SlideElement::Bullets(items) => {
            assert_eq!(items.len(), 3);
            assert_eq!(items[0], "Alpha");
        }
        _ => panic!("Expected Bullets"),
    }
}

#[test]
fn test_parse_chart_block() {
    let content = r#"# Revenue

```chart
source: sales.csv
x: month
y: revenue
type: bar
title: Monthly Revenue
```
"#;
    let pres = parse_presentation(content);
    assert_eq!(pres.slides.len(), 1);
    match &pres.slides[0].content[0] {
        SlideElement::Chart(chart) => {
            assert_eq!(chart.source, "sales.csv");
            assert_eq!(chart.x_col, Some("month".to_string()));
            assert_eq!(chart.y_col, Some("revenue".to_string()));
            assert_eq!(chart.chart_type, Some(ChartType::Bar));
            assert_eq!(chart.title, Some("Monthly Revenue".to_string()));
        }
        _ => panic!("Expected Chart"),
    }
}

#[test]
fn test_parse_chart_block_with_where() {
    let content = r#"# Filtered

```chart
source: sales.csv
x: date
y: revenue
where: city=Tokyo
where: revenue>500
```
"#;
    let pres = parse_presentation(content);
    match &pres.slides[0].content[0] {
        SlideElement::Chart(chart) => {
            assert_eq!(chart.filter, vec!["city=Tokyo", "revenue>500"]);
        }
        _ => panic!("Expected Chart"),
    }
}

#[test]
fn test_parse_chart_block_minimal() {
    let content = "# Chart\n\n```chart\nsource: data.csv\n```\n";
    let pres = parse_presentation(content);
    match &pres.slides[0].content[0] {
        SlideElement::Chart(chart) => {
            assert_eq!(chart.source, "data.csv");
            assert_eq!(chart.chart_type, None);
            assert_eq!(chart.x_col, None);
            assert_eq!(chart.y_col, None);
        }
        _ => panic!("Expected Chart"),
    }
}

#[test]
fn test_parse_separator_based() {
    let content = "---\nFirst slide\n---\nSecond slide\n---\nThird slide\n";
    let pres = parse_presentation(content);
    assert_eq!(pres.slides.len(), 3);
}

#[test]
fn test_present_app_navigation() {
    let pres = Presentation {
        slides: vec![
            Slide {
                title: Some("1".into()),
                content: vec![],
            },
            Slide {
                title: Some("2".into()),
                content: vec![],
            },
            Slide {
                title: Some("3".into()),
                content: vec![],
            },
        ],
    };
    let mut app = PresentApp::new(pres, std::path::PathBuf::new(), crate::theme::Theme::dark());
    assert_eq!(app.current_slide, 0);

    app.handle_key(KeyCode::Right);
    assert_eq!(app.current_slide, 1);

    app.handle_key(KeyCode::Right);
    assert_eq!(app.current_slide, 2);

    // Can't go past the end
    app.handle_key(KeyCode::Right);
    assert_eq!(app.current_slide, 2);

    app.handle_key(KeyCode::Left);
    assert_eq!(app.current_slide, 1);

    // Jump to start
    app.handle_key(KeyCode::Char('g'));
    assert_eq!(app.current_slide, 0);

    // Jump to end
    app.handle_key(KeyCode::Char('G'));
    assert_eq!(app.current_slide, 2);
}

#[test]
fn test_present_app_quit() {
    let pres = Presentation {
        slides: vec![Slide {
            title: None,
            content: vec![],
        }],
    };
    let mut app = PresentApp::new(pres, std::path::PathBuf::new(), crate::theme::Theme::dark());
    assert!(!app.should_quit);
    app.handle_key(KeyCode::Char('q'));
    assert!(app.should_quit);
}

#[test]
fn test_slide_indicator() {
    let pres = Presentation {
        slides: vec![
            Slide {
                title: None,
                content: vec![],
            },
            Slide {
                title: None,
                content: vec![],
            },
            Slide {
                title: None,
                content: vec![],
            },
        ],
    };
    let mut app = PresentApp::new(pres, std::path::PathBuf::new(), crate::theme::Theme::dark());
    assert_eq!(app.slide_indicator(), "1/3");
    app.handle_key(KeyCode::Right);
    assert_eq!(app.slide_indicator(), "2/3");
}

#[test]
fn test_parse_mixed_content() {
    let content = r#"# Overview

Here is some text.

- Bullet A
- Bullet B

```chart
source: metrics.csv
type: line
```

More text after chart.
"#;
    let pres = parse_presentation(content);
    assert_eq!(pres.slides.len(), 1);
    let elements = &pres.slides[0].content;
    assert_eq!(elements.len(), 4); // text, bullets, chart, text
    assert!(matches!(&elements[0], SlideElement::Text(_)));
    assert!(matches!(&elements[1], SlideElement::Bullets(_)));
    assert!(matches!(&elements[2], SlideElement::Chart(_)));
    assert!(matches!(&elements[3], SlideElement::Text(_)));
}

#[test]
fn test_empty_presentation() {
    let pres = parse_presentation("");
    assert_eq!(pres.slides.len(), 0);
}

#[test]
fn test_load_chart_data_relative_to_base_dir() {
    // Create the chart block as it would appear in demo.md
    let block = ChartBlock {
        source: "sales.csv".to_string(),
        chart_type: Some(ChartType::Line),
        x_col: Some("date".to_string()),
        y_col: Some("revenue".to_string()),
        color_col: None,
        title: Some("Revenue Trend".to_string()),
        filter: vec![],
        sort: None,
        agg: None,
        top: None,
        bins: None,
        height: None,
    };

    // Use the fixtures directory as base_dir (where demo.md lives)
    let base_dir = std::path::Path::new("fixtures");
    let result = super::load_chart_data(&block, base_dir, &crate::theme::Theme::dark());
    assert!(
        result.is_ok(),
        "load_chart_data should resolve sales.csv relative to fixtures/: {:?}",
        result.err()
    );
}

#[test]
fn test_load_chart_data_with_filter() {
    let block = ChartBlock {
        source: "sales.csv".to_string(),
        chart_type: Some(ChartType::Bar),
        x_col: Some("city".to_string()),
        y_col: Some("revenue".to_string()),
        color_col: None,
        title: None,
        filter: vec!["city=Tokyo".to_string()],
        sort: None,
        agg: None,
        top: None,
        bins: None,
        height: None,
    };

    let base_dir = std::path::Path::new("fixtures");
    let result = super::load_chart_data(&block, base_dir, &crate::theme::Theme::dark());
    assert!(
        result.is_ok(),
        "load_chart_data with where filter should succeed: {:?}",
        result.err()
    );
}

#[test]
fn test_load_chart_data_nonexistent_source() {
    let block = ChartBlock {
        source: "nonexistent.csv".to_string(),
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
    };

    let base_dir = std::path::Path::new("fixtures");
    let result = super::load_chart_data(&block, base_dir, &crate::theme::Theme::dark());
    assert!(result.is_err());
}
