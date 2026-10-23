use crate::app::StatsCraftApp;
use crate::output::OutputTable;
use crate::stats;

/// The single execution point for StatsCraft: every procedure (menus, dialogs,
/// syntax window) produces StatsBench syntax text that is executed here.
/// Commands mirror StatsBench keywords: LOAD, DESCRIPTIVES, FREQUENCIES,
/// CORRELATIONS, REGRESSION, T-TEST, SORT CASES, CLEAR.
///
/// The buffer is split into commands first, so a command written over
/// several lines is run as one command rather than as a row of unknown
/// words. The commands that were run are recorded in the app's Log, so
/// the Output Viewer can show what produced the results, as the
/// reference workbench does.
pub fn run(app: &mut StatsCraftApp, script: &str) {
    let commands: Vec<String> = split_commands(script)
        .into_iter()
        .map(|c| c.text)
        // A comment is a command in the reference workbench, and it is
        // not run.
        .filter(|t| !t.starts_with('*'))
        .collect();
    run_commands(app, &commands);
}

/// Run a list of commands, recording them in the Log.
pub fn run_commands(app: &mut StatsCraftApp, commands: &[String]) {
    let mut executed: Vec<String> = Vec::new();
    for command in commands {
        let line = command.trim();
        if line.is_empty() || line.starts_with('*') {
            continue;
        }
        execute_line(app, line);
        executed.push(line.to_string());
    }
    // The Log of the output document holds the commands of this run.
    app.log = executed;
}

/// The commands held in a selection of a syntax buffer.
///
/// The selection is given as two character positions, the way the
/// editor counts them, and the commands of the selected text are
/// returned. A comment is left out, because it is not run.
pub fn commands_in(text: &str, from: usize, to: usize) -> Vec<String> {
    let slice: String = text
        .chars()
        .skip(from)
        .take(to.saturating_sub(from))
        .collect();
    split_commands(&slice)
        .into_iter()
        .map(|c| c.text)
        .filter(|t| !t.starts_with('*'))
        .collect()
}

/// Run one command of a syntax buffer.
///
/// This is what Run Current Command uses: the command is run on its own,
/// and the Log records it and nothing else.
pub fn run_command(app: &mut StatsCraftApp, command: &str) {
    let line = command.trim();
    if line.is_empty() || line.starts_with('*') {
        app.log.clear();
        return;
    }
    execute_line(app, line);
    app.log = vec![line.to_string()];
}

/// One command of a syntax buffer, with the lines it covers.
///
/// A command is a run of lines that ends at a blank line, a comment, or
/// a line that starts a new command. The reference workbench ends a
/// command with a period; this one ends it when the next line begins
/// with a word, so a command written over several lines stays one
/// command.
#[derive(Debug, Clone, PartialEq)]
pub struct CommandBlock {
    /// The command text, with the lines joined by a space.
    pub text: String,
    /// The first line of the command, counted from zero.
    pub first_line: usize,
    /// The last line of the command, counted from zero.
    pub last_line: usize,
}

/// Split a syntax buffer into its commands.
///
/// The rules follow the reference workbench: a line beginning with an
/// asterisk is a comment, a blank line ends the command before it, and
/// a line that starts with a command word starts a new command. A line
/// that continues the previous one - a subcommand of the command above
/// it, for instance - stays with it.
pub fn split_commands(script: &str) -> Vec<CommandBlock> {
    let mut out: Vec<CommandBlock> = Vec::new();
    let mut current: Vec<String> = Vec::new();
    let mut first_line = 0usize;
    for (i, raw) in script.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() {
            // A blank line closes the command above it.
            if !current.is_empty() {
                out.push(CommandBlock {
                    text: current.join(" "),
                    first_line,
                    last_line: i.saturating_sub(1),
                });
                current.clear();
            }
            continue;
        }
        if line.starts_with('*') {
            // A comment is its own command, and it is not run.
            if !current.is_empty() {
                out.push(CommandBlock {
                    text: current.join(" "),
                    first_line,
                    last_line: i.saturating_sub(1),
                });
                current.clear();
            }
            out.push(CommandBlock {
                text: line.to_string(),
                first_line: i,
                last_line: i,
            });
            continue;
        }
        if starts_new_command(line) && !current.is_empty() {
            out.push(CommandBlock {
                text: current.join(" "),
                first_line,
                last_line: i - 1,
            });
            current.clear();
        }
        if current.is_empty() {
            first_line = i;
        }
        current.push(line.to_string());
    }
    if !current.is_empty() {
        let last = script.lines().count().saturating_sub(1);
        out.push(CommandBlock {
            text: current.join(" "),
            first_line,
            last_line: last,
        });
    }
    out
}

/// The command words the runner knows.
///
/// A line that begins with one of these starts a new command; anything
/// else continues the command above it.
const COMMAND_WORDS: &[&str] = &[
    "LOAD",
    "GET",
    "SAVE",
    "CLEAR",
    "DESCRIPTIVES",
    "DESCRIPTIVE",
    "FREQUENCIES",
    "FREQ",
    "CORRELATIONS",
    "CORR",
    "REGRESSION",
    "CROSSTABS",
    "CROSSTAB",
    "GLM",
    "ONEWAY",
    "ANOVA",
    "T-TEST",
    "TTEST",
    "MEANS",
    "EXPLORE",
    "NONPARAMETRIC",
    "NPAR",
    "MANN-WHITNEY",
    "MW",
    "WILCOXON",
    "KRUSKAL-WALLIS",
    "K-W",
    "FRIEDMAN",
    "RELIABILITY",
    "ALPHA",
    "FACTOR",
    "FACTORS",
    "ROC",
    "ROC-CURVE",
    "COMPUTE",
    "SORT",
    "DATASET",
    "DATASETNAME",
];

/// True when the line begins a new command rather than continuing the
/// one above it.
fn starts_new_command(line: &str) -> bool {
    let word = line.split_whitespace().next().unwrap_or("").to_uppercase();
    // A subcommand belongs to the command above it.
    if word.starts_with('/') {
        return false;
    }
    COMMAND_WORDS.contains(&word.as_str())
}

/// The command that covers the given line, if any.
pub fn command_at_line(script: &str, line: usize) -> Option<CommandBlock> {
    split_commands(script)
        .into_iter()
        .find(|c| line >= c.first_line && line <= c.last_line)
}


/// A parsed StatsBench command line: the positional arguments plus the
/// `/SUBCOMMAND=KEYWORD KEYWORD` subcommands.
///
/// StatsBench writes procedure options as subcommands, e.g.
/// `DESCRIPTIVES score age /STATISTICS=MEAN STDDEV.`
/// The procedure dialogs generate exactly this shape, so the runner and
/// the dialogs share one grammar.
#[derive(Debug, Default, Clone)]
pub struct Command {
    /// The command word, upper-cased.
    pub name: String,
    /// The positional arguments, in order.
    pub args: Vec<String>,
    /// The subcommands, each a (name, keywords) pair. The name is
    /// upper-cased; keywords are upper-cased too.
    pub subcommands: Vec<(String, Vec<String>)>,
}

impl Command {
    /// Parse one line of StatsBench syntax.
    pub fn parse(line: &str) -> Command {
        // Split on whitespace, but keep "/SUB=..." groups together.
        let mut tokens: Vec<String> = Vec::new();
        let mut current = String::new();
        for ch in line.chars() {
            if ch.is_whitespace() {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            } else {
                current.push(ch);
            }
        }
        if !current.is_empty() {
            tokens.push(current);
        }

        let mut cmd = Command::default();
        let mut i = 0;
        while i < tokens.len() {
            let t = &tokens[i];
            if let Some(rest) = t.strip_prefix('/') {
                // "/STATISTICS=MEAN STDDEV" — the keywords after the '='
                // are separate tokens until the next "/".
                // "/STATISTICS=MEAN STDDEV" gives an inline value, and
                // "/POSITIVE(case)" gives a value in parentheses. Both
                // are one keyword with its value.
                // The value of the keyword. A value in parentheses is a
                // piece of data - a group name, a scale name - so its
                // case is kept. A value after an '=' is a list of
                // keywords, so it is upper-cased.
                let (name, inline, is_data) = match rest.split_once('=') {
                    Some((n, v)) => (n.to_string(), Some(v.to_string()), false),
                    None => match rest.split_once('(') {
                        Some((n, v)) => {
                            let v = v.trim_end_matches(')').to_string();
                            (n.to_string(), Some(v), true)
                        }
                        None => (rest.to_string(), None, true),
                    },
                };
                let mut words = Vec::new();
                if let Some(v) = inline {
                    if is_data {
                        words.extend(v.split_whitespace().map(|s| s.to_string()));
                    } else {
                        words.extend(v.split_whitespace().map(|s| s.to_uppercase()));
                    }
                }
                i += 1;
                while i < tokens.len() && !tokens[i].starts_with('/') {
                    words.push(tokens[i].to_uppercase());
                    i += 1;
                }
                cmd.subcommands.push((name.to_uppercase(), words));
            } else {
                if cmd.name.is_empty() {
                    cmd.name = t.to_uppercase();
                } else {
                    cmd.args.push(t.clone());
                }
                i += 1;
            }
        }
        cmd
    }

    /// The keywords of a subcommand, if present.
    pub fn sub(&self, name: &str) -> Option<&[String]> {
        self.subcommands
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, k)| k.as_slice())
    }

    /// The position of a keyword such as "BY" in the positional arguments.
    pub fn pos(&self, keyword: &str) -> Option<usize> {
        self.args
            .iter()
            .position(|a| a.eq_ignore_ascii_case(keyword))
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
            app.dirty = false;
            app.status = "Dataset cleared.".to_string();
        }
        "DESCRIPTIVES" | "DESCRIPTIVE" => {
            let cmd = Command::parse(line);
            let vars = resolve_vars(app, &cmd.args, true);
            if let Some(res) = vars {
                if res.is_empty() {
                    app.status = "DESCRIPTIVES: no numeric variables resolved.".to_string();
                    return;
                }
                // The statistics the user ticked in the dialog's
                // Statistics... sub-dialog.
                let mut set = match cmd.sub("STATISTICS") {
                    Some(kw) if !kw.is_empty() => stats::StatSet::parse(kw),
                    _ => stats::StatSet::defaults(),
                };
                if set.is_empty() {
                    set = stats::StatSet::defaults();
                }
                if let Some(t) = &app.table {
                    let d = stats::descriptives_with(t, &res, &set);
                    app.outputs
                        .push(OutputTable::descriptives_with(&d, &set));
                    app.status = format!("DESCRIPTIVES executed for {} variables.", d.len());
                    if !app.outputs.is_empty() {
                        app.show_output_window = true;
                    }
                }
            }
        }
        "FREQUENCIES" | "FREQ" => {
            let cmd = Command::parse(line);
            let vars = resolve_vars(app, &cmd.args, false);
            if let Some(res) = vars {
                if res.is_empty() {
                    app.status = "FREQUENCIES: no variables resolved.".to_string();
                    return;
                }
                // Frequencies may also report descriptive statistics.
                let set = cmd
                    .sub("STATISTICS")
                    .filter(|kw| !kw.is_empty())
                    .map(stats::StatSet::parse)
                    .filter(|s| !s.is_empty());
                let show_tables = cmd
                    .sub("FORMAT")
                    .map(|kw| !kw.iter().any(|k| k == "NOTABLE"))
                    .unwrap_or(true);
                // /CHART=BARCHART or /CHART=HISTOGRAM adds a chart.
                let chart_kind = cmd.sub("CHART").and_then(|kw| {
                    kw.iter()
                        .find(|k| k.eq_ignore_ascii_case("BARCHART") || k.eq_ignore_ascii_case("HISTOGRAM"))
                        .cloned()
                });
                if let Some(t) = &app.table {
                    let mut n = 0usize;
                    for &ci in &res {
                        // The output uses the variable label when there is
                        // one; the syntax still resolved the machine name.
                        let name = t.columns[ci].display_name().to_string();
                        if show_tables {
                            let rows = stats::frequencies(t, ci);
                            let chart = if chart_kind.is_some() {
                                Some(crate::output::bar_chart_from_frequencies(&name, &rows))
                            } else {
                                None
                            };
                            app.outputs.push(OutputTable::frequencies_with_chart(&name, &rows, chart));
                        }
                        // A histogram of a numeric variable.
                        if let Some(kind) = &chart_kind {
                            if kind.eq_ignore_ascii_case("HISTOGRAM")
                                && t.columns[ci].kind == crate::data::ColumnKind::Numeric
                            {
                                if let Some(h) = stats::histogram(t, ci, 20) {
                                    app.outputs.push(OutputTable {
                                        title: format!("Histogram: {}", name),
                                        headers: vec!["Bin".into(), "Count".into()],
                                        rows: h
                                            .counts
                                            .iter()
                                            .zip(h.centres.iter())
                                            .map(|(c, x)| {
                                                vec![
                                                    format!("{:.2} - {:.2}", x - h.width / 2.0, x + h.width / 2.0),
                                                    format!("{:.0}", c),
                                                ]
                                            })
                                            .collect(),
                                        note: String::new(),
                                        chart: Some(crate::output::histogram_chart(&h)),
                                        pivot: None,
                                    });
                                }
                            }
                        }
                        if let Some(s) = &set {
                            if t.columns[ci].kind == crate::data::ColumnKind::Numeric {
                                let d = stats::descriptives_with(t, &[ci], s);
                                app.outputs
                                    .push(OutputTable::descriptives_with(&d, s));
                            }
                        }
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
            let cmd = Command::parse(line);
            let vars = resolve_vars(app, &cmd.args, true);
            if let Some(res) = vars {
                if res.len() < 2 {
                    app.status = "CORRELATIONS: need at least two numeric variables.".to_string();
                    return;
                }
                // /PRINT=SIG asks for the significance stars; the
                // DESCRIPTIVES statistic adds the summary block.
                let print_words = cmd.sub("PRINT").unwrap_or(&[]);
                let show_stars = print_words.iter().any(|k| k.eq_ignore_ascii_case("SIG"));
                let show_descriptives = print_words
                    .iter()
                    .any(|k| k.eq_ignore_ascii_case("DESCRIPTIVES"))
                    || cmd
                        .sub("STATISTICS")
                        .map(|kw| kw.iter().any(|k| k.eq_ignore_ascii_case("DESCRIPTIVES")))
                        .unwrap_or(false);
                if let Some(t) = &app.table {
                    let c = stats::correlations_in(t, &res);
                    let descs = if show_descriptives {
                        let set = stats::StatSet::defaults();
                        stats::descriptives_with(t, &res, &set)
                    } else {
                        Vec::new()
                    };
                    app.outputs.push(OutputTable::correlations_with(
                        &c,
                        show_stars,
                        show_descriptives,
                        &descs,
                    ));
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
                        // A scatter plot with the fitted line, as StatsBench draws
                        // it in the regression output.
                        let chart = stats::scatter_points(t, x, yi).map(|s| {
                            crate::output::scatter_chart(&s)
                        });
                        app.outputs.push(OutputTable::regression_with_chart(&r, chart));
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
            // CROSSTABS <var1> BY <var2> [/CELLS=... /STATISTICS=...]
            let cmd = Command::parse(line);
            let rest: Vec<String> = cmd.args.clone();
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
                // The cell counts and statistics the user chose.
                let cells = cmd
                    .sub("CELLS")
                    .filter(|kw| !kw.is_empty())
                    .map(stats::CellCounts::parse)
                    .unwrap_or_default();
                let stats_sel = cmd
                    .sub("STATISTICS")
                    .filter(|kw| !kw.is_empty())
                    .map(stats::CrosstabStats::parse)
                    .unwrap_or_default();
                match stats::crosstabs_chi2(t, i, j) {
                    Some(mut ct) => {
                        stats::crosstabs_extra(&mut ct, &stats_sel);
                        app.outputs.push(OutputTable::crosstabs_with(&ct, &cells, &stats_sel));
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
        "GLM" => {
            // GLM <dependent> BY <factor> [BY <factor> ...] [WITH <covariate>]
            //     /METHOD=SSTYPE(3) /PRINT=DESCRIPTIVE ETASQ
            let cmd = Command::parse(line);
            let rest: Vec<String> = cmd.args.clone();

            // The BY and WITH clauses.
            let mut factor_names: Vec<String> = Vec::new();
            let mut covariate: Option<String> = None;
            let mut dependent: Option<String> = None;
            let mut in_with = false;
            let mut i = 0usize;
            while i < rest.len() {
                let t = &rest[i];
                if t.eq_ignore_ascii_case("BY") {
                    // The factors run until the next BY or WITH.
                    i += 1;
                    while i < rest.len()
                        && !rest[i].eq_ignore_ascii_case("BY")
                        && !rest[i].eq_ignore_ascii_case("WITH")
                    {
                        factor_names.push(rest[i].clone());
                        i += 1;
                    }
                    continue;
                }
                if t.eq_ignore_ascii_case("WITH") {
                    in_with = true;
                    i += 1;
                    continue;
                }
                if in_with {
                    covariate = Some(t.clone());
                } else if dependent.is_none() {
                    dependent = Some(t.clone());
                }
                i += 1;
            }

            let Some(dep) = dependent else {
                app.status = "GLM: expected 'GLM depvar BY factor'.".to_string();
                return;
            };
            if factor_names.is_empty() {
                app.status = "GLM: expected at least one factor after BY.".to_string();
                return;
            }

            // The sum-of-squares type: /METHOD=SSTYPE(3) is the
            // default, and /DESIGN= names the effects.
            let ss_type = cmd
                .sub("METHOD")
                .and_then(|kw| {
                    kw.iter()
                        .find_map(|k| {
                            let t = k.trim_start_matches("SSTYPE").trim_matches(|c| c == '(' || c == ')');
                            t.parse::<i32>().ok().and_then(|n| match n {
                                1 => Some(crate::glm::SumOfSquaresType::TypeI),
                                2 => Some(crate::glm::SumOfSquaresType::TypeII),
                                3 => Some(crate::glm::SumOfSquaresType::TypeIII),
                                4 => Some(crate::glm::SumOfSquaresType::TypeIV),
                                _ => None,
                            })
                        })
                })
                .unwrap_or(crate::glm::SumOfSquaresType::TypeIII);

            let Some(t) = &app.table else {
                app.status = "GLM: no data loaded.".to_string();
                return;
            };
            let Some(yi) = t.column_index(&dep) else {
                app.status = format!("GLM: unknown variable '{}'.", dep);
                return;
            };
            let mut factors: Vec<(usize, String)> = Vec::new();
            for f in &factor_names {
                let Some(fi) = t.column_index(f) else {
                    app.status = format!("GLM: unknown variable '{}'.", f);
                    return;
                };
                factors.push((fi, f.clone()));
            }
            let cov = covariate.as_ref().and_then(|c| t.column_index(c));
            if covariate.is_some() && cov.is_none() {
                app.status = format!(
                    "GLM: unknown covariate '{}'.",
                    covariate.as_deref().unwrap_or("")
                );
                return;
            }

            match crate::glm::glm(t, yi, &factors, cov, ss_type) {
                Some(g) => {
                    app.outputs.push(OutputTable::glm_descriptives(&g));
                    app.outputs.push(OutputTable::glm_tests(&g));
                    app.status = format!(
                        "GLM executed for {} with {} factor(s): {} sums of squares.",
                        dep,
                        factor_names.len(),
                        ss_type.keyword()
                    );
                    app.show_output_window = true;
                }
                None => {
                    app.status = "GLM failed (the factors need at least two levels each).".to_string();
                }
            }
        }
        "ONEWAY" | "ANOVA" => {
            // ONEWAY <y> BY <factor> [/POSTHOC=... /STATISTICS=...]
            let cmd = Command::parse(line);
            let rest: Vec<String> = cmd.args.clone();
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
                // The post-hoc comparisons the user chose, and whether
                // Levene's test was asked for.
                let posthoc = cmd
                    .sub("POSTHOC")
                    .filter(|kw| !kw.is_empty())
                    .map(stats::PostHocSet::parse)
                    .unwrap_or_default();
                let stat_words = cmd.sub("STATISTICS").unwrap_or(&[]);
                let with_levene = stat_words
                    .iter()
                    .any(|k| k.eq_ignore_ascii_case("HOMOGENEITY") || k.eq_ignore_ascii_case("ALL"));
                // /STATISTICS=WELCH adds Welch's test, which does not
                // assume the group variances are equal.
                let with_welch = stat_words
                    .iter()
                    .any(|k| k.eq_ignore_ascii_case("WELCH") || k.eq_ignore_ascii_case("ROBUST"));
                match stats::one_way_anova_with(t, yi, fi, &posthoc, with_levene) {
                    Some(a) => {
                        // The group descriptives go in their own table,
                        // as StatsBench shows them.
                        app.outputs.push(OutputTable::anova_descriptives(&a));
                        if with_welch {
                            // The group data, for the variance tests.
                            if let Some(groups) = stats::collect_groups(t, yi, fi) {
                                if let Some(tests) = stats::anova_variance_tests(&groups) {
                                    app.outputs.push(OutputTable::anova_variance_tests(&tests));
                                }
                            }
                        }
                        app.outputs.push(OutputTable::anova_with(&a));
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
            let cmd = Command::parse(line);
            let rest: Vec<String> = cmd.args.clone();
            // /ES=DISPLAY asks for the effect size table.
            let show_effects = cmd
                .sub("ES")
                .map(|kw| kw.iter().any(|k| k.eq_ignore_ascii_case("DISPLAY")))
                .unwrap_or(false);
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
                        app.outputs.push(OutputTable::ttest_with(&r, show_effects));
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
        "MEANS" => {
            // MEANS TABLES <vars> BY <group> [/CELLS=...]
            let cmd = Command::parse(line);
            let rest: Vec<String> = cmd.args.clone();
            let Some(bi) = rest.iter().position(|t| t.eq_ignore_ascii_case("BY")) else {
                app.status = "MEANS: expected 'MEANS TABLES var BY group'.".to_string();
                return;
            };
            // Skip the optional "TABLES" keyword.
            let y_args: Vec<String> = rest[..bi]
                .iter()
                .filter(|t| !t.eq_ignore_ascii_case("TABLES"))
                .cloned()
                .collect();
            let Some(group) = rest.get(bi + 1).cloned() else {
                app.status = "MEANS: expected 'MEANS TABLES var BY group'.".to_string();
                return;
            };
            if y_args.len() != 1 {
                app.status = "MEANS: expected one variable before BY.".to_string();
                return;
            }
            if let Some(t) = &app.table {
                let (Some(yi), Some(gi)) = (t.column_index(&y_args[0]), t.column_index(&group))
                else {
                    app.status = "MEANS: unknown variable name.".to_string();
                    return;
                };
                match stats::means_by_group(t, yi, gi) {
                    Some(m) => {
                        app.outputs.push(OutputTable::from_means(&m));
                        app.status = format!(
                            "MEANS executed: {} BY {} ({} groups).",
                            m.outcome,
                            m.group_var,
                            m.groups.len()
                        );
                        app.show_output_window = true;
                    }
                    None => {
                        app.status = "MEANS failed (no valid cases).".to_string();
                    }
                }
            }
        }
        "EXPLORE" => {
            // EXPLORE <vars> [/GROUP BY <factor>]
            let cmd = Command::parse(line);
            let vars: Vec<String> = cmd
                .args
                .iter()
                .filter(|t| !t.eq_ignore_ascii_case("BY"))
                .cloned()
                .collect();
            // The grouping variable, if one was given.
            let group = cmd
                .sub("GROUP")
                .map(|kw| kw.first().cloned())
                .unwrap_or(None);
            if let Some(t) = &app.table {
                let mut n = 0usize;
                for v in &vars {
                    let Some(yi) = t.column_index(v) else {
                        app.status = format!("EXPLORE: unknown variable '{}'.", v);
                        return;
                    };
                    let gi = match &group {
                        Some(g) => match t.column_index(g) {
                            Some(i) => Some(i),
                            None => {
                                app.status = format!("EXPLORE: unknown variable '{}'.", g);
                                return;
                            }
                        },
                        None => None,
                    };
                    match stats::explore(t, yi, gi) {
                        Some(e) => {
                            app.outputs.push(OutputTable::from_explore(&e));
                            app.outputs.push(OutputTable::explore_normality(&e));
                            n += 1;
                        }
                        None => {
                            app.status = "EXPLORE failed (need a numeric variable).".to_string();
                            return;
                        }
                    }
                }
                app.status = format!("EXPLORE executed for {} variable(s).", n);
                app.show_output_window = true;
            }
        }
        "NONPARAMETRIC" | "NPAR" => {
            // NPAR TESTS /MW= <y> BY <group>
            // NPAR TESTS /WILCOXON = <v1> <v2>
            // NPAR TESTS /K-W = <y> BY <group>
            // NPAR TESTS /FRIEDMAN = <v1> <v2> <v3>
            let cmd = Command::parse(line);
            let mw = cmd.sub("MW").map(|kw| kw.to_vec());
            let wilcoxon = cmd.sub("WILCOXON").map(|kw| kw.to_vec());
            let kw_test = cmd.sub("K-W").map(|kw| kw.to_vec());
            let friedman = cmd.sub("FRIEDMAN").map(|kw| kw.to_vec());
            if let Some(words) = mw {
                run_by_group(app, &words, "Mann-Whitney", |t, yi, gi| {
                    stats::mann_whitney(t, yi, gi).map(|r| OutputTable::from_mann_whitney(&r))
                });
                return;
            }
            if let Some(words) = wilcoxon {
                run_pairwise(app, &words, "Wilcoxon", |t, a, b| {
                    stats::wilcoxon_signed_rank(t, a, b).map(|r| OutputTable::from_wilcoxon(&r))
                });
                return;
            }
            if let Some(words) = kw_test {
                run_by_group(app, &words, "Kruskal-Wallis", |t, yi, gi| {
                    stats::kruskal_wallis(t, yi, gi).map(|r| OutputTable::from_kruskal_wallis(&r))
                });
                return;
            }
            if let Some(words) = friedman {
                if let Some(t) = &app.table {
                    let idxs: Vec<usize> = words
                        .iter()
                        .filter_map(|n| t.column_index(n))
                        .collect();
                    match stats::friedman(t, &idxs) {
                        Some(r) => {
                            app.outputs.push(OutputTable::from_friedman(&r));
                            app.status = format!(
                                "FRIEDMAN executed: chi2={:.3} p={:.4}",
                                r.chi_square, r.p
                            );
                            app.show_output_window = true;
                        }
                        None => {
                            app.status =
                                "FRIEDMAN failed (need 3 or more related variables).".to_string();
                        }
                    }
                }
                return;
            }
            app.status = "NONPARAMETRIC: expected /MW=, /WILCOXON=, /K-W= or /FRIEDMAN=."
                .to_string();
        }
        "MANN-WHITNEY" | "MW" => {
            // The shortcut form: MANN-WHITNEY <y> BY <group>
            let cmd = Command::parse(line);
            let words = cmd.args.clone();
            run_by_group(app, &words, "Mann-Whitney", |t, yi, gi| {
                stats::mann_whitney(t, yi, gi).map(|r| OutputTable::from_mann_whitney(&r))
            });
        }
        "WILCOXON" => {
            // WILCOXON <v1> <v2>
            let cmd = Command::parse(line);
            let words = cmd.args.clone();
            run_pairwise(app, &words, "Wilcoxon", |t, a, b| {
                stats::wilcoxon_signed_rank(t, a, b).map(|r| OutputTable::from_wilcoxon(&r))
            });
        }
        "KRUSKAL-WALLIS" | "K-W" => {
            let cmd = Command::parse(line);
            let words = cmd.args.clone();
            run_by_group(app, &words, "Kruskal-Wallis", |t, yi, gi| {
                stats::kruskal_wallis(t, yi, gi).map(|r| OutputTable::from_kruskal_wallis(&r))
            });
        }
        "FRIEDMAN" => {
            let cmd = Command::parse(line);
            let words = cmd.args.clone();
            if let Some(t) = &app.table {
                let idxs: Vec<usize> = words.iter().filter_map(|n| t.column_index(n)).collect();
                match stats::friedman(t, &idxs) {
                    Some(r) => {
                        app.outputs.push(OutputTable::from_friedman(&r));
                        app.status = format!(
                            "FRIEDMAN executed: chi2={:.3} p={:.4}",
                            r.chi_square, r.p
                        );
                        app.show_output_window = true;
                    }
                    None => {
                        app.status =
                            "FRIEDMAN failed (need 3 or more related variables).".to_string();
                    }
                }
            }
        }
        "RELIABILITY" | "ALPHA" => {
            // RELIABILITY SCALE <items> [SCALE(name)]
            let cmd = Command::parse(line);
            // The scale name, if one was given.
            let scale_name = cmd
                .sub("SCALE")
                .and_then(|kw| kw.first().cloned())
                .unwrap_or_else(|| "SCALE".to_string());
            let items: Vec<String> = cmd
                .args
                .iter()
                .filter(|t| !t.eq_ignore_ascii_case("TABLES"))
                .cloned()
                .collect();
            if items.is_empty() {
                app.status = "RELIABILITY: expected at least two items.".to_string();
                return;
            }
            if let Some(t) = &app.table {
                let idxs: Vec<usize> = items
                    .iter()
                    .filter_map(|n| t.column_index(n))
                    .collect();
                if idxs.len() != items.len() {
                    app.status = "RELIABILITY: unknown variable name.".to_string();
                    return;
                }
                match stats::cronbach_alpha(t, &idxs, &scale_name) {
                    Some(r) => {
                        app.outputs.push(OutputTable::from_reliability(&r));
                        app.outputs.push(OutputTable::reliability_items(&r));
                        app.status = format!(
                            "RELIABILITY executed for {} items: Cronbach's alpha = {:.3}",
                            r.n_items, r.alpha
                        );
                        app.show_output_window = true;
                    }
                    None => {
                        app.status = "RELIABILITY failed (need 2+ numeric items).".to_string();
                    }
                }
            }
        }
        "FACTOR" | "FACTORS" => {
            // FACTOR <items> [/EXTRACTION=PC] [/ROTATION=VARIMAX]
            let cmd = Command::parse(line);
            let items: Vec<String> = cmd
                .args
                .iter()
                .filter(|t| !t.eq_ignore_ascii_case("VARIABLES"))
                .cloned()
                .collect();
            if items.is_empty() {
                app.status = "FACTOR: expected at least three items.".to_string();
                return;
            }
            // The number of components to keep.
            let n_comp = cmd
                .sub("EXTRACTION")
                .and_then(|kw| {
                    kw.iter()
                        .find_map(|k| k.trim_start_matches("PC").parse::<usize>().ok())
                })
                .unwrap_or(1);
            if let Some(t) = &app.table {
                let idxs: Vec<usize> = items
                    .iter()
                    .filter_map(|n| t.column_index(n))
                    .collect();
                if idxs.len() != items.len() {
                    app.status = "FACTOR: unknown variable name.".to_string();
                    return;
                }
                match stats::factor_analysis(t, &idxs, n_comp) {
                    Some(f) => {
                        app.outputs.push(OutputTable::from_factor(&f));
                        app.outputs.push(OutputTable::factor_component_matrix(&f));
                        app.outputs.push(OutputTable::factor_communalities(&f));
                        app.status = format!(
                            "FACTOR executed for {} items, {} component(s) retained.",
                            f.n_items, f.n_components
                        );
                        app.show_output_window = true;
                    }
                    None => {
                        app.status = "FACTOR failed (need 3+ numeric items).".to_string();
                    }
                }
            }
        }
        "ROC" | "ROC-CURVE" => {
            // ROC <testvar> BY <groupvar> [POSITIVE(<group>)]
            let cmd = Command::parse(line);
            let rest: Vec<String> = cmd.args.clone();
            let Some(bi) = rest.iter().position(|t| t.eq_ignore_ascii_case("BY")) else {
                app.status = "ROC: expected 'ROC var BY group'.".to_string();
                return;
            };
            let Some(group) = rest.get(bi + 1).cloned() else {
                app.status = "ROC: expected 'ROC var BY group'.".to_string();
                return;
            };
            if rest[0].is_empty() {
                app.status = "ROC: expected a test variable.".to_string();
                return;
            }
            // The group that counts as positive.
            let positive = cmd
                .sub("POSITIVE")
                .and_then(|kw| kw.first().cloned())
                .unwrap_or_else(|| group.clone());
            if let Some(t) = &app.table {
                let (Some(yi), Some(gi)) = (t.column_index(&rest[0]), t.column_index(&group))
                else {
                    app.status = "ROC: unknown variable name.".to_string();
                    return;
                };
                match stats::roc_curve(t, yi, gi, &positive) {
                    Some(r) => {
                        app.outputs.push(OutputTable::roc_summary(&r));
                        app.outputs.push(OutputTable::from_roc(&r));
                        app.status = format!(
                            "ROC executed: area under the curve = {:.3}",
                            r.auc
                        );
                        app.show_output_window = true;
                    }
                    None => {
                        app.status =
                            "ROC failed (the grouping variable needs exactly two groups).".to_string();
                    }
                }
            }
        }
        "COMPUTE" => {
            // COMPUTE <newvar> = <expression>
            let cmd = Command::parse(line);
            let Some(eq) = cmd.args.iter().position(|a| a == "=") else {
                app.status = "COMPUTE: expected 'COMPUTE name = expression'.".to_string();
                return;
            };
            if eq == 0 || eq + 1 >= cmd.args.len() {
                app.status = "COMPUTE: expected 'COMPUTE name = expression'.".to_string();
                return;
            }
            let name = cmd.args[0].clone();
            // The expression: the words after the '='.
            let expression = cmd.args[eq + 1..].join(" ");
            // The variable and its values are a change to the dataset, so
            // the dataset is remembered as it was before the command.
            let before = app.table.clone();
            let Some(t) = &mut app.table else {
                app.status = "COMPUTE: no data loaded.".to_string();
                return;
            };
            match stats::compute_variable(t, &name, &expression) {
                Ok(n) => {
                    app.record_table_change(before);
                    app.dirty = true;
                    app.status = format!("COMPUTE created the variable {}.", n);
                }
                Err(e) => {
                    app.status = format!("COMPUTE failed: {}", e);
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
            // The rows move, so the order before the sort is remembered
            // and the sort can be taken back.
            let before = app.table.clone();
            if let Some(t) = &mut app.table {
                match t.column_index(&name) {
                    Some(ci) => {
                        t.sort_rows(ci, asc);
                        app.record_table_change(before);
                        app.dirty = true;
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

/// Run a nonparametric test that takes one variable split by a group.
fn run_by_group<F>(
    app: &mut StatsCraftApp,
    words: &[String],
    name: &str,
    f: F,
) where
    F: Fn(&crate::data::DataTable, usize, usize) -> Option<OutputTable>,
{
    let Some(bi) = words.iter().position(|t| t.eq_ignore_ascii_case("BY")) else {
        app.status = format!("{}: expected '{} var BY group'.", name, name);
        return;
    };
    let y_args: Vec<String> = words[..bi]
        .iter()
        .filter(|t| !t.eq_ignore_ascii_case("TESTS"))
        .cloned()
        .collect();
    let Some(group) = words.get(bi + 1).cloned() else {
        app.status = format!("{}: expected '{} var BY group'.", name, name);
        return;
    };
    if y_args.len() != 1 {
        app.status = format!("{}: expected one variable before BY.", name);
        return;
    }
    if let Some(t) = &app.table {
        let (Some(yi), Some(gi)) = (t.column_index(&y_args[0]), t.column_index(&group)) else {
            app.status = format!("{}: unknown variable name.", name);
            return;
        };
        match f(t, yi, gi) {
            Some(out) => {
                app.outputs.push(out);
                app.status = format!("{} executed for {} BY {}.", name, y_args[0], group);
                app.show_output_window = true;
            }
            None => {
                app.status = format!("{} failed (not enough valid cases).", name);
            }
        }
    }
}

/// Run a nonparametric test that compares two variables directly.
fn run_pairwise<F>(
    app: &mut StatsCraftApp,
    words: &[String],
    name: &str,
    f: F,
) where
    F: Fn(&crate::data::DataTable, usize, usize) -> Option<OutputTable>,
{
    let vars: Vec<String> = words
        .iter()
        .filter(|t| !t.eq_ignore_ascii_case("TESTS"))
        .cloned()
        .collect();
    if vars.len() != 2 {
        app.status = format!("{}: expected two variables.", name);
        return;
    }
    if let Some(t) = &app.table {
        let (Some(a), Some(b)) = (t.column_index(&vars[0]), t.column_index(&vars[1])) else {
            app.status = format!("{}: unknown variable name.", name);
            return;
        };
        match f(t, a, b) {
            Some(out) => {
                app.outputs.push(out);
                app.status = format!("{} executed for {} and {}.", name, vars[0], vars[1]);
                app.show_output_window = true;
            }
            None => {
                app.status = format!("{} failed (not enough valid pairs).", name);
            }
        }
    }
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
    fn the_parser_splits_a_command_into_args_and_subcommands() {
        let c = Command::parse("DESCRIPTIVES score age /STATISTICS=MEAN STDDEV /SORT=MEAN");
        assert_eq!(c.name, "DESCRIPTIVES");
        assert_eq!(c.args, vec!["score", "age"]);
        assert_eq!(c.sub("STATISTICS").unwrap(), &["MEAN".to_string(), "STDDEV".to_string()]);
        assert_eq!(c.sub("SORT").unwrap(), &["MEAN".to_string()]);
        assert!(c.sub("MISSING").is_none());
    }

    #[test]
    fn the_parser_finds_the_position_of_a_keyword() {
        let c = Command::parse("CROSSTABS region BY group");
        assert_eq!(c.pos("BY"), Some(1));
    }

    #[test]
    fn a_stat_set_is_parsed_from_its_keywords() {
        let s = crate::stats::StatSet::parse(&[
            "MEAN".to_string(),
            "MEDIAN".to_string(),
            "STDDEV".to_string(),
        ]);
        assert!(s.mean && s.median && s.stddev);
        assert!(!s.variance);
        assert!(!s.is_empty());
    }

    #[test]
    fn the_whole_phase_three_script_runs() {
        let dir = std::env::temp_dir();
        let csv = dir.join("statscraft_p3_script.csv");
        std::fs::write(
            &csv,
            "id,region,group,age,satisfaction,score,response_time\n\
             1,North,A,25,High,72,120\n\
             2,North,B,31,Low,68,145\n\
             3,South,A,28,High,75,110\n\
             4,South,B,45,Med,61,190\n\
             5,East,A,22,High,80,95\n\
             6,East,B,38,Low,55,205\n\
             7,West,A,29,Med,70,130\n\
             8,West,B,41,High,66,175\n\
             9,North,A,33,Low,58,160\n\
             10,South,B,27,High,85,88\n\
             11,East,A,36,Med,73,120\n\
             12,West,B,24,High,90,100\n",
        )
        .unwrap();
        let mut app = StatsCraftApp::default();
        app.path_input = csv.display().to_string();
        app.load();

        let script = "\
FREQUENCIES region /CHART=BARCHART
FREQUENCIES score /CHART=HISTOGRAM
DESCRIPTIVES score age response_time /STATISTICS=MEAN MEDIAN STDDEV VARIANCE
CROSSTABS region BY group /CELLS=COUNT ROW COLUMN EXPECTED /STATISTICS=CHISQ PHI
ONEWAY score BY group /POSTHOC=TUKEY SCHEFFE /STATISTICS=HOMOGENEITY
T-TEST score BY group /ES=DISPLAY
CORRELATIONS score age response_time /PRINT=SIG DESCRIPTIVES
REGRESSION score WITH age
MEANS TABLES score BY group
EXPLORE score
NONPARAMETRIC /MW= score BY group
NONPARAMETRIC /K-W= score BY group
NONPARAMETRIC /WILCOXON= score age
NONPARAMETRIC /FRIEDMAN= score age response_time
";
        run(&mut app, script);

        // Every command should have produced output, and no status should
        // report a failure.
        assert!(
            !app.status.to_lowercase().contains("failed"),
            "a command failed: {}",
            app.status
        );
        assert!(
            !app.status.to_lowercase().contains("unknown"),
            "a command was unknown: {}",
            app.status
        );
        assert!(app.outputs.len() >= 14, "only {} outputs", app.outputs.len());

        // The crosstab must carry the extra cell counts. They are
        // measures of the same cells, so they live in the pivot.
        let crosstab = app
            .outputs
            .iter()
            .find(|o| o.title.starts_with("Crosstabulation"))
            .expect("a crosstab");
        let pivot = crosstab.pivot.as_ref().expect("a pivot table");
        assert!(
            pivot.measures.iter().any(|m| m == "Row %"),
            "no row percentages: {:?}",
            pivot.measures
        );
        assert!(
            pivot.measures.iter().any(|m| m == "Column %"),
            "no column percentages: {:?}",
            pivot.measures
        );
        assert!(
            pivot.measures.iter().any(|m| m == "Expected Count"),
            "no expected counts: {:?}",
            pivot.measures
        );
        // The totals are levels of the dimensions, as StatsBench has them.
        assert_eq!(
            pivot.row_dims[0].levels.last().map(|s| s.as_str()),
            Some("Total")
        );
        assert_eq!(
            pivot.col_dims[0].levels.last().map(|s| s.as_str()),
            Some("Total")
        );
        assert!(crosstab.note.contains("Phi"), "no Phi: {}", crosstab.note);

        // The ANOVA must carry the post-hoc comparisons.
        let anova = app
            .outputs
            .iter()
            .find(|o| o.title.starts_with("ANOVA"))
            .expect("an ANOVA");
        assert!(
            anova.rows.iter().any(|r| r[0].contains("Tukey")),
            "no Tukey rows"
        );
        assert!(anova.note.contains("Levene"), "no Levene: {}", anova.note);

        // The t-test must carry the effect size table.
        let ttest = app
            .outputs
            .iter()
            .find(|o| o.title.contains("T-Test"))
            .expect("a t-test");
        assert!(
            ttest.rows.iter().any(|r| r[0].contains("Cohen")),
            "no Cohen's d"
        );

        // The correlation matrix must carry the significance stars.
        let corr = app
            .outputs
            .iter()
            .find(|o| o.title == "Correlations")
            .expect("the correlations");
        assert!(corr.note.contains("0.01 level"));

        // The chart must be attached to at least one output.
        assert!(
            app.outputs.iter().any(|o| o.chart.is_some()),
            "no chart was produced"
        );
    }

    #[test]
    fn the_glm_runs_through_the_syntax() {
        let path = std::env::temp_dir().join("sc_glm.csv");
        let mut f = std::fs::File::create(&path).unwrap();
        use std::io::Write;
        writeln!(f, "g,c,y").unwrap();
        // A 2x2 with a real interaction.
        for (g, c, base) in [
            ("A1", "B1", 120.0),
            ("A1", "B2", 110.0),
            ("A2", "B1", 104.0),
            ("A2", "B2", 99.0),
        ] {
            for i in 0..40 {
                writeln!(f, "{},{},{}", g, c, base + i as f64 * 0.1).unwrap();
            }
        }
        let mut app = StatsCraftApp::default();
        app.path_input = path.display().to_string();
        app.load();

        run(&mut app, "GLM y BY g c /METHOD=SSTYPE(3)");
        assert!(
            !app.status.contains("failed") && !app.status.contains("unknown"),
            "GLM failed: {}",
            app.status
        );
        assert_eq!(app.outputs.len(), 2, "the descriptives and the tests");
        let tests = &app.outputs[1];
        assert!(
            tests.title.contains("Between-Subjects Effects"),
            "the title was {}",
            tests.title
        );
        // The three effects are there, with the interaction found.
        let names: Vec<&str> = tests.rows.iter().map(|r| r[0].as_str()).collect();
        assert!(names.contains(&"g"), "the rows were {:?}", names);
        assert!(names.contains(&"c"), "the rows were {:?}", names);
        assert!(names.contains(&"g*c"), "the rows were {:?}", names);
        // The interaction's F is large, because the design has one.
        let interaction = tests
            .rows
            .iter()
            .find(|r| r[0] == "g*c")
            .expect("the interaction row");
        let f: f64 = interaction[4].parse().unwrap();
        assert!(f > 10.0, "the interaction's F was {}", f);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn a_parenthesised_subcommand_value_is_parsed() {
        // "/POSITIVE(case)" is one keyword with one value.
        let cmd = Command::parse("ROC score BY status /POSITIVE(case)");
        assert_eq!(
            cmd.sub("POSITIVE"),
            Some(&["case".to_string()][..]),
            "the subcommand was {:?}",
            cmd.subcommands
        );
        // A keyword with no value still works.
        let cmd = Command::parse("FREQUENCIES x /STATISTICS=MEAN");
        assert_eq!(cmd.sub("STATISTICS"), Some(&["MEAN".to_string()][..]));
    }

    #[test]
    fn compute_creates_a_new_variable() {
        let dir = std::env::temp_dir();
        let csv = dir.join("statscraft_compute.csv");
        std::fs::write(&csv, "x,y\n1,10\n2,20\n3,30\n4,40\n").unwrap();
        let mut app = StatsCraftApp::default();
        app.path_input = csv.display().to_string();
        app.load();

        run(&mut app, "COMPUTE z = x + y");
        assert!(
            !app.status.starts_with("COMPUTE failed"),
            "COMPUTE failed: {}",
            app.status
        );
        let zi = app.table.as_ref().unwrap().column_index("z").expect("z");
        let t = app.table.as_ref().unwrap();
        match t.columns[zi].values[0] {
            Value::Number(v) => assert!((v - 11.0).abs() < 1e-9, "z was {}", v),
            _ => panic!("z is not numeric"),
        }
        match t.columns[zi].values[3] {
            Value::Number(v) => assert!((v - 44.0).abs() < 1e-9, "z was {}", v),
            _ => panic!("z is not numeric"),
        }
    }

    #[test]
    fn compute_handles_functions_and_parentheses() {
        let dir = std::env::temp_dir();
        let csv = dir.join("statscraft_compute2.csv");
        std::fs::write(&csv, "a,b\n2,4\n4,8\n6,12\n").unwrap();
        let mut app = StatsCraftApp::default();
        app.path_input = csv.display().to_string();
        app.load();

        // MEAN(2, 4) = 3, and 3 * 2 = 6.
        run(&mut app, "COMPUTE m = MEAN(a, b) * 2");
        let mi = app.table.as_ref().unwrap().column_index("m").expect("m");
        let t = app.table.as_ref().unwrap();
        match t.columns[mi].values[0] {
            Value::Number(v) => assert!((v - 6.0).abs() < 1e-9, "m was {}", v),
            _ => panic!("m is not numeric"),
        }
        // The second case: MEAN(4, 8) = 6, and 6 * 2 = 12.
        match t.columns[mi].values[1] {
            Value::Number(v) => assert!((v - 12.0).abs() < 1e-9, "m was {}", v),
            _ => panic!("m is not numeric"),
        }
    }

    #[test]
    fn the_buffer_is_split_into_commands() {
        let script = "DESCRIPTIVES score\nFREQUENCIES group\nREGRESSION score WITH age\n";
        let commands = split_commands(script);
        assert_eq!(commands.len(), 3);
        assert_eq!(commands[0].text, "DESCRIPTIVES score");
        assert_eq!(commands[0].first_line, 0);
        assert_eq!(commands[0].last_line, 0);
        assert_eq!(commands[1].text, "FREQUENCIES group");
        assert_eq!(commands[2].text, "REGRESSION score WITH age");
    }

    #[test]
    fn a_subcommand_line_stays_with_the_command_above_it() {
        // A line that begins with a slash is a subcommand of the command
        // above it, so the two lines are one command.
        let script = "REGRESSION score WITH age\n/PRINT=SIG\nFREQUENCIES group\n";
        let commands = split_commands(script);
        assert_eq!(commands.len(), 2);
        assert_eq!(
            commands[0].text,
            "REGRESSION score WITH age /PRINT=SIG",
            "the subcommand joined the command"
        );
        assert_eq!(commands[0].first_line, 0);
        assert_eq!(commands[0].last_line, 1);
        assert_eq!(commands[1].text, "FREQUENCIES group");
    }

    #[test]
    fn a_comment_is_a_command_of_its_own() {
        let script = "* a note\nDESCRIPTIVES score\n";
        let commands = split_commands(script);
        assert_eq!(commands.len(), 2);
        assert_eq!(commands[0].text, "* a note");
        assert_eq!(commands[1].text, "DESCRIPTIVES score");
    }

    #[test]
    fn the_command_at_the_cursor_is_found() {
        let script = "DESCRIPTIVES score\nFREQUENCIES group\nREGRESSION score WITH age\n";
        let c = command_at_line(script, 1).expect("a command on the second line");
        assert_eq!(c.text, "FREQUENCIES group");
        // A line past the end of the buffer has no command.
        assert!(command_at_line(script, 9).is_none());
    }

    #[test]
    fn a_sort_can_be_taken_back() {
        let path = std::env::temp_dir().join("sc_sort_undo.csv");
        write_demo_csv(&path);
        let mut app = StatsCraftApp::default();
        app.path_input = path.display().to_string();
        app.load();
        let before = app.table.as_ref().unwrap().columns[3].values.clone();
        run_command(&mut app, "SORT CASES BY age (D)");
        let after = app.table.as_ref().unwrap().columns[3].values.clone();
        assert_ne!(before, after, "the sort changed the order");
        // Undo puts the rows back where they were.
        app.undo();
        assert_eq!(app.table.as_ref().unwrap().columns[3].values, before);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn a_computed_variable_can_be_taken_back() {
        let path = std::env::temp_dir().join("sc_compute_undo.csv");
        write_demo_csv(&path);
        let mut app = StatsCraftApp::default();
        app.path_input = path.display().to_string();
        app.load();
        let cols = app.table.as_ref().unwrap().columns.len();
        run_command(&mut app, "COMPUTE twice = score * 2");
        assert_eq!(app.table.as_ref().unwrap().columns.len(), cols + 1);
        // Undo takes the new variable away again.
        app.undo();
        assert_eq!(app.table.as_ref().unwrap().columns.len(), cols);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn the_commands_of_a_selection_are_the_ones_it_holds() {
        let text = "* a comment\nFREQUENCIES region\nDESCRIPTIVES score\nCROSSTABS a BY b\n";
        // A selection of the second and third lines.
        let from = text.find("FREQUENCIES").unwrap();
        let to = text.find("CROSSTABS").unwrap();
        let commands = commands_in(text, from, to);
        assert_eq!(
            commands,
            vec![
                "FREQUENCIES region".to_string(),
                "DESCRIPTIVES score".to_string()
            ]
        );
        // A selection of a comment alone holds no command to run.
        assert!(commands_in(text, 0, text.find("FREQUENCIES").unwrap()).is_empty());
    }

    #[test]
    fn the_log_records_the_commands_that_were_run() {
        let path = std::env::temp_dir().join("sc_log_test.csv");
        write_demo_csv(&path);
        let mut app = StatsCraftApp::default();
        run(
            &mut app,
            &format!(
                "LOAD \"{}\"\nDESCRIPTIVES score\nFREQUENCIES group\n",
                path.display()
            ),
        );
        // The Log holds the commands of the run, and nothing else.
        assert_eq!(app.log.len(), 3, "the log was {:?}", app.log);
        assert!(app.log[1].starts_with("DESCRIPTIVES"));
        assert!(app.log[2].starts_with("FREQUENCIES"));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn running_one_command_records_only_that_command() {
        let path = std::env::temp_dir().join("sc_log_one.csv");
        write_demo_csv(&path);
        let mut app = StatsCraftApp::default();
        app.path_input = path.display().to_string();
        app.load();
        run_command(&mut app, "DESCRIPTIVES score");
        assert_eq!(app.log, vec!["DESCRIPTIVES score".to_string()]);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn running_a_selection_records_only_the_selected_commands() {
        let path = std::env::temp_dir().join("sc_log_sel.csv");
        write_demo_csv(&path);
        let mut app = StatsCraftApp::default();
        app.path_input = path.display().to_string();
        app.load();
        // The reader selects three lines and runs them.
        let selection = "DESCRIPTIVES score\nFREQUENCIES group\nCORRELATIONS score age\n";
        let commands: Vec<String> = split_commands(selection)
            .into_iter()
            .map(|c| c.text)
            .filter(|t| !t.starts_with('*'))
            .collect();
        run_commands(&mut app, &commands);
        assert_eq!(app.log.len(), 3, "the log was {:?}", app.log);
        assert!(app.log[0].starts_with("DESCRIPTIVES"));
        assert!(app.log[2].starts_with("CORRELATIONS"));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn the_statistics_subcommand_changes_the_output_table() {
        let dir = std::env::temp_dir();
        let csv = dir.join("statscraft_stats_test.csv");
        std::fs::write(
            &csv,
            "id,score\n1,10\n2,12\n3,14\n4,16\n5,18\n6,20\n",
        )
        .unwrap();
        let mut app = StatsCraftApp::default();
        app.path_input = csv.display().to_string();
        app.load();
        run(
            &mut app,
            "DESCRIPTIVES score /STATISTICS=MEAN MEDIAN VARIANCE",
        );
        assert_eq!(app.outputs.len(), 1);
        let t = &app.outputs[0];
        // One block: the variable row, then Mean, Median, Variance.
        assert!(t.headers.contains(&"Median".to_string()), "headers: {:?}", t.headers);
        assert!(t.headers.contains(&"Variance".to_string()), "headers: {:?}", t.headers);
        assert!(!t.headers.contains(&"Std. Deviation".to_string()));
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
        // The ANOVA now produces the group descriptives as their own
        // table, so there are four outputs.
        assert_eq!(app.outputs.len(), 4, "four outputs");
        assert!(app.outputs[0].title.contains("Crosstabulation"));
        // The group descriptives come first, then the ANOVA table.
        assert!(app.outputs[1].title.contains("Descriptives"));
        assert!(app.outputs[2].title.contains("ANOVA"));
        assert!(app.outputs[3].title.contains("Paired"));
        let _ = std::fs::remove_file(path);
    }
}
