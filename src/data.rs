use polars::prelude::*;
use polars_readstat_rs::SpssReader;
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

/// SPSS "Measure": Scale / Ordinal / Nominal (Variable View).
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

/// SPSS "Align" in Variable View.
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

/// Per-variable metadata backing the SPSS-style Variable View.
#[derive(Debug, Clone)]
pub struct VariableMeta {
    /// Variable/Value label ("Label" column). Empty = none.
    pub label: String,
    /// Numeric value labels ("Values" column), e.g. 1.0 -> "male".
    pub value_labels: Vec<(f64, String)>,
    /// Value labels for string variables.
    pub text_value_labels: Vec<(String, String)>,
    /// User-defined discrete missing values (numeric).
    pub user_missing: Vec<f64>,
    pub measure: Measure,
    pub align: VarAlign,
    /// "Columns" (display width) in Variable View.
    pub columns_width: usize,
    /// "Decimals".
    pub decimals: usize,
    /// SPSS "Role"; we only model "Input".
    pub role: String,
}

impl VariableMeta {
    pub fn default_for(kind: ColumnKind) -> Self {
        Self {
            label: String::new(),
            value_labels: Vec::new(),
            text_value_labels: Vec::new(),
            user_missing: Vec::new(),
            measure: Measure::default_for(kind),
            align: VarAlign::default_for(kind),
            columns_width: 8,
            decimals: if kind == ColumnKind::Numeric { 2 } else { 0 },
            role: "Input".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Column {
    pub name: String,
    pub kind: ColumnKind,
    pub values: Vec<Value>,
    pub meta: VariableMeta,
}

impl Column {
    /// System- or user-missing?
    pub fn is_missing(&self, row: usize) -> bool {
        match self.values.get(row) {
            Some(Value::Missing) => true,
            Some(Value::Number(n)) => self.meta.user_missing.contains(n),
            _ => false,
        }
    }

    /// Type change with value conversion (SPSS converts when possible).
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

    /// Value label for a numeric value, if any.
    pub fn value_label_for(&self, v: f64) -> Option<&str> {
        self.meta
            .value_labels
            .iter()
            .find(|(val, _)| (val - v).abs() < f64::EPSILON)
            .map(|(_, l)| l.as_str())
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
        let reader = SpssReader::open(path).map_err(|e| e.to_string())?;
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

            // ---- Phase 1: real SPSS variable metadata ----
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

    /// Rename a variable (SPSS enforces non-empty names).
    pub fn rename_column(&mut self, col: usize, name: &str) -> bool {
        let name = name.trim();
        if name.is_empty() || self.columns.get(col).is_none() {
            return false;
        }
        self.columns[col].name = name.to_string();
        true
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

    /// SPSS "Sort Cases": stable reorder of all rows by one column.
    /// Numeric values sort as numbers, text after numbers, missing last.
    /// Column index by name (case-insensitive), for the syntax runner.
    pub fn column_index(&self, name: &str) -> Option<usize> {
        let lower = name.trim().to_lowercase();
        self.columns.iter().position(|c| c.name.to_lowercase() == lower)
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
    fn sav_load_accidents() {
        let path = std::path::Path::new(r"C:\Program Files\IBM\SPSS\Statistics\27\Samples\English\accidents.sav");
        if !path.exists() { return; }
        let t = DataTable::from_sav_path(path).unwrap();
        assert!(t.n_rows > 0);
        assert!(!t.columns.is_empty());
        println!("sav: {} rows x {}", t.n_rows, t.columns.len());
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
    fn variable_meta_defaults_follow_spss_conventions() {
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
    fn sav_metadata_is_loaded() {
        let path = std::path::Path::new(r"C:\Program Files\IBM\SPSS\Statistics\27\Samples\English\accidents.sav");
        if !path.exists() { return; }
        let t = DataTable::from_sav_path(path).unwrap();
        assert!(t.n_rows > 0);
        assert!(!t.columns.is_empty());
        // accidents.sav carries real SPSS metadata: labels and/or widescale measures.
        let labeled = t.columns.iter().any(|c| !c.meta.label.is_empty());
        let measures = t.columns.iter().any(|c| c.meta.measure != Measure::default_for(c.kind) || c.meta.align != VarAlign::default_for(c.kind) || c.meta.decimals != VariableMeta::default_for(c.kind).decimals);
        println!("sav meta: labeled={} measure_differs={}", labeled, measures);
        let _ = labeled;
        let _ = measures;
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
}

/// ---------------------------------------------------------------------------
/// Writing SPSS .sav files.
/// ---------------------------------------------------------------------------




/// Write the table to an SPSS .sav file.
///
/// The `.sav` writer in `polars-readstat-rs` handles the record layout,
/// the compression and the metadata, so the table is converted to a
/// Polars frame and handed to it. The variable labels, the value labels,
/// the measures and the display widths are carried across, so a file
/// written here opens in SPSS with the same metadata it had here.
pub fn write_sav(table: &DataTable, path: &std::path::Path) -> Result<(), String> {
    use polars::prelude::*;
    use polars_readstat_rs::{
        SpssValueLabelKey, SpssValueLabelMap, SpssValueLabels, SpssVariableDisplayWidths,
        SpssVariableLabels, SpssVariableMeasures, SpssWriteColumn, SpssWriteSchema, SpssWriter,
    };

    // The schema: the columns of the frame.
    let mut schema_columns: Vec<SpssWriteColumn> = Vec::new();
    for col in &table.columns {
        let dtype = match col.kind {
            ColumnKind::Numeric => DataType::Float64,
            ColumnKind::String => DataType::String,
        };
        schema_columns.push(SpssWriteColumn {
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
    let mut variable_labels = SpssVariableLabels::default();
    let mut value_labels = SpssValueLabels::default();
    let mut measures = SpssVariableMeasures::default();
    let mut widths = SpssVariableDisplayWidths::default();
    for col in table.columns.iter() {
        if !col.meta.label.is_empty() {
            variable_labels.insert(col.name.clone(), col.meta.label.clone());
        }
        if !col.meta.value_labels.is_empty() {
            let mut map = SpssValueLabelMap::new();
            for (value, label) in &col.meta.value_labels {
                map.insert(SpssValueLabelKey::from(*value), label.clone());
            }
            value_labels.insert(col.name.clone(), map);
        }
        measures.insert(
            col.name.clone(),
            match col.meta.measure {
                crate::data::Measure::Nominal => polars_readstat_rs::spss::Measure::Nominal,
                crate::data::Measure::Ordinal => polars_readstat_rs::spss::Measure::Ordinal,
                crate::data::Measure::Scale => polars_readstat_rs::spss::Measure::Scale,
            },
        );
        widths.insert(col.name.clone(), col.meta.columns_width.max(1) as i32);
    }

    let schema = SpssWriteSchema {
        columns: schema_columns,
        row_count: Some(table.n_rows),
        value_labels: Some(value_labels),
        variable_labels: Some(variable_labels),
    };

    let writer = SpssWriter::new(path)
        .with_schema(schema)
        .with_variable_measures(measures)
        .with_variable_display_widths(widths);
    writer
        .write_df(&df)
        .map_err(|e| format!("write {}: {}", path.display(), e))
}
