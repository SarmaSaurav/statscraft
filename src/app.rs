use crate::data::DataTable;
use crate::output::OutputTable;
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

pub struct StatsCraftApp {
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
    /// Phase 5: the open procedure dialog, if any.
    pub dialog: Option<crate::dialog::DialogState>,
    /// Phase 10: the open Missing Values dialog, if any.
    pub missing_dialog: Option<crate::dialog::MissingValues>,
    /// Phase 11: the open Chart Builder, if any.
    pub chart_builder: Option<crate::chartbuilder::ChartBuilder>,
    /// Whether the dark theme is active.
    pub dark: bool,
    /// The most recently opened data files, newest first.
    pub recent_files: Vec<String>,
    /// The rect of the last chart drawn, for a copy request.
    pub last_chart_rect: Option<egui::Rect>,
    /// The viewport that chart was drawn in (the Output Viewer or the
    /// inline Output tab).
    pub last_chart_viewport: Option<egui::ViewportId>,
    /// The chart rect captured for a pending "Copy Chart" screenshot.
    pub pending_screenshot_rect: Option<egui::Rect>,
    /// The egui context, so the theme can be re-applied when it
    /// changes.
    pub ctx: egui::Context,
}

impl Default for StatsCraftApp {
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
            dialog: None,
            missing_dialog: None,
            chart_builder: None,
            dark: false,
            recent_files: Vec::new(),
            last_chart_rect: None,
            last_chart_viewport: None,
            pending_screenshot_rect: None,
            // A placeholder context, replaced when the app is created.
            // The theme is applied to the real one there.
            ctx: egui::Context::default(),
        }
    }
}

impl StatsCraftApp {
    pub fn load(&mut self) {
        let path_text = self.path_input.trim().to_string();
        let p = std::path::Path::new(&path_text);
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
                self.add_recent_file(p);
            }
            Err(e) => {
                self.status = format!("Error: {}", e);
                self.table = None;
            }
        }
    }

    /// Write the small preferences file (theme and recent files).
    pub fn save_settings(&self) {
        let settings = crate::settings::Settings {
            dark: self.dark,
            recent_files: self.recent_files.clone(),
        };
        crate::settings::save(&settings);
    }

    /// Put a successfully opened file at the top of File > Recent Files.
    fn add_recent_file(&mut self, path: &std::path::Path) {
        let mut settings = crate::settings::Settings {
            dark: self.dark,
            recent_files: self.recent_files.clone(),
        };
        settings.remember_file(path);
        crate::settings::save(&settings);
        self.recent_files = settings.recent_files;
    }

    /// File > New > Data: an empty dataset, ready for Variable View.
    pub fn new_data(&mut self) {
        self.table = Some(DataTable::default());
        self.path_input.clear();
        self.sel_cell = None;
        self.editing_cell = None;
        self.edit_buf.clear();
        self.dataset_no += 1;
        self.view = View::Data;
        self.status =
            "New dataset. Switch to Variable View to create the first variable.".to_string();
    }

    /// File > New > Syntax.
    pub fn new_syntax(&mut self) {
        self.syntax_text.clear();
        self.show_syntax_window = true;
        self.status = "New syntax file.".to_string();
    }

    /// File > New > Output.
    pub fn new_output(&mut self) {
        self.outputs.clear();
        self.selected_output = 0;
        self.show_output_window = true;
        self.status = "New output document.".to_string();
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

    /// Open a procedure dialog. This is what every Analyze menu item does
    /// now — the user chooses variables and options, then presses OK.
    pub fn open_dialog(&mut self, procedure: crate::dialog::Procedure) {
        if self.table.is_none() {
            self.status = "No data loaded — open a data file first.".to_string();
            return;
        }
        self.dialog = Some(crate::dialog::DialogState::new(procedure));
    }

    /// Handle the result of a dialog that closed: either run the syntax
    /// (OK) or paste it into the Syntax Editor (Paste).
    pub fn close_dialog(&mut self, d: crate::dialog::DialogState) {
        if let Some(syntax) = d.confirmed {
            self.paste_and_run(syntax);
        } else if let Some(syntax) = d.pasted {
            // SPSS's Paste puts the command in the syntax window without
            // running it.
            self.syntax_text = if self.syntax_text.trim().is_empty() {
                syntax.clone()
            } else {
                format!("{}\n{}", self.syntax_text.trim_end(), syntax)
            };
            self.show_syntax_window = true;
            self.status = format!("Pasted into Syntax1:\n{}", syntax);
        }
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

    /// Switch between the light and the dark theme.
    ///
    /// The palette is global, so the change reaches every window, and
    /// the style is re-applied to the context so the egui widgets
    /// follow.
    pub fn set_dark(&mut self, dark: bool) {
        // The flag is what matters: the style is re-applied on the next
        // frame, and every viewport follows it.
        self.dark = dark;
        let ctx = self.ctx.clone();
        crate::theme::apply_style(&ctx, dark);
        self.status = if dark {
            "Dark theme.".to_string()
        } else {
            "Light theme.".to_string()
        };
        self.save_settings();
    }

    /// Pivot the selected output: exchange its row and column
    /// dimensions.
    ///
    /// This is the drag-and-drop of a pivot table, done with a button.
    /// The flattened grid is rebuilt from the pivoted table, so the
    /// export carries the new orientation too.
    pub fn pivot_selected_output(&mut self) {
        let i = self.selected_output;
        let Some(p) = self.outputs.get(i).and_then(|o| o.pivot.clone()) else {
            self.status = "This table has no pivot structure.".to_string();
            return;
        };
        let pivoted = p.pivoted();
        // The flattened view follows the pivot.
        let flat = pivoted.flattened();
        let (headers, rows) = match flat.first() {
            Some((_, h, r)) => (h.clone(), r.clone()),
            None => (Vec::new(), Vec::new()),
        };
        let t = &mut self.outputs[i];
        t.pivot = Some(pivoted);
        t.headers = headers;
        t.rows = rows;
        self.status = "Table pivoted: the row and column dimensions were exchanged.".to_string();
    }

    /// Add a case at the end of the data, as SPSS's "Add Cases" does.
    pub fn add_case(&mut self) {
        let Some(t) = self.table.as_mut() else {
            self.status = "No data loaded.".to_string();
            return;
        };
        let row = t.add_case();
        self.sel_cell = Some((0, row));
        self.status = format!("Added case {}.", row + 1);
    }

    /// Delete the selected case.
    pub fn delete_case(&mut self) {
        let Some((_, row)) = self.sel_cell else {
            self.status = "Select a case first.".to_string();
            return;
        };
        let Some(t) = self.table.as_mut() else {
            self.status = "No data loaded.".to_string();
            return;
        };
        if t.delete_case(row) {
            self.status = format!("Deleted case {}.", row + 1);
            // The selection moves to the case that took its place.
            let last = t.n_rows.saturating_sub(1);
            self.sel_cell = Some((0, row.min(last)));
        }
    }

    /// Copy the selected cell to the clipboard.
    pub fn copy_cell(&mut self, _ctx: &egui::Context) {
        let Some((col, row)) = self.sel_cell else {
            self.status = "Select a cell first.".to_string();
            return;
        };
        let Some(t) = self.table.as_ref() else {
            self.status = "No data loaded.".to_string();
            return;
        };
        let text = t.cell_text(col, row);
        match arboard::Clipboard::new().and_then(|mut c| c.set_text(text.clone())) {
            Ok(()) => self.status = format!("Copied '{}'.", text),
            Err(e) => self.status = format!("Could not copy: {}", e),
        }
    }

    /// Paste the clipboard into the grid, at the selected cell.
    ///
    /// One line pastes one cell; a block of lines pastes a column of
    /// values down the selected column, which is what a spreadsheet
    /// copy gives.
    pub fn paste_cell(&mut self, ctx: &egui::Context) {
        let Some((col, row)) = self.sel_cell else {
            self.status = "Select a cell first.".to_string();
            return;
        };
        let text = match arboard::Clipboard::new().and_then(|mut c| c.get_text()) {
            Ok(t) => t,
            Err(e) => {
                self.status = format!("Could not read the clipboard: {}", e);
                return;
            }
        };
        if text.trim().is_empty() {
            self.status = "The clipboard is empty.".to_string();
            return;
        }
        let Some(t) = self.table.as_mut() else {
            self.status = "No data loaded.".to_string();
            return;
        };
        let pasted = t.paste_text(col, row, &text);
        self.status = format!("Pasted {} row(s).", pasted);
    }

    /// Define the missing values of a numeric variable.
    ///
    /// The values can be a list of discrete values or a range, and the
    /// procedures then leave them out.
    pub fn define_missing_values(&mut self, col: usize, discrete: &[f64], range: Option<(f64, f64)>) {
        let Some(t) = self.table.as_mut() else {
            self.status = "No data loaded.".to_string();
            return;
        };
        if t.define_user_missing(col, discrete, range) {
            let name = t.columns[col].name.clone();
            self.status = match range {
                Some((lo, hi)) => {
                    format!("{}: {} to {} is now missing.", name, lo, hi)
                }
                None => format!("{}: {} value(s) are now missing.", name, discrete.len()),
            };
        } else {
            self.status = "Missing values can only be defined for a numeric variable.".to_string();
        }
    }

    /// Save the data as an SPSS .sav file.
    pub fn save_data_sav(&mut self, path: &std::path::Path) {
        let Some(t) = &self.table else {
            self.status = "No data loaded.".to_string();
            return;
        };
        match crate::data::write_sav(t, path) {
            Ok(()) => {
                self.status = format!("Saved {} as SPSS .sav.", path.display());
            }
            Err(e) => self.status = format!("Save failed: {}", e),
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
        self.export_output_document(path, false);
    }

    /// Export the output as a Microsoft Word-compatible `.doc` or an
    /// Excel-compatible `.xls`.
    ///
    /// Both formats are HTML documents with the Office namespaces and a
    /// `ProgId` hint. Word and Excel open them as editable documents, and
    /// the pivot structure and the styling survive the round trip. This is
    /// the same idea as SPSS's `OUTPUT EXPORT /DOC` and `/XLS`, without
    /// carrying a full OOXML writer and a ZIP dependency.
    pub fn export_output_office_to(&mut self, path: &std::path::Path) {
        self.export_output_document(path, true);
    }

    fn export_output_document(&mut self, path: &std::path::Path, office: bool) {
        if self.outputs.is_empty() {
            self.status = "No output to export.".to_string();
            return;
        }
        let prog_id = if office {
            let is_excel = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.eq_ignore_ascii_case("xls") || e.eq_ignore_ascii_case("xlsx"))
                .unwrap_or(false);
            if is_excel {
                "Excel.Sheet"
            } else {
                "Word.Document"
            }
        } else {
            ""
        };
        let h = output_document_html(&self.outputs, office, prog_id);
        match std::fs::write(path, h) {
            Ok(_) => {
                self.status = if office {
                    format!("Exported output for Office to {}", path.display())
                } else {
                    format!("Exported output (HTML) to {}", path.display())
                }
            }
            Err(e) => self.status = format!("Export failed: {}", e),
        }
    }

    /// Copy the selected output table as rich HTML, with a tab-separated
    /// plain-text fallback. Word, Excel, PowerPoint and LibreOffice paste
    /// the HTML as an editable table, which is what SPSS's Viewer copy
    /// does.
    pub fn copy_selected_table(&mut self) {
        let Some(t) = self.outputs.get(self.selected_output) else {
            self.status = "No output to copy.".to_string();
            return;
        };
        let html = format!(
            "<html><head><meta charset=\"utf-8\"></head><body>{}</body></html>",
            output_table_html(t, false)
        );
        let alt = output_table_text(t);
        match arboard::Clipboard::new()
            .and_then(|mut c| c.set_html(html, Some(alt)))
        {
            Ok(()) => self.status = "Copied the table to the clipboard.".to_string(),
            Err(e) => self.status = format!("Could not copy the table: {}", e),
        }
    }

    /// Copy the selected chart as a PNG image.
    ///
    /// The chart is painted by egui, so the reliable way to copy exactly
    /// what the reader sees is a viewport screenshot cropped to the chart
    /// rectangle. The screenshot arrives as an event on the next frame.
    pub fn copy_selected_chart(&mut self, ctx: &egui::Context) {
        let Some(t) = self.outputs.get(self.selected_output) else {
            self.status = "No output to copy.".to_string();
            return;
        };
        if t.chart.is_none() {
            self.status = "The selected output has no chart.".to_string();
            return;
        }
        let Some(rect) = self.last_chart_rect else {
            self.status = "The chart has not been drawn yet; try again.".to_string();
            return;
        };
        self.pending_screenshot_rect = Some(rect);
        let viewport = self.last_chart_viewport.unwrap_or_else(|| ctx.viewport_id());
        ctx.send_viewport_cmd_to(viewport, egui::ViewportCommand::Screenshot);
        self.status = "Copying the chart…".to_string();
    }

    /// Handle the reply to a screenshot request: crop the chart rectangle
    /// and put it on the clipboard as RGBA image data.
    pub fn handle_screenshot(&mut self, ctx: &egui::Context) {
        let Some(rect) = self.pending_screenshot_rect else {
            return;
        };
        let shot = ctx.input(|i| {
            i.events.iter().find_map(|e| match e {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        let Some(image) = shot else {
            return;
        };
        self.pending_screenshot_rect = None;
        let ppp = ctx.pixels_per_point();
        let x0 = (rect.min.x * ppp).floor().max(0.0) as usize;
        let y0 = (rect.min.y * ppp).floor().max(0.0) as usize;
        let x1 = ((rect.max.x * ppp).ceil() as usize).min(image.width());
        let y1 = ((rect.max.y * ppp).ceil() as usize).min(image.height());
        if x1 <= x0 || y1 <= y0 {
            self.status = "The chart area could not be captured.".to_string();
            return;
        }
        let w = x1 - x0;
        let h = y1 - y0;
        let mut bytes = Vec::with_capacity(w * h * 4);
        for y in y0..y1 {
            for x in x0..x1 {
                let c = image.pixels[y * image.width() + x];
                bytes.extend_from_slice(&[c.r(), c.g(), c.b(), c.a()]);
            }
        }
        let data = arboard::ImageData {
            width: w,
            height: h,
            bytes: std::borrow::Cow::Owned(bytes),
        };
        match arboard::Clipboard::new().and_then(|mut c| c.set_image(data)) {
            Ok(()) => self.status = "Copied the chart to the clipboard.".to_string(),
            Err(e) => self.status = format!("Could not copy the chart: {}", e),
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

impl eframe::App for StatsCraftApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // A chart copied as an image is captured with a screenshot; the
        // reply arrives on the next frame.
        self.handle_screenshot(ctx);

        // ---- Phase 4: real OS document windows (SPSS multi-document UX) ----
        // The Output Viewer and Syntax editor live in their own viewports;
        // the Data Editor stays the main window, like IBM SPSS.
        if self.show_output_window {
            ctx.show_viewport_immediate(
                egui::ViewportId::from_hash_of("statscraft_output"),
                egui::ViewportBuilder::default()
                    .with_title(format!("Output1 - StatsCraft Statistics Viewer"))
                    .with_inner_size([1080.0, 720.0])
                    .with_min_inner_size([520.0, 360.0]),
                |vctx, _class| {
                    self.handle_screenshot(vctx);
                    egui::CentralPanel::default()
                        .frame(
                            egui::Frame::none()
                                .fill(crate::theme::surface())
                                .inner_margin(egui::Margin::same(12.0)),
                        )
                        .show(vctx, |ui| {
                            egui::TopBottomPanel::top("ov_title")
                                .frame(
                                    egui::Frame::none()
                                        .fill(crate::theme::surface_alt())
                                        .inner_margin(egui::Margin::symmetric(12.0, 8.0)),
                                )
                                .show_inside(ui, |ui| {
                                    ui.add(egui::Label::new(egui::RichText::new("Output1").font(crate::theme::semibold(18.0))));
                                    ui.separator();
                                });
                            if self.outputs.is_empty() {
                                ui.vertical_centered(|ui| {
                                    ui.add_space(40.0);
                                    crate::icon::paint_icon(
                                        ui.painter(),
                                        ui.ctx(),
                                        crate::theme::Icon::Output,
                                        egui::Rect::from_center_size(
                                            ui.painter().text(
                                                egui::Pos2::ZERO,
                                                egui::Align2::LEFT_CENTER,
                                                "",
                                                crate::theme::body(12.0),
                                                crate::theme::text(),
                                            )
                                            .min,
                                            egui::vec2(48.0, 48.0),
                                        ),
                                        crate::theme::text_faint(),
                                    );
                                    ui.add_space(10.0);
                                    ui.label(
                                        egui::RichText::new("No output yet")
                                            .font(crate::theme::semibold(15.0))
                                            .color(crate::theme::text_muted()),
                                    );
                                    ui.label(
                                        egui::RichText::new(
                                            "Run an analysis from the Analyze menu, or type SPSS syntax in Syntax1.",
                                        )
                                        .color(crate::theme::text_faint())
                                        .size(12.0),
                                    );
                                });
                                return;
                            }

                            // SPSS splits the Viewer into an outline tree on
                            // the left and the result on the right.
                            ui.columns(2, |cols| {
                                cols[0].set_min_width(210.0);
                                cols[0].group(|ui| {
                                    ui.add(
                                        egui::Label::new(
                                            egui::RichText::new("Outline")
                                                .font(crate::theme::semibold(11.0))
                                                .color(crate::theme::text_faint()),
                                        ),
                                    );
                                    ui.add_space(4.0);
                                    egui::ScrollArea::vertical()
                                        .auto_shrink([false, false])
                                        .show(ui, |ui| {
                                            for (i, t) in self.outputs.iter().enumerate() {
                                                let sel = self.selected_output == i;
                                                let color = if sel {
                                                    crate::theme::accent()
                                                } else {
                                                    crate::theme::text_muted()
                                                };
                                                let galley = ui.painter().layout_no_wrap(
                                                    t.title.clone(),
                                                    crate::theme::medium(12.0),
                                                    color,
                                                );
                                                let (rect, resp) = ui.allocate_exact_size(
                                                    egui::vec2(ui.available_width(), 22.0),
                                                    egui::Sense::click(),
                                                );
                                                let p = ui.painter();
                                                if sel {
                                                    p.rect_filled(
                                                        rect,
                                                        crate::theme::RADIUS_SM,
                                                        crate::theme::accent_soft(),
                                                    );
                                                } else if resp.hovered() {
                                                    p.rect_filled(
                                                        rect,
                                                        crate::theme::RADIUS_SM,
                                                        crate::theme::surface_alt(),
                                                    );
                                                }
                                                // a small tree bullet
                                                p.circle_filled(
                                                    egui::pos2(rect.left() + 10.0, rect.center().y),
                                                    2.5,
                                                    color,
                                                );
                                                p.galley(
                                                    egui::pos2(
                                                        rect.left() + 18.0,
                                                        rect.center().y - galley.size().y / 2.0,
                                                    ),
                                                    galley,
                                                    color,
                                                );
                                                if resp.clicked() {
                                                    self.selected_output = i;
                                                }
                                            }
                                        });
                                });
                                // The actions are applied after the
                                // borrow of the table ends.
                                let mut pivot_requested = false;
                                let mut copy_table = false;
                                let mut copy_chart = false;
                                let mut chart_rect = None;
                                cols[1].group(|ui| {
                                    let t = &self.outputs[self.selected_output.min(self.outputs.len() - 1)];
                                    egui::ScrollArea::vertical()
                                        .auto_shrink([false, false])
                                        .show(ui, |ui| {
                                            ui.add_space(2.0);
                                            ui.add(egui::Label::new(
                                                egui::RichText::new(&t.title)
                                                    .font(crate::theme::semibold(16.0)),
                                            ));
                                            ui.add_space(6.0);
                                            ui.horizontal_wrapped(|ui| {
                                                // SPSS's Viewer copies the
                                                // selected object; the rich
                                                // table survives a paste into
                                                // Word, Excel or PowerPoint.
                                                if ui
                                                    .button(
                                                        egui::RichText::new("Copy Table")
                                                            .size(11.5)
                                                            .color(crate::theme::text()),
                                                    )
                                                    .on_hover_text(
                                                        "Copy the table as an editable Office table.",
                                                    )
                                                    .clicked()
                                                {
                                                    copy_table = true;
                                                }
                                                if t.chart.is_some()
                                                    && ui
                                                        .button(
                                                            egui::RichText::new("Copy Chart")
                                                                .size(11.5)
                                                                .color(crate::theme::text()),
                                                        )
                                                        .on_hover_text(
                                                            "Copy the chart as a PNG image.",
                                                        )
                                                        .clicked()
                                                {
                                                    copy_chart = true;
                                                }
                                                // A pivot table can be
                                                // pivoted: the row and the
                                                // column dimensions
                                                // exchanged, the way SPSS
                                                // lets the reader drag one
                                                // across.
                                                if t.pivot.is_some()
                                                    && ui
                                                        .button(
                                                            egui::RichText::new("Pivot")
                                                                .size(11.5)
                                                                .color(crate::theme::text()),
                                                        )
                                                        .on_hover_text(
                                                            "Exchange the row and the column dimensions.",
                                                        )
                                                        .clicked()
                                                {
                                                    pivot_requested = true;
                                                }
                                            });
                                            ui.add_space(6.0);
                                            render_output_table(ui, t);
                                            if let Some(chart) = &t.chart {
                                                chart_rect = Some(render_output_chart(ui, chart));
                                            }
                                        });
                                });
                                if let Some(rect) = chart_rect {
                                    self.last_chart_rect = Some(rect);
                                    self.last_chart_viewport = Some(vctx.viewport_id());
                                }
                                if pivot_requested {
                                    self.pivot_selected_output();
                                }
                                if copy_table {
                                    self.copy_selected_table();
                                }
                                if copy_chart {
                                    self.copy_selected_chart(vctx);
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
        // ---- Phase 5: the open procedure dialog ----
        // SPSS opens a modal dialog for every procedure. We draw it in the
        // main window so the Data Editor stays visible behind it.
        // ---- Phase 11: the Chart Builder ----
        if self.chart_builder.is_some() {
            let mut cb = self.chart_builder.take().expect("the builder is present");
            crate::chartbuilder::show(ctx, self, &mut cb);
            if cb.confirmed {
                if let Some(t) = self.table.as_ref() {
                    if let Some(chart) = cb.build(t) {
                        let title = chart.title.clone();
                        self.outputs.push(crate::output::OutputTable {
                            title: title.clone(),
                            headers: vec!["Chart".into()],
                            rows: vec![vec![title.clone()]],
                            note: String::new(),
                            chart: Some(chart),
                            pivot: None,
                        });
                        self.status = format!("Chart '{}' added to the output.", title);
                        self.show_output_window = true;
                    } else {
                        self.status = "These choices do not make a chart.".to_string();
                    }
                }
            } else if !cb.cancelled {
                // The window was neither confirmed nor cancelled (for
                // example the click happened outside it), so it stays.
                self.chart_builder = Some(cb);
            }
            // Cancel and the title-bar close both drop the builder.
        }
        // ---- Phase 10: the Missing Values dialog ----
        if self.missing_dialog.is_some() {
            let mut mv = self.missing_dialog.take().expect("the dialog is present");
            crate::dialog::show_missing(ctx, self, &mut mv);
            if let Some((variable, discrete, range)) = mv.confirmed.clone() {
                if let Some(col) = self
                    .table
                    .as_ref()
                    .and_then(|t| t.column_index(&variable))
                {
                    self.define_missing_values(col, &discrete, range);
                }
            } else {
                // The dialog was closed without a decision, so it goes
                // back as it was.
                self.missing_dialog = Some(mv);
            }
        }
        if self.dialog.is_some() {
            // Take the dialog out of the app while it draws, so the dialog
            // can hold a mutable borrow of the app without aliasing.
            let mut d = self.dialog.take().expect("dialog present");
            crate::dialog::show(ctx, self, &mut d);
            if d.confirmed.is_some() || d.pasted.is_some() {
                // The user pressed OK or Paste: the dialog is done.
                self.close_dialog(d);
            } else if d.cancelled {
                // The window was closed with the X or Cancel.
                self.dialog = None;
            } else {
                self.dialog = Some(d);
            }
        }

        if self.show_syntax_window {
            ctx.show_viewport_immediate(
                egui::ViewportId::from_hash_of("statscraft_syntax"),
                egui::ViewportBuilder::default()
                    .with_title(format!("Syntax1 - StatsCraft Syntax Editor"))
                    .with_inner_size([860.0, 560.0])
                    .with_min_inner_size([520.0, 320.0]),
                |vctx, _class| {
                    egui::CentralPanel::default()
                        .frame(
                            egui::Frame::none()
                                .fill(crate::theme::surface())
                                .inner_margin(egui::Margin::same(12.0)),
                        )
                        .show(vctx, |ui| {
                            egui::TopBottomPanel::top("sx_title")
                                .frame(
                                    egui::Frame::none()
                                        .fill(crate::theme::surface_alt())
                                        .inner_margin(egui::Margin::symmetric(12.0, 8.0)),
                                )
                                .show_inside(ui, |ui| {
                                    ui.add(egui::Label::new(egui::RichText::new("Syntax1").font(crate::theme::semibold(18.0))));
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
                            let run = ui.add(
                                egui::Button::new(
                                    egui::RichText::new("Run Syntax")
                                        .font(crate::theme::medium(13.0))
                                        .color(crate::theme::text_on_accent()),
                                )
                                .fill(crate::theme::accent())
                                .rounding(crate::theme::RADIUS_SM)
                                .min_size(egui::vec2(110.0, 30.0)),
                            );
                            if run.clicked() {
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

        // The theme is installed once in `run`; re-assert it here so a
        // viewport that inherits the default egui look still gets
        // StatsCraft. The flag is the app's own, so a viewport that is
        // opened later follows the theme the user chose.
        crate::theme::apply_style(ctx, self.dark);
        // SPSS titles the window "<dataset> [DataSet<n>] - <product>".
        let title = format!("{} — StatsCraft Statistics", self.dataset_label());
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));

        egui::TopBottomPanel::top("menu")
            .frame(
                egui::Frame::none()
                    .fill(crate::theme::surface())
                    .inner_margin(egui::Margin {
                        left: 8.0,
                        right: 8.0,
                        top: 3.0,
                        bottom: 3.0,
                    }),
            )
            .show(ctx, |ui| {
                egui::menu::bar(ui, |ui| {
                    // ---------------- File ----------------
                    ui.menu_button("File", |ui| {
                        // SPSS's File > New has one entry per document type.
                        ui.menu_button("New", |ui| {
                            if crate::icon::menu_item(
                                ui,
                                crate::theme::Icon::DataView,
                                "Data",
                            )
                            .clicked()
                            {
                                ui.close_menu();
                                self.new_data();
                            }
                            if crate::icon::menu_item(
                                ui,
                                crate::theme::Icon::Syntax,
                                "Syntax",
                            )
                            .clicked()
                            {
                                ui.close_menu();
                                self.new_syntax();
                            }
                            if crate::icon::menu_item(
                                ui,
                                crate::theme::Icon::Output,
                                "Output",
                            )
                            .clicked()
                            {
                                ui.close_menu();
                                self.new_output();
                            }
                        });
                        ui.separator();
                        // Recent files are loaded when the item is chosen, just
                        // as SPSS's File > Recently Used Data does.
                        ui.menu_button("Recent Files", |ui| {
                            let recent = self.recent_files.clone();
                            if recent.is_empty() {
                                ui.add_enabled(
                                    false,
                                    egui::Button::new(
                                        egui::RichText::new("(No recent files)").weak(),
                                    ),
                                );
                            } else {
                                for path in recent {
                                    let label = std::path::Path::new(&path)
                                        .file_name()
                                        .and_then(|s| s.to_str())
                                        .unwrap_or(&path)
                                        .to_string();
                                    if ui
                                        .button(label)
                                        .on_hover_text(&path)
                                        .clicked()
                                    {
                                        ui.close_menu();
                                        self.path_input = path;
                                        self.load();
                                    }
                                }
                                ui.separator();
                                if ui.button("Clear Recent Files").clicked() {
                                    ui.close_menu();
                                    self.recent_files.clear();
                                    let settings = crate::settings::Settings {
                                        dark: self.dark,
                                        recent_files: Vec::new(),
                                    };
                                    crate::settings::save(&settings);
                                }
                            }
                        });
                        ui.separator();
                        if crate::icon::menu_item(ui, crate::theme::Icon::Open, "Open Data…").clicked() {
                            ui.close_menu();
                            if let Some(p) = rfd::FileDialog::new()
                                .add_filter("Data", &["csv", "sav"])
                                .pick_file()
                            {
                                self.path_input = p.display().to_string();
                                self.load();
                            }
                        }
                        ui.menu_button("Import Data", |ui| {
                            if ui.button("CSV Data…").clicked() {
                                ui.close_menu();
                                if let Some(p) = rfd::FileDialog::new()
                                    .add_filter("CSV", &["csv"])
                                    .pick_file()
                                {
                                    self.path_input = p.display().to_string();
                                    self.load();
                                }
                            }
                            if ui.button("SPSS Statistics (.sav)…").clicked() {
                                ui.close_menu();
                                if let Some(p) = rfd::FileDialog::new()
                                    .add_filter("SPSS", &["sav"])
                                    .pick_file()
                                {
                                    self.path_input = p.display().to_string();
                                    self.load();
                                }
                            }
                        });
                        ui.separator();
                        if crate::icon::menu_item(ui, crate::theme::Icon::Save, "Save Data As…").clicked() {
                            ui.close_menu();
                            if let Some(p) =
                                rfd::FileDialog::new().add_filter("CSV", &["csv"]).save_file()
                            {
                                self.export_data_csv_to(&p);
                            }
                        }
                        if crate::icon::menu_item(ui, crate::theme::Icon::Export, "Export Output…").clicked() {
                            ui.close_menu();
                            if let Some(p) =
                                rfd::FileDialog::new().add_filter("Markdown", &["md"]).save_file()
                            {
                                self.export_output_to(&p);
                            }
                        }
                        if crate::icon::menu_item(ui, crate::theme::Icon::Print, "Export Output (HTML)…").clicked() {
                            ui.close_menu();
                            if let Some(p) = rfd::FileDialog::new()
                                .add_filter("HTML", &["html", "htm"])
                                .save_file()
                            {
                                self.export_output_html_to(&p);
                            }
                        }
                        ui.menu_button("Export to Microsoft Office", |ui| {
                            if ui.button("Word Document (.doc)…").clicked() {
                                ui.close_menu();
                                if let Some(p) = rfd::FileDialog::new()
                                    .add_filter("Word Document", &["doc"])
                                    .set_file_name("StatsCraft Output.doc")
                                    .save_file()
                                {
                                    self.export_output_office_to(&p);
                                }
                            }
                            if ui.button("Excel Workbook (.xls)…").clicked() {
                                ui.close_menu();
                                if let Some(p) = rfd::FileDialog::new()
                                    .add_filter("Excel Workbook", &["xls"])
                                    .set_file_name("StatsCraft Output.xls")
                                    .save_file()
                                {
                                    self.export_output_office_to(&p);
                                }
                            }
                            ui.separator();
                            if ui.button("Copy Selected Table").clicked() {
                                ui.close_menu();
                                self.copy_selected_table();
                            }
                            if ui.button("Copy Selected Chart as Image").clicked() {
                                ui.close_menu();
                                self.copy_selected_chart(ui.ctx());
                            }
                        });
                        ui.separator();
                        if ui.button("Exit").clicked() {
                            ui.close_menu();
                            std::process::exit(0);
                        }
                    });
                    // ---------------- Edit ----------------
                    ui.menu_button("Edit", |ui| {
                        if crate::icon::menu_item(ui, crate::theme::Icon::Undo, "Undo").clicked() {
                            ui.close_menu();
                            self.status = "Undo: not implemented yet.".to_string();
                        }
                        if crate::icon::menu_item(ui, crate::theme::Icon::Redo, "Redo").clicked() {
                            ui.close_menu();
                            self.status = "Redo: not implemented yet.".to_string();
                        }
                        ui.separator();
                        if crate::icon::menu_item(ui, crate::theme::Icon::Cut, "Cut").clicked() {
                            ui.close_menu();
                            self.status = "Cut: not implemented yet.".to_string();
                        }
                        if crate::icon::menu_item(ui, crate::theme::Icon::Copy, "Copy").clicked() {
                            ui.close_menu();
                            let ctx = ui.ctx().clone();
                            self.copy_cell(&ctx);
                        }
                        if crate::icon::menu_item(ui, crate::theme::Icon::Paste, "Paste").clicked() {
                            ui.close_menu();
                            let ctx = ui.ctx().clone();
                            self.paste_cell(&ctx);
                        }
                    });
                    // ---------------- View ----------------
                    ui.menu_button("View", |ui| {
                        if crate::icon::menu_item(ui, crate::theme::Icon::Variables, "Variable View").clicked() {
                            ui.close_menu();
                            self.view = View::Variables;
                        }
                        if crate::icon::menu_item(ui, crate::theme::Icon::Output, "Output Viewer").clicked() {
                            ui.close_menu();
                            self.show_output_window = true;
                        }
                        if crate::icon::menu_item(ui, crate::theme::Icon::Syntax, "Syntax Editor").clicked() {
                            ui.close_menu();
                            self.show_syntax_window = true;
                        }
                        ui.separator();
                        if ui.checkbox(&mut self.grid_lines, "Grid Lines").clicked() {
                            ui.close_menu();
                        }
                        if ui.checkbox(&mut self.show_status, "Status Bar").clicked() {
                            ui.close_menu();
                        }
                        ui.separator();
                        // The theme: the light workbench one and the
                        // dark one, which follows the Dracula palette.
                        let mut dark = crate::theme::is_dark();
                        if ui.checkbox(&mut dark, "Dark Theme").clicked() {
                            ui.close_menu();
                            self.set_dark(dark);
                        }
                    });
                    // ---------------- Data ----------------
                    ui.menu_button("Data", |ui| {
                        if crate::icon::menu_item(ui, crate::theme::Icon::Settings, "Define Variable Properties…").clicked() {
                            ui.close_menu();
                            self.view = View::Variables;
                        }
                        ui.separator();
                        // The case-level editing, as SPSS's Data menu
                        // has it.
                        if crate::icon::menu_item(ui, crate::theme::Icon::InsertCase, "Add Cases").clicked() {
                            ui.close_menu();
                            self.add_case();
                        }
                        if crate::icon::menu_item(ui, crate::theme::Icon::DeleteCase, "Delete Cases…").clicked() {
                            ui.close_menu();
                            self.delete_case();
                        }
                        if crate::icon::menu_item(ui, crate::theme::Icon::ValueLabels, "Define Missing Values…").clicked() {
                            ui.close_menu();
                            self.missing_dialog = Some(crate::dialog::MissingValues::default());
                        }
                        ui.separator();
                        ui.menu_button("Sort Cases", |ui| {
                            if self.table.is_none() {
                                ui.label("No data loaded.");
                            } else {
                                let names: Vec<String> = self.table.as_ref().unwrap().columns.iter().map(|c| c.name.clone()).collect();
                                for name in names.iter() {
                                    ui.menu_button(name, |ui| {
                                        if ui.button("Ascending").clicked() { ui.close_menu(); let cmd = format!("SORT CASES BY {} (A)", name); self.paste_and_run(cmd); }
                                        if ui.button("Descending").clicked() { ui.close_menu(); let cmd = format!("SORT CASES BY {} (D)", name); self.paste_and_run(cmd); }
                                    });
                                }
                            }
                        });
                        ui.separator();
                        if crate::icon::menu_item(ui, crate::theme::Icon::Filter, "Select Cases…").clicked() {
                            ui.close_menu();
                            self.status = "Select Cases: not implemented yet.".to_string();
                        }
                        if crate::icon::menu_item(ui, crate::theme::Icon::ValueLabels, "Value Labels…").clicked() {
                            ui.close_menu();
                            self.status = "Value Labels: not implemented yet.".to_string();
                        }
                    });
                    // ---------------- Transform ----------------
                    ui.menu_button("Transform", |ui| {
                        if ui.button("Compute Variable…").clicked() { ui.close_menu(); self.status = "Compute Variable: not implemented yet.".to_string(); }
                        if ui.button("Recode into Same Variables…").clicked() { ui.close_menu(); self.status = "Recode into Same Variables: not implemented yet.".to_string(); }
                        if ui.button("Recode into Different Variables…").clicked() { ui.close_menu(); self.status = "Recode into Different Variables: not implemented yet.".to_string(); }
                        if ui.button("Count Values within Cases…").clicked() { ui.close_menu(); self.status = "Count Values within Cases: not implemented yet.".to_string(); }
                        if ui.button("Rank Cases…").clicked() { ui.close_menu(); self.status = "Rank Cases: not implemented yet.".to_string(); }
                        if ui.button("Automatic Recode…").clicked() { ui.close_menu(); self.status = "Automatic Recode: not implemented yet.".to_string(); }
                    });
                    // ---------------- Analyze ----------------
                    // Every procedure opens a dialog, as in SPSS: the user
                    // picks the variables and the options, then presses OK.
                    ui.menu_button("Analyze", |ui| {
                        ui.menu_button("Descriptive Statistics", |ui| {
                            if crate::icon::menu_item(ui, crate::theme::Icon::Histogram, "Frequencies…").clicked() { ui.close_menu(); self.open_dialog(crate::dialog::Procedure::Frequencies); }
                            if crate::icon::menu_item(ui, crate::theme::Icon::Describe, "Descriptives…").clicked() { ui.close_menu(); self.open_dialog(crate::dialog::Procedure::Descriptives); }
                            if crate::icon::menu_item(ui, crate::theme::Icon::Explore, "Explore…").clicked() { ui.close_menu(); self.open_dialog(crate::dialog::Procedure::Explore); }
                            if crate::icon::menu_item(ui, crate::theme::Icon::Crosstabs, "Crosstabs…").clicked() { ui.close_menu(); self.open_dialog(crate::dialog::Procedure::Crosstabs); }
                        });
                        ui.menu_button("Compare Means", |ui| {
                            if crate::icon::menu_item(ui, crate::theme::Icon::Means, "Means…").clicked() { ui.close_menu(); self.open_dialog(crate::dialog::Procedure::Means); }
                            if crate::icon::menu_item(ui, crate::theme::Icon::TTest, "Independent-Samples T Test…").clicked() { ui.close_menu(); self.open_dialog(crate::dialog::Procedure::TTest); }
                            if crate::icon::menu_item(ui, crate::theme::Icon::TTest, "Paired-Samples T Test…").clicked() { ui.close_menu(); self.open_dialog(crate::dialog::Procedure::PairedT); }
                            if crate::icon::menu_item(ui, crate::theme::Icon::Anova, "One-Way ANOVA…").clicked() { ui.close_menu(); self.open_dialog(crate::dialog::Procedure::Anova); }
                            if crate::icon::menu_item(ui, crate::theme::Icon::Glm, "General Linear Model…").clicked() { ui.close_menu(); self.open_dialog(crate::dialog::Procedure::Glm); }
                        });
                        ui.menu_button("Correlate", |ui| {
                            if crate::icon::menu_item(ui, crate::theme::Icon::Correlation, "Bivariate…").clicked() { ui.close_menu(); self.open_dialog(crate::dialog::Procedure::Correlations); }
                        });
                        ui.menu_button("Regression", |ui| {
                            if crate::icon::menu_item(ui, crate::theme::Icon::Regression, "Linear…").clicked() { ui.close_menu(); self.open_dialog(crate::dialog::Procedure::Regression); }
                            if ui.button("Curve Estimation…").clicked() { ui.close_menu(); self.status = "Curve Estimation: not implemented yet.".to_string(); }
                        });
                        ui.menu_button("Scale", |ui| {
                            if crate::icon::menu_item(ui, crate::theme::Icon::Variables, "Reliability Analysis…").clicked() { ui.close_menu(); self.open_dialog(crate::dialog::Procedure::Reliability); }
                            if crate::icon::menu_item(ui, crate::theme::Icon::Variables, "Factor Analysis…").clicked() { ui.close_menu(); self.open_dialog(crate::dialog::Procedure::Factor); }
                        });
                        ui.menu_button("Classify", |ui| {
                            if crate::icon::menu_item(ui, crate::theme::Icon::Correlation, "ROC Curve…").clicked() { ui.close_menu(); self.open_dialog(crate::dialog::Procedure::Roc); }
                        });
                        ui.menu_button("Nonparametric Tests", |ui| {
                            if crate::icon::menu_item(ui, crate::theme::Icon::TTest, "Mann-Whitney U…").clicked() { ui.close_menu(); self.open_dialog(crate::dialog::Procedure::MannWhitney); }
                            if crate::icon::menu_item(ui, crate::theme::Icon::TTest, "Kruskal-Wallis H…").clicked() { ui.close_menu(); self.open_dialog(crate::dialog::Procedure::KruskalWallis); }
                            if crate::icon::menu_item(ui, crate::theme::Icon::TTest, "Wilcoxon Signed-Rank…").clicked() { ui.close_menu(); self.open_dialog(crate::dialog::Procedure::Wilcoxon); }
                            if crate::icon::menu_item(ui, crate::theme::Icon::TTest, "Friedman…").clicked() { ui.close_menu(); self.open_dialog(crate::dialog::Procedure::Friedman); }
                        });
                        ui.separator();
                        if crate::icon::menu_item(ui, crate::theme::Icon::Output, "Reports").clicked() { ui.close_menu(); self.status = "Reports: not implemented yet.".to_string(); }
                    });
                    // ---------------- Graphs ----------------
                    ui.menu_button("Graphs", |ui| {
                        if crate::icon::menu_item(ui, crate::theme::Icon::Chart, "Chart Builder…").clicked() {
                            ui.close_menu();
                            // The builder opens with the defaults for
                            // the data that is loaded.
                            let mut b = match self.chart_builder.take() {
                                Some(b) => b,
                                None => self
                                    .table
                                    .as_ref()
                                    .map(crate::chartbuilder::ChartBuilder::for_table)
                                    .unwrap_or_default(),
                            };
                            b.confirmed = false;
                            b.cancelled = false;
                            self.chart_builder = Some(b);
                        }
                        if crate::icon::menu_item(ui, crate::theme::Icon::Histogram, "Legacy Chart View").clicked() {
                            ui.close_menu();
                            self.view = View::Charts;
                        }
                    });
                    // ---------------- Utilities ----------------
                    ui.menu_button("Utilities", |ui| {
                        if crate::icon::menu_item(ui, crate::theme::Icon::Syntax, "Run Syntax…").clicked() { ui.close_menu(); self.show_syntax_window = true; }
                        if crate::icon::menu_item(ui, crate::theme::Icon::Variables, "Variable Information…").clicked() { ui.close_menu(); self.view = View::Variables; }
                    });
                    // ---------------- Window ----------------
                    ui.menu_button("Window", |ui| {
                        if ui.button("Data Editor").clicked() { ui.close_menu(); self.view = View::Data; }
                        if crate::icon::menu_item(ui, crate::theme::Icon::Output, "Output1").clicked() { ui.close_menu(); self.show_output_window = true; }
                        if crate::icon::menu_item(ui, crate::theme::Icon::Syntax, "Syntax1").clicked() { ui.close_menu(); self.show_syntax_window = true; }
                        ui.separator();
                        if ui.button("Reset Windows").clicked() { ui.close_menu(); self.show_output_window = false; self.show_syntax_window = false; self.view = View::Data; }
                    });
                    // ---------------- Help ----------------
                    ui.menu_button("Help", |ui| {
                        if ui.button("About StatsCraft").clicked() { ui.close_menu(); self.status = "StatsCraft — lightweight SPSS-like workbench (Rust).".to_string(); }
                    });
                });
            });

        egui::TopBottomPanel::top("toolbar")
            .frame(
                egui::Frame::none()
                    .fill(crate::theme::surface())
                    .inner_margin(egui::Margin {
                        left: 8.0,
                        right: 8.0,
                        top: 4.0,
                        bottom: 5.0,
                    }),
            )
            .show(ctx, |ui| {
                // ---- Toolbar: action groups on the left, view switch and the
                //      variable filter on the right, like SPSS but flatter. ----
                ui.horizontal(|ui| {
                    if crate::icon::icon_button(
                        ui,
                        crate::theme::Icon::Open,
                        "Open data file (CSV / SPSS .sav)",
                    )
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
                    if crate::icon::icon_button(
                        ui,
                        crate::theme::Icon::Save,
                        "Save data (CSV or SPSS .sav)",
                    )
                    .clicked()
                    {
                        if let Some(p) = rfd::FileDialog::new()
                            .add_filter("Data", &["csv", "sav"])
                            .save_file()
                        {
                            if p.extension().is_some_and(|e| e.eq_ignore_ascii_case("sav")) {
                                self.save_data_sav(&p);
                            } else {
                                self.export_data_csv_to(&p);
                            }
                        }
                    }
                    if crate::icon::icon_button(
                        ui,
                        crate::theme::Icon::Export,
                        "Export output (Markdown / HTML)",
                    )
                    .clicked()
                    {
                        if let Some(p) =
                            rfd::FileDialog::new().add_filter("Markdown", &["md"]).save_file()
                        {
                            self.export_output_to(&p);
                        }
                    }
                    crate::icon::separator(ui);
                    if crate::icon::icon_button(ui, crate::theme::Icon::Undo, "Undo").clicked() {
                        self.status = "Undo: not implemented yet.".to_string();
                    }
                    if crate::icon::icon_button(ui, crate::theme::Icon::Redo, "Redo").clicked() {
                        self.status = "Redo: not implemented yet.".to_string();
                    }
                    crate::icon::separator(ui);
                    if crate::icon::icon_button(
                        ui,
                        crate::theme::Icon::InsertVariable,
                        "Insert Variable",
                    )
                    .clicked()
                    {
                        self.status = "Insert Variable: not implemented yet.".to_string();
                    }
                    if crate::icon::icon_button(ui, crate::theme::Icon::InsertCase, "Insert Cases")
                        .clicked()
                    {
                        self.status = "Insert Cases: not implemented yet.".to_string();
                    }
                    if crate::icon::icon_button(
                        ui,
                        crate::theme::Icon::ValueLabels,
                        "Value Labels",
                    )
                    .clicked()
                    {
                        self.status = "Value Labels: not implemented yet.".to_string();
                    }
                    crate::icon::separator(ui);
                    if crate::icon::icon_button(
                        ui,
                        crate::theme::Icon::Find,
                        "Go to Variable (Variable View)",
                    )
                    .clicked()
                    {
                        self.view = View::Variables;
                    }
                    if crate::icon::icon_button(ui, crate::theme::Icon::Chart, "Chart Builder")
                        .clicked()
                    {
                        self.view = View::Charts;
                    }
                    crate::icon::separator(ui);
                    if crate::icon::icon_button(ui, crate::theme::Icon::Output, "Output Viewer")
                        .clicked()
                    {
                        self.show_output_window = true;
                    }
                    if crate::icon::icon_button(ui, crate::theme::Icon::Syntax, "Syntax Editor")
                        .clicked()
                    {
                        self.show_syntax_window = true;
                    }

                    // ---- right side: view switch + variable filter ----
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
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
                        ui.label(
                            egui::RichText::new(format!("Visible: {} of {} Variables", visible, total))
                                .color(crate::theme::text_muted())
                                .size(11.5),
                        );
                        ui.add_space(10.0);
                        let filter_w = 200.0;
                        ui.add(
                            egui::TextEdit::singleline(&mut self.var_filter)
                                .hint_text("Filter variables…")
                                .desired_width(filter_w)
                                .frame(true),
                        );
                    });
                });
            });

        // ---- The bottom row: the view tabs on the left, the status on
        //      the right, in one panel as SPSS has it. ----
        egui::TopBottomPanel::bottom("tabs")
            .frame(
                egui::Frame::none()
                    .fill(crate::theme::surface_alt())
                    .inner_margin(egui::Margin {
                        left: 8.0,
                        right: 12.0,
                        top: 4.0,
                        bottom: 4.0,
                    }),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    for (label, v, icon) in [
                        ("Data View", View::Data, crate::theme::Icon::DataView),
                        (
                            "Variable View",
                            View::Variables,
                            crate::theme::Icon::Variables,
                        ),
                    ] {
                        let sel = self.view == v
                            || (v == View::Data
                                && self.view != View::Variables
                                && self.view != View::Output
                                && self.view != View::Charts
                                && self.view != View::Syntax);
                        // The active tab is a solid blue pill; the other is
                        // a quiet grey one that fills with blue on hover.
                        let color = if sel {
                            crate::theme::text_on_accent()
                        } else {
                            crate::theme::text_muted()
                        };

                        let galley = ui
                            .painter()
                            .layout_no_wrap(label.to_string(), crate::theme::semibold(12.0), color);
                        let icon_w = 15.0;
                        let size = egui::vec2(galley.size().x + icon_w + 24.0, 27.0);
                        let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());

                        let p = ui.painter();
                        if sel {
                            // A fully rounded blue pill.
                            p.rect_filled(rect, rect.height() / 2.0, crate::theme::accent());
                        } else if resp.hovered() {
                            p.rect_filled(
                                rect,
                                rect.height() / 2.0,
                                crate::theme::accent().gamma_multiply(0.14),
                            );
                        }

                        let ctx = ui.ctx().clone();
                        crate::icon::paint_icon(
                            p,
                            &ctx,
                            icon,
                            egui::Rect::from_center_size(
                                egui::pos2(rect.left() + 8.0 + icon_w / 2.0, rect.center().y),
                                egui::vec2(icon_w, icon_w),
                            ),
                            color,
                        );
                        p.galley(
                            egui::pos2(
                                rect.left() + 18.0 + icon_w,
                                rect.center().y - galley.size().y / 2.0,
                            ),
                            galley,
                            color,
                        );

                        if resp.clicked() {
                            self.view = v;
                        }
                    }

                    // The status sits on the right of the same row, as it
                    // does in SPSS.
                    if self.show_status {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new("Unicode: ON")
                                .color(crate::theme::text_faint())
                                .size(11.0),
                        );
                        ui.add_space(10.0);
                        // SPSS shows "IBM SPSS Statistics Processor is ready".
                        // We keep the same shape, with our own name.
                        let ready = !self.status.starts_with("Error");
                        let (label, col) = if ready {
                            (
                                "StatsCraft Statistics Processor is ready",
                                crate::theme::teal(),
                            )
                        } else {
                            (
                                "StatsCraft Statistics Processor has an error",
                                crate::theme::amber(),
                            )
                        };
                        ui.label(
                            egui::RichText::new(label)
                                .color(col)
                                .size(11.5),
                        );
                        ui.add_space(14.0);
                        // SPSS shows the cell count in the status bar.
                        if let Some(t) = &self.table {
                            ui.label(
                                egui::RichText::new(format!("{} cells", t.n_rows * t.columns.len()))
                                    .color(crate::theme::text_faint())
                                    .size(11.0),
                            );
                            ui.add_space(12.0);
                        }
                        ui.label(
                            egui::RichText::new(&self.status)
                                .color(crate::theme::text_muted())
                                .size(11.5),
                        );
                        });
                    }
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
        let mut add_variable = false;
        match self.view {
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
                let StatsCraftApp {
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
                    render_variables(ui, table, &mut add_variable);
                } else {
                    // A fresh session has no dataset yet. SPSS lets the
                    // reader start from Variable View; an empty screen
                    // with no controls does not.
                    ui.vertical_centered(|ui| {
                        ui.add_space(40.0);
                        ui.label(
                            egui::RichText::new("No dataset is open")
                                .font(crate::theme::semibold(16.0))
                                .color(crate::theme::text()),
                        );
                        ui.add_space(6.0);
                        ui.label(
                            egui::RichText::new(
                                "Create a variable to begin, or open a data file from File > Open.",
                            )
                            .color(crate::theme::text_muted())
                            .size(12.0),
                        );
                        ui.add_space(14.0);
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("New Variable…")
                                        .font(crate::theme::medium(13.0))
                                        .color(crate::theme::text_on_accent()),
                                )
                                .fill(crate::theme::accent())
                                .rounding(crate::theme::RADIUS_SM)
                                .min_size(egui::vec2(150.0, 32.0)),
                            )
                            .clicked()
                        {
                            add_variable = true;
                        }
                    });
                }
            }
            View::Output => {
                if self.outputs.is_empty() {
                    ui.label("No output yet. Run an analysis.");
                } else {
                    if self.selected_output >= self.outputs.len() {
                        self.selected_output = 0;
                    }
                    let mut copy_table = false;
                    let mut copy_chart = false;
                    let mut chart_rect = None;
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
                                    // A pivot table can be pivoted: the
                                    // row and the column dimensions
                                    // exchanged, the way SPSS lets
                                    // the reader drag one across.
                                    let has_pivot = self.outputs[self.selected_output]
                                        .pivot
                                        .is_some();
                                    let has_chart = self.outputs[self.selected_output]
                                        .chart
                                        .is_some();
                                    ui.horizontal_wrapped(|ui| {
                                        if ui
                                            .button(
                                                egui::RichText::new("Copy Table")
                                                    .size(11.5)
                                                    .color(crate::theme::text()),
                                            )
                                            .on_hover_text(
                                                "Copy the table as an editable Office table.",
                                            )
                                            .clicked()
                                        {
                                            copy_table = true;
                                        }
                                        if has_chart
                                            && ui
                                                .button(
                                                    egui::RichText::new("Copy Chart")
                                                        .size(11.5)
                                                        .color(crate::theme::text()),
                                                )
                                                .on_hover_text(
                                                    "Copy the chart as a PNG image.",
                                                )
                                                .clicked()
                                        {
                                            copy_chart = true;
                                        }
                                        if has_pivot
                                            && ui
                                                .button(
                                                    egui::RichText::new("Pivot")
                                                        .size(11.5)
                                                        .color(crate::theme::text()),
                                                )
                                                .on_hover_text(
                                                    "Exchange the row and the column dimensions.",
                                                )
                                                .clicked()
                                        {
                                            self.pivot_selected_output();
                                        }
                                    });
                                    ui.add_space(4.0);
                                    render_output_table(ui, &self.outputs[self.selected_output]);
                                    if let Some(chart) = &self.outputs[self.selected_output].chart {
                                        chart_rect = Some(render_output_chart(ui, chart));
                                    }
                                });
                        });
                    });
                    if let Some(rect) = chart_rect {
                        self.last_chart_rect = Some(rect);
                        self.last_chart_viewport = Some(ctx.viewport_id());
                    }
                    if copy_table {
                        self.copy_selected_table();
                    }
                    if copy_chart {
                        self.copy_selected_chart(ctx);
                    }
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
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("Run Syntax")
                                    .font(crate::theme::medium(13.0))
                                    .color(crate::theme::text_on_accent()),
                            )
                            .fill(crate::theme::accent())
                            .rounding(crate::theme::RADIUS_SM)
                            .min_size(egui::vec2(110.0, 30.0)),
                        )
                        .clicked()
                    {
                        crate::syntax::run(self, &self.syntax_text.clone());
                    }
                });
            }
        }
        if add_variable {
            let table = self.table.get_or_insert_with(DataTable::default);
            let idx = table.add_variable(crate::data::ColumnKind::Numeric);
            self.status = format!(
                "Added {}. Edit its name, label and measure in Variable View.",
                table.columns[idx].name
            );
        }
        });
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.save_settings();
    }
}

/// Small vector-drawn SPSS-like toolbar icons (no emoji/glyph font dependency).

const SPSS_ROW_H: f32 = 19.0;
const SPSS_ROWNUM_W: f32 = 48.0;
const SPSS_COL_W: f32 = 82.0;
const SPSS_GAP: f32 = 2.5;
const SPSS_MIN_COLS: usize = 20;

/// The Data Editor grid.
///
/// Modern take on the SPSS grid: white cells on a light canvas, a soft
/// header band, subtle zebra striping, rounded selection and a hairline
/// border around the whole table. Row 0 of the virtual list is the header
/// band. Clicking selects a cell (the SPSS active cell), double-clicking
/// edits it in place, Delete clears the selected cell.
fn spss_grid(
    ui: &mut egui::Ui,
    mut table: Option<&mut DataTable>,
    vis_cols: &[usize],
    sel_cell: &mut Option<(usize, usize)>,
    editing_cell: &mut Option<(usize, usize)>,
    edit_buf: &mut String,
) {
    use crate::theme;

    let n_data_rows = table.as_deref().map(|t| t.n_rows.min(999_999)).unwrap_or(0);
    let n_real = vis_cols.len();
    let total_cols = n_real.max(theme::MIN_COLS);
    let total_rows = 1 + n_data_rows.max(40);
    let content_w =
        theme::ROWNUM_W + theme::GAP + (total_cols as f32) * (theme::COL_W + theme::GAP);

    let header_fill = theme::header();
    let header_sel_fill = theme::accent_soft();
    let cell_fill = theme::cell();
    let cell_alt_fill = theme::cell_alt();
    let sel_fill = theme::selected();
    let sel_stroke = egui::Stroke::new(1.5, theme::accent());
    let header_text = theme::text();
    let placeholder_text = theme::text_faint();
    let value_text = theme::text();
    let rownum_text = theme::text_faint();
    let rownum_sel_text = theme::accent();

    let font = theme::body(12.5);
    let header_font = theme::medium(12.5);

    // Both axes: the data editor keeps its full column width and scrolls
    // sideways, and egui turns Shift+wheel into horizontal scrolling.
    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show_rows(ui, theme::ROW_H + theme::GAP, total_rows, |ui, rows| {
            for row in rows {
                let (rect, resp) = ui.allocate_exact_size(
                    egui::vec2(content_w, theme::ROW_H),
                    egui::Sense::click(),
                );
                let p = ui.painter();
                let mid_y = rect.center().y;
                let cell_x = |j: usize| {
                    rect.left() + theme::ROWNUM_W + theme::GAP
                        + (j as f32) * (theme::COL_W + theme::GAP)
                };
                let cell_rect = |j: usize, inset: f32| {
                    egui::Rect::from_min_max(
                        egui::pos2(cell_x(j), rect.top() + inset),
                        egui::pos2(cell_x(j) + theme::COL_W, rect.bottom() - inset),
                    )
                };
                let hit_col = |pos: egui::Pos2| -> Option<usize> {
                    if pos.x < rect.left() + theme::ROWNUM_W + theme::GAP {
                        return None;
                    }
                    let fx = pos.x - rect.left() - theme::ROWNUM_W - theme::GAP;
                    let j = (fx / (theme::COL_W + theme::GAP)).floor() as isize;
                    if j >= 0 && (j as usize) < n_real {
                        Some(j as usize)
                    } else {
                        None
                    }
                };

                if row == 0 {
                    // ---- header band ----
                    let sel_j = sel_cell
                        .as_ref()
                        .and_then(|(ci, _)| vis_cols.iter().position(|&c| c == *ci));
                    for j in 0..total_cols {
                        let r = cell_rect(j, 0.0);
                        let sel_here = sel_j == Some(j);
                        p.rect_filled(r, theme::RADIUS_SM, if sel_here { header_sel_fill } else { header_fill });
                        let (txt, color) = match table
                            .as_deref()
                            .and_then(|t| vis_cols.get(j).map(|&ci| &t.columns[ci].name))
                        {
                            Some(name) => (name.as_str(), header_text),
                            None => ("var", placeholder_text),
                        };
                        p.text(
                            r.center(),
                            egui::Align2::CENTER_CENTER,
                            txt,
                            header_font.clone(),
                            color,
                        );
                    }
                    // hairline under the header band
                    p.line_segment(
                        [
                            egui::pos2(rect.left(), rect.bottom()),
                            egui::pos2(rect.right(), rect.bottom()),
                        ],
                        egui::Stroke::new(1.0, theme::border()),
                    );
                } else {
                    // ---- data row ----
                    let di = row - 1;
                    let row_sel = sel_cell
                        .as_ref()
                        .map(|(_, r)| *r == di)
                        .unwrap_or(false);
                    p.text(
                        egui::pos2(rect.left() + 10.0, mid_y),
                        egui::Align2::LEFT_CENTER,
                        format!("{}", di + 1),
                        font.clone(),
                        if row_sel { rownum_sel_text } else { rownum_text },
                    );
                    if let Some(t) = table.as_deref() {
                        for j in 0..total_cols {
                            let r = cell_rect(j, 0.5);
                            if j < n_real {
                                let ci = vis_cols[j];
                                let selected = sel_cell.as_ref() == Some(&(ci, di));
                                // Zebra striping keeps long tables readable.
                                let base = if di % 2 == 0 { cell_fill } else { cell_alt_fill };
                                p.rect_filled(r, theme::RADIUS_SM, if selected { sel_fill } else { base });
                                if selected {
                                    p.rect_stroke(r, theme::RADIUS_SM, sel_stroke);
                                }
                                let c = &t.columns[ci];
                                let (anchor, px) = match c.meta.align {
                                    crate::data::VarAlign::Right => {
                                        (egui::Align2::RIGHT_CENTER, r.right() - 8.0)
                                    }
                                    crate::data::VarAlign::Center => {
                                        (egui::Align2::CENTER_CENTER, r.center().x)
                                    }
                                    crate::data::VarAlign::Left => {
                                        (egui::Align2::LEFT_CENTER, r.left() + 8.0)
                                    }
                                };
                                let txt = t.cell_text(ci, di);
                                if !txt.is_empty() {
                                    p.text(egui::pos2(px, mid_y), anchor, txt, font.clone(), value_text);
                                }
                            } else {
                                p.rect_filled(r, theme::RADIUS_SM, theme::surface_alt());
                            }
                        }
                    } else {
                        // No data yet: still draw the empty cell grid.
                        for j in 0..total_cols {
                            p.rect_filled(cell_rect(j, 0.5), theme::RADIUS_SM, cell_alt_fill);
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

                // ---- in-cell edit box overlay ----
                if let Some((eci, edi)) = *editing_cell {
                    if row > 0 && edi == di_of_row(row) {
                        if let Some(j) = vis_cols.iter().position(|&c| c == eci) {
                            let r = cell_rect(j, 0.5);
                            p.rect_filled(r, theme::RADIUS_SM, theme::cell());
                            p.rect_stroke(r, theme::RADIUS_SM, sel_stroke);
                            let mut child = ui.child_ui(
                                r.shrink(1.0),
                                egui::Layout::top_down(egui::Align::LEFT),
                                None,
                            );
                            let tresp = child.add(
                                egui::TextEdit::singleline(edit_buf)
                                    .desired_width(theme::COL_W - 10.0)
                                    .frame(false),
                            );
                            child.memory_mut(|m| m.request_focus(tresp.id));
                            let enter = child.input(|i| i.key_pressed(egui::Key::Enter));
                            let esc = child.input(|i| i.key_pressed(egui::Key::Escape));
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

/// One output object as an Office-friendly HTML table.
fn output_table_html(t: &OutputTable, include_chart: bool) -> String {
    let mut h = String::new();
    h.push_str(&format!("<h2>{}</h2>\n", html_escape(&t.title)));
    match &t.pivot {
        Some(p) => h.push_str(&pivot_to_html(p)),
        None => {
            h.push_str("<table>\n<tr>");
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
        }
    }
    if include_chart {
        if let Some(chart) = &t.chart {
            h.push_str(&chart_to_svg(chart, 720.0, 300.0));
            h.push('\n');
        }
    }
    if !t.note.is_empty() {
        h.push_str(&format!(
            "<p class=\"note\">{}</p>\n",
            html_escape(&t.note).replace('\n', "<br>")
        ));
    }
    h
}

/// The full HTML document for HTML and Office export.
fn output_document_html(outputs: &[OutputTable], office: bool, prog_id: &str) -> String {
    let mut h = String::new();
    h.push_str("<!DOCTYPE html>\n<html xmlns:o=\"urn:schemas-microsoft-com:office:office\" ");
    h.push_str("xmlns:w=\"urn:schemas-microsoft-com:office:word\" ");
    h.push_str("xmlns=\"http://www.w3.org/TR/REC-html40\">\n<head>\n<meta charset=\"utf-8\">\n");
    if office {
        h.push_str(&format!(
            "<meta name=\"ProgId\" content=\"{}\">\n",
            prog_id
        ));
        h.push_str("<meta name=\"Generator\" content=\"StatsCraft\">\n");
    }
    h.push_str("<title>StatsCraft Output</title>\n<style>\n");
    h.push_str(
        "@page{size:8.5in 11in;margin:0.6in;}\n\
         body{font-family:'Segoe UI',Calibri,Arial,sans-serif;margin:24px;color:#1a1f2b;}\n\
         h2{font-size:14pt;margin:18px 0 6px 0;color:#1a1f2b;}\n\
         table{border-collapse:collapse;font-size:10.5pt;margin:0 0 10px 0;}\n\
         th{background:#e8eef7;border:1px solid #8a8a8a;padding:3px 10px;text-align:left;}\n\
         td{border:1px solid #b5b5b5;padding:2px 10px;}\n\
         td.num{text-align:right;font-variant-numeric:tabular-nums;}\n\
         td.stub{font-weight:600;background:#f6f8fb;}\n\
         .note{color:#555;font-size:9.5pt;}\n\
         th.corner{text-align:right;}\n\
         .measures{color:#555;font-size:9.5pt;margin:0 0 4px 0;}\n\
         .measures span{color:#888;}\n\
         svg{display:block;margin:8px 0;}\n",
    );
    h.push_str("</style>\n</head>\n<body>\n");
    for t in outputs {
        h.push_str(&output_table_html(t, true));
    }
    h.push_str("</body>\n</html>\n");
    h
}

/// The plain-text alternative for the clipboard: tab-separated cells,
/// which every spreadsheet understands.
fn output_table_text(t: &OutputTable) -> String {
    let mut out = String::new();
    out.push_str(&t.title);
    out.push('\n');
    if !t.headers.is_empty() {
        out.push_str(&t.headers.join("\t"));
        out.push('\n');
    }
    for row in &t.rows {
        out.push_str(&row.join("\t"));
        out.push('\n');
    }
    if !t.note.is_empty() {
        out.push_str(&t.note);
        out.push('\n');
    }
    out
}

/// The chart as a self-contained SVG, so an Office export keeps the
/// picture rather than dropping it.
fn chart_to_svg(chart: &crate::output::Chart, width: f32, height: f32) -> String {
    use crate::output::ChartKind;
    let (w, h) = (width.max(320.0), height.max(200.0));
    let left = 56.0;
    let right = w - 16.0;
    let top = 26.0;
    let bottom = h - 42.0;
    let plot_w = (right - left).max(1.0);
    let plot_h = (bottom - top).max(1.0);

    let mut max_y = 0.0f64;
    let mut min_y = 0.0f64;
    let mut max_x = 0.0f64;
    for s in &chart.series {
        for y in &s.y {
            max_y = max_y.max(*y);
            min_y = min_y.min(*y);
        }
        for x in &s.x {
            max_x = max_x.max(*x);
        }
    }
    if max_y <= min_y {
        max_y = min_y + 1.0;
    }
    if max_x <= 0.0 {
        max_x = 1.0;
    }
    let y_max = max_y * 1.1;
    let px = |x: f64| left + (x / max_x) as f32 * plot_w;
    let py = |y: f64| bottom - ((y - min_y) / (y_max - min_y)) as f32 * plot_h;

    let mut s = String::new();
    s.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{:.0}\" height=\"{:.0}\" viewBox=\"0 0 {:.0} {:.0}\">",
        w, h, w, h
    ));
    s.push_str("<rect width=\"100%\" height=\"100%\" fill=\"#ffffff\"/>");
    s.push_str(&format!(
        "<text x=\"{:.1}\" y=\"16\" font-family=\"Segoe UI,Arial\" font-size=\"13\" font-weight=\"600\" fill=\"#1a1f2b\">{}</text>",
        left / 2.0,
        xml_escape(&chart.title)
    ));
    // Grid and y axis labels.
    for i in 0..=5 {
        let t = i as f64 / 5.0;
        let y = bottom - t as f32 * plot_h;
        s.push_str(&format!(
            "<line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"#dde3ed\" stroke-width=\"0.7\"/>",
            left, y, right, y
        ));
        let v = min_y + t * (y_max - min_y);
        s.push_str(&format!(
            "<text x=\"{:.1}\" y=\"{:.1}\" font-family=\"Segoe UI,Arial\" font-size=\"9\" text-anchor=\"end\" fill=\"#939eb0\">{:.2}</text>",
            left - 6.0,
            y + 3.0,
            v
        ));
    }
    s.push_str(&format!(
        "<line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"#c2ccdc\" stroke-width=\"1\"/>",
        left, bottom, right, bottom
    ));

    let palette = ["#2f6fed", "#0fa87e", "#e08a0b", "#bd93f9"];
    if chart.kind == ChartKind::Line || chart.kind == ChartKind::Scatter {
        for (si, series) in chart.series.iter().enumerate() {
            let color = palette[si % palette.len()];
            if chart.kind == ChartKind::Line {
                let points: Vec<String> = series
                    .x
                    .iter()
                    .zip(series.y.iter())
                    .map(|(x, y)| format!("{:.1},{:.1}", px(*x), py(*y)))
                    .collect();
                s.push_str(&format!(
                    "<polyline points=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"2\"/>",
                    points.join(" "),
                    color
                ));
            }
            for (x, y) in series.x.iter().zip(series.y.iter()) {
                s.push_str(&format!(
                    "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"2.4\" fill=\"{}\"/>",
                    px(*x),
                    py(*y),
                    color
                ));
            }
        }
    } else {
        // Bars: one bar per x value, using the category labels when the
        // chart has them.
        for (si, series) in chart.series.iter().enumerate() {
            let color = palette[si % palette.len()];
            let n = series.y.len().max(1);
            let bw = (plot_w / n as f32 * 0.72).max(3.0);
            for (i, y) in series.y.iter().enumerate() {
                let x_center = left + (i as f32 + 0.5) / n as f32 * plot_w;
                let y0 = py(*y);
                s.push_str(&format!(
                    "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" fill=\"{}\"/>",
                    x_center - bw / 2.0,
                    y0,
                    bw,
                    (bottom - y0).max(0.0),
                    color
                ));
                if let Some(label) = series.x_labels.get(i) {
                    s.push_str(&format!(
                        "<text x=\"{:.1}\" y=\"{:.1}\" font-family=\"Segoe UI,Arial\" font-size=\"8\" text-anchor=\"middle\" fill=\"#5b6678\">{}</text>",
                        x_center,
                        bottom + 12.0,
                        xml_escape(label)
                    ));
                }
            }
        }
    }
    s.push_str(&format!(
        "<text x=\"{:.1}\" y=\"{:.1}\" font-family=\"Segoe UI,Arial\" font-size=\"9\" text-anchor=\"middle\" fill=\"#5b6678\">{}</text>",
        (left + right) / 2.0,
        h - 8.0,
        xml_escape(&chart.x_label)
    ));
    s.push_str(&format!(
        "<text transform=\"translate(10,{:.1}) rotate(-90)\" font-family=\"Segoe UI,Arial\" font-size=\"9\" text-anchor=\"middle\" fill=\"#5b6678\">{}</text>",
        (top + bottom) / 2.0,
        xml_escape(&chart.y_label)
    ));
    s.push_str("</svg>");
    s
}

fn xml_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// A pivot table as HTML, with the spanning header and the measures.
///
/// The header is one band per column dimension, and the cells of the
/// outer dimension span the inner ones, so the exported table reads
/// the same way the table on the screen does.
fn pivot_to_html(pivot: &crate::pivot::PivotTable) -> String {
    use crate::pivot::label;

    let mut h = String::new();
    let row_keys = pivot.row_keys();
    let col_keys = pivot.col_keys();
    let measure = pivot.measure_index.min(pivot.measures.len().saturating_sub(1));

    // The measures are tabs, as they are in SPSS's output tree.
    if pivot.measures.len() > 1 {
        h.push_str("<p class=\"measures\">");
        for (i, m) in pivot.measures.iter().enumerate() {
            if i == measure {
                h.push_str(&format!("<strong>{}</strong>", html_escape(m)));
            } else {
                h.push_str(&format!("<span>{}</span>", html_escape(m)));
            }
            if i + 1 < pivot.measures.len() {
                h.push_str(" &middot; ");
            }
        }
        h.push_str("</p>\n");
    }

    h.push_str("<table>\n");
    // The header: one row per column dimension.
    for (d, dim) in pivot.col_dims.iter().enumerate() {
        h.push_str("<tr>");
        // The corner cell, over the stub.
        if d == 0 {
            h.push_str(&format!(
                "<th rowspan=\"{}\" class=\"corner\">{}</th>",
                pivot.col_dims.len(),
                html_escape(&dim.name)
            ));
        }
        // The cells of this dimension, each spanning the columns below.
        let below: usize = pivot.col_dims[d + 1..]
            .iter()
            .map(|x| x.levels.len().max(1))
            .product::<usize>()
            .max(1);
        let mut j = 0;
        while j < col_keys.len() {
            let level = dim.levels.get(col_keys[j].get(d).copied().unwrap_or(0));
            let span = below.min(col_keys.len() - j);
            h.push_str(&format!(
                "<th colspan=\"{}\">{}</th>",
                span,
                html_escape(&level.cloned().unwrap_or_default())
            ));
            j += span;
        }
        h.push_str("</tr>\n");
    }
    // The body.
    for rk in &row_keys {
        h.push_str("<tr>");
        h.push_str(&format!(
            "<td class=\"stub\">{}</td>",
            html_escape(&label(&pivot.row_dims, rk))
        ));
        for ck in &col_keys {
            let v = pivot.measure_at(rk, ck, measure);
            let esc = html_escape(v);
            if v.trim().parse::<f64>().is_ok() {
                h.push_str(&format!("<td class=\"num\">{}</td>", esc));
            } else {
                h.push_str(&format!("<td>{}</td>", esc));
            }
        }
        h.push_str("</tr>\n");
    }
    h.push_str("</table>\n");
    h
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
        let mut app = StatsCraftApp::default();
        app.outputs.push(crate::output::OutputTable {
            title: "T<1>".into(),
            headers: vec!["A".into(), "B".into()],
            rows: vec![vec!["x".into(), "1.5".into()]],
            note: "n".into(),
            chart: None,
            pivot: None,
        });
        let path = std::env::temp_dir().join("sc_out.html");
        app.export_output_html_to(&path);
        let s = std::fs::read_to_string(&path).unwrap();
        assert!(s.contains("T&lt;1&gt;"), "title escaped");
        assert!(s.contains("<td class=\"num\">1.5</td>"), "numeric right-aligned");
        assert!(s.contains("<p class=\"note\">n</p>"), "note rendered");
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn a_case_can_be_added_and_deleted_from_the_app() {
        let mut app = StatsCraftApp::default();
        app.table = Some(crate::data::DataTable {
            n_rows: 2,
            columns: vec![crate::data::Column {
                name: "x".into(),
                kind: crate::data::ColumnKind::Numeric,
                values: vec![
                    crate::data::Value::Number(1.0),
                    crate::data::Value::Number(2.0),
                ],
                meta: crate::data::VariableMeta::default_for(
                    crate::data::ColumnKind::Numeric,
                ),
            }],
        });
        // The new case is at the end, and the selection moves to it.
        app.add_case();
        assert_eq!(app.table.as_ref().unwrap().n_rows, 3);
        assert_eq!(app.sel_cell, Some((0, 2)));
        // Deleting it takes the case away again.
        app.delete_case();
        assert_eq!(app.table.as_ref().unwrap().n_rows, 2);
        // Deleting with nothing selected is refused.
        app.sel_cell = None;
        app.delete_case();
        assert_eq!(app.table.as_ref().unwrap().n_rows, 2);
        assert!(app.status.contains("Select a case"));
    }

    #[test]
    fn the_html_export_carries_the_pivot_structure() {
        // A pivot table is exported with the spanning header and the
        // measures, so the structure survives rather than being
        // flattened away.
        let ct = crate::stats::CrosstabsResult {
            var1: "region".into(),
            var2: "group".into(),
            levels1: vec!["East".into(), "West".into()],
            levels2: vec!["A".into(), "B".into()],
            observed: vec![vec![10, 5], vec![5, 20]],
            row_totals: vec![15, 25],
            col_totals: vec![15, 25],
            n: 40,
            chi2: 8.7111,
            df: 1,
            p: 0.00316,
            expected: vec![vec![5.625, 9.375], vec![9.375, 15.625]],
            phi: 0.4667,
            cramers_v: 0.4667,
            min_expected: 5.625,
            linear_by_linear: None,
            fisher_exact: None,
        };
        let mut app = StatsCraftApp::default();
        app.outputs.push(crate::output::OutputTable::crosstabs_with(
            &ct,
            &crate::stats::CellCounts {
                observed: true,
                expected: true,
                row_percent: true,
                column_percent: true,
                total_percent: true,
            },
            &Default::default(),
        ));
        let path = std::env::temp_dir().join("sc_pivot.html");
        app.export_output_html_to(&path);
        let s = std::fs::read_to_string(&path).unwrap();
        // The column dimension's name is in the corner cell.
        assert!(s.contains("group"), "the column dimension is named");
        // The levels are in the header.
        assert!(s.contains(">A</th>"), "the levels are in the header");
        // The measures are listed.
        assert!(s.contains("Expected Count"), "the measures are listed");
        // The counts are there.
        assert!(s.contains(">10</td>"), "the counts are exported");
        let _ = std::fs::remove_file(path);
    }
}

/// SPSS pivot-table look: bordered cells, shaded header band, right-aligned
/// numerics, footnote note beneath the table.

/// Draw a chart in the Output Viewer, under the pivot table.
///
/// The chart is painted directly rather than through a plotting library,
/// so it always fits the space it is given and the axis labels can be
/// the category names.
/// Draw a chart. Public so the Chart Builder can show a preview of
/// the chart the choices make.
pub fn render_chart_preview(ui: &mut egui::Ui, chart: &crate::output::Chart) {
    let _ = render_output_chart(ui, chart);
}

fn render_output_chart(ui: &mut egui::Ui, chart: &crate::output::Chart) -> egui::Rect {
    use crate::theme;
    use crate::output::ChartKind;

    ui.add_space(6.0);
    ui.label(
        egui::RichText::new(&chart.title)
            .font(theme::semibold(13.0))
            .color(theme::text()),
    );
    ui.add_space(4.0);

    let width = ui.available_width().min(720.0).max(320.0);
    let height = 240.0;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
    let painter = ui.painter();

    // A light card behind the plot.
    painter.rect_filled(rect, theme::RADIUS, theme::surface());
    painter.rect_stroke(rect, theme::RADIUS, egui::Stroke::new(1.0, theme::border()));

    // The plot area inside the card, leaving room for the axis labels.
    let left = rect.left() + 52.0;
    let right = rect.right() - 12.0;
    let top = rect.top() + 12.0;
    let bottom = rect.bottom() - 34.0;
    let plot_rect = egui::Rect::from_min_max(
        egui::pos2(left, top),
        egui::pos2(right, bottom),
    );

    // The data range.
    let mut max_y = 0.0f64;
    let mut min_y = 0.0f64;
    let mut max_x = 0.0f64;
    for s in &chart.series {
        for y in &s.y {
            if *y > max_y {
                max_y = *y;
            }
            if *y < min_y {
                min_y = *y;
            }
        }
        for x in &s.x {
            if *x > max_x {
                max_x = *x;
            }
        }
    }
    if max_y <= min_y {
        max_y = min_y + 1.0;
    }
    if max_x <= 0.0 {
        max_x = 1.0;
    }
    // A little headroom above the tallest bar.
    let y_max = max_y * 1.1;

    let to_px = |x: f64, y: f64| -> egui::Pos2 {
        egui::pos2(
            plot_rect.left() + ((x / max_x) * plot_rect.width() as f64) as f32,
            plot_rect.bottom() - (((y - min_y) / (y_max - min_y)) * plot_rect.height() as f64) as f32,
        )
    };

    // ---- The grid and the y axis ----
    let grid = egui::Stroke::new(0.5, theme::border());
    let y_ticks = 5;
    for i in 0..=y_ticks {
        let t = i as f64 / y_ticks as f64;
        let y = plot_rect.bottom() - (t * plot_rect.height() as f64) as f32;
        painter.line_segment(
            [egui::pos2(plot_rect.left(), y), egui::pos2(plot_rect.right(), y)],
            grid,
        );
        let value = min_y + t * (y_max - min_y);
        // The number of decimals follows the range, so an axis from 0
        // to 1 reads "0.0, 0.2, ..." rather than "0, 0, 0, 1, 1, 1".
        let decimals = if (y_max - min_y).abs() <= 2.5 {
            2
        } else if (y_max - min_y).abs() <= 20.0 {
            1
        } else {
            0
        };
        painter.text(
            egui::pos2(plot_rect.left() - 6.0, y),
            egui::Align2::RIGHT_CENTER,
            format!("{:.*}", decimals, value),
            theme::body(10.5),
            theme::text_faint(),
        );
    }
    // The x axis line.
    painter.line_segment(
        [
            egui::pos2(plot_rect.left(), plot_rect.bottom()),
            egui::pos2(plot_rect.right(), plot_rect.bottom()),
        ],
        egui::Stroke::new(1.0, theme::border_strong()),
    );

    // ---- The series ----
    let labels: Vec<String> = chart
        .series
        .first()
        .map(|s| s.x_labels.clone())
        .unwrap_or_default();

    for (si, s) in chart.series.iter().enumerate() {
        let color = series_color(si);

        match chart.kind {
            ChartKind::Bar | ChartKind::Histogram => {
                // The bars are spread across the plot area.
                let n = s.x.len().max(1) as f64;
                let slot = plot_rect.width() as f64 / n;
                let bar_w = (slot * 0.62).min(70.0) as f32;
                for (i, (&x, &y)) in s.x.iter().zip(s.y.iter()).enumerate() {
                    // The bar is centred on its slot.
                    let centre = plot_rect.left() + ((i as f64 + 0.5) * slot) as f32;
                    let top_y = to_px(x, y).y;
                    let base_y = to_px(x, min_y).y;
                    let bar_rect = egui::Rect::from_min_max(
                        egui::pos2(centre - bar_w / 2.0, top_y.min(base_y)),
                        egui::pos2(centre + bar_w / 2.0, base_y),
                    );
                    painter.rect_filled(bar_rect, 3.0, color);

                    // The category label under the bar.
                    if let Some(label) = labels.get(i) {
                        painter.text(
                            egui::pos2(centre, plot_rect.bottom() + 6.0),
                            egui::Align2::CENTER_TOP,
                            label,
                            theme::body(10.5),
                            theme::text_muted(),
                        );
                    }
                }
            }
            ChartKind::Scatter => {
                for (&x, &y) in s.x.iter().zip(s.y.iter()) {
                    let p = to_px(x, y);
                    painter.circle_filled(p, 3.5, color);
                }
            }
            ChartKind::Line => {
                // The points are joined in order, so the curve is
                // smooth. The stroke is drawn under the markers, so a
                // marker sits on the line.
                let pts: Vec<egui::Pos2> = s.x.iter().zip(s.y.iter()).map(|(&x, &y)| to_px(x, y)).collect();
                for w in pts.windows(2) {
                    painter.line_segment([w[0], w[1]], egui::Stroke::new(2.0, color));
                }
                for p in &pts {
                    painter.circle_filled(*p, 2.5, color);
                }
            }
            ChartKind::BoxPlot => {
                for (i, (&x, &y)) in s.x.iter().zip(s.y.iter()).enumerate() {
                    let p = to_px(x, y);
                    painter.circle_filled(p, 2.5, color.gamma_multiply(0.5));
                }
                if let Some((q1, med, q3, _)) = s.quartiles {
                    let centre = plot_rect.left() + plot_rect.width() / 2.0;
                    let w = 44.0;
                    let p1 = egui::pos2(centre, to_px(0.0, q1).y);
                    let p2 = egui::pos2(centre, to_px(0.0, q3).y);
                    let p3 = egui::pos2(centre, to_px(0.0, med).y);
                    let stroke = egui::Stroke::new(1.5, color);
                    painter.line_segment([p1, p2], stroke);
                    painter.line_segment(
                        [egui::pos2(centre - w / 2.0, p1.y), egui::pos2(centre + w / 2.0, p1.y)],
                        stroke,
                    );
                    painter.line_segment(
                        [egui::pos2(centre - w / 2.0, p2.y), egui::pos2(centre + w / 2.0, p2.y)],
                        stroke,
                    );
                    painter.line_segment(
                        [egui::pos2(centre - w / 2.0, p3.y), egui::pos2(centre + w / 2.0, p3.y)],
                        egui::Stroke::new(2.0, color),
                    );
                }
            }
        }
    }

    // ---- The reference line ----
    // A horizontal line at a chosen value, with its label at the
    // right-hand end.
    if let Some(value) = chart.reference_line {
        if value >= min_y && value <= y_max {
            let y = plot_rect.bottom() - (((value - min_y) / (y_max - min_y)) * plot_rect.height() as f64) as f32;
            // The line is dashed, so it reads as an annotation rather
            // than as part of the data.
            let mut x = plot_rect.left();
            while x < plot_rect.right() {
                let x2 = (x + 7.0).min(plot_rect.right());
                painter.line_segment(
                    [egui::pos2(x, y), egui::pos2(x2, y)],
                    egui::Stroke::new(1.4, theme::amber()),
                );
                x += 12.0;
            }
            if !chart.reference_label.is_empty() {
                painter.text(
                    egui::pos2(plot_rect.right() - 4.0, y - 9.0),
                    egui::Align2::RIGHT_BOTTOM,
                    &chart.reference_label,
                    theme::semibold(10.5),
                    theme::amber(),
                );
            }
        }
    }

    // The axis titles.
    painter.text(
        egui::pos2((plot_rect.left() + plot_rect.right()) / 2.0, rect.bottom() - 8.0),
        egui::Align2::CENTER_BOTTOM,
        &chart.x_label,
        theme::medium(10.5),
        theme::text_muted(),
    );
    painter.text(
        egui::pos2(rect.left() + 6.0, (plot_rect.top() + plot_rect.bottom()) / 2.0),
        egui::Align2::LEFT_CENTER,
        &chart.y_label,
        theme::medium(10.5),
        theme::text_muted(),
    );
    rect
}

/// The colour of the nth series.
///
/// The first three come from the theme, so a chart's series follow the
/// palette of whichever theme is active; the last two are fixed, so
/// there is always a fifth and sixth colour that no theme collides
/// with.
fn series_color(i: usize) -> egui::Color32 {
    match i % 5 {
        0 => crate::theme::accent(),
        1 => crate::theme::teal(),
        2 => crate::theme::amber(),
        3 => egui::Color32::from_rgb(140, 110, 220),
        _ => egui::Color32::from_rgb(220, 110, 140),
    }
}

/// Draw a pivot table, with the multi-level header SPSS builds.
///
/// A pivot table has a header for each column dimension, and the cells
/// of the outer dimension span the cells of the inner one. The body
/// is the grid of cells, and the stub is the row dimension.
///
/// The table is drawn with a fixed column layout rather than with the
/// `Grid` widget, because a spanning cell needs to know the width of
/// the columns it covers.
fn render_pivot_table(ui: &mut egui::Ui, pivot: &crate::pivot::PivotTable) {
    use crate::theme;
    use crate::pivot::label;

    let row_keys = pivot.row_keys();
    let col_keys = pivot.col_keys();
    let n_rows = row_keys.len();
    let n_cols = col_keys.len();
    if n_rows == 0 || n_cols == 0 {
        return;
    }

    // ---- The measures, and which one is shown ----
    // A pivot holds several measures of the same cells - the counts,
    // the expected counts, the percentages. SPSS shows them as
    // separate tables under the crosstabulation, so the reader picks
    // which one to look at.
    let mut measure = pivot.measure_index.min(pivot.measures.len().saturating_sub(1));
    if pivot.measures.len() > 1 {
        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new("Show:").size(11.5).color(theme::text_muted()));
            for (i, m) in pivot.measures.iter().enumerate() {
                let selected = i == measure;
                if ui
                    .selectable_label(selected, egui::RichText::new(m).size(11.5))
                    .clicked()
                {
                    measure = i;
                }
            }
        });
        ui.add_space(4.0);
    }

    // ---- The column widths ----
    // The stub holds the row dimension's label; the columns hold the
    // values. A header label is centred over the columns it spans, so
    // it needs a share of their width rather than all of it.
    let stub_text: Vec<String> = row_keys
        .iter()
        .map(|k| label(&pivot.row_dims, k))
        .collect();
    // The text is measured through the font atlas, which is how egui
    // sizes a label.
    let measure_text = |s: &str, font: egui::FontId| -> f32 {
        ui.ctx().fonts(|f| {
            f.layout_no_wrap(s.to_string(), font, egui::Color32::PLACEHOLDER)
                .size()
                .x
        })
    };
    let mut col_widths: Vec<f32> = vec![0.0; n_cols];
    for (j, ck) in col_keys.iter().enumerate() {
        let mut w = 0.0f32;
        for rk in &row_keys {
            let v = pivot.measure_at(rk, ck, measure);
            w = w.max(measure_text(v, theme::body(12.5)));
        }
        col_widths[j] = w;
    }
    // The header: for each dimension, the widest label sets the width
    // of the columns it covers, divided by how many it covers.
    for (d, dim) in pivot.col_dims.iter().enumerate() {
        // The number of leaf columns one level of this dimension
        // covers.
        let below: usize = pivot.col_dims[d + 1..]
            .iter()
            .map(|x| x.levels.len().max(1))
            .product::<usize>()
            .max(1);
        for (li, level) in dim.levels.iter().enumerate() {
            let w = measure_text(level, theme::semibold(12.0));
            let share = w / below as f32;
            // The first column of the group carries the whole label;
            // the rest are the same group and need no more.
            for j in 0..n_cols {
                if col_keys[j].get(d) == Some(&li) {
                    let first = col_keys[..j]
                        .iter()
                        .filter(|ck| ck.get(d) == Some(&li))
                        .count();
                    if first == 0 {
                        col_widths[j] = col_widths[j].max(share);
                    }
                }
            }
        }
    }
    let stub_width = stub_text
        .iter()
        .map(|t| measure_text(t, theme::medium(12.5)))
        .fold(0.0f32, f32::max)
        .max(90.0)
        + 20.0;
    for w in col_widths.iter_mut() {
        *w += 20.0;
    }
    let total_width: f32 = stub_width + col_widths.iter().sum::<f32>();

    egui::ScrollArea::horizontal()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let header_h = 24.0;
            let cell_h = 22.0;
            let header_rows = pivot.col_dims.len().max(1);
            let body_h = n_rows as f32 * cell_h;
            let height = header_rows as f32 * header_h + body_h;
            let (rect, _) =
                ui.allocate_exact_size(egui::vec2(total_width, height), egui::Sense::hover());
            let painter = ui.painter();

            let edge = theme::border();
            let header_fill = theme::header();
            let stub_fill = theme::surface_alt();

            // The x of each column, and the x of the stub.
            let mut xs = vec![rect.left() + stub_width];
            for w in &col_widths {
                xs.push(xs.last().unwrap() + w);
            }

            // ---- The header ----
            // The outer dimension's cells span the inner ones, so the
            // header is drawn as one band per dimension.
            for d in 0..header_rows {
                let y = rect.top() + d as f32 * header_h;
                // The stub's corner cell.
                painter.rect_filled(
                    egui::Rect::from_min_max(
                        egui::pos2(rect.left(), y),
                        egui::pos2(rect.left() + stub_width, y + header_h),
                    ),
                    0.0,
                    header_fill,
                );
                painter.rect_stroke(
                    egui::Rect::from_min_max(
                        egui::pos2(rect.left(), y),
                        egui::pos2(rect.left() + stub_width, y + header_h),
                    ),
                    0.0,
                    egui::Stroke::new(0.5, edge),
                );
                if d == 0 {
                    if let Some(dim) = pivot.col_dims.first() {
                        painter.text(
                            egui::pos2(rect.left() + stub_width - 8.0, y + header_h / 2.0),
                            egui::Align2::RIGHT_CENTER,
                            &dim.name,
                            theme::semibold(12.0),
                            theme::text(),
                        );
                    }
                }
                // The cells of this dimension.
                let mut j = 0;
                while j < n_cols {
                    let level = match pivot.col_dims.get(d) {
                        Some(dim) => dim.levels.get(col_keys[j].get(d).copied().unwrap_or(0)),
                        None => None,
                    };
                    let label_text = level.cloned().unwrap_or_default();
                    // How many columns this cell spans: the ones below
                    // it in the dimension stack.
                    let below: usize = pivot.col_dims[d + 1..]
                        .iter()
                        .map(|x| x.levels.len().max(1))
                        .product::<usize>()
                        .max(1);
                    let span = below.min(n_cols - j);
                    // The band is centred over the columns it covers.
                    let x0 = xs[j];
                    let x1 = xs[j + span];
                    let cell = egui::Rect::from_min_max(
                        egui::pos2(x0, y),
                        egui::pos2(x1, y + header_h),
                    );
                    painter.rect_filled(cell, 0.0, header_fill);
                    painter.rect_stroke(cell, 0.0, egui::Stroke::new(0.5, edge));
                    painter.text(
                        egui::pos2((x0 + x1) / 2.0, y + header_h / 2.0),
                        egui::Align2::CENTER_CENTER,
                        &label_text,
                        theme::semibold(12.0),
                        theme::text(),
                    );
                    j += span;
                }
            }

            // ---- The body ----
            let body_top = rect.top() + header_rows as f32 * header_h;
            for (i, rk) in row_keys.iter().enumerate() {
                let y = body_top + i as f32 * cell_h;
                // The stub.
                let stub = egui::Rect::from_min_max(
                    egui::pos2(rect.left(), y),
                    egui::pos2(rect.left() + stub_width, y + cell_h),
                );
                painter.rect_filled(stub, 0.0, stub_fill);
                painter.rect_stroke(stub, 0.0, egui::Stroke::new(0.5, edge));
                painter.text(
                    egui::pos2(rect.left() + 8.0, y + cell_h / 2.0),
                    egui::Align2::LEFT_CENTER,
                    &stub_text[i],
                    theme::medium(12.5),
                    theme::text(),
                );
                // The cells.
                for (j, ck) in col_keys.iter().enumerate() {
                    let v = pivot.measure_at(rk, ck, measure);
                    let cell = egui::Rect::from_min_max(
                        egui::pos2(xs[j], y),
                        egui::pos2(xs[j + 1], y + cell_h),
                    );
                    painter.rect_filled(cell, 0.0, theme::cell());
                    painter.rect_stroke(cell, 0.0, egui::Stroke::new(0.5, edge));
                    // A number is right-aligned; a label is centred.
                    let is_num = v.parse::<f64>().is_ok();
                    let align = if is_num {
                        egui::Align2::RIGHT_CENTER
                    } else {
                        egui::Align2::CENTER_CENTER
                    };
                    painter.text(
                        egui::pos2(cell.right() - 8.0, cell.center().y),
                        align,
                        v,
                        theme::body(12.5),
                        theme::text(),
                    );
                }
            }
        });
}

fn render_output_table(ui: &mut egui::Ui, t: &OutputTable) {
    use crate::theme;
    // A table with a pivot behind it is drawn as one, with the
    // multi-level header.
    if let Some(p) = &t.pivot {
        render_pivot_table(ui, p);
        if !t.note.is_empty() {
            ui.add_space(3.0);
            ui.label(egui::RichText::new(&t.note).size(10.5).weak());
        }
        return;
    }
    let header_fill = theme::header();
    let edge = theme::border();
    let stub_fill = theme::surface_alt();
    // A wide table gets a horizontal scrollbar, so that a table with
    // many columns stays readable instead of running off the panel.
    egui::ScrollArea::horizontal()
        .auto_shrink([false, false])
        .show(ui, |ui| {
    egui::Grid::new(&t.title)
        .striped(false)
        .spacing([0.0, 0.0])
        .show(ui, |ui| {
            for h in &t.headers {
                egui::Frame::none()
                    .fill(header_fill)
                    .stroke(egui::Stroke::new(0.5, edge))
                    .inner_margin(egui::Margin::symmetric(10.0, 5.0))
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width().min(96.0));
                        ui.label(
                            egui::RichText::new(h)
                                .font(crate::theme::semibold(12.0))
                                .color(theme::text()),
                        );
                    });
            }
            ui.end_row();
            for row in &t.rows {
                for (k, cell) in row.iter().enumerate() {
                    let is_num = k > 0 && cell.parse::<f64>().is_ok();
                    egui::Frame::none()
                        .fill(if k == 0 { stub_fill } else { theme::cell() })
                        .stroke(egui::Stroke::new(0.5, edge))
                        .inner_margin(egui::Margin::symmetric(10.0, 4.0))
                        .show(ui, |ui| {
                            ui.set_min_width(ui.available_width().min(96.0));
                            ui.with_layout(
                                if is_num {
                                    egui::Layout::right_to_left(egui::Align::Center)
                                } else {
                                    egui::Layout::left_to_right(egui::Align::Center)
                                },
                                |ui| {
                                    let rt = egui::RichText::new(cell).size(12.5);
                                    let rt = if k == 0 {
                                        rt.font(crate::theme::medium(12.5)).color(theme::text())
                                    } else {
                                        rt.color(theme::text())
                                    };
                                    ui.label(rt);
                                },
                            );
                        });
                }
                ui.end_row();
            }
        });
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

fn render_variables(ui: &mut egui::Ui, table: &mut DataTable, add_variable: &mut bool) {
    use crate::data::{Measure, VarAlign, ColumnKind};
    use crate::theme;
    let header_fill = theme::header();
    let cell_fill = theme::cell();
    let header_text = theme::text();

    // SPSS's Variable View can always make a new row. The button sits
    // above the grid so it is visible both for an empty dataset and for a
    // loaded one.
    ui.horizontal(|ui| {
        if ui
            .add(
                egui::Button::new(
                    egui::RichText::new("＋ New Variable")
                        .font(theme::medium(12.0))
                        .color(theme::text_on_accent()),
                )
                .fill(theme::accent())
                .rounding(theme::RADIUS_SM)
                .min_size(egui::vec2(126.0, 26.0)),
            )
            .on_hover_text("Add a numeric variable at the end of the dataset.")
            .clicked()
        {
            *add_variable = true;
        }
        if table.columns.is_empty() {
            ui.label(
                egui::RichText::new("No variables yet — click New Variable to define the first one.")
                    .color(theme::text_muted())
                    .size(12.0),
            );
        } else {
            ui.label(
                egui::RichText::new(format!("{} variable(s)", table.columns.len()))
                    .color(theme::text_faint())
                    .size(11.5),
            );
        }
    });
    ui.add_space(5.0);

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
                                egui::RichText::new(h)
                                    .font(crate::theme::medium(12.0))
                                    .color(header_text),
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

/// The StatsCraft application icon.
///
/// It is the same mark as the packaged one - a rounded square in the
/// Dracula background, three ascending bars in the brand accents, and
/// a rising plot line across them - drawn into a pixel buffer at
/// run time so the build stays free of image assets. The shapes are
/// scaled from the 64-pixel design, so the icon is crisp at every
/// size Windows asks for.
fn app_icon() -> egui::IconData {
    const S: usize = 64;
    // The background, the three bar colours and the line.
    const BG: (u8, u8, u8) = (40, 42, 54);
    const VIOLET: (u8, u8, u8) = (189, 147, 249);
    const BLUE: (u8, u8, u8) = (47, 111, 237);
    const MID: (u8, u8, u8) = (74, 154, 245);
    const TEAL: (u8, u8, u8) = (15, 168, 126);
    const FG: (u8, u8, u8) = (248, 248, 242);
    // The bars: (x start, width, height, colour).
    const BARS: [(usize, usize, usize, (u8, u8, u8)); 3] = [
        (12, 10, 19, BLUE),
        (27, 10, 27, MID),
        (42, 10, 36, TEAL),
    ];
    // The plot line's points, over the bars.
    const POINTS: [(usize, usize); 3] = [(17, 24), (32, 16), (47, 7)];

    let mut rgba = vec![0u8; S * S * 4];
    let put = |rgba: &mut Vec<u8>, x: usize, y: usize, c: (u8, u8, u8), a: u8| {
        if x < S && y < S {
            let i = (y * S + x) * 4;
            // The colour is composited over what is there, so the
            // glow and the bars can be layered.
            let old_a = rgba[i + 3] as f32 / 255.0;
            let new_a = a as f32 / 255.0;
            let out_a = new_a + old_a * (1.0 - new_a);
            if out_a <= 0.0 {
                return;
            }
            let channels = [c.0, c.1, c.2];
            for k in 0..3 {
                let src = channels[k] as f32 * new_a;
                let dst = rgba[i + k] as f32 * old_a * (1.0 - new_a);
                rgba[i + k] = ((src + dst) / out_a).round() as u8;
            }
            rgba[i + 3] = (out_a * 255.0).round() as u8;
        }
    };

    for y in 0..S {
        for x in 0..S {
            let fx = x as f32 + 0.5;
            let fy = y as f32 + 0.5;
            // The rounded square's signed distance.
            let half = S as f32 / 2.0;
            let corner = 14.0;
            let qx = (fx - half).abs() - (half - corner);
            let qy = (fy - half).abs() - (half - corner);
            let outside = (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt();
            let d = outside + qx.max(qy).min(0.0) - corner;
            if d > 0.0 {
                continue; // transparent outside the tile
            }
            put(&mut rgba, x, y, BG, 255);

            // The violet glow in the top-left.
            let gx = fx - 4.0;
            let gy = fy + 6.0;
            let glow = (1.0 - (gx * gx + gy * gy).sqrt() / 34.0).clamp(0.0, 1.0);
            if glow > 0.0 {
                put(&mut rgba, x, y, VIOLET, (glow * 44.0) as u8);
            }
            // The violet edge.
            if d > -1.4 {
                put(&mut rgba, x, y, VIOLET, 120);
            }
        }
    }

    // The bars.
    for (x0, w, h, c) in BARS.iter() {
        for y in (50 - h)..50 {
            for x in *x0..(x0 + w) {
                put(&mut rgba, x, y, *c, 255);
            }
        }
    }
    // The baseline.
    for y in 49..52 {
        for x in 11..53 {
            put(&mut rgba, x, y, FG, 220);
        }
    }
    // The plot line: a thick line between the points, then the points
    // themselves.
    for i in 0..POINTS.len() - 1 {
        let (x0, y0) = POINTS[i];
        let (x1, y1) = POINTS[i + 1];
        let steps = (x1 as i32 - x0 as i32).abs().max((y1 as i32 - y0 as i32).abs());
        for s in 0..=steps {
            let t = s as f32 / steps as f32;
            let px = (x0 as f32 + (x1 as f32 - x0 as f32) * t).round() as usize;
            let py = (y0 as f32 + (y1 as f32 - y0 as f32) * t).round() as usize;
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    let nx = px as i32 + dx;
                    let ny = py as i32 + dy;
                    if nx >= 0 && ny >= 0 {
                        put(&mut rgba, nx as usize, ny as usize, FG, 235);
                    }
                }
            }
        }
    }
    for (cx, cy) in POINTS.iter() {
        for y in cy.saturating_sub(2)..=(cy + 2).min(S - 1) {
            for x in cx.saturating_sub(2)..=(cx + 2).min(S - 1) {
                let dx = x as i32 - *cx as i32;
                let dy = y as i32 - *cy as i32;
                if dx * dx + dy * dy <= 7 {
                    put(&mut rgba, x, y, FG, 255);
                }
            }
        }
    }

    egui::IconData {
        rgba,
        width: S as u32,
        height: S as u32,
    }
}

pub fn run(initial_path: Option<String>) -> eframe::Result<()> {
    run_with(initial_path, None, None, false)
}

/// Run the app, optionally opening a procedure dialog on start.
///
/// The dialog argument exists so the app can be launched straight into a
/// procedure (handy for scripting, demos and screenshots) without going
/// through the Analyze menu.
pub fn run_with(
    initial_path: Option<String>,
    initial_dialog: Option<crate::dialog::Procedure>,
    initial_script: Option<String>,
    dark: bool,
) -> eframe::Result<()> {
    // Load the remembered theme and recent files. `--dark` is an explicit
    // override for demos and screenshots; otherwise the last choice wins.
    let settings = crate::settings::load();
    let dark = dark || settings.dark;

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_icon(app_icon())
            // Fit the window inside the work area so the tab strip and the
            // status bar are never pushed off the bottom of the screen.
            .with_inner_size(egui::vec2(1360.0, 780.0))
            .with_min_inner_size(egui::vec2(820.0, 480.0)),
        ..Default::default()
    };
    eframe::run_native(
        "StatsCraft",
        options,
        Box::new(move |cc| {
            // Install the bundled Inter family and the modern look before the
            // first frame, so nothing is ever painted with the old defaults.
            crate::theme::install_fonts(&cc.egui_ctx);
            crate::theme::apply_style(&cc.egui_ctx, dark);

            let mut app = StatsCraftApp::default();
            // The real egui context, so the theme can be re-applied when
            // the user switches it.
            app.ctx = cc.egui_ctx.clone();
            app.dark = dark;
            app.recent_files = settings.recent_files;
            if let Some(p) = initial_path {
                app.path_input = p;
                app.load();
            }
            if let Some(proc) = initial_dialog {
                app.dialog = Some(crate::dialog::DialogState::new(proc));
            }
            // A syntax file to run at start-up.
            if let Some(script_path) = initial_script {
                match std::fs::read_to_string(&script_path) {
                    Ok(text) => {
                        app.syntax_text = text.clone();
                        crate::syntax::run(&mut app, &text);
                    }
                    Err(e) => {
                        app.status = format!("Could not read {}: {}", script_path, e);
                    }
                }
            }
            Ok(Box::new(app))
        }),
    )
}
