#[derive(Debug, Clone, Default)]
pub struct OutputTable {
    pub title: String,
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
    /// Footnote rendered beneath the pivot table (SPSS "Notes").
    pub note: String,
    /// An optional chart drawn under the table, as SPSS draws one when
    /// the procedure asks for it.
    pub chart: Option<Chart>,
}

/// A chart to draw in the Output Viewer.
#[derive(Debug, Clone)]
pub struct Chart {
    pub kind: ChartKind,
    pub title: String,
    pub x_label: String,
    pub y_label: String,
    /// The series, one per named group.
    pub series: Vec<ChartSeries>,
}

/// The kind of chart.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChartKind {
    /// A bar chart of counts or percentages.
    Bar,
    /// A histogram of a numeric variable.
    Histogram,
    /// A scatter plot of two numeric variables.
    Scatter,
    /// A box plot of a numeric variable, with the quartiles marked.
    BoxPlot,
    /// A line, used for the ROC curve and for the scree plot.
    Line,
}

/// One series of a chart: a name and a set of points.
#[derive(Debug, Clone)]
pub struct ChartSeries {
    pub name: String,
    /// The x values.
    pub x: Vec<f64>,
    /// The y values.
    pub y: Vec<f64>,
    /// The minimum, lower quartile, median and upper quartile, for a box
    /// plot.
    pub quartiles: Option<(f64, f64, f64, f64)>,
    /// The label for each x value, so a bar chart can show the category
    /// names rather than 0, 1, 2.
    pub x_labels: Vec<String>,
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
            chart: None,
        }
    }

    /// The correlation matrix, with the significance stars SPSS prints
    /// when `Print > Sig.` is on.
    pub fn correlations_with(
        rows_in: &[crate::stats::Correlation],
        show_stars: bool,
        show_descriptives: bool,
        descriptives: &[crate::stats::Descriptive],
    ) -> Self {
        // Build the symmetric matrix the way SPSS does.
        let mut names: Vec<String> = Vec::new();
        for r in rows_in {
            if !names.contains(&r.var1) {
                names.push(r.var1.clone());
            }
            if !names.contains(&r.var2) {
                names.push(r.var2.clone());
            }
        }
        let n = names.len();
        let mut matrix = vec![vec![f64::NAN; n]; n];
        for r in rows_in {
            let i = names.iter().position(|x| *x == r.var1);
            let j = names.iter().position(|x| *x == r.var2);
            if let (Some(i), Some(j)) = (i, j) {
                matrix[i][j] = r.r;
                matrix[j][i] = r.r;
            }
        }

        let mut headers = vec![String::new()];
        headers.extend(names.iter().cloned());
        let mut rows = Vec::new();

        if show_descriptives {
            // The "Descriptive Statistics" block: N, mean and standard
            // deviation for each variable.
            let mut n_row = vec!["N".to_string()];
            let mut mean_row = vec!["Mean".to_string()];
            let mut sd_row = vec!["Std. Deviation".to_string()];
            for name in &names {
                match descriptives.iter().find(|d| d.variable == *name) {
                    Some(d) => {
                        n_row.push(d.n.to_string());
                        mean_row.push(format!("{:.4}", d.mean));
                        sd_row.push(format!("{:.4}", d.std_dev));
                    }
                    None => {
                        n_row.push(String::new());
                        mean_row.push(String::new());
                        sd_row.push(String::new());
                    }
                }
            }
            rows.push(n_row);
            rows.push(mean_row);
            rows.push(sd_row);
            rows.push(vec![String::new()]);
        }

        for (i, name) in names.iter().enumerate() {
            let mut row = vec![name.clone()];
            for j in 0..n {
                let v = matrix[i][j];
                if i == j {
                    row.push("1.000".to_string());
                } else if v.is_nan() {
                    row.push(String::new());
                } else {
                    let mut cell = format!("{:.3}", v);
                    if show_stars {
                        // The p-value for this pair.
                        let p = rows_in
                            .iter()
                            .find(|r| {
                                (r.var1 == *name && r.var2 == names[j])
                                    || (r.var2 == *name && r.var1 == names[j])
                            })
                            .map(|r| r.p)
                            .unwrap_or(f64::NAN);
                        cell.push_str(significance_star(p));
                    }
                    row.push(cell);
                }
            }
            rows.push(row);
        }

        let note = "**. Correlation is significant at the 0.01 level (2-tailed).\n*. Correlation is significant at the 0.05 level (2-tailed).\nn = Correlation is not significant at the 0.05 level (2-tailed).".to_string();

        Self {
            title: "Correlations".to_string(),
            headers,
            rows,
            note,
            chart: None,
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
            chart: None,
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

    /// The t-test output, with the effect size table SPSS shows when
    /// `Options... > Effect size` is ticked.
    pub fn ttest_with(r: &crate::stats::TTestResult, show_effects: bool) -> Self {
        let mut headers = vec![
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
        ];
        let mut rows = vec![vec![
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

        if show_effects {
            // The "Effect Size" table, as SPSS writes it.
            rows.push(vec![String::new()]);
            rows.push(vec!["Effect Size".into()]);
            rows.push(vec![
                "".into(),
                "Value".into(),
                "Std. Error".into(),
                "Sig.".into(),
            ]);
            rows.push(vec![
                "Mean difference".into(),
                format!("{:.4}", r.mean_diff),
                format!("{:.4}", r.std_error_diff),
                String::new(),
            ]);
            rows.push(vec![
                "Std. dev. group 1".into(),
                format!("{:.4}", r.std1),
                String::new(),
                String::new(),
            ]);
            rows.push(vec![
                "Std. dev. group 2".into(),
                format!("{:.4}", r.std2),
                String::new(),
                String::new(),
            ]);
            let n_total = (r.n1 + r.n2) as f64;
            let pooled_var = ((r.n1 as f64 - 1.0) * r.std1.powi(2)
                + (r.n2 as f64 - 1.0) * r.std2.powi(2))
                / (n_total - 2.0).max(1.0);
            let pooled_sd = pooled_var.sqrt();
            rows.push(vec![
                "Std. dev. pooled".into(),
                format!("{:.4}", pooled_sd),
                String::new(),
                String::new(),
            ]);
            rows.push(vec![
                "Cohen's d".into(),
                format!("{:.4}", r.cohens_d),
                String::new(),
                String::new(),
            ]);
            rows.push(vec![
                "Hedges' g".into(),
                format!("{:.4}", r.hedges_g),
                String::new(),
                String::new(),
            ]);
        }

        Self {
            title: "Independent Samples T-Test".to_string(),
            headers,
            rows,
            note: String::new(),
            chart: None,
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
            chart: None,
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
            chart: None,
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
            chart: None,
        }
    }

    /// SPSS ANOVA table: Between/Within/Total rows with SS, df, MS, F, Sig.
    /// The full ANOVA output: descriptives per group, the ANOVA table,
    /// Levene's test and the post-hoc comparisons.
    /// The descriptives of the groups, as SPSS shows them above the
    /// ANOVA table.
    pub fn anova_descriptives(a: &crate::stats::AnovaResult) -> Self {
        let headers = vec![
            String::new(),
            "N".into(),
            "Mean".into(),
            "Std. Dev.".into(),
            "Std. Error Mean".into(),
        ];
        let mut rows: Vec<Vec<String>> = Vec::new();
        for g in &a.groups {
            let se = if g.n > 0 {
                g.std_dev / (g.n as f64).sqrt()
            } else {
                f64::NAN
            };
            rows.push(vec![
                g.name.clone(),
                g.n.to_string(),
                format!("{:.4}", g.mean),
                format!("{:.4}", g.std_dev),
                format!("{:.4}", se),
            ]);
        }
        if !a.groups.is_empty() {
            let total_n: usize = a.groups.iter().map(|g| g.n).sum();
            let grand = a.groups.iter().map(|g| g.mean * g.n as f64).sum::<f64>()
                / total_n.max(1) as f64;
            // The pooled standard deviation of all the cases.
            let pooled_var = a.groups.iter().map(|g| g.std_dev.powi(2) * (g.n as f64 - 1.0)).sum::<f64>()
                / (total_n.max(1) as f64 - a.groups.len() as f64).max(1.0);
            let pooled = pooled_var.max(0.0).sqrt();
            let se = if total_n > 0 {
                pooled / (total_n as f64).sqrt()
            } else {
                f64::NAN
            };
            rows.push(vec![
                "Total".into(),
                total_n.to_string(),
                format!("{:.4}", grand),
                format!("{:.4}", pooled),
                format!("{:.4}", se),
            ]);
        }
        Self {
            title: format!("Descriptives — {} by {}", a.outcome, a.factor),
            headers,
            rows,
            note: String::new(),
            chart: None,
        }
    }

    pub fn anova_with(a: &crate::stats::AnovaResult) -> Self {
        let total_ss = a.ss_between + a.ss_within;
        let total_df = a.df_between + a.df_within;

        // The ANOVA table has its own columns - the sum of squares, the
        // degrees of freedom, the mean square, the F and the
        // significance - and the descriptives above it have a
        // different set. So the descriptives go in their own table.
        let mut headers = vec![
            String::new(),
            "Sum of Squares".into(),
            "df".into(),
            "Mean Square".into(),
            "F".into(),
            "Sig.".into(),
        ];
        let mut rows: Vec<Vec<String>> = Vec::new();

        // ---- The ANOVA table ----
        let anova_rows = vec![
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
        ];

        // ---- Levene's test ----
        let mut note_lines: Vec<String> = Vec::new();
        if let Some(l) = &a.levene {
            note_lines.push(format!(
                "Levene's Test: F({:.0}, {:.0}) = {:.3}, Sig. = {:.4}",
                l.df1, l.df2, l.f, l.p
            ));
        }

        // ---- Post-hoc comparisons ----
        if !a.posthoc.is_empty() {
            let mut ph_rows: Vec<Vec<String>> = Vec::new();
            let mut last_test = String::new();
            for ph in &a.posthoc {
                if ph.test != last_test {
                    ph_rows.push(vec![format!("{} comparisons", ph.test)]);
                    last_test = ph.test.clone();
                }
                ph_rows.push(vec![
                    format!("{} - {}", ph.group1, ph.group2),
                    format!("{:.4}", ph.mean_diff),
                    format!("{:.4}", ph.std_error),
                    format!("{:.3}", ph.statistic),
                    format!("{:.4}", ph.p),
                ]);
            }
            for h in ["Mean Difference", "Std. Error", "Statistic", "Sig."] {
                if !headers.contains(&h.to_string()) {
                    headers.push(h.to_string());
                }
            }
            for r in ph_rows {
                let mut row = vec![r[0].clone()];
                row.extend(r[1..].iter().cloned());
                while row.len() < headers.len() {
                    row.push(String::new());
                }
                row.truncate(headers.len());
                rows.push(row);
            }
        }

        // Merge the ANOVA table columns in.
        for h in ["Sum of Squares", "df", "Mean Square", "F", "Sig."] {
            if !headers.contains(&h.to_string()) {
                headers.push(h.to_string());
            }
        }
        for r in anova_rows {
            let mut row = vec![r[0].clone()];
            row.extend(r[1..].iter().cloned());
            while row.len() < headers.len() {
                row.push(String::new());
            }
            row.truncate(headers.len());
            rows.push(row);
        }

        if note_lines.is_empty() {
            note_lines.push(format!(
                "F({:.0}, {:.0}) = {:.3}, Sig. = {:.4}",
                a.df_between, a.df_within, a.f, a.p
            ));
        }

        Self {
            title: format!("ANOVA — {} by {}", a.outcome, a.factor),
            headers,
            rows,
            note: note_lines.join("\n"),
            chart: None,
        }
    }

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
            chart: None,
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

    // ---- The procedures added in Phase 3 ----

    /// The MEANS output: the group means with their counts, standard
    /// deviations and standard errors.
    pub fn from_means(m: &crate::stats::MeansResult) -> Self {
        let headers = vec![
            String::new(),
            "N".into(),
            "Mean".into(),
            "Std. Deviation".into(),
            "Std. Error of Mean".into(),
        ];
        let mut rows = Vec::new();
        for g in &m.groups {
            rows.push(vec![
                g.name.clone(),
                g.n.to_string(),
                format!("{:.4}", g.mean),
                format!("{:.4}", g.std_dev),
                format!("{:.4}", g.std_error),
            ]);
        }
        rows.push(vec![
            "Total".to_string(),
            m.total_n.to_string(),
            format!("{:.4}", m.grand_mean),
            String::new(),
            String::new(),
        ]);
        Self {
            title: format!("Report: {} by {}", m.outcome, m.group_var),
            headers,
            rows,
            note: String::new(),
            chart: None,
        }
    }

    /// The EXPLORE output: the descriptives for each group, laid out as
    /// SPSS lays it out.
    pub fn from_explore(e: &crate::stats::ExploreResult) -> Self {
        let headers = vec![String::new(), "Statistic".into(), "Std. Error".into()];
        let mut rows = Vec::new();

        for g in &e.groups {
            rows.push(vec![g.name.clone()]);
            rows.push(vec![
                String::new(),
                "Mean".into(),
                format!("{:.4}", g.mean),
                format!("{:.5}", g.std_error),
            ]);
            rows.push(vec![
                String::new(),
                "5% Trimmed Mean".into(),
                format!("{:.4}", g.trimmed_mean),
                String::new(),
            ]);
            rows.push(vec![
                String::new(),
                "Median".into(),
                format!("{:.4}", g.median),
                String::new(),
            ]);
            rows.push(vec![
                String::new(),
                "Variance".into(),
                format!("{:.4}", g.std_dev * g.std_dev),
                String::new(),
            ]);
            rows.push(vec![
                String::new(),
                "Std. Deviation".into(),
                format!("{:.4}", g.std_dev),
                String::new(),
            ]);
            rows.push(vec![
                String::new(),
                "Minimum".into(),
                format!("{:.4}", g.min),
                String::new(),
            ]);
            rows.push(vec![
                String::new(),
                "Maximum".into(),
                format!("{:.4}", g.max),
                String::new(),
            ]);
            rows.push(vec![
                String::new(),
                "Range".into(),
                format!("{:.4}", g.range),
                String::new(),
            ]);
            rows.push(vec![
                String::new(),
                "Interquartile Range".into(),
                format!("{:.4}", g.iqr),
                String::new(),
            ]);
        }

        let note = "a. Based on the trimmed mean.".to_string();
        Self {
            title: format!("Explore: {}", e.outcome),
            headers,
            rows,
            note,
            chart: None,
        }
    }

    /// The normality-test table of the EXPLORE output.
    pub fn explore_normality(e: &crate::stats::ExploreResult) -> Self {
        let mut headers = vec![String::new(), "Statistic".into()];
        for g in &e.groups {
            headers.push(g.name.clone());
        }
        let mut rows = Vec::new();
        rows.push(vec![
            String::new(),
            "Statistic".into(),
            e.groups
                .iter()
                .map(|g| format!("{:.3}", g.ks_stat))
                .collect::<Vec<_>>()
                .join(" | "),
        ]);
        rows.push(vec![
            String::new(),
            "df".into(),
            e.groups
                .iter()
                .map(|g| (g.n.saturating_sub(1)).to_string())
                .collect::<Vec<_>>()
                .join(" | "),
        ]);
        rows.push(vec![
            String::new(),
            "Sig.".into(),
            e.groups
                .iter()
                .map(|g| format!("{:.4}", g.ks_p))
                .collect::<Vec<_>>()
                .join(" | "),
        ]);
        Self {
            title: format!("Normality Test: {}", e.outcome),
            headers,
            rows,
            note: "a. Kolmogorov-Smirnov test for normality.".to_string(),
            chart: None,
        }
    }

    /// The Mann-Whitney U output.
    pub fn from_mann_whitney(r: &crate::stats::MannWhitneyResult) -> Self {
        Self {
            title: format!("Mann-Whitney Test: {} by {}", r.outcome, r.group_var),
            headers: vec![
                "Outcome".into(),
                "Group var".into(),
                "Group 1".into(),
                "Group 2".into(),
                "N1".into(),
                "N2".into(),
                "Rank Sum 1".into(),
                "Rank Sum 2".into(),
                "U1".into(),
                "U2".into(),
                "z".into(),
                "p (2-tailed)".into(),
            ],
            rows: vec![vec![
                r.outcome.clone(),
                r.group_var.clone(),
                r.group1.clone(),
                r.group2.clone(),
                r.n1.to_string(),
                r.n2.to_string(),
                format!("{:.1}", r.rank_sum1),
                format!("{:.1}", r.rank_sum2),
                format!("{:.3}", r.u1),
                format!("{:.3}", r.u2),
                format!("{:.3}", r.z),
                format!("{:.4}", r.p),
            ]],
            note: format!(
                "a. Based on the normal approximation with tie and continuity correction. Common-language effect size = {:.4}.",
                r.effect_size
            ),
            chart: None,
        }
    }

    /// The Wilcoxon signed-rank output.
    pub fn from_wilcoxon(r: &crate::stats::WilcoxonResult) -> Self {
        Self {
            title: format!("Wilcoxon Signed-Rank Test: {} - {}", r.var1, r.var2),
            headers: vec![
                "Variable 1".into(),
                "Variable 2".into(),
                "N".into(),
                "Sum of Ranks (W+)".into(),
                "Sum of Ranks (W-)".into(),
                "z".into(),
                "p (2-tailed)".into(),
            ],
            rows: vec![vec![
                r.var1.clone(),
                r.var2.clone(),
                r.n.to_string(),
                format!("{:.1}", r.w_plus),
                format!("{:.1}", r.w_minus),
                format!("{:.3}", r.z),
                format!("{:.4}", r.p),
            ]],
            note: "a. Based on the normal approximation with tie correction.".to_string(),
            chart: None,
        }
    }

    /// The Kruskal-Wallis output.
    pub fn from_kruskal_wallis(r: &crate::stats::KruskalWallisResult) -> Self {
        let mut rows: Vec<Vec<String>> = Vec::new();
        for (name, sum, n) in &r.rank_sums {
            rows.push(vec![
                name.clone(),
                n.to_string(),
                format!("{:.1}", sum),
                format!("{:.4}", *sum / *n as f64),
            ]);
        }
        rows.push(vec![String::new()]);
        rows.push(vec![
            "Chi-Square".into(),
            format!("{:.3}", r.h),
            String::new(),
            String::new(),
        ]);
        rows.push(vec![
            "df".into(),
            format!("{:.0}", r.df),
            String::new(),
            String::new(),
        ]);
        rows.push(vec![
            "Sig.".into(),
            format!("{:.4}", r.p),
            String::new(),
            String::new(),
        ]);
        Self {
            title: format!("Kruskal-Wallis Test: {} by {}", r.outcome, r.factor),
            headers: vec![String::new(), "N".into(), "Rank Sum".into(), "Mean Rank".into()],
            rows,
            note: "a. Based on the chi-square approximation.".to_string(),
            chart: None,
        }
    }

    /// The Friedman output.
    pub fn from_friedman(r: &crate::stats::FriedmanResult) -> Self {
        let mut rows: Vec<Vec<String>> = Vec::new();
        for (name, mr) in &r.mean_ranks {
            rows.push(vec![name.clone(), format!("{:.4}", mr)]);
        }
        rows.push(vec![String::new()]);
        rows.push(vec!["Chi-Square".into(), format!("{:.3}", r.chi_square)]);
        rows.push(vec!["df".into(), format!("{:.0}", r.df)]);
        rows.push(vec!["Sig.".into(), format!("{:.4}", r.p)]);
        Self {
            title: format!("Friedman Test: {}", r.var1),
            headers: vec![String::new(), "Mean Rank".into()],
            rows,
            note: "a. Based on the chi-square approximation.".to_string(),
            chart: None,
        }
    }

    /// The frequencies table with a bar chart of the counts, the way
    /// SPSS draws one when `Charts > Bar chart` is asked for.
    pub fn frequencies_with_chart(
        var: &str,
        rows_in: &[crate::stats::FreqRow],
        chart: Option<crate::output::Chart>,
    ) -> Self {
        let mut t = Self::from_frequencies(var, rows_in);
        t.chart = chart;
        t
    }

    /// The regression output with a scatter plot of the fit.
    pub fn regression_with_chart(
        r: &crate::stats::RegressionResult,
        chart: Option<crate::output::Chart>,
    ) -> Self {
        let mut t = Self::regression_with(r);
        t.chart = chart;
        t
    }

    // ---- The procedures added in Phase 4 ----

    /// The reliability analysis output: the "Reliability Statistics"
    /// table, the item statistics and the scale statistics.
    pub fn from_reliability(r: &crate::stats::ReliabilityResult) -> Self {
        // The "Reliability Statistics" block.
        let mut headers = vec![
            "Cronbach's Alpha".into(),
            "N of Items".into(),
            "N".into(),
        ];
        if r.standardised_alpha.is_finite() {
            headers.push("Standardised Alpha".into());
        }
        let mut rows = vec![vec![
            format!("{:.3}", r.alpha),
            r.n_items.to_string(),
            r.n_cases.to_string(),
            format!("{:.3}", r.standardised_alpha),
        ]];

        let mut note_lines = Vec::new();
        if !r.weak_items.is_empty() {
            note_lines.push(format!(
                "a. The item(s) {} correlate less than 0.3 with the total score.",
                r.weak_items.join(", ")
            ));
        }

        Self {
            title: format!("Reliability Statistics: {}", r.name),
            headers,
            rows,
            note: note_lines.join("\n"),
            chart: None,
        }
    }

    /// The item statistics table of the reliability analysis.
    pub fn reliability_items(r: &crate::stats::ReliabilityResult) -> Self {
        let headers = vec![
            "Item".into(),
            "Item-Total Correlation".into(),
            "Corrected Item-Total Correlation".into(),
            "Alpha if Item Deleted".into(),
        ];
        let mut rows = Vec::new();
        for (i, name) in r.item_names.iter().enumerate() {
            let corr = r
                .item_total_correlations
                .get(i)
                .map(|(_, c)| *c)
                .unwrap_or(f64::NAN);
            let corrected = r
                .corrected_item_total
                .get(i)
                .map(|(_, c)| *c)
                .unwrap_or(f64::NAN);
            let alpha = r
                .alpha_if_deleted
                .get(i)
                .map(|(_, a)| *a)
                .unwrap_or(f64::NAN);
            rows.push(vec![
                name.clone(),
                format!("{:.3}", corr),
                format!("{:.3}", corrected),
                format!("{:.3}", alpha),
            ]);
        }
        Self {
            title: format!("Item Statistics: {}", r.name),
            headers,
            rows,
            note: String::new(),
            chart: None,
        }
    }

    /// The factor analysis output: the total variance explained, the
    /// component matrix and the communalities.
    pub fn from_factor(f: &crate::stats::FactorResult) -> Self {
        // The "Total Variance Explained" table.
        let mut headers = vec![
            "Component".into(),
            "Initial Eigenvalue Total".into(),
            "% of Variance".into(),
            "Cumulative %".into(),
        ];
        let mut rows = Vec::new();
        for i in 0..f.eigenvalues.len() {
            let is_retained = i < f.n_components;
            let eigen = format!("{:.3}", f.eigenvalues[i]);
            let eigen = if is_retained {
                format!("{:.3}", f.retained_eigenvalues[i])
            } else {
                eigen
            };
            rows.push(vec![
                (i + 1).to_string(),
                eigen,
                format!("{:.1}", f.variance_percent[i]),
                format!("{:.1}", f.cumulative_percent[i]),
            ]);
        }

        let mut note_lines = Vec::new();
        if let Some((chi, p)) = f.bartlett {
            note_lines.push(format!(
                "a. Bartlett's test of sphericity: chi-square = {:.1}, df = {}, Sig. = {:.4}",
                chi,
                (f.n_items * (f.n_items - 1) / 2) as i64,
                p
            ));
        }
        note_lines.push(format!(
            "b. The Kaiser-Meyer-Olkin measure of sampling adequacy is {:.3}.",
            f.kmo
        ));

        Self {
            title: "Factor Analysis".to_string(),
            headers,
            rows,
            note: note_lines.join("\n"),
            chart: None,
        }
    }

    /// The "Component Matrix" table of the factor analysis.
    pub fn factor_component_matrix(f: &crate::stats::FactorResult) -> Self {
        let mut headers = vec!["Item".into()];
        for c in 0..f.n_components {
            headers.push(format!("Component {}", c + 1));
        }
        let mut rows = Vec::new();
        for (i, name) in f.item_names.iter().enumerate() {
            let mut row = vec![name.clone()];
            for c in 0..f.n_components {
                row.push(format!("{:.3}", f.loadings[i][c]));
            }
            rows.push(row);
        }
        Self {
            title: "Component Matrix".to_string(),
            headers,
            rows,
            note: "a. Extraction Method: Principal Component Analysis.".to_string(),
            chart: None,
        }
    }

    /// The communalities table of the factor analysis.
    pub fn factor_communalities(f: &crate::stats::FactorResult) -> Self {
        let mut rows = Vec::new();
        for (i, name) in f.item_names.iter().enumerate() {
            rows.push(vec![name.clone(), format!("{:.3}", f.communalities[i])]);
        }
        Self {
            title: "Communalities".to_string(),
            headers: vec!["Item".into(), "Communality".into()],
            rows,
            note: String::new(),
            chart: None,
        }
    }

    /// The Welch and Brown-Forsythe test output.
    pub fn anova_variance_tests(t: &crate::stats::AnovaVarianceTests) -> Self {
        let headers = vec![
            "Test".into(),
            "Statistic".into(),
            "df1".into(),
            "df2".into(),
            "Sig.".into(),
        ];
        let rows = vec![
            vec![
                "Welch's Test".to_string(),
                format!("{:.3}", t.welch.0),
                format!("{:.0}", t.welch.1),
                format!("{:.3}", t.welch.2),
                format!("{:.4}", t.welch.3),
            ],
            vec![
                "Brown-Forsythe Test".to_string(),
                format!("{:.3}", t.brown_forsythe.0),
                format!("{:.0}", t.brown_forsythe.1),
                format!("{:.3}", t.brown_forsythe.2),
                format!("{:.4}", t.brown_forsythe.3),
            ],
        ];
        Self {
            title: "Variance Tests".to_string(),
            headers,
            rows,
            note: "a. Welch's test does not assume equal group variances; Brown-Forsythe's test is robust to outliers.".to_string(),
            chart: None,
        }
    }

    /// The ROC curve output.
    /// The summary of an ROC curve: the area, its standard error and
    /// the confidence interval.
    pub fn roc_summary(r: &crate::stats::RocResult) -> Self {
        let headers = vec![
            "Area Under the Curve".into(),
            "Std. Error".into(),
            "95% CI Lower".into(),
            "95% CI Upper".into(),
            "Positive Group".into(),
            "N Positive".into(),
            "N Negative".into(),
        ];
        let rows = vec![vec![
            format!("{:.3}", r.auc),
            format!("{:.3}", r.auc_se),
            format!("{:.3}", r.auc_ci.0),
            format!("{:.3}", r.auc_ci.1),
            r.positive_group.clone(),
            r.n_positive.to_string(),
            r.n_negative.to_string(),
        ]];
        // The curve itself: the false-positive rate against the true-
        // positive rate, with the diagonal of the no-discrimination
        // line as a second series.
        let mut x = vec![0.0];
        let mut y = vec![0.0];
        for p in &r.points {
            x.push(1.0 - p.specificity);
            y.push(p.sensitivity);
        }
        x.push(1.0);
        y.push(1.0);
        let diagonal_x: Vec<f64> = vec![0.0, 1.0];
        let diagonal_y: Vec<f64> = vec![0.0, 1.0];
        let chart = Some(Chart {
            kind: ChartKind::Line,
            title: format!("ROC Curve: {}", r.outcome),
            x_label: "1 - Specificity".to_string(),
            y_label: "Sensitivity".to_string(),
            series: vec![
                ChartSeries {
                    name: r.outcome.clone(),
                    x,
                    y,
                    quartiles: None,
                    x_labels: Vec::new(),
                },
                ChartSeries {
                    name: "No discrimination".to_string(),
                    x: diagonal_x,
                    y: diagonal_y,
                    quartiles: None,
                    x_labels: Vec::new(),
                },
            ],
        });

        Self {
            title: format!("ROC Curve: {} by {}", r.outcome, r.group_var),
            headers,
            rows,
            note: "a. A higher test value counts as a positive result.".to_string(),
            chart,
        }
    }

    pub fn from_roc(r: &crate::stats::RocResult) -> Self {
        let mut headers = vec![
            "Cut-off".into(),
            "Sensitivity".into(),
            "Specificity".into(),
            "Youden Index".into(),
        ];
        let mut rows = Vec::new();
        for p in &r.points {
            rows.push(vec![
                format!("{:.2}", p.cutoff),
                format!("{:.3}", p.sensitivity),
                format!("{:.3}", p.specificity),
                format!("{:.3}", p.youden),
            ]);
        }
        Self {
            title: "Area Under the Curve".to_string(),
            headers,
            rows,
            note: format!(
                "The positive group is '{}' (n = {}); the negative group has n = {}.",
                r.positive_group, r.n_positive, r.n_negative
            ),
            chart: None,
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

/// The significance star SPSS prints next to a correlation.
pub fn significance_star(p: f64) -> &'static str {
    if !p.is_finite() {
        return "";
    }
    if p < 0.01 {
        "**"
    } else if p < 0.05 {
        "*"
    } else {
        ""
    }
}

/// Build a bar chart of the counts from a set of frequency rows.
pub fn bar_chart_from_frequencies(
    var: &str,
    rows_in: &[crate::stats::FreqRow],
) -> crate::output::Chart {
    // Skip the "Total" row, which the table adds separately.
    let data: Vec<&crate::stats::FreqRow> = rows_in
        .iter()
        .filter(|r| r.value != "Total")
        .collect();
    crate::output::Chart {
        kind: crate::output::ChartKind::Bar,
        title: format!("Bar Chart: {}", var),
        x_label: var.to_string(),
        y_label: "Count".to_string(),
        series: vec![crate::output::ChartSeries {
            name: var.to_string(),
            x: (0..data.len()).map(|i| i as f64).collect(),
            y: data.iter().map(|r| r.count as f64).collect(),
            quartiles: None,
            x_labels: data.iter().map(|r| r.value.clone()).collect(),
        }],
    }
}

/// Build a histogram chart from a histogram's bins.
pub fn histogram_chart(h: &crate::stats::Histogram) -> crate::output::Chart {
    crate::output::Chart {
        kind: crate::output::ChartKind::Histogram,
        title: format!("Histogram: {}", h.variable),
        x_label: h.variable.clone(),
        y_label: "Count".to_string(),
        series: vec![crate::output::ChartSeries {
            name: h.variable.clone(),
            x: h.centres.clone(),
            y: h.counts.clone(),
            quartiles: None,
            x_labels: Vec::new(),
        }],
    }
}

/// Build a scatter chart from a set of points.
pub fn scatter_chart(s: &crate::stats::Scatter) -> crate::output::Chart {
    crate::output::Chart {
        kind: crate::output::ChartKind::Scatter,
        title: format!("Scatter Plot: {} by {}", s.y_var, s.x_var),
        x_label: s.x_var.clone(),
        y_label: s.y_var.clone(),
        series: vec![crate::output::ChartSeries {
            name: format!("{} / {}", s.x_var, s.y_var),
            x: s.x.clone(),
            y: s.y.clone(),
            quartiles: None,
            x_labels: Vec::new(),
        }],
    }
}
