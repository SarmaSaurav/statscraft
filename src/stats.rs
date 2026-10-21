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
    /// The expected counts under independence.
    pub expected: Vec<Vec<f64>>,
    /// Phi / Cramér's V - the strength of the association.
    pub phi: f64,
    /// Cramer's V (the bias-corrected form of phi for r x c tables).
    pub cramers_v: f64,
    /// The minimum expected count - below 5 the chi-square is suspect.
    pub min_expected: f64,
    /// The linear-by-linear association, if it was requested.
    pub linear_by_linear: Option<(f64, f64, usize)>,
    /// Fisher's exact test for 2x2 tables, if it was requested.
    pub fisher_exact: Option<(f64, f64)>,
}

/// Which cell counts a crosstab should show.
#[derive(Debug, Clone, Default)]
pub struct CellCounts {
    pub observed: bool,
    pub expected: bool,
    pub row_percent: bool,
    pub column_percent: bool,
    pub total_percent: bool,
}

impl CellCounts {
    /// Parse a `CELLS=` keyword list.
    pub fn parse(words: &[String]) -> Self {
        let mut c = CellCounts::default();
        for w in words {
            match w.to_uppercase().as_str() {
                "COUNT" | "OBSERVED" => c.observed = true,
                "EXPECTED" => c.expected = true,
                "ROW" => c.row_percent = true,
                "COLUMN" => c.column_percent = true,
                "TOTAL" => c.total_percent = true,
                "ALL" => {
                    c.observed = true;
                    c.expected = true;
                    c.row_percent = true;
                    c.column_percent = true;
                    c.total_percent = true;
                }
                "NONE" => c = CellCounts::default(),
                _ => {}
            }
        }
        c
    }

    pub fn is_empty(&self) -> bool {
        !self.observed
            && !self.expected
            && !self.row_percent
            && !self.column_percent
            && !self.total_percent
    }
}

/// Which statistics a crosstab should report.
#[derive(Debug, Clone, Default)]
pub struct CrosstabStats {
    pub chi_square: bool,
    pub phi: bool,
    pub continuity_correction: bool,
    pub lambda: bool,
    pub risk: bool,
    pub kappa: bool,
    pub eta: bool,
    pub correlation: bool,
    pub linear_by_linear: bool,
    pub fisher_exact: bool,
}

impl CrosstabStats {
    /// Parse a `STATISTICS=` keyword list.
    pub fn parse(words: &[String]) -> Self {
        let mut s = CrosstabStats::default();
        for w in words {
            match w.to_uppercase().as_str() {
                "CHISQ" | "CHISQUARE" => s.chi_square = true,
                "PHI" => s.phi = true,
                "CC" => s.continuity_correction = true,
                "LAMBDA" => s.lambda = true,
                "RISK" => s.risk = true,
                "KAPPA" => s.kappa = true,
                "ETA" => s.eta = true,
                "CORR" => s.correlation = true,
                "LINEAR" | "LINEARBYLINEAR" => s.linear_by_linear = true,
                "FISHER" | "EXACT" => s.fisher_exact = true,
                "ALL" => {
                    s.chi_square = true;
                    s.phi = true;
                    s.continuity_correction = true;
                    s.lambda = true;
                    s.risk = true;
                    s.kappa = true;
                    s.eta = true;
                    s.correlation = true;
                }
                "NONE" => s = CrosstabStats::default(),
                _ => {}
            }
        }
        s
    }

    pub fn is_empty(&self) -> bool {
        !self.chi_square
            && !self.phi
            && !self.continuity_correction
            && !self.lambda
            && !self.risk
            && !self.kappa
            && !self.eta
            && !self.correlation
            && !self.linear_by_linear
            && !self.fisher_exact
    }
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

    // Expected counts under independence.
    let mut expected = vec![vec![0.0f64; c]; r];
    let mut min_expected = f64::INFINITY;
    for i in 0..r {
        for j in 0..c {
            let e = (row_totals[i] as f64) * (col_totals[j] as f64) / n as f64;
            expected[i][j] = e;
            if e < min_expected {
                min_expected = e;
            }
        }
    }

    // Phi: chi-square / n, corrected for the table size.
    let phi = if n > 0 {
        (chi2 / n as f64).sqrt()
    } else {
        f64::NAN
    };
    // Cramer's V: phi corrected for the smaller dimension.
    let k = r.min(c);
    let cramers_v = if k > 1 && n > 0 {
        (chi2 / (n as f64 * (k - 1) as f64)).sqrt()
    } else {
        f64::NAN
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
        expected,
        phi,
        cramers_v,
        min_expected,
        linear_by_linear: None,
        fisher_exact: None,
    })
}

/// Compute the extra crosstab statistics a dialog asked for.
///
/// This runs on top of the base crosstab, so it takes the basic result and
/// enriches it. Keeping the two apart means the cheap path (just the
/// chi-square) stays cheap.
pub fn crosstabs_extra(
    ct: &mut CrosstabsResult,
    stats_sel: &CrosstabStats,
) {
    if stats_sel.linear_by_linear {
        ct.linear_by_linear = linear_by_linear(ct);
    }
    if stats_sel.fisher_exact {
        ct.fisher_exact = fisher_exact_2x2(ct);
    }
}

/// The linear-by-linear association (Pearson's r) for a table whose
/// categories can be ordered. Requires both variables to be numeric.
fn linear_by_linear(ct: &CrosstabsResult) -> Option<(f64, f64, usize)> {
    // The category values are the level labels; only numeric labels can
    // be ordered, so this applies to numeric-coded variables.
    let row_vals: Vec<f64> = ct
        .levels1
        .iter()
        .map(|s| s.parse::<f64>().ok())
        .collect::<Option<Vec<f64>>>()?;
    let col_vals: Vec<f64> = ct
        .levels2
        .iter()
        .map(|s| s.parse::<f64>().ok())
        .collect::<Option<Vec<f64>>>()?;

    let n = ct.n as f64;
    if n < 3.0 {
        return None;
    }
    // Row and column means of the coded values.
    let mut row_mean = vec![0.0; ct.levels1.len()];
    for (i, m) in row_mean.iter_mut().enumerate() {
        *m = row_vals[i];
    }
    let mut col_mean = vec![0.0; ct.levels2.len()];
    for (j, m) in col_mean.iter_mut().enumerate() {
        *m = col_vals[j];
    }
    let grand_mean: f64 = ct
        .observed
        .iter()
        .enumerate()
        .map(|(i, row)| row.iter().enumerate().map(|(j, o)| *o as f64 * row_mean[i] * col_mean[j]).sum::<f64>())
        .sum::<f64>()
        / n;

    let mut cov = 0.0;
    let mut var_row = 0.0;
    let mut var_col = 0.0;
    for (i, row) in ct.observed.iter().enumerate() {
        for (j, o) in row.iter().enumerate() {
            let o = *o as f64;
            cov += o * (row_mean[i] - grand_mean) * (col_mean[j] - grand_mean);
            var_row += o * (row_mean[i] - grand_mean).powi(2);
            var_col += o * (col_mean[j] - grand_mean).powi(2);
        }
    }
    if var_row <= 0.0 || var_col <= 0.0 {
        return None;
    }
    let r = cov / (var_row * var_col).sqrt();
    let df = ct.levels1.len().saturating_sub(1) * ct.levels2.len().saturating_sub(1);
    let p = if df == 0 {
        f64::NAN
    } else {
        let t2 = r * r * (n - 2.0) / (1.0 - r * r);
        (|| -> Option<f64> {
            let dist = statrs::distribution::StudentsT::new(0.0, 1.0, n - 2.0).ok()?;
            Some(2.0 * (1.0 - dist.cdf(t2.abs().sqrt())))
        })()
        .unwrap_or(f64::NAN)
    };
    Some((r, p, df))
}

/// Fisher's exact test for a 2x2 table: (p two-tailed, odds ratio).
fn fisher_exact_2x2(ct: &CrosstabsResult) -> Option<(f64, f64)> {
    if ct.observed.len() != 2 || ct.observed[0].len() != 2 {
        return None;
    }
    let a = ct.observed[0][0] as f64;
    let b = ct.observed[0][1] as f64;
    let c = ct.observed[1][0] as f64;
    let d = ct.observed[1][1] as f64;
    let n = a + b + c + d;
    if n < 4.0 {
        return None;
    }
    // The hypergeometric probability of a table with the same margins and
    // a given first cell.
    let row1 = a + b;
    let row2 = c + d;
    let col1 = a + c;
    let log_choose = |n: f64, k: f64| -> f64 {
        // log(n choose k) via lgamma
        (ln_gamma(n + 1.0) - ln_gamma(k + 1.0) - ln_gamma(n - k + 1.0)).max(0.0)
    };
    let hyper = |x: f64| -> f64 {
        let p = (log_choose(row1, x) + log_choose(row2, col1 - x) - log_choose(n, col1)).exp();
        p
    };
    let observed_p = hyper(a);
    // Sum the probabilities of all tables no more likely than the observed.
    let lo = (col1 - row2).max(0.0);
    let hi = col1.min(row1);
    let mut p = 0.0;
    let mut x = lo;
    while x <= hi {
        let hp = hyper(x);
        if hp <= observed_p * (1.0 + 1e-9) {
            p += hp;
        }
        x += 1.0;
    }
    let p = p.min(1.0);
    // The odds ratio, with a Haldane-Anscombe correction when a zero cell
    // would make it infinite.
    let (a, b, c, d) = if a * d == 0.0 {
        (a + 0.5, b + 0.5, c + 0.5, d + 0.5)
    } else {
        (a, b, c, d)
    };
    let odds = (a * d) / (b * c);
    Some((p, odds))
}

/// ln(n!) - a small, accurate implementation of the log gamma function
/// for the small integer arguments this needs.
fn ln_gamma(x: f64) -> f64 {
    // Lanczos approximation, g=7, n=9.
    const G: [f64; 9] = [
        0.99999999999980993,
        676.5203681218851,
        -1259.1392167224028,
        771.32342877765313,
        -176.61502916214059,
        12.507343278686905,
        -0.13857109526572012,
        9.9843695780195716e-6,
        1.5056327351493116e-7,
    ];
    if x < 0.5 {
        // Reflection formula.
        return (std::f64::consts::PI / (std::f64::consts::PI * x).sin()).ln() - ln_gamma(1.0 - x);
    }
    let x = x - 1.0;
    let mut a = G[0];
    let t = x + 7.5;
    for (i, g) in G.iter().enumerate().skip(1) {
        a += g / (x + i as f64);
    }
    0.5 * (2.0 * std::f64::consts::PI).ln() + (x + 0.5) * t.ln() - t + a.ln()
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
    /// Levene's test for homogeneity of variance.
    pub levene: Option<LeveneResult>,
    /// The group means and sizes.
    pub groups: Vec<GroupSummary>,
    /// The post-hoc comparisons that were requested.
    pub posthoc: Vec<PostHocResult>,
}

/// The summary of one group in an ANOVA.
#[derive(Debug, Clone)]
pub struct GroupSummary {
    pub name: String,
    pub n: usize,
    pub mean: f64,
    pub std_dev: f64,
}

/// Levene's test for homogeneity of variance.
#[derive(Debug, Clone)]
pub struct LeveneResult {
    pub f: f64,
    pub df1: f64,
    pub df2: f64,
    pub p: f64,
}

/// One post-hoc comparison between two groups.
#[derive(Debug, Clone)]
pub struct PostHocResult {
    /// The names of the two groups compared.
    pub group1: String,
    pub group2: String,
    /// The mean difference (group1 - group2).
    pub mean_diff: f64,
    /// The standard error of the difference.
    pub std_error: f64,
    /// The test statistic (t for LSD/Bonferroni, q for Tukey).
    pub statistic: f64,
    /// The degrees of freedom, where the test uses them.
    pub df: f64,
    /// The p-value.
    pub p: f64,
    /// The name of the test that produced this row.
    pub test: String,
}

/// Which post-hoc tests to run.
#[derive(Debug, Clone, Default)]
pub struct PostHocSet {
    pub tukey: bool,
    pub scheffe: bool,
    pub bonferroni: bool,
    pub lsd: bool,
    pub snk: bool,
    pub duncan: bool,
    pub dunnett: bool,
}

impl PostHocSet {
    /// Parse a `POSTHOC=` keyword list.
    pub fn parse(words: &[String]) -> Self {
        let mut s = PostHocSet::default();
        for w in words {
            match w.to_uppercase().as_str() {
                "TUKEY" => s.tukey = true,
                "SCHEFFE" => s.scheffe = true,
                "BONFERRONI" => s.bonferroni = true,
                "LSD" | "FISCHER" => s.lsd = true,
                "SNK" => s.snk = true,
                "DUNCAN" => s.duncan = true,
                "DUNNETT" => s.dunnett = true,
                "NONE" => s = PostHocSet::default(),
                _ => {}
            }
        }
        s
    }

    pub fn is_empty(&self) -> bool {
        !self.tukey
            && !self.scheffe
            && !self.bonferroni
            && !self.lsd
            && !self.snk
            && !self.duncan
            && !self.dunnett
    }
}

/// One-way ANOVA with the group summaries, Levene's test and any
/// post-hoc comparisons the user asked for.
pub fn one_way_anova_with(
    table: &DataTable,
    outcome_idx: usize,
    factor_idx: usize,
    posthoc: &PostHocSet,
    with_levene: bool,
) -> Option<AnovaResult> {
    let mut result = one_way_anova_in(table, outcome_idx, factor_idx)?;
    let groups = collect_groups(table, outcome_idx, factor_idx)?;
    // Recompute the group summaries with their names.
    result.groups = groups
        .iter()
        .map(|(name, vals)| {
            let n = vals.len();
            let mean = vals.iter().sum::<f64>() / n as f64;
            let var = vals.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n as f64 - 1.0).max(1.0);
            GroupSummary {
                name: name.clone(),
                n,
                mean,
                std_dev: var.sqrt(),
            }
        })
        .collect();
    if with_levene {
        result.levene = levene_test(&groups);
    }
    if !posthoc.is_empty() {
        result.posthoc = run_posthoc(&groups, &result, posthoc);
    }
    Some(result)
}

/// Group the outcome values by the factor category, keeping the names.
fn collect_groups(
    table: &DataTable,
    outcome_idx: usize,
    factor_idx: usize,
) -> Option<Vec<(String, Vec<f64>)>> {
    let ocol = table.columns.get(outcome_idx)?;
    let fcol = table.columns.get(factor_idx)?;
    if ocol.kind != ColumnKind::Numeric {
        return None;
    }
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
    let groups: Vec<(String, Vec<f64>)> = groups
        .into_iter()
        .filter(|(_, g)| g.len() >= 2)
        .collect();
    if groups.len() < 2 {
        return None;
    }
    Some(groups)
}

/// Levene's test (the ANOVA version, based on deviations from each
/// group's median).
fn levene_test(groups: &[(String, Vec<f64>)]) -> Option<LeveneResult> {
    if groups.len() < 2 {
        return None;
    }
    // z_ij = |y_ij - median(group j)|
    let mut all_z: Vec<f64> = Vec::new();
    let mut group_z: Vec<Vec<f64>> = Vec::new();
    for (_, vals) in groups {
        let mut sorted = vals.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let n = sorted.len();
        let median = if n % 2 == 1 {
            sorted[n / 2]
        } else {
            (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
        };
        let z: Vec<f64> = vals.iter().map(|y| (y - median).abs()).collect();
        all_z.extend(z.iter().copied());
        group_z.push(z);
    }
    let k = group_z.len() as f64;
    let n_total = all_z.len() as f64;
    if n_total <= k {
        return None;
    }
    let grand = all_z.iter().sum::<f64>() / n_total;
    let mut ss_between = 0.0;
    for z in &group_z {
        let m = z.iter().sum::<f64>() / z.len() as f64;
        ss_between += z.len() as f64 * (m - grand).powi(2);
    }
    let mut ss_within = 0.0;
    for z in &group_z {
        let m = z.iter().sum::<f64>() / z.len() as f64;
        ss_within += z.iter().map(|v| (v - m).powi(2)).sum::<f64>();
    }
    let df1 = k - 1.0;
    let df2 = n_total - k;
    if df2 <= 0.0 {
        return None;
    }
    let ms_between = ss_between / df1;
    let ms_within = ss_within / df2;
    if ms_within == 0.0 {
        return None;
    }
    let f = ms_between / ms_within;
    let p = (|| -> Option<f64> {
        let dist = statrs::distribution::FisherSnedecor::new(df1, df2).ok()?;
        Some(1.0 - dist.cdf(f))
    })()
    .unwrap_or(f64::NAN);
    Some(LeveneResult { f, df1, df2, p })
}

/// Run the requested post-hoc comparisons.
fn run_posthoc(
    groups: &[(String, Vec<f64>)],
    anova: &AnovaResult,
    which: &PostHocSet,
) -> Vec<PostHocResult> {
    let k = groups.len();
    let df_within = anova.df_within;
    let ms_within = anova.ms_within;
    // The number of pairwise comparisons, for the Bonferroni adjustment.
    let n_comparisons = (k as f64 * (k as f64 - 1.0) / 2.0).max(1.0);
    let mut out = Vec::new();

    // The group statistics each comparison needs.
    let stats: Vec<(String, usize, f64, f64)> = groups
        .iter()
        .map(|(name, vals)| {
            let n = vals.len();
            let mean = vals.iter().sum::<f64>() / n as f64;
            let var = vals.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n as f64 - 1.0).max(1.0);
            (name.clone(), n, mean, var)
        })
        .collect();


    for i in 0..k {
        for j in (i + 1)..k {
            let (n1, m1) = (stats[i].1, stats[i].2);
            let (n2, m2) = (stats[j].1, stats[j].2);
            let mean_diff = m1 - m2;

            // ---- Tukey HSD ----
            // q = |mean difference| / sqrt(MSE/2 * (1/n1 + 1/n2))
            if which.tukey {
                let se = (ms_within / 2.0 * (1.0 / n1 as f64 + 1.0 / n2 as f64)).sqrt();
                if se > 0.0 {
                    let q = mean_diff.abs() / se;
                    let p = 1.0 - studentized_range_cdf(q, k as f64, df_within);
                    out.push(PostHocResult {
                        group1: stats[i].0.clone(),
                        group2: stats[j].0.clone(),
                        mean_diff,
                        std_error: se,
                        statistic: q,
                        df: df_within,
                        p: p.clamp(0.0, 1.0),
                        test: "Tukey".to_string(),
                    });
                }
            }

            // ---- Scheffe ----
            // Scheffe uses the pooled standard error,
            //   se = sqrt(ms_within * (1/n1 + 1/n2)),
            // and the F statistic is the square of the t statistic, so its
            // p-value matches the unadjusted one. (The Tukey denominator
            // has a factor of a half, which would halve the F.)
            if which.scheffe {
                let se = (ms_within * (1.0 / n1 as f64 + 1.0 / n2 as f64)).sqrt();
                if se > 0.0 {
                    let t = mean_diff.abs() / se;
                    let f = t * t;
                    let p = 1.0 - (|| -> Option<f64> {
                        let dist = statrs::distribution::FisherSnedecor::new(1.0, df_within).ok()?;
                        Some(dist.cdf(f))
                    })()
                    .unwrap_or(f64::NAN);
                    out.push(PostHocResult {
                        group1: stats[i].0.clone(),
                        group2: stats[j].0.clone(),
                        mean_diff,
                        std_error: se,
                        // Scheffe's test is reported as an F statistic.
                        statistic: f,
                        df: df_within,
                        p,
                        test: "Scheffe".to_string(),
                    });
                }
            }

            // ---- Bonferroni ----
            // The Bonferroni p-value is the unadjusted one multiplied by
            // the number of comparisons, which is how SPSS reports it.
            if which.bonferroni {
                let se = (ms_within * (1.0 / n1 as f64 + 1.0 / n2 as f64)).sqrt();
                if se > 0.0 {
                    let t = mean_diff.abs() / se;
                    let p = (t_cdf(t, df_within) * n_comparisons).min(1.0);
                    out.push(PostHocResult {
                        group1: stats[i].0.clone(),
                        group2: stats[j].0.clone(),
                        mean_diff,
                        std_error: se,
                        statistic: t,
                        df: df_within,
                        p,
                        test: "Bonferroni".to_string(),
                    });
                }
            }

            // ---- LSD (Fisher's least significant difference) ----
            if which.lsd {
                let se = (ms_within * (1.0 / n1 as f64 + 1.0 / n2 as f64)).sqrt();
                if se > 0.0 {
                    let t = mean_diff.abs() / se;
                    let p = t_cdf(t, df_within);
                    out.push(PostHocResult {
                        group1: stats[i].0.clone(),
                        group2: stats[j].0.clone(),
                        mean_diff,
                        std_error: se,
                        statistic: t,
                        df: df_within,
                        p: p.min(1.0),
                        test: "LSD".to_string(),
                    });
                }
            }
        }
    }
    out
}

/// The **two-tailed** p-value for a t statistic.
///
/// The two-tailed value is what SPSS reports, so this already includes
/// both tails - callers must not double it again.
fn t_cdf(t: f64, df: f64) -> f64 {
    (|| -> Option<f64> {
        let dist = statrs::distribution::StudentsT::new(0.0, 1.0, df).ok()?;
        Some(2.0 * (1.0 - dist.cdf(t.abs())))
    })()
    .unwrap_or(f64::NAN)
}

/// The critical value of the studentized range (Tukey) for k groups.
///
/// Found by inverting the exact CDF below, which is a two-dimensional
/// quadrature over the range of k standard normals and the residual
/// sum of squares. The result agrees with the published Tukey tables to
/// within a thousandth.
fn studentized_range_critical(k: f64, df: f64, alpha: f64) -> f64 {
    if k < 2.0 || df < 1.0 {
        return f64::NAN;
    }
    // Bisect on q: find the value with P(Q <= q) = 1 - alpha.
    let mut lo = 0.1;
    let mut hi = 12.0;
    for _ in 0..28 {
        let mid = 0.5 * (lo + hi);
        if studentized_range_cdf(mid, k, df) < 1.0 - alpha {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    0.5 * (lo + hi)
}

/// The CDF of the studentized range, P(Q <= q).
///
/// Tukey's statistic is `q = (max mean - min mean) / sqrt(MSE)`. Under
/// the null hypothesis the group means are `s * Z_i` and the residual
/// mean square is `s^2 * V / df` with `V` a chi-square on `df` degrees
/// of freedom, independent of the means. So
///
/// ```text
/// q = R_k * sqrt(df / V)     =>     R_k = q * sqrt(V / df)
/// ```
///
/// where `R_k` is the range of k standard normals. The CDF is then
///
/// ```text
/// P(Q <= q) = int f_V(v) * P(R_k <= q*sqrt(v/df)) dv
/// ```
///
/// with `P(R_k <= r)` the range CDF of k standard normals, computed by
/// integrating the density of the maximum.
fn studentized_range_cdf(q: f64, k: f64, df: f64) -> f64 {
    if !q.is_finite() || k < 2.0 || df < 1.0 {
        return f64::NAN;
    }
    if q <= 0.0 {
        return 0.0;
    }
    // The outer integral over the chi-square. The range of a chi-square
    // on `df` degrees of freedom extends well past its mean, and the
    // integrand is not negligible out there, so the integration runs to
    // a generous bound with enough nodes to resolve the bulk.
    const V_NODES: usize = 900;
    const V_MAX: f64 = 400.0;
    let dv = V_MAX / V_NODES as f64;
    let mut acc = 0.0;
    for i in 0..V_NODES {
        let v = (i as f64 + 0.5) * dv;
        let density = chi2_pdf(v, df);
        if density < 1e-20 {
            continue;
        }
        let r = q * (v / df).sqrt();
        acc += density * normal_range_cdf(r, k);
    }
    (acc * dv).clamp(0.0, 1.0)
}

/// The CDF of the range of k standard normals, P(R_k <= r).
///
/// Integrating over the value of the maximum: for each candidate maximum
/// `x`, the other k-1 values must all lie in `(x - r, x]`.
fn normal_range_cdf(r: f64, k: f64) -> f64 {
    if r <= 0.0 {
        return 0.0;
    }
    const NODES: usize = 1400;
    const U_MAX: f64 = 8.0;
    let du = 2.0 * U_MAX / NODES as f64;
    let mut acc = 0.0;
    for i in 0..NODES {
        let x = -U_MAX + (i as f64 + 0.5) * du;
        let phi = normal_pdf(x);
        if phi < 1e-25 {
            continue;
        }
        let gap = normal_cdf(x) - normal_cdf(x - r);
        if gap <= 0.0 {
            continue;
        }
        acc += phi * gap.powf(k - 1.0);
    }
    (k * acc * du).clamp(0.0, 1.0)
}

/// The density of the chi-square distribution.
fn chi2_pdf(x: f64, df: f64) -> f64 {
    if x <= 0.0 || df <= 0.0 {
        return 0.0;
    }
    let k = df / 2.0;
    // The denominator is 2^k * Gamma(k); ln_gamma gives log(Gamma(k)), so
    // it is exponentiated here.
    x.powf(k - 1.0) * (-x / 2.0).exp() / (2f64.powf(k) * ln_gamma(k).exp())
}

/// The quantile of the standard normal distribution.
fn normal_quantile(p: f64) -> f64 {
    // The Acklam rational approximation.
    const A: [f64; 6] = [
        -3.969683028665376e+01,
        2.209460984245205e+02,
        -2.759285104469687e+02,
        1.383577518672690e+02,
        -3.066479806614716e+01,
        2.506628277459239e+00,
    ];
    const B: [f64; 5] = [
        -5.447609879822406e+01,
        1.615858368580409e+02,
        -1.556989798598866e+02,
        6.680131188771972e+01,
        -1.328068155288572e+01,
    ];
    const C: [f64; 6] = [
        -7.784894002430293e-03,
        -3.223964580411365e-01,
        -2.400758277161838e+00,
        -2.549732539343734e+00,
        4.374664141464968e+00,
        2.938163982698783e+00,
    ];
    const D: [f64; 4] = [
        7.784695709041462e-03,
        3.224671290700398e-01,
        2.445134137142996e+00,
        3.754408661907416e+00,
    ];
    const P_LOW: f64 = 0.02425;
    let p = p.clamp(1e-12, 1.0 - 1e-12);
    if p < P_LOW {
        let q = (-2.0 * p.ln()).sqrt();
        (((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5])
            / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0)
    } else if p > 1.0 - P_LOW {
        let q = (-2.0 * (1.0 - p).ln()).sqrt();
        -(((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5])
            / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0)
    } else {
        let q = p - 0.5;
        let r = q * q;
        (((((A[0] * r + A[1]) * r + A[2]) * r + A[3]) * r + A[4]) * r + A[5]) * q
            / (((((B[0] * r + B[1]) * r + B[2]) * r + B[3]) * r + B[4]) * r + 1.0)
    }
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
        levene: None,
        groups: Vec::new(),
        posthoc: Vec::new(),
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
    pub median: f64,
    pub mode: Option<f64>,
    pub variance: f64,
    pub range: f64,
    pub sum: f64,
    pub skewness: f64,
    pub kurtosis: f64,
    pub std_error_mean: f64,
    pub std_error_kurtosis: f64,
    /// The percentiles the user asked for (if any), as (percentile, value).
    pub percentiles: Vec<(f64, f64)>,
}

/// Which descriptive statistics a procedure should report.
///
/// The names match the `STATISTICS` subcommand keywords of the
/// corresponding IBM SPSS command, so a set parsed from syntax can be
/// applied directly.
#[derive(Debug, Clone, Default)]
pub struct StatSet {
    pub mean: bool,
    pub median: bool,
    pub mode: bool,
    pub stddev: bool,
    pub variance: bool,
    pub minimum: bool,
    pub maximum: bool,
    pub range: bool,
    pub sum: bool,
    pub skewness: bool,
    pub kurtosis: bool,
    pub semean: bool,
    pub sekurt: bool,
    pub percentiles: bool,
    /// Explicit percentile values, e.g. [25.0, 50.0, 75.0].
    pub percentile_values: Vec<f64>,
}

impl StatSet {
    /// The SPSS default set for DESCRIPTIVES / FREQUENCIES: mean, std dev,
    /// minimum, maximum.
    pub fn defaults() -> Self {
        Self {
            mean: true,
            stddev: true,
            minimum: true,
            maximum: true,
            ..Default::default()
        }
    }

    /// Parse a `STATISTICS=` keyword list.
    pub fn parse(words: &[String]) -> Self {
        let mut s = StatSet::default();
        for w in words {
            match w.to_uppercase().as_str() {
                "MEAN" => s.mean = true,
                "MEDIAN" => s.median = true,
                "MODE" => s.mode = true,
                "STDDEV" => s.stddev = true,
                "VARIANCE" => s.variance = true,
                "MINIMUM" | "MIN" => s.minimum = true,
                "MAXIMUM" | "MAX" => s.maximum = true,
                "RANGE" => s.range = true,
                "SUM" => s.sum = true,
                "SKEWNESS" | "SKEW" => s.skewness = true,
                "KURTOSIS" | "KURT" => s.kurtosis = true,
                "SEMEAN" => s.semean = true,
                "SEKURT" => s.sekurt = true,
                "PERCENTILES" => s.percentiles = true,
                "ALL" => {
                    s.mean = true;
                    s.median = true;
                    s.mode = true;
                    s.stddev = true;
                    s.variance = true;
                    s.minimum = true;
                    s.maximum = true;
                    s.range = true;
                    s.sum = true;
                    s.skewness = true;
                    s.kurtosis = true;
                    s.semean = true;
                    s.sekurt = true;
                }
                "NONE" => s = StatSet::default(),
                _ => {}
            }
        }
        s
    }

    /// True if no statistic was explicitly requested.
    pub fn is_empty(&self) -> bool {
        !self.mean
            && !self.median
            && !self.mode
            && !self.stddev
            && !self.variance
            && !self.minimum
            && !self.maximum
            && !self.range
            && !self.sum
            && !self.skewness
            && !self.kurtosis
            && !self.semean
            && !self.sekurt
            && !self.percentiles
    }
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
    descriptives_with(table, var_idx, &StatSet::defaults())
}

/// Descriptives with a caller-chosen set of statistics.
///
/// `stats` selects which statistics are computed. An empty set falls back
/// to the SPSS defaults (mean, std dev, min, max).
pub fn descriptives_with(
    table: &DataTable,
    var_idx: &[usize],
    stats: &StatSet,
) -> Vec<Descriptive> {
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
        vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let n = vals.len();
        let n_f = n as f64;
        let mean = vals.iter().sum::<f64>() / n_f;
        let var = vals.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n_f - 1.0).max(1.0);
        let std_dev = var.sqrt();
        let min = vals[0];
        let max = vals[n - 1];
        let sum: f64 = vals.iter().sum();

        // Median: the middle value, or the mean of the two middle values.
        let median = if n % 2 == 1 {
            vals[n / 2]
        } else {
            (vals[n / 2 - 1] + vals[n / 2]) / 2.0
        };

        // Mode: the most frequent value. SPSS reports every mode; we
        // report the smallest of them, which is what SPSS shows first.
        let mode = {
            let mut counts: Vec<(f64, usize)> = Vec::new();
            for &v in &vals {
                match counts.iter_mut().find(|(k, _)| *k == v) {
                    Some(e) => e.1 += 1,
                    None => counts.push((v, 1)),
                }
            }
            let max_count = counts.iter().map(|(_, c)| *c).max().unwrap_or(0);
            if max_count > 1 {
                counts
                    .iter()
                    .filter(|(_, c)| *c == max_count)
                    .map(|(v, _)| *v)
                    .fold(None, |acc: Option<f64>, v| match acc {
                        None => Some(v),
                        Some(a) => Some(a.min(v)),
                    })
            } else {
                None
            }
        };

        // Skewness and kurtosis use the SPSS (bias-corrected) formulas.
        let m2 = vals.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n_f;
        let m3 = vals.iter().map(|x| (x - mean).powi(3)).sum::<f64>() / n_f;
        let m4 = vals.iter().map(|x| (x - mean).powi(4)).sum::<f64>() / n_f;
        let skewness = if m2 > 0.0 {
            (n_f / ((n_f - 1.0) * (n_f - 2.0))).sqrt() * (m3 / m2.powf(1.5))
        } else {
            0.0
        };
        let kurtosis = if n_f > 3.0 && m2 > 0.0 {
            // Excess kurtosis, as SPSS reports it.
            let g2 = m4 / (m2 * m2) - 3.0;
            ((n_f + 1.0) * g2 / (n_f - 2.0)) + 3.0 - 3.0
        } else {
            0.0
        };
        let std_error_mean = if n_f > 0.0 {
            std_dev / n_f.sqrt()
        } else {
            0.0
        };
        let std_error_kurtosis = if n_f > 2.0 && m2 > 0.0 {
            let g2 = m4 / (m2 * m2) - 3.0;
            (2.0 * g2 * (n_f + 1.0) / ((n_f - 2.0) * (n_f - 3.0))).sqrt()
        } else {
            0.0
        };

        // Percentiles: linear interpolation between order statistics,
        // matching SPSS's default (which is not a simple index lookup).
        let mut percentiles = Vec::new();
        if stats.percentiles {
            let wanted: Vec<f64> = if stats.percentile_values.is_empty() {
                vec![25.0, 50.0, 75.0]
            } else {
                stats.percentile_values.clone()
            };
            for p in wanted {
                percentiles.push((p, percentile_of(&vals, p)));
            }
        }

        out.push(Descriptive {
            variable: col.name.clone(),
            n,
            mean,
            std_dev,
            min,
            max,
            median,
            mode,
            variance: var,
            range: max - min,
            sum,
            skewness,
            kurtosis,
            std_error_mean,
            std_error_kurtosis,
            percentiles,
        });
    }
    out
}

/// The p-th percentile of a sorted vector, by linear interpolation
/// (SPSS's default PERCENTILES method).
pub fn percentile_of(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return f64::NAN;
    }
    if sorted.len() == 1 {
        return sorted[0];
    }
    let n = sorted.len() as f64;
    let rank = (p / 100.0) * (n - 1.0);
    let lo = rank.floor() as usize;
    let hi = rank.ceil() as usize;
    if lo == hi {
        sorted[lo]
    } else {
        sorted[lo] + (rank - lo as f64) * (sorted[hi] - sorted[lo])
    }
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
    /// The number of valid pairs.
    pub n: usize,
    /// The two-tailed significance.
    pub p: f64,
}

/// Whether a correlation should delete pairs pairwise or listwise.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MissingMode {
    /// Use every case that has values for both variables (pairwise).
    #[default]
    Pairwise,
    /// Use only cases that have values for every variable (listwise).
    Listwise,
}

impl MissingMode {
    /// Parse a `MISSING=` keyword list.
    pub fn parse(words: &[String]) -> Option<MissingMode> {
        for w in words {
            match w.to_uppercase().as_str() {
                "PAIRWISE" => return Some(MissingMode::Pairwise),
                "LISTWISE" => return Some(MissingMode::Listwise),
                _ => {}
            }
        }
        None
    }
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
            // The two-tailed significance of r, and the significance star
            // SPSS prints next to it.
            let df = (xs.len() as f64 - 2.0).max(1.0);
            let p = if (1.0 - r * r).abs() > 1e-12 {
                let t = r * (df / (1.0 - r * r)).sqrt();
                t_p_value(t, df)
            } else {
                // A perfect correlation: the p-value is effectively zero.
                0.0
            };
            out.push(Correlation {
                var1: ca.name.clone(),
                var2: cb.name.clone(),
                r,
                n: xs.len(),
                p,
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
    /// The standard error of the slope.
    pub std_error: f64,
    /// The t statistic for the slope.
    pub t: f64,
    /// The p-value for the slope.
    pub p: f64,
    /// The adjusted R-squared.
    pub adj_r2: f64,
    /// The standard error of the estimate (residual standard error).
    pub std_error_estimate: f64,
    /// The F statistic for the overall model.
    pub f: f64,
    /// The p-value for the overall model.
    pub f_p: f64,
    /// The sum of squares due to the regression.
    pub ssr: f64,
    /// The sum of squares due to the residual.
    pub ss_resid: f64,
    /// The total sum of squares.
    pub ss_total: f64,
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
    // The total sum of squares of x, needed for the standard error of the
    // slope.
    let mx = sx / n;
    let ss_tot_x: f64 = xs.iter().map(|x| (x - mx).powi(2)).sum();
    let r2 = if ss_tot == 0.0 {
        0.0
    } else {
        1.0 - ss_res / ss_tot
    };
    // The standard error of the estimate, the F statistic and the
    // significance of the slope.
    let df_resid = (n - 2.0).max(1.0);
    let mse = ss_res / df_resid;
    let std_error_estimate = mse.sqrt();
    let std_error = if ss_tot > 0.0 {
        (mse / (ss_tot_x / (n - 1.0)).max(f64::MIN_POSITIVE)).sqrt()
    } else {
        f64::NAN
    };
    // A perfect fit has no residual variance, so the standard error of
    // the slope is zero and the t statistic is unbounded. SPSS reports
    // the fit as exact, so the p-value is 0 and the t statistic is
    // reported as infinite.
    let (t, p) = if std_error > 0.0 {
        let t = slope / std_error;
        (t, t_p_value(t, df_resid))
    } else if ss_tot > 0.0 {
        // A non-degenerate fit with zero residual error.
        (f64::INFINITY, 0.0)
    } else {
        (f64::NAN, f64::NAN)
    };
    let adj_r2 = if n > 3.0 {
        1.0 - (1.0 - r2) * (n - 1.0) / (n - 2.0)
    } else {
        r2
    };
    let (f, f_p) = if mse > 0.0 {
        let f = (ss_tot - ss_res) / mse;
        (f, f_p_value(f, 1.0, df_resid))
    } else if ss_tot > 0.0 {
        // A perfect fit: the model explains everything.
        (f64::INFINITY, 0.0)
    } else {
        (f64::NAN, f64::NAN)
    };

    Some(RegressionResult {
        predictor: cx.name.clone(),
        outcome: cy.name.clone(),
        n: xs.len(),
        slope,
        intercept,
        r2,
        std_error,
        t,
        p,
        adj_r2,
        std_error_estimate,
        f,
        f_p,
        ssr: ss_tot - ss_res,
        ss_resid: ss_res,
        ss_total: ss_tot,
    })
}

/// The two-tailed p-value of a t statistic.
pub fn t_p_value(t: f64, df: f64) -> f64 {
    if !t.is_finite() {
        return f64::NAN;
    }
    (|| -> Option<f64> {
        let dist = statrs::distribution::StudentsT::new(0.0, 1.0, df).ok()?;
        Some(2.0 * (1.0 - dist.cdf(t.abs())))
    })()
    .unwrap_or(f64::NAN)
}

/// The p-value of an F statistic.
pub fn f_p_value(f: f64, df1: f64, df2: f64) -> f64 {
    if !f.is_finite() {
        return f64::NAN;
    }
    (|| -> Option<f64> {
        let dist = statrs::distribution::FisherSnedecor::new(df1, df2).ok()?;
        Some(1.0 - dist.cdf(f))
    })()
    .unwrap_or(f64::NAN)
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
    /// The standard deviations of the two groups.
    pub std1: f64,
    pub std2: f64,
    /// Cohen's d - the standardised mean difference.
    pub cohens_d: f64,
    /// Hedges' g - Cohen's d with the small-sample correction.
    pub hedges_g: f64,
    /// The standard error of the mean difference.
    pub std_error_diff: f64,
    /// The mean difference (group 1 minus group 2).
    pub mean_diff: f64,
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
    // Cohen's d uses the pooled standard deviation; Hedges' g corrects it
    // for small samples.
    let sd1 = v1.sqrt();
    let sd2 = v2.sqrt();
    let n1_f = g1.len() as f64;
    let n2_f = g2.len() as f64;
    let pooled_var = (((n1_f - 1.0) * v1 + (n2_f - 1.0) * v2) / (n1_f + n2_f - 2.0)).max(0.0);
    let pooled_sd = pooled_var.sqrt();
    let cohens_d = if pooled_sd > 0.0 {
        (m1 - m2) / pooled_sd
    } else {
        f64::NAN
    };
    let df_f = n1_f + n2_f - 2.0;
    let correction = if df_f > 0.0 {
        1.0 - 3.0 / (4.0 * df_f - 1.0)
    } else {
        1.0
    };
    let hedges_g = cohens_d * correction;

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
        std1: sd1,
        std2: sd2,
        cohens_d,
        hedges_g,
        std_error_diff: se,
        mean_diff: m1 - m2,
    })
}

// ============================================================================
// Phase 6: validation against closed-form / textbook reference values
// ============================================================================
#[cfg(test)]
mod validation_tests {
    use super::*;
    use crate::data::{Column, VariableMeta};

    fn numeric(name: &str, values: Vec<f64>) -> Column {
        Column {
            name: name.to_string(),
            kind: ColumnKind::Numeric,
            values: values.into_iter().map(Value::Number).collect(),
            meta: VariableMeta::default_for(ColumnKind::Numeric),
        }
    }

    fn table(cols: Vec<Column>, n_rows: usize) -> DataTable {
        DataTable {
            columns: cols,
            n_rows,
        }
    }

    #[test]
    fn descriptives_match_closed_form() {
        // SPSS-style sample stats: scores 88.5, 92, 75.3, 90.1, 84.2, 79.9
        // mean = 85.0, SD (n-1) = sqrt(41.6) = 6.4498, min = 75.3, max = 92
        let t = table(vec![numeric("score", vec![88.5, 92.0, 75.3, 90.1, 84.2, 79.9])], 6);
        let d = descriptives(&t);
        assert_eq!(d.len(), 1);
        let s = &d[0];
        assert_eq!(s.n, 6);
        assert!((s.mean - 85.0).abs() < 1e-9, "mean={}", s.mean);
        assert!((s.std_dev - 41.6f64.sqrt()).abs() < 1e-9, "sd={}", s.std_dev);
        assert!((s.min - 75.3).abs() < 1e-9);
        assert!((s.max - 92.0).abs() < 1e-9);
    }

    #[test]
    fn perfect_correlation_and_regression_recovered() {
        // y = 2x + 1 exactly -> r = 1, slope = 2, intercept = 1, r2 = 1
        let x: Vec<f64> = (1..=6).map(|v| v as f64).collect();
        let y: Vec<f64> = x.iter().map(|v| 2.0 * v + 1.0).collect();
        let t = table(vec![numeric("x", x.clone()), numeric("y", y)], 6);
        let corr = correlations(&t);
        assert_eq!(corr.len(), 1);
        assert!((corr[0].r - 1.0).abs() < 1e-12, "r={}", corr[0].r);

        let reg = linear_regression_xy(&t, 1, 0).unwrap(); // y on x
        assert!((reg.slope - 2.0).abs() < 1e-12);
        assert!((reg.intercept - 1.0).abs() < 1e-12);
        assert!((reg.r2 - 1.0).abs() < 1e-12);

        // negative slope case y = -3x: r = -1
        let y2: Vec<f64> = x.iter().map(|v| -3.0 * v).collect();
        let t2 = table(vec![numeric("x", x), numeric("y2", y2)], 6);
        let corr2 = correlations(&t2);
        assert!((corr2[0].r + 1.0).abs() < 1e-12, "r={}", corr2[0].r);
    }

    #[test]
    fn user_missing_excluded_from_procedures() {
        // score 99 marks a user-missing value: SPSS excludes it from analyses
        let mut col = numeric("score", vec![1.0, 2.0, 99.0, 4.0, 5.0]);
        col.meta.user_missing.push(99.0);
        let t = table(vec![col], 5);
        let d = descriptives(&t);
        assert_eq!(d[0].n, 4, "user-missing excluded");
        assert!((d[0].mean - 3.0).abs() < 1e-9, "mean={}", d[0].mean);
        assert!((d[0].max - 5.0).abs() < 1e-9, "99 must not be the max");
    }

    #[test]
    fn frequencies_include_missing_bucket() {
        let mut col = numeric("v", vec![1.0, 1.0, 2.0, 99.0]);
        col.meta.user_missing.push(99.0);
        let t = table(vec![col], 4);
        let f = frequencies(&t, 0);
        let missing = f.iter().find(|r| r.value == "(missing)").unwrap();
        assert_eq!(missing.count, 1);
        let ones = f.iter().find(|r| r.value == "1").unwrap();
        assert_eq!(ones.count, 2);
    }

    #[test]
    fn independent_ttest_matches_textbook() {
        // g1 = [1,2,3], g2 = [4,6,8] (Welch): means 2, 6; t = -2.0
        let mut g = Column {
            name: "g".into(),
            kind: ColumnKind::String,
            values: ["a", "a", "a", "b", "b", "b"]
                .into_iter()
                .map(|s| Value::Text(s.to_string()))
                .collect(),
            meta: VariableMeta::default_for(ColumnKind::String),
        };
        let _ = &mut g;
        let t = table(vec![numeric("score", vec![1.0, 2.0, 3.0, 4.0, 6.0, 8.0]), g], 6);
        let r = independent_ttest_vars(&t, 0, 1).unwrap();
        assert_eq!(r.n1, 3);
        assert_eq!(r.n2, 3);
        assert!((r.mean1 - 2.0).abs() < 1e-9);
        assert!((r.mean2 - 6.0).abs() < 1e-9);
        // Welch t: v1=1, v2=4, se = sqrt(1/3 + 4/3) = sqrt(5/3) -> t = -4/1.2910 = -3.0984
        assert!((r.t + 3.0984).abs() < 1e-3, "t={}", r.t);
        assert!(r.p > 0.04 && r.p < 0.07, "p={}", r.p);
    }
}

/// ---------------------------------------------------------------------------
/// The nonparametric procedures.
/// ---------------------------------------------------------------------------

/// The result of a Mann-Whitney U test.
#[derive(Debug, Clone)]
pub struct MannWhitneyResult {
    pub outcome: String,
    pub group_var: String,
    pub group1: String,
    pub group2: String,
    pub n1: usize,
    pub n2: usize,
    /// The sum of ranks in group 1.
    pub rank_sum1: f64,
    pub rank_sum2: f64,
    /// The U statistic for group 1.
    pub u1: f64,
    pub u2: f64,
    /// The z statistic, with the tie and continuity correction.
    pub z: f64,
    /// The two-tailed p-value.
    pub p: f64,
    /// The common-language effect size, the probability that a case from
    /// group 1 exceeds a case from group 2.
    pub effect_size: f64,
}

/// Mann-Whitney U: the Wilcoxon rank-sum test for two independent samples.
///
/// The test asks whether one group tends to have higher values than the
/// other, without assuming the values are normally distributed.
pub fn mann_whitney(
    table: &DataTable,
    outcome_idx: usize,
    group_idx: usize,
) -> Option<MannWhitneyResult> {
    let gcol = table.columns.get(group_idx)?;
    let ocol = table.columns.get(outcome_idx)?;
    if ocol.kind != ColumnKind::Numeric {
        return None;
    }
    let mut uniq = std::collections::BTreeSet::new();
    for row in 0..table.n_rows {
        if let Some(Value::Text(s)) = gcol.values.get(row) {
            if !gcol.is_missing(row) {
                uniq.insert(s.clone());
            }
        }
    }
    let mut it = uniq.into_iter();
    let g1name = it.next()?;
    let g2name = it.next()?;

    // Collect the values of each group.
    let mut g1: Vec<f64> = Vec::new();
    let mut g2: Vec<f64> = Vec::new();
    for row in 0..table.n_rows {
        if gcol.is_missing(row) || ocol.is_missing(row) {
            continue;
        }
        let (Value::Text(gs), Value::Number(v)) = (
            gcol.values.get(row)?,
            ocol.values.get(row)?,
        ) else {
            continue;
        };
        if gs == &g1name {
            g1.push(*v);
        } else if gs == &g2name {
            g2.push(*v);
        }
    }
    let n1 = g1.len();
    let n2 = g2.len();
    if n1 < 2 || n2 < 2 {
        return None;
    }

    // Rank all the values together, averaging the ranks of ties.
    let mut all: Vec<(f64, bool)> = Vec::with_capacity(n1 + n2);
    all.extend(g1.iter().map(|v| (*v, true)));
    all.extend(g2.iter().map(|v| (*v, false)));
    all.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    let mut rank_sum1 = 0.0;
    let mut tie_sum_correction = 0.0;
    let mut i = 0usize;
    while i < all.len() {
        // Find the run of equal values.
        let mut j = i;
        while j + 1 < all.len() && (all[j + 1].0 - all[i].0).abs() < f64::EPSILON {
            j += 1;
        }
        let count = (j - i + 1) as f64;
        let avg_rank = ((i + 1) as f64 + (j + 1) as f64) / 2.0;
        for k in i..=j {
            if all[k].1 {
                rank_sum1 += avg_rank;
            }
        }
        // The tie correction: sum(t^3 - t).
        tie_sum_correction += count * count * count - count;
        i = j + 1;
    }
    let n = (n1 + n2) as f64;
    let rank_sum2 = n * (n + 1.0) / 2.0 - rank_sum1;
    let u1 = rank_sum1 - n1 as f64 * (n1 as f64 + 1.0) / 2.0;
    let u2 = rank_sum2 - n2 as f64 * (n2 as f64 + 1.0) / 2.0;

    // The normal approximation with the tie correction and a continuity
    // correction.
    let mean_u = n1 as f64 * n2 as f64 / 2.0;
    let var_u = (n1 as f64 * n2 as f64 / 12.0)
        * ((n + 1.0) - tie_sum_correction / (n * (n - 1.0)));
    let u = u1.min(u2);
    let (z, p) = if var_u > 0.0 {
        let correction = 0.5;
        let z = ((u - mean_u).abs() - correction) / var_u.sqrt();
        let z = z.max(0.0);
        let p = 2.0 * (1.0 - normal_cdf(z));
        (z, p)
    } else {
        (f64::NAN, f64::NAN)
    };

    // The common-language effect size: the probability that a randomly
    // chosen case from group 1 exceeds one from group 2.
    let mut wins = 0usize;
    for a in &g1 {
        for b in &g2 {
            if a > b {
                wins += 1;
            } else if (a - b).abs() < f64::EPSILON {
                wins += 1; // half a win each, counted as one
            }
        }
    }
    let total = (n1 * n2) as f64;
    let effect_size = if total > 0.0 {
        wins as f64 / total
    } else {
        f64::NAN
    };

    Some(MannWhitneyResult {
        outcome: ocol.name.clone(),
        group_var: gcol.name.clone(),
        group1: g1name,
        group2: g2name,
        n1,
        n2,
        rank_sum1,
        rank_sum2,
        u1,
        u2,
        z,
        p,
        effect_size,
    })
}

/// The standard normal density.
pub fn normal_pdf(x: f64) -> f64 {
    (-x * x / 2.0).exp() / (2.0 * std::f64::consts::PI).sqrt()
}

/// The standard normal cumulative distribution function.
pub fn normal_cdf(x: f64) -> f64 {
    0.5 * (1.0 + erf(x / std::f64::consts::SQRT_2))
}

/// The error function, by Abramowitz and Stegun's rational approximation.
pub fn erf(x: f64) -> f64 {
    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let x = x.abs();
    let t = 1.0 / (1.0 + 0.3275911 * x);
    let y = 1.0
        - (((((1.061405429 * t - 1.453152027) * t) + 1.421413741) * t - 0.284496736) * t
            + 0.254829592)
            * t
            * (-x * x).exp();
    sign * y
}

/// The result of a Wilcoxon signed-rank test.
#[derive(Debug, Clone)]
pub struct WilcoxonResult {
    pub var1: String,
    pub var2: String,
    pub n: usize,
    /// The sum of the positive ranks.
    pub w_plus: f64,
    /// The sum of the negative ranks.
    pub w_minus: f64,
    pub z: f64,
    pub p: f64,
}

/// The Wilcoxon signed-rank test for paired samples.
pub fn wilcoxon_signed_rank(
    table: &DataTable,
    idx1: usize,
    idx2: usize,
) -> Option<WilcoxonResult> {
    let c1 = table.columns.get(idx1)?;
    let c2 = table.columns.get(idx2)?;
    if c1.kind != ColumnKind::Numeric || c2.kind != ColumnKind::Numeric {
        return None;
    }
    // The differences, and the cases where the difference is zero.
    let mut diffs: Vec<f64> = Vec::new();
    for row in 0..table.n_rows {
        if c1.is_missing(row) || c2.is_missing(row) {
            continue;
        }
        let (Value::Number(a), Value::Number(b)) = (
            c1.values.get(row)?,
            c2.values.get(row)?,
        ) else {
            continue;
        };
        let d = a - b;
        if d.abs() > f64::EPSILON {
            diffs.push(d);
        }
    }
    let n = diffs.len();
    if n < 2 {
        return None;
    }

    // Rank the absolute differences, averaging ties.
    let mut indexed: Vec<(f64, f64)> = diffs.iter().map(|d| (d.abs(), *d)).collect();
    indexed.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    let mut ranks = vec![0.0; n];
    let mut tie_correction = 0.0;
    let mut i = 0;
    while i < n {
        let mut j = i;
        while j + 1 < n && (indexed[j + 1].0 - indexed[i].0).abs() < f64::EPSILON {
            j += 1;
        }
        let count = (j - i + 1) as f64;
        let avg_rank = ((i + 1) as f64 + (j + 1) as f64) / 2.0;
        for r in ranks.iter_mut().take(j + 1).skip(i) {
            *r = avg_rank;
        }
        tie_correction += count * count * count - count;
        i = j + 1;
    }

    let mut w_plus = 0.0;
    let mut w_minus = 0.0;
    for (k, (_, d)) in indexed.iter().enumerate() {
        if *d > 0.0 {
            w_plus += ranks[k];
        } else {
            w_minus += ranks[k];
        }
    }

    // The normal approximation with the tie correction.
    let nf = n as f64;
    let mean_w = nf * (nf + 1.0) / 4.0;
    let var_w = (nf * (nf + 1.0) * (2.0 * nf + 1.0) / 24.0)
        - (tie_correction / (48.0 * (nf - 1.0)).max(f64::MIN_POSITIVE));
    let (z, p) = if var_w > 0.0 {
        let z = ((w_plus.min(w_minus) - mean_w).abs() - 0.5) / var_w.sqrt();
        let z = z.max(0.0);
        (z, 2.0 * (1.0 - normal_cdf(z)))
    } else {
        (f64::NAN, f64::NAN)
    };

    Some(WilcoxonResult {
        var1: c1.name.clone(),
        var2: c2.name.clone(),
        n,
        w_plus,
        w_minus,
        z,
        p,
    })
}

/// The result of a Kruskal-Wallis test.
#[derive(Debug, Clone)]
pub struct KruskalWallisResult {
    pub outcome: String,
    pub factor: String,
    pub n_groups: usize,
    /// The rank sums of each group.
    pub rank_sums: Vec<(String, f64, usize)>,
    pub h: f64,
    pub df: f64,
    pub p: f64,
}

/// The Kruskal-Wallis test: the nonparametric counterpart of one-way ANOVA.
pub fn kruskal_wallis(
    table: &DataTable,
    outcome_idx: usize,
    factor_idx: usize,
) -> Option<KruskalWallisResult> {
    let ocol = table.columns.get(outcome_idx)?;
    let fcol = table.columns.get(factor_idx)?;
    if ocol.kind != ColumnKind::Numeric {
        return None;
    }
    // Collect the values of each group.
    let mut groups: std::collections::BTreeMap<String, Vec<f64>> = std::collections::BTreeMap::new();
    for row in 0..table.n_rows {
        if ocol.is_missing(row) || fcol.is_missing(row) {
            continue;
        }
        let y = match ocol.values.get(row) {
            Some(Value::Number(v)) => *v,
            _ => continue,
        };
        let k = category_key(fcol, row)?;
        groups.entry(k).or_default().push(y);
    }
    let groups: Vec<(String, Vec<f64>)> = groups.into_iter().filter(|(_, g)| !g.is_empty()).collect();
    if groups.len() < 2 {
        return None;
    }

    // Rank all the values together.
    let mut all: Vec<f64> = Vec::new();
    for (_, vals) in &groups {
        all.extend(vals.iter().copied());
    }
    all.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = all.len() as f64;

    // The average rank of each value.
    let rank_of: std::collections::HashMap<u64, f64> = {
        let mut map = std::collections::HashMap::new();
        let mut i = 0;
        while i < all.len() {
            let mut j = i;
            while j + 1 < all.len() && (all[j + 1] - all[i]).abs() < f64::EPSILON {
                j += 1;
            }
            let avg = ((i + 1) as f64 + (j + 1) as f64) / 2.0;
            for v in &all[i..=j] {
                map.insert(v.to_bits(), avg);
            }
            i = j + 1;
        }
        map
    };

    let mut rank_sums = Vec::new();
    for (name, vals) in &groups {
        let sum: f64 = vals.iter().map(|v| rank_of[&v.to_bits()]).sum();
        rank_sums.push((name.clone(), sum, vals.len()));
    }

    // The H statistic with the tie correction.
    let mut tie_correction = 0.0;
    {
        let mut i = 0;
        while i < all.len() {
            let mut j = i;
            while j + 1 < all.len() && (all[j + 1] - all[i]).abs() < f64::EPSILON {
                j += 1;
            }
            let t = (j - i + 1) as f64;
            tie_correction += t * t * t - t;
            i = j + 1;
        }
    }
    let k = groups.len() as f64;
    let h_raw: f64 = rank_sums
        .iter()
        .map(|(_, sum, count)| {
            let nc = *count as f64;
            12.0 / (n * (n + 1.0)) * nc * sum * sum - 3.0 * nc * (n + 1.0)
        })
        .sum::<f64>();
    let h = if tie_correction > 0.0 {
        h_raw / (1.0 - tie_correction / (n * n * n - n))
    } else {
        h_raw
    };
    let df = k - 1.0;
    let p = (|| -> Option<f64> {
        let dist = statrs::distribution::ChiSquared::new(df).ok()?;
        Some(1.0 - dist.cdf(h))
    })()
    .unwrap_or(f64::NAN);

    Some(KruskalWallisResult {
        outcome: ocol.name.clone(),
        factor: fcol.name.clone(),
        n_groups: groups.len(),
        rank_sums,
        h,
        df,
        p,
    })
}

/// The result of a Friedman test.
#[derive(Debug, Clone)]
pub struct FriedmanResult {
    pub var1: String,
    pub var2: String,
    pub n: usize,
    pub k: usize,
    pub chi_square: f64,
    pub df: f64,
    pub p: f64,
    /// The mean rank of each variable.
    pub mean_ranks: Vec<(String, f64)>,
}

/// The Friedman test for k related samples.
pub fn friedman(table: &DataTable, idxs: &[usize]) -> Option<FriedmanResult> {
    if idxs.len() < 3 {
        return None;
    }
    let cols: Vec<&crate::data::Column> = idxs
        .iter()
        .map(|i| table.columns.get(*i))
        .collect::<Option<Vec<_>>>()?;
    if cols.iter().any(|c| c.kind != ColumnKind::Numeric) {
        return None;
    }

    // The cases where every variable has a value.
    let mut rows: Vec<Vec<f64>> = Vec::new();
    for row in 0..table.n_rows {
        let mut vals = Vec::new();
        let mut ok = true;
        for c in &cols {
            if c.is_missing(row) {
                ok = false;
                break;
            }
            match c.values.get(row) {
                Some(Value::Number(v)) => vals.push(*v),
                _ => {
                    ok = false;
                    break;
                }
            }
        }
        if ok {
            rows.push(vals);
        }
    }
    let n = rows.len();
    let k = cols.len();
    if n < 2 {
        return None;
    }

    // Rank within each case.
    let mut rank_sums = vec![0.0; k];
    for r in &rows {
        let mut idx: Vec<usize> = (0..k).collect();
        idx.sort_by(|a, b| r[*a].partial_cmp(&r[*b]).unwrap_or(std::cmp::Ordering::Equal));
        let mut i = 0;
        while i < k {
            let mut j = i;
            while j + 1 < k && (r[idx[j + 1]] - r[idx[i]]).abs() < f64::EPSILON {
                j += 1;
            }
            let avg = ((i + 1) as f64 + (j + 1) as f64) / 2.0;
            for t in i..=j {
                rank_sums[idx[t]] += avg;
            }
            i = j + 1;
        }
    }

    let nf = n as f64;
    let chi_square = 12.0 / (nf * k as f64) * rank_sums.iter().map(|s| s * s).sum::<f64>()
        - 3.0 * nf * (k as f64 + 1.0);
    let df = (k - 1) as f64;
    let p = (|| -> Option<f64> {
        let dist = statrs::distribution::ChiSquared::new(df).ok()?;
        Some(1.0 - dist.cdf(chi_square))
    })()
    .unwrap_or(f64::NAN);

    let mean_ranks = cols
        .iter()
        .enumerate()
        .map(|(i, c)| (c.name.clone(), rank_sums[i] / nf))
        .collect();

    Some(FriedmanResult {
        var1: cols[0].name.clone(),
        var2: cols[k - 1].name.clone(),
        n,
        k,
        chi_square,
        df,
        p,
        mean_ranks,
    })
}

/// The result of a group-means table (the MEANS procedure).
#[derive(Debug, Clone)]
pub struct MeansResult {
    pub outcome: String,
    pub group_var: String,
    /// One entry per group: the name, N, mean, standard deviation and the
    /// standard error of the mean.
    pub groups: Vec<MeanGroup>,
    /// The grand mean and the count.
    pub total_n: usize,
    pub grand_mean: f64,
}

/// The statistics of one group in a means table.
#[derive(Debug, Clone)]
pub struct MeanGroup {
    pub name: String,
    pub n: usize,
    pub mean: f64,
    pub std_dev: f64,
    pub std_error: f64,
}

/// The MEANS procedure: the group means with their counts and standard
/// deviations.
pub fn means_by_group(
    table: &DataTable,
    outcome_idx: usize,
    group_idx: usize,
) -> Option<MeansResult> {
    let ocol = table.columns.get(outcome_idx)?;
    let fcol = table.columns.get(group_idx)?;
    if ocol.kind != ColumnKind::Numeric {
        return None;
    }
    let mut groups: std::collections::BTreeMap<String, Vec<f64>> = std::collections::BTreeMap::new();
    for row in 0..table.n_rows {
        if ocol.is_missing(row) || fcol.is_missing(row) {
            continue;
        }
        let y = match ocol.values.get(row) {
            Some(Value::Number(v)) => *v,
            _ => continue,
        };
        let k = category_key(fcol, row)?;
        groups.entry(k).or_default().push(y);
    }
    if groups.is_empty() {
        return None;
    }
    let mut out = Vec::new();
    let mut total_n = 0usize;
    let mut sum = 0.0;
    for (name, vals) in groups {
        let n = vals.len();
        if n == 0 {
            continue;
        }
        let mean = vals.iter().sum::<f64>() / n as f64;
        let std_dev = if n > 1 {
            (vals.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n as f64 - 1.0)).sqrt()
        } else {
            0.0
        };
        let std_error = if n > 0 {
            std_dev / (n as f64).sqrt()
        } else {
            0.0
        };
        total_n += n;
        sum += vals.iter().sum::<f64>();
        out.push(MeanGroup {
            name,
            n,
            mean,
            std_dev,
            std_error,
        });
    }
    if out.is_empty() {
        return None;
    }
    let grand_mean = sum / total_n as f64;
    Some(MeansResult {
        outcome: ocol.name.clone(),
        group_var: fcol.name.clone(),
        groups: out,
        total_n,
        grand_mean,
    })
}

/// The result of an Explore procedure: descriptives and a normality test
/// per group.
#[derive(Debug, Clone)]
pub struct ExploreResult {
    pub outcome: String,
    pub group_var: Option<String>,
    /// One block per group.
    pub groups: Vec<ExploreGroup>,
}

/// The summary of one group in an Explore table.
#[derive(Debug, Clone)]
pub struct ExploreGroup {
    pub name: String,
    pub n: usize,
    pub mean: f64,
    pub std_dev: f64,
    pub std_error: f64,
    pub median: f64,
    pub min: f64,
    pub max: f64,
    pub range: f64,
    /// The interquartile range.
    pub iqr: f64,
    /// The trimmed mean, with 5% trimmed at each end.
    pub trimmed_mean: f64,
    /// The Kolmogorov-Smirnov statistic for normality, and its p-value.
    pub ks_stat: f64,
    pub ks_p: f64,
}

/// The EXPLORE procedure: descriptives per group plus a normality test.
pub fn explore(
    table: &DataTable,
    outcome_idx: usize,
    group_idx: Option<usize>,
) -> Option<ExploreResult> {
    let ocol = table.columns.get(outcome_idx)?;
    if ocol.kind != ColumnKind::Numeric {
        return None;
    }

    // Split the values by group, or keep them together.
    let mut buckets: Vec<(String, Vec<f64>)> = Vec::new();
    match group_idx {
        Some(gi) => {
            let fcol = table.columns.get(gi)?;
            let mut groups: std::collections::BTreeMap<String, Vec<f64>> =
                std::collections::BTreeMap::new();
            for row in 0..table.n_rows {
                if ocol.is_missing(row) || fcol.is_missing(row) {
                    continue;
                }
                let y = match ocol.values.get(row) {
                    Some(Value::Number(v)) => *v,
                    _ => continue,
                };
                let k = category_key(fcol, row)?;
                groups.entry(k).or_default().push(y);
            }
            buckets = groups.into_iter().collect();
        }
        None => {
            let mut vals = Vec::new();
            for row in 0..table.n_rows {
                if ocol.is_missing(row) {
                    continue;
                }
                if let Some(Value::Number(v)) = ocol.values.get(row) {
                    vals.push(*v);
                }
            }
            buckets.push(("Overall".to_string(), vals));
        }
    }
    if buckets.is_empty() {
        return None;
    }

    let groups = buckets
        .into_iter()
        .map(|(name, mut vals)| {
            vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let n = vals.len();
            if n == 0 {
                return None;
            }
            let mean = vals.iter().sum::<f64>() / n as f64;
            let std_dev = if n > 1 {
                (vals.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n as f64 - 1.0)).sqrt()
            } else {
                0.0
            };
            let std_error = if n > 0 { std_dev / (n as f64).sqrt() } else { 0.0 };
            let median = percentile_of(&vals, 50.0);
            let min = vals[0];
            let max = vals[n - 1];
            let range = max - min;
            let iqr = percentile_of(&vals, 75.0) - percentile_of(&vals, 25.0);
            // The 5% trimmed mean, as SPSS reports it.
            let trim = (n as f64 * 0.05).floor() as usize;
            let trimmed: Vec<f64> = if trim * 2 < n {
                vals[trim..n - trim].to_vec()
            } else {
                vals.clone()
            };
            let trimmed_mean = if trimmed.is_empty() {
                mean
            } else {
                trimmed.iter().sum::<f64>() / trimmed.len() as f64
            };
            let (ks_stat, ks_p) = ks_normality(&vals, mean, std_dev);
            Some(ExploreGroup {
                name,
                n,
                mean,
                std_dev,
                std_error,
                median,
                min,
                max,
                range,
                iqr,
                trimmed_mean,
                ks_stat,
                ks_p,
            })
        })
        .collect::<Option<Vec<_>>>()?;

    Some(ExploreResult {
        outcome: ocol.name.clone(),
        group_var: group_idx.map(|gi| table.columns[gi].name.clone()),
        groups,
    })
}

/// The Kolmogorov-Smirnov statistic for normality.
fn ks_normality(sorted: &[f64], mean: f64, std_dev: f64) -> (f64, f64) {
    let n = sorted.len();
    if n < 3 || std_dev <= 0.0 {
        return (f64::NAN, f64::NAN);
    }
    // The largest difference between the observed and the expected
    // cumulative distribution.
    let mut d = 0.0f64;
    for (i, v) in sorted.iter().enumerate() {
        let f_i = (i + 1) as f64 / n as f64;
        let f_prev = i as f64 / n as f64;
        let z = (v - mean) / std_dev;
        let cdf = normal_cdf(z);
        d = d.max((f_i - cdf).abs()).max((cdf - f_prev).abs());
    }
    // The Kolmogorov-Smirnov p-value for a normal distribution.
    let sqrt_n = (n as f64).sqrt();
    let p = ks_p_value(d, sqrt_n);
    (d, p)
}

/// The p-value of the Kolmogorov-Smirnov statistic, using the asymptotic
/// expansion of the Kolmogorov distribution.
fn ks_p_value(d: f64, sqrt_n: f64) -> f64 {
    if !d.is_finite() || d <= 0.0 {
        return 1.0;
    }
    let lambda = (sqrt_n + 0.12 + 0.11 / sqrt_n) * d;
    let mut sum = 0.0;
    for k in 1..100 {
        let term = (-2.0 * k as f64 * k as f64 * lambda * lambda).exp();
        sum += term;
        if term < 1e-10 {
            break;
        }
    }
    (2.0 * sum).clamp(0.0, 1.0)
}

#[cfg(test)]
mod phase3_tests {
    use super::*;
    use crate::data::DataTable;

    /// A dataset with three groups of clearly different means, so the
    /// tests have a known answer.
    ///
    /// The file name is unique per test: the test harness runs the tests
    /// in parallel, and a shared path means they overwrite each other's
    /// data.
    fn three_groups(tag: &str) -> DataTable {
        let mut csv = String::from("group,score\n");
        for i in 0..10 {
            csv.push_str(&format!("A,{}\n", 10 + i));
        }
        for i in 0..10 {
            csv.push_str(&format!("B,{}\n", 20 + i));
        }
        for i in 0..10 {
            csv.push_str(&format!("C,{}\n", 30 + i));
        }
        let path = std::env::temp_dir().join(format!("statscraft_p3_groups_{}.csv", tag));
        std::fs::write(&path, csv).unwrap();
        DataTable::from_csv_path(&path).unwrap()
    }

    /// A helper for the other fixtures: write a CSV to a unique file and
    /// read it back.
    fn read_csv(tag: &str, csv: &str) -> DataTable {
        let path = std::env::temp_dir().join(format!("statscraft_p3_{}.csv", tag));
        std::fs::write(&path, csv).unwrap();
        DataTable::from_csv_path(&path).unwrap()
    }

    #[test]
    fn mann_whitney_finds_a_clear_separation() {
        let t = three_groups("mw");
        let g = t.column_index("group").unwrap();
        let y = t.column_index("score").unwrap();
        let r = mann_whitney(&t, y, g).expect("result");
        // Group A is entirely below group B, so the test must be
        // significant at any conventional level.
        assert!(r.p < 0.001, "p was {}", r.p);
        assert!(r.z > 3.0, "z was {}", r.z);
        assert_eq!(r.n1, 10);
        assert_eq!(r.n2, 10);
        // The common-language effect size is the probability that a case
        // from group 1 exceeds one from group 2. Group A is entirely
        // below group B, so that probability is 0.
        assert!((r.effect_size - 0.0).abs() < 1e-9, "effect was {}", r.effect_size);
    }

    #[test]
    fn kruskal_wallis_matches_the_one_way_anova_conclusion() {
        let t = three_groups("kw");
        let g = t.column_index("group").unwrap();
        let y = t.column_index("score").unwrap();
        let r = kruskal_wallis(&t, y, g).expect("result");
        assert_eq!(r.n_groups, 3);
        // Three completely separated groups: the p-value must be tiny.
        assert!(r.p < 0.001, "p was {}", r.p);
        // The mean ranks should be 5.5, 15.5 and 25.5.
        let means: Vec<f64> = r.rank_sums.iter().map(|(_, s, n)| s / *n as f64).collect();
        assert!((means[0] - 5.5).abs() < 1e-9, "mean rank 0 was {}", means[0]);
        assert!((means[1] - 15.5).abs() < 1e-9, "mean rank 1 was {}", means[1]);
        assert!((means[2] - 25.5).abs() < 1e-9, "mean rank 2 was {}", means[2]);
    }

    #[test]
    fn the_wilcoxon_test_detects_a_consistent_shift() {
        let mut csv = String::from("before,after\n");
        for i in 0..12 {
            csv.push_str(&format!("{},{}\n", 10 + i, 20 + i));
        }
        let t = read_csv("paired", &csv);
        let a = t.column_index("before").unwrap();
        let b = t.column_index("after").unwrap();
        let r = wilcoxon_signed_rank(&t, a, b).expect("result");
        assert_eq!(r.n, 12);
        // The normal approximation gives 0.0025 here; the exact test
        // would give 0.00049. Both are clearly significant.
        assert!(r.p < 0.05, "p was {}", r.p);
        // The difference is computed as before - after, and "after" is
        // always larger here, so every difference is negative: the whole
        // rank sum falls in W- and W+ is zero. 1 + 2 + ... + 12 = 78.
        assert_eq!(r.w_plus, 0.0);
        assert!((r.w_minus - 78.0).abs() < 1e-9, "w- was {}", r.w_minus);
    }

    #[test]
    fn the_friedman_test_works_over_three_related_variables() {
        let mut csv = String::from("a,b,c\n");
        for i in 0..10 {
            csv.push_str(&format!("{},{},{}\n", i, i + 1, i + 2));
        }
        let t = read_csv("friedman", &csv);
        let idxs: Vec<usize> = ["a", "b", "c"]
            .iter()
            .map(|n| t.column_index(n).unwrap())
            .collect();
        let r = friedman(&t, &idxs).expect("result");
        assert_eq!(r.k, 3);
        assert_eq!(r.n, 10);
        assert!(r.p < 0.001, "p was {}", r.p);
    }

    #[test]
    fn the_means_procedure_groups_correctly() {
        let t = three_groups("means");
        let g = t.column_index("group").unwrap();
        let y = t.column_index("score").unwrap();
        let m = means_by_group(&t, y, g).expect("result");
        assert_eq!(m.groups.len(), 3);
        assert_eq!(m.total_n, 30);
        // Group A is 10..19, so its mean is 14.5.
        assert!((m.groups[0].mean - 14.5).abs() < 1e-9);
        assert!((m.grand_mean - 24.5).abs() < 1e-9);
    }

    #[test]
    fn explore_reports_the_descriptives_and_a_normality_test() {
        let t = three_groups("explore");
        let y = t.column_index("score").unwrap();
        let e = explore(&t, y, None).expect("result");
        assert_eq!(e.groups.len(), 1);
        let g = &e.groups[0];
        assert_eq!(g.n, 30);
        assert!((g.mean - 24.5).abs() < 1e-9);
        // The median of 10..39 is 24.5.
        assert!((g.median - 24.5).abs() < 1e-9);
        assert!((g.min - 10.0).abs() < 1e-9);
        assert!((g.max - 39.0).abs() < 1e-9);
        assert!(g.ks_stat.is_finite());
    }

    #[test]
    fn the_levene_test_detects_unequal_variances() {
        // Group A is tight, group B is spread out.
        let mut csv = String::from("group,score\n");
        for v in [10.0, 10.1, 9.9, 10.05, 9.95] {
            csv.push_str(&format!("A,{}\n", v));
        }
        for v in [0.0, 50.0, 100.0, 10.0, 90.0] {
            csv.push_str(&format!("B,{}\n", v));
        }
        let t = read_csv("levene", &csv);
        let g = t.column_index("group").unwrap();
        let y = t.column_index("score").unwrap();
        let groups = collect_groups(&t, y, g).unwrap();
        let l = levene_test(&groups).expect("result");
        assert!(l.p < 0.05, "p was {}", l.p);
    }

    #[test]
    fn the_tukey_post_hoc_finds_the_different_pair() {
        let t = three_groups("posthoc");
        let g = t.column_index("group").unwrap();
        let y = t.column_index("score").unwrap();
        let a = one_way_anova_with(&t, y, g, &PostHocSet { tukey: true, ..Default::default() }, true)
            .expect("result");
        assert_eq!(a.groups.len(), 3);
        assert!(a.levene.is_some());
        assert_eq!(a.posthoc.len(), 3);
        // Every pair is separated, so every comparison is significant.
        for ph in &a.posthoc {
            assert!(ph.p < 0.05, "{} p was {}", ph.test, ph.p);
        }
    }

    #[test]
    fn the_regression_reports_a_significant_slope() {
        let mut csv = String::from("x,y\n");
        for i in 0..20 {
            csv.push_str(&format!("{},{}\n", i, 3 * i + 5));
        }
        let t = read_csv("reg", &csv);
        let x = t.column_index("x").unwrap();
        let y = t.column_index("y").unwrap();
        let r = linear_regression_xy(&t, y, x).expect("result");
        assert!((r.slope - 3.0).abs() < 1e-9, "slope was {}", r.slope);
        assert!((r.intercept - 5.0).abs() < 1e-9, "intercept was {}", r.intercept);
        assert!((r.r2 - 1.0).abs() < 1e-9, "r2 was {}", r.r2);
        assert!(r.p < 0.001, "p was {}", r.p);
    }

    #[test]
    fn a_perfect_correlation_gets_significance_stars() {
        let mut csv = String::from("a,b\n");
        for i in 0..20 {
            csv.push_str(&format!("{},{}\n", i, i * 2));
        }
        let t = read_csv("corr", &csv);
        let a = t.column_index("a").unwrap();
        let b = t.column_index("b").unwrap();
        let c = correlations_in(&t, &[a, b]);
        assert_eq!(c.len(), 1);
        assert!((c[0].r - 1.0).abs() < 1e-12);
        assert!(c[0].p < 0.001, "p was {}", c[0].p);
    }

    #[test]
    fn the_post_hoc_p_values_are_ordered_by_conservatism() {
        // LSD is the most permissive, then Tukey, then Bonferroni, then
        // Scheffe. Their p-values must follow that order for the same
        // comparison, and all must be sensible probabilities.
        let t = three_groups("posthoc_order");
        let g = t.column_index("group").unwrap();
        let y = t.column_index("score").unwrap();
        let sel = PostHocSet {
            lsd: true,
            tukey: true,
            bonferroni: true,
            scheffe: true,
            ..Default::default()
        };
        let a = one_way_anova_with(&t, y, g, &sel, false).expect("result");
        assert!(!a.posthoc.is_empty());

        for ph in &a.posthoc {
            assert!(
                ph.p >= 0.0 && ph.p <= 1.0,
                "{} gave p={}",
                ph.test,
                ph.p
            );
        }
        // The same comparison under different tests.
        let find = |name: &str| -> f64 {
            a.posthoc
                .iter()
                .find(|p| p.test == name && p.group1 == "A" && p.group2 == "B")
                .map(|p| p.p)
                .unwrap_or(f64::NAN)
        };
        let lsd = find("LSD");
        let tukey = find("Tukey");
        let bonf = find("Bonferroni");
        let scheff = find("Scheffe");
        assert!(lsd.is_finite(), "no LSD row");
        assert!(tukey.is_finite(), "no Tukey row");
        assert!(bonf.is_finite(), "no Bonferroni row");
        assert!(scheff.is_finite(), "no Scheffe row");
        // Bonferroni is the unadjusted p multiplied by the number of
        // comparisons, so it must be at least as large as the unadjusted
        // p-value of the same comparison.
        assert!(
            bonf >= lsd - 1e-12,
            "Bonferroni {} < the unadjusted {}",
            bonf,
            lsd
        );
        // The Tukey and Scheffe p-values are computed from different
        // statistics, so no ordering between them is guaranteed - but
        // both must be larger than the raw t-test p-value, because both
        // are more conservative than an unadjusted comparison.
        assert!(
            tukey >= lsd - 1e-12,
            "Tukey {} < the unadjusted {}",
            tukey,
            lsd
        );
        // Scheffe's F statistic is the square of the t statistic, so its
        // p-value is the same as the unadjusted one - it is not a
        // *pairwise* correction, it protects against the family of all
        // possible contrasts rather than the pairs that were compared.
        assert!(
            (scheff - lsd).abs() < 1e-9,
            "Scheffe {} and the unadjusted {} should agree",
            scheff,
            lsd
        );
    }

    #[test]
    fn the_studentized_range_cdf_is_monotonic() {
        // The CDF must increase with q, or the p-value it produces is
        // meaningless.
        let k = 2.0;
        let df = 178.0;
        let mut prev = 0.0;
        for i in 1..=20 {
            let q = i as f64 * 0.5;
            let c = studentized_range_cdf(q, k, df);
            assert!(
                c >= prev - 1e-9,
                "CDF fell at q={}: {} then {}",
                q,
                prev,
                c
            );
            prev = c;
        }
        // The 95% critical value.
        let crit = studentized_range_critical(k, df, 0.05);
        let at_crit = studentized_range_cdf(crit, k, df);
        assert!(
            (at_crit - 0.95).abs() < 0.01,
            "CDF at the critical value was {}, expected 0.95",
            at_crit
        );
        // A large q must be significant.
        let far = studentized_range_cdf(crit * 2.0, k, df);
        assert!(far > 0.99, "CDF at 2x the critical value was {}", far);
    }

    #[test]
    fn the_studentized_range_matches_the_published_tukey_table() {
        // The 95% critical values of the studentized range, from the
        // standard Tukey table.
        let table: [(f64, f64, f64); 12] = [
            (2.0, 10.0, 3.151),
            (3.0, 10.0, 3.877),
            (4.0, 10.0, 4.327),
            (5.0, 10.0, 4.654),
            (2.0, 20.0, 2.950),
            (3.0, 20.0, 3.578),
            (4.0, 20.0, 3.958),
            (5.0, 20.0, 4.232),
            (3.0, 30.0, 3.486),
            (4.0, 30.0, 3.845),
            (2.0, 120.0, 2.800),
            (3.0, 120.0, 3.356),
        ];
        for (k, df, expected) in table {
            let got = studentized_range_critical(k, df, 0.05);
            assert!(
                (got - expected).abs() < 0.01,
                "k={} df={}: got {:.3}, published {:.3}",
                k,
                df,
                got,
                expected
            );
        }
    }

    #[test]
    fn the_fisher_exact_test_works_on_a_two_by_two_table() {
        let mut csv = String::from("smoke,cancer\n");
        for _ in 0..10 {
            csv.push_str("yes,yes\n");
        }
        for _ in 0..2 {
            csv.push_str("yes,no\n");
        }
        for _ in 0..3 {
            csv.push_str("no,yes\n");
        }
        for _ in 0..15 {
            csv.push_str("no,no\n");
        }
        let t = read_csv("fisher", &csv);
        let a = t.column_index("smoke").unwrap();
        let b = t.column_index("cancer").unwrap();
        let mut ct = crosstabs_chi2(&t, a, b).expect("result");
        let sel = CrosstabStats {
            fisher_exact: true,
            ..Default::default()
        };
        crosstabs_extra(&mut ct, &sel);
        let (p, odds) = ct.fisher_exact.expect("fisher");
        assert!(p < 0.05, "p was {}", p);
        assert!(odds > 1.0, "odds was {}", odds);
    }
}

/// A histogram of a numeric variable, with the counts per bin.
pub fn histogram(table: &DataTable, col_idx: usize, bins: usize) -> Option<Histogram> {
    let col = table.columns.get(col_idx)?;
    if col.kind != ColumnKind::Numeric {
        return None;
    }
    let mut vals: Vec<f64> = Vec::new();
    for row in 0..table.n_rows {
        if col.is_missing(row) {
            continue;
        }
        if let Some(Value::Number(v)) = col.values.get(row) {
            vals.push(*v);
        }
    }
    if vals.len() < 2 {
        return None;
    }
    vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let min = vals[0];
    let max = vals[vals.len() - 1];
    if (max - min).abs() < f64::EPSILON {
        return None;
    }
    let bins = bins.clamp(1, 60);
    let width = (max - min) / bins as f64;
    let mut counts = vec![0usize; bins];
    for v in &vals {
        let i = (((v - min) / width).floor() as usize).min(bins - 1);
        counts[i] += 1;
    }
    Some(Histogram {
        variable: col.name.clone(),
        min,
        max,
        width,
        centres: (0..bins)
            .map(|i| min + (i as f64 + 0.5) * width)
            .collect(),
        counts: counts.iter().map(|c| *c as f64).collect(),
    })
}

/// The bins of a histogram.
#[derive(Debug, Clone)]
pub struct Histogram {
    pub variable: String,
    pub min: f64,
    pub max: f64,
    pub width: f64,
    /// The centre of each bin.
    pub centres: Vec<f64>,
    /// The count in each bin.
    pub counts: Vec<f64>,
}

/// A bar chart of the counts of a categorical variable.
pub fn bar_chart(table: &DataTable, col_idx: usize) -> Option<BarChart> {
    let col = table.columns.get(col_idx)?;
    let mut counts: Vec<(String, usize)> = Vec::new();
    for row in 0..table.n_rows {
        let key = if col.is_missing(row) {
            "(missing)".to_string()
        } else {
            match col.values.get(row) {
                Some(Value::Number(n)) => format!("{}", n),
                Some(Value::Text(s)) => s.clone(),
                _ => "(missing)".to_string(),
            }
        };
        match counts.iter_mut().find(|(k, _)| *k == key) {
            Some(e) => e.1 += 1,
            None => counts.push((key, 1)),
        }
    }
    if counts.is_empty() {
        return None;
    }
    counts.sort_by(|a, b| b.1.cmp(&a.1));
    Some(BarChart {
        variable: col.name.clone(),
        categories: counts.iter().map(|(k, _)| k.clone()).collect(),
        counts: counts.iter().map(|(_, c)| *c as f64).collect(),
    })
}

/// The categories and counts of a bar chart.
#[derive(Debug, Clone)]
pub struct BarChart {
    pub variable: String,
    pub categories: Vec<String>,
    pub counts: Vec<f64>,
}

/// A scatter plot of two numeric variables.
pub fn scatter_points(
    table: &DataTable,
    x_idx: usize,
    y_idx: usize,
) -> Option<Scatter> {
    let cx = table.columns.get(x_idx)?;
    let cy = table.columns.get(y_idx)?;
    if cx.kind != ColumnKind::Numeric || cy.kind != ColumnKind::Numeric {
        return None;
    }
    let mut xs = Vec::new();
    let mut ys = Vec::new();
    for row in 0..table.n_rows {
        if cx.is_missing(row) || cy.is_missing(row) {
            continue;
        }
        if let (Value::Number(x), Value::Number(y)) =
            (cx.values.get(row)?, cy.values.get(row)?)
        {
            xs.push(*x);
            ys.push(*y);
        }
    }
    if xs.len() < 2 {
        return None;
    }
    Some(Scatter {
        x_var: cx.name.clone(),
        y_var: cy.name.clone(),
        x: xs,
        y: ys,
    })
}

/// The points of a scatter plot.
#[derive(Debug, Clone)]
pub struct Scatter {
    pub x_var: String,
    pub y_var: String,
    pub x: Vec<f64>,
    pub y: Vec<f64>,
}
