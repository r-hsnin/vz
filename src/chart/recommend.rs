//! Recommendation assembly: query hints + schema → `ChartRecommendation`.
//!
//! Single home for the former `helpers::{build_recommendation,
//! adjust_bar_recommendation, effective_agg, parse_y_options, YOptions}`.
//! Plan note: the app-plane adapters (`build_recommendation`,
//! `effective_agg`) take the Cli-free [`Query`] and return [`Warnings`]
//! instead of printing; the only `Cli → Query` conversion lives in the app
//! plane (`Cli::to_query` / `Cli::to_pipeline_params`). The pure core is
//! (`select_chart`, `adjust_bar_recommendation`,
//! `validate_extra_y_columns`).

use anyhow::Result;

use crate::chart::selector::{
    AggFunction, ChartRecommendation, ChartType, fallback_warning, select_chart,
};
use crate::cli::{ChartTypeArg, parse_column_spec, parse_multi_y_specs};
use crate::diagnostics::{format_column_suffix, suggest_column};
use crate::infer::types::{DataType, Schema};

/// Cli-free inputs for recommendation assembly (the Phase 2 `Query` seam).
///
/// Built once in the app plane (`Cli::to_query`); every
/// downstream consumer takes this instead of `&Cli`.
#[derive(Debug, Clone, Default)]
pub struct Query {
    pub x_col: Option<String>,
    pub y_col: Option<String>,
    pub chart_type: Option<ChartTypeArg>,
    pub color_col: Option<String>,
    pub agg: Option<AggFunction>,
}

/// Notifications that stay visible but no longer print from core.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Warnings(pub Vec<String>);

impl Warnings {
    pub fn push(&mut self, w: String) {
        self.0.push(w);
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

/// Parsed Y-axis options from CLI.
pub struct YOptions {
    pub hint: Option<String>,
    pub label_override: Option<String>,
    pub extra_columns: Vec<(String, Option<String>)>,
}

/// Parse Y-axis options: primary Y hint, label override, and extra Y columns.
pub fn parse_y_options(y_col: Option<&str>) -> YOptions {
    let y_specs: Vec<(&str, Option<&str>)> =
        y_col.map(|s| parse_multi_y_specs(s)).unwrap_or_default();
    let hint = y_specs.first().map(|(col, _)| col.to_string());
    let label_override = y_specs
        .first()
        .and_then(|(_, label)| *label)
        .map(|s| s.to_string());
    let extra_columns: Vec<(String, Option<String>)> = y_specs
        .iter()
        .skip(1)
        .map(|(col, label)| (col.to_string(), label.map(|l| l.to_string())))
        .collect();
    YOptions {
        hint,
        label_override,
        extra_columns,
    }
}

/// Determine effective aggregation function.
/// Auto-switches to Count when bar chart is forced on a categorical Y column
/// that was auto-inferred (not explicitly specified by the user).
pub fn effective_agg(
    query: &Query,
    recommendation: &ChartRecommendation,
    schema: &Schema,
) -> AggFunction {
    if let Some(agg) = query.agg {
        return agg;
    }

    // When bar chart is forced, Y was auto-inferred (not explicit), and Y is not numeric,
    // default to Count. This handles the case where both columns are categorical
    // (e.g., departments.csv with department + status).
    // Self-aggregation (`-x city` alone → x=y=city) also counts rows per category.
    if query.chart_type == Some(ChartTypeArg::Bar) && query.y_col.is_none() {
        let y_is_categorical = recommendation
            .y_column
            .as_ref()
            .and_then(|y| schema.find_column(y))
            .map(|c| c.data_type != DataType::Quantitative)
            .unwrap_or(false);
        if y_is_categorical {
            return AggFunction::Count;
        }
    }

    // Categorical X with no quantitative Y (incl. `-x city` alone):
    // count rows per category (matches the README "count auto-applied" claim).
    if query.y_col.is_none()
        && let Some(x_name) = query.x_col.as_deref().map(|s| parse_column_spec(s).0)
        && let Some(x_meta) = schema.find_column(x_name)
        && x_meta.data_type == DataType::Categorical
        && schema.columns_of_type(DataType::Quantitative).is_empty()
    {
        return AggFunction::Count;
    }

    AggFunction::Sum
}

pub fn build_recommendation(
    query: &Query,
    schema: &Schema,
    y_opts: &YOptions,
) -> Result<(ChartRecommendation, Warnings)> {
    let x_hint = query.x_col.as_deref().map(|s| parse_column_spec(s).0);
    let mut recommendation = select_chart(schema, x_hint, y_opts.hint.as_deref())?;
    validate_extra_y_columns(schema, y_opts)?;

    if query.chart_type == Some(ChartTypeArg::Bar) && query.x_col.is_none() {
        adjust_bar_recommendation(&mut recommendation, schema);
    }

    let mut warnings = Warnings::default();
    if let Some(warning) = fallback_warning(
        schema,
        &recommendation.x_column,
        recommendation.y_column.as_deref(),
        recommendation.chart_type,
    ) {
        warnings.push(warning);
    }

    if let Some(ref color) = query.color_col {
        recommendation.color_column = Some(color.clone());
    }

    if !y_opts.extra_columns.is_empty() && query.color_col.is_none() {
        recommendation.color_column = None;
    }

    if let Some(warning) = bar_color_ignored_warning(&recommendation) {
        warnings.push(warning);
    }

    Ok((recommendation, warnings))
}

/// Warning when `-c` names a column that no renderer consumes: grouped bars are
/// not implemented, so the color column only reaches the summary legend —
/// bar heights stay aggregated over all rows. Without this, `color=prod […]`
/// reads as if the chart were split by `prod` when it is not.
/// Returns the message instead of printing (callers print at the edge).
fn bar_color_ignored_warning(recommendation: &ChartRecommendation) -> Option<String> {
    if recommendation.chart_type == ChartType::Bar && recommendation.color_column.is_some() {
        Some(
            "warning: -c/--color has no effect on bar chart data (grouped bars are not supported); \
             showing aggregated values over all rows"
                .to_string(),
        )
    } else {
        None
    }
}

/// Validate extra `-y` columns against the schema.
///
/// The primary `-y` column is validated by [`select_chart`]; extra
/// columns were previously dropped silently, so `vz f.csv -y revenue,revnue`
/// rendered a single series without a word. Unknown names are an error with
/// a `Did you mean` hint instead.
fn validate_extra_y_columns(schema: &Schema, y_opts: &YOptions) -> Result<()> {
    let available: Vec<String> = schema.columns.iter().map(|c| c.name.clone()).collect();
    for (col, _) in &y_opts.extra_columns {
        if schema.find_column(col).is_none() {
            let suggestion = suggest_column(&available, col).map(|s| s.as_str().to_string());
            let suffix = format_column_suffix(suggestion.as_deref(), col);
            anyhow::bail!(
                "Column '{col}' not found. Available columns: {}{suffix}",
                available.join(", "),
            );
        }
    }
    Ok(())
}

/// When user overrides to bar chart, prefer a categorical column for X-axis.
pub fn adjust_bar_recommendation(recommendation: &mut ChartRecommendation, schema: &Schema) {
    let x_meta = schema
        .columns
        .iter()
        .find(|c| c.name == recommendation.x_column);
    if x_meta.map(|c| c.data_type) == Some(DataType::Categorical) {
        return;
    }

    let cat_cols = schema.columns_of_type(DataType::Categorical);
    if let Some(cat_col) = cat_cols.first() {
        recommendation.x_column = cat_col.name.clone();

        if recommendation.color_column.as_deref() == Some(cat_col.name.as_str()) {
            recommendation.color_column = None;
        }
    }
}

#[cfg(test)]
#[path = "recommend_tests.rs"]
mod tests;
