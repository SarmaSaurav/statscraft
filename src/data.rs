use polars::prelude::*;
use polars_readstat_rs::{scan_sav, ScanOptions};
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

#[derive(Debug, Clone)]
pub struct Column {
    pub name: String,
    pub kind: ColumnKind,
    pub values: Vec<Value>,
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
            rows.push(rec.iter().map(|s| s.to_string()).collect());
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
        let lf = scan_sav(path, ScanOptions::default()).map_err(|e| e.to_string())?;
        let df = lf.collect().map_err(|e| e.to_string())?;
        let n_rows = df.height();
        let mut columns = Vec::new();
        for name in df.get_column_names() {
            let series = df.column(name).map_err(|e| e.to_string())?.clone();
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
            columns.push(Column {
                name: name.to_string(),
                kind,
                values,
            });
        }
        Ok(DataTable { columns, n_rows })
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
}
