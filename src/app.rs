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
    /// Active (column, row) in Data View, like the SPSS active cell.
    pub sel_cell: Option<(usize, usize)>,
    /// Cell currently being edited.
    pub editing_cell: Option<(usize, usize)>,
    pub edit_buf: String,
    /// Phase 4: separate OS document windows (Output Viewer / Syntax editor).
    pub show_output_window: bool,
    pub show_syntax_window: bool,
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
            sel_cell: None,
            editing_cell: None,
            edit_buf: String::new(),
            show_output_window: false,
            show_syntax_window: false,
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

    /// Phase 2 pipeline: menus/dialogs generate SPSS syntax text; the single
    /// syntax runner executes it; results appear in the Output Viewer.
    pub fn paste_and_run(&mut self, cmd: String) {
        self.status = format!("Pasted: {}", cmd);
        crate::syntax::run(self, &cmd);
        // SPSS raises the Output Viewer document window
        if !self.outputs.is_empty() {
            self.show_output_window = true;
        }
    }

    fn numeric_var_names(&self) -> Vec<String> {
        self.table
            .as_ref()
            .map(|t| {
                t.columns
                    .iter()
                    .filter(|c| crate::data::ColumnKind::Numeric == c.kind)
                    .map(|c| c.name.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn analyze_descriptives(&mut self) {
        let names = self.numeric_var_names();
        if self.table.is_none() {
            self.status = "No data loaded.".to_string();
            return;
        }
        if names.is_empty() {
            self.status = "No numeric variables to analyze.".to_string();
            return;
        }
        self.paste_and_run(format!("DESCRIPTIVES {}", names.join(" ")));
    }

    pub fn analyze_frequencies(&mut self) {
        if self.table.is_none() {
            self.status = "No data loaded.".to_string();
            return;
        }
        self.paste_and_run("FREQUENCIES".to_string());
    }

    pub fn analyze_correlations(&mut self) {
        let names = self.numeric_var_names();
        if self.table.is_none() {
            self.status = "No data loaded.".to_string();
            return;
        }
        if names.len() < 2 {
            self.status = "CORRELATIONS needs two or more numeric variables.".to_string();
            return;
        }
        self.paste_and_run(format!("CORRELATIONS {}", names.join(" ")));
    }

    pub fn analyze_regression(&mut self) {
        if let Some(t) = &self.table {
            let nums: Vec<usize> = t
                .columns
                .iter()
                .enumerate()
                .filter(|(_, c)| c.kind == crate::data::ColumnKind::Numeric)
                .map(|(i, _)| i)
                .collect();
            if nums.len() < 2 {
                self.status = "Need at least 2 numeric columns.".to_string();
            } else {
                let (y, x) = (t.columns[nums[0]].name.clone(), t.columns[nums[1]].name.clone());
                self.paste_and_run(format!("REGRESSION {} WITH {}", y, x));
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
            if !t.note.is_empty() {
                md.push_str(&format!("*{}*\n\n", t.note));
            }
        }
        match std::fs::write(path, md) {
            Ok(_) => self.status = format!("Exported output to {}", path.display()),
            Err(e) => self.status = format!("Export failed: {}", e),
        }
    }

    /// SPSS-like pivot table exported as a styled HTML document.
    pub fn export_output_html_to(&mut self, path: &std::path::Path) {
        if self.outputs.is_empty() {
            self.status = "No output to export.".to_string();
            return;
        }
        let mut h = String::from(
            "<!DOCTYPE html>\n<html>\n<head>\n<meta charset=\"utf-8\">\n\
             <title>StateCraft Output</title>\n<style>\n\
             body{font-family:'Segoe UI',Arial,sans-serif;margin:24px;}\n\
             h2{font-size:14pt;margin-bottom:4px;}\n\
             table{border-collapse:collapse;font-size:10.5pt;margin-bottom:8px;}\n\
             th{background:#e8eef7;border:1px solid #8a8a8a;padding:3px 10px;text-align:left;}\n\
             td{border:1px solid #b5b5b5;padding:2px 10px;}\n\
             td.num{text-align:right;font-variant-numeric:tabular-nums;}\n\
             td.stub{font-weight:600;background:#f6f8fb;}\n\
             .note{color:#555;font-size:9.5pt;}\n\
             </style>\n</head>\n<body>\n",
        );
        for t in &self.outputs {
            h.push_str(&format!(
                "<h2>{}</h2>\n<table>\n<tr>",
                html_escape(&t.title)
            ));
            for hdr in &t.headers {
                h.push_str(&format!("<th>{}</th>", html_escape(hdr)));
            }
            h.push_str("</tr>\n");
            for row in &t.rows {
                h.push_str("<tr>");
                for (k, cell) in row.iter().enumerate() {
                    let esc = html_escape(cell);
                    if k > 0 && cell.trim().parse::<f64>().is_ok() {
                        h.push_str(&format!("<td class=\"num\">{}</td>", esc));
                    } else if k == 0 {
                        h.push_str(&format!("<td class=\"stub\">{}</td>", esc));
                    } else {
                        h.push_str(&format!("<td>{}</td>", esc));
                    }
                }
                h.push_str("</tr>\n");
            }
            h.push_str("</table>\n");
            if !t.note.is_empty() {
                h.push_str(&format!("<p class=\"note\">{}</p>\n", html_escape(&t.note)));
            }
        }
        h.push_str("</body>\n</html>\n");
        match std::fs::write(path, h) {
            Ok(_) => self.status = format!("Exported output (HTML) to {}", path.display()),
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

    /// Crosstabs: pick two low-cardinality variables, prefer categorical ones.
    pub fn analyze_crosstabs(&mut self) {
        if let Some(t) = &self.table {
            let uniq = |ci: usize| -> usize {
                let mut set = std::collections::BTreeSet::new();
                for r in 0..t.n_rows {
                    if !t.is_missing(ci, r) {
                        set.insert(t.cell_text(ci, r));
                    }
                }
                set.len()
            };
            let cat: Vec<usize> = (0..t.columns.len())
                .filter(|&ci| {
                    t.columns[ci].kind != crate::data::ColumnKind::Numeric
                        || uniq(ci) <= 12
                })
                .collect();
            let pair = cat
                .iter()
                .enumerate()
                .flat_map(|(a, &i)| cat.iter().skip(a + 1).map(move |&j| (i, j)))
                .find(|(i, j)| *i != *j);
            if let Some((i, j)) = pair {
                let cmd = format!(
                    "CROSSTABS {} BY {}",
                    t.columns[i].name, t.columns[j].name
                );
                self.paste_and_run(cmd);
            } else {
                self.status = "Crosstabs: need two categorical (or low-cardinality) variables.".to_string();
            }
        } else {
            self.status = "No data loaded.".to_string();
        }
    }

    /// One-Way ANOVA: first numeric outcome, first valid grouping variable.
    pub fn analyze_anova(&mut self) {
        if let Some(t) = &self.table {
            let outcome = t.columns.iter().position(|c| c.kind == crate::data::ColumnKind::Numeric);
            let factor = (0..t.columns.len()).find(|&ci| {
                let mut uniq = std::collections::BTreeSet::new();
                for r in 0..t.n_rows {
                    if !t.is_missing(ci, r) {
                        uniq.insert(t.cell_text(ci, r));
                    }
                }
                (2..=8).contains(&uniq.len())
            });
            match (outcome, factor) {
                (Some(y), Some(g)) if y != g => {
                    let cmd = format!("ONEWAY {} BY {}", t.columns[y].name, t.columns[g].name);
                    self.paste_and_run(cmd);
                }
                _ => self.status = "ANOVA: need a numeric outcome and a 2..8-level factor.".to_string(),
            }
        } else {
            self.status = "No data loaded.".to_string();
        }
    }

    /// Paired-samples t-test: first two numeric variables.
    pub fn analyze_paired_t(&mut self) {
        let names = self.numeric_var_names();
        if self.table.is_none() {
            self.status = "No data loaded.".to_string();
            return;
        }
        if names.len() < 2 {
            self.status = "Paired t-test: need two numeric variables.".to_string();
            return;
        }
        self.paste_and_run(format!("T-TEST PAIRED {} {}", names[0], names[1]));
    }

    pub fn analyze_ttest(&mut self) {
        // pick the same defaults the procedure dialog would: first 2-level
        // string variable as group, first numeric as outcome -> paste & run
        if let Some(t) = &self.table {
            let group = t.columns.iter().find(|c| {
                if c.kind != crate::data::ColumnKind::String {
                    return false;
                }
                let mut uniq = std::collections::BTreeSet::new();
                for v in &c.values {
                    if let crate::data::Value::Text(s) = v {
                        uniq.insert(s.clone());
                    }
                }
                uniq.len() == 2
            });
            let outcome = t.columns.iter().find(|c| c.kind == crate::data::ColumnKind::Numeric);
            match (outcome, group) {
                (Some(o), Some(g)) => {
                    let cmd = format!("T-TEST {} BY {}", o.name, g.name);
                    self.paste_and_run(cmd);
                }
                _ => self.status = "Need a 2-level group and numeric outcome.".to_string(),
            }
        } else {
            self.status = "No data loaded.".to_string();
        }
    }
}

impl eframe::App for StateCraftApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // ---- Phase 4: real OS document windows (SPSS multi-document UX) ----
        // The Output Viewer and Syntax editor live in their own viewports;
        // the Data Editor stays the main window, like IBM SPSS.
        if self.show_output_window {
            ctx.show_viewport_immediate(
                egui::ViewportId::from_hash_of("statecraft_output"),
                egui::ViewportBuilder::default()
                    .with_title(format!("Output1 - StateCraft Output Viewer"))
                    .with_inner_size([780.0, 560.0]),
                |vctx, _class| {
                    egui::CentralPanel::default()
                        .frame(egui::Frame::none().fill(egui::Color32::WHITE).inner_margin(egui::Margin::same(8.0)))
                        .show(vctx, |ui| {
                            egui::TopBottomPanel::top("ov_title")
                                .frame(egui::Frame::none().inner_margin(egui::Margin::same(3.0)))
                                .show_inside(ui, |ui| {
                                    ui.heading(egui::RichText::new("Output1").strong());
                                    ui.separator();
                                });
                            if self.outputs.is_empty() {
                                ui.label("No output yet. Run an analysis from the Data Editor menus.");
                                return;
                            }
                            egui::ScrollArea::vertical()
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    for t in &self.outputs {
                                        ui.add_space(6.0);
                                        ui.heading(egui::RichText::new(&t.title).strong());
                                        ui.add_space(4.0);
                                        render_output_table(ui, t);
                                    }
                                });
                        });
                    // closing the window hides it until re-opened from Window menu
                    if vctx.input(|i| i.viewport().close_requested()) {
                        vctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                        self.show_output_window = false;
                    }
                },
            );
        }
        if self.show_syntax_window {
            ctx.show_viewport_immediate(
                egui::ViewportId::from_hash_of("statecraft_syntax"),
                egui::ViewportBuilder::default()
                    .with_title(format!("Syntax1 - StateCraft Syntax Editor"))
                    .with_inner_size([640.0, 440.0]),
                |vctx, _class| {
                    egui::CentralPanel::default()
                        .frame(egui::Frame::none().fill(egui::Color32::WHITE).inner_margin(egui::Margin::same(8.0)))
                        .show(vctx, |ui| {
                            egui::TopBottomPanel::top("sx_title")
                                .frame(egui::Frame::none().inner_margin(egui::Margin::same(3.0)))
                                .show_inside(ui, |ui| {
                                    ui.heading(egui::RichText::new("Syntax1").strong());
                                    ui.separator();
                                });
                            egui::ScrollArea::vertical().show(ui, |ui| {
                                ui.add(
                                    egui::TextEdit::multiline(&mut self.syntax_text)
                                        .code_editor()
                                        .desired_rows(16)
                                        .desired_width(f32::INFINITY),
                                );
                            });
                            ui.add_space(6.0);
                            if ui.button("Run").clicked() {
                                let script = self.syntax_text.clone();
                                crate::syntax::run(self, &script);
                            }
                        });
                    if vctx.input(|i| i.viewport().close_requested()) {
                        vctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                        self.show_syntax_window = false;
                    }
                },
            );
        }

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
                    if ui.button("Export Output (Markdown)...").clicked() { ui.close_menu();
                        if let Some(p) = rfd::FileDialog::new().add_filter("Markdown", &["md"]).save_file() {
                            self.export_output_to(&p);
                        }
                    }
                    if ui.button("Export Output (HTML)...").clicked() { ui.close_menu();
                        if let Some(p) = rfd::FileDialog::new().add_filter("HTML", &["html", "htm"]).save_file() {
                            self.export_output_html_to(&p);
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
                                    if ui.button("Ascending").clicked() { ui.close_menu(); let cmd = format!("SORT CASES BY {} (A)", name); self.paste_and_run(cmd); }
                                    if ui.button("Descending").clicked() { ui.close_menu(); let cmd = format!("SORT CASES BY {} (D)", name); self.paste_and_run(cmd); }
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
                        if ui.button("Crosstabs...").clicked() { ui.close_menu(); self.analyze_crosstabs(); }
                    });
                    ui.menu_button("Compare Means", |ui| {
                        if ui.button("Means...").clicked() { ui.close_menu(); self.status = "Means: not implemented yet.".to_string(); }
                        if ui.button("Independent-Samples T Test...").clicked() { ui.close_menu(); self.analyze_ttest(); }
                        if ui.button("Paired-Samples T Test...").clicked() { ui.close_menu(); self.analyze_paired_t(); }
                        if ui.button("One-Way ANOVA...").clicked() { ui.close_menu(); self.analyze_anova(); }
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
                    if ui.button("Show Output1").clicked() { ui.close_menu(); self.show_output_window = true; }
                    if ui.button("Show Syntax1").clicked() { ui.close_menu(); self.show_syntax_window = true; }
                    if ui.button("Reset Windows").clicked() { ui.close_menu(); self.show_output_window = false; self.show_syntax_window = false; self.view = View::Data; }
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
                if icon_button(ui, Icon::Open, "Open data file (CSV / SPSS .sav)")
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
                if icon_button(ui, Icon::Save, "Save data as CSV").clicked() {
                    if let Some(p) =
                        rfd::FileDialog::new().add_filter("CSV", &["csv"]).save_file()
                    {
                        self.export_data_csv_to(&p);
                    }
                }
                if icon_button(ui, Icon::Print, "Export output (Markdown)").clicked() {
                    if let Some(p) =
                        rfd::FileDialog::new().add_filter("Markdown", &["md"]).save_file()
                    {
                        self.export_output_to(&p);
                    }
                }
                ui.separator();
                if icon_button(ui, Icon::Undo, "Undo").clicked() {
                    self.status = "Undo: not implemented yet.".to_string();
                }
                if icon_button(ui, Icon::Redo, "Redo").clicked() {
                    self.status = "Redo: not implemented yet.".to_string();
                }
                ui.separator();
                if icon_button(ui, Icon::InsVar, "Insert Variable").clicked() {
                    self.status = "Insert Variable: not implemented yet.".to_string();
                }
                if icon_button(ui, Icon::InsCase, "Insert Cases").clicked() {
                    self.status = "Insert Cases: not implemented yet.".to_string();
                }
                if icon_button(ui, Icon::Labels, "Value Labels").clicked() {
                    self.status = "Value Labels: not implemented yet.".to_string();
                }
                ui.separator();
                if icon_button(ui, Icon::Find, "Go to Variable (Variable View)").clicked() {
                    self.view = View::Variables;
                }
                if icon_button(ui, Icon::Chart, "Chart Builder").clicked() {
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
                                egui::RichText::new("âœ”")
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
                        table
                            .columns
                            .iter()
                            .enumerate()
                            .filter(|(_, c)| f.is_empty() || c.name.to_lowercase().contains(&f))
                            .map(|(i, _)| i)
                            .collect()
                    }
                    None => Vec::new(),
                };
                let StateCraftApp {
                    table,
                    sel_cell,
                    editing_cell,
                    edit_buf,
                    ..
                } = self;
                spss_grid(ui, table.as_mut(), &vis, sel_cell, editing_cell, edit_buf);
            }
            View::Variables => {
                if let Some(table) = &mut self.table {
                    render_variables(ui, table);
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
                        // left outline (SPSS Output Viewer tree)
                        cols[0].group(|ui| {
                            ui.label(
                                egui::RichText::new("Outline")
                                    .strong()
                                    .color(egui::Color32::from_rgb(0x55, 0x55, 0x55)),
                            );
                            egui::ScrollArea::vertical().show(ui, |ui| {
                                for (i, t) in self.outputs.iter().enumerate() {
                                    let label = format!("▤  {}", t.title);
                                    if ui
                                        .selectable_label(self.selected_output == i, label)
                                        .clicked()
                                    {
                                        self.selected_output = i;
                                    }
                                }
                            });
                        });
                        // right content: SPSS pivot-table display
                        cols[1].group(|ui| {
                            egui::ScrollArea::vertical()
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    ui.add_space(2.0);
                                    ui.heading(egui::RichText::new(&self.outputs[self.selected_output].title));
                                    ui.add_space(4.0);
                                    render_output_table(ui, &self.outputs[self.selected_output]);
                                });
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

/// Small vector-drawn SPSS-like toolbar icons (no emoji/glyph font dependency).

/// SPSS-like vector-drawn toolbar icons (no emoji/glyph font dependency).
#[derive(Clone, Copy, PartialEq)]
enum Icon {
    Open,
    Save,
    Print,
    Undo,
    Redo,
    InsVar,
    InsCase,
    Labels,
    Find,
    Chart,
}

/// Flat SPSS-style toolbar button: invisible chrome, subtle tint on hover.
fn icon_button(ui: &mut egui::Ui, icon: Icon, tip: &str) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(30.0, 26.0), egui::Sense::click());
    let p = ui.painter();
    if resp.hovered() || resp.is_pointer_button_down_on() {
        p.rect_filled(rect.shrink(1.5), 2.0, egui::Color32::from_rgb(0xDC, 0xE6, 0xF2));
    }
    let col = if resp.hovered() {
        egui::Color32::from_rgb(0x2E, 0x6D, 0xC8)
    } else {
        egui::Color32::from_rgb(0x40, 0x40, 0x40)
    };
    let st = egui::Stroke::new(1.4, col);
    let fill = egui::Color32::from_rgb(0xC9, 0xD6, 0xEA);
    let m = rect.center();
    let r = |x0: f32, y0: f32, x1: f32, y1: f32| egui::Rect::from_min_max(
        egui::pos2(m.x + x0, m.y + y0),
        egui::pos2(m.x + x1, m.y + y1),
    );
    let seg = |a: egui::Pos2, b: egui::Pos2| p.line_segment([a, b], st);
    match icon {
        Icon::Open => {
            // folder: back flap + body
            p.rect_filled(r(-10.0, -4.0, 9.0, 6.0), 1.0, fill);
            p.add(egui::Shape::convex_polygon(
                vec![
                    egui::pos2(m.x - 10.0, m.y - 4.0),
                    egui::pos2(m.x - 7.0, m.y - 7.5),
                    egui::pos2(m.x + 1.0, m.y - 7.5),
                    egui::pos2(m.x + 4.0, m.y - 4.0),
                ],
                col,
                egui::Stroke::NONE,
            ));
        }
        Icon::Save => {
            // floppy disk
            p.rect_stroke(r(-8.5, -8.0, 8.5, 7.0), 1.0, st);
            p.rect_filled(r(2.0, -8.0, 8.5, -2.0), 0.0, col);
            p.rect_stroke(r(-4.5, 0.5, 4.5, 7.0), 0.0, st);
        }
        Icon::Print => {
            p.rect_filled(r(-5.0, -8.0, 5.0, -3.0), 0.0, fill); // paper
            p.rect_stroke(r(-9.0, -3.0, 9.0, 5.0), 0.0, st); // body
            p.rect_stroke(r(-5.0, 2.0, 5.0, 7.5), 0.0, st); // output
        }
        Icon::Undo | Icon::Redo => {
            let sign = if icon == Icon::Undo { 1.0 } else { -1.0 };
            let cx = m.x + 1.0 * sign;
            let cy = m.y;
            // arc above the bar
            let mut pts = Vec::new();
            for k in 0..=8 {
                let t = std::f32::consts::PI * (k as f32) / 8.0;
                pts.push(egui::pos2(cx - 6.5 * t.cos() * sign, cy - 5.0 * t.sin() + 2.0));
            }
            p.add(egui::Shape::line(pts, st));
            // arrowhead at the left end
            let tip = egui::pos2(cx - 6.5 * sign, cy + 2.0);
            seg(
                tip,
                egui::pos2(tip.x + 4.0 * sign, tip.y - 1.0),
            );
            seg(
                tip,
                egui::pos2(tip.x + 3.5 * sign, tip.y + 3.5),
            );
        }
        Icon::InsVar => {
            // two existing columns + highlighted inserted column
            p.rect_stroke(r(-10.0, -6.0, -4.5, 6.0), 0.0, st);
            p.rect_stroke(r(4.5, -6.0, 10.0, 6.0), 0.0, st);
            p.rect_filled(r(-2.5, -6.0, 2.5, 6.0), 0.0, fill);
        }
        Icon::InsCase => {
            p.rect_stroke(r(-7.0, -8.0, 7.0, -3.0), 0.0, st);
            p.rect_stroke(r(-7.0, 3.0, 7.0, 8.0), 0.0, st);
            p.rect_filled(r(-7.0, -1.0, 7.0, 1.0), 0.0, fill);
        }
        Icon::Labels => {
            // value-label tag
            p.add(egui::Shape::convex_polygon(
                vec![
                    egui::pos2(m.x - 8.0, m.y - 4.0),
                    egui::pos2(m.x + 1.0, m.y - 4.0),
                    egui::pos2(m.x + 8.0, m.y),
                    egui::pos2(m.x + 1.0, m.y + 4.0),
                    egui::pos2(m.x - 8.0, m.y + 4.0),
                ],
                fill,
                st,
            ));
            p.circle_filled(egui::pos2(m.x - 4.0, m.y), 1.6, col);
        }
        Icon::Find => {
            p.circle_stroke(egui::pos2(m.x - 1.5, m.y - 1.5), 5.0, st);
            seg(
                egui::pos2(m.x + 2.0, m.y + 2.0),
                egui::pos2(m.x + 7.5, m.y + 7.5),
            );
        }
        Icon::Chart => {
            // axes
            seg(
                egui::pos2(m.x - 9.0, m.y - 8.0),
                egui::pos2(m.x - 9.0, m.y + 8.0),
            );
            seg(
                egui::pos2(m.x - 9.0, m.y + 8.0),
                egui::pos2(m.x + 10.0, m.y + 8.0),
            );
            p.rect_filled(r(-6.0, 0.0, -2.0, 8.0), 0.0, fill);
            p.rect_filled(r(-0.5, -4.0, 3.5, 8.0), 0.0, fill);
            p.rect_filled(r(5.0, -7.0, 9.0, 8.0), 0.0, fill);
        }
    }
    resp.on_hover_text(tip)
}

const SPSS_ROW_H: f32 = 19.0;
const SPSS_ROWNUM_W: f32 = 48.0;
const SPSS_COL_W: f32 = 82.0;
const SPSS_GAP: f32 = 2.5;
const SPSS_MIN_COLS: usize = 20;

/// SPSS Data Editor look: light-gray cell blocks separated by white gaps,
/// a white row-number gutter, and `var` placeholder headers beyond real columns.
/// Row 0 of the virtual list is the header band.
/// Interactive: click selects a cell (like the SPSS active cell),
/// double-click edits it in place, Delete clears the selected cell.
fn spss_grid(
    ui: &mut egui::Ui,
    mut table: Option<&mut DataTable>,
    vis_cols: &[usize],
    sel_cell: &mut Option<(usize, usize)>,
    editing_cell: &mut Option<(usize, usize)>,
    edit_buf: &mut String,
) {
    let n_data_rows = table.as_deref().map(|t| t.n_rows.min(999_999)).unwrap_or(0);
    let n_real = vis_cols.len();
    let total_cols = n_real.max(SPSS_MIN_COLS);
    let total_rows = 1 + n_data_rows.max(40);
    let content_w = SPSS_ROWNUM_W + SPSS_GAP + (total_cols as f32) * (SPSS_COL_W + SPSS_GAP);

    let header_fill = egui::Color32::from_rgb(0xF3, 0xF3, 0xF3);
    let header_sel_fill = egui::Color32::from_rgb(0xDC, 0xE8, 0xF7);
    let cell_fill = egui::Color32::from_rgb(0xF6, 0xF6, 0xF6);
    let sel_fill = egui::Color32::from_rgb(0xE4, 0xED, 0xFC);
    let sel_stroke = egui::Stroke::new(1.4, egui::Color32::from_rgb(0x2E, 0x6D, 0xC8));
    let header_text = egui::Color32::from_rgb(0x33, 0x33, 0x33);
    let placeholder_text = egui::Color32::from_rgb(0x9B, 0x9B, 0x9B);
    let value_text = egui::Color32::from_rgb(0x1E, 0x1E, 0x1E);
    let rownum_text = egui::Color32::from_rgb(0x66, 0x66, 0x66);
    let rownum_sel_text = egui::Color32::from_rgb(0x2E, 0x6D, 0xC8);

    let font = egui::FontId::proportional(12.5);

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show_rows(ui, SPSS_ROW_H + SPSS_GAP, total_rows, |ui, rows| {
            for row in rows {
                let (rect, resp) =
                    ui.allocate_exact_size(egui::vec2(content_w, SPSS_ROW_H), egui::Sense::click());
                let p = ui.painter();
                let mid_y = rect.center().y;
                let cell_x = |j: usize| rect.left() + SPSS_ROWNUM_W + SPSS_GAP + (j as f32) * (SPSS_COL_W + SPSS_GAP);
                let cell_rect = |j: usize, inset: f32| {
                    egui::Rect::from_min_max(
                        egui::pos2(cell_x(j), rect.top() + inset),
                        egui::pos2(cell_x(j) + SPSS_COL_W, rect.bottom() - inset),
                    )
                };
                // which data column is under the mouse (if any)?
                let hit_col = |pos: egui::Pos2| -> Option<usize> {
                    if pos.x < rect.left() + SPSS_ROWNUM_W + SPSS_GAP {
                        return None;
                    }
                    let fx = pos.x - rect.left() - SPSS_ROWNUM_W - SPSS_GAP;
                    let j = (fx / (SPSS_COL_W + SPSS_GAP)).floor() as isize;
                    if j >= 0 && (j as usize) < n_real {
                        Some(j as usize)
                    } else {
                        None
                    }
                };

                if row == 0 {
                    // header band
                    let sel_j = sel_cell
                        .as_ref()
                        .and_then(|(ci, _)| vis_cols.iter().position(|&c| c == *ci));
                    for j in 0..total_cols {
                        let r = cell_rect(j, 0.0);
                        let sel_here = sel_j == Some(j);
                        p.rect_filled(r, 0.0, if sel_here { header_sel_fill } else { header_fill });
                        let (txt, color) = match table.as_deref().and_then(|t| vis_cols.get(j).map(|&ci| &t.columns[ci].name)) {
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
                    let row_sel = sel_cell
                        .as_ref()
                        .map(|(_, r)| *r == di)
                        .unwrap_or(false);
                    p.text(
                        egui::pos2(rect.left() + 7.0, mid_y),
                        egui::Align2::LEFT_CENTER,
                        format!("{}", di + 1),
                        font.clone(),
                        if row_sel { rownum_sel_text } else { rownum_text },
                    );
                    if let Some(t) = table.as_deref() {
                        for j in 0..total_cols {
                            let r = cell_rect(j, 1.0);
                            // real cell (with its data) or empty placeholder block under a `var` header
                            if j < n_real {
                                let ci = vis_cols[j];
                                let selected = sel_cell.as_ref() == Some(&(ci, di));
                                p.rect_filled(
                                    r,
                                    0.0,
                                    if selected { sel_fill } else { cell_fill },
                                );
                                if selected {
                                    p.rect_stroke(r, 0.0, sel_stroke);
                                }
                                let c = &t.columns[ci];
                                let (anchor, px) = match c.meta.align {
                                    crate::data::VarAlign::Right => (egui::Align2::RIGHT_CENTER, r.right() - 6.0),
                                    crate::data::VarAlign::Center => (egui::Align2::CENTER_CENTER, r.center().x),
                                    crate::data::VarAlign::Left => (egui::Align2::LEFT_CENTER, r.left() + 6.0),
                                };
                                let txt = t.cell_text(ci, di);
                                if !txt.is_empty() {
                                    p.text(egui::pos2(px, mid_y), anchor, txt, font.clone(), value_text);
                                }
                            } else {
                                p.rect_filled(r, 0.0, cell_fill);
                            }
                        }
                    } else {
                        // no data yet: SPSS still shows the empty gray cell grid
                        for j in 0..total_cols {
                            p.rect_filled(cell_rect(j, 1.0), 0.0, cell_fill);
                        }
                    }
                }

                // ---- interactions ----
                let clicked_col = resp.clicked().then(|| resp.hover_pos()).flatten().and_then(&hit_col);
                if let Some(j) = clicked_col {
                    if row > 0 {
                        if n_real > 0 {
                            *sel_cell = Some((vis_cols[j], di_of_row(row)));
                            if *editing_cell != Some((vis_cols[j], di_of_row(row))) {
                                *editing_cell = None;
                            }
                        }
                    } else {
                        *sel_cell = None;
                    }
                }
                if resp.double_clicked() {
                    if let Some(j) = resp.hover_pos().and_then(&hit_col) {
                        if row > 0 {
                            if let Some(t) = table.as_deref() {
                                if n_real > 0 {
                                    let dummy = di_of_row(row);
                                    let (ci, di) = (vis_cols[j], dummy);
                                    *editing_cell = Some((ci, di));
                                    *edit_buf = t.cell_text(ci, di);
                                }
                            }
                        }
                    }
                }

                // in-cell edit box overlay
                if let Some((eci, edi)) = *editing_cell {
                    if row > 0 && edi == di_of_row(row) {
                        if let Some(j) = vis_cols.iter().position(|&c| c == eci) {
                            let r = cell_rect(j, 1.0);
                            p.rect_filled(r, 0.0, egui::Color32::WHITE);
                            p.rect_stroke(r, 0.0, sel_stroke);
                            let mut child = ui.child_ui(
                                r.shrink(1.0),
                                egui::Layout::top_down(egui::Align::LEFT),
                                None,
                            );
                            let tresp = child.add(
                                egui::TextEdit::singleline(edit_buf)
                                    .desired_width(SPSS_COL_W - 8.0)
                                    .frame(false),
                            );
                            child.memory_mut(|m| m.request_focus(tresp.id));
                            let enter = child
                                .input(|i| i.key_pressed(egui::Key::Enter));
                            let esc = child
                                .input(|i| i.key_pressed(egui::Key::Escape));
                            if esc {
                                *editing_cell = None;
                            } else if enter || tresp.lost_focus() {
                                if let Some(t) = table.as_deref_mut() {
                                    commit_edit(t, eci, edi, edit_buf);
                                }
                                *editing_cell = None;
                            }
                        }
                    }
                }
            }

            // Delete clears the selected cell
            if let Some((ci, di)) = *sel_cell {
                if editing_cell.is_none() && ui.input(|i| i.key_pressed(egui::Key::Delete)) {
                    if let Some(t) = table.as_deref_mut() {
                        if di < t.n_rows && ci < t.columns.len() {
                            t.columns[ci].values[di] = crate::data::Value::Missing;
                        }
                    }
                }
            }
        });
}

#[inline]
fn di_of_row(row: usize) -> usize {
    row - 1
}

/// SPSS semantics: empty -> missing; unparseable numeric -> missing.
fn commit_edit(table: &mut DataTable, ci: usize, di: usize, buf: &str) {
    if ci >= table.columns.len() || di >= table.n_rows {
        return;
    }
    let raw = buf.trim().to_string();
    let v = if raw.is_empty() {
        crate::data::Value::Missing
    } else {
        match table.columns[ci].kind {
            crate::data::ColumnKind::Numeric => raw
                .parse::<f64>()
                .map(crate::data::Value::Number)
                .unwrap_or(crate::data::Value::Missing),
            crate::data::ColumnKind::String => crate::data::Value::Text(raw),
        }
    };
    table.columns[ci].values[di] = v;
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_export_escapes_and_right_aligns_numbers() {
        let mut app = StateCraftApp::default();
        app.outputs.push(crate::output::OutputTable {
            title: "T<1>".into(),
            headers: vec!["A".into(), "B".into()],
            rows: vec![vec!["x".into(), "1.5".into()]],
            note: "n".into(),
        });
        let path = std::env::temp_dir().join("sc_out.html");
        app.export_output_html_to(&path);
        let s = std::fs::read_to_string(&path).unwrap();
        assert!(s.contains("T&lt;1&gt;"), "title escaped");
        assert!(s.contains("<td class=\"num\">1.5</td>"), "numeric right-aligned");
        assert!(s.contains("<p class=\"note\">n</p>"), "note rendered");
        let _ = std::fs::remove_file(path);
    }
}

/// SPSS pivot-table look: bordered cells, shaded header band, right-aligned
/// numerics, footnote note beneath the table.
fn render_output_table(ui: &mut egui::Ui, t: &OutputTable) {
    let header_fill = egui::Color32::from_rgb(0xEA, 0xF0, 0xF8);
    let edge = egui::Color32::from_rgb(0x99, 0x99, 0x99);
    egui::Grid::new(&t.title)
        .striped(false)
        .spacing([0.0, 0.0])
        .show(ui, |ui| {
            for h in &t.headers {
                egui::Frame::none()
                    .fill(header_fill)
                    .stroke(egui::Stroke::new(0.5, edge))
                    .inner_margin(egui::Margin::symmetric(8.0, 3.0))
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width().min(90.0));
                        ui.strong(egui::RichText::new(h).size(12.0));
                    });
            }
            ui.end_row();
            for row in &t.rows {
                for (k, cell) in row.iter().enumerate() {
                    let is_num = k > 0 && cell.parse::<f64>().is_ok();
                    egui::Frame::none()
                        .fill(egui::Color32::WHITE)
                        .stroke(egui::Stroke::new(0.5, edge))
                        .inner_margin(egui::Margin::symmetric(8.0, 2.0))
                        .show(ui, |ui| {
                            ui.set_min_width(ui.available_width().min(90.0));
                            ui.with_layout(
                                if is_num {
                                    egui::Layout::right_to_left(egui::Align::Center)
                                } else {
                                    egui::Layout::left_to_right(egui::Align::Center)
                                },
                                |ui| {
                                    ui.label(egui::RichText::new(cell).size(12.0));
                                },
                            );
                        });
                }
                ui.end_row();
            }
        });
    if !t.note.is_empty() {
        ui.add_space(3.0);
        ui.label(egui::RichText::new(&t.note).size(10.5).weak());
    }
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

fn render_variables(ui: &mut egui::Ui, table: &mut DataTable) {
    use crate::data::{Measure, VarAlign, ColumnKind};
    let header_fill = egui::Color32::from_rgb(0xE9, 0xE9, 0xE9);
    let cell_fill = egui::Color32::from_rgb(0xF7, 0xF7, 0xF7);
    let header_text = egui::Color32::from_rgb(0x5E, 0x5E, 0x5E);

    let mut block = |ui: &mut egui::Ui,
                     w: f32,
                     fill: egui::Color32,
                     add: &mut dyn FnMut(&mut egui::Ui)| {
        egui::Frame::none()
            .fill(fill)
            .inner_margin(egui::Margin {
                left: 6.0,
                right: 6.0,
                top: 2.0,
                bottom: 2.0,
            })
            .show(ui, |inner| {
                inner.set_min_width(w);
                inner.set_min_height(18.0);
                add(inner);
            });
    };

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("var_grid")
                .spacing([2.0, 2.0])
                .striped(false)
                .show(ui, |ui| {
                    // ---- header row (SPSS Variable View columns) ----
                    for h in ["Name", "Type", "Width", "Decimals", "Label", "Values",
                              "Missing", "Columns", "Align", "Measure", "Role"] {
                        block(ui, 92.0, header_fill, &mut |inner: &mut egui::Ui| {
                            inner.label(
                                egui::RichText::new(h).size(12.0).color(header_text),
                            );
                        });
                    }
                    ui.end_row();

                    for (ci, c) in table.columns.iter_mut().enumerate() {
                        // Name
                        block(ui, 110.0, cell_fill, &mut |inner: &mut egui::Ui| {
                            inner.add(
                                egui::TextEdit::singleline(&mut c.name)
                                    .desired_width(96.0)
                                    .frame(false),
                            );
                        });
                        // Type (converting values like SPSS)
                        block(ui, 82.0, cell_fill, &mut |inner: &mut egui::Ui| {
                            let label = match c.kind {
                                ColumnKind::Numeric => "Numeric",
                                ColumnKind::String => "String",
                            };
                            egui::menu::menu_button(inner, label, |menu| {
                                if menu.button("Numeric").clicked() {
                                    menu.close_menu();
                                    c.set_kind(ColumnKind::Numeric);
                                }
                                if menu.button("String").clicked() {
                                    menu.close_menu();
                                    c.set_kind(ColumnKind::String);
                                }
                            });
                        });
                        // Width
                        block(ui, 60.0, cell_fill, &mut |inner: &mut egui::Ui| {
                            inner.add(
                                egui::DragValue::new(&mut c.meta.columns_width)
                                    .range(1..=64),
                            );
                        });
                        // Decimals (numeric only)
                        block(ui, 66.0, cell_fill, &mut |inner: &mut egui::Ui| {
                            if c.kind == ColumnKind::Numeric {
                                inner.add(
                                    egui::DragValue::new(&mut c.meta.decimals).range(0..=16),
                                );
                            } else {
                                inner.label("");
                            }
                        });
                        // Label
                        block(ui, 120.0, cell_fill, &mut |inner: &mut egui::Ui| {
                            inner.add(
                                egui::TextEdit::singleline(&mut c.meta.label)
                                    .desired_width(104.0)
                                    .frame(false)
                                    .hint_text(""),
                            );
                        });
                        // Values (value labels)
                        block(ui, 88.0, cell_fill, &mut |inner: &mut egui::Ui| {
                            let n = c.meta.value_labels.len() + c.meta.text_value_labels.len();
                            let txt = if n == 0 { "None".to_string() } else { format!("Defined ({})", n) };
                            inner.label(
                                egui::RichText::new(txt).size(12.0),
                            );
                        });
                        // Missing
                        block(ui, 88.0, cell_fill, &mut |inner: &mut egui::Ui| {
                            let n = c.meta.user_missing.len();
                            let txt = if n == 0 { "None".to_string() } else { format!("Defined ({})", n) };
                            inner.label(egui::RichText::new(txt).size(12.0));
                        });
                        // Columns
                        block(ui, 66.0, cell_fill, &mut |inner: &mut egui::Ui| {
                            inner.add(
                                egui::DragValue::new(&mut c.meta.columns_width).range(1..=64),
                            );
                        });
                        // Align
                        block(ui, 76.0, cell_fill, &mut |inner: &mut egui::Ui| {
                            egui::ComboBox::from_id_source(("align", ci))
                                .selected_text(c.meta.align.as_str())
                                .width(56.0)
                                .show_ui(inner, |menu| {
                                    menu.selectable_value(
                                        &mut c.meta.align,
                                        VarAlign::Left,
                                        VarAlign::Left.as_str(),
                                    );
                                    menu.selectable_value(
                                        &mut c.meta.align,
                                        VarAlign::Right,
                                        VarAlign::Right.as_str(),
                                    );
                                    menu.selectable_value(
                                        &mut c.meta.align,
                                        VarAlign::Center,
                                        VarAlign::Center.as_str(),
                                    );
                                });
                        });
                        // Measure
                        block(ui, 92.0, cell_fill, &mut |inner: &mut egui::Ui| {
                            egui::ComboBox::from_id_source(("measure", ci))
                                .selected_text(c.meta.measure.as_str())
                                .width(68.0)
                                .show_ui(inner, |menu| {
                                    menu.selectable_value(
                                        &mut c.meta.measure,
                                        Measure::Nominal,
                                        Measure::Nominal.as_str(),
                                    );
                                    menu.selectable_value(
                                        &mut c.meta.measure,
                                        Measure::Ordinal,
                                        Measure::Ordinal.as_str(),
                                    );
                                    menu.selectable_value(
                                        &mut c.meta.measure,
                                        Measure::Scale,
                                        Measure::Scale.as_str(),
                                    );
                                });
                        });
                        // Role
                        block(ui, 70.0, cell_fill, &mut |inner: &mut egui::Ui| {
                            inner.label(egui::RichText::new("Input").size(12.0));
                        });
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
                app.load();
            }
            Ok(Box::new(app))
        }),
    )
}
