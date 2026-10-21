use crate::app::StateCraftApp;
use crate::output::OutputTable;
use crate::stats;

/// The single execution point for StateCraft: every procedure (menus, dialogs,
/// syntax window) produces SPSS syntax text that is executed here.
/// Commands mirror IBM SPSS keywords: LOAD, DESCRIPTIVES, FREQUENCIES,
/// CORRELATIONS, REGRESSION, T-TEST, SORT CASES, CLEAR.
pub fn run(app: &mut StateCraftApp, script: &str) {
    for raw in script.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('*') {
            continue; // blank lines and SPSS comment commands
        }
        execute_line(app, line);
    }
}

fn execute_line(app: &mut StateCraftApp, line: &str) {
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
        "T-TEST" | "TTEST" | "T" => {
            // T-TEST <y> BY <group>
            let rest: Vec<String> = tokens[1..].to_vec();
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
            let bi = joined.iter().position(|t| t.eq_ignore_ascii_case("BY"));
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
            // DATASET NAME <name>  â€” acknowledged; datasets are single in this build
            app.status = "DATASET NAME acknowledged.".to_string();
        }
        _ => {
            app.status = format!("Unknown command: {}", cmd);
        }
    }
}

/// Resolve variable names to column indices.
/// `numeric_only` filters to numeric columns.
/// Empty args default to all numeric (or all) columns â€” like menu defaults.
/// On failure sets app.status and returns None.
fn resolve_vars(app: &mut StateCraftApp, args: &[String], numeric_only: bool) -> Option<Vec<usize>> {
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
        let mut app = StateCraftApp::default();
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
        let mut app = StateCraftApp::default();
        run(&mut app, &format!("LOAD \"{}\"\nDESCRIPTIVES nonsense\n", path.display()));
        assert!(app.outputs.is_empty());
        assert!(app.status.contains("Unknown variable"));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn regression_with_clause() {
        let path = std::env::temp_dir().join("sc_syntax_reg.csv");
        write_demo_csv(&path);
        let mut app = StateCraftApp::default();
        run(&mut app, &format!("LOAD \"{}\"\nREGRESSION score WITH age\n", path.display()));
        assert!(app.outputs.last().unwrap().title.contains("Regression"));
        let _ = std::fs::remove_file(path);
    }
}
