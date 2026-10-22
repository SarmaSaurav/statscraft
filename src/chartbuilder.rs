//! The Chart Builder: the dialog that makes a chart.
//!
//! SPSS does not draw a chart straight from a procedure. The Chart
//! Builder is where the reader chooses the type of the chart, the
//! variables it uses, how the data is summarised, the labels, and any
//! reference line — and the chart is then drawn from those choices.
//!
//! This is the same thing. The builder holds the choices, and the
//! `build` function turns them into a `Chart`, which the Output
//! Viewer draws and the export carries.

use crate::data::{ColumnKind, DataTable, Value};
use crate::output::{Chart, ChartKind, ChartSeries};

/// The kind of chart the builder makes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChartType {
    /// A bar chart of the counts, or of a summary by a group.
    Bar,
    /// A histogram of a numeric variable.
    Histogram,
    /// A scatter plot of two numeric variables.
    Scatter,
    /// A box plot of a numeric variable, by a group.
    BoxPlot,
}

impl ChartType {
    /// The name shown in the dialog.
    pub fn title(self) -> &'static str {
        match self {
            ChartType::Bar => "Bar",
            ChartType::Histogram => "Histogram",
            ChartType::Scatter => "Scatter",
            ChartType::BoxPlot => "Box Plot",
        }
    }

    /// The kinds, in the order the dialog lists them.
    pub fn all() -> [ChartType; 4] {
        [
            ChartType::Bar,
            ChartType::Histogram,
            ChartType::Scatter,
            ChartType::BoxPlot,
        ]
    }
}

/// How a bar chart summarises the data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BarSummary {
    /// The count of the cases in each category.
    Count,
    /// The mean of a numeric variable, by a group.
    Mean,
    /// The median of a numeric variable, by a group.
    Median,
    /// The sum of a numeric variable, by a group.
    Sum,
}

impl BarSummary {
    pub fn title(self) -> &'static str {
        match self {
            BarSummary::Count => "Count",
            BarSummary::Mean => "Mean",
            BarSummary::Median => "Median",
            BarSummary::Sum => "Sum",
        }
    }

    pub fn all() -> [BarSummary; 4] {
        [
            BarSummary::Count,
            BarSummary::Mean,
            BarSummary::Median,
            BarSummary::Sum,
        ]
    }
}

/// The state of the Chart Builder.
#[derive(Debug, Clone, Default)]
pub struct ChartBuilder {
    /// The kind of chart.
    pub chart_type: Option<ChartType>,
    /// The variable the chart is about.
    pub y_var: Option<String>,
    /// The second variable, for a scatter plot.
    pub x_var: Option<String>,
    /// The grouping variable, for a bar chart or a box plot.
    pub group_var: Option<String>,
    /// How a bar chart summarises the data.
    pub summary: Option<BarSummary>,
    /// The number of bins of a histogram.
    pub bins: usize,
    /// The title of the chart.
    pub title: String,
    /// The label of the horizontal axis.
    pub x_label: String,
    /// The label of the vertical axis.
    pub y_label: String,
    /// A reference line at this value, if there is one.
    pub reference_line: Option<f64>,
    /// The label of the reference line.
    pub reference_label: String,
    /// Set when the user pressed OK.
    pub confirmed: bool,
    /// Set when the user pressed Cancel or closed the window, so the
    /// caller knows to drop the builder rather than keep it open.
    pub cancelled: bool,
}

impl ChartBuilder {
    /// A builder with the defaults filled in for a table.
    pub fn for_table(table: &DataTable) -> Self {
        let mut b = ChartBuilder {
            chart_type: Some(ChartType::Bar),
            bins: 20,
            ..Default::default()
        };
        // The default category is a variable that makes a readable
        // bar chart: one with a handful of distinct values. The first
        // string column will often be an identifier, with one value
        // per case, and a chart of that says nothing.
        let cat = table
            .columns
            .iter()
            .filter(|c| c.kind == ColumnKind::String)
            .map(|c| (c.name.clone(), distinct_values(c, table.n_rows)))
            .filter(|(_, d)| *d >= 2 && *d <= 20)
            .min_by_key(|(_, d)| *d)
            .map(|(n, _)| n)
            .or_else(|| {
                table
                    .columns
                    .iter()
                    .find(|c| c.kind == ColumnKind::String)
                    .map(|c| c.name.clone())
            });
        // The default numeric variable is the first one that is not
        // an identifier, for the same reason.
        let num = table
            .columns
            .iter()
            .filter(|c| c.kind == ColumnKind::Numeric)
            .map(|c| (c.name.clone(), distinct_values(c, table.n_rows)))
            .filter(|(_, d)| *d >= 2 && *d <= 100)
            .min_by_key(|(_, d)| *d)
            .map(|(n, _)| n)
            .or_else(|| {
                table
                    .columns
                    .iter()
                    .find(|c| c.kind == ColumnKind::Numeric)
                    .map(|c| c.name.clone())
            });
        b.y_var = cat.or_else(|| num.clone());
        b.x_var = num;
        b.summary = Some(BarSummary::Count);
        let shown = b
            .y_var
            .as_deref()
            .map(|name| table.display_name_for(name))
            .unwrap_or_default();
        b.title = format!("Bar Chart: {}", shown);
        b.x_label = shown;
        b.y_label = "Count".to_string();
        b
    }

    /// The chart the choices describe.
    ///
    /// The chart is `None` when the choices do not make one — a
    /// scatter plot needs two numeric variables, a histogram one, and
    /// so on.
    pub fn build(&self, table: &DataTable) -> Option<Chart> {
        let kind = self.chart_type?;
        match kind {
            ChartType::Bar => self.build_bar(table),
            ChartType::Histogram => self.build_histogram(table),
            ChartType::Scatter => self.build_scatter(table),
            ChartType::BoxPlot => self.build_box_plot(table),
        }
    }

    /// The index of a variable by name.
    fn idx(table: &DataTable, name: &Option<String>) -> Option<usize> {
        let name = name.as_ref()?;
        table.column_index(name)
    }

    fn build_bar(&self, table: &DataTable) -> Option<Chart> {
        let y = Self::idx(table, &self.y_var)?;
        let summary = self.summary.unwrap_or(BarSummary::Count);
        match summary {
            BarSummary::Count => {
                // The counts of the categories.
                let bc = crate::stats::bar_chart(table, y)?;
                let labels = bc.categories.clone();
                let counts = bc.counts.clone();
                Some(Chart {
                    kind: ChartKind::Bar,
                    title: self.title.clone(),
                    x_label: self.x_label.clone(),
                    y_label: "Count".to_string(),
                    series: vec![ChartSeries {
                        name: "Count".to_string(),
                        x: (0..labels.len()).map(|i| i as f64).collect(),
                        y: counts,
                        quartiles: None,
                        x_labels: labels,
                    }],
                    reference_line: self.reference_line,
                    reference_label: self.reference_label.clone(),
                })
            }
            _ => {
                // A summary of a numeric variable, by a group.
                let outcome = Self::idx(table, &self.x_var)?;
                let group = Self::idx(table, &self.group_var)?;
                let groups = crate::stats::collect_groups(table, outcome, group)?;
                let mut labels = Vec::new();
                let mut values = Vec::new();
                for (name, vals) in &groups {
                    if vals.is_empty() {
                        continue;
                    }
                    let mut sorted = vals.clone();
                    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                    let v = match summary {
                        BarSummary::Mean => sorted.iter().sum::<f64>() / sorted.len() as f64,
                        BarSummary::Median => crate::stats::percentile_of(&sorted, 50.0),
                        BarSummary::Sum => sorted.iter().sum::<f64>(),
                        BarSummary::Count => sorted.len() as f64,
                    };
                    labels.push(name.clone());
                    values.push(v);
                }
                if labels.is_empty() {
                    return None;
                }
                Some(Chart {
                    kind: ChartKind::Bar,
                    title: self.title.clone(),
                    x_label: self.x_label.clone(),
                    y_label: self.y_label.clone(),
                    series: vec![ChartSeries {
                        name: self.x_var.clone().unwrap_or_default(),
                        x: (0..labels.len()).map(|i| i as f64).collect(),
                        y: values,
                        quartiles: None,
                        x_labels: labels,
                    }],
                    reference_line: self.reference_line,
                    reference_label: self.reference_label.clone(),
                })
            }
        }
    }

    fn build_histogram(&self, table: &DataTable) -> Option<Chart> {
        let y = Self::idx(table, &self.y_var)?;
        let bins = self.bins.clamp(2, 200);
        let h = crate::stats::histogram(table, y, bins)?;
        Some(Chart {
            kind: ChartKind::Histogram,
            title: self.title.clone(),
            x_label: self.x_label.clone(),
            y_label: "Count".to_string(),
            series: vec![ChartSeries {
                name: h.variable.clone(),
                x: h.centres.clone(),
                y: h.counts.clone(),
                quartiles: None,
                x_labels: Vec::new(),
            }],
            reference_line: self.reference_line,
            reference_label: self.reference_label.clone(),
        })
    }

    fn build_scatter(&self, table: &DataTable) -> Option<Chart> {
        let x = Self::idx(table, &self.x_var)?;
        let y = Self::idx(table, &self.y_var)?;
        let s = crate::stats::scatter_points(table, x, y)?;
        Some(Chart {
            kind: ChartKind::Scatter,
            title: self.title.clone(),
            x_label: self.x_label.clone(),
            y_label: self.y_label.clone(),
            series: vec![ChartSeries {
                name: s.y_var.clone(),
                x: s.x.clone(),
                y: s.y.clone(),
                quartiles: None,
                x_labels: Vec::new(),
            }],
            reference_line: self.reference_line,
            reference_label: self.reference_label.clone(),
        })
    }

    fn build_box_plot(&self, table: &DataTable) -> Option<Chart> {
        let outcome = Self::idx(table, &self.y_var)?;
        // A box plot with no group is of the whole variable.
        let groups = match Self::idx(table, &self.group_var) {
            Some(g) => crate::stats::collect_groups(table, outcome, g)?,
            None => {
                let col = table.columns.get(outcome)?;
                let vals: Vec<f64> = col
                    .values
                    .iter()
                    .filter_map(|v| match v {
                        Value::Number(n) if !col.is_missing(0) => Some(*n),
                        _ => None,
                    })
                    .collect();
                vec![(col.name.clone(), vals)]
            }
        };
        let mut series = Vec::new();
        for (name, vals) in &groups {
            if vals.len() < 2 {
                continue;
            }
            let mut sorted = vals.clone();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let q1 = crate::stats::percentile_of(&sorted, 25.0);
            let median = crate::stats::percentile_of(&sorted, 50.0);
            let q3 = crate::stats::percentile_of(&sorted, 75.0);
            series.push(ChartSeries {
                name: name.clone(),
                x: vec![series.len() as f64],
                y: Vec::new(),
                quartiles: Some((sorted[0], q1, median, q3)),
                x_labels: vec![name.clone()],
            });
        }
        if series.is_empty() {
            return None;
        }
        Some(Chart {
            kind: ChartKind::BoxPlot,
            title: self.title.clone(),
            x_label: self.x_label.clone(),
            y_label: self.y_label.clone(),
            series,
            reference_line: self.reference_line,
            reference_label: self.reference_label.clone(),
        })
    }
}

/// The Chart Builder's window.
///
/// The dialog is the same shape as SPSS's: the chart type, the
/// variables, the labels, and the action row. The chart is drawn
/// underneath, so the reader sees what the choices make.
pub fn show(ctx: &egui::Context, app: &mut crate::app::StatsCraftApp, b: &mut ChartBuilder) {
    let mut open = true;
    egui::Window::new("Chart Builder - StatsCraft")
        .open(&mut open)
        .resizable(true)
        .default_width(560.0)
        .default_height(620.0)
        .default_pos(egui::pos2(360.0, 120.0))
        .show(ctx, |ui| {
            render(ui, app, b);
        });
    if !open {
        // The title-bar close button is a cancel: the caller drops the
        // builder instead of putting it back on screen.
        b.confirmed = false;
        b.cancelled = true;
    }
}

/// The body of the Chart Builder, split out so a test can drive it
/// without a window.
pub fn render(ui: &mut egui::Ui, app: &mut crate::app::StatsCraftApp, b: &mut ChartBuilder) {
    use crate::theme;

    let Some(table) = app.table.as_ref() else {
        ui.label(egui::RichText::new("No data loaded.").color(theme::text_muted()));
        return;
    };

    // The variables, by kind.
    let numeric: Vec<String> = table
        .columns
        .iter()
        .filter(|c| c.kind == ColumnKind::Numeric)
        .map(|c| c.name.clone())
        .collect();
    let categorical: Vec<String> = table
        .columns
        .iter()
        .filter(|c| c.kind != ColumnKind::Numeric)
        .map(|c| c.name.clone())
        .collect();

    // ---- The chart type ----
    ui.label(egui::RichText::new("Chart type:").strong());
    ui.horizontal_wrapped(|ui| {
        for t in ChartType::all() {
            let selected = b.chart_type == Some(t);
            if ui
                .selectable_label(selected, egui::RichText::new(t.title()).size(12.0))
                .clicked()
            {
                b.chart_type = Some(t);
                // The defaults follow the type, and use the variable
                // labels when the variables have them.
                b.title = format!("{} Chart", t.title());
                b.x_label = b
                    .y_var
                    .as_deref()
                    .map(|name| table.display_name_for(name))
                    .unwrap_or_default();
                b.y_label = match t {
                    ChartType::Bar => "Count".to_string(),
                    ChartType::Histogram => "Count".to_string(),
                    ChartType::Scatter | ChartType::BoxPlot => b
                        .y_var
                        .as_deref()
                        .map(|name| table.display_name_for(name))
                        .unwrap_or_default(),
                };
            }
        }
    });
    ui.add_space(8.0);

    // ---- The variables ----
    let kind = b.chart_type.unwrap_or(ChartType::Bar);
    ui.label(egui::RichText::new("Variables:").strong());

    if kind == ChartType::Scatter {
        ui.horizontal(|ui| {
            ui.label("X:");
            pick(ui, &mut b.x_var, &numeric, "cb_x", table);
        });
        ui.horizontal(|ui| {
            ui.label("Y:");
            pick(ui, &mut b.y_var, &numeric, "cb_y", table);
        });
    } else {
        ui.horizontal(|ui| {
            ui.label(if kind == ChartType::Bar {
                "Category:"
            } else {
                "Variable:"
            });
            // A bar chart of counts takes any variable; the others
            // take a numeric one.
            let list: &Vec<String> = if kind == ChartType::Bar && b.summary == Some(BarSummary::Count)
            {
                &all_names(table)
            } else {
                &numeric
            };
            pick(ui, &mut b.y_var, list, "cb_y", table);
        });
    }

    // The grouping variable, for a summary or a box plot.
    if kind == ChartType::Bar && b.summary != Some(BarSummary::Count) {
        ui.horizontal(|ui| {
            ui.label("Summarise by:");
            pick(ui, &mut b.group_var, &categorical, "cb_g", table);
            ui.label("Summary:");
            for s in BarSummary::all() {
                let selected = b.summary == Some(s);
                if ui
                    .selectable_label(selected, egui::RichText::new(s.title()).size(11.5))
                    .clicked()
                {
                    b.summary = Some(s);
                }
            }
        });
    }
    if kind == ChartType::BoxPlot {
        ui.horizontal(|ui| {
            ui.label("Group by:");
            pick(ui, &mut b.group_var, &categorical, "cb_g", table);
        });
    }
    if kind == ChartType::Histogram {
        ui.horizontal(|ui| {
            ui.label("Bins:");
            let mut bins = b.bins.clamp(2, 200) as u32;
            if ui.add(egui::Slider::new(&mut bins, 2..=100).text("")).changed() {
                b.bins = bins as usize;
            }
        });
    }
    ui.add_space(8.0);

    // ---- The labels ----
    ui.label(egui::RichText::new("Labels:").strong());
    ui.horizontal(|ui| {
        ui.label("Title:");
        ui.add(egui::TextEdit::singleline(&mut b.title).desired_width(300.0));
    });
    ui.horizontal(|ui| {
        ui.label("X axis:");
        ui.add(egui::TextEdit::singleline(&mut b.x_label).desired_width(300.0));
    });
    ui.horizontal(|ui| {
        ui.label("Y axis:");
        ui.add(egui::TextEdit::singleline(&mut b.y_label).desired_width(300.0));
    });
    ui.add_space(6.0);

    // ---- The reference line ----
    ui.label(egui::RichText::new("Reference line:").strong());
    ui.horizontal(|ui| {
        let mut on = b.reference_line.is_some();
        if ui.checkbox(&mut on, "Show a reference line").changed() {
            b.reference_line = if on { Some(0.0) } else { None };
        }
        if on {
            let mut v = b.reference_line.unwrap_or(0.0);
            if ui
                .add(egui::DragValue::new(&mut v).speed(0.05))
                .changed()
            {
                b.reference_line = Some(v);
            }
            ui.add(egui::TextEdit::singleline(&mut b.reference_label).desired_width(140.0));
        }
    });
    ui.add_space(8.0);

    // ---- The preview ----
    match b.build(table) {
        Some(chart) => {
            ui.label(egui::RichText::new("Preview:").strong());
            crate::app::render_chart_preview(ui, &chart);
        }
        None => {
            ui.label(
                egui::RichText::new("These choices do not make a chart yet.")
                    .color(theme::amber()),
            );
        }
    }
    ui.add_space(8.0);

    // ---- The action row ----
    ui.horizontal(|ui| {
        if ui.button("OK").clicked() {
            b.confirmed = true;
            b.cancelled = false;
        }
        if ui.button("Cancel").clicked() {
            b.confirmed = false;
            b.cancelled = true;
        }
        if ui.button("Reset").clicked() {
            *b = ChartBuilder::for_table(table);
        }
        if ui.button("Help").clicked() {
            app.status = "The Chart Builder makes a chart from the data, the way SPSS's does.".to_string();
        }
    });
}

/// How many distinct values a column has.
fn distinct_values(col: &crate::data::Column, n_rows: usize) -> usize {
    let mut seen: Vec<String> = Vec::new();
    for row in 0..n_rows {
        let key = match col.values.get(row) {
            Some(Value::Number(n)) => format!("{}", n),
            Some(Value::Text(s)) => s.clone(),
            _ => continue,
        };
        if !seen.contains(&key) {
            seen.push(key);
            if seen.len() > 200 {
                break;
            }
        }
    }
    seen.len()
}

/// The names of every variable.
fn all_names(table: &DataTable) -> Vec<String> {
    table.columns.iter().map(|c| c.name.clone()).collect()
}

/// A drop-down of the variable names.
fn pick(
    ui: &mut egui::Ui,
    slot: &mut Option<String>,
    names: &[String],
    id: &str,
    table: &DataTable,
) {
    if names.is_empty() {
        ui.label(egui::RichText::new("(none)").weak());
        return;
    }
    if slot.as_ref().map(|s| !names.contains(s)).unwrap_or(true) {
        *slot = Some(names[0].clone());
    }
    let current = slot
        .as_deref()
        .map(|name| table.display_name_for(name))
        .unwrap_or_default();
    egui::ComboBox::from_id_salt(id)
        .selected_text(current)
        .show_ui(ui, |ui| {
            for name in names {
                let selected = slot.as_deref() == Some(name.as_str());
                let shown = table.display_name_for(name);
                if ui.selectable_label(selected, shown).clicked() {
                    *slot = Some(name.clone());
                }
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{Column, ColumnKind, DataTable, VariableMeta, Value};

    /// A small table: a categorical, a numeric and another numeric.
    fn table() -> DataTable {
        let mut t = DataTable {
            n_rows: 4,
            columns: vec![
                Column {
                    name: "group".into(),
                    kind: ColumnKind::String,
                    values: vec![
                        Value::Text("A".into()),
                        Value::Text("B".into()),
                        Value::Text("A".into()),
                        Value::Text("B".into()),
                    ],
                    meta: VariableMeta::default_for(ColumnKind::String),
                },
                Column {
                    name: "score".into(),
                    kind: ColumnKind::Numeric,
                    values: vec![
                        Value::Number(10.0),
                        Value::Number(20.0),
                        Value::Number(30.0),
                        Value::Number(40.0),
                    ],
                    meta: VariableMeta::default_for(ColumnKind::Numeric),
                },
                Column {
                    name: "age".into(),
                    kind: ColumnKind::Numeric,
                    values: vec![
                        Value::Number(1.0),
                        Value::Number(2.0),
                        Value::Number(3.0),
                        Value::Number(4.0),
                    ],
                    meta: VariableMeta::default_for(ColumnKind::Numeric),
                },
            ],
        };
        t.n_rows = 4;
        t
    }

    #[test]
    fn a_bar_chart_counts_the_categories() {
        let t = table();
        let mut b = ChartBuilder::for_table(&t);
        b.chart_type = Some(ChartType::Bar);
        b.y_var = Some("group".into());
        b.summary = Some(BarSummary::Count);
        let c = b.build(&t).expect("a chart");
        assert_eq!(c.kind, ChartKind::Bar);
        assert_eq!(c.series[0].x_labels, vec!["A", "B"]);
        assert_eq!(c.series[0].y, vec![2.0, 2.0]);
    }

    #[test]
    fn a_bar_chart_can_summarise_by_a_group() {
        let t = table();
        let mut b = ChartBuilder::for_table(&t);
        b.chart_type = Some(ChartType::Bar);
        b.y_var = Some("group".into());
        b.x_var = Some("score".into());
        b.group_var = Some("group".into());
        b.summary = Some(BarSummary::Mean);
        let c = b.build(&t).expect("a chart");
        // The mean of A is (10 + 30) / 2 = 20, and of B is 30.
        assert_eq!(c.series[0].y, vec![20.0, 30.0]);
        // The median of A is 20, of B is 30.
        b.summary = Some(BarSummary::Median);
        let c = b.build(&t).expect("a chart");
        assert_eq!(c.series[0].y, vec![20.0, 30.0]);
        // The sum of A is 40, of B is 60.
        b.summary = Some(BarSummary::Sum);
        let c = b.build(&t).expect("a chart");
        assert_eq!(c.series[0].y, vec![40.0, 60.0]);
    }

    #[test]
    fn a_histogram_bins_the_values() {
        let t = table();
        let mut b = ChartBuilder::for_table(&t);
        b.chart_type = Some(ChartType::Histogram);
        b.y_var = Some("score".into());
        b.bins = 2;
        let c = b.build(&t).expect("a chart");
        assert_eq!(c.kind, ChartKind::Histogram);
        // The four values are in two bins, so the counts add up to the
        // number of cases.
        let total: f64 = c.series[0].y.iter().sum();
        assert_eq!(total, 4.0);
    }

    #[test]
    fn a_scatter_plot_pairs_the_two_variables() {
        let t = table();
        let mut b = ChartBuilder::for_table(&t);
        b.chart_type = Some(ChartType::Scatter);
        b.x_var = Some("age".into());
        b.y_var = Some("score".into());
        let c = b.build(&t).expect("a chart");
        assert_eq!(c.kind, ChartKind::Scatter);
        assert_eq!(c.series[0].x, vec![1.0, 2.0, 3.0, 4.0]);
        assert_eq!(c.series[0].y, vec![10.0, 20.0, 30.0, 40.0]);
    }

    #[test]
    fn a_box_plot_carries_the_quartiles() {
        let t = table();
        let mut b = ChartBuilder::for_table(&t);
        b.chart_type = Some(ChartType::BoxPlot);
        b.y_var = Some("score".into());
        b.group_var = Some("group".into());
        let c = b.build(&t).expect("a chart");
        assert_eq!(c.kind, ChartKind::BoxPlot);
        // One box per group.
        assert_eq!(c.series.len(), 2);
        // The quartiles of A, which is 10 and 30. The upper quartile
        // is 25, because the percentile is interpolated the way SPSS
        // interpolates it.
        let q = c.series[0].quartiles.expect("the quartiles");
        assert_eq!(q.0, 10.0);
        assert_eq!(q.1, 15.0);
        assert_eq!(q.2, 20.0);
        assert_eq!(q.3, 25.0);
    }

    #[test]
    fn a_reference_line_is_carried_on_the_chart() {
        let t = table();
        let mut b = ChartBuilder::for_table(&t);
        b.chart_type = Some(ChartType::Bar);
        b.y_var = Some("group".into());
        b.reference_line = Some(2.5);
        b.reference_label = "Target".into();
        let c = b.build(&t).expect("a chart");
        assert_eq!(c.reference_line, Some(2.5));
        assert_eq!(c.reference_label, "Target");
    }

    #[test]
    fn the_default_category_is_one_that_makes_a_readable_chart() {
        // A table with an identifier and a real category.
        let mut t = table();
        t.columns.insert(
            0,
            Column {
                name: "id".into(),
                kind: ColumnKind::String,
                values: vec![
                    Value::Text("R1".into()),
                    Value::Text("R2".into()),
                    Value::Text("R3".into()),
                    Value::Text("R4".into()),
                ],
                meta: VariableMeta::default_for(ColumnKind::String),
            },
        );
        let b = ChartBuilder::for_table(&t);
        // The identifier has four distinct values, and so does the
        // group; the first one that sorts out is the group, because
        // the identifier is skipped in favour of a categorical with
        // fewer levels.
        assert_eq!(b.y_var, Some("group".into()));
    }

    #[test]
    fn a_scatter_plot_needs_two_numeric_variables() {
        let t = table();
        let mut b = ChartBuilder::for_table(&t);
        b.chart_type = Some(ChartType::Scatter);
        // A categorical variable cannot be one of the two.
        b.x_var = Some("group".into());
        b.y_var = Some("score".into());
        assert!(b.build(&t).is_none());
    }
}
