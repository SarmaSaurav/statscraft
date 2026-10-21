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
        let mut rows = Vec::new();
        for d in descs {
            rows.push(vec![
                d.variable.clone(),
                d.n.to_string(),
                format!("{:.4}", d.mean),
                format!("{:.4}", d.std_dev),
                format!("{:.4}", d.min),
                format!("{:.4}", d.max),
            ]);
        }
        Self {
            title: "Descriptive Statistics".to_string(),
            headers: vec![
                "Variable".into(),
                "N".into(),
                "Mean".into(),
                "Std. Dev.".into(),
                "Min".into(),
                "Max".into(),
            ],
            rows,
            ..Default::default()
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
