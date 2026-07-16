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
    println!("{:<20} {:<15} {:>6}  Stats", "Name", "Type", "Nulls");
    println!("{}", "-".repeat(70));
    for (i, col) in schema.columns.iter().enumerate() {
        let stats = output::stats_text::compute_column_stats_text(i, &col.data_type, data);
        println!(
            "{:<20} {:<15} {:>6}  {}",
            col.name, col.data_type, col.null_count, stats
        );
    }
    println!();
    print_recommendation(schema);
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
