//! Minimal aligned-column tables for the terminal.

use std::io::{self, Write};

use anstyle::Style;

use super::width;

#[derive(Debug, Default)]
pub struct Table {
    headers: Vec<String>,
    styles: Vec<Option<Style>>,
    rows: Vec<Vec<String>>,
    /// Column index that may be truncated to fit `max_width`, if any.
    flexible: Option<usize>,
}

impl Table {
    pub fn new(headers: &[&str]) -> Self {
        Table {
            headers: headers.iter().map(|s| s.to_string()).collect(),
            styles: vec![None; headers.len()],
            rows: Vec::new(),
            flexible: None,
        }
    }

    /// Style applied to every cell in a column (headers are always bold).
    pub fn style_column(mut self, col: usize, style: Style) -> Self {
        if col < self.styles.len() {
            self.styles[col] = Some(style);
        }
        self
    }

    /// Mark a column whose cells get truncated so rows fit in `max_width`.
    pub fn flexible_column(mut self, col: usize) -> Self {
        self.flexible = Some(col);
        self
    }

    pub fn push(&mut self, row: Vec<String>) {
        self.rows.push(row);
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// Write with two spaces between columns. `max_width` of `None` never truncates.
    pub fn render(&self, w: &mut dyn Write, max_width: Option<usize>) -> io::Result<()> {
        let ncol = self.headers.len();
        let mut widths: Vec<usize> = self.headers.iter().map(|h| width(h)).collect();
        for row in &self.rows {
            for (i, cell) in row.iter().enumerate().take(ncol) {
                widths[i] = widths[i].max(width(cell));
            }
        }
        if let (Some(flex), Some(max)) = (self.flexible, max_width) {
            let others: usize = widths
                .iter()
                .enumerate()
                .filter(|(i, _)| *i != flex)
                .map(|(_, w)| w)
                .sum();
            let gaps = 2 * ncol.saturating_sub(1);
            let room = max.saturating_sub(others + gaps).max(8);
            widths[flex] = widths[flex].min(room);
        }
        let bold = Style::new().bold();
        let line = |w: &mut dyn Write,
                    cells: &[String],
                    style: &dyn Fn(usize) -> Option<Style>|
         -> io::Result<()> {
            let mut out = String::new();
            for (i, col_width) in widths.iter().copied().enumerate() {
                let raw = cells.get(i).map(String::as_str).unwrap_or("");
                let cell = if Some(i) == self.flexible {
                    super::truncate(raw, col_width)
                } else {
                    raw.to_string()
                };
                let pad = col_width.saturating_sub(width(&cell));
                let styled = match style(i) {
                    Some(s) => format!("{s}{cell}{s:#}"),
                    None => cell,
                };
                out.push_str(&styled);
                if i + 1 < ncol {
                    out.push_str(&" ".repeat(pad + 2));
                }
            }
            writeln!(w, "{}", out.trim_end())
        };
        line(w, &self.headers, &|_| Some(bold))?;
        for row in &self.rows {
            line(w, row, &|i| self.styles[i])?;
        }
        Ok(())
    }
}
