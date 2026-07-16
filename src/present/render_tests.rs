use super::*;
use ratatui::{Terminal, backend::TestBackend, layout::Rect};

fn make_app(slides: Vec<Slide>) -> PresentApp {
    use super::super::Presentation;
    PresentApp {
        presentation: Presentation { slides },
        current_slide: 0,
        should_quit: false,
        base_dir: std::path::PathBuf::from("."),
        input_buffer: String::new(),
        theme: crate::theme::Theme::default(),
    }
}

#[test]
fn test_element_constraint_chart() {
    let el = SlideElement::Chart(ChartBlock {
        source: "data.csv".into(),
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
    });
    assert_eq!(element_constraint(&el), Constraint::Min(10));
}

#[test]
fn test_element_constraint_text() {
    let el = SlideElement::Text("Hello world".into());
    assert_eq!(element_constraint(&el), Constraint::Length(2));
}

#[test]
fn test_element_constraint_bullets() {
    let el = SlideElement::Bullets(vec!["a".into(), "b".into(), "c".into()]);
    assert_eq!(element_constraint(&el), Constraint::Length(4)); // 3 items + 1
}

#[test]
fn test_element_constraint_code() {
    let el = SlideElement::Code {
        language: Some("rust".into()),
        content: "fn main() {\n    println!(\"hi\");\n}".into(),
    };
    // 3 lines + 2 (border)
    assert_eq!(element_constraint(&el), Constraint::Length(5));
}

#[test]
fn test_element_constraint_heading() {
    let el = SlideElement::Heading {
        level: 2,
        text: "Title".into(),
    };
    assert_eq!(element_constraint(&el), Constraint::Length(2));
}

#[test]
fn test_element_constraint_ordered_list() {
    let el = SlideElement::OrderedList(vec!["one".into(), "two".into()]);
    assert_eq!(element_constraint(&el), Constraint::Length(3)); // 2 items + 1
}

#[test]
fn test_element_constraint_table() {
    let el = SlideElement::Table {
        headers: vec!["A".into(), "B".into()],
        rows: vec![vec!["1".into(), "2".into()], vec!["3".into(), "4".into()]],
    };
    assert_eq!(element_constraint(&el), Constraint::Length(6)); // 2 rows + 4
}

#[test]
fn test_draw_slide_renders_footer() {
    let slide = Slide {
        title: Some("Test Slide".into()),
        content: vec![SlideElement::Text("Body text".into())],
    };
    let app = make_app(vec![slide]);
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|frame| draw_slide(frame, &app)).unwrap();
    let buffer = terminal.backend().buffer().clone();
    // Footer should contain slide indicator "1/1"
    let content = buffer_to_string(&buffer);
    assert!(
        content.contains("1/1"),
        "Footer should show slide indicator"
    );
    assert!(
        content.contains("navigate"),
        "Footer should show navigation hint"
    );
}

#[test]
fn test_draw_slide_renders_title() {
    let slide = Slide {
        title: Some("My Title".into()),
        content: vec![],
    };
    let app = make_app(vec![slide]);
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|frame| draw_slide(frame, &app)).unwrap();
    let buffer = terminal.backend().buffer().clone();
    let content = buffer_to_string(&buffer);
    assert!(
        content.contains("My Title"),
        "Should render slide title, got: {}",
        content
    );
}

#[test]
fn test_render_code_block_shows_language() {
    let backend = TestBackend::new(40, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| {
            render_code_block(frame, Some("python"), "print('hi')", Rect::new(0, 0, 40, 5));
        })
        .unwrap();
    let buffer = terminal.backend().buffer().clone();
    let content = buffer_to_string(&buffer);
    assert!(
        content.contains("python"),
        "Should show language label, got: {}",
        content
    );
}

/// Helper: convert buffer to a single string for searching.
fn buffer_to_string(buf: &ratatui::buffer::Buffer) -> String {
    let area = buf.area();
    let mut s = String::new();
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            s.push_str(buf.cell((x, y)).map_or(" ", |c| c.symbol()));
        }
        s.push('\n');
    }
    s
}

#[test]
fn test_build_progress_bar_first_slide() {
    let bar = build_progress_bar(0, 5, 10);
    assert_eq!(bar.chars().next(), Some('●'));
    assert_eq!(bar.chars().count(), 10);
}

#[test]
fn test_build_progress_bar_last_slide() {
    let bar = build_progress_bar(4, 5, 10);
    assert_eq!(bar.chars().last(), Some('●'));
    assert_eq!(bar.chars().count(), 10);
}

#[test]
fn test_build_progress_bar_middle() {
    let bar = build_progress_bar(2, 5, 9);
    let chars: Vec<char> = bar.chars().collect();
    assert_eq!(chars[4], '●'); // position 2/(5-1) * (9-1) = 4
}

#[test]
fn test_build_progress_bar_single_slide() {
    let bar = build_progress_bar(0, 1, 10);
    assert!(bar.is_empty());
}

#[test]
fn test_build_progress_bar_narrow() {
    let bar = build_progress_bar(0, 5, 2);
    assert!(bar.is_empty()); // width < 3
}
