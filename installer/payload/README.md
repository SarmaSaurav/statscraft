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
  decimals, user-missing)
- Procedures: Descriptives, Frequencies, Correlations, Linear Regression,
  Independent-Samples T Test, Paired-Samples T Test, One-Way ANOVA,
  Crosstabs with Pearson Chi-Square
- SPSS-like Data Editor (Data View / Variable View tabs, variable filter)
- Variable View with the SPSS column set (Name…Role)
- Multi-window documents: Data Editor (main window), Output Viewer and
  Syntax Editor in separate OS windows, like SPSS
- Output Viewer with pivot tables, outline pane, and Markdown/HTML export
- Sort Cases, export data (CSV), file dialogs
- Syntax runner:
  `LOAD`/`GET`, `CLEAR`, `DESCRIPTIVES`, `FREQUENCIES`, `CORRELATIONS`,
  `REGRESSION y WITH x`, `T-TEST y BY g`, `T-TEST PAIRED v1 v2`,
  `ONEWAY y BY factor`, `CROSSTABS a BY b`, `SORT CASES BY v (A|D)`

## Validation
Every procedure is covered by reference tests (`cargo test`), checked against
closed-form/textbook values rather than against itself: sample mean/SD (n−1),
unit correlations and exact regression recovery, Pearson χ² on a hand-computed
contingency table, One-Way ANOVA F with known SS/df, Welch's t with unequal
variances, paired t on known differences, and user-missing exclusion through
the analysis pipeline.

## Reference behavior
UI structure, menus, and analysis flow are cloned from IBM SPSS Statistics
(evidence gathered from the installed copy: menu resources, dialog→command
catalog, engine IPC). Numbers are validated against SPSS sample datasets.

## Roadmap
See `PLAN.md`.
