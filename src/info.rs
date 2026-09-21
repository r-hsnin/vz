//! Column metadata display (--info flag).

use std::path::Path;

use crate::chart;
use crate::infer::types::Schema;
use crate::loader::LoadedData;
use crate::output;

/// Print column metadata for --info flag.
pub fn print_info(file: &Path, data: &LoadedData, schema: &Schema) {
    println!("File: {}", file.display());
    println!("Rows: {}", data.rows.len());
    println!("Columns: {}", schema.columns.len());
    println!();
    let name_w = schema
        .columns
        .iter()
        .map(|c| c.name.chars().count())
        .max()
        .unwrap_or(4)
        .clamp(4, 24);
    println!(
        "{:<name_w$}  {:<11}  {:>5}  What this means",
        "Name", "Type", "Nulls"
    );
    println!("{}", "-".repeat(name_w + 32));
    for (i, col) in schema.columns.iter().enumerate() {
        let stats = output::stats_text::compute_column_stats_text(i, &col.data_type, data);
        let meaning = plain_type_hint(&col.data_type, i, data);
        println!(
            "{:<name_w$}  {:<11}  {:>5}  {}",
            col.name,
            col.data_type,
            col.null_count,
            [stats, meaning]
                .into_iter()
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join(" — "),
        );
    }
    println!();
    print_recommendation(schema);
    print_plain_recommendation(schema);
}

/// Print column metadata as JSON for machine-readable output.
pub fn print_info_json(file: &Path, data: &LoadedData, schema: &Schema) -> anyhow::Result<()> {
    let recommendation = chart::select_chart(schema, None, None).ok();
    let output = output::build_info_output(
        &file.display().to_string(),
        data,
        schema,
        recommendation.as_ref(),
    );
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

/// Print the auto-detected chart recommendation for the data.
fn print_recommendation(schema: &Schema) {
    match chart::select_chart(schema, None, None) {
        Ok(rec) => {
            let y_part = rec
                .y_column
                .as_ref()
                .map(|y| format!(", y={}", y))
                .unwrap_or_default();
            let color_part = rec
                .color_column
                .as_ref()
                .map(|c| format!(", color={}", c))
                .unwrap_or_default();
            println!(
                "Recommendation: {} (x={}{}{})",
                rec.chart_type, rec.x_column, y_part, color_part
            );
        }
        Err(_) => {
            println!("Recommendation: (insufficient data for chart selection)");
        }
    }
}

/// One plain sentence saying what vz will draw, for non-engineers.
fn print_plain_recommendation(schema: &Schema) {
    use crate::chart::selector::ChartType;
    let Ok(rec) = chart::select_chart(schema, None, None) else {
        return;
    };
    let sentence = match rec.chart_type {
        ChartType::Line => rec
            .y_column
            .as_deref()
            .map(|y| format!("In plain words: a timeline of {y} over {}.", rec.x_column)),
        ChartType::Bar => rec
            .y_column
            .as_deref()
            .map(|y| format!("In plain words: {y} compared across {}.", rec.x_column)),
        ChartType::Scatter => rec
            .y_column
            .as_deref()
            .map(|y| format!("In plain words: how {y} relates to {}.", rec.x_column)),
        ChartType::Histogram => Some(format!(
            "In plain words: how {} is distributed.",
            rec.x_column
        )),
        ChartType::Heatmap => rec
            .y_column
            .as_deref()
            .map(|y| format!("In plain words: how {} and {} combine.", rec.x_column, y)),
    };
    if let Some(s) = sentence {
        println!("{s}");
    }
}

/// Plain gloss for a column type: what the user can do with it.
fn plain_type_hint(
    data_type: &crate::infer::types::DataType,
    col_idx: usize,
    data: &LoadedData,
) -> String {
    use crate::infer::types::DataType;
    match data_type {
        DataType::Temporal => "dates — good for timelines".to_string(),
        DataType::Quantitative => {
            let nums: Vec<f64> = data
                .rows
                .iter()
                .filter_map(|r| r.get(col_idx).and_then(|v| crate::util::parse_number(v)))
                .filter(|v| v.is_finite())
                .collect();
            match crate::util::min_max(&nums) {
                Some((min, max)) => format!(
                    "numbers — chartable, from {} to {}",
                    crate::render::format_number(min),
                    crate::render::format_number(max)
                ),
                None => "numbers — chartable".to_string(),
            }
        }
        DataType::Categorical => "groups — good for comparing".to_string(),
        DataType::Nominal => "labels — used as names only".to_string(),
    }
}
