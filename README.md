# StateCraft

Lightweight SPSS-like statistical workbench written in Rust (native desktop app).

## Install (Windows)
Run `StateCraftSetup.exe` — a single native installer (no archives, no prompts):
installs to `%LOCALAPPDATA%\Programs\StateCraft` (no admin) and creates
Start Menu shortcuts: **StateCraft** and **Uninstall StateCraft**.
If StateCraft is running, the installer closes it and replaces the binary.

## Install (Linux, .deb-first)
```sh
bash packaging/linux/build_deb.sh   # produces statecraft_<version>_<arch>.deb
sudo dpkg -i statecraft_*.deb       # integrates with application menus
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
- Load CSV and SPSS `.sav`
- Descriptives, Frequencies, Correlations, Linear Regression, T-Test
- Charts (scatter, histogram)
- SPSS-like Data Editor (Data View / Variable View tabs, variable filter)
- Variable View with the SPSS column set (Name…Role)
- Sort Cases, Export output (Markdown) and data (CSV), file dialogs
- Syntax runner: LOAD/DESCRIPTIVES/FREQUENCIES/CORRELATIONS/REGRESSION/TTEST/CLEAR

## Reference behavior
UI structure, menus, and analysis flow are cloned from IBM SPSS Statistics
(evidence gathered from the installed copy: menu resources, dialog→command
catalog, engine IPC). Numbers are validated against SPSS sample datasets.

## Roadmap
See `PLAN.md`.
