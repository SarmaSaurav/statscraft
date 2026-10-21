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
