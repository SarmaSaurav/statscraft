use crate::data::DataTable;
use crate::output::OutputTable;
use crate::stats;
use eframe::egui;
use egui_plot::{Plot, PlotPoints};

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum View {
    Data,
    Variables,
    Output,
    Charts,
    Syntax,
}

pub struct StateCraftApp {
    pub path_input: String,
    pub table: Option<DataTable>,
    pub status: String,
    pub outputs: Vec<OutputTable>,
    pub view: View,
    pub syntax_text: String,
    pub selected_output: usize,
    pub var_filter: String,
    pub grid_lines: bool,
    pub show_status: bool,
    pub dataset_no: usize,
}

impl Default for StateCraftApp {
    fn default() -> Self {
        Self {
            path_input: String::new(),
            table: None,
            status: "Ready. Enter a CSV path and click Load.".to_string(),
            outputs: Vec::new(),
            view: View::Data,
            syntax_text: String::from("LOAD \"samples/demo.csv\"\nDESCRIPTIVES\nFREQUENCIES\nCORRELATIONS\nREGRESSION\n"),
            selected_output: 0,
            var_filter: String::new(),
            grid_lines: true,
            show_status: true,
            dataset_no: 0,
        }
    }
}

impl StateCraftApp {
    pub fn load(&mut self) {
        let p = std::path::Path::new(self.path_input.trim());
        let res = if p
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("sav"))
            .unwrap_or(false)
        {
            DataTable::from_sav_path(p)
        } else {
            DataTable::from_csv_path(p)
        };
        match res {
            Ok(t) => {
                self.status = format!("Loaded {} rows x {} cols.", t.n_rows, t.columns.len());
                self.table = Some(t);
                self.dataset_no += 1;
            }
            Err(e) => {
                self.status = format!("Error: {}", e);
                self.table = None;
            }
        }
    }

    /// SPSS-style dataset title, e.g. "bank [DataSet1]" or "Untitled1 [DataSet1]".
    fn dataset_label(&self) -> String {
        let stem = std::path::Path::new(self.path_input.trim())
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Untitled1")
            .to_string();
        format!("{} [DataSet{}]", stem, self.dataset_no)
    }

    pub fn analyze_descriptives(&mut self) {
        if let Some(t) = &self.table {
            let d = stats::descriptives(t);
            self.outputs.push(OutputTable::from_descriptives(&d));
            self.status = format!("Computed descriptives for {} variables.", d.len());
            self.view = View::Output;
        } else {
            self.status = "No data loaded.".to_string();
        }
    }

    pub fn analyze_frequencies(&mut self) {
        if let Some(t) = &self.table {
            for ci in 0..t.columns.len() {
                let name = t.columns[ci].name.clone();
                let rows = stats::frequencies(t, ci);
                self.outputs.push(OutputTable::from_frequencies(&name, &rows));
            }
            self.status = format!("Computed frequencies for {} variables.", t.columns.len());
            self.view = View::Output;
        } else {
            self.status = "No data loaded.".to_string();
        }
    }

    pub fn analyze_correlations(&mut self) {
        if let Some(t) = &self.table {
            let c = stats::correlations(t);
            self.outputs.push(OutputTable::from_correlations(&c));
            self.status = format!("Computed {} correlations.", c.len());
            self.view = View::Output;
        } else {
            self.status = "No data loaded.".to_string();
        }
    }

    pub fn analyze_regression(&mut self) {
        if let Some(t) = &self.table {
            if let Some(r) = stats::linear_regression(t) {
                self.outputs.push(OutputTable::from_regression(&r));
                self.status = "Computed linear regression.".to_string();
                self.view = View::Output;
            } else {
                self.status = "Need at least 2 numeric columns.".to_string();
            }
        } else {
            self.status = "No data loaded.".to_string();
        }
    }

    pub fn export_output_to(&mut self, path: &std::path::Path) {
        if self.outputs.is_empty() {
            self.status = "No output to export.".to_string();
            return;
        }
        let mut md = String::new();
        for t in &self.outputs {
            md.push_str(&format!("## {}\n\n", t.title));
            md.push_str("| ");
            md.push_str(&t.headers.join(" | "));
            md.push_str(" |\n|");
            for _ in &t.headers {
                md.push_str("---|");
            }
            md.push('\n');
            for row in &t.rows {
                md.push_str("| ");
                md.push_str(&row.join(" | "));
                md.push_str(" |\n");
            }
            md.push('\n');
        }
        match std::fs::write(path, md) {
            Ok(_) => self.status = format!("Exported output to {}", path.display()),
            Err(e) => self.status = format!("Export failed: {}", e),
        }
    }

    pub fn export_data_csv_to(&mut self, path: &std::path::Path) {
        if let Some(t) = &self.table {
            let mut wtr = match csv::Writer::from_path(path) {
                Ok(w) => w,
                Err(e) => {
                    self.status = format!("Export failed: {}", e);
                    return;
                }
            };
            let headers: Vec<&str> = t.columns.iter().map(|c| c.name.as_str()).collect();
            let _ = wtr.write_record(&headers);
            for r in 0..t.n_rows {
                let row: Vec<String> = (0..t.columns.len()).map(|ci| t.cell_text(ci, r)).collect();
                let _ = wtr.write_record(&row);
            }
            let _ = wtr.flush();
            self.status = format!("Exported data to {}", path.display());
        } else {
            self.status = "No data loaded.".to_string();
        }
    }

    pub fn analyze_ttest(&mut self) {
        if let Some(t) = &self.table {
            if let Some(r) = stats::independent_ttest(t) {
                self.outputs.push(OutputTable::from_ttest(&r));
                self.status = "Computed t-test.".to_string();
                self.view = View::Output;
            } else {
                self.status = "Need a 2-level group and numeric outcome.".to_string();
            }
        } else {
            self.status = "No data loaded.".to_string();
        }
    }
}

impl eframe::App for StateCraftApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.style_mut(|s| {
            s.visuals = egui::Visuals::light();
            s.visuals.panel_fill = egui::Color32::WHITE;
            s.visuals.window_fill = egui::Color32::WHITE;
        });
        let title = format!("{} - StateCraft Data Editor", self.dataset_label());
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));

        egui::TopBottomPanel::top("menu").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                // ---------------- File ----------------
                ui.menu_button("File", |ui| {
                    if ui.button("New").clicked() { ui.close_menu(); self.status = "New document: not implemented yet.".to_string(); }
                    ui.separator();
                    if ui.button("Open Data...").clicked() { ui.close_menu();
                        if let Some(p) = rfd::FileDialog::new().add_filter("Data", &["csv", "sav"]).pick_file() {
                            self.path_input = p.display().to_string();
                            self.load();
                        }
                    }
                    ui.menu_button("Import Data", |ui| {
                        if ui.button("CSV Data...").clicked() { ui.close_menu();
                            if let Some(p) = rfd::FileDialog::new().add_filter("CSV", &["csv"]).pick_file() {
                                self.path_input = p.display().to_string(); self.load();
                            }
                        }
                        if ui.button("SPSS Statistics (.sav)...").clicked() { ui.close_menu();
                            if let Some(p) = rfd::FileDialog::new().add_filter("SPSS", &["sav"]).pick_file() {
                                self.path_input = p.display().to_string(); self.load();
                            }
                        }
                    });
                    ui.separator();
                    if ui.button("Save Data As...").clicked() { ui.close_menu();
                        if let Some(p) = rfd::FileDialog::new().add_filter("CSV", &["csv"]).save_file() {
                            self.export_data_csv_to(&p);
                        }
                    }
                    if ui.button("Export Output...").clicked() { ui.close_menu();
                        if let Some(p) = rfd::FileDialog::new().add_filter("Markdown", &["md"]).save_file() {
                            self.export_output_to(&p);
                        }
                    }
                    ui.separator();
                    if ui.button("Exit").clicked() { ui.close_menu(); std::process::exit(0); }
                });
                // ---------------- Edit ----------------
                ui.menu_button("Edit", |ui| {
                    if ui.button("Undo").clicked() { ui.close_menu(); self.status = "Undo: not implemented yet.".to_string(); }
                    if ui.button("Redo").clicked() { ui.close_menu(); self.status = "Redo: not implemented yet.".to_string(); }
                    ui.separator();
                    if ui.button("Clear Output").clicked() { ui.close_menu(); self.outputs.clear(); self.status = "Output cleared.".to_string(); }
                });
                // ---------------- View ----------------
                ui.menu_button("View", |ui| {
                    if ui.button("Data View").clicked() { ui.close_menu(); self.view = View::Data; }
                    if ui.button("Variable View").clicked() { ui.close_menu(); self.view = View::Variables; }
                    ui.separator();
                    if ui.checkbox(&mut self.grid_lines, "Grid Lines").clicked() { ui.close_menu(); }
                    if ui.checkbox(&mut self.show_status, "Status Bar").clicked() { ui.close_menu(); }
                    ui.separator();
                    if ui.button("Value Labels").clicked() { ui.close_menu(); self.status = "Value Labels: not implemented yet.".to_string(); }
                });
                // ---------------- Data ----------------
                ui.menu_button("Data", |ui| {
                    if ui.button("Define Variable Properties...").clicked() { ui.close_menu(); self.view = View::Variables; }
                    ui.separator();
                    ui.menu_button("Sort Cases", |ui| {
                        if self.table.is_none() {
                            ui.label("No data loaded.");
                        } else {
                            let names: Vec<String> = self.table.as_ref().unwrap().columns.iter().map(|c| c.name.clone()).collect();
                            for (i, name) in names.iter().enumerate() {
                                ui.menu_button(name, |ui| {
                                    if ui.button("Ascending").clicked() { ui.close_menu(); if let Some(t) = &mut self.table { t.sort_rows(i, true); self.status = format!("Sorted cases by {} (ascending).", name); } }
                                    if ui.button("Descending").clicked() { ui.close_menu(); if let Some(t) = &mut self.table { t.sort_rows(i, false); self.status = format!("Sorted cases by {} (descending).", name); } }
                                });
                            }
                        }
                    });
                    if ui.button("Sort Variables...").clicked() { ui.close_menu(); self.status = "Sort Variables: not implemented yet.".to_string(); }
                    ui.separator();
                    if ui.button("Split File...").clicked() { ui.close_menu(); self.status = "Split File: not implemented yet.".to_string(); }
                    if ui.button("Select Cases...").clicked() { ui.close_menu(); self.status = "Select Cases: not implemented yet.".to_string(); }
                    if ui.button("Weight Cases...").clicked() { ui.close_menu(); self.status = "Weight Cases: not implemented yet.".to_string(); }
                });
                // ---------------- Transform ----------------
                ui.menu_button("Transform", |ui| {
                    if ui.button("Compute Variable...").clicked() { ui.close_menu(); self.status = "Compute Variable: not implemented yet.".to_string(); }
                    if ui.button("Recode into Same Variables...").clicked() { ui.close_menu(); self.status = "Recode into Same Variables: not implemented yet.".to_string(); }
                    if ui.button("Recode into Different Variables...").clicked() { ui.close_menu(); self.status = "Recode into Different Variables: not implemented yet.".to_string(); }
                    if ui.button("Count Values within Cases...").clicked() { ui.close_menu(); self.status = "Count Values within Cases: not implemented yet.".to_string(); }
                    if ui.button("Rank Cases...").clicked() { ui.close_menu(); self.status = "Rank Cases: not implemented yet.".to_string(); }
                    if ui.button("Automatic Recode...").clicked() { ui.close_menu(); self.status = "Automatic Recode: not implemented yet.".to_string(); }
                });
                // ---------------- Analyze ----------------
                ui.menu_button("Analyze", |ui| {
                    ui.menu_button("Descriptive Statistics", |ui| {
                        if ui.button("Frequencies...").clicked() { ui.close_menu(); self.analyze_frequencies(); }
                        if ui.button("Descriptives...").clicked() { ui.close_menu(); self.analyze_descriptives(); }
                        if ui.button("Explore...").clicked() { ui.close_menu(); self.status = "Explore: not implemented yet.".to_string(); }
                        if ui.button("Crosstabs...").clicked() { ui.close_menu(); self.status = "Crosstabs: not implemented yet.".to_string(); }
                    });
                    ui.menu_button("Compare Means", |ui| {
                        if ui.button("Means...").clicked() { ui.close_menu(); self.status = "Means: not implemented yet.".to_string(); }
                        if ui.button("Independent-Samples T Test...").clicked() { ui.close_menu(); self.analyze_ttest(); }
                        if ui.button("Paired-Samples T Test...").clicked() { ui.close_menu(); self.status = "Paired-Samples T Test: not implemented yet.".to_string(); }
                        if ui.button("One-Way ANOVA...").clicked() { ui.close_menu(); self.status = "One-Way ANOVA: not implemented yet.".to_string(); }
                    });
                    ui.menu_button("Correlate", |ui| {
                        if ui.button("Bivariate...").clicked() { ui.close_menu(); self.analyze_correlations(); }
                    });
                    ui.menu_button("Regression", |ui| {
                        if ui.button("Linear...").clicked() { ui.close_menu(); self.analyze_regression(); }
                        if ui.button("Curve Estimation...").clicked() { ui.close_menu(); self.status = "Curve Estimation: not implemented yet.".to_string(); }
                    });
                    ui.separator();
                    if ui.button("Reports").clicked() { ui.close_menu(); self.status = "Reports: not implemented yet.".to_string(); }
                });
                // ---------------- Graphs ----------------
                ui.menu_button("Graphs", |ui| {
                    if ui.button("Chart Builder...").clicked() { ui.close_menu(); self.view = View::Charts; }
                });
                // ---------------- Utilities ----------------
                ui.menu_button("Utilities", |ui| {
                    if ui.button("Data File Comments").clicked() { ui.close_menu(); self.status = "Data File Comments: not implemented yet.".to_string(); }
                    if ui.button("Variable Information...").clicked() { ui.close_menu(); self.view = View::Variables; }
                    ui.separator();
                    if ui.button("Run Syntax").clicked() { ui.close_menu(); self.view = View::Syntax; }
                });
                // ---------------- Extensions ----------------
                ui.menu_button("Extensions", |ui| {
                    if ui.button("Extension Hub...").clicked() { ui.close_menu(); self.status = "Extension Hub: not available in this build.".to_string(); }
                    if ui.button("Custom Dialog Builder...").clicked() { ui.close_menu(); self.status = "Custom Dialog Builder: not available in this build.".to_string(); }
                });
                // ---------------- Window ----------------
                ui.menu_button("Window", |ui| {
                    if ui.button("Data Editor").clicked() { ui.close_menu(); self.view = View::Data; }
                    if ui.button("Output Viewer").clicked() { ui.close_menu(); self.view = View::Output; }
                    if ui.button("Syntax Editor").clicked() { ui.close_menu(); self.view = View::Syntax; }
                });
                // ---------------- Help ----------------
                ui.menu_button("Help", |ui| {
                    if ui.button("About StateCraft").clicked() { ui.close_menu(); self.status = "StateCraft - lightweight SPSS-like workbench (Rust).".to_string(); }
                });
            });
        });

        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            // ---- SPSS-style icon toolbar ----
            ui.horizontal(|ui| {
                if ui
                    .add(egui::Button::new(icon_btn("📂")).small())
                    .on_hover_text("Open data file (CSV / SPSS .sav)")
                    .clicked()
                {
                    if let Some(p) = rfd::FileDialog::new()
                        .add_filter("Data", &["csv", "sav"])
                        .pick_file()
                    {
                        self.path_input = p.display().to_string();
                        self.load();
                    }
                }
                if ui
                    .add(egui::Button::new(icon_btn("💾")).small())
                    .on_hover_text("Save data as CSV")
                    .clicked()
                {
                    if let Some(p) =
                        rfd::FileDialog::new().add_filter("CSV", &["csv"]).save_file()
                    {
                        self.export_data_csv_to(&p);
                    }
                }
                if ui
                    .add(egui::Button::new(icon_btn("🖨")).small())
                    .on_hover_text("Export output (Markdown)")
                    .clicked()
                {
                    if let Some(p) =
                        rfd::FileDialog::new().add_filter("Markdown", &["md"]).save_file()
                    {
                        self.export_output_to(&p);
                    }
                }
                ui.separator();
                if ui
                    .add(egui::Button::new(icon_btn("↶")).small())
                    .on_hover_text("Undo")
                    .clicked()
                {
                    self.status = "Undo: not implemented yet.".to_string();
                }
                if ui
                    .add(egui::Button::new(icon_btn("↷")).small())
                    .on_hover_text("Redo")
                    .clicked()
                {
                    self.status = "Redo: not implemented yet.".to_string();
                }
                ui.separator();
                if ui
                    .add(egui::Button::new(icon_btn("⊞")).small())
                    .on_hover_text("Insert Variable")
                    .clicked()
                {
                    self.status = "Insert Variable: not implemented yet.".to_string();
                }
                if ui
                    .add(egui::Button::new(icon_btn("⬇")).small())
                    .on_hover_text("Insert Cases")
                    .clicked()
                {
                    self.status = "Insert Cases: not implemented yet.".to_string();
                }
                if ui
                    .add(egui::Button::new(icon_btn("🏷")).small())
                    .on_hover_text("Value Labels")
                    .clicked()
                {
                    self.status = "Value Labels: not implemented yet.".to_string();
                }
                ui.separator();
                if ui
                    .add(egui::Button::new(icon_btn("🔍")).small())
                    .on_hover_text("Go to Variable (Variable View)")
                    .clicked()
                {
                    self.view = View::Variables;
                }
                if ui
                    .add(egui::Button::new(icon_btn("📊")).small())
                    .on_hover_text("Chart Builder")
                    .clicked()
                {
                    self.view = View::Charts;
                }
            });
            ui.add_space(2.0);
            // ---- SPSS "Visible: X of Y Variables" filter row ----
            ui.horizontal(|ui| {
                let label_w = 150.0;
                let box_w = (ui.available_width() - label_w).max(100.0);
                ui.add(
                    egui::TextEdit::singleline(&mut self.var_filter)
                        .desired_width(box_w)
                        .frame(true),
                );
                let f = self.var_filter.to_lowercase();
                let total = self.table.as_ref().map(|t| t.columns.len()).unwrap_or(0);
                let visible = self
                    .table
                    .as_ref()
                    .map(|t| {
                        t.columns
                            .iter()
                            .filter(|c| f.is_empty() || c.name.to_lowercase().contains(&f))
                            .count()
                    })
                    .unwrap_or(0);
                ui.label(format!("Visible: {} of {} Variables", visible, total));
            });
        });

        // ---- SPSS bottom-left document tabs (Data View / Variable View) ----
        egui::TopBottomPanel::bottom("tabs").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.add_space(4.0);
                for (label, v) in
                    [("Data View", View::Data), ("Variable View", View::Variables)]
                {
                    let sel = self.view == v || (v == View::Data && self.view != View::Variables && self.view != View::Output && self.view != View::Charts && self.view != View::Syntax);
                    let fill = if sel {
                        egui::Color32::WHITE
                    } else {
                        egui::Color32::from_rgb(0xD9, 0xD9, 0xD9)
                    };
                    let mut txt = egui::RichText::new(label).size(13.0);
                    if sel {
                        txt = txt.strong();
                    }
                    if ui
                        .add(egui::Button::new(txt).fill(fill).small())
                        .clicked()
                    {
                        self.view = v;
                    }
                }
            });
        });

        // ---- SPSS status bar: gray strip, ready + Unicode on the right ----
        if self.show_status {
            egui::TopBottomPanel::bottom("statusbar").show(ctx, |ui| {
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(0xEA, 0xEA, 0xEA))
                    .inner_margin(egui::Margin {
                        left: 8.0,
                        right: 8.0,
                        top: 3.0,
                        bottom: 3.0,
                    })
                    .show(ui, |ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label("Unicode: ON");
                            ui.separator();
                            ui.label(
                                egui::RichText::new("✔")
                                    .color(egui::Color32::from_rgb(0x1E, 0x9E, 0x3A)),
                            );
                            ui.label("StateCraft Statistics Processor is ready");
                            ui.label(&self.status);
                        });
                    });
            });
        }

        egui::CentralPanel::default().show(ctx, |ui| match self.view {
            View::Data => {
                let vis: Vec<usize> = match &self.table {
                    Some(table) => {
                        let f = self.var_filter.to_lowercase();
                        let vis: Vec<usize> = table
                            .columns
                            .iter()
                            .enumerate()
                            .filter(|(_, c)| f.is_empty() || c.name.to_lowercase().contains(&f))
                            .map(|(i, _)| i)
                            .collect();
                        spss_grid(ui, Some(table), &vis);
                        vis
                    }
                    None => {
                        spss_grid(ui, None, &[]);
                        Vec::new()
                    }
                };
                let _ = vis;
            }
            View::Variables => {
                if let Some(table) = &self.table {
                    render_variables(ui, table);
                } else {
                    ui.label("No data loaded.");
                }
            }
            View::Output => {
                if self.outputs.is_empty() {
                    ui.label("No output yet. Run an analysis.");
                } else {
                    if self.selected_output >= self.outputs.len() {
                        self.selected_output = 0;
                    }
                    ui.columns(2, |cols| {
                        // left outline
                        cols[0].group(|ui| {
                            ui.label("Output Outline");
                            egui::ScrollArea::vertical().show(ui, |ui| {
                                for (i, t) in self.outputs.iter().enumerate() {
                                    if ui.selectable_label(self.selected_output == i, &t.title).clicked() {
                                        self.selected_output = i;
                                    }
                                }
                            });
                        });
                        // right content
                        cols[1].group(|ui| {
                            let t = &self.outputs[self.selected_output];
                            ui.heading(&t.title);
                            render_output_table(ui, t);
                        });
                    });
                }
            }
            View::Charts => {
                if let Some(table) = &self.table {
                    render_charts(ui, table);
                } else {
                    ui.label("No data loaded.");
                }
            }
            View::Syntax => {
                ui.vertical(|ui| {
                    ui.label("SPSS-like syntax (LOAD/DESCRIPTIVES/FREQUENCIES/CORRELATIONS/REGRESSION/TTEST/CLEAR):");
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut self.syntax_text)
                                .code_editor()
                                .desired_rows(20)
                                .desired_width(f32::INFINITY),
                        );
                    });
                    if ui.button("Run").clicked() {
                        crate::syntax::run(self, &self.syntax_text.clone());
                    }
                });
            }
        });
    }
}

fn icon_btn(glyph: &str) -> egui::RichText {
    egui::RichText::new(glyph).size(15.0)
}

const SPSS_ROW_H: f32 = 19.0;
const SPSS_ROWNUM_W: f32 = 48.0;
const SPSS_COL_W: f32 = 82.0;
const SPSS_GAP: f32 = 2.5;
const SPSS_MIN_COLS: usize = 20;

/// SPSS Data Editor look: light-gray cell blocks separated by white gaps,
/// a white row-number gutter, and `var` placeholder headers beyond real columns.
/// Row 0 of the virtual list is the header band.
fn spss_grid(ui: &mut egui::Ui, table: Option<&DataTable>, vis_cols: &[usize]) {
    let n_data_rows = table.map(|t| t.n_rows.min(999_999)).unwrap_or(0);
    let n_real = vis_cols.len();
    let total_cols = n_real.max(SPSS_MIN_COLS);
    let total_rows = 1 + n_data_rows.max(40);
    let content_w = SPSS_ROWNUM_W + SPSS_GAP + (total_cols as f32) * (SPSS_COL_W + SPSS_GAP);

    let header_fill = egui::Color32::from_rgb(0xF3, 0xF3, 0xF3);
    let cell_fill = egui::Color32::from_rgb(0xF6, 0xF6, 0xF6);
    let header_text = egui::Color32::from_rgb(0x33, 0x33, 0x33);
    let placeholder_text = egui::Color32::from_rgb(0x9B, 0x9B, 0x9B);
    let value_text = egui::Color32::from_rgb(0x1E, 0x1E, 0x1E);
    let rownum_text = egui::Color32::from_rgb(0x66, 0x66, 0x66);

    let font = egui::FontId::proportional(12.5);

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show_rows(ui, SPSS_ROW_H + SPSS_GAP, total_rows, |ui, rows| {
            for row in rows {
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(content_w, SPSS_ROW_H), egui::Sense::hover());
                let p = ui.painter();
                let mid_y = rect.center().y;
                let cell_x = |j: usize| rect.left() + SPSS_ROWNUM_W + SPSS_GAP + (j as f32) * (SPSS_COL_W + SPSS_GAP);

                if row == 0 {
                    // header band
                    for j in 0..total_cols {
                        let x0 = cell_x(j);
                        let r = egui::Rect::from_min_max(
                            egui::pos2(x0, rect.top()),
                            egui::pos2(x0 + SPSS_COL_W, rect.bottom()),
                        );
                        p.rect_filled(r, 0.0, header_fill);
                        let (txt, color) = match table.and_then(|t| vis_cols.get(j).map(|&ci| &t.columns[ci].name)) {
                            Some(name) => (name.as_str(), header_text),
                            None => ("var", placeholder_text),
                        };
                        p.text(
                            r.center(),
                            egui::Align2::CENTER_CENTER,
                            txt,
                            font.clone(),
                            color,
                        );
                    }
                } else {
                    // data row
                    let di = row - 1;
                    p.text(
                        egui::pos2(rect.left() + 7.0, mid_y),
                        egui::Align2::LEFT_CENTER,
                        format!("{}", di + 1),
                        font.clone(),
                        rownum_text,
                    );
                    if let Some(t) = table {
                        for (j, &ci) in vis_cols.iter().enumerate() {
                            let x0 = cell_x(j);
                            let r = egui::Rect::from_min_max(
                                egui::pos2(x0, rect.top() + 1.0),
                                egui::pos2(x0 + SPSS_COL_W, rect.bottom() - 1.0),
                            );
                            p.rect_filled(r, 0.0, cell_fill);
                            let is_num =
                                t.columns[ci].kind == crate::data::ColumnKind::Numeric;
                            let anchor = if is_num {
                                egui::Align2::RIGHT_CENTER
                            } else {
                                egui::Align2::LEFT_CENTER
                            };
                            let px = if is_num { r.right() - 6.0 } else { r.left() + 6.0 };
                            let txt = t.cell_text(ci, di);
                            if !txt.is_empty() {
                                p.text(egui::pos2(px, mid_y), anchor, txt, font.clone(), value_text);
                            }
                        }
                    }
                }
            }
        });
}

fn render_output_table(ui: &mut egui::Ui, t: &OutputTable) {
    egui::Grid::new(&t.title).striped(true).show(ui, |ui| {
        for h in &t.headers {
            ui.strong(h);
        }
        ui.end_row();
        for row in &t.rows {
            for cell in row {
                ui.label(cell);
            }
            ui.end_row();
        }
    });
}

fn render_charts(ui: &mut egui::Ui, table: &DataTable) {
    let nums: Vec<usize> = table
        .columns
        .iter()
        .enumerate()
        .filter(|(_, c)| c.kind == crate::data::ColumnKind::Numeric)
        .map(|(i, _)| i)
        .collect();
    if nums.is_empty() {
        ui.label("No numeric columns.");
        return;
    }
    // scatter of first two numeric columns
    if nums.len() >= 2 {
        let cx = &table.columns[nums[0]];
        let cy = &table.columns[nums[1]];
        let mut pts = Vec::new();
        for (a, b) in cx.values.iter().zip(cy.values.iter()) {
            if let (crate::data::Value::Number(x), crate::data::Value::Number(y)) = (a, b) {
                pts.push([*x, *y]);
            }
        }
        ui.label(format!("Scatter: {} vs {}", cx.name, cy.name));
        Plot::new("scatter").view_aspect(1.5).show(ui, |plot_ui| {
            plot_ui.points(egui_plot::Points::new(PlotPoints::new(pts)));
        });
    }
    // histogram of first numeric column
    let c = &table.columns[nums[0]];
    let mut vals: Vec<f64> = c
        .values
        .iter()
        .filter_map(|v| match v {
            crate::data::Value::Number(x) => Some(*x),
            _ => None,
        })
        .collect();
    if !vals.is_empty() {
        vals.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let (min, max) = (vals[0], vals[vals.len() - 1]);
        let bins = 12;
        let width = (max - min) / bins as f64;
        let mut counts = vec![0.0; bins];
        for v in &vals {
            let mut idx = ((v - min) / width).floor() as usize;
            if idx >= bins {
                idx = bins - 1;
            }
            counts[idx] += 1.0;
        }
        let bars: Vec<egui_plot::Bar> = counts
            .into_iter()
            .enumerate()
            .map(|(i, h)| {
                egui_plot::Bar::new(min + i as f64 * width + width / 2.0, h).width(width * 0.9)
            })
            .collect();
        ui.label(format!("Histogram: {}", c.name));
        Plot::new("hist").show(ui, |plot_ui| {
            plot_ui.bar_chart(egui_plot::BarChart::new(bars));
        });
    }
}

fn render_variables(ui: &mut egui::Ui, table: &DataTable) {
    let header_color = egui::Color32::from_rgb(0x6E, 0x6E, 0x6E);
    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("var_grid")
                .spacing([12.0, 2.0])
                .striped(false)
                .show(ui, |ui| {
                    // SPSS Variable View column set:
                    // Name Type Width Decimals Label Values Missing Columns Align Measure Role
                    for h in ["Name", "Type", "Width", "Decimals", "Label", "Values",
                              "Missing", "Columns", "Align", "Measure", "Role"] {
                        ui.label(
                            egui::RichText::new(h)
                                .strong()
                                .color(header_color),
                        );
                    }
                    ui.end_row();
                    for c in &table.columns {
                        let missing = c
                            .values
                            .iter()
                            .filter(|v| matches!(v, crate::data::Value::Missing))
                            .count();
                        let (typ, width, decimals, align, measure) = match c.kind {
                            crate::data::ColumnKind::Numeric => ("Numeric", "8", "2", "Right", "Scale"),
                            crate::data::ColumnKind::String => ("String", "8", "0", "Left", "Nominal"),
                        };
                        ui.label(&c.name);
                        ui.label(typ);
                        ui.label(width);
                        ui.label(decimals);
                        ui.label(&c.name); // Label defaults to the variable name
                        ui.label("None");
                        ui.label(if missing > 0 { missing.to_string() } else { "None".to_string() });
                        ui.label("8");
                        ui.label(align);
                        ui.label(measure);
                        ui.label("Input");
                        ui.end_row();
                    }
                });
        });
}

pub fn run(initial_path: Option<String>) -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "StateCraft",
        options,
        Box::new(move |_cc| {
            let mut app = StateCraftApp::default();
            if let Some(p) = initial_path {
                app.path_input = p;
            }
            Ok(Box::new(app))
        }),
    )
}
