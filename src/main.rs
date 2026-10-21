mod app;
mod data;
mod dialog;
mod icon;
mod output;
mod stats;
mod syntax;
mod theme;

use dialog::Procedure;

fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();

    // `statscraft <data.csv> [--dialog FREQUENCIES]`
    // The optional --dialog flag opens a procedure dialog on start, which
    // is handy for demos and for driving the app from a script.
    let mut path: Option<String> = None;
    let mut procedure: Option<Procedure> = None;
    let mut script: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        let a = args[i].to_ascii_lowercase();
        if a == "--dialog" || a == "-d" {
            if i + 1 < args.len() {
                procedure = parse_procedure(&args[i + 1]);
                if procedure.is_none() {
                    eprintln!(
                        "Unknown procedure '{}'. Known: {}",
                        args[i + 1],
                        known_procedures().join(", ")
                    );
                }
                i += 1;
            }
        } else if a == "--script" || a == "-s" {
            // A syntax file to run at start-up.
            if i + 1 < args.len() {
                script = Some(args[i + 1].clone());
                i += 1;
            }
        } else if path.is_none() {
            path = Some(args[i].clone());
        }
        i += 1;
    }

    app::run_with(path, procedure, script)
}

/// Parse a procedure name into the matching dialog.
fn parse_procedure(name: &str) -> Option<Procedure> {
    let n = name.to_ascii_lowercase().replace(['-', ' '], "_");
    match n.as_str() {
        "frequencies" | "frequency" | "freq" => Some(Procedure::Frequencies),
        "descriptives" | "descriptive" => Some(Procedure::Descriptives),
        "crosstabs" | "crosstab" => Some(Procedure::Crosstabs),
        "ttest" | "t_test" | "independent_t" => Some(Procedure::TTest),
        "pairedt" | "paired_t" | "paired_t_test" => Some(Procedure::PairedT),
        "anova" | "oneway" | "one_way_anova" => Some(Procedure::Anova),
        "correlations" | "bivariate" | "correlation" => Some(Procedure::Correlations),
        "regression" | "linear_regression" => Some(Procedure::Regression),
        "means" => Some(Procedure::Means),
        "explore" => Some(Procedure::Explore),
        "mann_whitney" | "mannwhitney" | "mw" => Some(Procedure::MannWhitney),
        "wilcoxon" => Some(Procedure::Wilcoxon),
        "kruskal_wallis" | "kruskalwallis" | "kruskal" => Some(Procedure::KruskalWallis),
        "friedman" => Some(Procedure::Friedman),
        "reliability" | "alpha" => Some(Procedure::Reliability),
        "factor" => Some(Procedure::Factor),
        "roc" => Some(Procedure::Roc),
        _ => None,
    }
}

fn known_procedures() -> Vec<&'static str> {
    vec![
        "frequencies",
        "descriptives",
        "crosstabs",
        "ttest",
        "pairedt",
        "anova",
        "correlations",
        "regression",
        "means",
        "explore",
        "mann_whitney",
        "wilcoxon",
        "kruskal_wallis",
        "friedman",
        "reliability",
        "factor",
        "roc",
    ]
}
