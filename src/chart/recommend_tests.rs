use super::*;
use crate::chart::selector::AggFunction;
use crate::chart::selector::ChartType;
use crate::cli::Cli;
use crate::infer::types::DataType;
use crate::test_helpers::{make_recommendation, make_schema};
use clap::Parser;

// --- effective_agg ---

#[test]
fn effective_agg_explicit_overrides_all() {
    let cli = Cli::try_parse_from(["vz", "data.csv", "--agg", "mean"]).unwrap();
    let schema = make_schema(&[
        ("city", DataType::Categorical),
        ("revenue", DataType::Quantitative),
    ]);
    let rec = make_recommendation(ChartType::Bar, "city", Some("revenue"), None);
    assert_eq!(effective_agg(&cli, &rec, &schema), AggFunction::Mean);
}

#[test]
fn effective_agg_defaults_to_sum() {
    let cli = Cli::try_parse_from(["vz", "data.csv"]).unwrap();
    let schema = make_schema(&[
        ("city", DataType::Categorical),
        ("revenue", DataType::Quantitative),
    ]);
    let rec = make_recommendation(ChartType::Bar, "city", Some("revenue"), None);
    assert_eq!(effective_agg(&cli, &rec, &schema), AggFunction::Sum);
}

#[test]
fn effective_agg_bar_forced_categorical_y_becomes_count() {
    let cli = Cli::try_parse_from(["vz", "data.csv", "-t", "bar"]).unwrap();
    let schema = make_schema(&[
        ("department", DataType::Categorical),
        ("status", DataType::Categorical),
    ]);
    let rec = make_recommendation(ChartType::Bar, "department", Some("status"), None);
    assert_eq!(effective_agg(&cli, &rec, &schema), AggFunction::Count);
}

// --- parse_y_options ---

#[test]
fn parse_y_options_single_column_no_label() {
    let cli = Cli::try_parse_from(["vz", "data.csv", "-y", "revenue"]).unwrap();
    let opts = parse_y_options(&cli);
    assert_eq!(opts.hint, Some("revenue".to_string()));
    assert_eq!(opts.label_override, None);
    assert!(opts.extra_columns.is_empty());
}

#[test]
fn parse_y_options_multi_y_with_labels() {
    let cli = Cli::try_parse_from(["vz", "data.csv", "-y", "revenue:Rev,profit:Profit"]).unwrap();
    let opts = parse_y_options(&cli);
    assert_eq!(opts.hint, Some("revenue".to_string()));
    assert_eq!(opts.label_override, Some("Rev".to_string()));
    assert_eq!(
        opts.extra_columns,
        vec![("profit".to_string(), Some("Profit".to_string()))]
    );
}

#[test]
fn parse_y_options_no_y_specified() {
    let cli = Cli::try_parse_from(["vz", "data.csv"]).unwrap();
    let opts = parse_y_options(&cli);
    assert_eq!(opts.hint, None);
    assert_eq!(opts.label_override, None);
    assert!(opts.extra_columns.is_empty());
}

// --- build_render_options (oneshot::RenderOptions::from_cli lives in oneshot) ---

#[test]
fn build_render_options_default_values() {
    let cli = Cli::try_parse_from(["vz", "data.csv"]).unwrap();
    let schema = make_schema(&[
        ("month", DataType::Temporal),
        ("revenue", DataType::Quantitative),
    ]);
    let y_opts = parse_y_options(&cli);
    let rec = make_recommendation(ChartType::Bar, "month", Some("revenue"), None);
    let opts = crate::oneshot::RenderOptions::from_cli(&cli, &y_opts, &rec, &schema);
    assert_eq!(opts.width, None);
    assert_eq!(opts.height, None);
    assert_eq!(opts.sort_order, None);
    assert_eq!(opts.agg, AggFunction::Sum);
    assert!(!opts.labels);
    assert_eq!(opts.bins, None);
    assert_eq!(opts.title, None);
    assert_eq!(opts.chart_type_override, None);
}

#[test]
fn build_render_options_with_all_overrides() {
    let cli = Cli::try_parse_from([
        "vz",
        "data.csv",
        "-W",
        "80",
        "-H",
        "20",
        "--top",
        "5",
        "--agg",
        "mean",
        "--title",
        "My Chart",
        "--labels",
        "--bins",
        "15",
        "--theme",
        "light",
        "-y",
        "revenue:Rev,profit",
    ])
    .unwrap();
    let schema = make_schema(&[
        ("city", DataType::Categorical),
        ("revenue", DataType::Quantitative),
        ("profit", DataType::Quantitative),
    ]);
    let y_opts = parse_y_options(&cli);
    let rec = make_recommendation(ChartType::Bar, "city", Some("revenue"), None);
    let opts = crate::oneshot::RenderOptions::from_cli(&cli, &y_opts, &rec, &schema);
    assert_eq!(opts.width, Some(80));
    assert_eq!(opts.height, Some(20));
    assert_eq!(opts.limit, Some(5));
    assert_eq!(opts.agg, AggFunction::Mean);
    assert_eq!(opts.title, Some("My Chart".to_string()));
    assert!(opts.labels);
    assert_eq!(opts.bins, Some(15));
    assert_eq!(opts.y_label_override, Some("Rev"));
    assert_eq!(opts.extra_y_columns, vec![("profit".to_string(), None)]);
}

// --- adjust_bar_recommendation ---

#[test]
fn adjust_bar_x_already_categorical_is_noop() {
    let schema = make_schema(&[
        ("city", DataType::Categorical),
        ("revenue", DataType::Quantitative),
    ]);
    let mut rec = make_recommendation(ChartType::Bar, "city", Some("revenue"), None);
    adjust_bar_recommendation(&mut rec, &schema);
    assert_eq!(rec.x_column, "city");
}

#[test]
fn adjust_bar_quantitative_x_no_categorical_available() {
    let schema = make_schema(&[
        ("x_val", DataType::Quantitative),
        ("y_val", DataType::Quantitative),
    ]);
    let mut rec = make_recommendation(ChartType::Bar, "x_val", Some("y_val"), None);
    adjust_bar_recommendation(&mut rec, &schema);
    assert_eq!(rec.x_column, "x_val");
}

#[test]
fn adjust_bar_swaps_temporal_to_categorical() {
    let schema = make_schema(&[
        ("date", DataType::Temporal),
        ("city", DataType::Categorical),
        ("revenue", DataType::Quantitative),
    ]);
    let mut rec = make_recommendation(ChartType::Bar, "date", Some("revenue"), None);
    adjust_bar_recommendation(&mut rec, &schema);
    assert_eq!(rec.x_column, "city");
}

#[test]
fn adjust_bar_clears_color_when_matches_new_x() {
    let schema = make_schema(&[
        ("date", DataType::Temporal),
        ("city", DataType::Categorical),
        ("revenue", DataType::Quantitative),
    ]);
    let mut rec = make_recommendation(ChartType::Bar, "date", Some("revenue"), Some("city"));
    adjust_bar_recommendation(&mut rec, &schema);
    assert_eq!(rec.x_column, "city");
    assert_eq!(rec.color_column, None);
}

#[test]
fn adjust_bar_preserves_color_when_different() {
    let schema = make_schema(&[
        ("date", DataType::Temporal),
        ("region", DataType::Categorical),
        ("city", DataType::Categorical),
        ("revenue", DataType::Quantitative),
    ]);
    let mut rec = make_recommendation(ChartType::Bar, "date", Some("revenue"), Some("city"));
    adjust_bar_recommendation(&mut rec, &schema);
    assert_eq!(rec.x_column, "region");
    assert_eq!(rec.color_column, Some("city".to_string()));
}

// --- build_recommendation ---

#[test]
fn build_recommendation_basic_temporal_quant() {
    let cli = Cli::try_parse_from(["vz", "data.csv", "-x", "month", "-y", "revenue"]).unwrap();
    let schema = make_schema(&[
        ("month", DataType::Temporal),
        ("revenue", DataType::Quantitative),
    ]);
    let y_opts = parse_y_options(&cli);
    let rec = build_recommendation(&cli, &schema, &y_opts).unwrap();
    assert_eq!(rec.x_column, "month");
    assert_eq!(rec.y_column, Some("revenue".to_string()));
}

#[test]
fn build_recommendation_color_col_overrides() {
    let cli = Cli::try_parse_from([
        "vz", "data.csv", "-x", "month", "-y", "revenue", "-c", "region",
    ])
    .unwrap();
    let schema = make_schema(&[
        ("month", DataType::Temporal),
        ("revenue", DataType::Quantitative),
        ("region", DataType::Categorical),
    ]);
    let y_opts = parse_y_options(&cli);
    let rec = build_recommendation(&cli, &schema, &y_opts).unwrap();
    assert_eq!(rec.color_column, Some("region".to_string()));
}

#[test]
fn build_recommendation_extra_y_clears_color() {
    let cli = Cli::try_parse_from(["vz", "data.csv", "-y", "revenue,profit"]).unwrap();
    let schema = make_schema(&[
        ("month", DataType::Temporal),
        ("revenue", DataType::Quantitative),
        ("profit", DataType::Quantitative),
    ]);
    let y_opts = parse_y_options(&cli);
    let rec = build_recommendation(&cli, &schema, &y_opts).unwrap();
    assert_eq!(rec.color_column, None);
}

#[test]
fn build_recommendation_unknown_extra_y_errors_with_hint() {
    let cli = Cli::try_parse_from(["vz", "data.csv", "-y", "revenue,revnue"]).unwrap();
    let schema = make_schema(&[
        ("month", DataType::Temporal),
        ("revenue", DataType::Quantitative),
    ]);
    let y_opts = parse_y_options(&cli);
    let err = build_recommendation(&cli, &schema, &y_opts)
        .expect_err("typo'd extra-y must not be silently dropped");
    let msg = format!("{err:#}");
    assert!(
        msg.contains("revnue"),
        "error must name the bad column: {msg}"
    );
    assert!(msg.contains("revenue"), "error must hint the fix: {msg}");
}
