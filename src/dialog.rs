//! Procedure dialogs — the StatsBench workflow of *choose variables, choose
//! options, then generate syntax and run it*.
//!
//! StatsBench does not run a procedure straight from the Analyze
//! menu. It opens a dialog window with:
//!
//! * a **source variable list** on the left, each row carrying a
//!   measure-level icon (a pencil for string/nominal, a ruler for
//!   numeric/scale);
//! * a **target variable list** in the middle, initially empty;
//! * a single **move arrow** between them;
//! * a right-hand column of **sub-dialog buttons** (`Statistics…`,
//!   `Charts…`, `Format…`) that open further options;
//! * procedure-specific checkboxes along the bottom;
//! * a button row of `OK`, `Paste`, `Reset`, `Cancel`, `Help` where **OK
//!   stays disabled until at least one variable is chosen**.
//!
//! This module reproduces that structure. Each dialog *builds command syntax*
//! and hands it to the existing runner, so the dialog -> syntax -> output
//! pipeline works exactly as the classic workbench does it, including `Paste`,
//! which puts the generated syntax into the Syntax Editor without running it.
//!
//! The geometry and wording were taken from the reference workbench (version
//! 27); see `docs/REFERENCE.md`.

use crate::app::StatsCraftApp;
use crate::data::ColumnKind;
use crate::theme::{self, Icon};
use eframe::egui;

/// Which procedure a dialog is configuring.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum Procedure {
    Frequencies,
    Descriptives,
    Crosstabs,
    TTest,
    PairedT,
    Anova,
    Correlations,
    Regression,
    Means,
    Explore,
    MannWhitney,
    Wilcoxon,
    KruskalWallis,
    Friedman,
    Reliability,
    Factor,
    Roc,
    Glm,
    OneSampleT,
    Proportion,
}

impl Procedure {
    /// The window title, matching StatsBench's dialog captions.
    pub fn title(self) -> &'static str {
        match self {
            Procedure::Frequencies => "Frequencies",
            Procedure::Descriptives => "Descriptives",
            Procedure::Crosstabs => "Crosstabs",
            Procedure::TTest => "Independent-Samples T Test",
            Procedure::PairedT => "Paired-Samples T Test",
            Procedure::Anova => "One-Way ANOVA",
            Procedure::Correlations => "Bivariate Correlation",
            Procedure::Regression => "Linear Regression",
            Procedure::Means => "Means",
            Procedure::Explore => "Explore",
            Procedure::MannWhitney => "Mann-Whitney Test",
            Procedure::Wilcoxon => "Wilcoxon Signed-Rank Test",
            Procedure::KruskalWallis => "Kruskal-Wallis Test",
            Procedure::Friedman => "Friedman Test",
            Procedure::Reliability => "Reliability Analysis",
            Procedure::Factor => "Factor Analysis",
            Procedure::Roc => "ROC Curve",
            Procedure::Glm => "General Linear Model",
            Procedure::OneSampleT => "One-Sample T Test",
            Procedure::Proportion => "One-Sample Proportion",
        }
    }

    /// The command name this dialog generates.
    pub fn command(self) -> &'static str {
        match self {
            Procedure::Frequencies => "FREQUENCIES",
            Procedure::Descriptives => "DESCRIPTIVES",
            Procedure::Crosstabs => "CROSSTABS",
            Procedure::TTest => "T-TEST",
            Procedure::PairedT => "T-TEST PAIRED",
            Procedure::Anova => "ONEWAY",
            Procedure::Correlations => "CORRELATIONS",
            Procedure::Regression => "REGRESSION",
            Procedure::Means => "MEANS",
            Procedure::Explore => "EXPLORE",
            Procedure::MannWhitney => "NONPARAMETRIC /MW=",
            Procedure::Wilcoxon => "NONPARAMETRIC /WILCOXON=",
            Procedure::KruskalWallis => "NONPARAMETRIC /K-W=",
            Procedure::Friedman => "NONPARAMETRIC /FRIEDMAN=",
            Procedure::Reliability => "RELIABILITY",
            Procedure::Factor => "FACTOR",
            Procedure::Roc => "ROC",
            Procedure::Glm => "GLM",
            Procedure::OneSampleT => "T-TEST ONESAMPLE",
            Procedure::Proportion => "PROPORTION",
        }
    }

    /// True when the procedure needs a grouping variable as well as the
    /// measurement variable (t-test, ANOVA, means, crosstabs).
    pub fn needs_group(self) -> bool {
        matches!(
            self,
            Procedure::TTest
                | Procedure::Anova
                | Procedure::Means
                | Procedure::Crosstabs
                | Procedure::MannWhitney
                | Procedure::KruskalWallis
                | Procedure::Roc
        )
    }

    /// True when the procedure operates on numeric variables only.
    pub fn numeric_only(self) -> bool {
        !matches!(
            self,
            Procedure::Frequencies | Procedure::Crosstabs | Procedure::Wilcoxon
        )
    }
}

/// Which sub-dialog is currently open, if any. StatsBench opens these as
/// separate windows stacked on the parent dialog.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum SubDialog {
    None,
    FrequenciesStatistics,
    FrequenciesCharts,
    FrequenciesFormat,
    DescriptivesOptions,
    CrosstabsCells,
    CrosstabsStatistics,
    TTestOptions,
    AnovaPostHoc,
    AnovaOptions,
    CorrelationsOptions,
    RegressionStatistics,
    MeansOptions,
    ExploreOptions,
    FactorOptions,
    RocOptions,
    GlmOptions,
}

/// The statistic choices offered by the `Statistics…` sub-dialog, keyed by
/// a stable id. The set mirrors the `STATISTICS` subcommand of the
/// corresponding StatsBench command.
#[derive(Clone)]
pub struct StatOptions {
    /// Which statistics are ticked. Empty means "the StatsBench default set".
    pub selected: Vec<&'static str>,
    /// For Frequencies: show a bar chart / histogram in the Viewer.
    pub show_charts: bool,
    /// For Frequencies: show the frequency table.
    pub display_tables: bool,
    /// For Descriptives: save standardised values as new variables.
    pub save_standardised: bool,
    /// For Crosstabs: which cell counts to show.
    pub cell_counts: Vec<&'static str>,
    /// For Crosstabs: which statistics to compute.
    pub crosstabs_stats: Vec<&'static str>,
    /// For ANOVA: which post-hoc comparisons to apply.
    pub posthoc: Vec<&'static str>,
    /// For the t-test: compute effect sizes.
    pub effect_sizes: bool,
    /// For correlations: show significance levels and mark them with stars.
    pub significance_stars: bool,
    /// For correlations: delete pairwise instead of listwise.
    pub pairwise_delete: bool,
    /// For the general linear model: the type of the sums of squares.
    pub ss_type: Option<u8>,
}

impl Default for StatOptions {
    fn default() -> Self {
        Self {
            selected: Vec::new(),
            show_charts: false,
            // StatsBench shows the frequency table by default.
            display_tables: true,
            save_standardised: false,
            cell_counts: Vec::new(),
            crosstabs_stats: Vec::new(),
            posthoc: Vec::new(),
            effect_sizes: false,
            significance_stars: false,
            pairwise_delete: false,
            // StatsBench uses Type III by default.
            ss_type: Some(3),
        }
    }
}

impl StatOptions {
    fn has(&self, id: &str) -> bool {
        self.selected.iter().any(|s| *s == id)
    }

    fn toggle(&mut self, id: &'static str) {
        if let Some(i) = self.selected.iter().position(|s| *s == id) {
            self.selected.remove(i);
        } else {
            self.selected.push(id);
        }
    }
}

/// The state of one open procedure dialog.
pub struct DialogState {
    pub procedure: Procedure,
    /// Variables in the source list that the user has selected.
    pub selected_source: Option<String>,
    /// Variables moved into the target list, in order.
    pub targets: Vec<String>,
    /// The grouping variable (for t-test / ANOVA / means / crosstabs).
    pub group_var: Option<String>,
    /// The names of the target variables that are numeric. Descriptive
    /// statistics only apply to those.
    pub numeric_targets: Vec<String>,
    /// The number of components to keep, for the factor analysis.
    pub n_components: Option<usize>,
    /// Hypothesised mean, or hypothesised proportion.
    pub test_value: f64,
    /// Numeric code counted as a success in a one-sample proportion.
    pub success_value: f64,
    pub options: StatOptions,
    pub sub: SubDialog,
    /// Set when the user pressed Paste — the generated syntax goes to the
    /// Syntax Editor instead of running.
    pub pasted: Option<String>,
    /// Set when the user pressed OK — the generated syntax should run.
    pub confirmed: Option<String>,
    /// Set when the dialog was dismissed (X or Cancel).
    pub cancelled: bool,
}

impl DialogState {
    pub fn new(procedure: Procedure) -> Self {
        Self {
            procedure,
            selected_source: None,
            targets: Vec::new(),
            group_var: None,
            numeric_targets: Vec::new(),
            n_components: None,
            test_value: if procedure == Procedure::Proportion {
                0.5
            } else {
                0.0
            },
            success_value: 1.0,
            options: StatOptions::default(),
            sub: SubDialog::None,
            pasted: None,
            confirmed: None,
            cancelled: false,
        }
    }

    /// StatsBench disables OK until the dialog has everything it needs.
    pub fn ok_enabled(&self) -> bool {
        match self.procedure {
            // The paired and Friedman tests compare several variables.
            Procedure::Wilcoxon => self.targets.len() >= 2,
            Procedure::Friedman => self.targets.len() >= 3,
            _ if self.procedure.needs_group() => {
                self.targets.len() >= 1 && self.group_var.is_some()
            }
            _ => !self.targets.is_empty(),
        }
    }

    /// Build the StatsBench syntax for the current state, or `None` if the
    /// dialog is not yet complete.
    pub fn build_syntax(&self) -> Option<String> {
        if self.targets.is_empty() {
            return None;
        }
        let vars = self.targets.join(" ");
        let mut parts: Vec<String> = Vec::new();

        match self.procedure {
            Procedure::Frequencies => {
                let mut p = format!("FREQUENCIES {}", vars);
                if !self.options.display_tables {
                    p.push_str(" /FORMAT=NOTABLE");
                }
                let stats = self.statistics_clause();
                if !stats.is_empty() {
                    p.push_str(" /STATISTICS=");
                    p.push_str(&stats);
                }
                if self.options.show_charts {
                    p.push_str(" /CHART=BARCHART");
                }
                parts.push(p);
            }
            Procedure::Descriptives => {
                let mut p = format!("DESCRIPTIVES {}", vars);
                let stats = self.statistics_clause();
                if !stats.is_empty() {
                    p.push_str(" /STATISTICS=");
                    p.push_str(&stats);
                }
                if self.options.save_standardised {
                    p.push_str(" /SAVE=ZSTD");
                }
                parts.push(p);
            }
            Procedure::Crosstabs => {
                let g = self.group_var.clone()?;
                let mut p = format!("CROSSTABS {} BY {}", vars, g);
                if !self.options.cell_counts.is_empty() {
                    p.push_str(" /CELLS=");
                    p.push_str(&self.options.cell_counts.join(" "));
                }
                if !self.options.crosstabs_stats.is_empty() {
                    p.push_str(" /STATISTICS=");
                    p.push_str(&self.options.crosstabs_stats.join(" "));
                }
                parts.push(p);
            }
            Procedure::TTest => {
                let g = self.group_var.clone()?;
                let mut p = format!("T-TEST {} BY {}", vars, g);
                if self.options.effect_sizes {
                    p.push_str(" /ES=DISPLAY");
                }
                parts.push(p);
            }
            Procedure::PairedT => {
                if self.targets.len() < 2 {
                    return None;
                }
                parts.push(format!("T-TEST PAIRED {}", self.targets.join(" ")));
            }
            Procedure::Anova => {
                let g = self.group_var.clone()?;
                let mut p = format!("ONEWAY {} BY {}", vars, g);
                if !self.options.posthoc.is_empty() {
                    p.push_str(" /POSTHOC=");
                    p.push_str(&self.options.posthoc.join(" "));
                }
                parts.push(p);
            }
            Procedure::Correlations => {
                let mut p = format!("CORRELATIONS {}", vars);
                if self.options.significance_stars {
                    p.push_str(" /PRINT=SIG");
                }
                if self.options.pairwise_delete {
                    p.push_str(" /MISSING=PAIRWISE");
                }
                parts.push(p);
            }
            Procedure::Regression => {
                if self.targets.len() < 2 {
                    return None;
                }
                parts.push(format!(
                    "REGRESSION {} WITH {}",
                    self.targets[0],
                    self.targets[1..].join(" ")
                ));
            }
            Procedure::Means => {
                let g = self.group_var.clone()?;
                parts.push(format!("MEANS TABLES {} BY {}", vars, g));
            }
            Procedure::Explore => {
                parts.push(format!("EXPLORE {}", vars));
            }
            Procedure::MannWhitney => {
                let g = self.group_var.clone()?;
                parts.push(format!("NONPARAMETRIC /MW= {} BY {}", vars, g));
            }
            Procedure::Wilcoxon => {
                if self.targets.len() < 2 {
                    return None;
                }
                parts.push(format!(
                    "NONPARAMETRIC /WILCOXON= {} {}",
                    self.targets[0],
                    self.targets[1]
                ));
            }
            Procedure::KruskalWallis => {
                let g = self.group_var.clone()?;
                parts.push(format!("NONPARAMETRIC /K-W= {} BY {}", vars, g));
            }
            Procedure::Friedman => {
                if self.targets.len() < 3 {
                    return None;
                }
                parts.push(format!(
                    "NONPARAMETRIC /FRIEDMAN= {}",
                    self.targets.join(" ")
                ));
            }
            Procedure::Glm => {
                // GLM <dependent> BY <factor> [<factor> ...]
                //     /METHOD=SSTYPE(3)
                let g = self.group_var.clone()?;
                let mut p = format!("GLM {} BY {}", vars, g);
                if let Some(ss) = self.options.ss_type {
                    p.push_str(&format!(" /METHOD=SSTYPE({})", ss));
                }
                parts.push(p);
            }
            Procedure::OneSampleT => {
                parts.push(format!(
                    "T-TEST ONESAMPLE {} /TESTVAL={}",
                    vars, self.test_value
                ));
            }
            Procedure::Proportion => {
                parts.push(format!(
                    "PROPORTION {} /TESTVAL={} /SUCCESS={}",
                    vars, self.test_value, self.success_value
                ));
            }
            Procedure::Reliability => {
                if self.targets.len() < 2 {
                    return None;
                }
                parts.push(format!("RELIABILITY {}", self.targets.join(" ")));
            }
            Procedure::Factor => {
                if self.targets.len() < 3 {
                    return None;
                }
                let n = self.n_components.unwrap_or(1).max(1);
                parts.push(format!(
                    "FACTOR {} /EXTRACTION=PC{}",
                    self.targets.join(" "),
                    n
                ));
            }
            Procedure::Roc => {
                let g = self.group_var.clone()?;
                parts.push(format!("ROC {} BY {}", vars, g));
            }
        }

        Some(parts.join("\n"))
    }

    /// Recompute which of the target variables are numeric.
    pub fn recompute_numeric_targets(&mut self, vars: &[(String, ColumnKind)]) {
        self.numeric_targets = self
            .targets
            .iter()
            .filter(|t| {
                vars.iter()
                    .any(|(n, k)| n == *t && *k == ColumnKind::Numeric)
            })
            .cloned()
            .collect();
    }

    /// The `STATISTICS=` keyword list for the selected statistics.
    fn statistics_clause(&self) -> String {
        if self.options.selected.is_empty() {
            return String::new();
        }
        self.options
            .selected
            .iter()
            .map(|s| s.to_uppercase())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// The statistics each procedure offers, in the order StatsBench lists them.
pub fn statistics_choices(procedure: Procedure) -> &'static [(&'static str, &'static str)] {
    match procedure {
        Procedure::Frequencies | Procedure::Descriptives => &[
            ("MEAN", "Mean"),
            ("MEDIAN", "Median"),
            ("MODE", "Mode"),
            ("STDDEV", "Std. Deviation"),
            ("VARIANCE", "Variance"),
            ("MINIMUM", "Minimum"),
            ("MAXIMUM", "Maximum"),
            ("RANGE", "Range"),
            ("SKEWNESS", "Skewness"),
            ("KURTOSIS", "Kurtosis"),
            ("SUM", "Sum"),
            ("SEKURT", "Std. Error of Kurtosis"),
        ],
        Procedure::Explore => &[
            ("MEAN", "Mean"),
            ("MEDIAN", "Median"),
            ("MODE", "Mode"),
            ("STDDEV", "Std. Deviation"),
            ("VARIANCE", "Variance"),
            ("MINIMUM", "Minimum"),
            ("MAXIMUM", "Maximum"),
            ("RANGE", "Range"),
            ("SKEWNESS", "Skewness"),
            ("KURTOSIS", "Kurtosis"),
            ("IQR", "Interquartile Range"),
            ("PERCENTILES", "Percentiles"),
        ],
        Procedure::Correlations => &[
            ("MEAN", "Means and Std. Dev"),
            ("XPROD", "Cross-Correlations"),
        ],
        _ => &[],
    }
}

/// The post-hoc tests ANOVA offers, in StatsBench's order.
pub const POSTHOC_CHOICES: &[(&str, &str)] = &[
    ("SCHEFFE", "Scheffe"),
    ("TUKEY", "Tukey"),
    ("BONFERRONI", "Bonferroni"),
    ("LSD", "Least Significant Difference"),
    ("DUNCAN", "Duncan"),
    ("SNK", "Student-Newman-Keuls"),
    ("GT2", "Gabriel-Tokey-Hochberg GT2"),
    ("DUNNETT", "Dunnett's t2"),
];

/// The cell counts Crosstabs offers.
pub const CELL_COUNT_CHOICES: &[(&str, &str)] = &[
    ("COUNT", "Observed counts"),
    ("EXPECTED", "Expected counts"),
    ("ROW", "Row percentages"),
    ("COLUMN", "Column percentages"),
    ("TOTAL", "Total percentages"),
];

/// The statistics Crosstabs offers.
pub const CROSSTABS_STAT_CHOICES: &[(&str, &str)] = &[
    ("CHISQ", "Chi-square"),
    ("PHI", "Phi"),
    ("CC", "Continuity correction"),
    ("LAMBDA", "Lambda"),
    ("RISK", "Risk"),
    ("KAPPA", "Kappa"),
    ("ETA", "Eta"),
    ("CORR", "Correlation"),
];

/// Draw the dialog window. Returns the generated syntax if the user
/// confirmed, or the syntax to paste.
pub fn show(ctx: &egui::Context, app: &mut StatsCraftApp, d: &mut DialogState) {
    let mut open = true;
    let mut close_requested = false;

    let title = format!("{} - StatsCraft", d.procedure.title());
    egui::Window::new(title)
        .open(&mut open)
        .resizable(true)
        .default_width(815.0)
        .default_height(556.0)
        .min_width(560.0)
        .min_height(360.0)
        .show(ctx, |ui| {
            render(ui, app, d, &mut close_requested);
        });

    // OK, Paste and Cancel all ask for the window to close.
    if close_requested {
        open = false;
    }

    // The window can close in two ways: the user pressed OK or Paste (a
    // decision), or they closed the window with the X or Cancel (no
    // decision). Only the latter clears the result.
    if !open && d.confirmed.is_none() && d.pasted.is_none() {
        d.cancelled = true;
    }
}

/// The dialog body. Split out so tests can drive it without a window.
pub fn render(
    ui: &mut egui::Ui,
    app: &mut StatsCraftApp,
    d: &mut DialogState,
    close_requested: &mut bool,
) {
    let vars: Vec<(String, ColumnKind)> = app
        .table
        .as_ref()
        .map(|t| {
            t.columns
                .iter()
                .map(|c| (c.name.clone(), c.kind))
                .collect()
        })
        .unwrap_or_default();

    if vars.is_empty() {
        ui_placeholder(ui);
        return;
    }

    ui.columns(3, |cols| {
        // ---- Source variable list ----
        cols[0].set_min_width(170.0);
        cols[0].group(|ui| {
            ui.label(
                egui::RichText::new("Source Variable(s)")
                    .font(theme::semibold(11.0))
                    .color(theme::text_muted()),
            );
            ui.add_space(2.0);
            let selected_source = d.selected_source.clone();
            let mut list = UiList::new(
                ui,
                &vars,
                selected_source.as_deref(),
                d.procedure.numeric_only(),
                true,
            );
            list.show();
            if let Some(sel) = list.take_picked() {
                d.selected_source = Some(sel);
            }
            // Double-clicking a source variable moves it across, the way
            // StatsBench's variable lists do.
            if let Some(sel) = list.take_activated() {
                if !d.targets.iter().any(|t| *t == sel) {
                    d.targets.push(sel);
                }
                d.recompute_numeric_targets(&vars);
            }
        });

        // ---- Move arrow ----
        cols[1].set_min_width(56.0);
        cols[1].group(|ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(6.0);
                let can_move = d
                    .selected_source
                    .as_ref()
                    .map(|s| !d.targets.iter().any(|t| t == s))
                    .unwrap_or(false);
                let resp = ui.add_enabled(
                    can_move,
                    egui::Button::new(
                        egui::RichText::new("→").font(theme::semibold(18.0)),
                    )
                    .min_size(egui::vec2(38.0, 30.0)),
                );
                if resp.clicked() {
                    if let Some(s) = d.selected_source.clone() {
                        if !d.targets.iter().any(|t| *t == s) {
                            d.targets.push(s);
                        }
                        d.recompute_numeric_targets(&vars);
                    }
                }
                ui.add_space(6.0);
                let can_remove = !d.targets.is_empty();
                let resp = ui.add_enabled(
                    can_remove,
                    egui::Button::new(
                        egui::RichText::new("←").font(theme::semibold(18.0)),
                    )
                    .min_size(egui::vec2(38.0, 30.0)),
                );
                if resp.clicked() {
                    d.targets.pop();
                    d.recompute_numeric_targets(&vars);
                }
                ui.add_space(14.0);
                // The grouping variable, for procedures that need one.
                if d.procedure.needs_group() {
                    ui.label(
                        egui::RichText::new("Grouping")
                            .font(theme::semibold(10.5))
                            .color(theme::text_muted()),
                    );
                    ui.add_space(2.0);
                    // The grouping variable is chosen from the string
                    // columns — a factor is a categorical variable.
                    let mut group_choice = d.group_var.clone().unwrap_or_default();
                    egui::ComboBox::from_label("")
                        .selected_text(if group_choice.is_empty() {
                            "—".to_string()
                        } else {
                            group_choice.clone()
                        })
                        .width(120.0)
                        .show_ui(ui, |ui| {
                            for (name, kind) in &vars {
                                if *kind == ColumnKind::Numeric {
                                    continue;
                                }
                                ui.selectable_value(
                                    &mut group_choice,
                                    name.clone(),
                                    name.as_str(),
                                );
                            }
                        });
                    if !group_choice.is_empty() {
                        d.group_var = Some(group_choice);
                    }
                }
            });
        });

        // ---- Target list + options + buttons ----
        cols[2].set_min_width(300.0);
        cols[2].group(|ui| {
            ui.label(
                egui::RichText::new("Variable(s)")
                    .font(theme::semibold(11.0))
                    .color(theme::text_muted()),
            );
            ui.add_space(2.0);
            // Show the variables that are in the target list.
            let target_rows: Vec<(String, ColumnKind)> = d
                .targets
                .iter()
                .filter_map(|name| {
                    vars.iter()
                        .find(|(n, _)| n == name)
                        .cloned()
                })
                .collect();
            // The target list is a drop zone for variables dragged out of
            // the source list. Its frame changes while a compatible drag
            // is over it.
            let mut picked_target = None;
            let mut activated_target = None;
            let mut dropped_target = None;
            let (_, payload) = ui.dnd_drop_zone::<String, _>(egui::Frame::none(), |ui| {
                let mut tlist = UiList::new(ui, &target_rows, None, false, false);
                tlist.show();
                picked_target = tlist.take_picked();
                activated_target = tlist.take_activated();
            });
            if let Some(name) = payload {
                dropped_target = Some((*name).clone());
            }
            // Clicking or double-clicking a target removes it (StatsBench's
            // behaviour).
            if let Some(sel) = picked_target.or(activated_target) {
                d.targets.retain(|t| *t != sel);
                d.recompute_numeric_targets(&vars);
            }
            // A drop onto the target list moves the variable in.
            if let Some(sel) = dropped_target {
                if !d.targets.iter().any(|t| *t == sel) {
                    d.targets.push(sel);
                }
                d.recompute_numeric_targets(&vars);
            }

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(6.0);

            // ---- Sub-dialog buttons (right-hand column, as in StatsBench) ----
            ui.horizontal_wrapped(|ui| {
                match d.procedure {
                    Procedure::Frequencies => {
                        sub_button(ui, d, SubDialog::FrequenciesStatistics, "Statistics…");
                        sub_button(ui, d, SubDialog::FrequenciesCharts, "Charts…");
                        sub_button(ui, d, SubDialog::FrequenciesFormat, "Format…");
                    }
                    Procedure::Descriptives => {
                        sub_button(ui, d, SubDialog::DescriptivesOptions, "Options…");
                    }
                    Procedure::Crosstabs => {
                        sub_button(ui, d, SubDialog::CrosstabsCells, "Cells…");
                        sub_button(ui, d, SubDialog::CrosstabsStatistics, "Statistics…");
                    }
                    Procedure::TTest => {
                        sub_button(ui, d, SubDialog::TTestOptions, "Options…");
                    }
                    Procedure::Anova => {
                        sub_button(ui, d, SubDialog::AnovaOptions, "Options…");
                        sub_button(ui, d, SubDialog::AnovaPostHoc, "Post-hoc…");
                    }
                    Procedure::Correlations => {
                        sub_button(ui, d, SubDialog::CorrelationsOptions, "Options…");
                    }
                    Procedure::Regression => {
                        sub_button(ui, d, SubDialog::RegressionStatistics, "Statistics…");
                    }
                    Procedure::Means => {
                        sub_button(ui, d, SubDialog::MeansOptions, "Options…");
                    }
                    Procedure::Explore => {
                        sub_button(ui, d, SubDialog::ExploreOptions, "Options…");
                    }
                    Procedure::Factor => {
                        sub_button(ui, d, SubDialog::FactorOptions, "Options…");
                    }
                    Procedure::Roc => {
                        sub_button(ui, d, SubDialog::RocOptions, "Options…");
                    }
                    Procedure::Glm => {
                        sub_button(ui, d, SubDialog::GlmOptions, "Model…");
                    }
                    // The nonparametric tests have no options of their own.
                    Procedure::PairedT
                    | Procedure::MannWhitney
                    | Procedure::Wilcoxon
                    | Procedure::KruskalWallis
                    | Procedure::Friedman
                    | Procedure::Reliability
                    | Procedure::OneSampleT
                    | Procedure::Proportion => {}
                }
            });

            ui.add_space(6.0);
            // The active sub-dialog, rendered inline below the buttons.
            if d.sub != SubDialog::None {
                ui.separator();
                ui.add_space(4.0);
                render_sub_dialog(ui, d);
                ui.add_space(4.0);
            }

            ui.add_space(4.0);
            // ---- Procedure-specific checkboxes ----
            match d.procedure {
                Procedure::Frequencies => {
                    ui.checkbox(
                        &mut d.options.display_tables,
                        egui::RichText::new("Display frequency tables").size(12.0),
                    );
                    ui.checkbox(
                        &mut d.options.show_charts,
                        egui::RichText::new("Create charts (bar chart)").size(12.0),
                    );
                }
                Procedure::Descriptives => {
                    ui.checkbox(
                        &mut d.options.save_standardised,
                        egui::RichText::new("Save standardized values as variables").size(12.0),
                    );
                }
                Procedure::OneSampleT => {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Test value:").size(12.0));
                        ui.add(egui::DragValue::new(&mut d.test_value).speed(0.1));
                    });
                }
                Procedure::Proportion => {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Test proportion:").size(12.0));
                        ui.add(egui::DragValue::new(&mut d.test_value).speed(0.01));
                        d.test_value = d.test_value.clamp(0.0, 1.0);
                        ui.label(egui::RichText::new("Success code:").size(12.0));
                        ui.add(egui::DragValue::new(&mut d.success_value).speed(1.0));
                    });
                }
                Procedure::Correlations => {
                    ui.checkbox(
                        &mut d.options.significance_stars,
                        egui::RichText::new("Mark significance with stars").size(12.0),
                    );
                    ui.checkbox(
                        &mut d.options.pairwise_delete,
                        egui::RichText::new("Delete pairwise instead of listwise").size(12.0),
                    );
                }
                _ => {}
            }
        });
    });

    ui.add_space(8.0);
    ui.separator();
    ui.add_space(6.0);

    // ---- Button row: OK, Paste, Reset, Cancel, Help ----
    ui.horizontal_wrapped(|ui| {
        let ok = ui.add_enabled(
            d.ok_enabled(),
            egui::Button::new(
                egui::RichText::new("OK")
                    .font(theme::semibold(12.0))
                    .color(theme::text_on_accent()),
            )
            .min_size(egui::vec2(74.0, 28.0))
            .fill(theme::accent()),
        );
        if ok.clicked() {
            if let Some(s) = d.build_syntax() {
                d.confirmed = Some(s);
                *close_requested = true;
            }
        }

        let paste = ui.add_enabled(
            d.ok_enabled(),
            egui::Button::new(egui::RichText::new("Paste").size(12.0))
                .min_size(egui::vec2(74.0, 28.0)),
        );
        if paste.clicked() {
            if let Some(s) = d.build_syntax() {
                d.pasted = Some(s);
                *close_requested = true;
            }
        }

        if ui
            .add(
                egui::Button::new(egui::RichText::new("Reset").size(12.0))
                    .min_size(egui::vec2(74.0, 28.0)),
            )
            .clicked()
        {
            *d = DialogState::new(d.procedure);
        }

        if ui
            .add(
                egui::Button::new(egui::RichText::new("Cancel").size(12.0))
                    .min_size(egui::vec2(74.0, 28.0)),
            )
            .clicked()
        {
            *close_requested = true;
        }

        if ui
            .add(
                egui::Button::new(egui::RichText::new("Help").size(12.0))
                    .min_size(egui::vec2(74.0, 28.0)),
            )
            .clicked()
        {
            app.status = format!(
                "{}: see docs/REFERENCE.md for the full command grammar.",
                d.procedure.title()
            );
        }
    });
}

/// Shown when there is no data to analyse.
fn ui_placeholder(ui: &mut egui::Ui) {
    ui.label(
        egui::RichText::new("No data loaded — open a data file first.")
            .color(theme::text_muted()),
    );
}

/// Draw one of the right-hand sub-dialog buttons.
fn sub_button(ui: &mut egui::Ui, d: &mut DialogState, which: SubDialog, label: &str) {
    let active = d.sub == which;
    let btn = egui::Button::new(
        egui::RichText::new(label)
            .size(11.5)
            .color(if active { theme::accent() } else { theme::text() }),
    )
    .min_size(egui::vec2(0.0, 26.0));
    if ui.add(btn).clicked() {
        d.sub = if active { SubDialog::None } else { which };
    }
}

/// Render the active sub-dialog's contents.
fn render_sub_dialog(ui: &mut egui::Ui, d: &mut DialogState) {
    egui::Frame::group(ui.style())
        .inner_margin(egui::Margin {
            left: 8.0,
            right: 8.0,
            top: 6.0,
            bottom: 6.0,
        })
        .show(ui, |ui| {
            match d.sub {
                SubDialog::None => {}
                SubDialog::FrequenciesStatistics
                | SubDialog::DescriptivesOptions
                | SubDialog::ExploreOptions => {
                    // Descriptive statistics only apply to numeric
                    // variables, so say so rather than offering a list
                    // that cannot be used.
                    if !d
                        .targets
                        .iter()
                        .any(|t| d.numeric_targets.contains(t))
                    {
                        ui.label(
                            egui::RichText::new(
                                "Descriptive statistics need a numeric variable. Select one in the source list.",
                            )
                            .size(11.5)
                            .color(theme::text_faint()),
                        );
                        return;
                    }
                    let choices = statistics_choices(d.procedure);
                    ui.label(
                        egui::RichText::new("Statistics").font(theme::semibold(11.0)),
                    );
                    ui.add_space(3.0);
                    if choices.is_empty() {
                        ui.label(egui::RichText::new("No extra options.").size(11.5));
                    } else {
                        egui::ScrollArea::vertical()
                            .max_height(120.0)
                            .show(ui, |ui| {
                                for (id, label) in choices {
                                    let mut on = d.options.has(id);
                                    if ui
                                        .checkbox(&mut on, egui::RichText::new(*label).size(11.5))
                                        .changed()
                                    {
                                        d.options.toggle(id);
                                    }
                                }
                            });
                    }
                }
                SubDialog::FrequenciesCharts => {
                    ui.label(egui::RichText::new("Charts").font(theme::semibold(11.0)));
                    ui.add_space(3.0);
                    ui.checkbox(
                        &mut d.options.show_charts,
                        egui::RichText::new("Bar chart of the counts").size(11.5),
                    );
                    ui.label(
                        egui::RichText::new("Charts are drawn in the Output Viewer.")
                            .size(10.5)
                            .color(theme::text_faint()),
                    );
                }
                SubDialog::FrequenciesFormat => {
                    ui.label(egui::RichText::new("Format").font(theme::semibold(11.0)));
                    ui.add_space(3.0);
                    ui.checkbox(
                        &mut d.options.display_tables,
                        egui::RichText::new("Display frequency tables").size(11.5),
                    );
                }
                SubDialog::CrosstabsCells => {
                    ui.label(egui::RichText::new("Cells").font(theme::semibold(11.0)));
                    ui.add_space(3.0);
                    for (id, label) in CELL_COUNT_CHOICES {
                        let mut on = d.options.cell_counts.contains(id);
                        if ui
                            .checkbox(&mut on, egui::RichText::new(*label).size(11.5))
                            .changed()
                        {
                            if on {
                                d.options.cell_counts.push(id);
                            } else {
                                d.options.cell_counts.retain(|c| *c != *id);
                            }
                        }
                    }
                }
                SubDialog::CrosstabsStatistics => {
                    ui.label(egui::RichText::new("Statistics").font(theme::semibold(11.0)));
                    ui.add_space(3.0);
                    for (id, label) in CROSSTABS_STAT_CHOICES {
                        let mut on = d.options.crosstabs_stats.contains(id);
                        if ui
                            .checkbox(&mut on, egui::RichText::new(*label).size(11.5))
                            .changed()
                        {
                            if on {
                                d.options.crosstabs_stats.push(id);
                            } else {
                                d.options.crosstabs_stats.retain(|c| *c != *id);
                            }
                        }
                    }
                }
                SubDialog::TTestOptions => {
                    ui.label(egui::RichText::new("Options").font(theme::semibold(11.0)));
                    ui.add_space(3.0);
                    ui.checkbox(
                        &mut d.options.effect_sizes,
                        egui::RichText::new("Effect size (Cohen's d)").size(11.5),
                    );
                }
                SubDialog::AnovaOptions => {
                    ui.label(egui::RichText::new("Options").font(theme::semibold(11.0)));
                    ui.add_space(3.0);
                    ui.label(
                        egui::RichText::new("Homogeneity of variance is tested (Levene's).")
                            .size(11.5),
                    );
                }
                SubDialog::AnovaPostHoc => {
                    ui.label(egui::RichText::new("Post-hoc Tests").font(theme::semibold(11.0)));
                    ui.add_space(3.0);
                    egui::ScrollArea::vertical()
                        .max_height(110.0)
                        .show(ui, |ui| {
                            for (id, label) in POSTHOC_CHOICES {
                                let mut on = d.options.posthoc.contains(id);
                                if ui
                                    .checkbox(&mut on, egui::RichText::new(*label).size(11.5))
                                    .changed()
                                {
                                    if on {
                                        d.options.posthoc.push(id);
                                    } else {
                                        d.options.posthoc.retain(|c| *c != *id);
                                    }
                                }
                            }
                        });
                }
                SubDialog::CorrelationsOptions => {
                    ui.label(egui::RichText::new("Options").font(theme::semibold(11.0)));
                    ui.add_space(3.0);
                    ui.checkbox(
                        &mut d.options.significance_stars,
                        egui::RichText::new("Mark significance with stars").size(11.5),
                    );
                    ui.checkbox(
                        &mut d.options.pairwise_delete,
                        egui::RichText::new("Delete pairwise instead of listwise").size(11.5),
                    );
                }
                SubDialog::RegressionStatistics => {
                    ui.label(egui::RichText::new("Statistics").font(theme::semibold(11.0)));
                    ui.add_space(3.0);
                    ui.label(
                        egui::RichText::new("Coefficients, ANOVA table and R² are reported.")
                            .size(11.5),
                    );
                }
                SubDialog::FactorOptions => {
                    ui.label(egui::RichText::new("Extraction").font(theme::semibold(11.0)));
                    ui.add_space(3.0);
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Number of components:").size(11.5));
                        let mut n = d.n_components.unwrap_or(1) as i64;
                        ui.add(
                            egui::DragValue::new(&mut n)
                                .range(1..=10)
                                .speed(0.2),
                        );
                        d.n_components = Some(n.max(1) as usize);
                    });
                    ui.label(
                        egui::RichText::new("Principal components, rotated with varimax.")
                            .size(10.5)
                            .color(theme::text_faint()),
                    );
                }
                SubDialog::RocOptions => {
                    ui.label(egui::RichText::new("Options").font(theme::semibold(11.0)));
                    ui.add_space(3.0);
                    ui.label(
                        egui::RichText::new(
                            "The area under the curve and the 95% confidence interval are always reported.",
                        )
                        .size(11.5),
                    );
                }
                SubDialog::MeansOptions => {
                    ui.label(egui::RichText::new("Options").font(theme::semibold(11.0)));
                    ui.add_space(3.0);
                    ui.label(
                        egui::RichText::new("Group means with counts and standard deviations.")
                            .size(11.5),
                    );
                }
                SubDialog::GlmOptions => {
                    ui.label(egui::RichText::new("Model").font(theme::semibold(11.0)));
                    ui.add_space(3.0);
                    ui.label(
                        egui::RichText::new("The type of the sums of squares:").size(11.5),
                    );
                    ui.add_space(3.0);
                    // The three radio buttons StatsBench offers.
                    for (n, label, help) in [
                        (1u8, "Type I", "Sequential: each factor is tested after the ones before it."),
                        (2u8, "Type II", "Each factor is tested after the others, except the interactions that contain it."),
                        (3u8, "Type III", "Each factor is tested after all the others. The default."),
                    ] {
                        let mut current = d.options.ss_type.unwrap_or(3);
                        let selected = current == n;
                        if ui.radio_value(&mut current, n, label).clicked() {
                            d.options.ss_type = Some(n);
                        }
                        if selected {
                            ui.label(
                                egui::RichText::new(help).size(10.5).color(theme::text_faint()),
                            );
                        }
                    }
                }
            }
        });
}

/// A simple scrollable list of variable names with a selection highlight.
struct UiList<'a> {
    ui: &'a mut egui::Ui,
    vars: &'a [(String, ColumnKind)],
    selected: Option<&'a str>,
    numeric_only: bool,
    /// Whether the rows can be dragged into a target list.
    draggable: bool,
    /// The variable the user just clicked, reported back to the caller.
    picked: Option<String>,
    /// The variable the user double-clicked, reported back separately.
    activated: Option<String>,
}

impl<'a> UiList<'a> {
    fn new(
        ui: &'a mut egui::Ui,
        vars: &'a [(String, ColumnKind)],
        selected: Option<&'a str>,
        numeric_only: bool,
        draggable: bool,
    ) -> Self {
        Self {
            ui,
            vars,
            selected,
            numeric_only,
            draggable,
            picked: None,
            activated: None,
        }
    }

    fn show(&mut self) {
        let mut picked = None;
        let mut activated = None;
        egui::ScrollArea::vertical()
            .max_height(210.0)
            .auto_shrink([false, false])
            .show(self.ui, |ui| {
                for (name, kind) in self.vars {
                    let dim = self.numeric_only && *kind != ColumnKind::Numeric;
                    let is_sel = self.selected == Some(name.as_str());
                    let resp = if self.draggable && !dim {
                        // The whole row is the drag handle. egui paints the
                        // row under the pointer while it is dragged.
                        let id = ui.make_persistent_id(("variable-drag", name));
                        ui.dnd_drag_source(id, name.clone(), |ui| {
                            variable_row(ui, name, *kind, is_sel, dim)
                        })
                        .response
                    } else {
                        variable_row(ui, name, *kind, is_sel, dim)
                    }
                    .on_hover_cursor(egui::CursorIcon::PointingHand);
                    if resp.double_clicked() {
                        activated = Some(name.clone());
                    } else if resp.clicked() {
                        picked = Some(name.clone());
                    }
                }
            });
        self.picked = picked;
        self.activated = activated;
    }

    /// The variable clicked since the last call.
    fn take_picked(&mut self) -> Option<String> {
        self.picked.take()
    }

    /// The variable double-clicked since the last call.
    fn take_activated(&mut self) -> Option<String> {
        self.activated.take()
    }
}

/// One row of a variable list.
///
/// The interaction rect is the whole row, not just the text, so a click
/// anywhere across the list selects the variable. This is the hit area
/// StatsBench gives its variable lists.
fn variable_row(
    ui: &mut egui::Ui,
    name: &str,
    kind: ColumnKind,
    is_sel: bool,
    dim: bool,
) -> egui::Response {
    let text_col = if dim {
        theme::text_faint()
    } else if is_sel {
        theme::accent()
    } else {
        theme::text()
    };
    let galley = ui
        .painter()
        .layout_no_wrap(name.to_string(), theme::medium(12.0), text_col);
    let width = ui.available_width().max(120.0);
    let (rect, resp) = ui.allocate_exact_size(
        egui::vec2(width, 28.0),
        egui::Sense::click_and_drag(),
    );
    let p = ui.painter();
    if is_sel {
        p.rect_filled(rect, theme::RADIUS_SM, theme::accent_soft());
    } else if resp.hovered() {
        p.rect_filled(rect, theme::RADIUS_SM, theme::surface_alt());
    }
    // The measure-level icon StatsBench puts beside each variable: a ruler for
    // numeric/scale, a pencil for string/nominal.
    let icon = if kind == ColumnKind::Numeric {
        Icon::MeasureScale
    } else {
        Icon::MeasureNominal
    };
    let icon_rect = egui::Rect::from_min_size(
        egui::pos2(rect.left() + 4.0, rect.center().y - 6.0),
        egui::vec2(12.0, 12.0),
    );
    crate::icon::paint_icon(p, ui.ctx(), icon, icon_rect, text_col);
    p.galley(
        egui::pos2(
            rect.left() + 20.0,
            rect.center().y - galley.size().y / 2.0,
        ),
        galley,
        text_col,
    );
    resp
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(procedure: Procedure) -> DialogState {
        DialogState::new(procedure)
    }

    #[test]
    fn ok_is_disabled_until_a_variable_is_chosen() {
        let mut s = d(Procedure::Frequencies);
        assert!(!s.ok_enabled());
        s.targets.push("score".into());
        assert!(s.ok_enabled());
    }

    #[test]
    fn a_grouping_variable_is_required_where_needed() {
        let mut s = d(Procedure::TTest);
        s.targets.push("score".into());
        assert!(!s.ok_enabled(), "t-test needs a grouping variable");
        s.group_var = Some("group".into());
        assert!(s.ok_enabled());
    }

    #[test]
    fn frequencies_builds_the_expected_syntax() {
        let mut s = d(Procedure::Frequencies);
        s.targets.push("region".into());
        s.targets.push("group".into());
        let syn = s.build_syntax().expect("syntax");
        assert_eq!(syn, "FREQUENCIES region group");
    }

    #[test]
    fn the_statistics_sub_dialog_changes_the_generated_syntax() {
        let mut s = d(Procedure::Descriptives);
        s.targets.push("score".into());
        s.options.selected = vec!["MEAN", "MEDIAN", "STDDEV"];
        let syn = s.build_syntax().expect("syntax");
        assert_eq!(syn, "DESCRIPTIVES score /STATISTICS=MEAN MEDIAN STDDEV");
    }

    #[test]
    fn crosstabs_needs_both_a_row_and_a_column_variable() {
        let mut s = d(Procedure::Crosstabs);
        s.targets.push("region".into());
        assert!(s.build_syntax().is_none());
        s.group_var = Some("group".into());
        let syn = s.build_syntax().expect("syntax");
        assert_eq!(syn, "CROSSTABS region BY group");
    }

    #[test]
    fn the_t_test_generates_the_independent_samples_form() {
        let mut s = d(Procedure::TTest);
        s.targets.push("score".into());
        s.group_var = Some("group".into());
        assert_eq!(
            s.build_syntax().expect("syntax"),
            "T-TEST score BY group"
        );
    }

    #[test]
    fn the_paired_t_test_needs_two_variables() {
        let mut s = d(Procedure::PairedT);
        s.targets.push("score".into());
        assert!(s.build_syntax().is_none());
        s.targets.push("age".into());
        assert_eq!(
            s.build_syntax().expect("syntax"),
            "T-TEST PAIRED score age"
        );
    }

    #[test]
    fn regression_puts_the_first_variable_as_the_outcome() {
        let mut s = d(Procedure::Regression);
        s.targets.push("score".into());
        s.targets.push("age".into());
        assert_eq!(
            s.build_syntax().expect("syntax"),
            "REGRESSION score WITH age"
        );
    }

    #[test]
    fn post_hoc_tests_are_appended_to_the_anova_command() {
        let mut s = d(Procedure::Anova);
        s.targets.push("score".into());
        s.group_var = Some("group".into());
        s.options.posthoc = vec!["TUKEY", "SCHEFFE"];
        let syn = s.build_syntax().expect("syntax");
        assert_eq!(syn, "ONEWAY score BY group /POSTHOC=TUKEY SCHEFFE");
    }

    #[test]
    fn the_correlation_options_reach_the_syntax() {
        let mut s = d(Procedure::Correlations);
        s.targets.push("score".into());
        s.targets.push("age".into());
        s.options.significance_stars = true;
        s.options.pairwise_delete = true;
        let syn = s.build_syntax().expect("syntax");
        assert_eq!(
            syn,
            "CORRELATIONS score age /PRINT=SIG /MISSING=PAIRWISE"
        );
    }
}

// ---------------------------------------------------------------------------
// The Missing Values dialog
//
// This is not a procedure dialog - it changes a variable's properties
// rather than running an analysis - so it has its own state and its
// own window.
// ---------------------------------------------------------------------------

/// The state of the "Missing Values" dialog.
#[derive(Debug, Clone, Default)]
pub struct MissingValues {
    /// The variable the values are for.
    pub variable: Option<String>,
    /// The discrete values, as typed.
    pub discrete: String,
    /// The ends of the range, as typed.
    pub range_lo: String,
    pub range_hi: String,
    /// Whether the range is used rather than the discrete values.
    pub use_range: bool,
    /// Set when the user pressed OK.
    pub confirmed: Option<(String, Vec<f64>, Option<(f64, f64)>)>,
}

impl MissingValues {
    /// The discrete values, parsed.
    pub fn discrete_values(&self) -> Vec<f64> {
        self.discrete
            .split(|c: char| c == ',' || c.is_whitespace() || c == ';')
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.trim().parse::<f64>().ok())
            .collect()
    }

    /// The range, parsed.
    pub fn range(&self) -> Option<(f64, f64)> {
        let lo = self.range_lo.trim().parse::<f64>().ok()?;
        let hi = self.range_hi.trim().parse::<f64>().ok()?;
        Some((lo, hi))
    }
}

/// Draw the "Missing Values" dialog.
///
/// The window is the same shape as StatsBench's: the variable, the two ways
/// of declaring the values, and the action row.
pub fn show_missing(ctx: &egui::Context, app: &mut StatsCraftApp, mv: &mut MissingValues) {
    let mut open = true;
    let title = "Missing Values - StatsCraft".to_string();
    egui::Window::new(title)
        .open(&mut open)
        .resizable(false)
        .default_width(440.0)
        .default_height(330.0)
        // The window opens clear of the menu bar, the way StatsBench's
        // dialogs do.
        .default_pos(egui::pos2(420.0, 190.0))
        .show(ctx, |ui| {
            render_missing(ui, app, mv);
        });
    if !open && mv.confirmed.is_none() {
        // The X or Cancel: no decision.
    }
}

/// The body of the "Missing Values" dialog, split out so a test can
/// drive it without a window.
pub fn render_missing(ui: &mut egui::Ui, app: &mut StatsCraftApp, mv: &mut MissingValues) {
    use crate::theme;

    // The numeric variables: only a numeric variable can have missing
    // values declared.
    let numeric: Vec<String> = app
        .table
        .as_ref()
        .map(|t| {
            t.columns
                .iter()
                .filter(|c| c.kind == ColumnKind::Numeric)
                .map(|c| c.name.clone())
                .collect()
        })
        .unwrap_or_default();

    if numeric.is_empty() {
        ui.label(
            egui::RichText::new("No numeric variables in the data.")
                .color(theme::text_muted()),
        );
        ui.add_space(8.0);
        if ui.button("Cancel").clicked() {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
        return;
    }

    // The variable.
    ui.label(egui::RichText::new("Variable:").strong());
    if mv.variable.is_none() || !numeric.contains(mv.variable.as_ref().unwrap()) {
        mv.variable = Some(numeric[0].clone());
    }
    egui::ComboBox::from_id_salt("missing_var")
        .selected_text(mv.variable.clone().unwrap_or_default())
        .show_ui(ui, |ui| {
            for name in &numeric {
                let selected = mv.variable.as_deref() == Some(name.as_str());
                if ui.selectable_label(selected, name).clicked() {
                    mv.variable = Some(name.clone());
                }
            }
        });
    ui.add_space(8.0);

    // The two ways of declaring the values.
    ui.horizontal(|ui| {
        ui.selectable_value(&mut mv.use_range, false, "Discrete missing values");
        ui.selectable_value(&mut mv.use_range, true, "Range of missing values");
    });
    ui.add_space(6.0);

    if mv.use_range {
        ui.horizontal(|ui| {
            ui.label("Range:");
            ui.add(egui::TextEdit::singleline(&mut mv.range_lo).desired_width(80.0));
            ui.label("to");
            ui.add(egui::TextEdit::singleline(&mut mv.range_hi).desired_width(80.0));
        });
    } else {
        ui.label("Values (separated by commas):");
        ui.add(egui::TextEdit::singleline(&mut mv.discrete).desired_width(380.0));
    }
    ui.add_space(10.0);

    // The action row.
    ui.horizontal(|ui| {
        if ui.button("OK").clicked() {
            let variable = mv.variable.clone().unwrap_or_default();
            let discrete = if mv.use_range { Vec::new() } else { mv.discrete_values() };
            let range = if mv.use_range { mv.range() } else { None };
            mv.confirmed = Some((variable, discrete, range));
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if ui.button("Cancel").clicked() {
            mv.confirmed = None;
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if ui.button("Reset").clicked() {
            mv.discrete.clear();
            mv.range_lo.clear();
            mv.range_hi.clear();
            mv.use_range = false;
        }
        if ui.button("Help").clicked() {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
    });
}

// ---------------------------------------------------------------------------
// The Value Labels dialog
//
// Like Missing Values, this edits a variable's properties rather than
// running a procedure. It is opened from the "Values" cell in Variable
// View, from Data > Value Labels, and it writes the labels back to the
// variable's metadata, which the .sav writer preserves.
// ---------------------------------------------------------------------------

/// The state of the "Value Labels" dialog.
#[derive(Debug, Clone, Default)]
pub struct ValueLabels {
    /// The variable the labels are for.
    pub variable: Option<String>,
    /// One row per label: the value as typed, and the label shown for it.
    pub rows: Vec<(String, String)>,
    /// A validation message, shown under the grid.
    pub error: Option<String>,
    /// Set when the user pressed Cancel.
    pub cancelled: bool,
    /// Set when the labels were applied, so the caller can drop the dialog.
    pub confirmed: Option<String>,
}

impl ValueLabels {
    /// Open the dialog on one variable, with its current labels loaded.
    pub fn for_variable(app: &StatsCraftApp, variable: &str) -> Self {
        let mut dlg = Self {
            variable: Some(variable.to_string()),
            ..Self::default()
        };
        dlg.reload(app);
        dlg
    }

    /// Open the dialog on the first variable in the dataset.
    pub fn for_first_variable(app: &StatsCraftApp) -> Self {
        let name = app
            .table
            .as_ref()
            .and_then(|t| t.columns.first())
            .map(|c| c.name.clone())
            .unwrap_or_default();
        Self::for_variable(app, &name)
    }

    /// Replace the rows with the variable's current labels.
    pub fn reload(&mut self, app: &StatsCraftApp) {
        self.rows.clear();
        self.error = None;
        let Some(name) = self.variable.as_deref() else {
            return;
        };
        let Some(table) = app.table.as_ref() else {
            return;
        };
        let Some(c) = table
            .columns
            .iter()
            .find(|c| c.name.eq_ignore_ascii_case(name))
        else {
            return;
        };
        for (value, label) in &c.meta.value_labels {
            self.rows.push((format!("{}", value), label.clone()));
        }
        for (value, label) in &c.meta.text_value_labels {
            self.rows.push((value.clone(), label.clone()));
        }
        if self.rows.is_empty() {
            self.rows.push((String::new(), String::new()));
        }
    }
}

/// Draw the "Value Labels" dialog.
pub fn show_value_labels(ctx: &egui::Context, app: &mut StatsCraftApp, dlg: &mut ValueLabels) {
    let mut open = true;
    egui::Window::new("Value Labels - StatsCraft")
        .open(&mut open)
        .resizable(false)
        .default_width(480.0)
        .default_pos(egui::pos2(400.0, 170.0))
        .show(ctx, |ui| {
            render_value_labels(ui, app, dlg);
        });
    if !open {
        dlg.cancelled = true;
    }
}

/// The body of the "Value Labels" dialog, split out so a test can drive it
/// without a window.
pub fn render_value_labels(ui: &mut egui::Ui, app: &mut StatsCraftApp, dlg: &mut ValueLabels) {
    let names: Vec<String> = app
        .table
        .as_ref()
        .map(|t| t.columns.iter().map(|c| c.name.clone()).collect())
        .unwrap_or_default();

    if names.is_empty() {
        ui.label(
            egui::RichText::new("No variables in the data.")
                .color(theme::text_muted()),
        );
        ui.add_space(8.0);
        if ui.button("Close").clicked() {
            dlg.cancelled = true;
        }
        return;
    }

    // The variable. Switching it reloads that variable's labels.
    ui.label(egui::RichText::new("Variable:").strong());
    let current = dlg
        .variable
        .clone()
        .filter(|n| names.iter().any(|x| x.eq_ignore_ascii_case(n)))
        .unwrap_or_else(|| names[0].clone());
    dlg.variable = Some(current.clone());
    let mut chosen = current;
    egui::ComboBox::from_id_salt("value_labels_var")
        .selected_text(chosen.clone())
        .width(240.0)
        .show_ui(ui, |ui| {
            for name in &names {
                if ui
                    .selectable_label(name.eq_ignore_ascii_case(&chosen), name)
                    .clicked()
                {
                    chosen = name.clone();
                }
            }
        });
    if dlg.variable.as_deref() != Some(chosen.as_str()) {
        dlg.variable = Some(chosen);
        dlg.reload(app);
    }
    ui.add_space(8.0);

    let numeric = app
        .table
        .as_ref()
        .and_then(|t| {
            t.columns
                .iter()
                .find(|c| dlg.variable.as_deref().is_some_and(|n| c.name.eq_ignore_ascii_case(n)))
        })
        .map(|c| c.kind == ColumnKind::Numeric)
        .unwrap_or(false);
    ui.label(
        egui::RichText::new(if numeric {
            "Each row maps a numeric value to the label shown for it."
        } else {
            "Each row maps a text value to the label shown for it."
        })
        .color(theme::text_faint())
        .size(11.5),
    );
    ui.add_space(6.0);

    let mut remove_row = None;
    let danger = ui.visuals().error_fg_color;
    ui.horizontal(|ui| {
        ui.add_sized(
            [100.0, 18.0],
            egui::Label::new(egui::RichText::new("Value").strong()),
        );
        ui.add_sized(
            [240.0, 18.0],
            egui::Label::new(egui::RichText::new("Label").strong()),
        );
    });
    egui::ScrollArea::vertical()
        .max_height(250.0)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            for (i, (value, label)) in dlg.rows.iter_mut().enumerate() {
                ui.push_id(("value_label_row", i), |ui| {
                    ui.horizontal(|ui| {
                        ui.add_sized(
                            [100.0, 22.0],
                            egui::TextEdit::singleline(value)
                                .hint_text(if numeric { "1" } else { "text" }),
                        );
                        ui.add_sized(
                            [240.0, 22.0],
                            egui::TextEdit::singleline(label).hint_text("Label shown in output"),
                        );
                        if crate::icon::small_icon_button(
                            ui,
                            theme::Icon::DeleteVariable,
                            "Remove this value label",
                            danger,
                        )
                        .clicked()
                        {
                            remove_row = Some(i);
                        }
                    });
                });
            }
        });
    if let Some(i) = remove_row {
        dlg.rows.remove(i);
    }

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        if ui.button("+ Add Label").clicked() {
            dlg.rows.push((String::new(), String::new()));
        }
        if ui.button("Reset").clicked() {
            dlg.reload(app);
        }
    });

    if let Some(error) = &dlg.error {
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new(error)
                .color(ui.visuals().error_fg_color)
                .size(11.5),
        );
    }

    ui.add_space(10.0);
    ui.horizontal(|ui| {
        if ui.button("OK").clicked() {
            let Some(name) = dlg.variable.clone() else {
                dlg.error = Some("Choose a variable.".to_string());
                return;
            };
            let col = app.table.as_ref().and_then(|t| t.column_index(&name));
            let Some(col) = col else {
                dlg.error = Some("That variable no longer exists.".to_string());
                return;
            };
            let result = app
                .table
                .as_mut()
                .map(|t| t.set_value_labels(col, &dlg.rows))
                .unwrap_or_else(|| Err("No data loaded.".to_string()));
            match result {
                Ok(n) => {
                    app.dirty = true;
                    app.status = if n == 0 {
                        format!("{}: value labels cleared.", name)
                    } else {
                        format!("{}: {} value label(s) defined.", name, n)
                    };
                    dlg.confirmed = Some(name);
                }
                Err(e) => dlg.error = Some(e),
            }
        }
        if ui.button("Cancel").clicked() {
            dlg.cancelled = true;
        }
    });
}

// ---------------------------------------------------------------------------
// Compute Variable dialog (Transform)
// ---------------------------------------------------------------------------

/// The state of the "Compute Variable" dialog.
#[derive(Debug, Clone, Default)]
pub struct ComputeVariable {
    pub target: String,
    pub expression: String,
    pub use_if: bool,
    pub if_condition: String,
    pub error: Option<String>,
    pub confirmed: Option<String>,
    pub pasted: Option<String>,
    pub cancelled: bool,
}

impl ComputeVariable {
    pub fn build_syntax(&self) -> Option<String> {
        let target = self.target.trim();
        let expression = self.expression.trim();
        if target.is_empty() || expression.is_empty() {
            return None;
        }
        if !is_valid_var_name(target) {
            return None;
        }
        if self.use_if {
            let cond = self.if_condition.trim();
            if cond.is_empty() {
                return None;
            }
            Some(format!("COMPUTE {} = {} IF {}", target, expression, cond))
        } else {
            Some(format!("COMPUTE {} = {}", target, expression))
        }
    }
}

fn is_valid_var_name(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '@' || c == '#' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '@' || c == '#')
}

/// Draw the "Compute Variable" dialog.
pub fn show_compute(ctx: &egui::Context, app: &mut StatsCraftApp, dlg: &mut ComputeVariable) {
    let mut open = true;
    egui::Window::new("Compute Variable - StatsCraft")
        .open(&mut open)
        .resizable(true)
        .default_width(520.0)
        .default_height(420.0)
        .default_pos(egui::pos2(380.0, 160.0))
        .show(ctx, |ui| {
            render_compute(ui, app, dlg);
        });
    if !open && dlg.confirmed.is_none() && dlg.pasted.is_none() {
        dlg.cancelled = true;
    }
}

pub fn render_compute(ui: &mut egui::Ui, app: &mut StatsCraftApp, dlg: &mut ComputeVariable) {
    let names: Vec<String> = app
        .table
        .as_ref()
        .map(|t| t.columns.iter().map(|c| c.name.clone()).collect())
        .unwrap_or_default();

    if names.is_empty() {
        ui.label(
            egui::RichText::new("No variables in the data.")
                .color(theme::text_muted()),
        );
        if ui.button("Close").clicked() {
            dlg.cancelled = true;
        }
        return;
    }

    ui.columns(2, |cols| {
        cols[0].set_min_width(160.0);
        cols[0].label(egui::RichText::new("Variables:").strong());
        egui::ScrollArea::vertical()
            .max_height(220.0)
            .show(&mut cols[0], |ui| {
                for name in &names {
                    if ui.small_button(name).clicked() {
                        if dlg.expression.is_empty() {
                            dlg.expression = name.clone();
                        } else {
                            dlg.expression.push(' ');
                            dlg.expression.push_str(name);
                        }
                    }
                }
            });

        cols[1].vertical(|ui| {
            ui.label(egui::RichText::new("Target Variable:").strong());
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut dlg.target)
                        .desired_width(180.0)
                        .hint_text("new or existing name"),
                );
                egui::ComboBox::from_id_salt("compute_target_pick")
                    .selected_text("Existing…")
                    .width(100.0)
                    .show_ui(ui, |ui| {
                        for name in &names {
                            if ui.selectable_label(false, name).clicked() {
                                dlg.target = name.clone();
                            }
                        }
                    });
            });
            ui.add_space(6.0);
            ui.label(egui::RichText::new("Numeric Expression:").strong());
            ui.add(
                egui::TextEdit::multiline(&mut dlg.expression)
                    .desired_width(f32::INFINITY)
                    .desired_rows(4)
                    .font(egui::TextStyle::Monospace),
            );
            ui.horizontal_wrapped(|ui| {
                ui.label("Functions:");
                for f in [
                    "MEAN", "MIN", "MAX", "SUM", "ABS", "SQRT", "LN", "LOG10", "ROUND",
                ] {
                    if ui.small_button(f).clicked() {
                        if dlg.expression.is_empty() {
                            dlg.expression = format!("{}()", f);
                        } else {
                            dlg.expression.push(' ');
                            dlg.expression.push_str(&format!("{}()", f));
                        }
                    }
                }
            });
            ui.add_space(6.0);
            ui.checkbox(&mut dlg.use_if, "If case satisfies condition:");
            ui.add_enabled(
                dlg.use_if,
                egui::TextEdit::singleline(&mut dlg.if_condition)
                    .desired_width(f32::INFINITY)
                    .hint_text("e.g. age >= 18"),
            );
        });
    });

    if let Some(error) = &dlg.error {
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new(error)
                .color(ui.visuals().error_fg_color)
                .size(11.5),
        );
    }

    ui.add_space(10.0);
    let ok_enabled = dlg.build_syntax().is_some();
    ui.horizontal(|ui| {
        if ui.add_enabled(ok_enabled, egui::Button::new("OK")).clicked() {
            dlg.error = None;
            if let Some(syntax) = dlg.build_syntax() {
                dlg.confirmed = Some(syntax);
            } else {
                dlg.error = Some(
                    "Enter a target name, an expression, and a condition when IF is on.".to_string(),
                );
            }
        }
        if ui.add_enabled(ok_enabled, egui::Button::new("Paste")).clicked() {
            dlg.error = None;
            if let Some(syntax) = dlg.build_syntax() {
                dlg.pasted = Some(syntax);
            }
        }
        if ui.button("Reset").clicked() {
            *dlg = ComputeVariable::default();
        }
        if ui.button("Cancel").clicked() {
            dlg.cancelled = true;
        }
        if ui.button("Help").clicked() {
            dlg.error = Some(
                "Build an expression from variables and functions. IF limits which cases change."
                    .to_string(),
            );
        }
    });
}

fn syntax_action_row(
    ui: &mut egui::Ui,
    syntax: Option<String>,
    confirmed: &mut Option<String>,
    pasted: &mut Option<String>,
    cancelled: &mut bool,
    error: &mut Option<String>,
    help: &str,
) {
    let ok_enabled = syntax.is_some();
    ui.horizontal(|ui| {
        if ui.add_enabled(ok_enabled, egui::Button::new("OK")).clicked() {
            *error = None;
            if let Some(syntax) = syntax.clone() {
                *confirmed = Some(syntax);
            }
        }
        if ui.add_enabled(ok_enabled, egui::Button::new("Paste")).clicked() {
            *error = None;
            if let Some(syntax) = syntax.clone() {
                *pasted = Some(syntax);
            }
        }
        if ui.button("Cancel").clicked() {
            *cancelled = true;
        }
        if ui.button("Help").clicked() {
            *error = Some(help.to_string());
        }
    });
}

fn var_names(app: &StatsCraftApp) -> Vec<String> {
    app.table
        .as_ref()
        .map(|t| t.columns.iter().map(|c| c.name.clone()).collect())
        .unwrap_or_default()
}

/// Recode into the same variables, or into new ones.
#[derive(Debug, Clone)]
pub struct RecodeVariables {
    pub into_different: bool,
    pub sources: Vec<String>,
    pub dests: Vec<String>,
    pub rules: Vec<(String, String)>,
    pub else_copy: bool,
    pub else_value: String,
    pub error: Option<String>,
    pub confirmed: Option<String>,
    pub pasted: Option<String>,
    pub cancelled: bool,
}

impl RecodeVariables {
    pub fn same() -> Self {
        Self {
            into_different: false,
            sources: Vec::new(),
            dests: Vec::new(),
            rules: vec![(String::new(), String::new())],
            else_copy: true,
            else_value: String::new(),
            error: None,
            confirmed: None,
            pasted: None,
            cancelled: false,
        }
    }

    pub fn different() -> Self {
        let mut s = Self::same();
        s.into_different = true;
        s
    }

    pub fn build_syntax(&self) -> Option<String> {
        if self.sources.is_empty() {
            return None;
        }
        let mappings: Vec<String> = self
            .rules
            .iter()
            .filter(|(o, n)| !o.trim().is_empty() && !n.trim().is_empty())
            .map(|(o, n)| format!("({}={})", o.trim(), n.trim()))
            .collect();
        if mappings.is_empty() {
            return None;
        }
        let mut cmd = format!("RECODE {} {}", self.sources.join(" "), mappings.join(" "));
        if self.else_copy {
            cmd.push_str(" (ELSE=COPY)");
        } else if !self.else_value.trim().is_empty() {
            cmd.push_str(&format!(" (ELSE={})", self.else_value.trim()));
        }
        if self.into_different {
            if self.dests.len() != self.sources.len()
                || self.dests.iter().any(|d| !is_valid_var_name(d.trim()))
            {
                return None;
            }
            cmd.push_str(" INTO ");
            cmd.push_str(
                &self
                    .dests
                    .iter()
                    .map(|d| d.trim().to_string())
                    .collect::<Vec<_>>()
                    .join(" "),
            );
        }
        Some(cmd)
    }

    fn toggle_source(&mut self, name: &str) {
        if let Some(i) = self
            .sources
            .iter()
            .position(|s| s.eq_ignore_ascii_case(name))
        {
            self.sources.remove(i);
            if i < self.dests.len() {
                self.dests.remove(i);
            }
        } else {
            self.sources.push(name.to_string());
            self.dests.push(format!("{}_r", name));
        }
    }
}

pub fn show_recode(ctx: &egui::Context, app: &mut StatsCraftApp, dlg: &mut RecodeVariables) {
    let mut open = true;
    let title = if dlg.into_different {
        "Recode into Different Variables - StatsCraft"
    } else {
        "Recode into Same Variables - StatsCraft"
    };
    egui::Window::new(title)
        .open(&mut open)
        .resizable(true)
        .default_width(540.0)
        .default_pos(egui::pos2(360.0, 140.0))
        .show(ctx, |ui| {
            render_recode(ui, app, dlg);
        });
    if !open && dlg.confirmed.is_none() && dlg.pasted.is_none() {
        dlg.cancelled = true;
    }
}

pub fn render_recode(ui: &mut egui::Ui, app: &mut StatsCraftApp, dlg: &mut RecodeVariables) {
    let names = var_names(app);
    if names.is_empty() {
        ui.label(egui::RichText::new("No variables in the data.").color(theme::text_muted()));
        if ui.button("Close").clicked() {
            dlg.cancelled = true;
        }
        return;
    }
    ui.label(egui::RichText::new("Input Variable(s):").strong());
    ui.horizontal_wrapped(|ui| {
        for name in &names {
            let on = dlg.sources.iter().any(|s| s.eq_ignore_ascii_case(name));
            if ui.selectable_label(on, name).clicked() {
                dlg.toggle_source(name);
            }
        }
    });
    if dlg.into_different && !dlg.sources.is_empty() {
        ui.add_space(6.0);
        ui.label(egui::RichText::new("Output Variable(s):").strong());
        dlg.dests.resize(dlg.sources.len(), String::new());
        for (i, src) in dlg.sources.iter().enumerate() {
            ui.horizontal(|ui| {
                ui.label(format!("{} →", src));
                ui.add(egui::TextEdit::singleline(&mut dlg.dests[i]).desired_width(160.0));
            });
        }
    }
    ui.add_space(8.0);
    ui.label(egui::RichText::new("Old → New:").strong());
    ui.label(
        egui::RichText::new("Old can be a value, a list, MISSING, or a range such as 1 THRU 5.")
            .small()
            .color(theme::text_muted()),
    );
    let mut remove = None;
    for (i, (old, new)) in dlg.rules.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(old).desired_width(160.0).hint_text("old"));
            ui.label("=");
            ui.add(egui::TextEdit::singleline(new).desired_width(120.0).hint_text("new"));
            if ui.small_button("Remove").clicked() {
                remove = Some(i);
            }
        });
    }
    if let Some(i) = remove {
        dlg.rules.remove(i);
        if dlg.rules.is_empty() {
            dlg.rules.push((String::new(), String::new()));
        }
    }
    if ui.button("Add mapping").clicked() {
        dlg.rules.push((String::new(), String::new()));
    }
    ui.add_space(6.0);
    ui.checkbox(&mut dlg.else_copy, "Copy remaining values");
    ui.add_enabled(
        !dlg.else_copy,
        egui::TextEdit::singleline(&mut dlg.else_value)
            .desired_width(120.0)
            .hint_text("ELSE value or SYSMIS"),
    );
    if let Some(error) = &dlg.error {
        ui.label(egui::RichText::new(error).color(ui.visuals().error_fg_color).size(11.5));
    }
    ui.add_space(8.0);
    let syntax = dlg.build_syntax();
    syntax_action_row(
        ui,
        syntax,
        &mut dlg.confirmed,
        &mut dlg.pasted,
        &mut dlg.cancelled,
        &mut dlg.error,
        "Each mapping is (old=new). INTO writes new variables; without INTO the sources change.",
    );
}

/// Count how many of the chosen variables hold a listed value.
#[derive(Debug, Clone, Default)]
pub struct CountValues {
    pub dest: String,
    pub sources: Vec<String>,
    pub values: String,
    pub error: Option<String>,
    pub confirmed: Option<String>,
    pub pasted: Option<String>,
    pub cancelled: bool,
}

impl CountValues {
    pub fn build_syntax(&self) -> Option<String> {
        if !is_valid_var_name(self.dest.trim()) || self.sources.is_empty() {
            return None;
        }
        let values = self.values.trim();
        if values.is_empty() {
            return None;
        }
        Some(format!(
            "COUNT {} = {} ({})",
            self.dest.trim(),
            self.sources.join(" "),
            values
        ))
    }

    fn toggle(&mut self, name: &str) {
        if let Some(i) = self.sources.iter().position(|s| s.eq_ignore_ascii_case(name)) {
            self.sources.remove(i);
        } else {
            self.sources.push(name.to_string());
        }
    }
}

pub fn show_count(ctx: &egui::Context, app: &mut StatsCraftApp, dlg: &mut CountValues) {
    let mut open = true;
    egui::Window::new("Count Values within Cases - StatsCraft")
        .open(&mut open)
        .default_width(480.0)
        .default_pos(egui::pos2(380.0, 160.0))
        .show(ctx, |ui| render_count(ui, app, dlg));
    if !open && dlg.confirmed.is_none() && dlg.pasted.is_none() {
        dlg.cancelled = true;
    }
}

pub fn render_count(ui: &mut egui::Ui, app: &mut StatsCraftApp, dlg: &mut CountValues) {
    let names = var_names(app);
    if names.is_empty() {
        ui.label(egui::RichText::new("No variables in the data.").color(theme::text_muted()));
        if ui.button("Close").clicked() {
            dlg.cancelled = true;
        }
        return;
    }
    ui.label(egui::RichText::new("Target Variable:").strong());
    ui.add(egui::TextEdit::singleline(&mut dlg.dest).desired_width(180.0).hint_text("hits"));
    ui.add_space(6.0);
    ui.label(egui::RichText::new("Numeric Variables:").strong());
    ui.horizontal_wrapped(|ui| {
        for name in &names {
            let on = dlg.sources.iter().any(|s| s.eq_ignore_ascii_case(name));
            if ui.selectable_label(on, name).clicked() {
                dlg.toggle(name);
            }
        }
    });
    ui.add_space(6.0);
    ui.label("Values:");
    ui.add(
        egui::TextEdit::singleline(&mut dlg.values)
            .desired_width(f32::INFINITY)
            .hint_text("1 2 3"),
    );
    if let Some(error) = &dlg.error {
        ui.label(egui::RichText::new(error).color(ui.visuals().error_fg_color).size(11.5));
    }
    ui.add_space(8.0);
    let syntax = dlg.build_syntax();
    syntax_action_row(
        ui,
        syntax,
        &mut dlg.confirmed,
        &mut dlg.pasted,
        &mut dlg.cancelled,
        &mut dlg.error,
        "The target counts how many of the chosen variables equal a listed value.",
    );
}

/// Rank cases of one numeric variable.
#[derive(Debug, Clone, Default)]
pub struct RankCases {
    pub source: String,
    pub dest: String,
    pub error: Option<String>,
    pub confirmed: Option<String>,
    pub pasted: Option<String>,
    pub cancelled: bool,
}

impl RankCases {
    pub fn build_syntax(&self) -> Option<String> {
        if self.source.trim().is_empty() {
            return None;
        }
        let dest = self.dest.trim();
        if dest.is_empty() {
            Some(format!("RANK {}", self.source.trim()))
        } else if is_valid_var_name(dest) {
            Some(format!("RANK {} INTO {}", self.source.trim(), dest))
        } else {
            None
        }
    }
}

pub fn show_rank(ctx: &egui::Context, app: &mut StatsCraftApp, dlg: &mut RankCases) {
    let mut open = true;
    egui::Window::new("Rank Cases - StatsCraft")
        .open(&mut open)
        .default_width(420.0)
        .default_pos(egui::pos2(400.0, 180.0))
        .show(ctx, |ui| render_rank(ui, app, dlg));
    if !open && dlg.confirmed.is_none() && dlg.pasted.is_none() {
        dlg.cancelled = true;
    }
}

pub fn render_rank(ui: &mut egui::Ui, app: &mut StatsCraftApp, dlg: &mut RankCases) {
    let names = var_names(app);
    if names.is_empty() {
        ui.label(egui::RichText::new("No variables in the data.").color(theme::text_muted()));
        if ui.button("Close").clicked() {
            dlg.cancelled = true;
        }
        return;
    }
    if dlg.source.is_empty() {
        dlg.source = names[0].clone();
    }
    ui.label(egui::RichText::new("Variable:").strong());
    egui::ComboBox::from_id_salt("rank_src")
        .selected_text(&dlg.source)
        .show_ui(ui, |ui| {
            for name in &names {
                if ui.selectable_label(dlg.source == *name, name).clicked() {
                    dlg.source = name.clone();
                }
            }
        });
    ui.add_space(6.0);
    ui.label("Rank into:");
    ui.add(
        egui::TextEdit::singleline(&mut dlg.dest)
            .desired_width(180.0)
            .hint_text(format!("{}_r", dlg.source)),
    );
    if let Some(error) = &dlg.error {
        ui.label(egui::RichText::new(error).color(ui.visuals().error_fg_color).size(11.5));
    }
    ui.add_space(8.0);
    let syntax = dlg.build_syntax();
    syntax_action_row(
        ui,
        syntax,
        &mut dlg.confirmed,
        &mut dlg.pasted,
        &mut dlg.cancelled,
        &mut dlg.error,
        "Tied values share the mean rank. Missing values stay missing.",
    );
}

/// Automatic recode of unique values into 1..n.
#[derive(Debug, Clone, Default)]
pub struct AutoRecode {
    pub source: String,
    pub dest: String,
    pub error: Option<String>,
    pub confirmed: Option<String>,
    pub pasted: Option<String>,
    pub cancelled: bool,
}

impl AutoRecode {
    pub fn build_syntax(&self) -> Option<String> {
        if self.source.trim().is_empty() || !is_valid_var_name(self.dest.trim()) {
            return None;
        }
        Some(format!(
            "AUTORECODE {} INTO {}",
            self.source.trim(),
            self.dest.trim()
        ))
    }
}

pub fn show_autorecode(ctx: &egui::Context, app: &mut StatsCraftApp, dlg: &mut AutoRecode) {
    let mut open = true;
    egui::Window::new("Automatic Recode - StatsCraft")
        .open(&mut open)
        .default_width(420.0)
        .default_pos(egui::pos2(400.0, 180.0))
        .show(ctx, |ui| render_autorecode(ui, app, dlg));
    if !open && dlg.confirmed.is_none() && dlg.pasted.is_none() {
        dlg.cancelled = true;
    }
}

pub fn render_autorecode(ui: &mut egui::Ui, app: &mut StatsCraftApp, dlg: &mut AutoRecode) {
    let names = var_names(app);
    if names.is_empty() {
        ui.label(egui::RichText::new("No variables in the data.").color(theme::text_muted()));
        if ui.button("Close").clicked() {
            dlg.cancelled = true;
        }
        return;
    }
    if dlg.source.is_empty() {
        dlg.source = names[0].clone();
    }
    ui.label(egui::RichText::new("Variable:").strong());
    egui::ComboBox::from_id_salt("auto_src")
        .selected_text(&dlg.source)
        .show_ui(ui, |ui| {
            for name in &names {
                if ui.selectable_label(dlg.source == *name, name).clicked() {
                    dlg.source = name.clone();
                    if dlg.dest.is_empty() {
                        dlg.dest = format!("{}N", name);
                    }
                }
            }
        });
    ui.add_space(6.0);
    ui.label("Into:");
    ui.add(
        egui::TextEdit::singleline(&mut dlg.dest)
            .desired_width(180.0)
            .hint_text("groupn"),
    );
    if let Some(error) = &dlg.error {
        ui.label(egui::RichText::new(error).color(ui.visuals().error_fg_color).size(11.5));
    }
    ui.add_space(8.0);
    let syntax = dlg.build_syntax();
    syntax_action_row(
        ui,
        syntax,
        &mut dlg.confirmed,
        &mut dlg.pasted,
        &mut dlg.cancelled,
        &mut dlg.error,
        "Unique values become 1, 2, 3… in sorted order. The original values are stored as labels.",
    );
}

/// Select Cases: keep rows that match a condition.
#[derive(Debug, Clone, Default)]
pub struct SelectCases {
    pub condition: String,
    pub error: Option<String>,
    pub confirmed: Option<String>,
    pub pasted: Option<String>,
    pub cancelled: bool,
}

impl SelectCases {
    pub fn build_syntax(&self) -> Option<String> {
        let cond = self.condition.trim();
        if cond.is_empty() {
            return None;
        }
        Some(format!("SELECT IF {}", cond))
    }
}

pub fn show_select(ctx: &egui::Context, app: &mut StatsCraftApp, dlg: &mut SelectCases) {
    let mut open = true;
    egui::Window::new("Select Cases - StatsCraft")
        .open(&mut open)
        .default_width(460.0)
        .default_pos(egui::pos2(400.0, 180.0))
        .show(ctx, |ui| render_select(ui, app, dlg));
    if !open && dlg.confirmed.is_none() && dlg.pasted.is_none() {
        dlg.cancelled = true;
    }
}

pub fn render_select(ui: &mut egui::Ui, app: &mut StatsCraftApp, dlg: &mut SelectCases) {
    let names = var_names(app);
    if names.is_empty() {
        ui.label(egui::RichText::new("No variables in the data.").color(theme::text_muted()));
        if ui.button("Close").clicked() {
            dlg.cancelled = true;
        }
        return;
    }
    ui.label(
        egui::RichText::new("Cases that fail the condition are removed. Undo restores them.")
            .small()
            .color(theme::text_muted()),
    );
    ui.add_space(6.0);
    ui.label(egui::RichText::new("Condition:").strong());
    ui.add(
        egui::TextEdit::singleline(&mut dlg.condition)
            .desired_width(f32::INFINITY)
            .hint_text("age >= 18"),
    );
    ui.horizontal_wrapped(|ui| {
        ui.label("Insert:");
        for name in &names {
            if ui.small_button(name).clicked() {
                if dlg.condition.is_empty() {
                    dlg.condition = name.clone();
                } else {
                    dlg.condition.push(' ');
                    dlg.condition.push_str(name);
                }
            }
        }
    });
    if let Some(error) = &dlg.error {
        ui.label(egui::RichText::new(error).color(ui.visuals().error_fg_color).size(11.5));
    }
    ui.add_space(8.0);
    let syntax = dlg.build_syntax();
    syntax_action_row(
        ui,
        syntax,
        &mut dlg.confirmed,
        &mut dlg.pasted,
        &mut dlg.cancelled,
        &mut dlg.error,
        "SELECT IF uses the same comparisons as Compute: =, <>, <, <=, >, >=.",
    );
}

fn toggle_name(list: &mut Vec<String>, name: &str) {
    if let Some(i) = list.iter().position(|s| s.eq_ignore_ascii_case(name)) {
        list.remove(i);
    } else {
        list.push(name.to_string());
    }
}

#[derive(Debug, Clone, Default)]
pub struct WeightCases {
    pub variable: String,
    pub error: Option<String>,
    pub confirmed: Option<String>,
    pub pasted: Option<String>,
    pub cancelled: bool,
}

impl WeightCases {
    pub fn build_syntax(&self) -> Option<String> {
        if self.variable.trim().eq_ignore_ascii_case("OFF") || self.variable.trim().is_empty() {
            Some("WEIGHT OFF".to_string())
        } else {
            Some(format!("WEIGHT BY {}", self.variable.trim()))
        }
    }
}

pub fn show_weight(ctx: &egui::Context, app: &mut StatsCraftApp, dlg: &mut WeightCases) {
    let mut open = true;
    egui::Window::new("Weight Cases - StatsCraft")
        .open(&mut open)
        .default_width(420.0)
        .default_pos(egui::pos2(400.0, 180.0))
        .show(ctx, |ui| render_weight(ui, app, dlg));
    if !open && dlg.confirmed.is_none() && dlg.pasted.is_none() {
        dlg.cancelled = true;
    }
}

pub fn render_weight(ui: &mut egui::Ui, app: &mut StatsCraftApp, dlg: &mut WeightCases) {
    let names = var_names(app);
    ui.label(egui::RichText::new("Frequency weight:").strong());
    egui::ComboBox::from_id_salt("weight_var")
        .selected_text(if dlg.variable.is_empty() {
            "(none)".to_string()
        } else {
            dlg.variable.clone()
        })
        .show_ui(ui, |ui| {
            if ui.selectable_label(dlg.variable.is_empty(), "(none / WEIGHT OFF)").clicked() {
                dlg.variable.clear();
            }
            for name in &names {
                if ui.selectable_label(dlg.variable == *name, name).clicked() {
                    dlg.variable = name.clone();
                }
            }
        });
    ui.add_space(8.0);
    let syntax = dlg.build_syntax();
    syntax_action_row(
        ui,
        syntax,
        &mut dlg.confirmed,
        &mut dlg.pasted,
        &mut dlg.cancelled,
        &mut dlg.error,
        "A positive weight is rounded to a whole number of copies of the case.",
    );
}

#[derive(Debug, Clone, Default)]
pub struct SplitFile {
    pub variables: Vec<String>,
    pub off: bool,
    pub error: Option<String>,
    pub confirmed: Option<String>,
    pub pasted: Option<String>,
    pub cancelled: bool,
}

impl SplitFile {
    pub fn build_syntax(&self) -> Option<String> {
        if self.off || self.variables.is_empty() {
            Some("SPLIT FILE OFF".to_string())
        } else {
            Some(format!("SPLIT FILE BY {}", self.variables.join(" ")))
        }
    }
}

pub fn show_split(ctx: &egui::Context, app: &mut StatsCraftApp, dlg: &mut SplitFile) {
    let mut open = true;
    egui::Window::new("Split File - StatsCraft")
        .open(&mut open)
        .default_width(460.0)
        .default_pos(egui::pos2(380.0, 170.0))
        .show(ctx, |ui| render_split(ui, app, dlg));
    if !open && dlg.confirmed.is_none() && dlg.pasted.is_none() {
        dlg.cancelled = true;
    }
}

pub fn render_split(ui: &mut egui::Ui, app: &mut StatsCraftApp, dlg: &mut SplitFile) {
    let names = var_names(app);
    ui.checkbox(&mut dlg.off, "Analyze all cases, no split");
    ui.add_enabled_ui(!dlg.off, |ui| {
        ui.label(egui::RichText::new("Groups based on:").strong());
        ui.horizontal_wrapped(|ui| {
            for name in &names {
                let on = dlg.variables.iter().any(|s| s.eq_ignore_ascii_case(name));
                if ui.selectable_label(on, name).clicked() {
                    toggle_name(&mut dlg.variables, name);
                }
            }
        });
    });
    ui.add_space(8.0);
    let syntax = dlg.build_syntax();
    syntax_action_row(
        ui,
        syntax,
        &mut dlg.confirmed,
        &mut dlg.pasted,
        &mut dlg.cancelled,
        &mut dlg.error,
        "Each analysis runs once per combination of the split variables.",
    );
}

#[derive(Debug, Clone, Default)]
pub struct Aggregate {
    pub breaks: Vec<String>,
    pub source: String,
    pub mean: bool,
    pub sum: bool,
    pub n: bool,
    pub sd: bool,
    pub min: bool,
    pub max: bool,
    pub error: Option<String>,
    pub confirmed: Option<String>,
    pub pasted: Option<String>,
    pub cancelled: bool,
}

impl Aggregate {
    pub fn build_syntax(&self) -> Option<String> {
        let mut parts = vec!["AGGREGATE".to_string()];
        if !self.breaks.is_empty() {
            parts.push(format!("BREAK {}", self.breaks.join(" ")));
        }
        let src = self.source.trim();
        if self.mean {
            if src.is_empty() {
                return None;
            }
            parts.push(format!("/MEAN({})", src));
        }
        if self.sum && !src.is_empty() {
            parts.push(format!("/SUM({})", src));
        }
        if self.min && !src.is_empty() {
            parts.push(format!("/MIN({})", src));
        }
        if self.max && !src.is_empty() {
            parts.push(format!("/MAX({})", src));
        }
        if self.sd && !src.is_empty() {
            parts.push(format!("/SD({})", src));
        }
        if self.n || parts.len() == 1 {
            parts.push("/N".to_string());
        }
        Some(parts.join(" "))
    }
}

pub fn show_aggregate(ctx: &egui::Context, app: &mut StatsCraftApp, dlg: &mut Aggregate) {
    let mut open = true;
    egui::Window::new("Aggregate - StatsCraft")
        .open(&mut open)
        .default_width(480.0)
        .default_pos(egui::pos2(380.0, 160.0))
        .show(ctx, |ui| render_aggregate(ui, app, dlg));
    if !open && dlg.confirmed.is_none() && dlg.pasted.is_none() {
        dlg.cancelled = true;
    }
}

pub fn render_aggregate(ui: &mut egui::Ui, app: &mut StatsCraftApp, dlg: &mut Aggregate) {
    let names = var_names(app);
    if names.is_empty() {
        ui.label(egui::RichText::new("No variables in the data.").color(theme::text_muted()));
        if ui.button("Close").clicked() {
            dlg.cancelled = true;
        }
        return;
    }
    ui.label(egui::RichText::new("Break variables:").strong());
    ui.horizontal_wrapped(|ui| {
        for name in &names {
            let on = dlg.breaks.iter().any(|s| s.eq_ignore_ascii_case(name));
            if ui.selectable_label(on, name).clicked() {
                toggle_name(&mut dlg.breaks, name);
            }
        }
    });
    ui.add_space(6.0);
    ui.label(egui::RichText::new("Summarise:").strong());
    egui::ComboBox::from_id_salt("agg_src")
        .selected_text(if dlg.source.is_empty() {
            "(variable)".to_string()
        } else {
            dlg.source.clone()
        })
        .show_ui(ui, |ui| {
            for name in &names {
                if ui.selectable_label(dlg.source == *name, name).clicked() {
                    dlg.source = name.clone();
                }
            }
        });
    ui.horizontal_wrapped(|ui| {
        ui.checkbox(&mut dlg.n, "N");
        ui.checkbox(&mut dlg.mean, "Mean");
        ui.checkbox(&mut dlg.sum, "Sum");
        ui.checkbox(&mut dlg.min, "Min");
        ui.checkbox(&mut dlg.max, "Max");
        ui.checkbox(&mut dlg.sd, "SD");
    });
    ui.add_space(8.0);
    let syntax = dlg.build_syntax();
    syntax_action_row(
        ui,
        syntax,
        &mut dlg.confirmed,
        &mut dlg.pasted,
        &mut dlg.cancelled,
        &mut dlg.error,
        "The dataset is replaced by one row per break group. Undo restores the cases.",
    );
}

#[derive(Debug, Clone, Default)]
pub struct FileJoin {
    pub match_files: bool,
    pub path: String,
    pub by: String,
    pub error: Option<String>,
    pub confirmed: Option<String>,
    pub pasted: Option<String>,
    pub cancelled: bool,
}

impl FileJoin {
    pub fn add() -> Self {
        Self::default()
    }
    pub fn match_join() -> Self {
        Self {
            match_files: true,
            ..Self::default()
        }
    }
    pub fn build_syntax(&self) -> Option<String> {
        let path = self.path.trim();
        if path.is_empty() {
            return None;
        }
        if self.match_files {
            let by = self.by.trim();
            if by.is_empty() {
                return None;
            }
            Some(format!("MATCH FILES \"{}\" BY {}", path.replace('"', ""), by))
        } else {
            Some(format!("ADD FILES \"{}\"", path.replace('"', "")))
        }
    }
}

pub fn show_file_join(ctx: &egui::Context, app: &mut StatsCraftApp, dlg: &mut FileJoin) {
    let mut open = true;
    let title = if dlg.match_files {
        "Match Files - StatsCraft"
    } else {
        "Add Files - StatsCraft"
    };
    egui::Window::new(title)
        .open(&mut open)
        .default_width(500.0)
        .default_pos(egui::pos2(360.0, 170.0))
        .show(ctx, |ui| render_file_join(ui, app, dlg));
    if !open && dlg.confirmed.is_none() && dlg.pasted.is_none() {
        dlg.cancelled = true;
    }
}

pub fn render_file_join(ui: &mut egui::Ui, app: &mut StatsCraftApp, dlg: &mut FileJoin) {
    ui.label(egui::RichText::new("Other file:").strong());
    ui.horizontal(|ui| {
        ui.add(egui::TextEdit::singleline(&mut dlg.path).desired_width(320.0));
        if ui.button("Browse…").clicked() {
            if let Some(p) = rfd::FileDialog::new()
                .add_filter("Data", &["csv", "sav"])
                .pick_file()
            {
                dlg.path = p.display().to_string();
            }
        }
    });
    if dlg.match_files {
        ui.add_space(6.0);
        ui.label(egui::RichText::new("Key variable:").strong());
        let names = var_names(app);
        egui::ComboBox::from_id_salt("match_by")
            .selected_text(if dlg.by.is_empty() {
                "(key)".to_string()
            } else {
                dlg.by.clone()
            })
            .show_ui(ui, |ui| {
                for name in &names {
                    if ui.selectable_label(dlg.by == *name, name).clicked() {
                        dlg.by = name.clone();
                    }
                }
            });
    }
    ui.add_space(8.0);
    let syntax = dlg.build_syntax();
    syntax_action_row(
        ui,
        syntax,
        &mut dlg.confirmed,
        &mut dlg.pasted,
        &mut dlg.cancelled,
        &mut dlg.error,
        if dlg.match_files {
            "Rows of the other file are matched by the key. New columns are added."
        } else {
            "Cases of the other file are appended. New columns are added."
        },
    );
}

#[derive(Debug, Clone, Default)]
pub struct Restructure {
    pub to_long: bool,
    pub make: String,
    pub from: Vec<String>,
    pub index: String,
    pub id: String,
    pub error: Option<String>,
    pub confirmed: Option<String>,
    pub pasted: Option<String>,
    pub cancelled: bool,
}

impl Restructure {
    pub fn vars_to_cases() -> Self {
        Self {
            to_long: true,
            ..Self::default()
        }
    }
    pub fn cases_to_vars() -> Self {
        Self::default()
    }
    pub fn build_syntax(&self) -> Option<String> {
        if self.to_long {
            if self.make.trim().is_empty() || self.from.is_empty() {
                return None;
            }
            let mut s = format!(
                "VARSTOCASES MAKE {} FROM {}",
                self.make.trim(),
                self.from.join(" ")
            );
            if !self.index.trim().is_empty() {
                s.push_str(&format!(" /INDEX={}", self.index.trim()));
            }
            Some(s)
        } else if !self.id.trim().is_empty() && !self.index.trim().is_empty() {
            Some(format!(
                "CASESTOVARS ID={} /INDEX={}",
                self.id.trim(),
                self.index.trim()
            ))
        } else {
            None
        }
    }
}

pub fn show_restructure(ctx: &egui::Context, app: &mut StatsCraftApp, dlg: &mut Restructure) {
    let mut open = true;
    let title = if dlg.to_long {
        "Restructure (Variables to Cases) - StatsCraft"
    } else {
        "Restructure (Cases to Variables) - StatsCraft"
    };
    egui::Window::new(title)
        .open(&mut open)
        .default_width(520.0)
        .default_pos(egui::pos2(350.0, 150.0))
        .show(ctx, |ui| render_restructure(ui, app, dlg));
    if !open && dlg.confirmed.is_none() && dlg.pasted.is_none() {
        dlg.cancelled = true;
    }
}

pub fn render_restructure(ui: &mut egui::Ui, app: &mut StatsCraftApp, dlg: &mut Restructure) {
    let names = var_names(app);
    if names.is_empty() {
        ui.label(egui::RichText::new("No variables in the data.").color(theme::text_muted()));
        if ui.button("Close").clicked() {
            dlg.cancelled = true;
        }
        return;
    }
    if dlg.to_long {
        ui.label(egui::RichText::new("New stacked variable:").strong());
        ui.add(egui::TextEdit::singleline(&mut dlg.make).desired_width(180.0).hint_text("score"));
        ui.add_space(4.0);
        ui.label(egui::RichText::new("From:").strong());
        ui.horizontal_wrapped(|ui| {
            for name in &names {
                let on = dlg.from.iter().any(|s| s.eq_ignore_ascii_case(name));
                if ui.selectable_label(on, name).clicked() {
                    toggle_name(&mut dlg.from, name);
                }
            }
        });
        ui.add_space(4.0);
        ui.label("Index name (optional):");
        ui.add(egui::TextEdit::singleline(&mut dlg.index).desired_width(180.0).hint_text("wave"));
    } else {
        ui.label(egui::RichText::new("ID variable:").strong());
        egui::ComboBox::from_id_salt("re_id")
            .selected_text(if dlg.id.is_empty() { "(id)".into() } else { dlg.id.clone() })
            .show_ui(ui, |ui| {
                for name in &names {
                    if ui.selectable_label(dlg.id == *name, name).clicked() {
                        dlg.id = name.clone();
                    }
                }
            });
        ui.add_space(4.0);
        ui.label(egui::RichText::new("Index variable:").strong());
        egui::ComboBox::from_id_salt("re_idx")
            .selected_text(if dlg.index.is_empty() {
                "(index)".into()
            } else {
                dlg.index.clone()
            })
            .show_ui(ui, |ui| {
                for name in &names {
                    if ui.selectable_label(dlg.index == *name, name).clicked() {
                        dlg.index = name.clone();
                    }
                }
            });
    }
    ui.add_space(8.0);
    let syntax = dlg.build_syntax();
    syntax_action_row(
        ui,
        syntax,
        &mut dlg.confirmed,
        &mut dlg.pasted,
        &mut dlg.cancelled,
        &mut dlg.error,
        "The dataset is replaced. Undo restores the previous shape.",
    );
}
