//! Row filtering for data subsets.
//!
//! Supports simple predicates: `col=value`, `col!=value`, `col>value`, `col<value`.

use crate::loader::LoadedData;
use anyhow::{Context, Result};

/// A parsed filter predicate.
#[derive(Debug, Clone, PartialEq)]
pub struct Predicate {
    pub column: String,
    pub op: FilterOp,
    pub value: String,
}

/// Supported filter operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterOp {
    Eq,
    NotEq,
    Gt,
    Lt,
    Gte,
    Lte,
}

/// Parse a filter expression like "city=Tokyo" or "revenue>1000".
pub fn parse_predicate(expr: &str) -> Result<Predicate> {
    // Try multi-char operators first
    for (pat, op) in [
        ("!=", FilterOp::NotEq),
        (">=", FilterOp::Gte),
        ("<=", FilterOp::Lte),
    ] {
        if let Some((col, val)) = expr.split_once(pat) {
            let col = col.trim().to_string();
            let val = val.trim().to_string();
            if col.is_empty() {
                anyhow::bail!("Invalid filter: missing column name in '{expr}'");
            }
            return Ok(Predicate {
                column: col,
                op,
                value: val,
            });
        }
    }
    // Single-char operators
    for (pat, op) in [
        ('>', FilterOp::Gt),
        ('<', FilterOp::Lt),
        ('=', FilterOp::Eq),
    ] {
        if let Some((col, val)) = expr.split_once(pat) {
            let col = col.trim().to_string();
            let val = val.trim().to_string();
            if col.is_empty() {
                anyhow::bail!("Invalid filter: missing column name in '{expr}'");
            }
            return Ok(Predicate {
                column: col,
                op,
                value: val,
            });
        }
    }
    anyhow::bail!(
        "Invalid filter expression: '{expr}'. Expected format: col=value, col>value, col<value"
    )
}

/// Apply predicates to loaded data, returning only matching rows.
pub fn filter_data(data: LoadedData, predicates: &[Predicate]) -> Result<LoadedData> {
    if predicates.is_empty() {
        return Ok(data);
    }

    // Resolve column indices for each predicate
    let resolved: Vec<(usize, &FilterOp, &str)> = predicates
        .iter()
        .map(|p| {
            let idx = data
                .headers
                .iter()
                .position(|h| h == &p.column)
                .with_context(|| {
                    format!(
                        "Filter column '{}' not found. Available: {:?}",
                        p.column, data.headers
                    )
                })?;
            Ok((idx, &p.op, p.value.as_str()))
        })
        .collect::<Result<Vec<_>>>()?;

    let rows = data
        .rows
        .into_iter()
        .filter(|row| {
            resolved
                .iter()
                .all(|(idx, op, val)| matches_row(row, *idx, op, val))
        })
        .collect();

    Ok(LoadedData {
        headers: data.headers,
        rows,
    })
}

/// Check if a single row satisfies a predicate.
fn matches_row(row: &[String], col_idx: usize, op: &FilterOp, value: &str) -> bool {
    let cell = match row.get(col_idx) {
        Some(v) => v.as_str(),
        None => return false,
    };

    match op {
        FilterOp::Eq => cell == value,
        FilterOp::NotEq => cell != value,
        FilterOp::Gt | FilterOp::Lt | FilterOp::Gte | FilterOp::Lte => {
            // Try numeric comparison first, fall back to string
            if let (Ok(a), Ok(b)) = (cell.parse::<f64>(), value.parse::<f64>()) {
                match op {
                    FilterOp::Gt => a > b,
                    FilterOp::Lt => a < b,
                    FilterOp::Gte => a >= b,
                    FilterOp::Lte => a <= b,
                    _ => unreachable!(),
                }
            } else {
                match op {
                    FilterOp::Gt => cell > value,
                    FilterOp::Lt => cell < value,
                    FilterOp::Gte => cell >= value,
                    FilterOp::Lte => cell <= value,
                    _ => unreachable!(),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_predicate_eq() {
        let p = parse_predicate("city=Tokyo").unwrap();
        assert_eq!(p.column, "city");
        assert_eq!(p.op, FilterOp::Eq);
        assert_eq!(p.value, "Tokyo");
    }

    #[test]
    fn test_parse_predicate_gt() {
        let p = parse_predicate("revenue>1000").unwrap();
        assert_eq!(p.column, "revenue");
        assert_eq!(p.op, FilterOp::Gt);
        assert_eq!(p.value, "1000");
    }

    #[test]
    fn test_parse_predicate_not_eq() {
        let p = parse_predicate("status!=active").unwrap();
        assert_eq!(p.column, "status");
        assert_eq!(p.op, FilterOp::NotEq);
        assert_eq!(p.value, "active");
    }

    #[test]
    fn test_parse_predicate_gte() {
        let p = parse_predicate("age>=18").unwrap();
        assert_eq!(p.column, "age");
        assert_eq!(p.op, FilterOp::Gte);
        assert_eq!(p.value, "18");
    }

    #[test]
    fn test_parse_predicate_invalid() {
        assert!(parse_predicate("noop").is_err());
        assert!(parse_predicate("=value").is_err());
    }

    #[test]
    fn test_filter_data_eq() {
        let data = LoadedData {
            headers: vec!["city".into(), "revenue".into()],
            rows: vec![
                vec!["Tokyo".into(), "1000".into()],
                vec!["Osaka".into(), "2000".into()],
                vec!["Tokyo".into(), "1500".into()],
            ],
        };
        let pred = parse_predicate("city=Tokyo").unwrap();
        let result = filter_data(data, &[pred]).unwrap();
        assert_eq!(result.rows.len(), 2);
        assert_eq!(result.rows[0][0], "Tokyo");
        assert_eq!(result.rows[1][0], "Tokyo");
    }

    #[test]
    fn test_filter_data_numeric_gt() {
        let data = LoadedData {
            headers: vec!["city".into(), "revenue".into()],
            rows: vec![
                vec!["Tokyo".into(), "1000".into()],
                vec!["Osaka".into(), "2000".into()],
                vec!["Tokyo".into(), "500".into()],
            ],
        };
        let pred = parse_predicate("revenue>900").unwrap();
        let result = filter_data(data, &[pred]).unwrap();
        assert_eq!(result.rows.len(), 2);
    }

    #[test]
    fn test_filter_data_invalid_column() {
        let data = LoadedData {
            headers: vec!["city".into()],
            rows: vec![vec!["Tokyo".into()]],
        };
        let pred = parse_predicate("missing=x").unwrap();
        assert!(filter_data(data, &[pred]).is_err());
    }

    #[test]
    fn test_filter_data_multiple_predicates() {
        let data = LoadedData {
            headers: vec!["city".into(), "revenue".into()],
            rows: vec![
                vec!["Tokyo".into(), "1000".into()],
                vec!["Tokyo".into(), "500".into()],
                vec!["Osaka".into(), "2000".into()],
            ],
        };
        let p1 = parse_predicate("city=Tokyo").unwrap();
        let p2 = parse_predicate("revenue>800").unwrap();
        let result = filter_data(data, &[p1, p2]).unwrap();
        assert_eq!(result.rows.len(), 1);
        assert_eq!(result.rows[0][1], "1000");
    }
}
