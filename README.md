# StatsCraft

Lightweight SPSS-like statistical workbench written in Rust (native desktop app).

## Interface

The UI is a modern take on the SPSS workbench, not a pixel copy of it:

- **Typography** — the bundled **Inter** family (Regular / Medium / SemiBold /
  Bold), embedded in the binary so the app looks the same on every machine.
- **Icons** — a 28-glyph **SVG line-icon set** drawn with VectorCraft and
  embedded in the binary. Toolbar buttons, menu entries, tabs and the app
  icon all use it. Regenerate with `python tools/build_icons.py`.
- **Palette** — a single accent blue with a teal "ready" state, defined once
  in `src/theme.rs`.
- **Chrome** — rounded cards, soft shadows, zebra-striped grid, pill-shaped
  document tabs, hairline borders, and a proper primary button for *Run Syntax*.

## Procedure dialogs

Like SPSS, a procedure does not run straight from the Analyze menu. It opens
a dialog:

- a **source variable list**, each row marked with a measure-level icon (a
  ruler for numeric/scale, a pencil for string/nominal);
- a **target list**, with an arrow to move a variable across and back;
- a right-hand column of **sub-dialog buttons** — `Statistics…`, `Charts…`,
  `Format…`, `Cells…`, `Post-hoc…`, `Options…`;
- procedure-specific checkboxes;
- **OK / Paste / Reset / Cancel / Help**, with OK disabled until the dialog
  has what it needs (and a grouping variable for t-test, ANOVA, means and
  crosstabs).

The dialog computes nothing — it **builds SPSS syntax** and hands it to the
same runner the Syntax Editor uses. **OK** runs it; **Paste** puts it in
the Syntax Editor without running it, so you can edit the command by hand
first. That is the whole dialog → syntax → output pipeline, and it is why
the two editors and the menus behave identically.

Sixteen procedures have dialogs: Frequencies, Descriptives, Crosstabs,
Independent-Samples T Test, Paired-Samples T Test, One-Way ANOVA, Bivariate
Correlations, Linear Regression, Means, Explore, and the nonparametric
family — Mann-Whitney U, Kruskal-Wallis H, Wilcoxon signed-rank and
Friedman.

The dialogs' options reach the statistics engine, not just the syntax:
`/STATISTICS=` chooses which descriptives are computed, `/CELLS=` gives
the crosstab its row, column and expected counts, `/POSTHOC=` adds the
ANOVA's Tukey, Scheffe, Bonferroni and LSD comparisons, `/ES=DISPLAY`
adds the t-test's effect sizes (Cohen's d, Hedges' g), and `/PRINT=SIG`
puts the significance stars on the correlation matrix.

To launch straight into a dialog, or to run a syntax file at start-up:

```sh
statscraft data.csv --dialog frequencies
statscraft data.csv --script run.sps
```

`docs/SPSS_REFERENCE.md` records the observed behaviour of IBM SPSS
Statistics 27 that the dialogs are modelled on.

## Install (Windows)
Run `StatsCraftSetup.exe` — a single native installer (no archives, no prompts):
installs to `%LOCALAPPDATA%\Programs\StatsCraft` (no admin) and creates
Start Menu shortcuts: **StatsCraft** and **Uninstall StatsCraft**.
If StatsCraft is running, the installer closes it and replaces the binary.

## Install (Linux, .deb-first)
```sh
bash packaging/linux/build_deb.sh   # produces statscraft_<version>_<arch>.deb
sudo dpkg -i statscraft_*.deb       # integrates with application menus
```
A CI workflow (`.github/workflows/build.yml`) builds the Windows installer
and the Linux `.deb` on every push.

## Run from source
```sh
cargo run
```
Optional: `cargo run -- path\to\data.csv`

## Build (release)
```sh
cargo build --release        # app
cd installer && cargo build --release   # Windows installer (embeds the app exe)
```

## Architecture (locked-in decisions)
- **Stack**: Rust + egui/eframe, one codebase for Windows + Linux, fully
  custom-drawn UI that reproduces the IBM SPSS Statistics 27 look.
- **Pipeline (mirrors SPSS internals)**: UI and dialogs generate **syntax**
  commands → the syntax runner executes them → analyses emit **pivot-table
  models** shown in the Output Viewer. GUI and syntax editing stay consistent.
- **Windows packaging**: prebuilt native installer (`installer/` crate).
- **Linux packaging**: `.deb` first (Ubuntu/Debian), AppImage optional later.
- **Dependencies**: polars is kept (70 MB binary is acceptable) for `.sav`
  loading via polars-readstat-rs.

## Features
- Load CSV and SPSS `.sav` (labels, value labels, measure, alignment,
  decimals, user-missing), and **write** `.sav` back out with the labels,
  the value labels, the measures and the display widths intact
- Procedures: Descriptives, Frequencies, Correlations, Linear Regression,
  Independent-Samples T Test, Paired-Samples T Test, One-Way ANOVA,
  Crosstabs with Pearson Chi-Square, **Reliability (Cronbach's alpha)**,
  **Factor Analysis (principal components)**, **ROC curves**, **Means**,
  **Explore**, and the **nonparametric** family (Mann-Whitney U,
  Kruskal-Wallis H, Wilcoxon signed-rank, Friedman)
- SPSS-like Data Editor (Data View / Variable View tabs, variable filter)
- Variable View with the SPSS column set (Name…Role)
- Multi-window documents: Data Editor (main window), Output Viewer and
  Syntax Editor in separate OS windows, like SPSS
- Output Viewer with pivot tables, outline pane, and Markdown/HTML export
- Sort Cases, export data (CSV), file dialogs
- Syntax runner:
  `LOAD`/`GET`, `CLEAR`, `DESCRIPTIVES`, `FREQUENCIES`, `CORRELATIONS`,
  `REGRESSION y WITH x`, `T-TEST y BY g`, `T-TEST PAIRED v1 v2`,
  `ONEWAY y BY factor`, `CROSSTABS a BY b`, `SORT CASES BY v (A|D)`,
  `MEANS TABLES`, `EXPLORE`, `RELIABILITY`/`ALPHA`, `FACTOR`, `ROC`,
  `COMPUTE`
- Computed variables with `COMPUTE name = expression` — the arithmetic
  operators plus `MEAN`, `MIN`, `MAX`, `SUM`, `ABS`, `SQRT`, `LN`, `LOG10`
  and `ROUND`
- Output Viewer charts: bar, histogram, scatter, box plot, ROC curve

## Validation
Every procedure is covered by reference tests (`cargo test`), checked against
closed-form/textbook values rather than against itself: sample mean/SD (n−1),
unit correlations and exact regression recovery, Pearson χ² on a hand-computed
contingency table, One-Way ANOVA F with known SS/df, Welch's t with unequal
variances, paired t on known differences, and user-missing exclusion through
the analysis pipeline.

The `.sav` writer is covered by a round-trip test: write the table, read it
back, and check the variables, the rows and the values match.

## Reference behavior
UI structure, menus, and analysis flow are cloned from IBM SPSS Statistics
(evidence gathered from the installed copy: menu resources, dialog→command
catalog, engine IPC). Numbers are validated against SPSS sample datasets.

## Roadmap
See `PLAN.md`.
