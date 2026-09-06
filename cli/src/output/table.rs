/// A minimal aligned-column table renderer for scriptable, non-interactive
/// command output (e.g. `session list`). Colorized/styled output lives in
/// this module once the HTTP response renderer needs it too.
pub struct Table {
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
}

impl Table {
    pub fn new<const N: usize>(headers: [&str; N]) -> Self {
        Self {
            headers: headers.iter().map(|s| s.to_string()).collect(),
            rows: Vec::new(),
        }
    }

    pub fn push<const N: usize>(&mut self, row: [String; N]) {
        self.rows.push(row.to_vec());
    }

    pub fn print(&self) {
        let widths = self.column_widths();

        self.print_row(&self.headers, &widths);
        let rule: Vec<String> = widths.iter().map(|w| "-".repeat(*w)).collect();
        println!("{}", rule.join("  "));

        for row in &self.rows {
            self.print_row(row, &widths);
        }
    }

    fn column_widths(&self) -> Vec<usize> {
        let cols = self.headers.len();
        let mut widths: Vec<usize> = self.headers.iter().map(|h| h.chars().count()).collect();
        for row in &self.rows {
            for (i, cell) in row.iter().enumerate().take(cols) {
                widths[i] = widths[i].max(cell.chars().count());
            }
        }
        widths
    }

    fn print_row(&self, cells: &[String], widths: &[usize]) {
        let line: Vec<String> = cells
            .iter()
            .enumerate()
            .map(|(i, cell)| format!("{:width$}", cell, width = widths[i]))
            .collect();
        println!("{}", line.join("  ").trim_end());
    }
}
