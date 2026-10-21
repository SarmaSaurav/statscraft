#[derive(Debug, Clone, Default)]
pub struct OutputTable {
    pub title: String,
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
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
        Self {
            title: format!("Frequencies: {}", var),
            headers: vec!["Value".into(), "Count".into(), "Percent".into()],
            rows,
        }
    }
}
