use crate::app::StatsCraftApp;
use crate::output::OutputTable;
use crate::stats;

/// The single execution point for StatsCraft: every procedure (menus, dialogs,
/// syntax window) produces SPSS syntax text that is executed here.
/// Commands mirror IBM SPSS keywords: LOAD, DESCRIPTIVES, FREQUENCIES,
/// CORRELATIONS, REGRESSION, T-TEST, SORT CASES, CLEAR.
pub fn run(app: &mut StatsCraftApp, script: &str) {
    for raw in script.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('*') {
            continue; // blank lines and SPSS comment commands
        }
        execute_line(app, line);
    }
}

fn execute_line(app: &mut StatsCraftApp, line: &str) {
    let tokens: Vec<String> = line.split_whitespace().map(|s| s.to_string()).collect();
    if tokens.is_empty() {
        return;
    }
    let cmd = tokens[0].to_uppercase();

    match cmd.as_str() {
        "LOAD" | "GET" => {
            let path = extract_path(line);
            if path.is_empty() {
                app.status = "LOAD: missing file path.".to_string();
                return;
            }
            app.path_input = path;
            app.load();
        }
        "CLEAR" => {
            app.table = None;
            app.sel_cell = None;
            app.editing_cell = None;
            app.status = "Dataset cleared.".to_string();
        }
        "DESCRIPTIVES" | "DESCRIPTIVE" => {
            let vars = resolve_vars(app, &tokens[1..], true);
            if let Some(res) = vars {
                if res.is_empty() {
                    app.status = "DESCRIPTIVES: no numeric variables resolved.".to_string();
                    return;
                }
                if let Some(t) = &app.table {
                    let d = stats::descriptives_in(t, &res);
                    app.outputs.push(OutputTable::from_descriptives(&d));
                    app.status = format!("DESCRIPTIVES executed for {} variables.", d.len());
                    if !app.outputs.is_empty() {
                        app.show_output_window = true;
                    }
                }
            }
        }
        "FREQUENCIES" | "FREQ" => {
            let vars = resolve_vars(app, &tokens[1..], false);
            if let Some(res) = vars {
                if res.is_empty() {
                    app.status = "FREQUENCIES: no variables resolved.".to_string();
                    return;
                }
                if let Some(t) = &app.table {
                    let mut n = 0usize;
                    for &ci in &res {
                        let name = t.columns[ci].name.clone();
                        let rows = stats::frequencies(t, ci);
                        app.outputs.push(OutputTable::from_frequencies(&name, &rows));
                        n += 1;
                    }
                    app.status = format!("FREQUENCIES executed for {} variables.", n);
                    if !app.outputs.is_empty() {
                        app.show_output_window = true;
                    }
                }
            }
        }
        "CORRELATIONS" | "CORR" => {
            let vars = resolve_vars(app, &tokens[1..], true);
            if let Some(res) = vars {
                if res.len() < 2 {
                    app.status = "CORRELATIONS: need at least two numeric variables.".to_string();
                    return;
                }
                if let Some(t) = &app.table {
                    let c = stats::correlations_in(t, &res);
                    app.outputs.push(OutputTable::from_correlations(&c));
                    app.status = format!("CORRELATIONS executed: {} pair(s).", c.len());
                    if !app.outputs.is_empty() {
                        app.show_output_window = true;
                    }
                }
            }
        }
        "REGRESSION" => {
            // REGRESSION <y> WITH <x1> [x2 ...]
            let rest: Vec<String> = tokens[1..].to_vec();
            let wi = rest.iter().position(|t| t.eq_ignore_ascii_case("WITH"));
            let (y_args, x_start) = match wi {
                Some(i) => (rest[..i].to_vec(), i + 1),
                None => (rest.clone(), rest.len()),
            };
            if y_args.len() != 1 || x_start >= rest.len() {
                app.status = "REGRESSION: expected 'REGRESSION y WITH x...'.".to_string();
                return;
            }
            let (Some(yi), Some(xs)) = (
                app.table.as_ref().and_then(|t| t.column_index(&y_args[0])),
                app.table.as_ref().map(|t| {
                    rest[x_start..]
                        .iter()
                        .filter_map(|n| t.column_index(n))
                        .collect::<Vec<usize>>()
                }),
            ) else {
                app.status = "REGRESSION: unknown variable name.".to_string();
                return;
            };
            if let Some(t) = &app.table {
                let mut ok = false;
                if let Some(&x) = xs.first() {
                    if let Some(r) = stats::linear_regression_xy(t, yi, x) {
                        app.outputs.push(OutputTable::from_regression(&r));
                        app.status = format!(
                            "REGRESSION executed: {} on {}.",
                            t.columns[yi].name, t.columns[x].name
                        );
                        ok = true;
                    }
                }
                if !ok {
                    app.status = "REGRESSION failed (need >=3 paired numeric cases).".to_string();
                    return;
                }
                if !app.outputs.is_empty() {
                    app.show_output_window = true;
                }
            }
        }
        "CROSSTABS" | "CROSSTAB" => {
            // CROSSTABS <var1> BY <var2>
            let rest: Vec<String> = tokens[1..].to_vec();
            let Some(bi) = rest.iter().position(|t| t.eq_ignore_ascii_case("BY")) else {
                app.status = "CROSSTABS: expected 'CROSSTABS var1 BY var2'.".to_string();
                return;
            };
            let Some(v2) = rest.get(bi + 1).cloned() else {
                app.status = "CROSSTABS: expected 'CROSSTABS var1 BY var2'.".to_string();
                return;
            };
            let v1 = rest[0].clone();
            if let Some(t) = &app.table {
                let (Some(i), Some(j)) = (t.column_index(&v1), t.column_index(&v2)) else {
                    app.status = "CROSSTABS: unknown variable name.".to_string();
                    return;
                };
                match stats::crosstabs_chi2(t, i, j) {
                    Some(ct) => {
                        app.outputs.push(OutputTable::from_crosstabs(&ct));
                        app.status = format!(
                            "CROSSTABS executed: chi2={:.3} df={} p={:.4}",
                            ct.chi2, ct.df, ct.p
                        );
                        app.show_output_window = true;
                    }
                    None => {
                        app.status =
                            "CROSSTABS failed (need 2+ categories per variable).".to_string();
                    }
                }
            }
        }
        "ONEWAY" | "ANOVA" => {
            // ONEWAY <y> BY <factor>
            let rest: Vec<String> = tokens[1..].to_vec();
            let Some(bi) = rest.iter().position(|t| t.eq_ignore_ascii_case("BY")) else {
                app.status = "ONEWAY: expected 'ONEWAY y BY factor'.".to_string();
                return;
            };
            let Some(f) = rest.get(bi + 1).cloned() else {
                app.status = "ONEWAY: expected 'ONEWAY y BY factor'.".to_string();
                return;
            };
            let y = rest[0].clone();
            if let Some(t) = &app.table {
                let (Some(yi), Some(fi)) = (t.column_index(&y), t.column_index(&f)) else {
                    app.status = "ONEWAY: unknown variable name.".to_string();
                    return;
                };
                match stats::one_way_anova_in(t, yi, fi) {
                    Some(a) => {
                        app.outputs.push(OutputTable::from_anova(&a));
                        app.status = format!("ONEWAY executed: F={:.3} p={:.4}", a.f, a.p);
                        app.show_output_window = true;
                    }
                    None => {
                        app.status = "ONEWAY failed (need 2+ groups with 2+ cases each)."
                            .to_string();
                    }
                }
            }
        }
        "T-TEST" | "TTEST" | "T" => {
            // T-TEST <y> BY <group>            (independent samples)
            // T-TEST PAIRED <v1> <v2>          (paired samples)
            let rest: Vec<String> = tokens[1..].to_vec();
            if rest.first().map(|t| t.eq_ignore_ascii_case("PAIRED")).unwrap_or(false) {
                let vars = &rest[1..];
                if vars.len() != 2 {
                    app.status = "T-TEST PAIRED: expected 'T-TEST PAIRED var1 var2'.".to_string();
                    return;
                }
                if let Some(t) = &app.table {
                    let (Some(ia), Some(ib)) = (t.column_index(&vars[0]), t.column_index(&vars[1]))
                    else {
                        app.status = "T-TEST PAIRED: unknown variable name.".to_string();
                        return;
                    };
                    match stats::paired_ttest_in(t, ia, ib) {
                        Some(r) => {
                            app.outputs.push(OutputTable::from_paired_t(&r));
                            app.status =
                                format!("Paired t-test executed: t={:.3} p={:.4}", r.t, r.p);
                            app.show_output_window = true;
                        }
                        None => {
                            app.status = "Paired t-test failed (need >=2 paired cases)."
                                .to_string();
                        }
                    }
                }
                return;
            }
            let bi = rest.iter().position(|t| t.eq_ignore_ascii_case("BY"));
            let (y_args, g_start) = match bi {
                Some(i) => (rest[..i].to_vec(), i + 1),
                None => (rest.clone(), rest.len()),
            };
            if y_args.len() != 1 || g_start >= rest.len() {
                app.status = "T-TEST: expected 'T-TEST y BY group'.".to_string();
                return;
            }
            if let Some(t) = &app.table {
                let (Some(yi), Some(gi)) = (
                    t.column_index(&y_args[0]),
                    t.column_index(&rest[g_start]),
                ) else {
                    app.status = "T-TEST: unknown variable name.".to_string();
                    return;
                };
                match stats::independent_ttest_vars(t, yi, gi) {
                    Some(r) => {
                        app.outputs.push(OutputTable::from_ttest(&r));
                        app.status = format!("T-TEST executed: {} BY {}.", t.columns[yi].name, t.columns[gi].name);
                        if !app.outputs.is_empty() {
                            app.show_output_window = true;
                        }
                    }
                    None => {
                        app.status = "T-TEST failed (group must have exactly 2 levels).".to_string();
                    }
                }
            }
        }
        "SORT" => {
            // SORT CASES BY <var> (A|D)   [direction optional, default ascending]
            let rest: Vec<String> = tokens[1..].to_vec();
            let joined: Vec<String> = rest
                .iter()
                .filter(|t| !t.eq_ignore_ascii_case("CASES"))
                .cloned()
                .collect();
            let Some(bi) = joined.iter().position(|t| t.eq_ignore_ascii_case("BY")) else {
                app.status = "SORT: expected 'SORT CASES BY var (A)'.".to_string();
                return;
            };
            let Some(name) = joined.get(bi + 1).cloned() else {
                app.status = "SORT: expected 'SORT CASES BY var (A)'.".to_string();
                return;
            };
            let asc = !joined
                .iter()
                .skip(bi + 2)
                .any(|t| t.eq_ignore_ascii_case("(D)") || t.eq_ignore_ascii_case("D"));
            if let Some(t) = &mut app.table {
                match t.column_index(&name) {
                    Some(ci) => {
                        t.sort_rows(ci, asc);
                        app.status = format!(
                            "SORT CASES BY {} ({}) executed.",
                            name,
                            if asc { "A" } else { "D" }
                        );
                    }
                    None => {
                        app.status = format!("SORT: unknown variable '{}'.", name);
                    }
                }
            } else {
                app.status = "SORT: no data loaded.".to_string();
            }
        }
        "DATASET" | "DATASETNAME" => {
            // DATASET NAME <name>  — acknowledged; datasets are single in this build
            app.status = "DATASET NAME acknowledged.".to_string();
        }
        _ => {
            app.status = format!("Unknown command: {}", cmd);
        }
    }
}

/// Resolve variable names to column indices.
/// `numeric_only` filters to numeric columns.
/// Empty args default to all numeric (or all) columns — like menu defaults.
/// On failure sets app.status and returns None.
fn resolve_vars(app: &mut StatsCraftApp, args: &[String], numeric_only: bool) -> Option<Vec<usize>> {
    let t = app.table.as_ref()?;
    if args.is_empty() {
        return Some(
            t.columns
                .iter()
                .enumerate()
                .filter(|(_, c)| !numeric_only || c.kind == crate::data::ColumnKind::Numeric)
                .map(|(i, _)| i)
                .collect(),
        );
    }
    let mut idxs = Vec::new();
    for name in args {
        match t.column_index(name) {
            Some(i) => {
                if numeric_only && t.columns[i].kind != crate::data::ColumnKind::Numeric {
                    app.status = format!("Variable '{}' is not numeric.", name);
                    return None;
                }
                idxs.push(i);
            }
            None => {
                app.status = format!("Unknown variable: {}", name);
                return None;
            }
        }
    }
    Some(idxs)
}

fn extract_path(line: &str) -> String {
    // quoted string
    if let Some(start) = line.find('"') {
        if let Some(end) = line[start + 1..].find('"') {
            return line[start + 1..start + 1 + end].to_string();
        }
    }
    if let Some(start) = line.find('\'') {
        if let Some(end) = line[start + 1..].find('\'') {
            return line[start + 1..start + 1 + end].to_string();
        }
    }
    // last token
    line.split_whitespace().last().unwrap_or("").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_demo_csv(path: &std::path::Path) {
        let mut f = std::fs::File::create(path).unwrap();
        use std::io::Write;
        writeln!(f, "id,group,score,age").unwrap();
        writeln!(f, "1,A,10,21").unwrap();
        writeln!(f, "2,B,14,22").unwrap();
        writeln!(f, "3,A,9,23").unwrap();
        writeln!(f, "4,B,16,24").unwrap();
        writeln!(f, "5,A,11,25").unwrap();
        writeln!(f, "6,B,15,26").unwrap();
    }

    #[test]
    fn run_basic_script() {
        let path = std::env::temp_dir().join("sc_syntax_demo.csv");
        write_demo_csv(&path);
        let mut app = StatsCraftApp::default();
        let script = format!(
            "LOAD \"{}\"\nDESCRIPTIVES score\nFREQUENCIES group\nCORRELATIONS score age\nT-TEST score BY group\nSORT CASES BY score (D)\n",
            path.display()
        );
        run(&mut app, &script);
        let table = app.table.expect("table loaded");
        assert_eq!(app.outputs.len(), 4, "four procedures produced output");
        assert!(app.outputs[0].title.contains("Descriptive"));
        assert!(app.outputs[1].title.contains("group"));
        assert!(app.outputs[2].title.contains("Correlations"));
        assert!(app.outputs[3].title.contains("T-Test"));
        // SORT ... (D): first case must be the highest score (16)
        let first_score = &table.columns[2].values[0];
        assert!(matches!(first_score, crate::data::Value::Number(x) if *x == 16.0));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn unknown_variable_reports_error() {
        let path = std::env::temp_dir().join("sc_syntax_err.csv");
        write_demo_csv(&path);
        let mut app = StatsCraftApp::default();
        run(&mut app, &format!("LOAD \"{}\"\nDESCRIPTIVES nonsense\n", path.display()));
        assert!(app.outputs.is_empty());
        assert!(app.status.contains("Unknown variable"));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn regression_with_clause() {
        let path = std::env::temp_dir().join("sc_syntax_reg.csv");
        write_demo_csv(&path);
        let mut app = StatsCraftApp::default();
        run(&mut app, &format!("LOAD \"{}\"\nREGRESSION score WITH age\n", path.display()));
        assert!(app.outputs.last().unwrap().title.contains("Regression"));
        let _ = std::fs::remove_file(path);
    }

    // ---------------- Phase 5+6: new procedures & reference validation ----------

    use crate::data::{Column, ColumnKind, DataTable, VariableMeta, Value};

    fn table_from(v: Vec<(&str, ColumnKind, Vec<Value>)>) -> DataTable {
        let columns: Vec<Column> = v
            .into_iter()
            .map(|(name, kind, values)| Column {
                name: name.to_string(),
                kind,
                values,
                meta: VariableMeta::default_for(kind),
            })
            .collect();
        let n = columns[0].values.len();
        DataTable { columns, n_rows: n }
    }

    #[test]
    fn crosstabs_chi2_textbook_value() {
        // g1: x1=10, x2=5 ; g2: x1=5, x2=20 (n=40) -> chi2 = 8.7111, df = 1, p = 0.00316
        let mut vals_x = Vec::new();
        let mut vals_g = Vec::new();
        for _ in 0..10 { vals_x.push(Value::Number(1.0)); vals_g.push(Value::Number(1.0)); }
        for _ in 0..5  { vals_x.push(Value::Number(2.0)); vals_g.push(Value::Number(1.0)); }
        for _ in 0..5  { vals_x.push(Value::Number(1.0)); vals_g.push(Value::Number(2.0)); }
        for _ in 0..20 { vals_x.push(Value::Number(2.0)); vals_g.push(Value::Number(2.0)); }
        let t = table_from(vec![
            ("x", ColumnKind::Numeric, vals_x),
            ("g", ColumnKind::Numeric, vals_g),
        ]);
        let ct = crate::stats::crosstabs_chi2(&t, 0, 1).unwrap();
        assert_eq!(ct.n, 40);
        assert_eq!(ct.df, 1);
        assert!((ct.chi2 - 8.7111).abs() < 0.001, "chi2={}", ct.chi2);
        assert!(ct.p > 0.0 && ct.p < 0.01, "p={}", ct.p);
    }

    #[test]
    fn anova_textbook_value() {
        // A: 1,2,3 ; B: 6,7,8 -> F = 37.5, df=(1,4), p = 0.00362
        let vals: Vec<Value> = vec![1.0, 2.0, 3.0, 6.0, 7.0, 8.0]
            .into_iter()
            .map(Value::Number)
            .collect();
        let grp: Vec<Value> = ["a", "a", "a", "b", "b", "b"]
            .into_iter()
            .map(|s| Value::Text(s.to_string()))
            .collect();
        let t = table_from(vec![
            ("score", ColumnKind::Numeric, vals),
            ("factor", ColumnKind::String, grp),
        ]);
        let a = crate::stats::one_way_anova_in(&t, 0, 1).unwrap();
        assert!((a.f - 37.5).abs() < 1e-9, "F={}", a.f);
        assert_eq!(a.df_between, 1.0);
        assert_eq!(a.df_within, 4.0);
        assert!(a.p > 0.003 && a.p < 0.004, "p={}", a.p);
    }

    #[test]
    fn paired_t_textbook_value() {
        // pairs x=(1,2,3) y=(2,4,6): diffs (post-pre)=(+1,+2,+3), mean_d=+2, sd=1 -> t=+3.4641, df=2, p=0.0742
        let x: Vec<Value> = vec![1.0, 2.0, 3.0].into_iter().map(Value::Number).collect();
        let y: Vec<Value> = vec![2.0, 4.0, 6.0].into_iter().map(Value::Number).collect();
        let t = table_from(vec![
            ("pre", ColumnKind::Numeric, x),
            ("post", ColumnKind::Numeric, y),
        ]);
        let r = crate::stats::paired_ttest_in(&t, 0, 1).unwrap();
        assert!((r.t - 3.4641).abs() < 1e-3, "t={}", r.t);
        assert_eq!(r.df, 2.0);
        assert!(r.p > 0.07 && r.p < 0.08, "p={}", r.p);
    }

    #[test]
    fn new_procedures_run_through_syntax() {
        let path = std::env::temp_dir().join("sc_syntax5.csv");
        let mut f = std::fs::File::create(&path).unwrap();
        use std::io::Write;
        writeln!(f, "score,factor,prescore").unwrap();
        for (s, g, p) in [(10.0, "A", 9.0), (14.0, "B", 12.0), (9.0, "A", 8.5), (16.0, "B", 15.0)] {
            writeln!(f, "{},{},{}", s, g, p).unwrap();
        }
        let mut app = StatsCraftApp::default();
        run(
            &mut app,
            &format!(
                "LOAD \"{}\"\nCROSSTABS score BY factor\nONEWAY score BY factor\nT-TEST PAIRED score prescore\n",
                path.display()
            ),
        );
        assert_eq!(app.outputs.len(), 3, "three outputs");
        assert!(app.outputs[0].title.contains("Crosstabulation"));
        assert!(app.outputs[1].title.contains("ANOVA"));
        assert!(app.outputs[2].title.contains("Paired"));
        let _ = std::fs::remove_file(path);
    }
}
