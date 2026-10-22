//! The pivot table: the structure behind StatsBench's output.
//!
//! An output table in StatsBench is not a grid of strings. It is a *pivot
//! table*: a set of dimensions — the variables down the side and the
//! variables across the top — and a value in each cell, addressed by
//! the combination of levels the dimensions are at.
//!
//! That structure is what makes the StatsBench operations possible. A table
//! can be *pivoted*, which means exchanging the row and the column
//! dimensions and transposing the cells along with them. A table can
//! be *drilled into*, which means showing the sub-table for one cell.
//! And the export carries the structure rather than a flattened grid.
//!
//! The crosstabulation is the clearest example. Its rows are the
//! levels of one variable, its columns the levels of another, and the
//! counts sit in the cells — but the same table also holds the row
//! percentages, the column percentages and the expected counts, as
//! separate *measures* of the same cells.

use std::collections::BTreeMap;

/// One dimension of a pivot table: a variable, and the levels it takes.
///
/// The levels are in the order they should appear, and the "Total"
/// level, when the table has one, is simply the last of them.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PivotDimension {
    /// The variable name, shown above the levels.
    pub name: String,
    /// The level labels.
    pub levels: Vec<String>,
}

impl PivotDimension {
    /// A dimension with the given name and levels.
    pub fn new(name: impl Into<String>, levels: Vec<String>) -> Self {
        Self {
            name: name.into(),
            levels,
        }
    }

    /// How many leaf columns the levels of this dimension span.
    fn span(&self) -> usize {
        self.levels.len().max(1)
    }
}

/// A pivot table.
///
/// The cells are keyed by a row key and a column key, each of which is
/// the list of level indices the dimensions are at. The key order is
/// the dimension order, and the enumeration order of the keys is the
/// spreadsheet one: the last dimension varies fastest.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PivotTable {
    /// The title of the table.
    pub title: String,
    /// The dimensions down the side, outermost first.
    pub row_dims: Vec<PivotDimension>,
    /// The dimensions across the top, outermost first.
    pub col_dims: Vec<PivotDimension>,
    /// The measures of a cell — "Count", "Row %" and so on.
    pub measures: Vec<String>,
    /// The cells, keyed by the row key and the column key. Each holds
    /// one value per measure.
    pub cells: BTreeMap<(Vec<usize>, Vec<usize>), Vec<String>>,
    /// The footnote.
    pub note: String,
    /// Which measure the reader is looking at.
    ///
    /// A pivot holds several measures of the same cells — the counts,
    /// the expected counts, the percentages — and only one of them is
    /// shown at a time, the way StatsBench shows them as separate tables
    /// under the crosstabulation.
    pub measure_index: usize,
}

impl PivotTable {
    /// A table with the given dimensions and measures.
    pub fn new(
        title: impl Into<String>,
        row_dims: Vec<PivotDimension>,
        col_dims: Vec<PivotDimension>,
        measures: Vec<String>,
    ) -> Self {
        Self {
            title: title.into(),
            row_dims,
            col_dims,
            measures,
            cells: BTreeMap::new(),
            note: String::new(),
            measure_index: 0,
        }
    }

    /// How many leaf rows the table has.
    pub fn row_count(&self) -> usize {
        self.row_dims.iter().map(|d| d.span()).product::<usize>().max(1)
    }

    /// How many leaf columns the table has.
    pub fn col_count(&self) -> usize {
        self.col_dims.iter().map(|d| d.span()).product::<usize>().max(1)
    }

    /// The row keys, in the order the rows are shown.
    pub fn row_keys(&self) -> Vec<Vec<usize>> {
        keys(&self.row_dims)
    }

    /// The column keys, in the order the columns are shown.
    pub fn col_keys(&self) -> Vec<Vec<usize>> {
        keys(&self.col_dims)
    }

    /// Set a cell. The values are one per measure.
    pub fn set(&mut self, row_key: &[usize], col_key: &[usize], values: Vec<String>) {
        self.cells
            .insert((row_key.to_vec(), col_key.to_vec()), values);
    }

    /// The value of a cell, if it has one.
    pub fn get(&self, row_key: &[usize], col_key: &[usize]) -> Option<&Vec<String>> {
        self.cells.get(&(row_key.to_vec(), col_key.to_vec()))
    }

    /// The value of a cell for one measure.
    pub fn measure_at(&self, row_key: &[usize], col_key: &[usize], measure: usize) -> &str {
        self.get(row_key, col_key)
            .and_then(|v| v.get(measure))
            .map(|s| s.as_str())
            .unwrap_or("")
    }

    /// The table with its row and column dimensions exchanged.
    ///
    /// This is the "pivot" of a pivot table: the same cells, read the
    /// other way round. The measures and the note travel with it.
    pub fn pivoted(&self) -> Self {
        let mut cells = BTreeMap::new();
        for ((row_key, col_key), values) in &self.cells {
            cells.insert((col_key.clone(), row_key.clone()), values.clone());
        }
        Self {
            title: self.title.clone(),
            row_dims: self.col_dims.clone(),
            col_dims: self.row_dims.clone(),
            measures: self.measures.clone(),
            cells,
            note: self.note.clone(),
            measure_index: self.measure_index,
        }
    }

    /// The table with one dimension moved between the sides.
    ///
    /// This is the drag-and-drop of a pivot table: a variable that is
    /// across the top is brought down the side, and the other way
    /// round. The moved dimension becomes the outermost of its new
    /// side, the way StatsBench does it, and the cells are re-addressed so
    /// the values travel with the move.
    ///
    /// The dimension is named; `to_rows` says which side it goes to.
    pub fn with_dimension_moved(&self, name: &str, to_rows: bool) -> Option<Self> {
        if to_rows {
            let i = self.col_dims.iter().position(|d| d.name == name)?;
            let moved = self.col_dims[i].clone();
            let mut row_dims = vec![moved];
            row_dims.extend(self.row_dims.iter().cloned());
            let mut col_dims = Vec::new();
            for (j, d) in self.col_dims.iter().enumerate() {
                if j != i {
                    col_dims.push(d.clone());
                }
            }
            let mut cells = BTreeMap::new();
            for ((row_key, col_key), values) in &self.cells {
                let mut new_row = Vec::new();
                new_row.push(col_key[i]);
                new_row.extend(row_key.iter().copied());
                let mut new_col = Vec::new();
                for (j, v) in col_key.iter().enumerate() {
                    if j != i {
                        new_col.push(*v);
                    }
                }
                cells.insert((new_row, new_col), values.clone());
            }
            Some(Self {
                title: self.title.clone(),
                row_dims,
                col_dims,
                measures: self.measures.clone(),
                cells,
                note: self.note.clone(),
                measure_index: self.measure_index,
            })
        } else {
            let i = self.row_dims.iter().position(|d| d.name == name)?;
            let moved = self.row_dims[i].clone();
            let mut col_dims = vec![moved];
            col_dims.extend(self.col_dims.iter().cloned());
            let mut row_dims = Vec::new();
            for (j, d) in self.row_dims.iter().enumerate() {
                if j != i {
                    row_dims.push(d.clone());
                }
            }
            let mut cells = BTreeMap::new();
            for ((row_key, col_key), values) in &self.cells {
                let mut new_col = Vec::new();
                new_col.push(row_key[i]);
                new_col.extend(col_key.iter().copied());
                let mut new_row = Vec::new();
                for (j, v) in row_key.iter().enumerate() {
                    if j != i {
                        new_row.push(*v);
                    }
                }
                cells.insert((new_row, new_col), values.clone());
            }
            Some(Self {
                title: self.title.clone(),
                row_dims,
                col_dims,
                measures: self.measures.clone(),
                cells,
                note: self.note.clone(),
                measure_index: self.measure_index,
            })
        }
    }

    /// The table flattened into a grid of strings, one table per
    /// measure.
    ///
    /// Each table has a stub column carrying the row dimension's name
    /// and the measure, then a column for each leaf column.
    pub fn flattened(&self) -> Vec<(String, Vec<String>, Vec<Vec<String>>)> {
        let row_keys = self.row_keys();
        let col_keys = self.col_keys();
        let mut out = Vec::new();
        for (m, measure) in self.measures.iter().enumerate() {
            let mut headers = Vec::new();
            let stub = match self.row_dims.first() {
                Some(d) => format!("{} ({})", d.name, measure.to_lowercase()),
                None => measure.clone(),
            };
            headers.push(stub);
            for col_key in &col_keys {
                headers.push(label(&self.col_dims, col_key));
            }
            let mut rows = Vec::new();
            for row_key in &row_keys {
                let mut row = Vec::new();
                row.push(format!("{} ({})", label(&self.row_dims, row_key), measure.to_lowercase()));
                for col_key in &col_keys {
                    row.push(self.measure_at(row_key, col_key, m).to_string());
                }
                rows.push(row);
            }
            out.push((measure.clone(), headers, rows));
        }
        out
    }
}

/// The keys of a set of dimensions, in the order the leaves are shown.
///
/// The last dimension varies fastest, the way a spreadsheet reads, so
/// the levels of the first dimension group the ones below them.
pub fn keys(dims: &[PivotDimension]) -> Vec<Vec<usize>> {
    if dims.is_empty() {
        return vec![Vec::new()];
    }
    let total: usize = dims.iter().map(|d| d.span()).product();
    let mut out = Vec::with_capacity(total);
    for i in 0..total {
        let mut key = Vec::with_capacity(dims.len());
        let mut rest = i;
        for d in dims.iter().rev() {
            let s = d.span();
            key.push(rest % s);
            rest /= s;
        }
        key.reverse();
        out.push(key);
    }
    out
}

/// The label of a key: the level names, joined.
pub fn label(dims: &[PivotDimension], key: &[usize]) -> String {
    if dims.is_empty() {
        return String::new();
    }
    let parts: Vec<String> = dims
        .iter()
        .zip(key.iter())
        .map(|(d, &i)| {
            d.levels
                .get(i)
                .cloned()
                .unwrap_or_else(|| i.to_string())
        })
        .collect();
    parts.join(" / ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> PivotTable {
        let mut t = PivotTable::new(
            "Crosstabulation",
            vec![PivotDimension::new("region", vec!["East".into(), "West".into()])],
            vec![PivotDimension::new(
                "group",
                vec!["A".into(), "B".into(), "Total".into()],
            )],
            vec!["Count".into(), "Row %".into()],
        );
        let rk = t.row_keys();
        let ck = t.col_keys();
        for (i, r) in rk.iter().enumerate() {
            for (j, c) in ck.iter().enumerate() {
                let n = (i * 3 + j) as u32;
                t.set(
                    r,
                    c,
                    vec![n.to_string(), format!("{}.0%", n * 10)],
                );
            }
        }
        t
    }

    #[test]
    fn the_keys_enumerate_the_last_dimension_fastest() {
        let dims = vec![
            PivotDimension::new("a", vec!["a0".into(), "a1".into()]),
            PivotDimension::new("b", vec!["b0".into(), "b1".into(), "b2".into()]),
        ];
        let k = keys(&dims);
        assert_eq!(k.len(), 6);
        assert_eq!(k[0], vec![0, 0]);
        assert_eq!(k[1], vec![0, 1]);
        assert_eq!(k[2], vec![0, 2]);
        assert_eq!(k[3], vec![1, 0]);
    }

    #[test]
    fn a_cell_is_addressed_by_its_two_keys() {
        let t = table();
        let rk = t.row_keys();
        let ck = t.col_keys();
        assert_eq!(t.measure_at(&rk[1], &ck[2], 0), "5");
        assert_eq!(t.measure_at(&rk[1], &ck[2], 1), "50.0%");
    }

    #[test]
    fn the_pivot_exchanges_the_dimensions_and_transposes_the_cells() {
        let t = table();
        let p = t.pivoted();
        assert_eq!(p.row_dims[0].name, "group");
        assert_eq!(p.col_dims[0].name, "region");
        let rk = p.row_keys();
        let ck = p.col_keys();
        // The cell that was East/A is now A/East.
        assert_eq!(p.measure_at(&rk[0], &ck[0], 0), "0");
        assert_eq!(p.measure_at(&rk[1], &ck[1], 0), "4");
    }

    #[test]
    fn the_flattened_form_has_a_table_per_measure() {
        let t = table();
        let flat = t.flattened();
        assert_eq!(flat.len(), 2);
        assert_eq!(flat[0].0, "Count");
        // The stub names the variable and the measure.
        assert!(flat[0].1[0].contains("region"));
        assert!(flat[0].1[0].contains("count"));
        // One header per leaf column, plus the stub.
        assert_eq!(flat[0].1.len(), 4);
        assert_eq!(flat[0].2.len(), 2);
    }

    #[test]
    fn a_dimension_can_be_moved_to_the_other_side() {
        let t = table();
        let moved = t.with_dimension_moved("group", true).unwrap();
        // The moved dimension becomes the outermost of the rows.
        assert_eq!(moved.row_dims[0].name, "group");
        assert_eq!(moved.row_dims[1].name, "region");
        assert!(moved.col_dims.is_empty());
        // The values travel with the move: the cell that was
        // East/A is now A/East.
        let rk = moved.row_keys();
        assert_eq!(rk[0], vec![0, 0]);
        assert_eq!(moved.measure_at(&rk[0], &[], 0), "0");
        assert_eq!(moved.measure_at(&rk[1], &[], 0), "3");
    }

    #[test]
    fn a_row_dimension_can_be_moved_to_the_columns() {
        let t = table();
        let moved = t.with_dimension_moved("region", false).unwrap();
        // The moved dimension becomes the outermost of the columns,
        // and the row side is now empty.
        assert_eq!(moved.col_dims[0].name, "region");
        assert_eq!(moved.col_dims[1].name, "group");
        assert!(moved.row_dims.is_empty());
        // The cell that was East/A is now A/East: the row key is
        // empty and the column key holds both levels.
        let ck = moved.col_keys();
        assert_eq!(ck[0], vec![0, 0]);
        assert_eq!(moved.measure_at(&[], &ck[0], 0), "0");
        assert_eq!(moved.measure_at(&[], &ck[1], 0), "1");
    }
}
