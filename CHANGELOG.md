# Changelog

All the notable changes to StatsCraft.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the project uses [semantic versioning](https://semver.org/spec/v2.0.0.html).

## 0.1.0 — 2026-10-22

The first release. Nineteen procedures, a pivot-table output model, a
data editor that edits, and a chart builder.

### Added

**The workbench**
- A **Data Editor** with the full variable column set, a zebra-striped grid, and
  the Data View and Variable View tabs.
- The **Data Editor's editing**: cases can be added and deleted, cells
  and blocks of cells copied and pasted, and a variable's **missing
  values** declared as discrete values or as a range, which every
  procedure then leaves out.
- A **Syntax Editor** and a syntax runner, so the menus, the dialogs
  and the typed commands all go through one path.
- The **Output Viewer**, with an outline tree and a **pivot-table**
  output model: the cells are addressed by a row key and a column key
  rather than by a row and a column number, the totals are levels of
  the dimensions, and the counts, the expected counts and the
  percentages are *measures* of the same cells. A **Pivot** button
  exchanges the row and the column dimensions, and the HTML export
  carries the spanning header and the measures rather than a
  flattened grid.
- The **Chart Builder**: the type of the chart, the variables, how the
  data is summarised, the title and the axis labels, and a reference
  line, with a live preview of the result.
- A light theme and a **Dracula-inspired dark theme**, switched from
  the View menu.
- A 37-glyph **icon set** and an **application icon**, in the
  executable, the installer, the shortcuts and the window title bar.

**The procedures**

Frequencies, Descriptives, Crosstabs, one-sample and independent and
paired t-tests, One-Way ANOVA, the **General Linear Model** (a
factorial ANOVA with any number of factors, a covariate, and the three
types of sums of squares, checked against statsmodels), Correlations,
Linear Regression, Mean Tables, Explore, Reliability (Cronbach's
alpha), Factor Analysis, ROC curves, and the nonparametric family
(Mann-Whitney, Wilcoxon, Kruskal-Wallis, Friedman).

The options reach the engine: `/CELLS`, `/STATISTICS`, `/POSTHOC`, the
effect sizes, the significance stars and the missing-value mode all
change the output.

**The data**

- Reads and **writes** `.sav` system files — labels, value labels,
  measures and display widths intact.
- CSV, and the Variable View's full metadata.

### Notes

- The statistics are checked against closed-form reference values and
  published tables, not against themselves. The GLM is checked against
  statsmodels.
- The interface and command grammar were developed from a reference
  desktop statistics environment; `docs/REFERENCE.md` records what was
  observed and where the app deliberately differs.
