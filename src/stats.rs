use crate::data::{Column, ColumnKind, DataTable, Value};
use statrs::distribution::ContinuousCDF;

/// Category key for a non-loading cell: text as-is, numbers formatted.
fn category_key(col: &Column, row: usize) -> Option<String> {
    if col.is_missing(row) {
        return None;
    }
    match col.values.get(row) {
        Some(Value::Text(s)) => Some(s.clone()),
        Some(Value::Number(n)) => Some(format!("{}", n)),
        _ => None,
    }
}

fn unique_levels(col: &Column, n_rows: usize) -> Vec<String> {
    let mut set = std::collections::BTreeSet::new();
    for r in 0..n_rows {
        if let Some(k) = category_key(col, r) {
            set.insert(k);
        }
    }
    set.into_iter().collect()
}

// ============================================================================
// Phase 5: Crosstabs + Pearson Chi-Square
// ============================================================================

#[derive(Debug, Clone)]
pub struct CrosstabsResult {
    pub var1: String,
    pub var2: String,
    pub levels1: Vec<String>,
    pub levels2: Vec<String>,
    pub observed: Vec<Vec<usize>>,
    pub row_totals: Vec<usize>,
    pub col_totals: Vec<usize>,
    pub n: usize,
    pub chi2: f64,
    pub df: usize,
    pub p: f64,
}

pub fn crosstabs_chi2(table: &DataTable, ia: usize, ib: usize) -> Option<CrosstabsResult> {
    let ca = table.columns.get(ia)?;
    let cb = table.columns.get(ib)?;
    let levels1 = unique_levels(ca, table.n_rows);
    let levels2 = unique_levels(cb, table.n_rows);
    if levels1.len() < 2 || levels2.len() < 2 || levels1.len() > 50 || levels2.len() > 50 {
        return None;
    }
    let idx1: std::collections::HashMap<&str, usize> =
        levels1.iter().enumerate().map(|(i, s)| (s.as_str(), i)).collect();
    let idx2: std::collections::HashMap<&str, usize> =
        levels2.iter().enumerate().map(|(i, s)| (s.as_str(), i)).collect();

    let r = levels1.len();
    let c = levels2.len();
    let mut observed = vec![vec![0usize; c]; r];
    let mut row_totals = vec![0usize; r];
    let mut col_totals = vec![0usize; c];
    let mut n = 0usize;
    for row in 0..table.n_rows {
        let (Some(k1), Some(k2)) = (category_key(ca, row), category_key(cb, row)) else {
            continue; // pairwise missing excluded (SPSS listwise for crosstabs)
        };
        let (Some(i), Some(j)) = (idx1.get(k1.as_str()).copied(), idx2.get(k2.as_str()).copied()) else {
            continue;
        };
        observed[i][j] += 1;
        row_totals[i] += 1;
        col_totals[j] += 1;
        n += 1;
    }
    if n == 0 {
        return None;
    }
    // Pearson chi-square on observed counts
    let mut chi2 = 0.0;
    for i in 0..r {
        for j in 0..c {
            let expected = (row_totals[i] as f64) * (col_totals[j] as f64) / n as f64;
            if expected > 0.0 {
                let o = observed[i][j] as f64;
                chi2 += (o - expected) * (o - expected) / expected;
            }
        }
    }
    let df = (r - 1) * (c - 1);
    let p = if df == 0 {
        f64::NAN
    } else {
        (|| -> Option<f64> {
            let dist = statrs::distribution::ChiSquared::new(df as f64).ok()?;
            Some(1.0 - dist.cdf(chi2))
        })()
        .unwrap_or(f64::NAN)
    };
    Some(CrosstabsResult {
        var1: ca.name.clone(),
        var2: cb.name.clone(),
        levels1,
        levels2,
        observed,
        row_totals,
        col_totals,
        n,
        chi2,
        df,
        p,
    })
}

// ============================================================================
// Phase 5: One-Way ANOVA
// ============================================================================

#[derive(Debug, Clone)]
pub struct AnovaResult {
    pub outcome: String,
    pub factor: String,
    pub n_groups: usize,
    pub df_between: f64,
    pub df_within: f64,
    pub ss_between: f64,
    pub ss_within: f64,
    pub ms_between: f64,
    pub ms_within: f64,
    pub f: f64,
    pub p: f64,
}

pub fn one_way_anova_in(table: &DataTable, outcome_idx: usize, factor_idx: usize) -> Option<AnovaResult> {
    let ocol = table.columns.get(outcome_idx)?;
    let fcol = table.columns.get(factor_idx)?;
    if ocol.kind != ColumnKind::Numeric {
        return None;
    }
    // group outcome values by factor category
    let mut groups: std::collections::BTreeMap<String, Vec<f64>> = std::collections::BTreeMap::new();
    for row in 0..table.n_rows {
        if ocol.is_missing(row) || fcol.is_missing(row) {
            continue;
        }
        let y = match ocol.values.get(row) {
            Some(Value::Number(x)) => *x,
            _ => continue,
        };
        let k = category_key(fcol, row)?;
        groups.entry(k).or_default().push(y);
    }
    let groups: Vec<Vec<f64>> = groups
        .into_values()
        .filter(|g| g.len() >= 2)
        .collect();
    if groups.len() < 2 {
        return None;
    }
    let total_n: f64 = groups.iter().map(|g| g.len()).sum::<usize>() as f64;
    let grand: f64 = groups.iter().flat_map(|g| g.iter()).sum::<f64>() / total_n;
    let mut ss_between = 0.0;
    let mut ss_within = 0.0;
    for g in &groups {
        let ng = g.len() as f64;
        let gm = g.iter().sum::<f64>() / ng;
        ss_between += ng * (gm - grand) * (gm - grand);
        ss_within += g.iter().map(|y| (y - gm) * (y - gm)).sum::<f64>();
    }
    let k = groups.len() as f64;
    let df_between = k - 1.0;
    let df_within = total_n - k;
    if df_between <= 0.0 || df_within <= 0.0 {
        return None;
    }
    let ms_between = ss_between / df_between;
    let ms_within = ss_within / df_within;
    if ms_within == 0.0 {
        return None;
    }
    let f = ms_between / ms_within;
    let p = (|| -> Option<f64> {
        let dist = statrs::distribution::FisherSnedecor::new(df_between, df_within).ok()?;
        Some(1.0 - dist.cdf(f))
    })()
    .unwrap_or(f64::NAN);
    Some(AnovaResult {
        outcome: ocol.name.clone(),
        factor: fcol.name.clone(),
        n_groups: groups.len(),
        df_between,
        df_within,
        ss_between,
        ss_within,
        ms_between,
        ms_within,
        f,
        p,
    })
}

// ============================================================================
// Phase 5: Paired-samples t-test
// ============================================================================

#[derive(Debug, Clone)]
pub struct PairedTResult {
    pub var1: String,
    pub var2: String,
    pub n: usize,
    pub mean1: f64,
    pub mean2: f64,
    pub mean_diff: f64,
    pub t: f64,
    pub df: f64,
    pub p: f64,
}

pub fn paired_ttest_in(table: &DataTable, ia: usize, ib: usize) -> Option<PairedTResult> {
    let ca = table.columns.get(ia)?;
    let cb = table.columns.get(ib)?;
    if ca.kind != ColumnKind::Numeric || cb.kind != ColumnKind::Numeric {
        return None;
    }
    let mut ds = Vec::new();
    let mut xs = Vec::new();
    let mut ys = Vec::new();
    for row in 0..table.n_rows {
        if ca.is_missing(row) || cb.is_missing(row) {
            continue;
        }
        if let (Value::Number(x), Value::Number(y)) =
            (&ca.values[row].clone(), &cb.values[row].clone())
        {
            xs.push(*x);
            ys.push(*y);
            ds.push(y - x);
        }
    }
    let n = ds.len();
    if n < 2 {
        return None;
    }
    let nf = n as f64;
    let mean1 = xs.iter().sum::<f64>() / nf;
    let mean2 = ys.iter().sum::<f64>() / nf;
    let md = ds.iter().sum::<f64>() / nf;
    let var_d = ds.iter().map(|d| (d - md).powi(2)).sum::<f64>() / (nf - 1.0);
    let se = (var_d / nf).sqrt();
    if se == 0.0 {
        return None;
    }
    let t = md / se;
    let df = nf - 1.0;
    let p = (|| -> Option<f64> {
        let dist = statrs::distribution::StudentsT::new(0.0, 1.0, df).ok()?;
        Some(2.0 * (1.0 - dist.cdf(t.abs())))
    })()
    .unwrap_or(f64::NAN);
    Some(PairedTResult {
        var1: ca.name.clone(),
        var2: cb.name.clone(),
        n,
        mean1,
        mean2,
        mean_diff: md,
        t,
        df,
        p,
    })
}

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
