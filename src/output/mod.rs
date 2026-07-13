//! Machine-readable output formats for AI agent integration.

pub mod chart_json;
pub mod markdown;
pub mod spark;
pub mod stats_text;
pub mod svg;
pub mod table;

use serde::Serialize;

use crate::chart::selector::ChartRecommendation;
use crate::infer::types::{DataType, Schema};
use crate::loader::LoadedData;

/// Top-level JSON output for `--info --output json`.
#[derive(Debug, Serialize)]
pub struct InfoOutput {
    pub version: u32,
    pub file: String,
    pub rows: usize,
    pub columns: Vec<ColumnOutput>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recommendation: Option<RecommendationOutput>,
    /// First N rows of data as array of objects (for agent inspection).
    pub data: Vec<serde_json::Value>,
}

/// Column metadata in JSON output.
#[derive(Debug, Serialize)]
pub struct ColumnOutput {
    pub name: String,
    #[serde(rename = "type")]
    pub data_type: String,
    pub nulls: usize,
    pub stats: ColumnStats,
}

/// Per-column statistics (variant depends on data type).
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum ColumnStats {
    Quantitative { min: f64, max: f64, mean: f64 },
    Categorical { unique: usize, values: Vec<String> },
    Temporal { min: String, max: String },
    Empty {},
}

/// Chart recommendation in JSON output.
#[derive(Debug, Serialize)]
pub struct RecommendationOutput {
    pub chart_type: String,
    pub x: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub y: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

/// Build JSON info output from schema and data.
pub fn build_info_output(
    file: &str,
    data: &LoadedData,
    schema: &Schema,
    recommendation: Option<&ChartRecommendation>,
) -> InfoOutput {
    let columns = schema
        .columns
        .iter()
        .enumerate()
        .map(|(i, col)| {
            let stats = compute_column_stats(i, &col.data_type, data);
            ColumnOutput {
                name: col.name.clone(),
                data_type: format_data_type(col.data_type),
                nulls: col.null_count,
                stats,
            }
        })
        .collect();

    let rec = recommendation.map(|r| RecommendationOutput {
        chart_type: r.chart_type.to_string().to_lowercase(),
        x: r.x_column.clone(),
        y: r.y_column.clone(),
        color: r.color_column.clone(),
    });

    InfoOutput {
        version: 1,
        file: file.to_string(),
        rows: data.rows.len(),
        columns,
        recommendation: rec,
        data: build_data_sample(&data.headers, &data.rows),
    }
}

fn format_data_type(dt: DataType) -> String {
    match dt {
        DataType::Temporal => "temporal".to_string(),
        DataType::Quantitative => "quantitative".to_string(),
        DataType::Categorical => "categorical".to_string(),
        DataType::Nominal => "nominal".to_string(),
    }
}

/// Maximum rows to include in JSON data sample.
const DATA_SAMPLE_LIMIT: usize = 100;

/// Build a data sample as array of JSON objects (column-name → value).
fn build_data_sample(headers: &[String], rows: &[Vec<String>]) -> Vec<serde_json::Value> {
    rows.iter()
        .take(DATA_SAMPLE_LIMIT)
        .map(|row| {
            let mut obj = serde_json::Map::new();
            for (i, header) in headers.iter().enumerate() {
                let val = row.get(i).map(|s| s.as_str()).unwrap_or("");
                // Try to parse as number for cleaner JSON
                if let Ok(n) = val.parse::<f64>() {
                    obj.insert(
                        header.clone(),
                        serde_json::Value::Number(
                            serde_json::Number::from_f64(n)
                                .unwrap_or_else(|| serde_json::Number::from(0)),
                        ),
                    );
                } else {
                    obj.insert(header.clone(), serde_json::Value::String(val.to_string()));
                }
            }
            serde_json::Value::Object(obj)
        })
        .collect()
}

/// Compute statistics for a single column based on its inferred type.
pub fn compute_column_stats(
    col_idx: usize,
    data_type: &DataType,
    data: &LoadedData,
) -> ColumnStats {
    let values: Vec<&str> = data
        .rows
        .iter()
        .filter_map(|row| row.get(col_idx).map(|s| s.as_str()))
        .filter(|s| !s.trim().is_empty())
        .collect();

    if values.is_empty() {
        return ColumnStats::Empty {};
    }

    match data_type {
        DataType::Quantitative => quantitative_stats(&values),
        DataType::Categorical | DataType::Nominal => categorical_stats(&values),
        DataType::Temporal => temporal_stats(&values),
    }
}

fn quantitative_stats(values: &[&str]) -> ColumnStats {
    let nums: Vec<f64> = values.iter().filter_map(|v| v.parse().ok()).collect();
    if nums.is_empty() {
        return ColumnStats::Empty {};
    }
    let (min, max) = crate::util::min_max(&nums).unwrap_or((0.0, 0.0));
    let mean = nums.iter().sum::<f64>() / nums.len() as f64;
    if !min.is_finite() || !max.is_finite() || !mean.is_finite() {
        return ColumnStats::Empty {};
    }
    ColumnStats::Quantitative { min, max, mean }
}

fn categorical_stats(values: &[&str]) -> ColumnStats {
    let mut unique_set = std::collections::HashSet::new();
    let mut unique_ordered = Vec::new();
    for &v in values {
        if unique_set.insert(v) {
            unique_ordered.push(v.to_string());
        }
    }
    ColumnStats::Categorical {
        unique: unique_set.len(),
        values: unique_ordered,
    }
}

fn temporal_stats(values: &[&str]) -> ColumnStats {
    let min_val = values
        .iter()
        .filter(|v| !v.is_empty())
        .min()
        .unwrap_or(&"")
        .to_string();
    let max_val = values
        .iter()
        .filter(|v| !v.is_empty())
        .max()
        .unwrap_or(&"")
        .to_string();
    ColumnStats::Temporal {
        min: min_val,
        max: max_val,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infer::types::{ColumnMeta, Schema};

    #[test]
    fn test_build_info_output_basic() {
        let data = LoadedData {
            headers: vec!["city".into(), "revenue".into()],
            rows: vec![
                vec!["Tokyo".into(), "1000".into()],
                vec!["Osaka".into(), "2000".into()],
            ],
        };
        let schema = Schema::new(vec![
            ColumnMeta {
                name: "city".into(),
                data_type: DataType::Categorical,
                null_count: 0,
                sample_size: 2,
            },
            ColumnMeta {
                name: "revenue".into(),
                data_type: DataType::Quantitative,
                null_count: 0,
                sample_size: 2,
            },
        ]);

        let output = build_info_output("test.csv", &data, &schema, None);
        assert_eq!(output.version, 1);
        assert_eq!(output.rows, 2);
        assert_eq!(output.columns.len(), 2);
        assert_eq!(output.columns[0].data_type, "categorical");
        assert_eq!(output.columns[1].data_type, "quantitative");
    }

    #[test]
    fn test_info_output_serializes_to_json() {
        let data = LoadedData {
            headers: vec!["x".into()],
            rows: vec![vec!["10".into()], vec!["20".into()]],
        };
        let schema = Schema::new(vec![ColumnMeta {
            name: "x".into(),
            data_type: DataType::Quantitative,
            null_count: 0,
            sample_size: 2,
        }]);

        let output = build_info_output("data.csv", &data, &schema, None);
        let json = serde_json::to_string(&output).unwrap();
        assert!(json.contains("\"version\":1"));
        assert!(json.contains("\"quantitative\""));
        assert!(json.contains("\"min\":10"));
    }

    #[test]
    fn test_info_output_with_recommendation() {
        use crate::chart::selector::{ChartRecommendation, ChartType};

        let data = LoadedData {
            headers: vec!["date".into(), "val".into()],
            rows: vec![vec!["2024-01".into(), "100".into()]],
        };
        let schema = Schema::new(vec![
            ColumnMeta {
                name: "date".into(),
                data_type: DataType::Temporal,
                null_count: 0,
                sample_size: 1,
            },
            ColumnMeta {
                name: "val".into(),
                data_type: DataType::Quantitative,
                null_count: 0,
                sample_size: 1,
            },
        ]);
        let rec = ChartRecommendation {
            chart_type: ChartType::Line,
            x_column: "date".into(),
            y_column: Some("val".into()),
            color_column: None,
        };

        let output = build_info_output("data.csv", &data, &schema, Some(&rec));
        assert!(output.recommendation.is_some());
        let r = output.recommendation.unwrap();
        assert_eq!(r.chart_type, "line");
        assert_eq!(r.x, "date");
        assert_eq!(r.y, Some("val".to_string()));
    }

    #[test]
    fn test_quantitative_stats() {
        let data = LoadedData {
            headers: vec!["n".into()],
            rows: vec![vec!["10".into()], vec!["20".into()], vec!["30".into()]],
        };
        let stats = compute_column_stats(0, &DataType::Quantitative, &data);
        match stats {
            ColumnStats::Quantitative { min, max, mean } => {
                assert!((min - 10.0).abs() < f64::EPSILON);
                assert!((max - 30.0).abs() < f64::EPSILON);
                assert!((mean - 20.0).abs() < f64::EPSILON);
            }
            _ => panic!("Expected Quantitative stats"),
        }
    }

    #[test]
    fn test_data_sample_included() {
        let data = LoadedData {
            headers: vec!["name".into(), "val".into()],
            rows: vec![
                vec!["Alice".into(), "10".into()],
                vec!["Bob".into(), "20".into()],
            ],
        };
        let schema = Schema::new(vec![
            ColumnMeta {
                name: "name".into(),
                data_type: DataType::Categorical,
                null_count: 0,
                sample_size: 2,
            },
            ColumnMeta {
                name: "val".into(),
                data_type: DataType::Quantitative,
                null_count: 0,
                sample_size: 2,
            },
        ]);

        let output = build_info_output("test.csv", &data, &schema, None);
        assert_eq!(output.data.len(), 2);
        assert_eq!(output.data[0]["name"], "Alice");
        assert_eq!(output.data[0]["val"], 10.0);
        assert_eq!(output.data[1]["name"], "Bob");
    }

    #[test]
    fn test_data_sample_limit() {
        let data = LoadedData {
            headers: vec!["x".into()],
            rows: (0..150).map(|i| vec![format!("{}", i)]).collect(),
        };
        let schema = Schema::new(vec![ColumnMeta {
            name: "x".into(),
            data_type: DataType::Quantitative,
            null_count: 0,
            sample_size: 150,
        }]);

        let output = build_info_output("big.csv", &data, &schema, None);
        assert_eq!(output.data.len(), 100); // capped at DATA_SAMPLE_LIMIT
    }

    #[test]
    fn test_info_output_serializes_with_nan_stats() {
        // If all values are non-numeric but type is Quantitative, stats should handle gracefully
        let data = LoadedData {
            headers: vec!["val".into()],
            rows: vec![vec!["not_a_number".into()], vec!["also_bad".into()]],
        };
        let schema = Schema::new(vec![crate::infer::types::ColumnMeta {
            name: "val".into(),
            data_type: DataType::Quantitative,
            null_count: 0,
            sample_size: 2,
        }]);

        let output = build_info_output("test.csv", &data, &schema, None);
        // This must not panic — NaN/Infinity should be handled
        let json = serde_json::to_string_pretty(&output);
        assert!(json.is_ok(), "Serialization failed: {:?}", json.err());
    }

    #[test]
    fn test_temporal_stats_unsorted_data() {
        use crate::loader::LoadedData;
        // Temporal data arriving in non-chronological order
        let data = LoadedData {
            headers: vec!["date".to_string()],
            rows: vec![
                vec!["2024-03-01".to_string()],
                vec!["2024-01-15".to_string()],
                vec!["2024-06-20".to_string()],
                vec!["2024-02-10".to_string()],
            ],
        };
        let stats = compute_column_stats(0, &DataType::Temporal, &data);
        match stats {
            ColumnStats::Temporal { min, max } => {
                assert_eq!(min, "2024-01-15", "min should be earliest date");
                assert_eq!(max, "2024-06-20", "max should be latest date");
            }
            _ => panic!("Expected Temporal stats"),
        }
    }
}
