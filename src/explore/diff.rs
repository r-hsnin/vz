//! Diff exploration mode: interactive TUI for diff results.

use crossterm::event::KeyCode;

use crate::cli::SortOrder;
use crate::diff::{DiffResult, DiffTimeSeries};

use super::ViewMode;

/// Diff data variant: categorical or temporal.
#[derive(Debug, Clone)]
pub enum DiffData {
    Categorical(DiffResult),
    Temporal(DiffTimeSeries),
}

/// Application state for Diff Explore mode.
pub struct DiffExploreApp {
    pub diff_data: DiffData,
    pub before_name: String,
    pub after_name: String,
    pub should_quit: bool,
    pub view_mode: ViewMode,
    pub table_offset: usize,
    pub status_message: Option<String>,
    pub show_help: bool,
    pub theme: crate::theme::Theme,
    pub sort_order: Option<SortOrder>,
}

impl DiffExploreApp {
    pub fn new(
        diff_data: DiffData,
        before_name: String,
        after_name: String,
        theme: crate::theme::Theme,
    ) -> Self {
        Self {
            diff_data,
            before_name,
            after_name,
            should_quit: false,
            view_mode: ViewMode::Chart,
            table_offset: 0,
            status_message: Some("? help │ d table │ s sort │ q quit".to_string()),
            show_help: false,
            theme,
            sort_order: None,
        }
    }

    pub fn handle_key(&mut self, key: KeyCode) {
        if self.show_help {
            self.show_help = false;
            return;
        }

        match key {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Char('?') => self.show_help = true,
            KeyCode::Char('d') | KeyCode::Tab => match self.view_mode {
                ViewMode::Chart => self.view_mode = ViewMode::Table,
                ViewMode::Table => self.view_mode = ViewMode::Chart,
            },
            KeyCode::Char('s') => self.cycle_sort(),
            KeyCode::Char('j') | KeyCode::Down => self.scroll_table(1),
            KeyCode::Char('k') | KeyCode::Up => self.scroll_table(-1),
            KeyCode::Char('G') | KeyCode::End => self.scroll_table(isize::MAX),
            KeyCode::Char('g') | KeyCode::Home => self.scroll_table(isize::MIN),
            KeyCode::PageDown => self.scroll_table(12),
            KeyCode::PageUp => self.scroll_table(-12),
            KeyCode::Char('y') => self.yank_command(),
            KeyCode::Char('h') | KeyCode::Left | KeyCode::Char('l') | KeyCode::Right => {
                self.status_message = Some("N/A in diff mode".to_string());
            }
            KeyCode::Char('c') | KeyCode::Char('a') => {
                self.status_message = Some("N/A in diff mode".to_string());
            }
            KeyCode::Char('1')
            | KeyCode::Char('2')
            | KeyCode::Char('3')
            | KeyCode::Char('4')
            | KeyCode::Char('0') => {
                self.status_message = Some("N/A in diff mode".to_string());
            }
            _ => {}
        }
    }

    fn scroll_table(&mut self, direction: isize) {
        let max = self.table_row_count().saturating_sub(1);
        match direction {
            isize::MAX => self.table_offset = max,
            isize::MIN => self.table_offset = 0,
            d if d > 0 => self.table_offset = (self.table_offset + d as usize).min(max),
            d => self.table_offset = self.table_offset.saturating_sub(d.unsigned_abs()),
        }
    }

    fn cycle_sort(&mut self) {
        self.sort_order = match self.sort_order {
            None => Some(SortOrder::Desc),
            Some(SortOrder::Desc) => Some(SortOrder::Asc),
            Some(SortOrder::Asc) | Some(SortOrder::None) => None,
        };
        let label = match self.sort_order {
            None => "sort: off",
            Some(SortOrder::Desc) => "sort: desc (by Δ)",
            Some(SortOrder::Asc) => "sort: asc (by Δ)",
            Some(SortOrder::None) => "sort: off",
        };
        self.status_message = Some(label.to_string());
    }

    fn yank_command(&mut self) {
        let sort_part = match self.sort_order {
            Some(SortOrder::Desc) => " --sort desc",
            Some(SortOrder::Asc) => " --sort asc",
            _ => "",
        };
        let cmd = format!("vz {} {}{}", self.before_name, self.after_name, sort_part);
        self.status_message = Some(cmd);
    }

    /// Get the number of rows for the table view.
    pub fn table_row_count(&self) -> usize {
        match &self.diff_data {
            DiffData::Categorical(r) => r.entries.len(),
            DiffData::Temporal(ts) => ts.x_labels.len(),
        }
    }

    /// Get sorted entries for categorical diff (respects sort_order).
    pub fn sorted_entries(&self) -> Vec<&crate::diff::DiffEntry> {
        match &self.diff_data {
            DiffData::Categorical(r) => {
                let mut entries: Vec<&crate::diff::DiffEntry> = r.entries.iter().collect();
                match self.sort_order {
                    Some(SortOrder::Desc) => {
                        entries.sort_by(|a, b| {
                            b.delta
                                .abs()
                                .partial_cmp(&a.delta.abs())
                                .unwrap_or(std::cmp::Ordering::Equal)
                        });
                    }
                    Some(SortOrder::Asc) => {
                        entries.sort_by(|a, b| {
                            a.delta
                                .abs()
                                .partial_cmp(&b.delta.abs())
                                .unwrap_or(std::cmp::Ordering::Equal)
                        });
                    }
                    _ => {}
                }
                entries
            }
            DiffData::Temporal(_) => vec![],
        }
    }

    /// Build table data rows for rendering.
    pub fn table_rows(&self) -> Vec<Vec<String>> {
        match &self.diff_data {
            DiffData::Categorical(_) => {
                let entries = self.sorted_entries();
                entries
                    .iter()
                    .map(|e| {
                        let dir = if e.delta > 0.0 {
                            "▲"
                        } else if e.delta < 0.0 {
                            "▼"
                        } else {
                            "─"
                        };
                        let pct = e
                            .pct_change
                            .map(|p| format!("{:+.1}%", p))
                            .unwrap_or_else(|| "new".to_string());
                        vec![
                            e.label.clone(),
                            format!("{:.1}", e.before),
                            format!("{:.1}", e.after),
                            format!("{:+.1}", e.delta),
                            pct,
                            dir.to_string(),
                        ]
                    })
                    .collect()
            }
            DiffData::Temporal(ts) => {
                let before_map: std::collections::HashMap<usize, f64> =
                    ts.before.iter().map(|(x, y)| (*x as usize, *y)).collect();
                let after_map: std::collections::HashMap<usize, f64> =
                    ts.after.iter().map(|(x, y)| (*x as usize, *y)).collect();

                let mut rows: Vec<Vec<String>> = ts
                    .x_labels
                    .iter()
                    .enumerate()
                    .map(|(i, label)| {
                        let bv = before_map.get(&i).copied().unwrap_or(0.0);
                        let av = after_map.get(&i).copied().unwrap_or(0.0);
                        let delta = av - bv;
                        let dir = if delta > 0.0 {
                            "▲"
                        } else if delta < 0.0 {
                            "▼"
                        } else {
                            "─"
                        };
                        let pct = if bv.abs() > f64::EPSILON {
                            format!("{:+.1}%", delta / bv * 100.0)
                        } else if av.abs() > f64::EPSILON {
                            "new".to_string()
                        } else {
                            "0.0%".to_string()
                        };
                        vec![
                            label.clone(),
                            format!("{:.1}", bv),
                            format!("{:.1}", av),
                            format!("{:+.1}", delta),
                            pct,
                            dir.to_string(),
                        ]
                    })
                    .collect();

                // Apply sort for temporal too
                match self.sort_order {
                    Some(SortOrder::Desc) => {
                        rows.sort_by(|a, b| {
                            let da: f64 = a[3].parse().unwrap_or(0.0);
                            let db: f64 = b[3].parse().unwrap_or(0.0);
                            db.abs()
                                .partial_cmp(&da.abs())
                                .unwrap_or(std::cmp::Ordering::Equal)
                        });
                    }
                    Some(SortOrder::Asc) => {
                        rows.sort_by(|a, b| {
                            let da: f64 = a[3].parse().unwrap_or(0.0);
                            let db: f64 = b[3].parse().unwrap_or(0.0);
                            da.abs()
                                .partial_cmp(&db.abs())
                                .unwrap_or(std::cmp::Ordering::Equal)
                        });
                    }
                    _ => {}
                }
                rows
            }
        }
    }

    /// Get X column name.
    pub fn x_column(&self) -> &str {
        match &self.diff_data {
            DiffData::Categorical(r) => &r.x_column,
            DiffData::Temporal(ts) => &ts.x_column,
        }
    }

    /// Get Y column name.
    pub fn y_column(&self) -> &str {
        match &self.diff_data {
            DiffData::Categorical(r) => &r.y_column,
            DiffData::Temporal(ts) => &ts.y_column,
        }
    }

    /// Get overall percentage change.
    pub fn overall_pct(&self) -> Option<f64> {
        match &self.diff_data {
            DiffData::Categorical(r) => r.overall_pct,
            DiffData::Temporal(ts) => ts.overall_pct,
        }
    }

    /// Get entry count.
    pub fn entry_count(&self) -> usize {
        match &self.diff_data {
            DiffData::Categorical(r) => r.entries.len(),
            DiffData::Temporal(ts) => ts.x_labels.len(),
        }
    }

    /// Is this a temporal diff?
    pub fn is_temporal(&self) -> bool {
        matches!(&self.diff_data, DiffData::Temporal(_))
    }
}

#[cfg(test)]
mod tests;
