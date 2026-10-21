use crate::data::{ColumnKind, DataTable, Value};
use statrs::distribution::ContinuousCDF;

#[derive(Debug, Clone)]
pub struct Descriptive {
    pub variable: String,
    pub n: usize,
    pub mean: f64,
    pub std_dev: f64,
    pub min: f64,
    pub max: f64,
}

pub fn descriptives(table: &DataTable) -> Vec<Descriptive> {
    // Phase 2 pipeline: menus/syntax select explicit variables; default = all numerics
    let idx: Vec<usize> = table
        .columns
        .iter()
        .enumerate()
        .filter(|(_, c)| c.kind == ColumnKind::Numeric)
        .map(|(i, _)| i)
        .collect();
    descriptives_in(table, &idx)
}

pub fn descriptives_in(table: &DataTable, var_idx: &[usize]) -> Vec<Descriptive> {
    let mut out = Vec::new();
    for &ci in var_idx {
        let col = match table.columns.get(ci) {
            Some(c) if c.kind == ColumnKind::Numeric => c,
            _ => continue,
        };
        let mut vals: Vec<f64> = Vec::new();
        for (row, v) in col.values.iter().enumerate() {
            if col.is_missing(row) {
                continue; // system- or user-missing excluded
            }
            if let Value::Number(x) = v {
                vals.push(*x);
            }
        }
        if vals.is_empty() {
            continue;
        }
        let n = vals.len();
        let mean = vals.iter().sum::<f64>() / n as f64;
        let var = vals.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n as f64 - 1.0).max(1.0);
        let std_dev = var.sqrt();
        let min = vals.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = vals.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        out.push(Descriptive {
            variable: col.name.clone(),
            n,
            mean,
            std_dev,
            min,
            max,
        });
    }
    out
}

#[derive(Debug, Clone)]
pub struct FreqRow {
    pub value: String,
    pub count: usize,
    pub percent: f64,
}

pub fn frequencies(table: &DataTable, col_idx: usize) -> Vec<FreqRow> {
    let col = match table.columns.get(col_idx) {
        Some(c) => c,
        None => return Vec::new(),
    };
    let mut counts: Vec<(String, usize)> = Vec::new();
    for (row, v) in col.values.iter().enumerate() {
        let key = if col.is_missing(row) {
            String::from("(missing)")
        } else {
            match v {
                Value::Number(n) => format!("{}", n),
                Value::Text(s) => s.clone(),
                Value::Missing => String::from("(missing)"),
            }
        };
        if let Some(e) = counts.iter_mut().find(|(k, _)| k == &key) {
            e.1 += 1;
        } else {
            counts.push((key, 1));
        }
    }
    let total = table.n_rows.max(1) as f64;
    let mut rows: Vec<FreqRow> = counts
        .into_iter()
        .map(|(value, count)| FreqRow {
            value,
            count,
            percent: count as f64 / total * 100.0,
        })
        .collect();
    rows.sort_by(|a, b| b.count.cmp(&a.count));
    rows
}

#[derive(Debug, Clone)]
pub struct Correlation {
    pub var1: String,
    pub var2: String,
    pub r: f64,
}

pub fn correlations(table: &DataTable) -> Vec<Correlation> {
    let nums: Vec<usize> = table
        .columns
        .iter()
        .enumerate()
        .filter(|(_, c)| c.kind == ColumnKind::Numeric)
        .map(|(i, _)| i)
        .collect();
    correlations_in(table, &nums)
}

pub fn correlations_in(table: &DataTable, nums: &[usize]) -> Vec<Correlation> {
    let mut out = Vec::new();
    for a in 0..nums.len() {
        for b in (a + 1)..nums.len() {
            let ia = nums[a];
            let ib = nums[b];
            let ca = &table.columns[ia];
            let cb = &table.columns[ib];
            // paired complete cases (system- or user-missing excluded)
            let mut xs = Vec::new();
            let mut ys = Vec::new();
            for (row, (va, vb)) in ca.values.iter().zip(cb.values.iter()).enumerate() {
                if ca.is_missing(row) || cb.is_missing(row) {
                    continue;
                }
                if let (Value::Number(x), Value::Number(y)) = (va, vb) {
                    xs.push(*x);
                    ys.push(*y);
                }
            }
            if xs.len() < 3 {
                continue;
            }
            let n = xs.len() as f64;
            let mx = xs.iter().sum::<f64>() / n;
            let my = ys.iter().sum::<f64>() / n;
            let cov = xs
                .iter()
                .zip(ys.iter())
                .map(|(x, y)| (x - mx) * (y - my))
                .sum::<f64>();
            let vx = xs.iter().map(|x| (x - mx).powi(2)).sum::<f64>().sqrt();
            let vy = ys.iter().map(|y| (y - my).powi(2)).sum::<f64>().sqrt();
            let r = if vx == 0.0 || vy == 0.0 {
                0.0
            } else {
                cov / (vx * vy)
            };
            out.push(Correlation {
                var1: ca.name.clone(),
                var2: cb.name.clone(),
                r,
            });
        }
    }
    out
}

#[derive(Debug, Clone)]
pub struct RegressionResult {
    pub predictor: String,
    pub outcome: String,
    pub n: usize,
    pub slope: f64,
    pub intercept: f64,
    pub r2: f64,
}

pub fn linear_regression(table: &DataTable) -> Option<RegressionResult> {
    let nums: Vec<usize> = table
        .columns
        .iter()
        .enumerate()
        .filter(|(_, c)| c.kind == ColumnKind::Numeric)
        .map(|(i, _)| i)
        .collect();
    if nums.len() < 2 {
        return None;
    }
    linear_regression_xy(table, nums[0], nums[1])
}

pub fn linear_regression_xy(table: &DataTable, yi: usize, xi: usize) -> Option<RegressionResult> {
    if yi >= table.columns.len() || xi >= table.columns.len() {
        return None;
    }
    let cx = &table.columns[xi];
    let cy = &table.columns[yi];
    let mut xs = Vec::new();
    let mut ys = Vec::new();
    for (row, (a, b)) in cx.values.iter().zip(cy.values.iter()).enumerate() {
        if cx.is_missing(row) || cy.is_missing(row) {
            continue;
        }
        if let (Value::Number(x), Value::Number(y)) = (a, b) {
            xs.push(*x);
            ys.push(*y);
        }
    }
    let n = xs.len();
    if n < 3 {
        return None;
    }
    let n = n as f64;
    let sx: f64 = xs.iter().sum();
    let sy: f64 = ys.iter().sum();
    let sxx: f64 = xs.iter().map(|x| x * x).sum();
    let sxy: f64 = xs.iter().zip(ys.iter()).map(|(x, y)| x * y).sum();
    let denom = n * sxx - sx * sx;
    if denom == 0.0 {
        return None;
    }
    let slope = (n * sxy - sx * sy) / denom;
    let intercept = (sy - slope * sx) / n;
    // r2
    let my = sy / n;
    let ss_tot: f64 = ys.iter().map(|y| (y - my).powi(2)).sum();
    let ss_res: f64 = xs
        .iter()
        .zip(ys.iter())
        .map(|(x, y)| {
            let pred = intercept + slope * x;
            (y - pred).powi(2)
        })
        .sum();
    let r2 = if ss_tot == 0.0 {
        0.0
    } else {
        1.0 - ss_res / ss_tot
    };
    Some(RegressionResult {
        predictor: cx.name.clone(),
        outcome: cy.name.clone(),
        n: xs.len(),
        slope,
        intercept,
        r2,
    })
}

#[derive(Debug, Clone)]
pub struct TTestResult {
    pub outcome: String,
    pub group_var: String,
    pub group1: String,
    pub group2: String,
    pub n1: usize,
    pub n2: usize,
    pub mean1: f64,
    pub mean2: f64,
    pub t: f64,
    pub df: f64,
    pub p: f64,
}

pub fn independent_ttest(table: &DataTable) -> Option<TTestResult> {
    // first string column with exactly 2 unique values, first numeric column
    let group_idx = table.columns.iter().position(|c| {
        if c.kind != ColumnKind::String {
            return false;
        }
        let mut uniq = std::collections::BTreeSet::new();
        for v in &c.values {
            if let Value::Text(s) = v {
                uniq.insert(s.clone());
            }
        }
        uniq.len() == 2
    })?;
    let outcome_idx = table.columns.iter().position(|c| c.kind == ColumnKind::Numeric)?;
    independent_ttest_vars(table, outcome_idx, group_idx)
}

pub fn independent_ttest_vars(
    table: &DataTable,
    outcome_idx: usize,
    group_idx: usize,
) -> Option<TTestResult> {
    let gcol = &table.columns.get(group_idx)?;
    let ocol = &table.columns.get(outcome_idx)?;
    if gcol.kind != ColumnKind::String || ocol.kind != ColumnKind::Numeric {
        return None;
    }

    let mut uniq = std::collections::BTreeSet::new();
    for v in &gcol.values {
        if let Value::Text(s) = v {
            uniq.insert(s.clone());
        }
    }
    let mut it = uniq.into_iter();
    let g1name = it.next().unwrap();
    let g2name = it.next().unwrap();

    let mut g1 = Vec::new();
    let mut g2 = Vec::new();
    for (row, (gv, ov)) in gcol.values.iter().zip(ocol.values.iter()).enumerate() {
        if gcol.is_missing(row) || ocol.is_missing(row) {
            continue;
        }
        if let (Value::Text(gs), Value::Number(ov)) = (gv, ov) {
            if gs == &g1name {
                g1.push(*ov);
            } else if gs == &g2name {
                g2.push(*ov);
            }
        }
    }
    if g1.len() < 2 || g2.len() < 2 {
        return None;
    }
    let n1 = g1.len() as f64;
    let n2 = g2.len() as f64;
    let m1 = g1.iter().sum::<f64>() / n1;
    let m2 = g2.iter().sum::<f64>() / n2;
    let v1 = g1.iter().map(|x| (x - m1).powi(2)).sum::<f64>() / (n1 - 1.0);
    let v2 = g2.iter().map(|x| (x - m2).powi(2)).sum::<f64>() / (n2 - 1.0);
    let se = (v1 / n1 + v2 / n2).sqrt();
    if se == 0.0 {
        return None;
    }
    let t = (m1 - m2) / se;
    let df = (v1 / n1 + v2 / n2).powi(2)
        / ((v1 / n1).powi(2) / (n1 - 1.0) + (v2 / n2).powi(2) / (n2 - 1.0));
    // p-value via statrs StudentsT
    let p = (|| -> Option<f64> {
        let dist = statrs::distribution::StudentsT::new(0.0, 1.0, df).ok()?;
        Some(2.0 * (1.0 - dist.cdf(t.abs())))
    })()
    .unwrap_or(f64::NAN);
    Some(TTestResult {
        outcome: ocol.name.clone(),
        group_var: gcol.name.clone(),
        group1: g1name,
        group2: g2name,
        n1: g1.len(),
        n2: g2.len(),
        mean1: m1,
        mean2: m2,
        t,
        df,
        p,
    })
}
