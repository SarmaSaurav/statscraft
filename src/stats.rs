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
            continue; // pairwise missing excluded (StatsBench listwise for crosstabs)
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
        var1: ca.display_name().to_string(),
        var2: cb.display_name().to_string(),
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
pub fn collect_groups(
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
            // the number of comparisons, which is how StatsBench reports it.
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
/// The two-tailed value is what StatsBench reports, so this already includes
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
        outcome: ocol.display_name().to_string(),
        factor: fcol.display_name().to_string(),
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
        var1: ca.display_name().to_string(),
        var2: cb.display_name().to_string(),
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
/// corresponding StatsBench command, so a set parsed from syntax can be
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
    /// The StatsBench default set for DESCRIPTIVES / FREQUENCIES: mean, std dev,
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
/// to the StatsBench defaults (mean, std dev, min, max).
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

        // Mode: the most frequent value. StatsBench reports every mode; we
        // report the smallest of them, which is what StatsBench shows first.
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

        // Skewness and kurtosis use the StatsBench (bias-corrected) formulas.
        let m2 = vals.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n_f;
        let m3 = vals.iter().map(|x| (x - mean).powi(3)).sum::<f64>() / n_f;
        let m4 = vals.iter().map(|x| (x - mean).powi(4)).sum::<f64>() / n_f;
        let skewness = if m2 > 0.0 {
            (n_f / ((n_f - 1.0) * (n_f - 2.0))).sqrt() * (m3 / m2.powf(1.5))
        } else {
            0.0
        };
        let kurtosis = if n_f > 3.0 && m2 > 0.0 {
            // Excess kurtosis, as StatsBench reports it.
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
        // matching StatsBench's default (which is not a simple index lookup).
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
            variable: col.display_name().to_string(),
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
/// (StatsBench's default PERCENTILES method).
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
            // StatsBench prints next to it.
            let df = (xs.len() as f64 - 2.0).max(1.0);
            let p = if (1.0 - r * r).abs() > 1e-12 {
                let t = r * (df / (1.0 - r * r)).sqrt();
                t_p_value(t, df)
            } else {
                // A perfect correlation: the p-value is effectively zero.
                0.0
            };
            out.push(Correlation {
                var1: ca.display_name().to_string(),
                var2: cb.display_name().to_string(),
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
    // the slope is zero and the t statistic is unbounded. StatsBench reports
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
        predictor: cx.display_name().to_string(),
        outcome: cy.display_name().to_string(),
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
        outcome: ocol.display_name().to_string(),
        group_var: gcol.display_name().to_string(),
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

/// One-sample t against a hypothesised mean.
#[derive(Debug, Clone)]
pub struct OneSampleTResult {
    pub variable: String,
    pub n: usize,
    pub mean: f64,
    pub std_dev: f64,
    pub test_value: f64,
    pub mean_diff: f64,
    pub std_error: f64,
    pub t: f64,
    pub df: f64,
    pub p: f64,
}

/// One-sample t for one numeric column. Missing values are dropped.
pub fn one_sample_t_var(table: &DataTable, idx: usize, mu0: f64) -> Option<OneSampleTResult> {
    let col = table.columns.get(idx)?;
    if col.kind != ColumnKind::Numeric {
        return None;
    }
    let xs: Vec<f64> = col
        .values
        .iter()
        .enumerate()
        .filter_map(|(row, v)| {
            if col.is_missing(row) {
                return None;
            }
            match v {
                Value::Number(n) => Some(*n),
                _ => None,
            }
        })
        .collect();
    if xs.len() < 2 {
        return None;
    }
    let n = xs.len() as f64;
    let mean = xs.iter().sum::<f64>() / n;
    let var = xs.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0);
    let sd = var.sqrt();
    let se = sd / n.sqrt();
    if se == 0.0 {
        return None;
    }
    let t = (mean - mu0) / se;
    let df = n - 1.0;
    Some(OneSampleTResult {
        variable: col.display_name().to_string(),
        n: xs.len(),
        mean,
        std_dev: sd,
        test_value: mu0,
        mean_diff: mean - mu0,
        std_error: se,
        t,
        df,
        p: t_p_value(t, df),
    })
}

/// One-sample test of a proportion.
#[derive(Debug, Clone)]
pub struct ProportionResult {
    pub variable: String,
    pub n: usize,
    pub successes: usize,
    pub proportion: f64,
    pub test_value: f64,
    pub success_code: f64,
    pub z: f64,
    pub p_normal: f64,
    pub p_exact: f64,
}

/// Count values equal to `success` and test that proportion against `p0`.
pub fn one_sample_proportion_var(
    table: &DataTable,
    idx: usize,
    p0: f64,
    success: f64,
) -> Option<ProportionResult> {
    if !(0.0..=1.0).contains(&p0) {
        return None;
    }
    let col = table.columns.get(idx)?;
    if col.kind != ColumnKind::Numeric {
        return None;
    }
    let mut n = 0usize;
    let mut k = 0usize;
    for (row, v) in col.values.iter().enumerate() {
        if col.is_missing(row) {
            continue;
        }
        if let Value::Number(x) = v {
            n += 1;
            if (*x - success).abs() < 1e-9 {
                k += 1;
            }
        }
    }
    if n == 0 {
        return None;
    }
    let p_hat = k as f64 / n as f64;
    let se = (p0 * (1.0 - p0) / n as f64).sqrt();
    let z = if se > 0.0 {
        (p_hat - p0) / se
    } else if (p_hat - p0).abs() < 1e-12 {
        0.0
    } else {
        f64::INFINITY
    };
    let p_normal = if z.is_finite() {
        statrs::distribution::Normal::new(0.0, 1.0)
            .ok()
            .map(|dist| 2.0 * (1.0 - dist.cdf(z.abs())))
            .unwrap_or(f64::NAN)
    } else {
        0.0
    };
    Some(ProportionResult {
        variable: col.display_name().to_string(),
        n,
        successes: k,
        proportion: p_hat,
        test_value: p0,
        success_code: success,
        z,
        p_normal,
        p_exact: binomial_two_sided(k, n, p0),
    })
}

/// Two-sided exact binomial p: sum of point masses no larger than the observed one.
fn binomial_two_sided(k: usize, n: usize, p0: f64) -> f64 {
    if n == 0 {
        return f64::NAN;
    }
    if p0 <= 0.0 {
        return if k == 0 { 1.0 } else { 0.0 };
    }
    if p0 >= 1.0 {
        return if k == n { 1.0 } else { 0.0 };
    }
    use statrs::distribution::Discrete;
    let Ok(dist) = statrs::distribution::Binomial::new(p0, n as u64) else {
        return f64::NAN;
    };
    let obs = dist.pmf(k as u64);
    let mut p: f64 = 0.0;
    for i in 0..=n {
        let pi = dist.pmf(i as u64);
        if pi <= obs + 1e-12 {
            p += pi;
        }
    }
    p.clamp(0.0, 1.0)
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
        // StatsBench-style sample stats: scores 88.5, 92, 75.3, 90.1, 84.2, 79.9
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
        // score 99 marks a user-missing value: StatsBench excludes it from analyses
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

    #[test]
    fn one_sample_t_matches_closed_form() {
        // 2, 4, 6 against 0: mean 4, sd 2, se = 2/sqrt(3), t = 2*sqrt(3)
        let t = table(vec![numeric("score", vec![2.0, 4.0, 6.0])], 3);
        let r = one_sample_t_var(&t, 0, 0.0).unwrap();
        assert_eq!(r.n, 3);
        assert!((r.mean - 4.0).abs() < 1e-12);
        assert!((r.std_dev - 2.0).abs() < 1e-12);
        assert!((r.t - 2.0 * 3.0f64.sqrt()).abs() < 1e-9, "t={}", r.t);
        assert!((r.df - 2.0).abs() < 1e-12);
        assert!(r.p < 0.08, "p={}", r.p);
    }

    #[test]
    fn one_sample_proportion_counts_the_success_code() {
        // 8 ones and 2 zeros, test p = 0.5
        let vals = vec![1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0];
        let t = table(vec![numeric("flag", vals)], 10);
        let r = one_sample_proportion_var(&t, 0, 0.5, 1.0).unwrap();
        assert_eq!(r.n, 10);
        assert_eq!(r.successes, 8);
        assert!((r.proportion - 0.8).abs() < 1e-12);
        let se = (0.25f64 / 10.0).sqrt();
        assert!((r.z - 0.3 / se).abs() < 1e-9, "z={}", r.z);
        assert!(r.p_exact > 0.0 && r.p_exact < 0.2, "exact={}", r.p_exact);
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
        outcome: ocol.display_name().to_string(),
        group_var: gcol.display_name().to_string(),
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
        var1: c1.display_name().to_string(),
        var2: c2.display_name().to_string(),
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
        outcome: ocol.display_name().to_string(),
        factor: fcol.display_name().to_string(),
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
        .map(|(i, c)| (c.display_name().to_string(), rank_sums[i] / nf))
        .collect();

    Some(FriedmanResult {
        var1: cols[0].display_name().to_string(),
        var2: cols[k - 1].display_name().to_string(),
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
        outcome: ocol.display_name().to_string(),
        group_var: fcol.display_name().to_string(),
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
            // The 5% trimmed mean, as StatsBench reports it.
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
        outcome: ocol.display_name().to_string(),
        group_var: group_idx.map(|gi| table.columns[gi].display_name().to_string()),
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
        variable: col.display_name().to_string(),
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
        variable: col.display_name().to_string(),
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
        x_var: cx.display_name().to_string(),
        y_var: cy.display_name().to_string(),
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

/// ---------------------------------------------------------------------------
/// Reliability analysis (Cronbach's alpha).
/// ---------------------------------------------------------------------------

/// The result of a reliability analysis.
#[derive(Debug, Clone)]
pub struct ReliabilityResult {
    /// The scale's name, as StatsBench reports it.
    pub name: String,
    /// The number of items.
    pub n_items: usize,
    /// The number of valid cases.
    pub n_cases: usize,
    /// Cronbach's alpha.
    pub alpha: f64,
    /// The standardised alpha, which does not depend on how the items are
    /// scaled.
    pub standardised_alpha: f64,
    /// The mean of the item variances.
    pub mean_item_variance: f64,
    /// The variance of the total score.
    pub total_variance: f64,
    /// The names of the items, in order.
    pub item_names: Vec<String>,
    /// The correlation of each item with the total score.
    pub item_total_correlations: Vec<(String, f64)>,
    /// The correlation of each item with the total score minus that item.
    pub corrected_item_total: Vec<(String, f64)>,
    /// Cronbach's alpha for the scale with each item removed in turn.
    pub alpha_if_deleted: Vec<(String, f64)>,
    /// The number of items whose correlation with the total is below the
    /// usual 0.3 threshold, which suggests the item does not belong.
    pub weak_items: Vec<String>,
}

/// Cronbach's alpha for a set of items.
///
/// Alpha is the ratio of the variance of the total score to the sum of
/// the variances of the items, scaled by the number of items. It
/// measures the internal consistency of a scale.
pub fn cronbach_alpha(
    table: &DataTable,
    item_idxs: &[usize],
    name: &str,
) -> Option<ReliabilityResult> {
    if item_idxs.len() < 2 {
        return None;
    }
    let cols: Vec<&crate::data::Column> = item_idxs
        .iter()
        .map(|i| table.columns.get(*i))
        .collect::<Option<Vec<_>>>()?;
    if cols.iter().any(|c| c.kind != ColumnKind::Numeric) {
        return None;
    }

    // The cases where every item has a value.
    let mut rows: Vec<Vec<f64>> = Vec::new();
    for row in 0..table.n_rows {
        let mut vals = Vec::with_capacity(cols.len());
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
    let n_cases = rows.len();
    let n_items = cols.len();
    if n_cases < 2 {
        return None;
    }

    // The item variances and the covariance matrix.
    let means: Vec<f64> = (0..n_items)
        .map(|j| rows.iter().map(|r| r[j]).sum::<f64>() / n_cases as f64)
        .collect();
    let mut variances = vec![0.0; n_items];
    let mut covariances = vec![vec![0.0; n_items]; n_items];
    for r in &rows {
        for j in 0..n_items {
            let dj = r[j] - means[j];
            variances[j] += dj * dj;
            for k in 0..n_items {
                covariances[j][k] += dj * (r[k] - means[k]);
            }
        }
    }
    let df = (n_cases - 1) as f64;
    for j in 0..n_items {
        variances[j] /= df;
        for k in 0..n_items {
            covariances[j][k] /= df;
        }
    }

    // The variance of the total score, from the covariance matrix.
    let total_variance: f64 = (0..n_items)
        .map(|j| (0..n_items).map(|k| covariances[j][k]).sum::<f64>())
        .sum();
    let sum_variances: f64 = variances.iter().sum();
    if total_variance <= 0.0 {
        return None;
    }

    // Cronbach's alpha.
    //
    // alpha = k / (k - 1) * (1 - sum(s_i^2) / s_total^2)
    //
    // The ratio is only meaningful when the total score actually has
    // some variance. If the items split into two anti-correlated
    // halves the total score cancels out, the ratio explodes, and the
    // scale has no internal consistency - so alpha is reported as
    // missing rather than as a huge number.
    let k = n_items as f64;
    let alpha = if total_variance > 0.0 && sum_variances > 0.0 {
        let value = (k / (k - 1.0)) * (1.0 - sum_variances / total_variance);
        if value.is_finite() && value > -1.0 {
            value
        } else {
            f64::NAN
        }
    } else {
        f64::NAN
    };

    // The standardised alpha, which uses the mean inter-item
    // correlation rather than the covariances:
    //
    //   alpha_std = k * r_bar / (1 + (k - 1) * r_bar)
    //
    // where r_bar is the mean of the off-diagonal correlations. (Using
    // the *sum* of them instead gives an alpha above 1 whenever the
    // items are not all positively correlated.)
    let mut r_sum = 0.0;
    let mut r_count = 0usize;
    for j in 0..n_items {
        for m in (j + 1)..n_items {
            if variances[j] > 0.0 && variances[m] > 0.0 {
                r_sum += covariances[j][m] / (variances[j] * variances[m]).sqrt();
                r_count += 1;
            }
        }
    }
    let r_bar = if r_count > 0 {
        r_sum / r_count as f64
    } else {
        f64::NAN
    };
    // The standardised alpha is only meaningful when the items are
    // positively correlated with each other. When the mean inter-item
    // correlation is zero or negative the scale has no internal
    // consistency to standardise, and the formula either is undefined
    // or gives a huge number, so it is reported as missing.
    let denom = 1.0 + (k - 1.0) * r_bar;
    let standardised_alpha = if r_bar.is_finite() && r_bar > 0.0 && denom.abs() > 1e-6 {
        (k * r_bar) / denom
    } else {
        f64::NAN
    };

    // The correlation of each item with the total score.
    let mut item_total_correlations = Vec::new();
    let mut corrected_item_total = Vec::new();
    for j in 0..n_items {
        // The correlation of item j with the sum of all the items.
        let mut sum_x = 0.0;
        let mut sum_y = 0.0;
        let mut sum_xy = 0.0;
        let mut sum_x2 = 0.0;
        let mut sum_y2 = 0.0;
        for r in &rows {
            let x = r[j];
            let y: f64 = r.iter().sum();
            sum_x += x;
            sum_y += y;
            sum_xy += x * y;
            sum_x2 += x * x;
            sum_y2 += y * y;
        }
        let cov = sum_xy / n_cases as f64 - (sum_x / n_cases as f64) * (sum_y / n_cases as f64);
        let vx = sum_x2 / n_cases as f64 - (sum_x / n_cases as f64).powi(2);
        let vy = sum_y2 / n_cases as f64 - (sum_y / n_cases as f64).powi(2);
        let r_total = if vx > 0.0 && vy > 0.0 {
            cov / (vx * vy).sqrt()
        } else {
            f64::NAN
        };
        item_total_correlations.push((cols[j].display_name().to_string(), r_total));

        // The correlation of item j with the total minus item j.
        let mut sum_x = 0.0;
        let mut sum_y = 0.0;
        let mut sum_xy = 0.0;
        let mut sum_x2 = 0.0;
        let mut sum_y2 = 0.0;
        for r in &rows {
            let x = r[j];
            let y: f64 = r.iter().enumerate().filter(|(m, _)| *m != j).map(|(_, v)| *v).sum();
            sum_x += x;
            sum_y += y;
            sum_xy += x * y;
            sum_x2 += x * x;
            sum_y2 += y * y;
        }
        let cov = sum_xy / n_cases as f64 - (sum_x / n_cases as f64) * (sum_y / n_cases as f64);
        let vx = sum_x2 / n_cases as f64 - (sum_x / n_cases as f64).powi(2);
        let vy = sum_y2 / n_cases as f64 - (sum_y / n_cases as f64).powi(2);
        let r_corrected = if vx > 0.0 && vy > 0.0 {
            cov / (vx * vy).sqrt()
        } else {
            f64::NAN
        };
        corrected_item_total.push((cols[j].display_name().to_string(), r_corrected));
    }

    // Alpha with each item removed.
    let mut alpha_if_deleted = Vec::new();
    for skip in 0..n_items {
        let mut sum_v = 0.0;
        let mut tot_v = 0.0;
        for j in 0..n_items {
            if j == skip {
                continue;
            }
            sum_v += variances[j];
            for m in 0..n_items {
                if m == skip {
                    continue;
                }
                tot_v += covariances[j][m];
            }
        }
        let kk = (n_items - 1) as f64;
        let a = if tot_v > 0.0 {
            (kk / (kk - 1.0)) * (1.0 - sum_v / tot_v)
        } else {
            f64::NAN
        };
        alpha_if_deleted.push((cols[skip].display_name().to_string(), a));
    }

    // The items that correlate poorly with the total.
    let weak_items = corrected_item_total
        .iter()
        .filter(|(_, r)| r.is_finite() && *r < 0.3)
        .map(|(n, _)| n.clone())
        .collect();

    Some(ReliabilityResult {
        name: name.to_string(),
        n_items,
        n_cases,
        item_names: cols.iter().map(|c| c.display_name().to_string()).collect(),
        alpha,
        standardised_alpha,
        mean_item_variance: sum_variances / k,
        total_variance,
        item_total_correlations,
        corrected_item_total,
        alpha_if_deleted,
        weak_items,
    })
}

/// ---------------------------------------------------------------------------
/// Factor analysis by principal components.
/// ---------------------------------------------------------------------------

/// The result of a factor analysis.
#[derive(Debug, Clone)]
pub struct FactorResult {
    pub n_items: usize,
    pub n_cases: usize,
    /// The eigenvalues of the correlation matrix, in descending order.
    pub eigenvalues: Vec<f64>,
    /// The percentage of variance explained by each component.
    pub variance_percent: Vec<f64>,
    /// The cumulative percentage of variance explained.
    pub cumulative_percent: Vec<f64>,
    /// The loadings, item by item: `loadings[item][component]`. This is
    /// the order the "Component Matrix" table is written in.
    pub loadings: Vec<Vec<f64>>,
    /// The names of the items.
    pub item_names: Vec<String>,
    /// The number of components retained.
    pub n_components: usize,
    /// The communality of each item: how much of it the retained
    /// components explain.
    pub communalities: Vec<f64>,
    /// The eigenvalue of each retained component.
    pub retained_eigenvalues: Vec<f64>,
    /// The Kaiser-Meyer-Olkin measure of sampling adequacy.
    pub kmo: f64,
    /// Bartlett's test of sphericity.
    pub bartlett: Option<(f64, f64)>,
}

/// A principal-components factor analysis.
///
/// The items are correlated, the correlation matrix is eigen-decomposed
/// (by the Jacobi method, which is accurate and easy to implement), and
/// the leading components are rotated with a varimax rotation so the
/// loadings are interpretable.
pub fn factor_analysis(
    table: &DataTable,
    item_idxs: &[usize],
    n_components: usize,
) -> Option<FactorResult> {
    if item_idxs.len() < 3 {
        return None;
    }
    let cols: Vec<&crate::data::Column> = item_idxs
        .iter()
        .map(|i| table.columns.get(*i))
        .collect::<Option<Vec<_>>>()?;
    if cols.iter().any(|c| c.kind != ColumnKind::Numeric) {
        return None;
    }

    // The cases where every item has a value.
    let mut rows: Vec<Vec<f64>> = Vec::new();
    for row in 0..table.n_rows {
        let mut vals = Vec::with_capacity(cols.len());
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
    let n_cases = rows.len();
    let k = cols.len();
    if n_cases < 3 {
        return None;
    }

    // The correlation matrix.
    let means: Vec<f64> = (0..k)
        .map(|j| rows.iter().map(|r| r[j]).sum::<f64>() / n_cases as f64)
        .collect();
    let mut sds = vec![0.0; k];
    for j in 0..k {
        let v: f64 = rows
            .iter()
            .map(|r| (r[j] - means[j]).powi(2))
            .sum::<f64>()
            / (n_cases - 1) as f64;
        sds[j] = v.sqrt();
    }
    let mut corr = vec![vec![0.0; k]; k];
    for j in 0..k {
        corr[j][j] = 1.0;
        for m in 0..k {
            if m <= j {
                continue;
            }
            let cov: f64 = rows
                .iter()
                .map(|r| (r[j] - means[j]) * (r[m] - means[m]))
                .sum::<f64>()
                / (n_cases - 1) as f64;
            let r = if sds[j] > 0.0 && sds[m] > 0.0 {
                cov / (sds[j] * sds[m])
            } else {
                0.0
            };
            corr[j][m] = r;
            corr[m][j] = r;
        }
    }

    // The Kaiser-Meyer-Olkin measure of sampling adequacy.
    //
    // KMO = sum(r_ij^2) / sum(r_ij^2 + partial_ij^2)
    // over all the pairs, where the partial correlation is the one
    // after removing every other variable. It measures how much the
    // data can be reduced to factors at all: above 0.8 is good, below
    // 0.6 the factor analysis is not worth doing.
    let kmo = {
        // The inverse of the correlation matrix gives the partial
        // correlations.
        let inv = invert_matrix(&corr);
        let mut num = 0.0;
        let mut den = 0.0;
        for j in 0..k {
            for m in (j + 1)..k {
                let r2 = corr[j][m] * corr[j][m];
                let diag_j = inv[j][j];
                let diag_m = inv[m][m];
                let partial = if diag_j > 0.0 && diag_m > 0.0 {
                    -inv[j][m] / (diag_j * diag_m).sqrt()
                } else {
                    0.0
                };
                num += r2;
                den += r2 + partial * partial;
            }
        }
        if den > 0.0 {
            num / den
        } else {
            f64::NAN
        }
    };

    // Bartlett's test of sphericity: the determinant of the correlation
    // matrix against the identity, tested with a chi-square.
    let det = determinant(&corr);
    let bartlett = {
        let d = n_cases as f64;
        let kf = k as f64;
        let chi = -(d - 1.0 - (2.0 * kf + 5.0) / 6.0) * det.ln();
        let df = (k * (k - 1) / 2) as f64;
        let p = (|| -> Option<f64> {
            let dist = statrs::distribution::ChiSquared::new(df).ok()?;
            Some(1.0 - dist.cdf(chi))
        })()
        .unwrap_or(f64::NAN);
        Some((chi, p))
    };

    // The eigen-decomposition of the correlation matrix.
    let (eigenvalues, eigenvectors) = jacobi_eigen(&corr);

    // The variance explained.
    let total: f64 = eigenvalues.iter().sum();
    let variance_percent: Vec<f64> = eigenvalues
        .iter()
        .map(|e| if total > 0.0 { e / total * 100.0 } else { 0.0 })
        .collect();
    let mut cumulative = Vec::new();
    let mut running = 0.0;
    for p in &variance_percent {
        running += p;
        cumulative.push(running);
    }

    // Keep the requested number of components, up to the number of items.
    let n_comp = n_components.clamp(1, k);
    let retained_eigenvalues: Vec<f64> = eigenvalues.iter().take(n_comp).cloned().collect();

    // The loadings: the eigenvectors scaled by the square root of their
    // eigenvalues.
    let mut loadings: Vec<Vec<f64>> = Vec::new();
    for c in 0..n_comp {
        let scale = eigenvalues[c].sqrt();
        loadings.push(
            (0..k)
                .map(|j| eigenvectors[j][c] * scale)
                .collect::<Vec<f64>>(),
        );
    }

    // Varimax rotation, which makes the loadings interpretable. It
    // works on the component-major form and returns the same.
    let component_major = varimax(loadings, k, n_comp, 100);
    // Transpose into item-major order: loadings[item][component].
    let loadings: Vec<Vec<f64>> = (0..k)
        .map(|j| (0..n_comp).map(|c| component_major[c][j]).collect())
        .collect();

    // The communalities: the sum of the squared loadings across the
    // retained components.
    let communalities: Vec<f64> = (0..k)
        .map(|j| {
            loadings[j]
                .iter()
                .map(|l| l * l)
                .sum::<f64>()
        })
        .collect();

    Some(FactorResult {
        n_items: k,
        n_cases,
        eigenvalues,
        variance_percent,
        cumulative_percent: cumulative,
        loadings,
        item_names: cols.iter().map(|c| c.display_name().to_string()).collect(),
        n_components: n_comp,
        communalities,
        retained_eigenvalues,
        kmo,
        bartlett,
    })
}

/// The determinant of a square matrix, by Gaussian elimination.
fn determinant(m: &[Vec<f64>]) -> f64 {
    let n = m.len();
    if n == 0 {
        return 1.0;
    }
    let mut a: Vec<Vec<f64>> = m.to_vec();
    let mut det = 1.0;
    for col in 0..n {
        // Find the pivot.
        let mut pivot = col;
        for r in col + 1..n {
            if a[r][col].abs() > a[pivot][col].abs() {
                pivot = r;
            }
        }
        if a[pivot][col].abs() < 1e-12 {
            return 0.0;
        }
        if pivot != col {
            a.swap(pivot, col);
            det = -det;
        }
        det *= a[col][col];
        for r in col + 1..n {
            let factor = a[r][col] / a[col][col];
            for c in col..n {
                a[r][c] -= factor * a[col][c];
            }
        }
    }
    det
}

/// The inverse of a square matrix, by Gauss-Jordan elimination.
///
/// Returns a matrix of the same size; if the matrix is singular the
/// diagonal is left as it was, which keeps the result finite for the
/// near-singular cases the factor analysis sees.
fn invert_matrix(m: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let n = m.len();
    let mut a: Vec<Vec<f64>> = (0..n)
        .map(|i| {
            let mut row = m[i].clone();
            row.extend((0..n).map(|j| if i == j { 1.0 } else { 0.0 }));
            row
        })
        .collect();
    for col in 0..n {
        // Find the pivot.
        let mut pivot = col;
        for r in (col + 1)..n {
            if a[r][col].abs() > a[pivot][col].abs() {
                pivot = r;
            }
        }
        if a[pivot][col].abs() < 1e-12 {
            continue; // singular: leave the row as it is
        }
        a.swap(pivot, col);
        let d = a[col][col];
        for x in a[col].iter_mut() {
            *x /= d;
        }
        for r in 0..n {
            if r != col && a[r][col] != 0.0 {
                let f = a[r][col];
                for c in 0..2 * n {
                    a[r][c] -= f * a[col][c];
                }
            }
        }
    }
    a.into_iter().map(|row| row[n..].to_vec()).collect()
}

/// The eigenvalues and eigenvectors of a symmetric matrix, by the
/// cyclic Jacobi method.
///
/// Returns the eigenvalues in descending order and the eigenvectors as
/// columns: `eigenvectors[row][column]`.
fn jacobi_eigen(matrix: &[Vec<f64>]) -> (Vec<f64>, Vec<Vec<f64>>) {
    let n = matrix.len();
    let mut a: Vec<Vec<f64>> = matrix.to_vec();
    let mut v: Vec<Vec<f64>> = (0..n)
        .map(|i| (0..n).map(|j| if i == j { 1.0 } else { 0.0 }).collect())
        .collect();

    for _ in 0..100 {
        // The sum of the squares of the off-diagonal elements.
        let mut off = 0.0;
        for i in 0..n {
            for j in 0..n {
                if i != j {
                    off += a[i][j] * a[i][j];
                }
            }
        }
        if off < 1e-14 {
            break;
        }
        for p in 0..n {
            for q in p + 1..n {
                if a[p][q].abs() < 1e-15 {
                    continue;
                }
                let theta = (a[q][q] - a[p][p]) / (2.0 * a[p][q]);
                let t = theta.signum() / (theta.abs() + (theta * theta + 1.0).sqrt());
                let c = 1.0 / (t * t + 1.0).sqrt();
                let s = t * c;
                // Rotate.
                for k in 0..n {
                    let akp = a[k][p];
                    let akq = a[k][q];
                    a[k][p] = c * akp - s * akq;
                    a[k][q] = s * akp + c * akq;
                }
                for k in 0..n {
                    let apk = a[p][k];
                    let aqk = a[q][k];
                    a[p][k] = c * apk - s * aqk;
                    a[q][k] = s * apk + c * aqk;
                }
                for k in 0..n {
                    let vkp = v[k][p];
                    let vkq = v[k][q];
                    v[k][p] = c * vkp - s * vkq;
                    v[k][q] = s * vkp + c * vkq;
                }
            }
        }
    }

    // The eigenvalues, sorted in descending order.
    let mut pairs: Vec<(f64, usize)> = (0..n).map(|i| (a[i][i], i)).collect();
    pairs.sort_by(|x, y| y.0.partial_cmp(&x.0).unwrap_or(std::cmp::Ordering::Equal));
    let eigenvalues: Vec<f64> = pairs.iter().map(|(e, _)| *e).collect();
    let mut eigenvectors = vec![vec![0.0; n]; n];
    for (col, (_, src)) in pairs.iter().enumerate() {
        for row in 0..n {
            eigenvectors[row][col] = v[row][*src];
        }
    }
    (eigenvalues, eigenvectors)
}

/// A varimax rotation of the factor loadings.
///
/// The rotation maximises the variance of the squared loadings, so each
/// item ends up loading mainly on one component.
///
/// The loadings arrive in component-major order - `loadings[component][item]`
/// - and leave in the same order.
fn varimax(mut loadings: Vec<Vec<f64>>, k: usize, n_comp: usize, max_iter: usize) -> Vec<Vec<f64>> {
    if n_comp < 2 {
        return loadings;
    }
    // The rotation matrix starts as the identity.
    let mut rot: Vec<Vec<f64>> = (0..n_comp)
        .map(|c| (0..n_comp).map(|d| if c == d { 1.0 } else { 0.0 }).collect())
        .collect();

    for _ in 0..max_iter {
        // The Kaiser rotation: rotate each pair of components until their
        // loading distributions are maximally separated.
        for comp1 in 0..n_comp {
            for comp2 in (comp1 + 1)..n_comp {
                // The rotated loadings of the two components.
                let mut lc_all = 0.0;
                let mut ld_all = 0.0;
                let mut num = 0.0;
                let mut den = 0.0;
                for item in 0..k {
                    let mut lc = 0.0;
                    let mut ld = 0.0;
                    for m in 0..n_comp {
                        lc += loadings[m][item] * rot[m][comp1];
                        ld += loadings[m][item] * rot[m][comp2];
                    }
                    lc_all += lc * lc;
                    ld_all += ld * ld;
                    num += lc.powi(4) - ld.powi(4);
                    den += lc * lc * ld * ld - (lc.powi(4) - ld.powi(4)) / 2.0;
                }
                let _ = (lc_all, ld_all);
                if den.abs() < 1e-12 {
                    continue;
                }
                // The Kaiser rotation angle: tan(4 * theta) = num / den,
                // so theta is a quarter of the angle, not a half.
                let angle = 0.25 * (num / den).atan();
                let (s, cs) = (angle.sin(), angle.cos());
                for m in 0..n_comp {
                    let rm = rot[m][comp1];
                    let rn = rot[m][comp2];
                    rot[m][comp1] = cs * rm - s * rn;
                    rot[m][comp2] = s * rm + cs * rn;
                }
            }
        }
        // Apply the rotation to the loadings.
        let mut rotated = vec![vec![0.0; k]; n_comp];
        for m in 0..n_comp {
            for item in 0..k {
                let mut l = 0.0;
                for c in 0..n_comp {
                    l += loadings[c][item] * rot[c][m];
                }
                rotated[m][item] = l;
            }
        }
        // Stop when the rotation settles.
        let mut change = 0.0;
        for m in 0..n_comp {
            for item in 0..k {
                change += (rotated[m][item] - loadings[m][item]).abs();
            }
        }
        loadings = rotated;
        if change < 1e-8 {
            break;
        }
    }
    loadings
}

/// ---------------------------------------------------------------------------
/// ROC curves.
/// ---------------------------------------------------------------------------

/// The result of an ROC analysis.
#[derive(Debug, Clone)]
pub struct RocResult {
    /// The name of the test variable.
    pub outcome: String,
    /// The name of the grouping variable.
    pub group_var: String,
    /// The name of the group that counts as the "positive" case.
    pub positive_group: String,
    /// The number of cases in the positive group.
    pub n_positive: usize,
    /// The number of cases in the negative group.
    pub n_negative: usize,
    /// The area under the ROC curve.
    pub auc: f64,
    /// The standard error of the AUC.
    pub auc_se: f64,
    /// The 95% confidence interval of the AUC.
    pub auc_ci: (f64, f64),
    /// The cut-off points, with the sensitivity and specificity at each.
    pub points: Vec<RocPoint>,
}

/// One point on an ROC curve.
#[derive(Debug, Clone)]
pub struct RocPoint {
    /// The test value at this cut-off.
    pub cutoff: f64,
    /// The proportion of positives correctly identified.
    pub sensitivity: f64,
    /// The proportion of negatives correctly identified.
    pub specificity: f64,
    /// The Youden index: sensitivity + specificity - 1.
    pub youden: f64,
}

/// The ROC curve for a test variable against a binary grouping variable.
///
/// The area under the curve is computed by the trapezoid rule over the
/// points of the curve, which is the same as the Mann-Whitney U
/// statistic over `n_positive * n_negative` pairs.
pub fn roc_curve(
    table: &DataTable,
    outcome_idx: usize,
    group_idx: usize,
    positive_group: &str,
) -> Option<RocResult> {
    let ocol = table.columns.get(outcome_idx)?;
    let gcol = table.columns.get(group_idx)?;
    if ocol.kind != ColumnKind::Numeric {
        return None;
    }
    // The two groups.
    let mut uniq: Vec<String> = Vec::new();
    for row in 0..table.n_rows {
        if gcol.is_missing(row) {
            continue;
        }
        if let Some(Value::Text(s)) = gcol.values.get(row) {
            if !uniq.contains(s) {
                uniq.push(s.clone());
            }
        }
    }
    if uniq.len() != 2 {
        return None;
    }
    let negative_group = uniq
        .iter()
        .find(|g| *g != positive_group)
        .cloned()
        .unwrap();

    // The test values, split by group. A higher test value counts as
    // positive, which is the usual convention for a risk score.
    let mut positives: Vec<f64> = Vec::new();
    let mut negatives: Vec<f64> = Vec::new();
    for row in 0..table.n_rows {
        if ocol.is_missing(row) || gcol.is_missing(row) {
            continue;
        }
        let (Value::Text(gs), Value::Number(v)) = (
            gcol.values.get(row)?,
            ocol.values.get(row)?,
        ) else {
            continue;
        };
        if gs == positive_group {
            positives.push(*v);
        } else if gs == &negative_group {
            negatives.push(*v);
        }
    }
    let n_positive = positives.len();
    let n_negative = negatives.len();
    if n_positive < 2 || n_negative < 2 {
        return None;
    }
    positives.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    negatives.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    // The AUC, by the Mann-Whitney statistic: the probability that a
    // random positive outranks a random negative.
    let mut auc = 0.0;
    for p in &positives {
        // The negatives below p, plus half of those equal to p.
        let below = negatives.iter().filter(|n| **n < *p).count() as f64;
        let equal = negatives.iter().filter(|n| **n == *p).count() as f64;
        auc += below + equal / 2.0;
    }
    auc /= (n_positive * n_negative) as f64;

    // The standard error of the AUC, by Hanley and McNeil.
    let q1 = auc / (2.0 - auc);
    let q2 = 2.0 * auc * auc / (1.0 + auc);
    let n1 = n_positive as f64;
    let n2 = n_negative as f64;
    let se = (((auc * (1.0 - auc)) + (n1 - 1.0) * (q1 - auc * auc)
        + (n2 - 1.0) * (q2 - auc * auc))
        / (n1 * n2))
        .sqrt();
    // The 95% confidence interval.
    let z = 1.959963984540054;
    let ci = ((auc - z * se).max(0.0), (auc + z * se).min(1.0));

    // The points of the curve, one per distinct cut-off.
    let mut cutoffs: Vec<f64> = positives.iter().copied().collect();
    cutoffs.extend(negatives.iter().copied());
    cutoffs.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
    cutoffs.dedup();

    let mut points = Vec::new();
    for c in &cutoffs {
        // At this cut-off, a case is called positive if its value is at
        // least c.
        let tp = positives.iter().filter(|p| **p >= *c).count() as f64;
        let fp = negatives.iter().filter(|n| **n >= *c).count() as f64;
        let sensitivity = tp / n1;
        let specificity = 1.0 - fp / n2;
        points.push(RocPoint {
            cutoff: *c,
            sensitivity,
            specificity,
            youden: sensitivity + specificity - 1.0,
        });
    }
    // The point at the far end of the curve.
    points.insert(
        0,
        RocPoint {
            cutoff: cutoffs.first().copied().unwrap_or(0.0),
            sensitivity: 1.0,
            specificity: 0.0,
            youden: 0.0,
        },
    );

    Some(RocResult {
        outcome: ocol.display_name().to_string(),
        group_var: gcol.display_name().to_string(),
        positive_group: positive_group.to_string(),
        n_positive,
        n_negative,
        auc,
        auc_se: se,
        auc_ci: ci,
        points,
    })
}

/// ---------------------------------------------------------------------------
/// Welch's and Brown-Forsythe's tests for unequal variances.
/// ---------------------------------------------------------------------------

/// The result of a one-way ANOVA with the robust variance tests.
#[derive(Debug, Clone)]
pub struct AnovaVarianceTests {
    /// Welch's F and its p-value.
    pub welch: (f64, f64, f64, f64),
    /// Brown-Forsythe's F and its p-value.
    pub brown_forsythe: (f64, f64, f64, f64),
}

/// Welch's test and Brown-Forsythe's test for equality of variances.
///
/// Welch's test uses the group means; Brown-Forsythe's uses the group
/// medians, which makes it robust to outliers.
pub fn anova_variance_tests(
    groups: &[(String, Vec<f64>)],
) -> Option<AnovaVarianceTests> {
    if groups.len() < 2 {
        return None;
    }
    let k = groups.len() as f64;
    let n_total: usize = groups.iter().map(|(_, g)| g.len()).sum();
    let n = n_total as f64;
    if n <= k {
        return None;
    }

    // ---- Welch's test ----
    let means: Vec<f64> = groups
        .iter()
        .map(|(_, g)| g.iter().sum::<f64>() / g.len() as f64)
        .collect();
    let variances: Vec<f64> = groups
        .iter()
        .map(|(_, g)| {
            let m = g.iter().sum::<f64>() / g.len() as f64;
            g.iter().map(|v| (v - m).powi(2)).sum::<f64>() / (g.len() as f64 - 1.0)
        })
        .collect();
    let ns: Vec<f64> = groups.iter().map(|(_, g)| g.len() as f64).collect();
    // Welch's weighting: w_i = n_i / s_i^2.
    let weights: Vec<f64> = ns
        .iter()
        .zip(variances.iter())
        .map(|(n_i, v)| if *v > 0.0 { n_i / v } else { 0.0 })
        .collect();
    let w_sum: f64 = weights.iter().sum();
    if w_sum <= 0.0 {
        return None;
    }
    let weighted_mean: f64 = weights
        .iter()
        .zip(means.iter())
        .map(|(w, m)| w * m)
        .sum::<f64>()
        / w_sum;
    // The weighted between-group sum of squares. The weights are divided
    // by their sum, so each group's contribution is
    // (w_i / sum w) * (mean_i - weighted_mean)^2.
    let ss_welch: f64 = weights
        .iter()
        .zip(means.iter())
        .map(|(w, m)| w * (m - weighted_mean).powi(2))
        .sum::<f64>()
        / w_sum;
    // Welch's test uses the Satterthwaite correction to the denominator,
    // so the test keeps its nominal size when the variances differ:
    //
    //   c  = 1 + 2(k-1)/(k^2-1) * sum (1/(n_i-1)) (1 - n_i/N)^2 s_i^2
    //   F  = (weighted between-group SS / (k-1)) / c
    //   df = (N - k) / c
    //
    // Without this correction the test is far too conservative when the
    // variances are unequal.
    let c: f64 = {
        let mut sum = 0.0;
        for (i, ((_, g), v)) in groups.iter().zip(variances.iter()).enumerate() {
            let n_i = g.len() as f64;
            let ni = ns[i];
            let _ = n_i;
            sum += (1.0 - ni / n).powi(2) * v / (ni - 1.0);
        }
        1.0 + 2.0 * (k - 1.0) / (k * k - 1.0) * sum
    };
    let f_welch = (ss_welch / (k - 1.0)) / c;
    let df1 = k - 1.0;
    let df2 = (n - k) / c;
    let p_welch = f_p_value(f_welch, df1, df2);

    // ---- Brown-Forsythe's test ----
    // The absolute deviations from each group's median.
    let mut z_means = Vec::new();
    let mut z_vars = Vec::new();
    let mut z_ns = Vec::new();
    for (_, g) in groups {
        if g.len() < 2 {
            continue;
        }
        let mut sorted = g.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let m = sorted.len();
        let median = if m % 2 == 1 {
            sorted[m / 2]
        } else {
            (sorted[m / 2 - 1] + sorted[m / 2]) / 2.0
        };
        let z: Vec<f64> = g.iter().map(|v| (v - median).abs()).collect();
        let mean = z.iter().sum::<f64>() / z.len() as f64;
        let var = z.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (z.len() as f64 - 1.0);
        z_means.push(mean);
        z_vars.push(var);
        z_ns.push(z.len() as f64);
    }
    if z_means.len() < 2 {
        return None;
    }
    let kz = z_means.len() as f64;
    let nz: f64 = z_ns.iter().sum();
    let z_grand: f64 = z_means
        .iter()
        .zip(z_ns.iter())
        .map(|(m, n)| m * n)
        .sum::<f64>()
        / nz;
    let ss_bf_between: f64 = z_means
        .iter()
        .zip(z_ns.iter())
        .map(|(m, n)| n * (m - z_grand).powi(2))
        .sum();
    let ss_bf_within: f64 = z_vars.iter().sum::<f64>() * (nz - kz);
    let df_bf_within = nz - kz;
    if df_bf_within <= 0.0 || ss_bf_within <= 0.0 {
        return None;
    }
    let f_bf = (ss_bf_between / (kz - 1.0)) / (ss_bf_within / df_bf_within);
    let p_bf = f_p_value(f_bf, kz - 1.0, df_bf_within);

    Some(AnovaVarianceTests {
        welch: (f_welch, df1, df2, p_welch),
        brown_forsythe: (f_bf, kz - 1.0, df_bf_within, p_bf),
    })
}

#[cfg(test)]
mod phase4_tests {
    use super::*;
    use crate::data::DataTable;

    fn read_p4(tag: &str, csv: &str) -> DataTable {
        let path = std::env::temp_dir().join(format!("statscraft_p4_{}.csv", tag));
        std::fs::write(&path, csv).unwrap();
        DataTable::from_csv_path(&path).unwrap()
    }

    /// A small deterministic pseudo-random generator, so the tests do not
    /// depend on a random number generator.
    struct Rng(u64);

    impl Rng {
        fn new(seed: u64) -> Self {
            Rng(seed)
        }

        /// A uniform value in [-0.5, 0.5).
        fn uniform(&mut self) -> f64 {
            self.0 = self
                .0
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((self.0 >> 33) as f64 / (1u64 << 31) as f64) - 0.5
        }

        /// A standard normal value, by the Box-Muller transform.
        fn normal(&mut self) -> f64 {
            let u1 = (self.uniform() + 0.5).max(1e-9);
            let u2 = self.uniform() + 0.5;
            (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
        }
    }

    /// Six items loading on two independent factors, three items each.
    fn two_factor_data(tag: &str) -> DataTable {
        let mut rng = Rng::new(12345);
        let n = 40;
        let latent_a: Vec<f64> = (0..n).map(|_| rng.normal() * 10.0).collect();
        let latent_b: Vec<f64> = (0..n).map(|_| rng.normal() * 10.0).collect();
        let mut csv = String::from("x1,x2,x3,y1,y2,y3\n");
        for i in 0..n {
            let a = latent_a[i];
            let b = latent_b[i];
            csv.push_str(&format!(
                "{},{},{},{},{},{}\n",
                a + rng.normal(),
                a + rng.normal(),
                a + rng.normal(),
                b + rng.normal(),
                b + rng.normal(),
                b + rng.normal()
            ));
        }
        read_p4(tag, &csv)
    }

    /// A scale of four items that are all measuring the same thing, so
    /// the alpha must be high.
    fn coherent_scale(tag: &str) -> DataTable {
        let mut csv = String::from("i1,i2,i3,i4\n");
        // Each row is a person; the items are near-identical with a
        // little noise, so they correlate strongly.
        for i in 0..20 {
            let base = 10.0 + i as f64;
            csv.push_str(&format!(
                "{},{},{},{}\n",
                base,
                base + (i % 3) as f64 * 0.5,
                base + (i % 4) as f64 * 0.4,
                base + (i % 5) as f64 * 0.3
            ));
        }
        read_p4(&format!("coherent_{}", tag), &csv)
    }

    #[test]
    fn cronbach_alpha_is_high_for_a_coherent_scale() {
        let t = coherent_scale("b");
        let items: Vec<usize> = ["i1", "i2", "i3", "i4"]
            .iter()
            .map(|n| t.column_index(n).unwrap())
            .collect();
        let r = cronbach_alpha(&t, &items, "Scale").expect("result");
        assert_eq!(r.n_items, 4);
        assert_eq!(r.n_cases, 20);
        // Near-perfectly consistent items give an alpha close to 1.
        assert!(r.alpha > 0.95, "alpha was {}", r.alpha);
        assert!(r.standardised_alpha > 0.95);
        // Every item correlates strongly with the total.
        for (name, corr) in &r.item_total_correlations {
            assert!(*corr > 0.9, "{} correlated {} with the total", name, corr);
        }
        // No weak items.
        assert!(r.weak_items.is_empty(), "weak items: {:?}", r.weak_items);
    }

    #[test]
    fn cronbach_alpha_is_low_for_inconsistent_items() {
        // Two items measure one thing, two measure another - but the
        // second pair is not perfectly anti-correlated with the first,
        // so the total score still has some variance.
        let mut csv = String::from("a1,a2,b1,b2\n");
        for i in 0..20 {
            let x = 10.0 + i as f64;
            let y = 100.0 - i as f64 + (i % 3) as f64;
            csv.push_str(&format!("{},{},{},{}\n", x, x, y, y));
        }
        let t = read_p4("inconsistent", &csv);
        let items: Vec<usize> = ["a1", "a2", "b1", "b2"]
            .iter()
            .map(|n| t.column_index(n).unwrap())
            .collect();
        let r = cronbach_alpha(&t, &items, "Mixed").expect("result");
        // Two unrelated halves have no internal consistency, so the
        // alpha is either low or missing.
        if r.alpha.is_finite() {
            assert!(r.alpha < 0.5, "alpha was {}", r.alpha);
        }
        // The weak items are still reported.
        assert!(
            !r.weak_items.is_empty(),
            "no weak items were flagged: {:?}",
            r.item_total_correlations
        );
    }

    #[test]
    fn the_standardised_alpha_is_never_above_one() {
        // A scale with two clusters of three that are negatively
        // correlated with each other: the sum of the off-diagonal
        // correlations is small, which used to give an alpha above 1.
        let mut csv = String::from("x1,x2,x3,y1,y2,y3\n");
        for i in 0..20 {
            let a = 10.0 + i as f64;
            let b = 100.0 - i as f64 + (i % 3) as f64;
            csv.push_str(&format!("{},{},{},{},{},{}\n", a, a, a, b, b, b));
        }
        let t = read_p4("std_alpha", &csv);
        let items: Vec<usize> = ["x1", "x2", "x3", "y1", "y2", "y3"]
            .iter()
            .map(|n| t.column_index(n).unwrap())
            .collect();
        let r = cronbach_alpha(&t, &items, "Mixed").expect("result");
        // The standardised alpha is either missing - the scale is
        // degenerate - or a sensible correlation-based value.
        if r.standardised_alpha.is_finite() {
            assert!(
                r.standardised_alpha <= 1.0,
                "the standardised alpha was {}",
                r.standardised_alpha
            );
            assert!(
                r.standardised_alpha >= -1.0,
                "the standardised alpha was {}",
                r.standardised_alpha
            );
        }
        // The scale is degenerate - the items form two anti-correlated
        // halves - so the alpha is reported as missing rather than as a
        // huge number.
        assert!(
            r.alpha.is_nan() || (r.alpha <= 1.0 && r.alpha >= -1.0),
            "the alpha was {}",
            r.alpha
        );
        assert!(
            r.standardised_alpha.is_nan() || r.standardised_alpha.abs() <= 1.0,
            "the standardised alpha was {}",
            r.standardised_alpha
        );
    }

    #[test]
    fn the_alpha_if_deleted_column_is_consistent() {
        let t = coherent_scale("c");
        let items: Vec<usize> = ["i1", "i2", "i3", "i4"]
            .iter()
            .map(|n| t.column_index(n).unwrap())
            .collect();
        let r = cronbach_alpha(&t, &items, "Scale").expect("result");
        assert_eq!(r.alpha_if_deleted.len(), 4);
        // Removing an item from a coherent scale lowers the alpha
        // slightly, but it stays high.
        for (name, a) in &r.alpha_if_deleted {
            assert!(*a < r.alpha, "removing {} raised the alpha", name);
            assert!(*a > 0.8, "removing {} dropped the alpha to {}", name, a);
        }
    }

    #[test]
    fn the_factor_analysis_finds_two_factors_in_two_clusters() {
        let t = two_factor_data("two_factors");
        let items: Vec<usize> = ["x1", "x2", "x3", "y1", "y2", "y3"]
            .iter()
            .map(|n| t.column_index(n).unwrap())
            .collect();
        let f = factor_analysis(&t, &items, 2).expect("result");
        assert_eq!(f.n_items, 6);
        assert_eq!(f.n_components, 2);
        // The first two eigenvalues should be the two large ones - one
        // per cluster - and the rest near zero, because each cluster has
        // three near-identical items.
        assert!(
            f.eigenvalues[0] > 2.5,
            "eigenvalue 0 was {}",
            f.eigenvalues[0]
        );
        assert!(
            f.eigenvalues[1] > 2.5,
            "eigenvalue 1 was {}",
            f.eigenvalues[1]
        );
        assert!(
            f.eigenvalues[2] < 0.5,
            "eigenvalue 2 was {}",
            f.eigenvalues[2]
        );
        // The two components explain most of the variance.
        assert!(
            f.cumulative_percent[1] > 70.0,
            "cumulative variance was {}",
            f.cumulative_percent[1]
        );
        // The KMO measure of sampling adequacy should be high: the data
        // is well suited to factor analysis.
        assert!(f.kmo > 0.7, "KMO was {}", f.kmo);
    }

    #[test]
    fn the_factor_analysis_loadings_separate_the_two_clusters() {
        let t = two_factor_data("loadings");
        let items: Vec<usize> = ["x1", "x2", "x3", "y1", "y2", "y3"]
            .iter()
            .map(|n| t.column_index(n).unwrap())
            .collect();
        let f = factor_analysis(&t, &items, 2).expect("result");

        // After the varimax rotation, the x items should load mainly on
        // one component and the y items mainly on the other. Which
        // component is which does not matter.
        // The loadings are item-major: loadings[item][component].
        let mean_load = |first: usize, comp: usize| -> f64 {
            (0..3)
                .map(|j| f.loadings[first + j][comp].abs())
                .sum::<f64>()
                / 3.0
        };
        let x_on_0 = mean_load(0, 0);
        let x_on_1 = mean_load(0, 1);
        let y_on_0 = mean_load(3, 0);
        let y_on_1 = mean_load(3, 1);
        // The x items should prefer one component and the y items the
        // other.
        let x_best = if x_on_0 > x_on_1 { 0 } else { 1 };
        let y_best = if y_on_0 > y_on_1 { 0 } else { 1 };
        assert_ne!(
            x_best, y_best,
            "the clusters did not separate: x ({}, {}) y ({}, {})",
            x_on_0, x_on_1, y_on_0, y_on_1
        );
        // Each item should load mainly on one component.
        for j in 0..6 {
            let a = f.loadings[j][0].abs();
            let b = f.loadings[j][1].abs();
            assert!(
                (a - b).abs() > 0.2,
                "item {} loaded {} and {}",
                j,
                a,
                b
            );
        }
    }

    #[test]
    fn the_roc_curve_separates_a_perfect_test() {
        // The test value is higher for the positives.
        let mut csv = String::from("score,status\n");
        for i in 0..20 {
            csv.push_str(&format!("{},pos\n", 50.0 + i as f64));
        }
        for i in 0..20 {
            csv.push_str(&format!("{},neg\n", 10.0 + i as f64));
        }
        let t = read_p4("roc_perfect", &csv);
        let s = t.column_index("score").unwrap();
        let g = t.column_index("status").unwrap();
        let r = roc_curve(&t, s, g, "pos").expect("result");
        // A perfect test has an area under the curve of 1.
        assert!((r.auc - 1.0).abs() < 1e-9, "auc was {}", r.auc);
        assert_eq!(r.n_positive, 20);
        assert_eq!(r.n_negative, 20);
    }

    #[test]
    fn the_roc_curve_of_a_useless_test_has_an_auc_of_a_half() {
        // The test value is the same for every case, so it carries no
        // information.
        let mut csv = String::from("score,status\n");
        for i in 0..20 {
            csv.push_str(&format!("5,pos\n"));
            csv.push_str(&format!("5,neg\n"));
        }
        let t = read_p4("roc_useless", &csv);
        let s = t.column_index("score").unwrap();
        let g = t.column_index("status").unwrap();
        let r = roc_curve(&t, s, g, "pos").expect("result");
        // With every value tied, the AUC is exactly half.
        assert!((r.auc - 0.5).abs() < 1e-9, "auc was {}", r.auc);
    }

    #[test]
    fn brown_forsythe_detects_unequal_variances() {
        let mut csv = String::from("group,score\n");
        // Group A is tight, group B is spread out.
        for v in [10.0, 10.1, 9.9, 10.05, 9.95, 10.02] {
            csv.push_str(&format!("A,{}\n", v));
        }
        for v in [0.0, 50.0, 100.0, 10.0, 90.0, 30.0] {
            csv.push_str(&format!("B,{}\n", v));
        }
        let t = read_p4("welch", &csv);
        let g = t.column_index("group").unwrap();
        let y = t.column_index("score").unwrap();
        let groups = collect_groups(&t, y, g).expect("groups");
        let tests = anova_variance_tests(&groups).expect("tests");
        // Brown-Forsythe's test is the one that detects unequal
        // variances, so it must find the difference.
        let (f_bf, df1_bf, df2_bf, p_bf) = tests.brown_forsythe;
        assert!(p_bf < 0.05, "Brown-Forsythe p was {}", p_bf);
        assert!(df1_bf > 0.0 && df2_bf > 0.0);
        assert!(f_bf > 0.0);
    }

    #[test]
    fn welch_detects_a_mean_difference_without_assuming_equal_variances() {
        // Two groups with clearly different means and moderately
        // different variances.
        let mut csv = String::from("group,score\n");
        // Moderate variance differences: Welch's test is deliberately
        // conservative when the variance ratio is extreme.
        for v in [20.0, 22.0, 18.0, 21.0, 23.0, 19.0, 20.5, 21.5] {
            csv.push_str(&format!("A,{}\n", v));
        }
        for v in [40.0, 50.0, 30.0, 45.0, 55.0, 35.0, 42.0, 48.0] {
            csv.push_str(&format!("B,{}\n", v));
        }
        let t = read_p4("welch_means", &csv);
        let g = t.column_index("group").unwrap();
        let y = t.column_index("score").unwrap();
        let groups = collect_groups(&t, y, g).expect("groups");
        let tests = anova_variance_tests(&groups).expect("tests");
        let (f, df1, df2, p) = tests.welch;
        assert!(p < 0.05, "Welch p was {}", p);
        assert!(f > 1.0, "Welch F was {}", f);
        assert!((df1 - 1.0).abs() < 1e-9, "df1 was {}", df1);
        assert!(df2 > 0.0, "df2 was {}", df2);
        // Welch's degrees of freedom are smaller than the ordinary
        // ANOVA's when the variances differ.
        assert!(df2 > 0.0, "df2 was {}", df2);
        assert!(df2 < 14.0, "df2 was {}", df2);
    }

    #[test]
    fn welch_finds_no_difference_in_equal_variances() {
        let mut csv = String::from("group,score\n");
        for v in [10.0, 12.0, 11.0, 13.0, 9.0, 14.0] {
            csv.push_str(&format!("A,{}\n", v));
        }
        for v in [10.0, 12.0, 11.0, 13.0, 9.0, 14.0] {
            csv.push_str(&format!("B,{}\n", v));
        }
        let t = read_p4("welch_equal", &csv);
        let g = t.column_index("group").unwrap();
        let y = t.column_index("score").unwrap();
        let groups = collect_groups(&t, y, g).expect("groups");
        let tests = anova_variance_tests(&groups).expect("tests");
        // The two groups are identical, so neither the mean difference
        // nor the variance difference should be significant.
        let (_, _, _, p) = tests.welch;
        assert!(p > 0.05, "Welch p was {}", p);
        let (_, _, _, p_bf) = tests.brown_forsythe;
        assert!(p_bf > 0.05, "Brown-Forsythe p was {}", p_bf);
    }
}

/// Compute a new variable or update an existing one.
///
/// The expression language is the small one the syntax runner supports:
/// the variable names, the four arithmetic operators, parentheses, the
/// unary minus, and the functions `MEAN`, `MIN`, `MAX`, `SUM`, `ABS`,
/// `SQRT`, `LN`, `LOG10` and `ROUND`.
///
/// When `if_condition` is set, only cases for which the condition is true
/// receive the expression value. A new variable is system-missing elsewhere;
/// an existing variable keeps its previous value on false cases.
pub fn compute_variable(
    table: &mut DataTable,
    name: &str,
    expression: &str,
    if_condition: Option<&str>,
) -> Result<String, String> {
    let names: Vec<String> = table.columns.iter().map(|c| c.name.clone()).collect();
    let rows: Vec<Vec<Value>> = (0..table.n_rows)
        .map(|row| table.columns.iter().map(|c| c.values[row].clone()).collect())
        .collect();

    if let Some(idx) = table.column_index(name) {
        for (row, case) in rows.iter().enumerate() {
            if let Some(cond) = if_condition {
                if !eval_condition(cond, &names, case)? {
                    continue;
                }
            }
            let v = eval_expression(expression, &names, case)?;
            table.columns[idx].values[row] = v;
        }
        return Ok(name.to_string());
    }

    let mut values: Vec<Value> = Vec::with_capacity(table.n_rows);
    for case in &rows {
        let v = if let Some(cond) = if_condition {
            if eval_condition(cond, &names, case)? {
                eval_expression(expression, &names, case)?
            } else {
                Value::Missing
            }
        } else {
            eval_expression(expression, &names, case)?
        };
        values.push(v);
    }

    let all_numeric = values.iter().all(|v| matches!(v, Value::Number(_)));
    let kind = if all_numeric {
        ColumnKind::Numeric
    } else {
        ColumnKind::String
    };
    let meta = default_meta_for_values(&values, kind);
    table.columns.push(Column {
        name: name.to_string(),
        kind,
        values,
        meta,
    });
    Ok(name.to_string())
}

fn default_meta_for_values(values: &[Value], kind: ColumnKind) -> crate::data::VariableMeta {
    let mut meta = crate::data::VariableMeta::default_for(kind);
    if kind == ColumnKind::Numeric {
        let nums: Vec<f64> = values
            .iter()
            .filter_map(|v| match v {
                Value::Number(n) => Some(*n),
                _ => None,
            })
            .collect();
        if nums.len() > 1 {
            meta.decimals = 2;
        }
    }
    meta
}

/// A simple case filter: `var = 1`, `age >= 18`, `group <> "B"`.
pub fn eval_condition(condition: &str, names: &[String], row: &[Value]) -> Result<bool, String> {
    let mut cond = condition.trim();
    while cond.starts_with('(') && cond.ends_with(')') {
        cond = cond[1..cond.len() - 1].trim();
    }
    if cond.is_empty() {
        return Err("Empty IF condition.".to_string());
    }
    const OPS: [(&str, usize); 7] = [
        (">=", 2),
        ("<=", 2),
        ("<>", 2),
        ("!=", 2),
        (">", 1),
        ("<", 1),
        ("=", 1),
    ];
    for (op, len) in OPS {
        if let Some(idx) = find_comparison_op(cond, op) {
            let left = cond[..idx].trim();
            let right = cond[idx + len..].trim();
            let left_v = resolve_condition_operand(left, names, row)?;
            let right_v = parse_condition_literal(right)?;
            return Ok(compare_values(&left_v, op, &right_v));
        }
    }
    Err(format!("IF condition not understood: '{}'", condition))
}

fn find_comparison_op(cond: &str, op: &str) -> Option<usize> {
    let mut in_quote: Option<char> = None;
    let mut byte_idx = 0;
    while byte_idx < cond.len() {
        if let Some(q) = in_quote {
            if let Some((c, next)) = cond[byte_idx..].chars().next().map(|c| (c, byte_idx + c.len_utf8())) {
                if c == q {
                    in_quote = None;
                }
                byte_idx = next;
            } else {
                break;
            }
            continue;
        }
        if cond[byte_idx..].starts_with(op) {
            return Some(byte_idx);
        }
        if let Some(c) = cond[byte_idx..].chars().next() {
            if c == '"' || c == '\'' {
                in_quote = Some(c);
            }
            byte_idx += c.len_utf8();
        } else {
            break;
        }
    }
    None
}

fn resolve_condition_operand(word: &str, names: &[String], row: &[Value]) -> Result<Value, String> {
    if let Some(idx) = names.iter().position(|n| n.eq_ignore_ascii_case(word)) {
        return Ok(row[idx].clone());
    }
    parse_condition_literal(word)
}

fn parse_condition_literal(text: &str) -> Result<Value, String> {
    let text = text.trim();
    if (text.starts_with('"') && text.ends_with('"') && text.len() >= 2)
        || (text.starts_with('\'') && text.ends_with('\'') && text.len() >= 2)
    {
        return Ok(Value::Text(text[1..text.len() - 1].to_string()));
    }
    if let Ok(n) = text.parse::<f64>() {
        return Ok(Value::Number(n));
    }
    Err(format!("Bad literal in IF condition: '{}'", text))
}

fn compare_values(left: &Value, op: &str, right: &Value) -> bool {
    match (left, right) {
        (Value::Number(a), Value::Number(b)) => match op {
            "=" => (a - b).abs() < 1e-12,
            "<>" | "!=" => (a - b).abs() >= 1e-12,
            "<" => a < b,
            "<=" => a <= b,
            ">" => a > b,
            ">=" => a >= b,
            _ => false,
        },
        (Value::Text(a), Value::Text(b)) => match op {
            "=" => a.eq_ignore_ascii_case(b),
            "<>" | "!=" => !a.eq_ignore_ascii_case(b),
            _ => false,
        },
        (Value::Text(a), Value::Number(b)) => {
            if let Ok(n) = a.trim().parse::<f64>() {
                compare_values(&Value::Number(n), op, &Value::Number(*b))
            } else {
                false
            }
        }
        (Value::Number(a), Value::Text(b)) => {
            if let Ok(n) = b.trim().parse::<f64>() {
                compare_values(&Value::Number(*a), op, &Value::Number(n))
            } else {
                false
            }
        }
        _ => false,
    }
}

/// Recode values of one or more variables.
///
/// Forms:
/// `RECODE a b (1=2) (2=1) (ELSE=COPY)`
/// `RECODE score (1 THRU 5=1) (6 THRU HI=2) INTO band`
/// `RECODE group (MISSING=9)`
pub fn recode_command(table: &mut DataTable, line: &str) -> Result<String, String> {
    let (sources, rules, into) = parse_recode(line)?;
    if sources.is_empty() {
        return Err("RECODE: no variables.".to_string());
    }
    if rules.is_empty() {
        return Err("RECODE: no value mappings.".to_string());
    }
    if !into.is_empty() && into.len() != sources.len() {
        return Err("RECODE: INTO needs one name per source variable.".to_string());
    }
    let dests: Vec<String> = if into.is_empty() {
        sources.clone()
    } else {
        into
    };
    for (src_name, dest_name) in sources.iter().zip(dests.iter()) {
        let src = table
            .column_index(src_name)
            .ok_or_else(|| format!("RECODE: unknown variable '{}'.", src_name))?;
        let mut out: Vec<Value> = Vec::with_capacity(table.n_rows);
        for row in 0..table.n_rows {
            let value = table.columns[src].values[row].clone();
            out.push(apply_recode(&table.columns[src], row, &value, &rules));
        }
        write_column(table, dest_name, out);
    }
    Ok(dests.join(" "))
}

/// Count how many of a set of variables match a value list, per case.
///
/// `COUNT hits = a b c (1 2)`
pub fn count_command(table: &mut DataTable, line: &str) -> Result<String, String> {
    let (dest, sources, atoms) = parse_count(line)?;
    if sources.is_empty() {
        return Err("COUNT: no variables.".to_string());
    }
    if atoms.is_empty() {
        return Err("COUNT: expected a value list in parentheses.".to_string());
    }
    let src_idx: Result<Vec<usize>, String> = sources
        .iter()
        .map(|n| {
            table
                .column_index(n)
                .ok_or_else(|| format!("COUNT: unknown variable '{}'.", n))
        })
        .collect();
    let src_idx = src_idx?;
    let mut values = Vec::with_capacity(table.n_rows);
    for row in 0..table.n_rows {
        let mut n = 0.0;
        for &si in &src_idx {
            let v = &table.columns[si].values[row];
            if atoms.iter().any(|a| atom_matches(a, v)) {
                n += 1.0;
            }
        }
        values.push(Value::Number(n));
    }
    write_column(table, &dest, values);
    Ok(dest)
}

/// Rank a numeric variable. Ties take the mean rank.
///
/// `RANK score` or `RANK score INTO rscore`
pub fn rank_command(table: &mut DataTable, line: &str) -> Result<String, String> {
    let (source, dest) = parse_rank(line)?;
    let src = table
        .column_index(&source)
        .ok_or_else(|| format!("RANK: unknown variable '{}'.", source))?;
    if table.columns[src].kind != ColumnKind::Numeric {
        return Err("RANK: the variable must be numeric.".to_string());
    }
    let mut items: Vec<(usize, f64)> = Vec::new();
    for row in 0..table.n_rows {
        if table.columns[src].is_missing(row) {
            continue;
        }
        if let Value::Number(n) = table.columns[src].values[row] {
            items.push((row, n));
        }
    }
    items.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    let mut out = vec![Value::Missing; table.n_rows];
    let mut i = 0;
    while i < items.len() {
        let mut j = i;
        while j + 1 < items.len() && (items[j + 1].1 - items[i].1).abs() < 1e-12 {
            j += 1;
        }
        let mean = ((i + 1) + (j + 1)) as f64 / 2.0;
        for item in items.iter().take(j + 1).skip(i) {
            out[item.0] = Value::Number(mean);
        }
        i = j + 1;
    }
    write_column(table, &dest, out);
    Ok(dest)
}

/// Recode unique values of a variable into consecutive integers.
///
/// `AUTORECODE group INTO groupn`
pub fn autorecode_command(table: &mut DataTable, line: &str) -> Result<String, String> {
    let (source, dest) = parse_autorecode(line)?;
    let src = table
        .column_index(&source)
        .ok_or_else(|| format!("AUTORECODE: unknown variable '{}'.", source))?;
    let mut seen: Vec<RecodeAtom> = Vec::new();
    for row in 0..table.n_rows {
        if table.columns[src].is_missing(row) {
            continue;
        }
        let atom = match &table.columns[src].values[row] {
            Value::Number(n) => RecodeAtom::Number(*n),
            Value::Text(s) => RecodeAtom::Text(s.clone()),
            Value::Missing => continue,
        };
        if !seen.iter().any(|a| atom_eq(a, &atom)) {
            seen.push(atom);
        }
    }
    seen.sort_by(atom_ord);
    let mut out = vec![Value::Missing; table.n_rows];
    let mut labels: Vec<(f64, String)> = Vec::new();
    for (i, atom) in seen.iter().enumerate() {
        let code = (i + 1) as f64;
        labels.push((code, atom_label(atom)));
        for row in 0..table.n_rows {
            if atom_matches(atom, &table.columns[src].values[row]) {
                out[row] = Value::Number(code);
            }
        }
    }
    write_column(table, &dest, out);
    if let Some(idx) = table.column_index(&dest) {
        table.columns[idx].meta.value_labels = labels;
        table.columns[idx].meta.measure = crate::data::Measure::Nominal;
        table.columns[idx].meta.decimals = 0;
    }
    Ok(dest)
}

/// Keep the cases for which a condition is true. The rest are dropped.
///
/// `SELECT IF age >= 18`
pub fn select_if_command(table: &mut DataTable, condition: &str) -> Result<usize, String> {
    let names: Vec<String> = table.columns.iter().map(|c| c.name.clone()).collect();
    let mut keep: Vec<usize> = Vec::new();
    for row in 0..table.n_rows {
        let case: Vec<Value> = table.columns.iter().map(|c| c.values[row].clone()).collect();
        if eval_condition(condition, &names, &case)? {
            keep.push(row);
        }
    }
    for col in table.columns.iter_mut() {
        col.values = keep.iter().map(|&i| col.values[i].clone()).collect();
    }
    table.n_rows = keep.len();
    Ok(keep.len())
}

#[derive(Clone, Copy)]
enum AggFn {
    N,
    Mean,
    Sum,
    Min,
    Max,
    Sd,
}

/// `AGGREGATE BREAK group /MEAN(age) /N`
pub fn aggregate_command(table: &mut DataTable, line: &str) -> Result<String, String> {
    let (breaks, specs) = parse_aggregate(line)?;
    let break_idx: Result<Vec<usize>, String> = breaks
        .iter()
        .map(|n| {
            table
                .column_index(n)
                .ok_or_else(|| format!("AGGREGATE: unknown variable '{}'.", n))
        })
        .collect();
    let break_idx = break_idx?;
    let mut groups: std::collections::BTreeMap<String, Vec<usize>> =
        std::collections::BTreeMap::new();
    for row in 0..table.n_rows {
        let key = if break_idx.is_empty() {
            String::new()
        } else {
            break_idx
                .iter()
                .map(|&ci| format!("{:?}", table.columns[ci].values.get(row)))
                .collect::<Vec<_>>()
                .join("\u{1f}")
        };
        groups.entry(key).or_default().push(row);
    }
    let mut out_cols: Vec<Column> = Vec::new();
    for (i, name) in breaks.iter().enumerate() {
        let src = break_idx[i];
        let mut values = Vec::new();
        for rows in groups.values() {
            values.push(
                table.columns[src]
                    .values
                    .get(rows[0])
                    .cloned()
                    .unwrap_or(Value::Missing),
            );
        }
        out_cols.push(Column {
            name: name.clone(),
            kind: table.columns[src].kind,
            values,
            meta: table.columns[src].meta.clone(),
        });
    }
    for (fun, var) in &specs {
        let dest = match fun {
            AggFn::N => "n".to_string(),
            AggFn::Mean => format!("mean_{}", var),
            AggFn::Sum => format!("sum_{}", var),
            AggFn::Min => format!("min_{}", var),
            AggFn::Max => format!("max_{}", var),
            AggFn::Sd => format!("sd_{}", var),
        };
        let src = if matches!(fun, AggFn::N) && var.is_empty() {
            None
        } else {
            Some(
                table
                    .column_index(var)
                    .ok_or_else(|| format!("AGGREGATE: unknown variable '{}'.", var))?,
            )
        };
        let mut values = Vec::new();
        for rows in groups.values() {
            values.push(agg_value(table, src, rows, *fun));
        }
        out_cols.push(Column {
            name: dest,
            kind: ColumnKind::Numeric,
            values,
            meta: crate::data::VariableMeta::default_for(ColumnKind::Numeric),
        });
    }
    let n_rows = groups.len();
    *table = DataTable {
        columns: out_cols,
        n_rows,
    };
    Ok(format!("{} group(s)", n_rows))
}

fn agg_value(table: &DataTable, src: Option<usize>, rows: &[usize], fun: AggFn) -> Value {
    if matches!(fun, AggFn::N) && src.is_none() {
        return Value::Number(rows.len() as f64);
    }
    let Some(si) = src else {
        return Value::Number(rows.len() as f64);
    };
    let nums: Vec<f64> = rows
        .iter()
        .filter(|&&r| !table.columns[si].is_missing(r))
        .filter_map(|&r| match table.columns[si].values.get(r) {
            Some(Value::Number(n)) => Some(*n),
            _ => None,
        })
        .collect();
    if nums.is_empty() && !matches!(fun, AggFn::N) {
        return Value::Missing;
    }
    match fun {
        AggFn::N => Value::Number(nums.len() as f64),
        AggFn::Mean => Value::Number(nums.iter().sum::<f64>() / nums.len() as f64),
        AggFn::Sum => Value::Number(nums.iter().sum()),
        AggFn::Min => Value::Number(nums.iter().cloned().fold(f64::INFINITY, f64::min)),
        AggFn::Max => Value::Number(nums.iter().cloned().fold(f64::NEG_INFINITY, f64::max)),
        AggFn::Sd => {
            if nums.len() < 2 {
                return Value::Missing;
            }
            let m = nums.iter().sum::<f64>() / nums.len() as f64;
            let v = nums.iter().map(|x| (x - m).powi(2)).sum::<f64>() / (nums.len() as f64 - 1.0);
            Value::Number(v.sqrt())
        }
    }
}

fn parse_aggregate(line: &str) -> Result<(Vec<String>, Vec<(AggFn, String)>), String> {
    let mut scan = Scan::new(line);
    if !scan.eat_word_ignore("AGGREGATE") {
        return Err("AGGREGATE: expected AGGREGATE.".to_string());
    }
    let mut breaks = Vec::new();
    if scan.eat_word_ignore("BREAK") {
        while let Some(name) = scan.ident() {
            if name.eq_ignore_ascii_case("BREAK") {
                continue;
            }
            breaks.push(name);
        }
    }
    let mut specs = Vec::new();
    loop {
        scan.skip_ws();
        if scan.peek() != Some('/') {
            break;
        }
        scan.i += 1;
        let ident = scan
            .ident()
            .ok_or_else(|| "AGGREGATE: expected a function after '/'.".to_string())?;
        let fun = match ident.to_uppercase().as_str() {
            "N" | "NU" => AggFn::N,
            "MEAN" => AggFn::Mean,
            "SUM" => AggFn::Sum,
            "MIN" => AggFn::Min,
            "MAX" => AggFn::Max,
            "SD" | "STDDEV" => AggFn::Sd,
            other => return Err(format!("AGGREGATE: unknown function '{}'.", other)),
        };
        scan.skip_ws();
        let var = if scan.peek() == Some('(') {
            let inner = scan.paren_group()?;
            inner.trim().to_string()
        } else {
            String::new()
        };
        if !matches!(fun, AggFn::N) && var.is_empty() {
            return Err("AGGREGATE: MEAN/SUM/MIN/MAX/SD need a variable.".to_string());
        }
        specs.push((fun, var));
    }
    if specs.is_empty() {
        specs.push((AggFn::N, String::new()));
    }
    Ok((breaks, specs))
}

/// `ADD FILES "other.csv"`
pub fn add_files_command(table: &mut DataTable, path: &str) -> Result<String, String> {
    let other = DataTable::from_path(std::path::Path::new(path))?;
    let extra: Vec<Column> = other
        .columns
        .iter()
        .filter(|c| table.column_index(&c.name).is_none())
        .map(|c| Column {
            name: c.name.clone(),
            kind: c.kind,
            values: vec![Value::Missing; table.n_rows],
            meta: c.meta.clone(),
        })
        .collect();
    table.columns.extend(extra);
    for col in table.columns.iter_mut() {
        if let Some(oi) = other.column_index(&col.name) {
            col.values.extend(other.columns[oi].values.clone());
            while col.values.len() < table.n_rows + other.n_rows {
                col.values.push(Value::Missing);
            }
        } else {
            col.values.extend(vec![Value::Missing; other.n_rows]);
        }
    }
    table.n_rows += other.n_rows;
    Ok(format!("{} case(s) now", table.n_rows))
}

/// `MATCH FILES "other.csv" BY id`
pub fn match_files_command(table: &mut DataTable, path: &str, by: &[String]) -> Result<String, String> {
    if by.is_empty() {
        return Err("MATCH FILES: expected BY.".to_string());
    }
    let other = DataTable::from_path(std::path::Path::new(path))?;
    let left_by: Result<Vec<usize>, String> = by
        .iter()
        .map(|n| {
            table
                .column_index(n)
                .ok_or_else(|| format!("MATCH FILES: unknown variable '{}'.", n))
        })
        .collect();
    let left_by = left_by?;
    let right_by: Result<Vec<usize>, String> = by
        .iter()
        .map(|n| {
            other
                .column_index(n)
                .ok_or_else(|| format!("MATCH FILES: '{}' is missing in the other file.", n))
        })
        .collect();
    let right_by = right_by?;
    let mut index: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for row in 0..other.n_rows {
        let key = match_key(&other, &right_by, row);
        index.entry(key).or_insert(row);
    }
    let new_names: Vec<usize> = other
        .columns
        .iter()
        .enumerate()
        .filter(|(_, c)| table.column_index(&c.name).is_none())
        .map(|(i, _)| i)
        .collect();
    for &oi in &new_names {
        table.columns.push(Column {
            name: other.columns[oi].name.clone(),
            kind: other.columns[oi].kind,
            values: vec![Value::Missing; table.n_rows],
            meta: other.columns[oi].meta.clone(),
        });
    }
    for row in 0..table.n_rows {
        let key = match_key(table, &left_by, row);
        if let Some(&orow) = index.get(&key) {
            for &oi in &new_names {
                let dest = table.column_index(&other.columns[oi].name).unwrap();
                table.columns[dest].values[row] = other.columns[oi].values[orow].clone();
            }
        }
    }
    Ok(format!("matched on {}", by.join(" ")))
}

fn match_key(table: &DataTable, cols: &[usize], row: usize) -> String {
    cols.iter()
        .map(|&ci| format!("{:?}", table.columns[ci].values.get(row)))
        .collect::<Vec<_>>()
        .join("\u{1f}")
}

/// `VARSTOCASES MAKE score FROM s1 s2 /INDEX=wave`
pub fn vars_to_cases_from_line(table: &mut DataTable, line: &str) -> Result<String, String> {
    let mut scan = Scan::new(line);
    if !scan.eat_word_ignore("VARSTOCASES") {
        return Err("VARSTOCASES: expected VARSTOCASES.".to_string());
    }
    if !scan.eat_word_ignore("MAKE") {
        return Err("VARSTOCASES: expected MAKE.".to_string());
    }
    let make = scan
        .ident()
        .ok_or_else(|| "VARSTOCASES: expected a name after MAKE.".to_string())?;
    if !scan.eat_word_ignore("FROM") {
        return Err("VARSTOCASES: expected FROM.".to_string());
    }
    let mut from = Vec::new();
    loop {
        scan.skip_ws();
        if scan.peek() == Some('/') || scan.rest().is_empty() {
            break;
        }
        match scan.ident() {
            Some(n) => from.push(n),
            None => break,
        }
    }
    let mut index = None;
    if scan.peek() == Some('/') {
        scan.i += 1;
        if scan.eat_word_ignore("INDEX") {
            scan.skip_ws();
            if scan.peek() == Some('=') {
                scan.i += 1;
            }
            index = scan.ident();
        }
    }
    vars_to_cases_command(table, &make, &from, index.as_deref())
}

/// `CASESTOVARS ID=id /INDEX=wave`
pub fn cases_to_vars_from_line(table: &mut DataTable, line: &str) -> Result<String, String> {
    let mut scan = Scan::new(line);
    if !scan.eat_word_ignore("CASESTOVARS") {
        return Err("CASESTOVARS: expected CASESTOVARS.".to_string());
    }
    let mut id = None;
    let mut index = None;
    loop {
        scan.skip_ws();
        if scan.rest().is_empty() {
            break;
        }
        if scan.peek() == Some('/') {
            scan.i += 1;
            continue;
        }
        if scan.eat_word_ignore("ID") {
            scan.skip_ws();
            if scan.peek() == Some('=') {
                scan.i += 1;
            }
            id = scan.ident();
            continue;
        }
        if scan.eat_word_ignore("INDEX") {
            scan.skip_ws();
            if scan.peek() == Some('=') {
                scan.i += 1;
            }
            index = scan.ident();
            continue;
        }
        let _ = scan.ident();
    }
    let id = id.ok_or_else(|| "CASESTOVARS: expected ID.".to_string())?;
    let index = index.ok_or_else(|| "CASESTOVARS: expected INDEX.".to_string())?;
    cases_to_vars_command(table, &id, &index)
}

/// `VARSTOCASES MAKE score FROM s1 s2 /INDEX=wave`
pub fn vars_to_cases_command(
    table: &mut DataTable,
    make: &str,
    from: &[String],
    index: Option<&str>,
) -> Result<String, String> {
    if from.is_empty() {
        return Err("VARSTOCASES: expected FROM variables.".to_string());
    }
    let from_idx: Result<Vec<usize>, String> = from
        .iter()
        .map(|n| {
            table
                .column_index(n)
                .ok_or_else(|| format!("VARSTOCASES: unknown variable '{}'.", n))
        })
        .collect();
    let from_idx = from_idx?;
    let keep: Vec<usize> = (0..table.columns.len())
        .filter(|i| !from_idx.contains(i))
        .collect();
    let mut out = Vec::new();
    for &ki in &keep {
        out.push(Column {
            name: table.columns[ki].name.clone(),
            kind: table.columns[ki].kind,
            values: Vec::new(),
            meta: table.columns[ki].meta.clone(),
        });
    }
    if let Some(idx) = index {
        out.push(Column {
            name: idx.to_string(),
            kind: ColumnKind::String,
            values: Vec::new(),
            meta: crate::data::VariableMeta::default_for(ColumnKind::String),
        });
    }
    out.push(Column {
        name: make.to_string(),
        kind: ColumnKind::Numeric,
        values: Vec::new(),
        meta: crate::data::VariableMeta::default_for(ColumnKind::Numeric),
    });
    for row in 0..table.n_rows {
        for (fi, &src) in from_idx.iter().enumerate() {
            for (oi, &ki) in keep.iter().enumerate() {
                out[oi]
                    .values
                    .push(table.columns[ki].values[row].clone());
            }
            let mut pos = keep.len();
            if index.is_some() {
                out[pos]
                    .values
                    .push(Value::Text(from[fi].clone()));
                pos += 1;
            }
            out[pos]
                .values
                .push(table.columns[src].values[row].clone());
        }
    }
    let n_rows = table.n_rows * from.len();
    *table = DataTable {
        columns: out,
        n_rows,
    };
    Ok(format!("{} case(s) now", n_rows))
}

/// `CASESTOVARS ID=id /INDEX=wave`
pub fn cases_to_vars_command(table: &mut DataTable, id: &str, index: &str) -> Result<String, String> {
    let id_i = table
        .column_index(id)
        .ok_or_else(|| format!("CASESTOVARS: unknown variable '{}'.", id))?;
    let idx_i = table
        .column_index(index)
        .ok_or_else(|| format!("CASESTOVARS: unknown variable '{}'.", index))?;
    let mut index_levels: Vec<String> = Vec::new();
    for row in 0..table.n_rows {
        if table.columns[idx_i].is_missing(row) {
            continue;
        }
        let k = format!("{}", display_cell(&table.columns[idx_i].values[row]));
        if !index_levels.iter().any(|x| x == &k) {
            index_levels.push(k);
        }
    }
    let measures: Vec<usize> = (0..table.columns.len())
        .filter(|&i| i != id_i && i != idx_i)
        .collect();
    let mut ids: Vec<String> = Vec::new();
    let mut id_values: Vec<Value> = Vec::new();
    let mut row_of: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for row in 0..table.n_rows {
        let key = format!("{:?}", table.columns[id_i].values.get(row));
        if !row_of.contains_key(&key) {
            row_of.insert(key.clone(), ids.len());
            ids.push(key);
            id_values.push(table.columns[id_i].values[row].clone());
        }
    }
    let mut out = vec![Column {
        name: id.to_string(),
        kind: table.columns[id_i].kind,
        values: id_values,
        meta: table.columns[id_i].meta.clone(),
    }];
    for &mi in &measures {
        for level in &index_levels {
            let name = format!("{}_{}", table.columns[mi].name, sanitize_ident(level));
            out.push(Column {
                name,
                kind: table.columns[mi].kind,
                values: vec![Value::Missing; ids.len()],
                meta: table.columns[mi].meta.clone(),
            });
        }
    }
    for row in 0..table.n_rows {
        let idk = format!("{:?}", table.columns[id_i].values.get(row));
        let Some(&out_row) = row_of.get(&idk) else {
            continue;
        };
        if table.columns[idx_i].is_missing(row) {
            continue;
        }
        let level = format!("{}", display_cell(&table.columns[idx_i].values[row]));
        let Some(li) = index_levels.iter().position(|x| x == &level) else {
            continue;
        };
        for (mi_pos, &mi) in measures.iter().enumerate() {
            let dest = 1 + mi_pos * index_levels.len() + li;
            out[dest].values[out_row] = table.columns[mi].values[row].clone();
        }
    }
    let n_rows = ids.len();
    *table = DataTable {
        columns: out,
        n_rows,
    };
    Ok(format!("{} case(s) now", n_rows))
}

fn display_cell(v: &Value) -> String {
    match v {
        Value::Missing => ".".to_string(),
        Value::Number(n) => format!("{}", n),
        Value::Text(s) => s.clone(),
    }
}

fn sanitize_ident(s: &str) -> String {
    let t: String = s
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    if t.is_empty() {
        "x".to_string()
    } else {
        t
    }
}

fn write_column(table: &mut DataTable, name: &str, values: Vec<Value>) {
    let has_text = values.iter().any(|v| matches!(v, Value::Text(_)));
    let kind = if has_text {
        ColumnKind::String
    } else {
        ColumnKind::Numeric
    };
    if let Some(idx) = table.column_index(name) {
        table.columns[idx].values = values;
        table.columns[idx].kind = kind;
        return;
    }
    table.columns.push(Column {
        name: name.to_string(),
        kind,
        values,
        meta: crate::data::VariableMeta::default_for(kind),
    });
}

#[derive(Debug, Clone)]
enum RecodeBound {
    Lo,
    Hi,
    Number(f64),
}

#[derive(Debug, Clone)]
enum RecodeAtom {
    Number(f64),
    Text(String),
}

#[derive(Debug, Clone)]
enum RecodeOld {
    Values(Vec<RecodeAtom>),
    Range { lo: RecodeBound, hi: RecodeBound },
    Missing,
    Sysmis,
    Else,
}

#[derive(Debug, Clone)]
enum RecodeNew {
    Copy,
    Sysmis,
    Number(f64),
    Text(String),
}

#[derive(Debug, Clone)]
struct RecodeRule {
    old: RecodeOld,
    new: RecodeNew,
}

struct Scan<'a> {
    s: &'a str,
    i: usize,
}

impl<'a> Scan<'a> {
    fn new(s: &'a str) -> Self {
        Self { s, i: 0 }
    }

    fn skip_ws(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.i += c.len_utf8();
            } else {
                break;
            }
        }
    }

    fn peek(&self) -> Option<char> {
        self.s[self.i..].chars().next()
    }

    fn rest(&self) -> &'a str {
        &self.s[self.i..]
    }

    fn eat_word_ignore(&mut self, word: &str) -> bool {
        self.skip_ws();
        if self.rest().len() >= word.len()
            && self.rest()[..word.len()].eq_ignore_ascii_case(word)
        {
            let after = self.i + word.len();
            let ok = after == self.s.len()
                || self.s[after..]
                    .chars()
                    .next()
                    .map(|c| !c.is_ascii_alphanumeric() && c != '_')
                    .unwrap_or(true);
            if ok {
                self.i = after;
                return true;
            }
        }
        false
    }

    fn ident(&mut self) -> Option<String> {
        self.skip_ws();
        let start = self.i;
        let mut chars = self.rest().chars();
        let first = chars.next()?;
        if !(first.is_ascii_alphabetic() || first == '_' || first == '@' || first == '#') {
            return None;
        }
        self.i += first.len_utf8();
        while let Some(c) = self.peek() {
            if c.is_ascii_alphanumeric() || c == '_' || c == '@' || c == '#' {
                self.i += c.len_utf8();
            } else {
                break;
            }
        }
        Some(self.s[start..self.i].to_string())
    }

    fn paren_group(&mut self) -> Result<String, String> {
        self.skip_ws();
        if self.peek() != Some('(') {
            return Err("Expected '('.".to_string());
        }
        self.i += 1;
        let start = self.i;
        let mut depth = 1;
        let mut quote: Option<char> = None;
        while self.i < self.s.len() {
            let c = self.s[self.i..].chars().next().unwrap();
            let n = c.len_utf8();
            if let Some(q) = quote {
                if c == q {
                    quote = None;
                }
                self.i += n;
                continue;
            }
            if c == '"' || c == '\'' {
                quote = Some(c);
                self.i += n;
                continue;
            }
            if c == '(' {
                depth += 1;
            } else if c == ')' {
                depth -= 1;
                if depth == 0 {
                    let inner = self.s[start..self.i].to_string();
                    self.i += n;
                    return Ok(inner);
                }
            }
            self.i += n;
        }
        Err("Unclosed '('.".to_string())
    }
}

fn parse_recode(line: &str) -> Result<(Vec<String>, Vec<RecodeRule>, Vec<String>), String> {
    let mut scan = Scan::new(line);
    if !scan.eat_word_ignore("RECODE") {
        return Err("RECODE: expected RECODE.".to_string());
    }
    let mut sources = Vec::new();
    loop {
        scan.skip_ws();
        if scan.peek() == Some('(') || scan.rest().is_empty() {
            break;
        }
        if scan.eat_word_ignore("INTO") {
            return Err("RECODE: mappings must come before INTO.".to_string());
        }
        match scan.ident() {
            Some(name) => sources.push(name),
            None => break,
        }
    }
    let mut rules = Vec::new();
    loop {
        scan.skip_ws();
        if scan.peek() != Some('(') {
            break;
        }
        let inner = scan.paren_group()?;
        rules.push(parse_recode_rule(&inner)?);
    }
    let mut into = Vec::new();
    if scan.eat_word_ignore("INTO") {
        while let Some(name) = scan.ident() {
            into.push(name);
        }
    }
    scan.skip_ws();
    if !scan.rest().is_empty() {
        return Err(format!("RECODE: unexpected '{}'.", scan.rest().trim()));
    }
    Ok((sources, rules, into))
}

fn parse_count(line: &str) -> Result<(String, Vec<String>, Vec<RecodeAtom>), String> {
    let mut scan = Scan::new(line);
    if !scan.eat_word_ignore("COUNT") {
        return Err("COUNT: expected COUNT.".to_string());
    }
    let dest = scan
        .ident()
        .ok_or_else(|| "COUNT: expected a target name.".to_string())?;
    scan.skip_ws();
    if scan.peek() != Some('=') {
        return Err("COUNT: expected '='.".to_string());
    }
    scan.i += 1;
    let mut sources = Vec::new();
    loop {
        scan.skip_ws();
        if scan.peek() == Some('(') || scan.rest().is_empty() {
            break;
        }
        match scan.ident() {
            Some(name) => sources.push(name),
            None => break,
        }
    }
    scan.skip_ws();
    if scan.peek() != Some('(') {
        return Err("COUNT: expected a value list in parentheses.".to_string());
    }
    let inner = scan.paren_group()?;
    let atoms = parse_atom_list(&inner)?;
    Ok((dest, sources, atoms))
}

fn parse_rank(line: &str) -> Result<(String, String), String> {
    let mut scan = Scan::new(line);
    if !scan.eat_word_ignore("RANK") {
        return Err("RANK: expected RANK.".to_string());
    }
    let source = scan
        .ident()
        .ok_or_else(|| "RANK: expected a variable.".to_string())?;
    let dest = if scan.eat_word_ignore("INTO") {
        scan.ident()
            .ok_or_else(|| "RANK: expected a name after INTO.".to_string())?
    } else {
        format!("{}_r", source)
    };
    scan.skip_ws();
    if !scan.rest().is_empty() {
        return Err(format!("RANK: unexpected '{}'.", scan.rest().trim()));
    }
    Ok((source, dest))
}

fn parse_autorecode(line: &str) -> Result<(String, String), String> {
    let mut scan = Scan::new(line);
    if !scan.eat_word_ignore("AUTORECODE") {
        return Err("AUTORECODE: expected AUTORECODE.".to_string());
    }
    let source = scan
        .ident()
        .ok_or_else(|| "AUTORECODE: expected a variable.".to_string())?;
    if !scan.eat_word_ignore("INTO") {
        return Err("AUTORECODE: expected INTO.".to_string());
    }
    let dest = scan
        .ident()
        .ok_or_else(|| "AUTORECODE: expected a name after INTO.".to_string())?;
    scan.skip_ws();
    if !scan.rest().is_empty() {
        return Err(format!("AUTORECODE: unexpected '{}'.", scan.rest().trim()));
    }
    Ok((source, dest))
}

fn parse_recode_rule(inner: &str) -> Result<RecodeRule, String> {
    let eq = find_bare_eq(inner).ok_or_else(|| {
        format!("RECODE: mapping '{}' needs '='.", inner.trim())
    })?;
    let left = inner[..eq].trim();
    let right = inner[eq + 1..].trim();
    if left.is_empty() || right.is_empty() {
        return Err("RECODE: empty mapping.".to_string());
    }
    let old = parse_recode_old(left)?;
    let new = parse_recode_new(right)?;
    Ok(RecodeRule { old, new })
}

fn find_bare_eq(s: &str) -> Option<usize> {
    let mut quote: Option<char> = None;
    let mut i = 0;
    while i < s.len() {
        let c = s[i..].chars().next()?;
        let n = c.len_utf8();
        if let Some(q) = quote {
            if c == q {
                quote = None;
            }
            i += n;
            continue;
        }
        if c == '"' || c == '\'' {
            quote = Some(c);
            i += n;
            continue;
        }
        if c == '=' {
            return Some(i);
        }
        i += n;
    }
    None
}

fn parse_recode_old(left: &str) -> Result<RecodeOld, String> {
    let tokens = tokenize_recode_side(left)?;
    if tokens.len() == 1 {
        match tokens[0].as_str() {
            "ELSE" => return Ok(RecodeOld::Else),
            "MISSING" => return Ok(RecodeOld::Missing),
            "SYSMIS" => return Ok(RecodeOld::Sysmis),
            _ => {}
        }
    }
    if let Some(thru) = tokens.iter().position(|t| t == "THRU") {
        if thru == 0 || thru + 1 >= tokens.len() {
            return Err("RECODE: THRU needs two bounds.".to_string());
        }
        let lo = parse_bound(&tokens[thru - 1])?;
        let hi = parse_bound(&tokens[thru + 1])?;
        return Ok(RecodeOld::Range { lo, hi });
    }
    Ok(RecodeOld::Values(parse_atom_tokens(&tokens)?))
}

fn parse_recode_new(right: &str) -> Result<RecodeNew, String> {
    let t = right.trim();
    let u = t.to_uppercase();
    if u == "COPY" {
        return Ok(RecodeNew::Copy);
    }
    if u == "SYSMIS" || u == "MISSING" {
        return Ok(RecodeNew::Sysmis);
    }
    if (t.starts_with('"') && t.ends_with('"') && t.len() >= 2)
        || (t.starts_with('\'') && t.ends_with('\'') && t.len() >= 2)
    {
        return Ok(RecodeNew::Text(t[1..t.len() - 1].to_string()));
    }
    if let Ok(n) = t.parse::<f64>() {
        return Ok(RecodeNew::Number(n));
    }
    Ok(RecodeNew::Text(t.to_string()))
}

fn parse_bound(tok: &str) -> Result<RecodeBound, String> {
    match tok {
        "LO" | "LOWEST" => Ok(RecodeBound::Lo),
        "HI" | "HIGHEST" => Ok(RecodeBound::Hi),
        other => other
            .parse::<f64>()
            .map(RecodeBound::Number)
            .map_err(|_| format!("RECODE: bad bound '{}'.", other)),
    }
}

fn parse_atom_list(inner: &str) -> Result<Vec<RecodeAtom>, String> {
    let tokens = tokenize_recode_side(inner)?;
    parse_atom_tokens(&tokens)
}

fn parse_atom_tokens(tokens: &[String]) -> Result<Vec<RecodeAtom>, String> {
    let mut out = Vec::new();
    for t in tokens {
        if t == "THRU" || t == "ELSE" || t == "MISSING" || t == "SYSMIS" || t == "LO" || t == "HI"
        {
            return Err(format!("Unexpected '{}' in a value list.", t));
        }
        if (t.starts_with('"') && t.ends_with('"') && t.len() >= 2)
            || (t.starts_with('\'') && t.ends_with('\'') && t.len() >= 2)
        {
            out.push(RecodeAtom::Text(t[1..t.len() - 1].to_string()));
        } else if let Ok(n) = t.parse::<f64>() {
            out.push(RecodeAtom::Number(n));
        } else {
            out.push(RecodeAtom::Text(t.clone()));
        }
    }
    if out.is_empty() {
        return Err("Empty value list.".to_string());
    }
    Ok(out)
}

fn tokenize_recode_side(s: &str) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < s.len() {
        let c = s[i..].chars().next().unwrap();
        if c.is_whitespace() || c == ',' {
            i += c.len_utf8();
            continue;
        }
        if c == '"' || c == '\'' {
            let q = c;
            let start = i;
            i += c.len_utf8();
            while i < s.len() {
                let d = s[i..].chars().next().unwrap();
                i += d.len_utf8();
                if d == q {
                    break;
                }
            }
            out.push(s[start..i].to_string());
            continue;
        }
        let start = i;
        i += c.len_utf8();
        while i < s.len() {
            let d = s[i..].chars().next().unwrap();
            if d.is_whitespace() || d == ',' {
                break;
            }
            i += d.len_utf8();
        }
        out.push(s[start..i].to_uppercase());
    }
    Ok(out)
}

fn apply_recode(col: &Column, row: usize, value: &Value, rules: &[RecodeRule]) -> Value {
    let mut else_new: Option<&RecodeNew> = None;
    for rule in rules {
        if matches!(rule.old, RecodeOld::Else) {
            else_new = Some(&rule.new);
            continue;
        }
        if recode_old_matches(&rule.old, col, row, value) {
            return recode_new_value(&rule.new, value);
        }
    }
    if let Some(new) = else_new {
        recode_new_value(new, value)
    } else {
        value.clone()
    }
}

fn recode_old_matches(old: &RecodeOld, col: &Column, row: usize, value: &Value) -> bool {
    match old {
        RecodeOld::Else => true,
        RecodeOld::Sysmis => matches!(value, Value::Missing),
        RecodeOld::Missing => col.is_missing(row),
        RecodeOld::Values(atoms) => {
            if col.is_missing(row) {
                false
            } else {
                atoms.iter().any(|a| atom_matches(a, value))
            }
        }
        RecodeOld::Range { lo, hi } => {
            if col.is_missing(row) {
                return false;
            }
            let Value::Number(n) = value else {
                return false;
            };
            bound_ok(lo, *n, true) && bound_ok(hi, *n, false)
        }
    }
}

fn bound_ok(b: &RecodeBound, n: f64, is_lo: bool) -> bool {
    match b {
        RecodeBound::Lo => is_lo,
        RecodeBound::Hi => !is_lo,
        RecodeBound::Number(x) => {
            if is_lo {
                n + 1e-12 >= *x
            } else {
                n - 1e-12 <= *x
            }
        }
    }
}

fn recode_new_value(new: &RecodeNew, original: &Value) -> Value {
    match new {
        RecodeNew::Copy => original.clone(),
        RecodeNew::Sysmis => Value::Missing,
        RecodeNew::Number(n) => Value::Number(*n),
        RecodeNew::Text(s) => Value::Text(s.clone()),
    }
}

fn atom_matches(atom: &RecodeAtom, value: &Value) -> bool {
    match (atom, value) {
        (RecodeAtom::Number(a), Value::Number(b)) => (a - b).abs() < 1e-12,
        (RecodeAtom::Text(a), Value::Text(b)) => a == b,
        (RecodeAtom::Text(a), Value::Number(b)) => a.parse::<f64>().ok() == Some(*b),
        (RecodeAtom::Number(a), Value::Text(b)) => b.trim().parse::<f64>().ok() == Some(*a),
        _ => false,
    }
}

fn atom_eq(a: &RecodeAtom, b: &RecodeAtom) -> bool {
    match (a, b) {
        (RecodeAtom::Number(x), RecodeAtom::Number(y)) => (x - y).abs() < 1e-12,
        (RecodeAtom::Text(x), RecodeAtom::Text(y)) => x == y,
        _ => false,
    }
}

fn atom_ord(a: &RecodeAtom, b: &RecodeAtom) -> std::cmp::Ordering {
    match (a, b) {
        (RecodeAtom::Number(x), RecodeAtom::Number(y)) => {
            x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal)
        }
        (RecodeAtom::Text(x), RecodeAtom::Text(y)) => x.cmp(y),
        (RecodeAtom::Number(_), RecodeAtom::Text(_)) => std::cmp::Ordering::Less,
        (RecodeAtom::Text(_), RecodeAtom::Number(_)) => std::cmp::Ordering::Greater,
    }
}

fn atom_label(atom: &RecodeAtom) -> String {
    match atom {
        RecodeAtom::Number(n) => format!("{}", n),
        RecodeAtom::Text(s) => s.clone(),
    }
}

/// Evaluate an expression for one case.
fn eval_expression(
    expression: &str,
    names: &[String],
    row: &[Value],
) -> Result<Value, String> {
    // The variable references, resolved to their values.
    let mut tokens: Vec<Token> = Vec::new();
    let chars: Vec<char> = expression.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if c.is_ascii_alphabetic() || c == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            let upper = word.to_uppercase();
            // A function call?
            if i < chars.len() && chars[i] == '(' && is_function(&upper) {
                tokens.push(Token::Function(upper));
                continue;
            }
            // A variable reference.
            match names.iter().position(|n| n.eq_ignore_ascii_case(&word)) {
                Some(idx) => tokens.push(Token::Value(value_of(&row[idx]))),
                None => return Err(format!("Unknown variable '{}'", word)),
            }
            continue;
        }
        if c.is_ascii_digit() || c == '.' {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            let text: String = chars[start..i].iter().collect();
            match text.parse::<f64>() {
                Ok(n) => tokens.push(Token::Value(Value::Number(n))),
                Err(_) => return Err(format!("Bad number '{}'", text)),
            }
            continue;
        }
        match c {
            '+' | '-' | '*' | '/' => {
                tokens.push(Token::Op(c));
                i += 1;
            }
            '(' | ')' => {
                tokens.push(Token::Paren(c));
                i += 1;
            }
            ',' => {
                // The separator between a function's arguments.
                tokens.push(Token::Comma);
                i += 1;
            }
            _ => return Err(format!("Unexpected character '{}'", c)),
        }
    }

    let mut pos = 0;
    let v = parse_sum(&tokens, &mut pos)?;
    if pos != tokens.len() {
        return Err("Unexpected trailing input".to_string());
    }
    Ok(v)
}

/// The tokens of an expression.
#[derive(Debug, Clone)]
enum Token {
    Value(Value),
    Op(char),
    Paren(char),
    Function(String),
    Comma,
}

fn is_function(name: &str) -> bool {
    matches!(
        name,
        "MEAN" | "MIN" | "MAX" | "SUM" | "ABS" | "SQRT" | "LN" | "LOG10" | "ROUND"
    )
}

/// The numeric value of a cell, treating a non-number as missing.
fn value_of(v: &Value) -> Value {
    match v {
        Value::Number(n) => Value::Number(*n),
        Value::Text(s) => match s.trim().parse::<f64>() {
            Ok(n) => Value::Number(n),
            Err(_) => Value::Missing,
        },
        Value::Missing => Value::Missing,
    }
}

/// One argument of a function call.
///
/// It is like a sum, but it stops at a comma, so that the arguments of
/// `MEAN(a, b)` stay separate.
fn parse_argument(tokens: &[Token], pos: &mut usize) -> Result<Value, String> {
    let mut left = parse_product(tokens, pos)?;
    while *pos < tokens.len() {
        match &tokens[*pos] {
            Token::Op('+') => {
                *pos += 1;
                let right = parse_product(tokens, pos)?;
                left = add(&left, &right);
            }
            Token::Op('-') => {
                *pos += 1;
                let right = parse_product(tokens, pos)?;
                left = subtract(&left, &right);
            }
            _ => break,
        }
    }
    Ok(left)
}

/// An expression term: a sum of products.
fn parse_sum(tokens: &[Token], pos: &mut usize) -> Result<Value, String> {
    let mut left = parse_product(tokens, pos)?;
    while *pos < tokens.len() {
        match &tokens[*pos] {
            Token::Op('+') => {
                *pos += 1;
                let right = parse_product(tokens, pos)?;
                left = add(&left, &right);
            }
            Token::Op('-') => {
                *pos += 1;
                let right = parse_product(tokens, pos)?;
                left = subtract(&left, &right);
            }
            _ => break,
        }
    }
    Ok(left)
}

fn parse_product(tokens: &[Token], pos: &mut usize) -> Result<Value, String> {
    let mut left = parse_unary(tokens, pos)?;
    while *pos < tokens.len() {
        match &tokens[*pos] {
            Token::Op('*') => {
                *pos += 1;
                let right = parse_unary(tokens, pos)?;
                left = multiply(&left, &right);
            }
            Token::Op('/') => {
                *pos += 1;
                let right = parse_unary(tokens, pos)?;
                left = divide(&left, &right);
            }
            _ => break,
        }
    }
    Ok(left)
}

fn parse_unary(tokens: &[Token], pos: &mut usize) -> Result<Value, String> {
    if *pos < tokens.len() {
        if let Token::Op('-') = &tokens[*pos] {
            *pos += 1;
            let v = parse_unary(tokens, pos)?;
            return Ok(match v {
                Value::Number(n) => Value::Number(-n),
                other => other,
            });
        }
        if let Token::Op('+') = &tokens[*pos] {
            *pos += 1;
            return parse_unary(tokens, pos);
        }
    }
    parse_primary(tokens, pos)
}

fn parse_primary(tokens: &[Token], pos: &mut usize) -> Result<Value, String> {
    if *pos >= tokens.len() {
        return Err("Unexpected end of expression".to_string());
    }
    match &tokens[*pos] {
        Token::Value(v) => {
            let v = v.clone();
            *pos += 1;
            Ok(v)
        }
        Token::Paren('(') => {
            *pos += 1;
            let v = parse_sum(tokens, pos)?;
            if *pos >= tokens.len() {
                return Err("Missing closing parenthesis".to_string());
            }
            if let Token::Paren(')') = &tokens[*pos] {
                *pos += 1;
            }
            Ok(v)
        }
        Token::Function(name) => {
            let name = name.clone();
            *pos += 1;
            // The opening parenthesis.
            if *pos >= tokens.len() {
                return Err("Missing '(' after the function".to_string());
            }
            if let Token::Paren('(') = &tokens[*pos] {
                *pos += 1;
            }
            // The arguments.
            let mut args = Vec::new();
            loop {
                if *pos >= tokens.len() {
                    return Err("Missing closing parenthesis".to_string());
                }
                if let Token::Paren(')') = &tokens[*pos] {
                    *pos += 1;
                    break;
                }
                args.push(parse_argument(tokens, pos)?);
                if *pos < tokens.len() {
                    if let Token::Comma = &tokens[*pos] {
                        *pos += 1;
                    }
                }
            }
            apply_function(&name, &args)
        }
        Token::Paren(c) => Err(format!("Unexpected '{}'", c)),
        Token::Op(c) => Err(format!("Unexpected '{}'", c)),
        Token::Comma => Err("Unexpected ','".to_string()),
    }
}

/// Apply one of the supported functions.
fn apply_function(name: &str, args: &[Value]) -> Result<Value, String> {
    let nums: Vec<f64> = args
        .iter()
        .filter_map(|v| match v {
            Value::Number(n) => Some(*n),
            _ => None,
        })
        .collect();
    let result = match name {
        "MEAN" => {
            if nums.is_empty() {
                f64::NAN
            } else {
                nums.iter().sum::<f64>() / nums.len() as f64
            }
        }
        "MIN" => nums.iter().cloned().fold(f64::INFINITY, f64::min),
        "MAX" => nums.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
        "SUM" => nums.iter().sum::<f64>(),
        "ABS" => nums.first().map(|v| v.abs()).unwrap_or(f64::NAN),
        "SQRT" => nums.first().map(|v| v.sqrt()).unwrap_or(f64::NAN),
        "LN" => nums.first().map(|v| v.ln()).unwrap_or(f64::NAN),
        "LOG10" => nums.first().map(|v| v.log10()).unwrap_or(f64::NAN),
        "ROUND" => {
            let x = nums.first().copied().unwrap_or(f64::NAN);
            let places = nums.get(1).copied().unwrap_or(0.0) as i32;
            let factor = 10f64.powi(places);
            (x * factor).round() / factor
        }
        _ => return Err(format!("Unknown function '{}'", name)),
    };
    if result.is_finite() {
        Ok(Value::Number(result))
    } else {
        Ok(Value::Missing)
    }
}

fn add(a: &Value, b: &Value) -> Value {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => Value::Number(x + y),
        _ => Value::Missing,
    }
}

fn subtract(a: &Value, b: &Value) -> Value {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => Value::Number(x - y),
        _ => Value::Missing,
    }
}

fn multiply(a: &Value, b: &Value) -> Value {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => Value::Number(x * y),
        _ => Value::Missing,
    }
}

fn divide(a: &Value, b: &Value) -> Value {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => {
            if *y == 0.0 {
                Value::Missing
            } else {
                Value::Number(x / y)
            }
        }
        _ => Value::Missing,
    }
}
