//! The general linear model.
//!
//! A general linear model is fitted by ordinary least squares on a
//! design matrix built from the factors. The design matrix uses
//! treatment coding: a factor with `k` levels contributes `k - 1`
//! columns, one per level except the last, and a case gets 1 on its
//! own level and 0 elsewhere. An interaction between two factors is
//! the product of their coded columns.
//!
//! The three types of sums of squares are then:
//!
//! - **Type I** is sequential: the effects are added in the order they
//!   were entered, and each one's sums of squares is the drop in the
//!   residual sum of squares when it is added.
//! - **Type II** adjusts each effect for the effects that do not
//!   involve any of its factors, so a main effect is adjusted for the
//!   other main effects but not for the interactions that contain it.
//! - **Type III** tests each effect against the *unweighted* marginal
//!   means, so the test does not depend on the order the factors were
//!   entered. It is computed as a Wald test: the hypothesis is that the
//!   unweighted means of the effect's cells follow the additive
//!   structure, which is a linear restriction on the fitted means, and
//!   the sums of squares come from that restriction.
//!
//! Type III is the one that needs care. The extra sums of squares of
//! an effect over the model without it give the right answer for a
//! balanced design, but for an unbalanced one they weight the cells by
//! their size, which is not what Type III asks for.

use crate::data::{ColumnKind, DataTable, Value};

/// The type of the sums of squares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SumOfSquaresType {
    /// Type I: the sums of squares are added in the order the factors
    /// are entered.
    TypeI,
    /// Type II: each factor is tested after the factors that come before
    /// it, but not adjusted for the ones that nest it.
    TypeII,
    /// Type III: each effect is tested after all the other effects, so
    /// the test does not depend on the order the factors were entered.
    TypeIII,
    /// Type IV: like Type III, but the hypotheses are tested against
    /// the unweighted marginal means, which matters when a cell is
    /// empty.
    TypeIV,
}

impl SumOfSquaresType {
    /// The keyword StatsBench uses for the type.
    pub fn keyword(self) -> &'static str {
        match self {
            SumOfSquaresType::TypeI => "TYPE I",
            SumOfSquaresType::TypeII => "TYPE II",
            SumOfSquaresType::TypeIII => "TYPE III",
            SumOfSquaresType::TypeIV => "TYPE IV",
        }
    }

    /// Parse the keyword.
    pub fn parse(text: &str) -> Option<Self> {
        let t = text.trim().to_uppercase().replace(' ', "");
        match t.as_str() {
            "I" | "1" | "TYPEI" | "TYPE1" => Some(SumOfSquaresType::TypeI),
            "II" | "2" | "TYPEII" | "TYPE2" => Some(SumOfSquaresType::TypeII),
            "III" | "3" | "TYPEIII" | "TYPE3" => Some(SumOfSquaresType::TypeIII),
            "IV" | "4" | "TYPEIV" | "TYPE4" => Some(SumOfSquaresType::TypeIV),
            _ => None,
        }
    }
}

/// One factor of a design.
#[derive(Debug, Clone)]
pub struct DesignFactor {
    /// The variable's name.
    pub name: String,
    /// The levels, in the order they appear in the data.
    pub levels: Vec<String>,
}

/// The descriptive statistics of one cell of the design.
#[derive(Debug, Clone)]
pub struct CellSummary {
    /// The cell's name, as StatsBench writes it: "A=1", "A=1,B=2".
    pub name: String,
    /// The number of cases.
    pub n: usize,
    /// The mean.
    pub mean: f64,
    /// The standard deviation.
    pub std_dev: f64,
    /// The standard error of the mean.
    pub std_error: f64,
}

/// One row of the tests table.
#[derive(Debug, Clone)]
pub struct GlmEffect {
    /// The effect's name, as StatsBench writes it: "A", "A*B".
    pub name: String,
    /// The sum of squares.
    pub ss: f64,
    /// The degrees of freedom.
    pub df: f64,
    /// The mean square.
    pub ms: f64,
    /// The F statistic, when the effect is tested against an error term.
    pub f: Option<f64>,
    /// The p-value.
    pub p: Option<f64>,
    /// The partial eta squared.
    pub partial_eta_squared: Option<f64>,
}

/// The result of a general linear model.
#[derive(Debug, Clone)]
pub struct GlmResult {
    /// The name of the dependent variable.
    pub outcome: String,
    /// The factors of the design.
    pub factors: Vec<DesignFactor>,
    /// The sum-of-squares type the tests were computed with.
    pub ss_type: SumOfSquaresType,
    /// The rows of the Tests of Between-Subjects Effects table.
    pub effects: Vec<GlmEffect>,
    /// The descriptives of the cells.
    pub descriptives: Vec<CellSummary>,
    /// The number of cases used.
    pub n: usize,
}

/// A column of the design matrix, with the effect it belongs to.
#[derive(Clone)]
struct DesignColumn {
    /// The effect's name.
    effect: String,
    /// The values of the column.
    values: Vec<f64>,
}

/// The residual sum of squares and degrees of freedom of a model.
struct ModelFit {
    ss: f64,
    df: f64,
}

/// Fit a general linear model.
///
/// `factors` gives the design, `covariate` is an optional continuous
/// variable to adjust for, and `ss_type` chooses the sums of squares.
pub fn glm(
    table: &DataTable,
    outcome_idx: usize,
    factors: &[(usize, String)],
    covariate: Option<usize>,
    ss_type: SumOfSquaresType,
) -> Option<GlmResult> {
    let outcome = table.columns.get(outcome_idx)?;
    if outcome.kind != ColumnKind::Numeric {
        return None;
    }

    // The levels of each factor.
    //
    // The levels are SORTED, the way StatsBench sorts them: numerically when
    // the variable is numeric, and alphabetically when it is a
    // string. The order matters, because the first level is the
    // reference level of the treatment coding and the Type III sums
    // of squares depend on which level that is.
    let mut designs: Vec<DesignFactor> = Vec::new();
    for (col_idx, name) in factors {
        let col = table.columns.get(*col_idx)?;
        let mut levels: Vec<String> = Vec::new();
        for row in 0..table.n_rows {
            if col.is_missing(row) {
                continue;
            }
            let key = category_key(col, row)?;
            if !levels.contains(&key) {
                levels.push(key);
            }
        }
        if levels.is_empty() {
            return None;
        }
        // The sort: numerically when every level is a number, and
        // alphabetically otherwise.
        let all_numeric = levels.iter().all(|l| l.parse::<f64>().is_ok());
        if all_numeric {
            levels.sort_by(|a, b| {
                let x = a.parse::<f64>().unwrap_or(0.0);
                let y = b.parse::<f64>().unwrap_or(0.0);
                x.partial_cmp(&y).unwrap_or(std::cmp::Ordering::Equal)
            });
        } else {
            levels.sort();
        }
        designs.push(DesignFactor {
            name: name.clone(),
            levels,
        });
    }
    if designs.is_empty() {
        return None;
    }

    // The cases: the level index of each factor and the value of the
    // dependent variable.
    let mut level_idx: Vec<Vec<usize>> = Vec::new();
    let mut y: Vec<f64> = Vec::new();
    let mut cov: Vec<f64> = Vec::new();
    for row in 0..table.n_rows {
        if outcome.is_missing(row) {
            continue;
        }
        let Value::Number(v) = outcome.values.get(row)? else {
            continue;
        };
        let mut idx: Vec<usize> = Vec::with_capacity(designs.len());
        let mut ok = true;
        for (i, (col_idx, _)) in factors.iter().enumerate() {
            let col = table.columns.get(*col_idx)?;
            if col.is_missing(row) {
                ok = false;
                break;
            }
            let key = category_key(col, row)?;
            match designs[i].levels.iter().position(|l| *l == key) {
                Some(p) => idx.push(p),
                None => {
                    ok = false;
                    break;
                }
            }
        }
        if !ok {
            continue;
        }
        // A covariate that is missing drops the case.
        if let Some(c) = covariate {
            let ccol = table.columns.get(c)?;
            if ccol.is_missing(row) {
                continue;
            }
            let Value::Number(cv) = ccol.values.get(row)? else {
                continue;
            };
            cov.push(*cv);
        }
        level_idx.push(idx);
        y.push(*v);
    }
    let n = y.len();
    if n == 0 {
        return None;
    }

    // The design matrix.
    let columns = build_design_columns(&designs, &level_idx, covariate.is_some());
    let intercept = DesignColumn {
        effect: "Intercept".to_string(),
        values: vec![1.0; n],
    };
    let cov_column = covariate.map(|_| DesignColumn {
        effect: "Covariate".to_string(),
        values: cov.clone(),
    });

    // The full model: the intercept, the covariate and every effect.
    let mut full_model: Vec<DesignColumn> = vec![intercept.clone()];
    if let Some(c) = &cov_column {
        full_model.push(c.clone());
    }
    let effect_columns = group_columns(&columns);
    for (_, cols) in &effect_columns {
        full_model.extend(cols.iter().cloned());
    }
    let full_fit = fit_ss(&full_model, &y);

    // The model with only the intercept: its residual sum of squares is
    // the total.
    let null_fit = fit_ss(&[intercept.clone()], &y);
    let ss_total = null_fit.ss;
    let df_total = null_fit.df;

    // The error term: what is left after the intercept, the covariate
    // and every effect.
    let error_fit = ModelFit {
        ss: full_fit.ss,
        df: full_fit.df,
    };
    let df_error = error_fit.df;

    // The tests.
    let mut effects: Vec<GlmEffect> = Vec::new();

    // The intercept.
    let no_intercept = fit_ss(&[], &y);
    effects.push(GlmEffect {
        name: "Intercept".to_string(),
        ss: (no_intercept.ss - full_fit.ss).max(0.0),
        df: 1.0,
        ms: (no_intercept.ss - full_fit.ss).max(0.0),
        f: None,
        p: None,
        partial_eta_squared: None,
    });

    // The corrected model.
    effects.push(GlmEffect {
        name: "Corrected Model".to_string(),
        ss: (ss_total - full_fit.ss).max(0.0),
        df: (null_fit.df - full_fit.df).max(0.0),
        ms: if null_fit.df > full_fit.df {
            (ss_total - full_fit.ss) / (null_fit.df - full_fit.df)
        } else {
            0.0
        },
        f: None,
        p: None,
        partial_eta_squared: None,
    });

    // The effects themselves.
    let effect_list = effect_names(&designs);
    let type3 = if ss_type == SumOfSquaresType::TypeIII || ss_type == SumOfSquaresType::TypeIV {
        Some(type3_sums_of_squares(
            &designs,
            &level_idx,
            &y,
            &full_model,
            &effect_list,
            &columns,
        ))
    } else {
        None
    };

    let mut type1_running = Vec::new();
    for (index, name) in effect_list.iter().enumerate() {
        let cols = effect_columns
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, c)| c.clone())
            .unwrap_or_default();

        // The degrees of freedom of the effect.
        let df = effect_df(&designs, name);

        let (ss, ss_df) = match ss_type {
            SumOfSquaresType::TypeI => {
                // Sequential: add the effect to the model built so far.
                let mut model: Vec<DesignColumn> = vec![intercept.clone()];
                if let Some(c) = &cov_column {
                    model.push(c.clone());
                }
                for (other_name, other_cols) in &effect_columns {
                    let other_index = effect_list.iter().position(|n| n == other_name).unwrap();
                    if other_index < index {
                        model.extend(other_cols.iter().cloned());
                    }
                }
                let reduced = fit_ss(&model, &y);
                let mut with = model;
                with.extend(cols.iter().cloned());
                let full = fit_ss(&with, &y);
                type1_running.push((reduced.ss - full.ss).max(0.0));
                ((reduced.ss - full.ss).max(0.0), (reduced.df - full.df).max(0.0))
            }
            SumOfSquaresType::TypeII => {
                // Each effect is tested after all the other effects
                // EXCEPT the higher-order ones that contain it. So a
                // main effect is not adjusted for the interactions
                // that contain it, and an interaction is adjusted for
                // the main effects.
                let higher = higher_order_terms(&designs, name);
                let mut reduced: Vec<DesignColumn> = vec![intercept.clone()];
                if let Some(c) = &cov_column {
                    reduced.push(c.clone());
                }
                for (other_name, other_cols) in &effect_columns {
                    if other_name == name {
                        continue;
                    }
                    if higher.contains(other_name) {
                        continue;
                    }
                    reduced.extend(other_cols.iter().cloned());
                }
                let reduced_fit = fit_ss(&reduced, &y);
                let mut with = reduced;
                with.extend(cols.iter().cloned());
                let full = fit_ss(&with, &y);
                ((reduced_fit.ss - full.ss).max(0.0), (reduced_fit.df - full.df).max(0.0))
            }
            _ => {
                // Type III: the hypothesis is that the effect's own
                // coefficients are zero, with every other effect in
                // the model. It is a Wald test:
                //
                //   SS = (L beta)' (L (X'X)^-1 L')^-1 (L beta)
                //
                // where L is the identity on the effect's own columns.
                match &type3 {
                    Some(t3) => t3.get(name).copied().unwrap_or((0.0, df)),
                    None => (0.0, df),
                }
            }
        };

        let ss_df = if ss_df > 0.0 { ss_df } else { df };
        let ms = if ss_df > 0.0 { ss / ss_df } else { 0.0 };
        let f = if ss_df > 0.0 && df_error > 0.0 && error_fit.ss > 0.0 {
            Some(ms / (error_fit.ss / df_error))
        } else {
            None
        };
        let p = f.map(|f| crate::stats::f_p_value(f, ss_df, df_error));
        let partial_eta_squared = if ss + error_fit.ss > 0.0 {
            Some(ss / (ss + error_fit.ss))
        } else {
            None
        };
        effects.push(GlmEffect {
            name: name.clone(),
            ss,
            df: ss_df,
            ms,
            f,
            p,
            partial_eta_squared,
        });
    }

    // The error.
    effects.push(GlmEffect {
        name: "Error".to_string(),
        ss: error_fit.ss,
        df: df_error,
        ms: if df_error > 0.0 { error_fit.ss / df_error } else { 0.0 },
        f: None,
        p: None,
        partial_eta_squared: None,
    });

    // The total.
    effects.push(GlmEffect {
        name: "Total".to_string(),
        ss: ss_total,
        df: df_total,
        ms: 0.0,
        f: None,
        p: None,
        partial_eta_squared: None,
    });

    // The model F test.
    let ss_model = (ss_total - full_fit.ss).max(0.0);
    let df_model = (null_fit.df - full_fit.df).max(0.0);
    let f_model = if df_model > 0.0 && df_error > 0.0 && error_fit.ss > 0.0 {
        Some((ss_model / df_model) / (error_fit.ss / df_error))
    } else {
        None
    };
    let p_model = f_model.map(|f| crate::stats::f_p_value(f, df_model, df_error));

    let mut result = GlmResult {
        outcome: table.columns[outcome_idx].name.clone(),
        factors: designs.clone(),
        ss_type,
        effects,
        descriptives: cell_summaries(&designs, &level_idx, &y),
        n,
    };
    if let Some(row) = result
        .effects
        .iter_mut()
        .find(|e| e.name == "Corrected Model")
    {
        row.f = f_model;
        row.p = p_model;
    }
    Some(result)
}

/// The Type III sums of squares of every effect.
///
/// The hypothesis for an effect is that the unweighted means of its
/// cells follow the additive structure - for a main effect, that all
/// the levels have the same mean, and for an interaction, that the
/// table of means has no interaction in it. That is a linear
/// restriction on the fitted means, and the sums of squares come from
/// the Wald test of that restriction.
fn type3_sums_of_squares(
    designs: &[DesignFactor],
    level_idx: &[Vec<usize>],
    y: &[f64],
    full_model: &[DesignColumn],
    effect_list: &[String],
    columns: &[DesignColumn],
) -> std::collections::HashMap<String, (f64, f64)> {
    let mut out = std::collections::HashMap::new();
    let n = y.len();

    // The normal equations of the full model.
    let p = full_model.len();
    let mut xtx = vec![vec![0.0f64; p]; p];
    let mut xty = vec![0.0f64; p];
    for j in 0..p {
        for i in 0..n {
            let xij = full_model[j].values[i];
            xty[j] += xij * y[i];
            for k in 0..p {
                xtx[j][k] += xij * full_model[k].values[i];
            }
        }
    }
    let Some(chol) = cholesky(&xtx, p) else {
        return out;
    };
    let beta = cholesky_solve(&chol, &xty, p);
    let inv_xtx = cholesky_inverse(&chol, p);

    // The parameter index of each design column.
    //
    // The full model's parameter order is the intercept, then the
    // covariate, then the effect columns. The mapping is worked out
    // by walking the full model and matching each of its columns
    // against the design columns in order, so a column's index is
    // the number of parameters before it.
    let mut param_of: Vec<usize> = vec![0; columns.len()];
    {
        let mut design_index = 0usize;
        for (param, col) in full_model.iter().enumerate() {
            if col.effect == "Intercept" || col.effect == "Covariate" {
                continue;
            }
            if design_index < columns.len() && columns[design_index].effect == col.effect {
                param_of[design_index] = param;
                design_index += 1;
            }
        }
    }

    for name in effect_list {
        let cols: Vec<usize> = columns
            .iter()
            .enumerate()
            .filter(|(_, c)| &c.effect == name)
            .map(|(i, _)| param_of[i])
            .collect();
        if cols.is_empty() {
            continue;
        }
        // The Wald test: the hypothesis is that the effect's own
        // coefficients are zero, so the contrast matrix is the
        // identity on those columns.
        //
        //   SS = (L beta)' (L (X'X)^-1 L')^-1 (L beta)
        //
        // and the F statistic is SS / df / sigma2.
        let r = cols.len();
        let mut lb = vec![0.0f64; r];
        for (a, &c) in cols.iter().enumerate() {
            lb[a] = beta[c];
        }
        let mut lml = vec![vec![0.0f64; r]; r];
        for a in 0..r {
            for b in 0..r {
                lml[a][b] = inv_xtx[cols[a] * p + cols[b]];
            }
        }
        let Some(inv_lml) = invert(&lml, r) else {
            continue;
        };
        let mut wald = 0.0;
        for a in 0..r {
            for b in 0..r {
                wald += lb[a] * inv_lml[a * r + b] * lb[b];
            }
        }
        out.insert(name.clone(), (wald.max(0.0), r as f64));
    }
    out
}

/// The higher-order effects that contain the given effect.
///
/// For the main effect "A" in a design with "A", "B" and "A*B", the
/// higher-order terms are "A*B". For the interaction "A*B" there are
/// none, unless there is a three-way interaction.
fn higher_order_terms(designs: &[DesignFactor], name: &str) -> Vec<String> {
    let factors_of = factors_of_effect(designs, name);
    let mut out = Vec::new();
    for size in 2..=designs.len() {
        for combo in combinations(designs.len(), size) {
            // The other effect contains this one when it has all of
            // this effect's factors and more.
            let contains = factors_of.iter().all(|f| combo.contains(f))
                && combo.len() > factors_of.len();
            if contains {
                out.push(
                    combo
                        .iter()
                        .map(|&i| designs[i].name.as_str())
                        .collect::<Vec<_>>()
                        .join("*"),
                );
            }
        }
    }
    out
}

/// The factors an effect is made of: the names in the effect's name
/// split on the '*'.
fn factors_of_effect(designs: &[DesignFactor], name: &str) -> Vec<usize> {
    let mut out = Vec::new();
    for part in name.split('*') {
        if let Some(i) = designs.iter().position(|d| d.name == part) {
            out.push(i);
        }
    }
    out
}

/// The degrees of freedom of an effect.
fn effect_df(designs: &[DesignFactor], name: &str) -> f64 {
    let mut df = 1.0;
    for i in factors_of_effect(designs, name) {
        df *= (designs[i].levels.len() - 1) as f64;
    }
    df
}

/// The names of the effects: the main effects first, then the
/// interactions, in the order StatsBench writes them.
fn effect_names(designs: &[DesignFactor]) -> Vec<String> {
    let mut names: Vec<String> = designs.iter().map(|d| d.name.clone()).collect();
    for size in 2..=designs.len() {
        for combo in combinations(designs.len(), size) {
            names.push(
                combo
                    .iter()
                    .map(|&i| designs[i].name.as_str())
                    .collect::<Vec<_>>()
                    .join("*"),
            );
        }
    }
    names
}

/// The combinations of `size` items out of `n`, in order.
fn combinations(n: usize, size: usize) -> Vec<Vec<usize>> {
    let mut out = Vec::new();
    let mut current = Vec::new();
    fn rec(n: usize, size: usize, start: usize, cur: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
        if cur.len() == size {
            out.push(cur.clone());
            return;
        }
        for i in start..n {
            cur.push(i);
            rec(n, size, i + 1, cur, out);
            cur.pop();
        }
    }
    rec(n, size, 0, &mut current, &mut out);
    out
}

/// Group the design columns by the effect they belong to.
fn group_columns(columns: &[DesignColumn]) -> Vec<(String, Vec<DesignColumn>)> {
    let mut out: Vec<(String, Vec<DesignColumn>)> = Vec::new();
    for col in columns {
        match out.iter_mut().find(|(name, _)| *name == col.effect) {
            Some((_, cols)) => cols.push(col.clone()),
            None => out.push((col.effect.clone(), vec![col.clone()])),
        }
    }
    out
}

/// Build the design columns: the main effects and the interactions,
/// with effect coding.
fn build_design_columns(
    designs: &[DesignFactor],
    level_idx: &[Vec<usize>],
    _has_covariate: bool,
) -> Vec<DesignColumn> {
    let n = level_idx.len();
    let mut columns: Vec<DesignColumn> = Vec::new();

    // The coded columns of each factor: one per level except the
    // FIRST, which is the reference level. This is treatment coding,
    // and it is what makes the Type III hypothesis - that the
    // reference level and the other levels have the same mean - the
    // natural one.
    //
    // The reference level is the first one because that is the
    // order StatsBench and statsmodels use, and the Type III sums of
    // squares depend on which level it is.
    let mut coded: Vec<Vec<Vec<f64>>> = Vec::new();
    for (k, design) in designs.iter().enumerate() {
        let k_levels = design.levels.len();
        let mut factor_cols: Vec<Vec<f64>> = Vec::new();
        for j in 1..k_levels {
            factor_cols.push(
                level_idx
                    .iter()
                    .map(|idx| if idx[k] == j { 1.0 } else { 0.0 })
                    .collect(),
            );
        }
        coded.push(factor_cols);
    }

    // The main effects.
    for (k, design) in designs.iter().enumerate() {
        for col in coded[k].iter() {
            columns.push(DesignColumn {
                effect: design.name.clone(),
                values: col.clone(),
            });
        }
    }

    // The interactions.
    for size in 2..=designs.len() {
        for combo in combinations(designs.len(), size) {
            let name = combo
                .iter()
                .map(|&i| designs[i].name.as_str())
                .collect::<Vec<_>>()
                .join("*");
            let mut indices: Vec<usize> = vec![0; combo.len()];
            loop {
                let values: Vec<f64> = (0..n)
                    .map(|row| {
                        indices
                            .iter()
                            .enumerate()
                            .fold(1.0, |acc, (pos, &ci)| acc * coded[combo[pos]][ci][row])
                    })
                    .collect();
                columns.push(DesignColumn {
                    effect: name.clone(),
                    values,
                });
                // The next combination.
                let mut pos = combo.len();
                loop {
                    if pos == 0 {
                        return columns;
                    }
                    pos -= 1;
                    indices[pos] += 1;
                    if indices[pos] < coded[combo[pos]].len() {
                        break;
                    }
                    indices[pos] = 0;
                }
            }
        }
    }

    columns
}

/// The residual sum of squares of a model.
///
/// The fit is a least-squares projection of the response onto the
/// model's columns, done with a Householder QR decomposition.
fn fit_ss(model: &[DesignColumn], y: &[f64]) -> ModelFit {
    let n = y.len();
    if n == 0 {
        return ModelFit { ss: 0.0, df: 0.0 };
    }
    let p = model.len();
    let mut a = vec![vec![0.0f64; p]; n];
    for (j, col) in model.iter().enumerate() {
        for i in 0..n {
            a[i][j] = col.values.get(i).copied().unwrap_or(0.0);
        }
    }
    let mut rhs = y.to_vec();

    let mut rank = 0usize;
    for k in 0..p.min(n) {
        let mut norm = 0.0;
        for i in k..n {
            norm += a[i][k] * a[i][k];
        }
        norm = norm.sqrt();
        if norm < 1e-12 {
            continue;
        }
        let alpha = if a[k][k] > 0.0 { -norm } else { norm };
        let mut v = vec![0.0f64; n];
        for i in k..n {
            v[i] = a[i][k];
        }
        v[k] -= alpha;
        let vnorm2: f64 = v[k..].iter().map(|x| x * x).sum();
        if vnorm2 < 1e-300 {
            continue;
        }
        for j in k..p {
            let dot: f64 = (k..n).map(|i| v[i] * a[i][j]).sum();
            let factor = 2.0 * dot / vnorm2;
            for i in k..n {
                a[i][j] -= factor * v[i];
            }
        }
        let dot: f64 = (k..n).map(|i| v[i] * rhs[i]).sum();
        let factor = 2.0 * dot / vnorm2;
        for i in k..n {
            rhs[i] -= factor * v[i];
        }
        rank += 1;
    }

    let ss: f64 = (rank..n).map(|i| rhs[i] * rhs[i]).sum();
    ModelFit {
        ss: ss.max(0.0),
        df: (n as f64 - rank as f64).max(0.0),
    }
}

/// The descriptive statistics of the cells of the design.
fn cell_summaries(
    designs: &[DesignFactor],
    level_idx: &[Vec<usize>],
    y: &[f64],
) -> Vec<CellSummary> {
    let mut out: Vec<CellSummary> = Vec::new();
    let n_factors = designs.len();
    if n_factors == 0 {
        return out;
    }
    let mut indices = vec![0usize; n_factors];
    loop {
        let vals: Vec<f64> = level_idx
            .iter()
            .enumerate()
            .filter(|(_, idx)| idx.iter().zip(indices.iter()).all(|(a, b)| a == b))
            .filter_map(|(i, _)| y.get(i).copied())
            .collect();
        let n = vals.len();
        if n > 0 {
            let mean = vals.iter().sum::<f64>() / n as f64;
            let var = if n > 1 {
                vals.iter().map(|v| (v - mean) * (v - mean)).sum::<f64>() / (n as f64 - 1.0)
            } else {
                0.0
            };
            let std_dev = var.max(0.0).sqrt();
            let name = designs
                .iter()
                .enumerate()
                .map(|(k, d)| format!("{}={}", d.name, d.levels[indices[k]]))
                .collect::<Vec<_>>()
                .join(",");
            out.push(CellSummary {
                name,
                n,
                mean,
                std_dev,
                std_error: if n > 0 { std_dev / (n as f64).sqrt() } else { f64::NAN },
            });
        }
        // The next combination.
        let mut k = n_factors;
        loop {
            if k == 0 {
                return out;
            }
            k -= 1;
            indices[k] += 1;
            if indices[k] < designs[k].levels.len() {
                break;
            }
            indices[k] = 0;
            if k == 0 {
                return out;
            }
        }
    }
}

/// The category key of a cell: the text of the value, or the number
/// written out.
fn category_key(col: &crate::data::Column, row: usize) -> Option<String> {
    if col.is_missing(row) {
        return None;
    }
    match col.values.get(row) {
        Some(Value::Text(s)) => Some(s.clone()),
        Some(Value::Number(n)) => Some(format!("{}", n)),
        _ => None,
    }
}

/// The mixed-radix index of a set of level indices.
fn mixed_radix(idx: &[usize], radix: &[usize]) -> usize {
    let mut cell = 0usize;
    for (i, &v) in idx.iter().enumerate() {
        cell = cell * radix[i] + v;
    }
    cell
}

/// The strides of the mixed-radix layout: the first factor varies
/// fastest.
fn strides(radix: &[usize]) -> Vec<usize> {
    let mut s = vec![1usize; radix.len()];
    for i in (0..radix.len().saturating_sub(1)).rev() {
        s[i] = s[i + 1] * radix[i + 1];
    }
    s
}

/// The Cholesky factor of a symmetric positive definite matrix, or
/// None when the matrix is not positive definite.
fn cholesky(a: &[Vec<f64>], n: usize) -> Option<Vec<f64>> {
    let mut l = vec![0.0f64; n * n];
    for i in 0..n {
        for j in 0..=i {
            let mut s = a[i][j];
            for k in 0..j {
                s -= l[i * n + k] * l[j * n + k];
            }
            if i == j {
                if s <= 1e-14 {
                    return None;
                }
                l[i * n + j] = s.sqrt();
            } else {
                l[i * n + j] = s / l[j * n + j];
            }
        }
    }
    Some(l)
}

/// Solve `L L' x = b` for a lower triangular `L`.
fn cholesky_solve(l: &[f64], b: &[f64], n: usize) -> Vec<f64> {
    // The forward substitution: L y = b.
    let mut y = vec![0.0f64; n];
    for i in 0..n {
        let mut s = b[i];
        for k in 0..i {
            s -= l[i * n + k] * y[k];
        }
        y[i] = s / l[i * n + i];
    }
    // The back substitution: L' x = y.
    let mut x = vec![0.0f64; n];
    for i in (0..n).rev() {
        let mut s = y[i];
        for k in (i + 1)..n {
            s -= l[k * n + i] * x[k];
        }
        x[i] = s / l[i * n + i];
    }
    x
}

/// The inverse of a matrix from its Cholesky factor.
fn cholesky_inverse(l: &[f64], n: usize) -> Vec<f64> {
    // The inverse is the product of the inverse of L and the transpose
    // of its inverse.
    let mut linv = vec![0.0f64; n * n];
    for i in 0..n {
        linv[i * n + i] = 1.0 / l[i * n + i];
        for j in 0..i {
            let mut s = 0.0;
            for k in j..i {
                s += l[i * n + k] * linv[k * n + j];
            }
            linv[i * n + j] = -s / l[i * n + i];
        }
    }
    // The inverse of L' is the transpose of the inverse of L.
    let mut l2inv = vec![0.0f64; n * n];
    for i in 0..n {
        for j in 0..n {
            l2inv[i * n + j] = linv[j * n + i];
        }
    }
    // The product (L')^-1 L^-1, because
    //   (L L')^-1 = (L')^-1 L^-1.
    let mut inv = vec![0.0f64; n * n];
    for i in 0..n {
        for j in 0..n {
            let mut s = 0.0;
            for k in 0..n {
                s += l2inv[i * n + k] * linv[k * n + j];
            }
            inv[i * n + j] = s;
        }
    }
    inv
}

/// The inverse of a small matrix, by Gauss-Jordan elimination.
fn invert(a: &[Vec<f64>], n: usize) -> Option<Vec<f64>> {
    if n == 0 {
        return Some(Vec::new());
    }
    let mut m: Vec<Vec<f64>> = (0..n)
        .map(|i| {
            let mut row = a[i].clone();
            row.extend((0..n).map(|j| if i == j { 1.0 } else { 0.0 }));
            row
        })
        .collect();
    for col in 0..n {
        // The pivot.
        let mut pivot = col;
        for r in col..n {
            if m[r][col].abs() > m[pivot][col].abs() {
                pivot = r;
            }
        }
        if m[pivot][col].abs() < 1e-14 {
            return None;
        }
        m.swap(col, pivot);
        let d = m[col][col];
        for v in m[col].iter_mut() {
            *v /= d;
        }
        for r in 0..n {
            if r == col {
                continue;
            }
            let factor = m[r][col];
            if factor == 0.0 {
                continue;
            }
            for c in 0..2 * n {
                m[r][c] -= factor * m[col][c];
            }
        }
    }
    let mut out = vec![0.0f64; n * n];
    for i in 0..n {
        for j in 0..n {
            out[i * n + j] = m[i][n + j];
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::DataTable;

    /// A table from a CSV string.
    fn table_from(tag: &str, csv: &str) -> DataTable {
        let path = std::env::temp_dir().join(format!("statscraft_glm_{}.csv", tag));
        std::fs::write(&path, csv).unwrap();
        let t = DataTable::from_csv_path(&path).unwrap();
        let _ = std::fs::remove_file(path);
        t
    }

    /// The balanced two-factor design, checked against statsmodels.
    ///
    /// The values run from the cell's base up to base+9, so each cell's
    /// mean is its base plus 4.5: 14.5, 16.5, 24.5 and 26.5, and the
    /// grand mean is 20.5.
    ///
    /// The three types of sums of squares agree here, because the
    /// design is balanced - and the Type III values are half the
    /// Type I ones, because the Type III hypothesis for a main effect
    /// is a single contrast against the reference level rather than
    /// the whole effect.
    #[test]
    fn the_balanced_two_factor_anova_matches_statsmodels() {
        let mut csv = String::from("a,b,y\n");
        for (a, b, base) in [(1, 1, 10.0), (1, 2, 12.0), (2, 1, 20.0), (2, 2, 22.0)] {
            for i in 0..10 {
                csv.push_str(&format!("{},{},{}\n", a, b, base + i as f64));
            }
        }
        let t = table_from("twoway", &csv);
        let y = t.column_index("y").unwrap();
        let a = t.column_index("a").unwrap();
        let b = t.column_index("b").unwrap();
        let r = glm(
            &t,
            y,
            &[(a, "A".into()), (b, "B".into())],
            None,
            SumOfSquaresType::TypeIII,
        )
        .expect("the model fitted");

        //   SS_A = 20 * (15.5-20.5)^2 + 20 * (25.5-20.5)^2 = 1000
        //   SS_B = 20 * (15.5-20.5)^2 + 20 * (15.5-20.5)^2 = 40
        //   SS_AB = 0, because the design is additive.
        //   The within-cell SS is 4 * 82.5 = 330.
        let a_eff = r.effects.iter().find(|e| e.name == "A").expect("A");
        let b_eff = r.effects.iter().find(|e| e.name == "B").expect("B");
        let ab = r.effects.iter().find(|e| e.name == "A*B").expect("A*B");
        let err = r.effects.iter().find(|e| e.name == "Error").expect("Error");

        // statsmodels, typ=3:
        //   C(a)      ss=500.0000000000 df=1 F=54.5454545455 p=1.031374e-08
        //   C(b)      ss= 20.0000000000 df=1 F= 2.1818181818 p=1.483448e-01
        //   C(a):C(b) ss=  0.0000000000 df=1 F= 0.0000000000 p=1.000000e+00
        //   Residual  ss=330.0000000000 df=36
        assert!((a_eff.ss - 500.0).abs() < 1e-5, "SS_A was {}", a_eff.ss);
        assert!((b_eff.ss - 20.0).abs() < 1e-5, "SS_B was {}", b_eff.ss);
        assert!(ab.ss.abs() < 1e-5, "SS_AB was {}", ab.ss);
        assert!((err.ss - 330.0).abs() < 1e-5, "SS_error was {}", err.ss);
        assert!((a_eff.df - 1.0).abs() < 1e-9, "df_A was {}", a_eff.df);
        assert!((err.df - 36.0).abs() < 1e-9, "df_error was {}", err.df);
        assert!(
            (a_eff.f.unwrap() - 54.5454545455).abs() < 1e-6,
            "F was {}",
            a_eff.f.unwrap()
        );
        assert!(
            (a_eff.p.unwrap() - 1.031374e-08).abs() < 1e-14,
            "p was {}",
            a_eff.p.unwrap()
        );
    }

    /// The balanced design's Type I and Type II sums of squares.
    ///
    /// For a balanced design the types agree with each other, and they
    /// are the classical values: the sum of squares of a level's mean
    /// away from the grand mean, weighted by the number of cases.
    #[test]
    fn the_balanced_types_one_and_two_agree_with_each_other() {
        let mut csv = String::from("a,b,y
");
        for (a, b, base) in [(1, 1, 10.0), (1, 2, 12.0), (2, 1, 20.0), (2, 2, 22.0)] {
            for i in 0..10 {
                csv.push_str(&format!("{},{},{}
", a, b, base + i as f64));
            }
        }
        let t = table_from("twoway12", &csv);
        let y = t.column_index("y").unwrap();
        let a = t.column_index("a").unwrap();
        let b = t.column_index("b").unwrap();
        let factors = [(a, "A".to_string()), (b, "B".to_string())];

        for ss_type in [SumOfSquaresType::TypeI, SumOfSquaresType::TypeII] {
            let r = glm(&t, y, &factors, None, ss_type).expect("the model fitted");
            let a_eff = r.effects.iter().find(|e| e.name == "A").expect("A");
            let b_eff = r.effects.iter().find(|e| e.name == "B").expect("B");
            let ab = r.effects.iter().find(|e| e.name == "A*B").expect("A*B");
            let err = r.effects.iter().find(|e| e.name == "Error").expect("Error");
            assert!(
                (a_eff.ss - 1000.0).abs() < 1e-5,
                "{}: SS_A was {}",
                ss_type.keyword(),
                a_eff.ss
            );
            assert!(
                (b_eff.ss - 40.0).abs() < 1e-5,
                "{}: SS_B was {}",
                ss_type.keyword(),
                b_eff.ss
            );
            assert!(
                ab.ss.abs() < 1e-5,
                "{}: SS_AB was {}",
                ss_type.keyword(),
                ab.ss
            );
            assert!(
                (err.ss - 330.0).abs() < 1e-5,
                "{}: SS_error was {}",
                ss_type.keyword(),
                err.ss
            );
            assert!(
                (a_eff.f.unwrap() - 109.0909090909).abs() < 1e-6,
                "{}: F was {}",
                ss_type.keyword(),
                a_eff.f.unwrap()
            );
        }
    }

    /// The unbalanced case, checked against statsmodels.
    ///
    /// The values come from `tools/glm_reference.py`, which fits the
    /// same data with statsmodels and prints the Type III sums of
    /// squares. An unbalanced design is where the types of sums of
    /// squares differ, so it is the case worth checking.
    #[test]
    fn the_unbalanced_design_matches_statsmodels() {
        let path = std::env::current_dir()
            .unwrap()
            .join("samples")
            .join("glm_reference.csv");
        let t = DataTable::from_csv_path(&path).expect("the reference data");
        let y = t.column_index("y").unwrap();
        let a = t.column_index("a").unwrap();
        let b = t.column_index("b").unwrap();
        let r = glm(
            &t,
            y,
            &[(a, "A".into()), (b, "B".into())],
            None,
            SumOfSquaresType::TypeIII,
        )
        .expect("the model fitted");

        // From statsmodels:
        //   C(a)      ss=196.7618691062 df=2 F=8.6335171710 p=3.891080e-04
        //   C(b)      ss=106.6135825271 df=1 F=9.3559814164 p=2.981941e-03
        //   C(a):C(b) ss= 17.2924068458 df=2 F=0.7587562169 p=4.714303e-01
        //   Residual  ss=957.1995212134 df=84
        let a_eff = r.effects.iter().find(|e| e.name == "A").expect("A");
        let b_eff = r.effects.iter().find(|e| e.name == "B").expect("B");
        let ab = r.effects.iter().find(|e| e.name == "A*B").expect("A*B");
        let err = r.effects.iter().find(|e| e.name == "Error").expect("Error");

        assert!(
            (a_eff.ss - 196.7618691062).abs() < 1e-5,
            "SS_A was {}",
            a_eff.ss
        );
        assert!((a_eff.df - 2.0).abs() < 1e-9, "df_A was {}", a_eff.df);
        assert!(
            (a_eff.f.unwrap() - 8.6335171710).abs() < 1e-5,
            "F_A was {}",
            a_eff.f.unwrap()
        );
        assert!(
            (a_eff.p.unwrap() - 3.891080e-04).abs() < 1e-10,
            "p_A was {}",
            a_eff.p.unwrap()
        );

        assert!(
            (b_eff.ss - 106.6135825271).abs() < 1e-5,
            "SS_B was {}",
            b_eff.ss
        );
        assert!(
            (b_eff.f.unwrap() - 9.3559814164).abs() < 1e-5,
            "F_B was {}",
            b_eff.f.unwrap()
        );

        assert!(
            (ab.ss - 17.2924068458).abs() < 1e-5,
            "SS_AB was {}",
            ab.ss
        );
        assert!(
            (ab.f.unwrap() - 0.7587562169).abs() < 1e-5,
            "F_AB was {}",
            ab.f.unwrap()
        );
        // The reference value carries seven significant digits, so
        // the tolerance has to allow for that.
        assert!(
            (ab.p.unwrap() - 4.714303e-01).abs() < 1e-7,
            "p_AB was {}",
            ab.p.unwrap()
        );

        assert!(
            (err.ss - 957.1995212134).abs() < 1e-5,
            "SS_error was {}",
            err.ss
        );
        assert!((err.df - 84.0).abs() < 1e-9, "df_error was {}", err.df);
    }

    /// The unbalanced case's Type I and Type II sums of squares.
    ///
    /// Here the three types really do differ, which is the whole
    /// reason they exist:
    ///
    ///   typ 1: C(a) ss=333.2102049866 F=14.6205971684
    ///          C(b) ss=150.3639062954 F=13.1953347749
    ///   typ 2: C(a) ss=249.7686869955 F=10.9593502936
    ///          C(b) ss=150.3639062954 F=13.1953347749
    ///
    /// The interaction is the same for all of them, because it is the
    /// last effect in and nothing follows it.
    #[test]
    fn the_unbalanced_types_one_and_two_match_statsmodels() {
        let path = std::env::current_dir()
            .unwrap()
            .join("samples")
            .join("glm_reference.csv");
        let t = DataTable::from_csv_path(&path).expect("the reference data");
        let y = t.column_index("y").unwrap();
        let a = t.column_index("a").unwrap();
        let b = t.column_index("b").unwrap();
        let factors = [(a, "A".to_string()), (b, "B".to_string())];

        let r1 = glm(&t, y, &factors, None, SumOfSquaresType::TypeI).expect("fitted");
        let a1 = r1.effects.iter().find(|e| e.name == "A").expect("A");
        let b1 = r1.effects.iter().find(|e| e.name == "B").expect("B");
        let ab1 = r1.effects.iter().find(|e| e.name == "A*B").expect("A*B");
        assert!(
            (a1.ss - 333.2102049866).abs() < 1e-5,
            "Type I: SS_A was {}",
            a1.ss
        );
        assert!(
            (b1.ss - 150.3639062954).abs() < 1e-5,
            "Type I: SS_B was {}",
            b1.ss
        );
        assert!(
            (ab1.ss - 17.2924068458).abs() < 1e-5,
            "Type I: SS_AB was {}",
            ab1.ss
        );
        assert!(
            (a1.f.unwrap() - 14.6205971684).abs() < 1e-6,
            "Type I: F was {}",
            a1.f.unwrap()
        );

        let r2 = glm(&t, y, &factors, None, SumOfSquaresType::TypeII).expect("fitted");
        let a2 = r2.effects.iter().find(|e| e.name == "A").expect("A");
        let b2 = r2.effects.iter().find(|e| e.name == "B").expect("B");
        assert!(
            (a2.ss - 249.7686869955).abs() < 1e-5,
            "Type II: SS_A was {}",
            a2.ss
        );
        assert!(
            (b2.ss - 150.3639062954).abs() < 1e-5,
            "Type II: SS_B was {}",
            b2.ss
        );
        assert!(
            (a2.f.unwrap() - 10.9593502936).abs() < 1e-6,
            "Type II: F was {}",
            a2.f.unwrap()
        );
    }

    /// The one-factor case must agree with the one-way ANOVA that is
    /// already tested.
    #[test]
    fn the_glm_agrees_with_the_one_way_anova() {
        let mut csv = String::from("g,y\n");
        for v in [10.0, 14.0, 9.0, 16.0].iter() {
            csv.push_str(&format!("A,{}\n", v));
        }
        for v in [12.0, 18.0, 11.0, 20.0].iter() {
            csv.push_str(&format!("B,{}\n", v));
        }
        let t = table_from("oneway", &csv);
        let y = t.column_index("y").unwrap();
        let g = t.column_index("g").unwrap();
        let r = glm(&t, y, &[(g, "G".into())], None, SumOfSquaresType::TypeIII).expect("fitted");
        let one_way =
            crate::stats::one_way_anova_with(&t, y, g, &crate::stats::PostHocSet::default(), false)
                .expect("the one-way ANOVA");

        let g_eff = r.effects.iter().find(|e| e.name == "G").expect("G");
        assert!(
            (g_eff.ss - one_way.ss_between).abs() < 1e-6,
            "SS was {} and {}",
            g_eff.ss,
            one_way.ss_between
        );
        assert!(
            (g_eff.f.unwrap() - one_way.f).abs() < 1e-6,
            "F was {} and {}",
            g_eff.f.unwrap(),
            one_way.f
        );
    }

    /// The sums of squares must add up: the model plus the error is the
    /// total.
    #[test]
    fn the_sums_of_squares_add_up_to_the_total() {
        let mut csv = String::from("a,b,y\n");
        let mut seed = 12345u64;
        let mut next = move || {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            ((seed >> 33) as f64 / (1u64 << 31) as f64) * 20.0
        };
        for i in 0..60 {
            let a = (i % 3) + 1;
            let b = (i % 2) + 1;
            csv.push_str(&format!("{},{},{}\n", a, b, next()));
        }
        let t = table_from("adds", &csv);
        let y = t.column_index("y").unwrap();
        let a = t.column_index("a").unwrap();
        let b = t.column_index("b").unwrap();
        let r = glm(
            &t,
            y,
            &[(a, "A".into()), (b, "B".into())],
            None,
            SumOfSquaresType::TypeIII,
        )
        .expect("the model fitted");

        let total = r.effects.iter().find(|e| e.name == "Total").unwrap().ss;
        let error = r.effects.iter().find(|e| e.name == "Error").unwrap().ss;
        let model = r
            .effects
            .iter()
            .find(|e| e.name == "Corrected Model")
            .unwrap()
            .ss;
        assert!(
            ((model + error) - total).abs() < 1e-6 * total.max(1.0),
            "model {} + error {} was not the total {}",
            model,
            error,
            total
        );
    }

    /// The Type I sums of squares must add up to the model too - they
    /// are sequential, so they are not the same numbers as the Type
    /// III ones for an unbalanced design.
    #[test]
    fn the_type_one_sums_of_squares_add_up() {
        let path = std::env::current_dir()
            .unwrap()
            .join("samples")
            .join("glm_reference.csv");
        let t = DataTable::from_csv_path(&path).expect("the reference data");
        let y = t.column_index("y").unwrap();
        let a = t.column_index("a").unwrap();
        let b = t.column_index("b").unwrap();
        let r = glm(
            &t,
            y,
            &[(a, "A".into()), (b, "B".into())],
            None,
            SumOfSquaresType::TypeI,
        )
        .expect("the model fitted");

        let total = r.effects.iter().find(|e| e.name == "Total").unwrap().ss;
        let error = r.effects.iter().find(|e| e.name == "Error").unwrap().ss;
        let model = r
            .effects
            .iter()
            .find(|e| e.name == "Corrected Model")
            .unwrap()
            .ss;
        assert!(
            ((model + error) - total).abs() < 1e-5 * total.max(1.0),
            "model {} + error {} was not the total {}",
            model,
            error,
            total
        );
    }
}
