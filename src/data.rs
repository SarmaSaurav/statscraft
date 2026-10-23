use polars::prelude::*;
use polars_readstat_rs::SpssReader as SavReader;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Missing,
    Number(f64),
    Text(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnKind {
    Numeric,
    String,
}

/// StatsBench "Measure": Scale / Ordinal / Nominal (Variable View).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Measure {
    Nominal,
    Ordinal,
    Scale,
}

impl Measure {
    pub fn as_str(self) -> &'static str {
        match self {
            Measure::Nominal => "Nominal",
            Measure::Ordinal => "Ordinal",
            Measure::Scale => "Scale",
        }
    }
    pub fn default_for(kind: ColumnKind) -> Self {
        match kind {
            ColumnKind::Numeric => Measure::Scale,
            ColumnKind::String => Measure::Nominal,
        }
    }
}

/// StatsBench "Align" in Variable View.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VarAlign {
    Left,
    Right,
    Center,
}

impl VarAlign {
    pub fn as_str(self) -> &'static str {
        match self {
            VarAlign::Left => "Left",
            VarAlign::Right => "Right",
            VarAlign::Center => "Center",
        }
    }
    pub fn default_for(kind: ColumnKind) -> Self {
        match kind {
            ColumnKind::Numeric => VarAlign::Right,
            ColumnKind::String => VarAlign::Left,
        }
    }
}

/// Per-variable metadata backing the StatsBench-style Variable View.
#[derive(Debug, Clone, PartialEq)]
pub struct VariableMeta {
    /// Variable/Value label ("Label" column). Empty = none.
    pub label: String,
    /// Numeric value labels ("Values" column), e.g. 1.0 -> "male".
    pub value_labels: Vec<(f64, String)>,
    /// Value labels for string variables.
    pub text_value_labels: Vec<(String, String)>,
    /// User-defined discrete missing values (numeric).
    pub user_missing: Vec<f64>,
    /// A user-defined range of missing values, if there is one.
    ///
    /// The discrete values and the range are kept apart, because two
    /// discrete values and a range of two are different things, and
    /// StatsBench's dialog asks which one is meant.
    pub user_missing_range: Option<(f64, f64)>,
    pub measure: Measure,
    pub align: VarAlign,
    /// "Columns" (display width) in Variable View.
    pub columns_width: usize,
    /// "Decimals".
    pub decimals: usize,
    /// StatsBench "Role"; we only model "Input".
    pub role: String,
}

impl VariableMeta {
    pub fn default_for(kind: ColumnKind) -> Self {
        Self {
            label: String::new(),
            value_labels: Vec::new(),
            text_value_labels: Vec::new(),
            user_missing: Vec::new(),
            user_missing_range: None,
            measure: Measure::default_for(kind),
            align: VarAlign::default_for(kind),
            columns_width: 8,
            decimals: if kind == ColumnKind::Numeric { 2 } else { 0 },
            role: "Input".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Column {
    pub name: String,
    pub kind: ColumnKind,
    pub values: Vec<Value>,
    pub meta: VariableMeta,
}

impl Column {
    /// System- or user-missing?
    ///
    /// A single declared value is one discrete missing value; a pair
    /// of them is the ends of a range, which is how StatsBench stores it.
    pub fn is_missing(&self, row: usize) -> bool {
        match self.values.get(row) {
            Some(Value::Missing) => true,
            Some(Value::Number(n)) => {
                if self.meta.user_missing.iter().any(|&v| v == *n) {
                    return true;
                }
                // A range of missing values.
                if let Some((lo, hi)) = self.meta.user_missing_range {
                    let (lo, hi) = (lo.min(hi), lo.max(hi));
                    return *n >= lo && *n <= hi;
                }
                false
            }
            _ => false,
        }
    }

    /// Type change with value conversion (StatsBench converts when possible).
    pub fn set_kind(&mut self, kind: ColumnKind) {
        if kind == self.kind {
            return;
        }
        self.values = self
            .values
            .iter()
            .map(|v| match (kind, v) {
                (ColumnKind::String, Value::Number(n)) => Value::Text(format!("{}", n)),
                (ColumnKind::Numeric, Value::Text(s)) => s
                    .trim()
                    .parse::<f64>()
                    .map(Value::Number)
                    .unwrap_or(Value::Missing),
                (ColumnKind::Numeric, Value::Missing) => Value::Missing,
                (ColumnKind::String, Value::Missing) => Value::Missing,
                (ColumnKind::String, Value::Text(s)) => Value::Text(s.clone()),
                (ColumnKind::Numeric, Value::Number(n)) => Value::Number(*n),
            })
            .collect();
        self.kind = kind;
        let label = self.meta.label.clone();
        self.meta = VariableMeta::default_for(kind);
        self.meta.label = label;
    }

    /// The name StatsBench shows for a variable: its label when it has one,
    /// otherwise its name. Calculations and output use this, while the
    /// syntax and the data model keep using the machine name.
    pub fn display_name(&self) -> &str {
        let label = self.meta.label.trim();
        if label.is_empty() {
            &self.name
        } else {
            label
        }
    }

    /// Value label for a numeric value, if any.
    pub fn value_label_for(&self, v: f64) -> Option<&str> {
        self.meta
            .value_labels
            .iter()
            .find(|(val, _)| (val - v).abs() < f64::EPSILON)
            .map(|(_, l)| l.as_str())
    }

    /// Value label for a text value, if any.
    pub fn text_value_label_for(&self, v: &str) -> Option<&str> {
        self.meta
            .text_value_labels
            .iter()
            .find(|(val, _)| val == v)
            .map(|(_, l)| l.as_str())
    }

    /// The numeric value behind a value label, if the label is one of
    /// this variable's labels.
    ///
    /// The Data Editor uses this so that editing a cell that shows a
    /// label writes the code back, not the label text.
    pub fn value_for_label(&self, label: &str) -> Option<f64> {
        self.meta
            .value_labels
            .iter()
            .find(|(_, l)| l == label)
            .map(|(v, _)| *v)
    }

    /// The text value behind a value label, if the label is one of this
    /// variable's labels.
    pub fn text_value_for_label(&self, label: &str) -> Option<String> {
        self.meta
            .text_value_labels
            .iter()
            .find(|(_, l)| l == label)
            .map(|(v, _)| v.clone())
    }
}

#[derive(Debug, Clone, Default)]
pub struct DataTable {
    pub columns: Vec<Column>,
    pub n_rows: usize,
}

impl DataTable {
    pub fn from_csv_path(path: &Path) -> Result<Self, String> {
        let file = File::open(path).map_err(|e| format!("open {}: {}", path.display(), e))?;
        let mut rdr = csv::Reader::from_reader(BufReader::new(file));
        let headers: Vec<String> = rdr
            .headers()
            .map_err(|e| format!("read headers: {}", e))?
            .iter()
            .map(|s| s.to_string())
            .collect();

        let mut rows: Vec<Vec<String>> = Vec::new();
        for rec in rdr.records() {
            let rec = rec.map_err(|e| format!("read row: {}", e))?;
            // A file written with CRLF line endings leaves a carriage
            // return at the end of the last field, which would become
            // part of the value.
            rows.push(
                rec.iter()
                    .map(|s| s.trim_end_matches('\r').to_string())
                    .collect(),
            );
        }

        let n_cols = headers.len();
        let n_rows = rows.len();

        // infer kinds
        let mut kinds = vec![ColumnKind::Numeric; n_cols];
        for (ci, _) in headers.iter().enumerate() {
            for row in &rows {
                let cell = row.get(ci).map(|s| s.trim()).unwrap_or("");
                if cell.is_empty() {
                    continue;
                }
                if cell.parse::<f64>().is_err() {
                    kinds[ci] = ColumnKind::String;
                    break;
                }
            }
        }

        let mut columns: Vec<Column> = headers
            .into_iter()
            .enumerate()
            .map(|(ci, name)| Column {
                name,
                kind: kinds[ci],
                values: Vec::with_capacity(n_rows),
                meta: VariableMeta::default_for(kinds[ci]),
            })
            .collect();

        for row in &rows {
            for (ci, col) in columns.iter_mut().enumerate() {
                let raw = row.get(ci).map(|s| s.trim()).unwrap_or("");
                let v = if raw.is_empty() {
                    Value::Missing
                } else {
                    match col.kind {
                        ColumnKind::Numeric => raw
                            .parse::<f64>()
                            .map(Value::Number)
                            .unwrap_or_else(|_| Value::Text(raw.to_string())),
                        ColumnKind::String => Value::Text(raw.to_string()),
                    }
                };
                col.values.push(v);
            }
        }

        // pad short rows
        for col in &mut columns {
            while col.values.len() < n_rows {
                col.values.push(Value::Missing);
            }
        }

        Ok(DataTable { columns, n_rows })
    }

    pub fn from_sav_path(path: &Path) -> Result<Self, String> {
        let reader = SavReader::open(path).map_err(|e| e.to_string())?;
        let md = reader.metadata().clone();
        let df = reader.read().finish().map_err(|e| e.to_string())?;
        let n_rows = df.height();

        // helper accessors into the metadata_df (label/measure/alignment/display_width/decimals)
        fn md_any<'a>(df: &'a polars::prelude::DataFrame, name: &str, i: usize) -> Option<polars::prelude::AnyValue<'a>> {
            df.column(name).ok()?.get(i).ok()
        }
        fn md_str(df: &polars::prelude::DataFrame, name: &str, i: usize) -> Option<String> {
            match md_any(df, name, i) {
                Some(polars::prelude::AnyValue::String(s)) => Some(s.to_string()),
                Some(polars::prelude::AnyValue::StringOwned(s)) => Some(s.to_string()),
                _ => None,
            }
        }
        fn md_int(df: &polars::prelude::DataFrame, name: &str, i: usize) -> Option<usize> {
            match md_any(df, name, i) {
                Some(polars::prelude::AnyValue::Int64(v)) if v > 0 => Some(v as usize),
                Some(polars::prelude::AnyValue::Int32(v)) if v > 0 => Some(v as usize),
                Some(polars::prelude::AnyValue::UInt32(v)) => Some(v as usize),
                Some(polars::prelude::AnyValue::UInt64(v)) if v > 0 => Some(v as usize),
                Some(polars::prelude::AnyValue::Float64(v)) if v > 0.0 => Some(v as usize),
                _ => None,
            }
        }

        let mut columns = Vec::new();
        for name in df.get_column_names() {
            let series = df.column(name).map_err(|e| e.to_string())?.clone();
            let plan = md
                .variables
                .iter()
                .filter(|p| p.name == name.to_string() || p.short_name == name.to_string())
                .next()
                .cloned();
            let mi = columns.len();
            let kind = match series.dtype() {
                DataType::Float32 | DataType::Float64 => ColumnKind::Numeric,
                DataType::Int8 | DataType::Int16 | DataType::Int32 | DataType::Int64
                | DataType::UInt8 | DataType::UInt16 | DataType::UInt32 | DataType::UInt64 => {
                    ColumnKind::Numeric
                }
                _ => ColumnKind::String,
            };
            let mut values = Vec::with_capacity(n_rows);
            for i in 0..n_rows {
                let av = series.get(i).map_err(|e| e.to_string())?;
                let v = match av {
                    AnyValue::Null => Value::Missing,
                    AnyValue::Float32(x) => Value::Number(x as f64),
                    AnyValue::Float64(x) => Value::Number(x),
                    AnyValue::Int8(x) => Value::Number(x as f64),
                    AnyValue::Int16(x) => Value::Number(x as f64),
                    AnyValue::Int32(x) => Value::Number(x as f64),
                    AnyValue::Int64(x) => Value::Number(x as f64),
                    AnyValue::UInt8(x) => Value::Number(x as f64),
                    AnyValue::UInt16(x) => Value::Number(x as f64),
                    AnyValue::UInt32(x) => Value::Number(x as f64),
                    AnyValue::UInt64(x) => Value::Number(x as f64),
                    AnyValue::String(s) => Value::Text(s.to_string()),
                    AnyValue::StringOwned(s) => Value::Text(s.to_string()),
                    other => Value::Text(format!("{}", other)),
                };
                values.push(v);
            }

            // ---- Phase 1: real StatsBench variable metadata ----
            let mut meta = VariableMeta::default_for(kind);
            if let Some(p) = &plan {
                if let Some(l) = &md_str(&md.metadata_df, "label", mi) {
                    meta.label = l.clone();
                }
                if let Some(m) = md_str(&md.metadata_df, "measure", mi) {
                    meta.measure = match m.to_lowercase().as_str() {
                        "nominal" => Measure::Nominal,
                        "ordinal" => Measure::Ordinal,
                        "scale" => Measure::Scale,
                        _ => Measure::default_for(kind),
                    };
                }
                if let Some(a) = md_str(&md.metadata_df, "alignment", mi) {
                    meta.align = match a.to_lowercase().as_str() {
                        "left" => VarAlign::Left,
                        "center" => VarAlign::Center,
                        "right" => VarAlign::Right,
                        _ => VarAlign::default_for(kind),
                    };
                }
                if let Some(w) = md_int(&md.metadata_df, "display_width", mi) {
                    meta.columns_width = w.max(1);
                }
                if let Some(d) = md_int(&md.metadata_df, "format_decimals", mi) {
                    if kind == ColumnKind::Numeric {
                        meta.decimals = d.min(16);
                    }
                }
                // user-missing values (numeric)
                if kind == ColumnKind::Numeric {
                    meta.user_missing = p
                        .missing_doubles
                        .iter()
                        .copied()
                        .chain(p.missing_double_bits.iter().map(|b| f64::from_bits(*b)))
                        .collect();
                }
                // value labels set linked to this variable
                if let Some(set_name) = &p.value_label {
                    if let Some(set) = md.value_labels.iter().find(|s| &s.name == set_name) {
                        for (key, label) in &set.mapping {
                            // ValueLabelKey lives in a crate-private module; use its Debug
                            // rendering ("Double(<f64>)" / "Str(<string>)") to recover the value.
                            let dbg = format!("{:?}", key);
                            if dbg.starts_with("Double(") {
                                let inner = dbg
                                    .trim_start_matches("Double(")
                                    .trim_end_matches(')');
                                if let Ok(d) = inner.parse::<f64>() {
                                    meta.value_labels.push((d, label.clone()));
                                }
                            } else if dbg.starts_with("Str(") {
                                let inner = dbg
                                    .trim_start_matches("Str(")
                                    .trim_end_matches(')')
                                    .trim_matches('"');
                                meta.text_value_labels.push((inner.to_string(), label.clone()));
                            }
                        }
                    }
                }
            }

            columns.push(Column { name: name.to_string(), kind, values, meta });
        }
        Ok(DataTable { columns, n_rows })
    }

    pub fn is_missing(&self, col: usize, row: usize) -> bool {
        self.columns.get(col).map(|c| c.is_missing(row)).unwrap_or(false)
    }

    /// Rename a variable (StatsBench enforces non-empty names).
    pub fn rename_column(&mut self, col: usize, name: &str) -> bool {
        let name = name.trim();
        if name.is_empty() || self.columns.get(col).is_none() {
            return false;
        }
        self.columns[col].name = name.to_string();
        true
    }

    /// Add a variable at the end, the way Variable View's "New Variable"
    /// row does.
    ///
    /// The name is the first free `VAR00001`, `VAR00002`, ... so a new
    /// variable is valid before the user types anything. The values are
    /// system-missing for every existing case.
    pub fn add_variable(&mut self, kind: ColumnKind) -> usize {
        let mut n = self.columns.len() + 1;
        let name = loop {
            let candidate = format!("VAR{:05}", n);
            if !self
                .columns
                .iter()
                .any(|c| c.name.eq_ignore_ascii_case(&candidate))
            {
                break candidate;
            }
            n += 1;
        };
        let values = vec![Value::Missing; self.n_rows];
        self.columns.push(Column {
            name,
            kind,
            values,
            meta: VariableMeta::default_for(kind),
        });
        self.columns.len() - 1
    }

    /// The display name of a variable, looked up by machine name.
    pub fn display_name_for(&self, name: &str) -> String {
        self.column_index(name)
            .map(|ci| self.columns[ci].display_name().to_string())
            .unwrap_or_else(|| name.to_string())
    }

    /// Remove a variable and all of its values.
    ///
    /// Deleting a variable never changes the number of cases; the
    /// remaining columns keep their values row for row.
    pub fn remove_variable(&mut self, index: usize) -> bool {
        if index >= self.columns.len() {
            return false;
        }
        self.columns.remove(index);
        true
    }

    /// Add a case at the end of the table.
    ///
    /// StatsBench's "Add Cases" adds empty cases at the end; the new case
    /// holds the system-missing value in every column.
    pub fn add_case(&mut self) -> usize {
        for c in self.columns.iter_mut() {
            c.values.push(Value::Missing);
        }
        self.n_rows += 1;
        self.n_rows - 1
    }

    /// Add a number of cases at the end of the table.
    pub fn add_cases(&mut self, count: usize) -> usize {
        for _ in 0..count {
            self.add_case();
        }
        self.n_rows.saturating_sub(1)
    }

    /// Delete a case.
    ///
    /// The values of every column are shifted up, and the last one is
    /// dropped, so the columns stay the same length.
    pub fn delete_case(&mut self, row: usize) -> bool {
        if row >= self.n_rows {
            return false;
        }
        for c in self.columns.iter_mut() {
            if row < c.values.len() {
                c.values.remove(row);
            }
        }
        self.n_rows = self.n_rows.saturating_sub(1);
        true
    }

    /// Set a cell's value, parsing it the way the column's kind
    /// requires.
    ///
    /// A numeric column takes a number; an empty entry is the
    /// system-missing value. A string column takes the text as it is.
    pub fn set_cell(&mut self, col: usize, row: usize, text: &str) -> bool {
        let Some(c) = self.columns.get_mut(col) else {
            return false;
        };
        if row >= c.values.len() {
            return false;
        }
        let t = text.trim();
        let v = match c.kind {
            ColumnKind::Numeric => {
                if t.is_empty() {
                    Value::Missing
                } else {
                    match t.parse::<f64>() {
                        Ok(n) => Value::Number(n),
                        // A value that is not a number is not a
                        // missing value; it is rejected, as StatsBench
                        // rejects it.
                        Err(_) => return false,
                    }
                }
            }
            _ => {
                if t.is_empty() {
                    Value::Missing
                } else {
                    Value::Text(text.to_string())
                }
            }
        };
        c.values[row] = v;
        true
    }

    /// The value of a cell, for the clipboard.
    pub fn cell_value(&self, col: usize, row: usize) -> Option<&Value> {
        self.columns.get(col).and_then(|c| c.values.get(row))
    }

    /// Copy a rectangle of cells as tab-separated text, the way a
    /// spreadsheet does.
    pub fn copy_rectangle(&self, col0: usize, row0: usize, cols: usize, rows: usize) -> String {
        let mut out = String::new();
        for r in row0..(row0 + rows).min(self.n_rows) {
            for c in col0..(col0 + cols).min(self.columns.len()) {
                if c > col0 {
                    out.push('\t');
                }
                out.push_str(&self.cell_text(c, r));
            }
            if r + 1 < (row0 + rows).min(self.n_rows) {
                out.push('\n');
            }
        }
        out
    }

    /// Paste tab-separated text into the grid, starting at the given
    /// cell.
    ///
    /// The number of cases is grown if the pasted block is taller than
    /// the table, which is what StatsBench does.
    pub fn paste_text(&mut self, col0: usize, row0: usize, text: &str) -> usize {
        let mut pasted = 0;
        for (i, line) in text.lines().enumerate() {
            let row = row0 + i;
            if row >= self.n_rows {
                self.add_case();
            }
            for (j, cell) in line.split('\t').enumerate() {
                let col = col0 + j;
                if col >= self.columns.len() {
                    break;
                }
                // A cell that will not take the value is left alone,
                // and the paste carries on.
                self.set_cell(col, row, cell);
            }
            pasted += 1;
        }
        pasted
    }

    /// Define the user-missing values of a numeric variable.
    ///
    /// StatsBench's "Missing" dialog takes either a list of discrete values
    /// or a range, and the values that fall in them are treated as
    /// missing by every procedure.
    pub fn define_user_missing(
        &mut self,
        col: usize,
        discrete: &[f64],
        range: Option<(f64, f64)>,
    ) -> bool {
        let Some(c) = self.columns.get_mut(col) else {
            return false;
        };
        if c.kind != ColumnKind::Numeric {
            return false;
        }
        c.meta.user_missing = discrete.to_vec();
        c.meta.user_missing_range = range.map(|(lo, hi)| {
            if lo <= hi {
                (lo, hi)
            } else {
                (hi, lo)
            }
        });
        true
    }

    /// Replace a variable's value labels.
    ///
    /// Each entry is the value as the user typed it and the label shown
    /// for it. Numeric variables need a number in the value column; string
    /// variables take the text itself. Blank rows are ignored, so the
    /// dialog can start with an empty row.
    pub fn set_value_labels(
        &mut self,
        col: usize,
        rows: &[(String, String)],
    ) -> Result<usize, String> {
        let Some(c) = self.columns.get_mut(col) else {
            return Err("That variable no longer exists.".to_string());
        };
        let mut numeric: Vec<(f64, String)> = Vec::new();
        let mut text: Vec<(String, String)> = Vec::new();
        for (raw_value, raw_label) in rows {
            let value = raw_value.trim();
            let label = raw_label.trim();
            if value.is_empty() && label.is_empty() {
                continue;
            }
            if value.is_empty() {
                return Err("Every value label needs a value.".to_string());
            }
            if label.is_empty() {
                return Err(format!("Value '{}' needs a label.", value));
            }
            match c.kind {
                ColumnKind::Numeric => {
                    let n: f64 = value
                        .parse()
                        .map_err(|_| format!("'{}' is not a number.", value))?;
                    if numeric.iter().any(|(v, _)| *v == n) {
                        return Err(format!("Value '{}' is entered twice.", value));
                    }
                    numeric.push((n, label.to_string()));
                }
                ColumnKind::String => {
                    if text.iter().any(|(v, _)| v == value) {
                        return Err(format!("Value '{}' is entered twice.", value));
                    }
                    text.push((value.to_string(), label.to_string()));
                }
            }
        }
        let count = numeric.len() + text.len();
        c.meta.value_labels = numeric;
        c.meta.text_value_labels = text;
        Ok(count)
    }

    /// The values of a numeric variable that the user has declared
    /// missing.
    ///
    /// A list of more than two values is a set of discrete values; a
    /// pair is the ends of a range.
    pub fn is_user_missing(&self, col: usize, value: f64) -> bool {
        let Some(c) = self.columns.get(col) else {
            return false;
        };
        if c.meta.user_missing.iter().any(|&v| v == value) {
            return true;
        }
        if let Some((lo, hi)) = c.meta.user_missing_range {
            let (lo, hi) = (lo.min(hi), lo.max(hi));
            return value >= lo && value <= hi;
        }
        false
    }

    pub fn cell_text(&self, col: usize, row: usize) -> String {
        match self.columns.get(col) {
            Some(c) => match c.values.get(row) {
                Some(Value::Missing) => String::new(),
                Some(Value::Number(n)) => format!("{}", n),
                Some(Value::Text(s)) => s.clone(),
                None => String::new(),
            },
            None => String::new(),
        }
    }

    /// The text a cell shows in Data View.
    ///
    /// With the value labels turned on, a cell shows the label of its
    /// value rather than the value itself. The stored value does not
    /// change: the labels are a presentation of the dictionary, not a
    /// second copy of the data.
    pub fn cell_display_text(&self, col: usize, row: usize, show_labels: bool) -> String {
        let Some(c) = self.columns.get(col) else {
            return String::new();
        };
        if show_labels {
            let label = match c.values.get(row) {
                Some(Value::Number(n)) => c.value_label_for(*n).map(|s| s.to_string()),
                Some(Value::Text(s)) => c.text_value_label_for(s).map(|s| s.to_string()),
                _ => None,
            };
            if let Some(label) = label {
                return label;
            }
        }
        self.cell_text(col, row)
    }

    /// Write a value into a cell from what the reader typed.
    ///
    /// The reader may type the stored value or the label that is shown
    /// for it, because the grid shows the label when the value labels
    /// are on. Either way the cell keeps holding a value, never a
    /// label: the dictionary stays the single source of truth.
    pub fn set_cell_display(&mut self, col: usize, row: usize, text: &str) -> bool {
        let Some(c) = self.columns.get_mut(col) else {
            return false;
        };
        if row >= c.values.len() {
            return false;
        }
        let t = text.trim();
        let value = match c.kind {
            ColumnKind::Numeric => {
                if t.is_empty() {
                    Value::Missing
                } else if let Ok(n) = t.parse::<f64>() {
                    Value::Number(n)
                } else if let Some(v) = c.value_for_label(t) {
                    // The reader typed the label that is shown for a
                    // value: the value itself is stored.
                    Value::Number(v)
                } else {
                    return false;
                }
            }
            ColumnKind::String => {
                if t.is_empty() {
                    Value::Missing
                } else if let Some(v) = c.text_value_for_label(t) {
                    Value::Text(v)
                } else {
                    Value::Text(text.to_string())
                }
            }
        };
        c.values[row] = value;
        true
    }

    /// StatsBench "Sort Cases": stable reorder of all rows by one column.
    /// Numeric values sort as numbers, text after numbers, missing last.
    /// Column index by name (case-insensitive), for the syntax runner.
    pub fn column_index(&self, name: &str) -> Option<usize> {
        let lower = name.trim().to_lowercase();
        self.columns.iter().position(|c| c.name.to_lowercase() == lower)
    }

    /// A table that keeps only the listed row indices, in that order.
    pub fn with_rows(&self, rows: &[usize]) -> DataTable {
        let columns = self
            .columns
            .iter()
            .map(|c| Column {
                name: c.name.clone(),
                kind: c.kind,
                values: rows
                    .iter()
                    .map(|&i| c.values.get(i).cloned().unwrap_or(Value::Missing))
                    .collect(),
                meta: c.meta.clone(),
            })
            .collect();
        DataTable {
            columns,
            n_rows: rows.len(),
        }
    }

    /// Tables used for analysis: frequency-weighted, then split.
    ///
    /// A weight is rounded to a whole number of copies of the case.
    /// Non-positive and missing weights drop the case. Split File makes
    /// one table per combination of the split variables.
    pub fn analysis_groups(
        &self,
        weight: Option<&str>,
        split: &[String],
    ) -> Result<Vec<(String, DataTable)>, String> {
        let weighted = self.frequency_weighted(weight)?;
        if split.is_empty() {
            return Ok(vec![(String::new(), weighted)]);
        }
        let split_idx: Result<Vec<usize>, String> = split
            .iter()
            .map(|n| {
                weighted
                    .column_index(n)
                    .ok_or_else(|| format!("SPLIT FILE: unknown variable '{}'.", n))
            })
            .collect();
        let split_idx = split_idx?;
        let mut groups: std::collections::BTreeMap<String, Vec<usize>> =
            std::collections::BTreeMap::new();
        for row in 0..weighted.n_rows {
            let key = split_idx
                .iter()
                .map(|&ci| {
                    weighted.columns[ci]
                        .values
                        .get(row)
                        .map(|v| match v {
                            Value::Missing => ".".to_string(),
                            Value::Number(n) => format!("{}", n),
                            Value::Text(s) => s.clone(),
                        })
                        .unwrap_or_default()
                })
                .collect::<Vec<_>>()
                .join(" | ");
            groups.entry(key).or_default().push(row);
        }
        Ok(groups
            .into_iter()
            .map(|(k, rows)| {
                let label = split
                    .iter()
                    .zip(k.split(" | "))
                    .map(|(n, v)| format!("{}={}", n, v))
                    .collect::<Vec<_>>()
                    .join(" ");
                (label, weighted.with_rows(&rows))
            })
            .collect())
    }

    fn frequency_weighted(&self, weight: Option<&str>) -> Result<DataTable, String> {
        let Some(name) = weight else {
            return Ok(self.clone());
        };
        let wi = self
            .column_index(name)
            .ok_or_else(|| format!("WEIGHT: unknown variable '{}'.", name))?;
        if self.columns[wi].kind != ColumnKind::Numeric {
            return Err("WEIGHT: the weight variable must be numeric.".to_string());
        }
        let mut rows = Vec::new();
        for row in 0..self.n_rows {
            if self.columns[wi].is_missing(row) {
                continue;
            }
            let copies = match self.columns[wi].values[row] {
                Value::Number(n) if n.is_finite() && n > 0.0 => n.round() as i64,
                _ => 0,
            };
            for _ in 0..copies {
                rows.push(row);
            }
        }
        Ok(self.with_rows(&rows))
    }

    /// Load a CSV or `.sav` file, chosen by the extension.
    pub fn from_path(path: &Path) -> Result<Self, String> {
        let sav = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("sav"))
            .unwrap_or(false);
        if sav {
            Self::from_sav_path(path)
        } else {
            Self::from_csv_path(path)
        }
    }

    pub fn sort_rows(&mut self, col: usize, ascending: bool) {
        if col >= self.columns.len() || self.n_rows == 0 {
            return;
        }
        let mut idx: Vec<usize> = (0..self.n_rows).collect();
        let vals = &self.columns[col].values;
        let class_of = |v: &Value| match v {
            Value::Number(_) => 0u8,
            Value::Text(_) => 1u8,
            Value::Missing => 2u8,
        };
        idx.sort_by(|&a, &b| {
            let (va, vb) = (&vals[a], &vals[b]);
            // class order is fixed: numbers < text < missing (missing always last)
            let mut ord = class_of(va).cmp(&class_of(vb));
            if class_of(va) == class_of(vb) {
                let inner = match (va, vb) {
                    (Value::Number(x), Value::Number(y)) => x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal),
                    (Value::Text(x), Value::Text(y)) => x.cmp(y),
                    _ => std::cmp::Ordering::Equal,
                };
                ord = ord.then(if ascending { inner } else { inner.reverse() });
            }
            ord
        });
        for c in self.columns.iter_mut() {
            let old = c.values.clone();
            c.values = idx.iter().map(|&i| old[i].clone()).collect();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn csv_load_infers_types() {
        let path = std::env::temp_dir().join("sc_test.csv");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, "id,name,age,score").unwrap();
        writeln!(f, "1,Alice,25,88.5").unwrap();
        writeln!(f, "2,Bob,30,92.0").unwrap();
        writeln!(f, "3,Charlie,,75.3").unwrap();
        let t = DataTable::from_csv_path(&path).unwrap();
        assert_eq!(t.n_rows, 3);
        assert_eq!(t.columns.len(), 4);
        assert_eq!(t.columns[0].kind, ColumnKind::Numeric);
        assert_eq!(t.columns[1].kind, ColumnKind::String);
        assert_eq!(t.columns[2].kind, ColumnKind::Numeric);
        assert!(matches!(t.columns[2].values[2], Value::Missing));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn sort_rows_orders_cases() {
        let mut t = DataTable {
            n_rows: 4,
            columns: vec![
                Column {
                    name: "name".into(),
                    kind: ColumnKind::String,
                    values: vec![
                        Value::Text("d".into()),
                        Value::Text("b".into()),
                        Value::Text("a".into()),
                        Value::Text("c".into()),
                    ],
                    meta: VariableMeta::default_for(ColumnKind::String),
                },
                Column {
                    name: "age".into(),
                    kind: ColumnKind::Numeric,
                    values: vec![
                        Value::Number(30.0),
                        Value::Number(25.0),
                        Value::Missing,
                        Value::Number(28.0),
                    ],
                    meta: VariableMeta::default_for(ColumnKind::Numeric),
                },
            ],
        };

        // ascending by name
        t.sort_rows(0, true);
        let names: Vec<&str> = t.columns[0]
            .values
            .iter()
            .map(|v| match v {
                Value::Text(s) => s.as_str(),
                _ => "?",
            })
            .collect();
        assert_eq!(names, vec!["a", "b", "c", "d"]);
        // companion column must follow the permutation (a had the missing age)
        assert_eq!(t.columns[1].values[0], Value::Missing);      // a -> missing
        assert_eq!(t.columns[1].values[1], Value::Number(25.0)); // b -> 25
        assert_eq!(t.columns[1].values[2], Value::Number(28.0)); // c -> 28
        assert_eq!(t.columns[1].values[3], Value::Number(30.0)); // d -> 30

        // descending by age: 30,28,25 numeric first, missing last
        t.sort_rows(1, false);
        let names2: Vec<&str> = t.columns[0]
            .values
            .iter()
            .map(|v| match v {
                Value::Text(s) => s.as_str(),
                _ => "?",
            })
            .collect();
        assert_eq!(names2, vec!["d", "c", "b", "a"]);
    }

    #[test]
    fn variable_meta_defaults_follow_measure_conventions() {
        let path = std::env::temp_dir().join("sc_meta_test.csv");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, "id,name").unwrap();
        writeln!(f, "1,Alice").unwrap();
        writeln!(f, "2,Bob").unwrap();
        let t = DataTable::from_csv_path(&path).unwrap();

        let id = &t.columns[0]; // numeric
        assert_eq!(id.meta.measure, Measure::Scale);
        assert_eq!(id.meta.align, VarAlign::Right);
        assert_eq!(id.meta.decimals, 2);
        assert_eq!(id.meta.columns_width, 8);
        assert_eq!(id.meta.role, "Input");
        assert!(id.meta.label.is_empty());

        let name = &t.columns[1]; // string
        assert_eq!(name.meta.measure, Measure::Nominal);
        assert_eq!(name.meta.align, VarAlign::Left);
        assert_eq!(name.meta.decimals, 0);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn user_missing_marks_cases() {
        let path = std::env::temp_dir().join("sc_missing_test.csv");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, "score").unwrap();
        writeln!(f, "1").unwrap();
        writeln!(f, "99").unwrap(); // will be declared user-missing
        writeln!(f, "\"\"").unwrap(); // quoted empty -> system-missing
        let mut t = crate::data::DataTable::from_csv_path(&path).unwrap();
        t.columns[0].meta.user_missing.push(99.0);

        assert!(!t.is_missing(0, 0)); // 1 is valid
        assert!(t.is_missing(0, 1)); // 99 user-missing
        assert!(t.is_missing(0, 2)); // empty system-missing
        // user-missing still displays its raw value in Data View
        assert_eq!(t.cell_text(0, 1), "99");
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn rename_rejects_empty_and_updates() {
        let path = std::env::temp_dir().join("sc_rename_test.csv");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, "a,b").unwrap();
        writeln!(f, "1,2").unwrap();
        let mut t = DataTable::from_csv_path(&path).unwrap();
        assert!(t.rename_column(0, "age"));
        assert_eq!(t.columns[0].name, "age");
        assert!(!t.rename_column(0, "   "));
        assert_eq!(t.columns[0].name, "age");
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn set_kind_converts_values() {
        let path = std::env::temp_dir().join("sc_kind_test.csv");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, "num,alpha").unwrap();
        writeln!(f, "1,42").unwrap();
        writeln!(f, "2,").unwrap();
        let mut t = DataTable::from_csv_path(&path).unwrap();

        // string -> numeric: parseable becomes a number, blanks stay missing
        t.columns[1].set_kind(ColumnKind::Numeric);
        assert_eq!(t.columns[1].kind, ColumnKind::Numeric);
        assert!(matches!(t.columns[1].values[0], Value::Number(x) if (x - 42.0).abs() < 1e-9));
        assert!(matches!(t.columns[1].values[1], Value::Missing));

        // numeric -> string: formats the number as text
        t.columns[0].set_kind(ColumnKind::String);
        assert!(matches!(t.columns[0].values[0], Value::Text(_)));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn the_sav_writer_round_trips_through_the_reader() {
        let csv = std::env::temp_dir().join("sc_sav_rt.csv");
        std::fs::write(
            &csv,
            "id,name,age,score\n1,Alice,25,88.5\n2,Bob,30,92.0\n3,Carol,41,77.25\n",
        )
        .unwrap();
        let t = DataTable::from_csv_path(&csv).unwrap();
        assert_eq!(t.n_rows, 3);
        assert_eq!(t.columns.len(), 4);

        let sav = std::env::temp_dir().join("sc_sav_rt.sav");
        write_sav(&t, &sav).expect("write");
        assert!(sav.exists());

        // Read it back with the same reader the app uses.
        let back = match DataTable::from_sav_path(&sav) {
            Ok(t) => t,
            Err(e) => panic!("read back: {}", e),
        };
        assert_eq!(back.n_rows, 3, "rows: {:?}", back.n_rows);
        assert_eq!(back.columns.len(), 4, "columns: {}", back.columns.len());

        // The numeric values must survive exactly.
        let score = back.column_index("score").expect("the score column");
        match back.columns[score].values[0] {
            Value::Number(v) => assert!((v - 88.5).abs() < 1e-9, "score was {}", v),
            _ => panic!("the score column is not numeric"),
        }
        let age = back.column_index("age").expect("the age column");
        match back.columns[age].values[2] {
            Value::Number(v) => assert!((v - 41.0).abs() < 1e-9, "age was {}", v),
            _ => panic!("the age column is not numeric"),
        }
        let _ = std::fs::remove_file(&sav);
    }
    /// A small table to edit: two numeric columns and a string one.
    fn editable() -> DataTable {
        DataTable {
            n_rows: 3,
            columns: vec![
                Column {
                    name: "x".into(),
                    kind: ColumnKind::Numeric,
                    values: vec![Value::Number(1.0), Value::Number(2.0), Value::Number(3.0)],
                    meta: VariableMeta::default_for(ColumnKind::Numeric),
                },
                Column {
                    name: "g".into(),
                    kind: ColumnKind::String,
                    values: vec![Value::Text("A".into()), Value::Text("B".into()), Value::Text("A".into())],
                    meta: VariableMeta::default_for(ColumnKind::String),
                },
            ],
        }
    }

    #[test]
    fn a_case_can_be_added_and_deleted() {
        let mut t = editable();
        let row = t.add_case();
        assert_eq!(row, 3);
        assert_eq!(t.n_rows, 4);
        // The new case is missing in every column.
        assert!(matches!(t.columns[0].values[3], Value::Missing));
        assert!(matches!(t.columns[1].values[3], Value::Missing));
        // Deleting it takes the table back to three cases.
        assert!(t.delete_case(3));
        assert_eq!(t.n_rows, 3);
        assert!(!t.delete_case(99));
    }

    #[test]
    fn deleting_a_case_shifts_the_others_up() {
        let mut t = editable();
        t.delete_case(0);
        assert_eq!(t.n_rows, 2);
        // The first case is now the old second one.
        assert_eq!(t.cell_text(0, 0), "2");
        assert_eq!(t.columns[0].values.len(), 2);
    }

    #[test]
    fn a_variable_can_be_removed_without_touching_the_cases() {
        let mut t = editable();
        assert!(t.remove_variable(0));
        assert_eq!(t.columns.len(), 1);
        assert_eq!(t.columns[0].name, "g");
        assert_eq!(t.n_rows, 3);
        assert_eq!(t.cell_text(0, 0), "A");
        // The old index is gone, and an out-of-range delete is a no-op.
        assert!(!t.remove_variable(1));
        assert_eq!(t.columns.len(), 1);
        // Removing the final variable leaves an empty table with its rows.
        assert!(t.remove_variable(0));
        assert!(t.columns.is_empty());
        assert_eq!(t.n_rows, 3);
    }

    #[test]
    fn a_cell_takes_the_value_of_its_column_kind() {
        let mut t = editable();
        assert!(t.set_cell(0, 0, "42"));
        assert_eq!(t.cell_text(0, 0), "42");
        // An empty entry is the system-missing value.
        assert!(t.set_cell(0, 0, ""));
        assert!(matches!(t.columns[0].values[0], Value::Missing));
        // A string column takes the text as it is.
        assert!(t.set_cell(1, 0, "hello"));
        assert_eq!(t.cell_text(1, 0), "hello");
        // A value that is not a number is rejected in a numeric column.
        assert!(!t.set_cell(0, 0, "abc"));
    }

    #[test]
    fn text_is_pasted_into_the_grid() {
        let mut t = editable();
        // A block of values, as a spreadsheet would give.
        let pasted = t.paste_text(0, 0, "10\n20\n30");
        assert_eq!(pasted, 3);
        assert_eq!(t.cell_text(0, 0), "10");
        assert_eq!(t.cell_text(0, 2), "30");
        // A paste that is taller than the table grows it.
        let pasted = t.paste_text(0, 2, "40\n50\n60\n70");
        assert_eq!(pasted, 4);
        assert_eq!(t.n_rows, 6);
        assert_eq!(t.cell_text(0, 5), "70");
    }

    #[test]
    fn a_tab_separated_block_pastes_across_the_columns() {
        let mut t = editable();
        t.paste_text(0, 0, "7\tQ");
        assert_eq!(t.cell_text(0, 0), "7");
        assert_eq!(t.cell_text(1, 0), "Q");
    }

    #[test]
    fn value_labels_are_replaced_and_validated() {
        let mut t = editable();
        // Numeric labels.
        let rows = vec![
            ("1".to_string(), "Low".to_string()),
            ("2".to_string(), "High".to_string()),
        ];
        assert_eq!(t.set_value_labels(0, &rows).unwrap(), 2);
        assert_eq!(t.columns[0].value_label_for(1.0), Some("Low"));
        assert_eq!(t.columns[0].value_label_for(2.0), Some("High"));
        // Replacing the set drops the labels that are no longer listed.
        assert_eq!(
            t.set_value_labels(0, &[("2".to_string(), "Upper".to_string())])
                .unwrap(),
            1
        );
        assert_eq!(t.columns[0].value_label_for(1.0), None);
        assert_eq!(t.columns[0].value_label_for(2.0), Some("Upper"));
        // A non-number and a duplicate are rejected, and the old set stays.
        assert!(t.set_value_labels(0, &[("x".to_string(), "Bad".to_string())]).is_err());
        assert!(t
            .set_value_labels(
                0,
                &[
                    ("2".to_string(), "A".to_string()),
                    ("2".to_string(), "B".to_string())
                ]
            )
            .is_err());
        assert_eq!(t.columns[0].value_label_for(2.0), Some("Upper"));
        // A blank row is ignored, and an empty value with a label is an error.
        assert_eq!(
            t.set_value_labels(
                0,
                &[("".to_string(), "".to_string()), ("3".to_string(), "Top".to_string())]
            )
            .unwrap(),
            1
        );
        assert!(t.set_value_labels(0, &[("".to_string(), "Orphan".to_string())]).is_err());
        // String variables use their text values.
        let rows = vec![("A".to_string(), "Alpha".to_string())];
        assert_eq!(t.set_value_labels(1, &rows).unwrap(), 1);
        assert_eq!(t.columns[1].meta.text_value_labels.len(), 1);
        assert_eq!(t.columns[1].meta.text_value_labels[0].0, "A");
        // Clearing every label is allowed.
        assert_eq!(t.set_value_labels(0, &[]).unwrap(), 0);
        assert!(t.columns[0].meta.value_labels.is_empty());
    }

    #[test]
    fn the_missing_values_of_a_variable_are_declared() {
        // The column holds 1.0, 2.0 and 3.0, in rows 0, 1 and 2.
        let mut t = editable();
        // A range: the two ends of it.
        assert!(t.define_user_missing(0, &[], Some((2.0, 3.0))));
        assert!(t.is_user_missing(0, 1.0) == false, "1 is below the range");
        assert!(t.is_user_missing(0, 2.0), "2 is in the range");
        assert!(t.is_user_missing(0, 3.0), "3 is in the range");
        assert!(!t.is_user_missing(0, 4.0), "4 is above the range");
        // The rows of the column are then missing, so the procedures
        // leave them out.
        assert!(!t.is_missing(0, 0), "the first case is not missing");
        assert!(t.is_missing(0, 1), "the second case is in the range");
        assert!(t.is_missing(0, 2), "the third case is in the range");
        // Discrete values.
        assert!(t.define_user_missing(0, &[1.0, 3.0], None));
        assert!(t.is_user_missing(0, 1.0));
        assert!(!t.is_user_missing(0, 2.0));
        assert!(t.is_user_missing(0, 3.0));
        // A string variable cannot have missing values declared.
        assert!(!t.define_user_missing(1, &[1.0], None));
    }

    #[test]
    fn a_rectangle_of_cells_is_copied_as_text() {
        let mut t = editable();
        let text = t.copy_rectangle(0, 0, 2, 2);
        assert_eq!(text, "1\tA\n2\tB");
    }

    #[test]
    fn new_variables_get_a_unique_name_and_missing_cells() {
        let mut t = editable();
        let a = t.add_variable(ColumnKind::Numeric);
        let b = t.add_variable(ColumnKind::String);
        assert_eq!(t.columns[a].name, "VAR00003");
        assert_eq!(t.columns[b].name, "VAR00004");
        assert_eq!(t.columns[a].values.len(), t.n_rows);
        assert!(matches!(t.columns[a].values[0], Value::Missing));
        // A name the user has already typed is not offered again.
        t.rename_column(a, "VAR00005");
        let c = t.add_variable(ColumnKind::Numeric);
        assert_ne!(t.columns[c].name, "VAR00005");
        assert_eq!(t.columns[c].name, "VAR00006");
    }

    #[test]
    fn a_variable_shows_its_label_when_it_has_one() {
        let mut t = editable();
        assert_eq!(t.columns[0].display_name(), "x");
        t.columns[0].meta.label = "Age in years".to_string();
        assert_eq!(t.columns[0].display_name(), "Age in years");
        assert_eq!(t.display_name_for("x"), "Age in years");
        assert_eq!(t.display_name_for("not-a-variable"), "not-a-variable");
    }

}

/// ---------------------------------------------------------------------------
/// Writing StatsBench .sav files.
/// ---------------------------------------------------------------------------




/// Write the table to an StatsBench .sav file.
///
/// The `.sav` writer in `polars-readstat-rs` handles the record layout,
/// the compression and the metadata, so the table is converted to a
/// Polars frame and handed to it. The variable labels, the value labels,
/// the measures and the display widths are carried across, so a file
/// written here opens in StatsBench with the same metadata it had here.
pub fn write_sav(table: &DataTable, path: &std::path::Path) -> Result<(), String> {
    use polars::prelude::*;
    use polars_readstat_rs::{
        SpssMeasure as SavMeasure, SpssValueLabelKey as SavValueLabelKey,
        SpssValueLabelMap as SavValueLabelMap, SpssValueLabels as SavValueLabels,
        SpssVariableDisplayWidths as SavVariableDisplayWidths,
        SpssVariableLabels as SavVariableLabels, SpssVariableMeasures as SavVariableMeasures,
        SpssWriteColumn as SavWriteColumn, SpssWriteSchema as SavWriteSchema,
        SpssWriter as SavWriter,
    };

    // The schema: the columns of the frame.
    let mut schema_columns: Vec<SavWriteColumn> = Vec::new();
    for col in &table.columns {
        let dtype = match col.kind {
            ColumnKind::Numeric => DataType::Float64,
            ColumnKind::String => DataType::String,
        };
        schema_columns.push(SavWriteColumn {
            name: col.name.clone(),
            dtype,
            string_width_bytes: if col.kind == ColumnKind::String {
                Some(8)
            } else {
                None
            },
        });
    }

    // The data, as a column of series.
    let mut series: Vec<Series> = Vec::new();
    for col in &table.columns {
        let name = col.name.clone().into();
        let s = match col.kind {
            ColumnKind::Numeric => {
                let values: Vec<Option<f64>> = (0..table.n_rows)
                    .map(|row| match col.values.get(row) {
                        Some(Value::Number(n)) => Some(*n),
                        _ => None,
                    })
                    .collect();
                Series::new(name, &values)
            }
            ColumnKind::String => {
                let values: Vec<Option<String>> = (0..table.n_rows)
                    .map(|row| match col.values.get(row) {
                        Some(Value::Text(s)) => Some(s.clone()),
                        _ => None,
                    })
                    .collect();
                Series::new(name, &values)
            }
        };
        series.push(s);
    }
    // Polars 0.53 builds a frame from a height and a list of columns.
    let columns: Vec<Column> = series.into_iter().map(Column::from).collect();
    let df = DataFrame::new(table.n_rows, columns).map_err(|e| format!("frame: {}", e))?;

    // The metadata: the variable labels, the value labels and the
    // measures. The maps are keyed by the variable's name.
    let mut variable_labels = SavVariableLabels::default();
    let mut value_labels = SavValueLabels::default();
    let mut measures = SavVariableMeasures::default();
    let mut widths = SavVariableDisplayWidths::default();
    for col in table.columns.iter() {
        if !col.meta.label.is_empty() {
            variable_labels.insert(col.name.clone(), col.meta.label.clone());
        }
        if !col.meta.value_labels.is_empty() {
            let mut map = SavValueLabelMap::new();
            for (value, label) in &col.meta.value_labels {
                map.insert(SavValueLabelKey::from(*value), label.clone());
            }
            value_labels.insert(col.name.clone(), map);
        }
        measures.insert(
            col.name.clone(),
            match col.meta.measure {
                crate::data::Measure::Nominal => SavMeasure::Nominal,
                crate::data::Measure::Ordinal => SavMeasure::Ordinal,
                crate::data::Measure::Scale => SavMeasure::Scale,
            },
        );
        widths.insert(col.name.clone(), col.meta.columns_width.max(1) as i32);
    }

    let schema = SavWriteSchema {
        columns: schema_columns,
        row_count: Some(table.n_rows),
        value_labels: Some(value_labels),
        variable_labels: Some(variable_labels),
    };

    let writer = SavWriter::new(path)
        .with_schema(schema)
        .with_variable_measures(measures)
        .with_variable_display_widths(widths);
    writer
        .write_df(&df)
        .map_err(|e| format!("write {}: {}", path.display(), e))




}
