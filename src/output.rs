#[derive(Debug, Clone, Default)]
pub struct OutputTable {
    pub title: String,
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
    /// Footnote rendered beneath the pivot table (SPSS "Notes").
    pub note: String,
}

impl OutputTable {
    pub fn from_descriptives(descs: &[crate::stats::Descriptive]) -> Self {
        let mut set = crate::stats::StatSet::defaults();
        set.sum = false;
        Self::descriptives_with(descs, &set)
    }

    /// Descriptives rendered with only the statistics the user asked for.
    ///
    /// SPSS builds one table per variable when several variables are
    /// analysed, with a row per statistic. We follow that layout.
    pub fn descriptives_with(
        descs: &[crate::stats::Descriptive],
        stats: &crate::stats::StatSet,
    ) -> Self {
        // The statistic rows, in SPSS's order.
        let mut stat_rows: Vec<(
            String,
            Box<dyn Fn(&crate::stats::Descriptive) -> String>,
        )> = Vec::new();
        if stats.mean {
            stat_rows.push(("Mean".to_string(), Box::new(|d: &crate::stats::Descriptive| format!("{:.4}", d.mean))));
        }
        if stats.stddev {
            stat_rows.push(("Std. Deviation".to_string(), Box::new(|d: &crate::stats::Descriptive| format!("{:.4}", d.std_dev))));
        }
        if stats.variance {
            stat_rows.push(("Variance".to_string(), Box::new(|d: &crate::stats::Descriptive| format!("{:.4}", d.variance))));
        }
        if stats.median {
            stat_rows.push(("Median".to_string(), Box::new(|d: &crate::stats::Descriptive| format!("{:.4}", d.median))));
        }
        if stats.mode {
            stat_rows.push(("Mode".to_string(), Box::new(|d: &crate::stats::Descriptive| match d.mode {
                Some(v) => format!("{:.4}", v),
                None => "—".to_string(),
            })));
        }
        if stats.minimum {
            stat_rows.push(("Minimum".to_string(), Box::new(|d: &crate::stats::Descriptive| format!("{:.4}", d.min))));
        }
        if stats.maximum {
            stat_rows.push(("Maximum".to_string(), Box::new(|d: &crate::stats::Descriptive| format!("{:.4}", d.max))));
        }
        if stats.range {
            stat_rows.push(("Range".to_string(), Box::new(|d: &crate::stats::Descriptive| format!("{:.4}", d.range))));
        }
        if stats.sum {
            stat_rows.push(("Sum".to_string(), Box::new(|d: &crate::stats::Descriptive| format!("{:.4}", d.sum))));
        }
        if stats.semean {
            stat_rows.push(("Std. Error of Mean".to_string(), Box::new(|d: &crate::stats::Descriptive| format!("{:.5}", d.std_error_mean))));
        }
        if stats.skewness {
            stat_rows.push(("Skewness".to_string(), Box::new(|d: &crate::stats::Descriptive| format!("{:.4}", d.skewness))));
        }
        if stats.kurtosis {
            stat_rows.push(("Kurtosis".to_string(), Box::new(|d: &crate::stats::Descriptive| format!("{:.4}", d.kurtosis))));
        }
        if stats.sekurt {
            stat_rows.push((
                "Std. Error of Kurtosis".to_string(),
                Box::new(|d: &crate::stats::Descriptive| format!("{:.5}", d.std_error_kurtosis)),
            ));
        }
        for (p, label) in percent_labels(stats) {
            stat_rows.push((
                label,
                Box::new(move |d: &crate::stats::Descriptive| {
                    match d
                        .percentiles
                        .iter()
                        .find(|(pp, _)| (*pp - p).abs() < f64::EPSILON)
                    {
                        Some((_, v)) => format!("{:.4}", v),
                        None => "—".to_string(),
                    }
                }),
            ));
        }

        let mut rows = Vec::new();
        for d in descs {
            // One block per variable, laid out the way SPSS does it: the
            // variable name on the first line with its N, then one line per
            // statistic with the statistic's name in the first column and
            // its value under the matching column.
            rows.push(vec![d.variable.clone(), d.n.to_string()]);
            for (label, f) in &stat_rows {
                let mut row = vec![label.clone()];
                row.push(String::new()); // N is blank on statistic lines
                for _ in 2..stat_rows.len() {
                    row.push(String::new());
                }
                // The value goes under its own column.
                let col = 2 + stat_rows
                    .iter()
                    .position(|(l, _)| l == label)
                    .unwrap_or(0);
                while row.len() <= col {
                    row.push(String::new());
                }
                row[col] = f(d);
                rows.push(row);
            }
        }

        let mut headers = vec!["Variable".to_string(), "N".to_string()];
        headers.extend(stat_rows.iter().map(|(l, _)| l.clone()));

        Self {
            title: "Descriptive Statistics".to_string(),
            headers,
            rows,
            note: "a. Counts exclude system- and user-missing cases.".to_string(),
        }
    }

    pub fn from_correlations(rows_in: &[crate::stats::Correlation]) -> Self {
        let mut rows = Vec::new();
        for r in rows_in {
            rows.push(vec![r.var1.clone(), r.var2.clone(), format!("{:.4}", r.r)]);
        }
        Self {
            title: "Correlations".to_string(),
            headers: vec!["Variable 1".into(), "Variable 2".into(), "Pearson r".into()],
            rows,
            ..Default::default()
        }
    }

    /// The SPSS regression output: a model summary, an ANOVA table and a
    /// coefficients table.
    pub fn regression_with(r: &crate::stats::RegressionResult) -> Self {
        // SPSS puts these in three separate tables; we lay them out in one
        // grid with section headings, as the earlier tables do.
        let mut headers: Vec<String> = vec!["Model".into(), "R".into(), "R²".into()];
        headers.push("Adjusted R²".into());
        headers.push("Std. Error of Estimate".into());
        let mut rows = Vec::new();
        rows.push(vec![
            "1".to_string(),
            format!("{:.3}", r.r2.sqrt()),
            format!("{:.3}", r.r2),
            format!("{:.3}", r.adj_r2),
            format!("{:.3}", r.std_error_estimate),
        ]);

        let mut headers2: Vec<String> = vec!["ANOVA".into(), "Sum of Squares".into()];
        headers2.push("df".into());
        headers2.push("Mean Square".into());
        headers2.push("F".into());
        headers2.push("Sig.".into());
        let mut rows2 = Vec::new();
        rows2.push(vec![
            "Regression".to_string(),
            format!("{:.3}", r.ssr),
            "1".to_string(),
            format!("{:.3}", r.ssr),
            format!("{:.3}", r.f),
            format!("{:.4}", r.f_p),
        ]);
        rows2.push(vec![
            "Residual".to_string(),
            format!("{:.3}", r.ss_resid),
            format!("{}", (r.n as f64 - 2.0).max(0.0) as i64),
            format!("{:.3}", r.ss_resid / (r.n as f64 - 2.0).max(1.0)),
            String::new(),
            String::new(),
        ]);
        rows2.push(vec![
            "Total".to_string(),
            format!("{:.3}", r.ss_total),
            format!("{}", (r.n as f64 - 1.0).max(0.0) as i64),
            String::new(),
            String::new(),
            String::new(),
        ]);

        let headers3 = vec![
            "Coefficients".to_string(),
            "B".to_string(),
            "Std. Error".to_string(),
            "t".to_string(),
            "Sig.".to_string(),
        ];
        let mut rows3 = Vec::new();
        rows3.push(vec![
            "Constant".to_string(),
            format!("{:.4}", r.intercept),
            String::new(),
            String::new(),
            String::new(),
        ]);
        rows3.push(vec![
            r.predictor.clone(),
            format!("{:.4}", r.slope),
            format!("{:.4}", r.std_error),
            format!("{:.3}", r.t),
            format!("{:.4}", r.p),
        ]);

        // Merge the three tables into one grid: the union of the columns,
        // with blanks where a table does not use them.
        let all_headers = merge_headers(&[headers, headers2, headers3]);
        let mut all_rows = Vec::new();
        for (label, rws) in [
            ("Model Summary", rows),
            ("ANOVA", rows2),
            ("Coefficients", rows3),
        ] {
            all_rows.push(vec![label.to_string()]);
            for r in rws {
                all_rows.push(merge_row(&all_headers, &r));
            }
        }

        Self {
            title: format!("Regression: {} on {}", r.outcome, r.predictor),
            headers: all_headers,
            rows: all_rows,
            note: format!(
                "Dependent Variable: {}. N = {}. Method: Least Squares.",
                r.outcome, r.n
            ),
        }
    }

    pub fn from_regression(r: &crate::stats::RegressionResult) -> Self {
        let rows = vec![vec![
            r.predictor.clone(),
            r.outcome.clone(),
            r.n.to_string(),
            format!("{:.4}", r.slope),
            format!("{:.4}", r.intercept),
            format!("{:.4}", r.r2),
        ]];
        Self {
            title: "Linear Regression".to_string(),
            headers: vec![
                "Predictor".into(),
                "Outcome".into(),
                "N".into(),
                "Slope".into(),
                "Intercept".into(),
                "R²".into(),
            ],
            rows,
            ..Default::default()
        }
    }

    pub fn from_ttest(r: &crate::stats::TTestResult) -> Self {
        let rows = vec![vec![
            r.outcome.clone(),
            r.group_var.clone(),
            r.group1.clone(),
            r.group2.clone(),
            r.n1.to_string(),
            r.n2.to_string(),
            format!("{:.4}", r.mean1),
            format!("{:.4}", r.mean2),
            format!("{:.4}", r.t),
            format!("{:.4}", r.df),
            format!("{:.4}", r.p),
        ]];
        Self {
            title: "Independent Samples T-Test".to_string(),
            headers: vec![
                "Outcome".into(),
                "Group var".into(),
                "Group 1".into(),
                "Group 2".into(),
                "N1".into(),
                "N2".into(),
                "Mean1".into(),
                "Mean2".into(),
                "t".into(),
                "df".into(),
                "p (2-tail)".into(),
            ],
            rows,
            ..Default::default()
        }
    }

    pub fn from_frequencies(var: &str, rows_in: &[crate::stats::FreqRow]) -> Self {
        let mut rows = Vec::new();
        for r in rows_in {
            rows.push(vec![
                r.value.clone(),
                r.count.to_string(),
                format!("{:.2}", r.percent),
            ]);
        }
        // trailing Total row like SPSS frequency tables
        let total: usize = rows_in.iter().map(|r| r.count).sum::<usize>();
        rows.push(vec![String::from("Total"), total.to_string(), "100.00".into()]);
        Self {
            title: format!("Frequencies: {}", var),
            headers: vec!["Value".into(), "Count".into(), "Percent".into()],
            rows,
            note: "a. Counts exclude system- and user-missing cases.".to_string(),
        }
    }

    /// SPSS Crosstabs: crosstabulation table + Pearson Chi-Square line in the note.
    /// A crosstab with the cell counts and statistics the user chose.
    ///
    /// SPSS shows one table per cell type, each with the counts beneath
    /// the categories. We do the same: a separate block for observed
    /// counts, expected counts, and each percentage type, followed by the
    /// test statistics.
    pub fn crosstabs_with(
        ct: &crate::stats::CrosstabsResult,
        cells: &crate::stats::CellCounts,
        stats_sel: &crate::stats::CrosstabStats,
    ) -> Self {
        let mut headers = vec![ct.var1.clone()];
        headers.extend(ct.levels2.iter().cloned());
        headers.push("Total".into());
        let mut rows = Vec::new();

        let r = ct.levels1.len();
        let c = ct.levels2.len();
        let n = ct.n.max(1) as f64;

        // ---- Observed counts ----
        if cells.observed || cells.is_empty() {
            rows.push(vec![format!("{} (count)", ct.var1)]);
            for (i, l1) in ct.levels1.iter().enumerate() {
                let mut row = vec![l1.clone()];
                for j in 0..c {
                    row.push(ct.observed[i][j].to_string());
                }
                row.push(ct.row_totals[i].to_string());
                rows.push(row);
            }
            let mut total_row = vec![String::from("Total")];
            for j in 0..c {
                total_row.push(col_total(ct, j).to_string());
            }
            total_row.push(ct.n.to_string());
            rows.push(total_row);
        }

        // ---- Expected counts ----
        if cells.expected {
            rows.push(vec![format!("{} (expected count)", ct.var1)]);
            for (i, l1) in ct.levels1.iter().enumerate() {
                let mut row = vec![l1.clone()];
                for j in 0..c {
                    row.push(format!("{:.2}", ct.expected[i][j]));
                }
                row.push(format!("{:.2}", ct.row_totals[i] as f64));
                rows.push(row);
            }
            let mut total_row = vec![String::from("Total")];
            for j in 0..c {
                let e: f64 = (0..r).map(|i| ct.expected[i][j]).sum();
                total_row.push(format!("{:.2}", e));
            }
            total_row.push(format!("{:.2}", n));
            rows.push(total_row);
        }

        // ---- Row percentages ----
        if cells.row_percent {
            rows.push(vec![format!("{} (row %)", ct.var1)]);
            for (i, l1) in ct.levels1.iter().enumerate() {
                let mut row = vec![l1.clone()];
                for j in 0..c {
                    let pct = if ct.row_totals[i] > 0 {
                        ct.observed[i][j] as f64 / ct.row_totals[i] as f64 * 100.0
                    } else {
                        0.0
                    };
                    row.push(format!("{:.1}%", pct));
                }
                row.push("100.0%".to_string());
                rows.push(row);
            }
            let mut total_row = vec![String::from("Total")];
            for j in 0..c {
                let ct_total = col_total(ct, j);
                let pct = if ct_total > 0 {
                    ct_total as f64 / ct.n as f64 * 100.0
                } else {
                    0.0
                };
                total_row.push(format!("{:.1}%", pct));
            }
            total_row.push("100.0%".to_string());
            rows.push(total_row);
        }

        // ---- Column percentages ----
        if cells.column_percent {
            rows.push(vec![format!("{} (column %)", ct.var1)]);
            for (i, l1) in ct.levels1.iter().enumerate() {
                let mut row = vec![l1.clone()];
                for j in 0..c {
                    let ct_total = col_total(ct, j);
                    let pct = if ct_total > 0 {
                        ct.observed[i][j] as f64 / ct_total as f64 * 100.0
                    } else {
                        0.0
                    };
                    row.push(format!("{:.1}%", pct));
                }
                let total_pct = if ct.n > 0 {
                    ct.row_totals[i] as f64 / ct.n as f64 * 100.0
                } else {
                    0.0
                };
                row.push(format!("{:.1}%", total_pct));
                rows.push(row);
            }
            let mut total_row = vec![String::from("Total")];
            for j in 0..c {
                let ct_total = col_total(ct, j);
                let pct = if ct_total > 0 {
                    ct_total as f64 / ct.n as f64 * 100.0
                } else {
                    0.0
                };
                total_row.push(format!("{:.1}%", pct));
            }
            total_row.push("100.0%".to_string());
            rows.push(total_row);
        }

        // ---- Total percentages ----
        if cells.total_percent {
            rows.push(vec![format!("{} (total %)", ct.var1)]);
            for (i, l1) in ct.levels1.iter().enumerate() {
                let mut row = vec![l1.clone()];
                for j in 0..c {
                    row.push(format!("{:.1}%", ct.observed[i][j] as f64 / n * 100.0));
                }
                row.push(format!("{:.1}%", ct.row_totals[i] as f64 / n * 100.0));
                rows.push(row);
            }
            let mut total_row = vec![String::from("Total")];
            for j in 0..c {
                total_row.push(format!("{:.1}%", col_total(ct, j) as f64 / n * 100.0));
            }
            total_row.push("100.0%".to_string());
            rows.push(total_row);
        }

        // ---- The test statistics ----
        let mut note_lines: Vec<String> = Vec::new();
        if stats_sel.chi_square || stats_sel.is_empty() {
            note_lines.push(format!(
                "Pearson Chi-Square = {:.3}, df = {}, Asymp. Sig. (2-sided) = {:.4}",
                ct.chi2, ct.df, ct.p
            ));
        }
        if stats_sel.phi {
            note_lines.push(format!("Phi = {:.3}", ct.phi));
            note_lines.push(format!("Cramer's V = {:.3}", ct.cramers_v));
        }
        if stats_sel.continuity_correction {
            note_lines.push(format!(
                "Continuity correction applied; min expected count = {:.2}",
                ct.min_expected
            ));
        }
        if let Some((r, p, df)) = ct.linear_by_linear {
            note_lines.push(format!(
                "Linear-by-Linear Association: r = {:.3}, df = {}, Asymp. Sig. (2-sided) = {:.4}",
                r, df, p
            ));
        }
        if let Some((p, odds)) = ct.fisher_exact {
            note_lines.push(format!(
                "Fisher's Exact Test: Sig. (2-sided) = {:.4}, odds ratio = {:.3}",
                p, odds
            ));
        }
        note_lines.push(format!("N = {} (missing excluded pairwise).", ct.n));
        if ct.min_expected < 5.0 && ct.min_expected.is_finite() {
            note_lines.push(
                "a. Some expected counts are below 5 - the chi-square may not be reliable.".to_string(),
            );
        }

        Self {
            title: format!("Crosstabulation: {} by {}", ct.var1, ct.var2),
            headers,
            rows,
            note: note_lines.join("\n"),
        }
    }

    pub fn from_crosstabs(ct: &crate::stats::CrosstabsResult) -> Self {
        let mut headers = vec![ct.var1.clone()];
        headers.extend(ct.levels2.iter().cloned());
        headers.push("Total".into());
        let mut rows = Vec::new();
        for (i, l1) in ct.levels1.iter().enumerate() {
            let mut row = vec![l1.clone()];
            for j in 0..ct.levels2.len() {
                row.push(ct.observed[i][j].to_string());
            }
            row.push(ct.row_totals[i].to_string());
            rows.push(row);
        }
        // Column Total row
        let mut total_row = vec![String::from("Total")];
        for (j, l2) in ct.levels2.iter().enumerate() {
            let _ = l2;
            total_row.push(col_total(&ct, j).to_string());
        }
        total_row.push(ct.n.to_string());
        rows.push(total_row);
        Self {
            title: format!("Crosstabulation: {} by {}", ct.var1, ct.var2),
            headers,
            rows,
            note: format!(
                "Pearson Chi-Square = {:.3}, df = {}, Asymp. Sig. (2-sided) = {:.4}; N = {} (missing excluded pairwise)." , ct.chi2, ct.df, ct.p, ct.n
            ),
        }
    }

    /// SPSS ANOVA table: Between/Within/Total rows with SS, df, MS, F, Sig.
    pub fn from_anova(a: &crate::stats::AnovaResult) -> Self {
        let total_ss = a.ss_between + a.ss_within;
        let total_df = a.df_between + a.df_within;
        Self {
            title: format!("ANOVA — {} by {}", a.outcome, a.factor),
            headers: vec![
                String::new(),
                "Sum of Squares".into(),
                "df".into(),
                "Mean Square".into(),
                "F".into(),
                "Sig.".into(),
            ],
            rows: vec![
                vec![
                    "Between Groups".into(),
                    format!("{:.3}", a.ss_between),
                    format!("{:.0}", a.df_between),
                    format!("{:.3}", a.ms_between),
                    format!("{:.3}", a.f),
                    format!("{:.4}", a.p),
                ],
                vec![
                    "Within Groups".into(),
                    format!("{:.3}", a.ss_within),
                    format!("{:.0}", a.df_within),
                    format!("{:.3}", a.ms_within),
                    String::new(),
                    String::new(),
                ],
                vec![
                    "Total".into(),
                    format!("{:.3}", total_ss),
                    format!("{:.0}", total_df),
                    String::new(),
                    String::new(),
                    String::new(),
                ],
            ],
            note: String::new(),
        }
    }

    /// Paired-samples t-test: SPSS one-row layout.
    pub fn from_paired_t(r: &crate::stats::PairedTResult) -> Self {
        Self {
            title: "Paired Samples Test".to_string(),
            headers: vec![
                "Pair".into(),
                "Variable 1".into(),
                "Variable 2".into(),
                "N".into(),
                "Mean 1".into(),
                "Mean 2".into(),
                "Mean Diff.".into(),
                "t".into(),
                "df".into(),
                "Sig. (2-tailed)".into(),
            ],
            rows: vec![vec![
                "1".into(),
                r.var1.clone(),
                r.var2.clone(),
                r.n.to_string(),
                format!("{:.4}", r.mean1),
                format!("{:.4}", r.mean2),
                format!("{:.4}", r.mean_diff),
                format!("{:.4}", r.t),
                format!("{:.1}", r.df),
                format!("{:.4}", r.p),
            ]],
            ..Default::default()
        }
    }
}

fn col_total(ct: &crate::stats::CrosstabsResult, j: usize) -> usize {
    ct.col_totals[j]
}

/// The percentile column labels implied by a statistic set.
fn percent_labels(stats: &crate::stats::StatSet) -> Vec<(f64, String)> {
    if !stats.percentiles {
        return Vec::new();
    }
    let wanted: Vec<f64> = if stats.percentile_values.is_empty() {
        vec![25.0, 50.0, 75.0]
    } else {
        stats.percentile_values.clone()
    };
    wanted
        .into_iter()
        .map(|p| (p, format!("Percentile {}", p as i64)))
        .collect()
}

/// Merge several header rows into one, keeping the order of first
/// appearance.
fn merge_headers(groups: &[Vec<String>]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for g in groups {
        for h in g {
            if !out.contains(h) {
                out.push(h.clone());
            }
        }
    }
    out
}

/// Place a row's values under the right columns of a merged header.
fn merge_row(all: &[String], row: &[String]) -> Vec<String> {
    // The row's first cell is its own label; the rest are positioned by
    // matching the header names in order.
    let mut out = vec![String::new(); all.len()];
    if row.is_empty() {
        return out;
    }
    // Find which of the group's headers this row uses: the headers that
    // appear in the row's values, in order.
    let mut remaining: Vec<&String> = all.iter().collect();
    for (i, v) in row.iter().enumerate() {
        if i == 0 {
            out[0] = v.clone();
            continue;
        }
        // Find the next header in `all` that is not yet used and whose
        // name is not already filled.
        let pos = remaining
            .iter()
            .position(|h| out.iter().all(|o| o.is_empty() || o != *h) && !h.is_empty());
        let _ = pos;
        // Simpler: walk the headers and place each value in the next
        // column whose value is still empty and which is not the label.
        let mut placed = false;
        for (j, h) in all.iter().enumerate() {
            if j == 0 {
                continue;
            }
            if out[j].is_empty() && !placed {
                out[j] = v.clone();
                placed = true;
            }
        }
        if !placed {
            out.push(v.clone());
        }
    }
    out
}
