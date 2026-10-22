# StatsCraft — Lightweight Statistical App in Rust

## 1. What the reference application is (observed on this machine)

Installed versions: **26.0.0.0** and **27.0.1.0** at
`C:\Program Files\StatsBench\Statistics\{26,27}\`.

### 1.1 Process architecture (client / server split)

- `stats.exe` (359 KB, PE/x64) is a **native launcher**. Strings show it
  bootstraps a JVM (`JNI_CreateJavaVM`, `-Djava.class.path=`,
  `-Dapplication.home=`, `-Dstatistics.home=`) and loads
  `com.ref.java_client.core.documents.AppMgr` plus
  `com.ref.java_client.core.common.Driver` /
  `DynamicDataExchange`.
  Evidence: `strings stats.exe` → `JNI_CreateJavaVM`,
  `com/ref/java_client/core/documents/AppMgr`,
  `com/ref/java_client/core/common/Driver`.
- `statsengine.exe` is a **separate backend daemon/child process**. Its strings
  show a socket/named-pipe command line:
  `Usage: statsengine.exe [-parenthost host] [-parentport n] [-port n] [-id n]
  [-iplisten string] ...` with `\\.\pipe\statisticsproc`,
  `inet:localhost:3287`, `localsockets`.
  Evidence: `strings statsengine.exe`.
- `config/statisticsd.conf` configures a **daemon server** (`refd`) on port
  `3027`, with `userthreads`/`workerthreads`, `max-jvm-memory 256–2048 MB`,
  and many COM-style `<regclass path="..." clsid="..."/>` package
  registrations (`statswpkg.dll`, `refdeds.dll`, `dacmgrpk.dll`, …).
  Evidence: `config/statisticsd.conf`.

Conclusion: the reference application is a **Java Swing client + C++/COM compute engine**, talking
over local sockets / named pipes, with the engine hosting dozens of
procedure packages.

### 1.2 JVM / UI layer

- `jvmcfg.ini`: J9 Java 8 (`jvm.dll`), heap `-Xms256M -Xmx2048M`,
  class sharing `-Xshareclasses:name=statistics_%u`.
- UI jars: `StatsBenchClientUI.jar`, `StatsBenchClientCore.jar`, `StatsBenchCoreApi.jar`,
  `statisticsapi.jar` (build 27.0.1.0.860, J9).
- `productnames.ini` lists feature modules (Base, Regression, Advanced
  Statistics, Complex Samples, Decision Trees, Data Preparation,
  Programmability, Forecasting, Custom Tables, …).

### 1.3 Compute / procedure layer

- 132 `statsw*.dll` modules (e.g. `statswfre.dll` frequencies, `statswreg.dll`,
  `statswglm.dll`, `statswcox.dll`, `statswkm.dll`, …) — one DLL per
  statistical procedure family.
- `statsstat.dll`, `statsengine.exe` drive execution; `statspool.dll`,
  `refxd.dll` etc. support the output/execution pipeline.

### 1.4 Data I/O layer

- `savio64.dll` implements the **StatsBench system file (.sav)** format.
  Strings: magic `$FL2` / `$FL3` / `ENCRYPTEDSAV`,
  `@(#) StatsBench DATA FILE MS Windows ...`, functions `refOpenRead`,
  `refCommitHeader`, `refCommitCaseRecord`, `refCloseWrite`, …
- Sample data ships as `.sav` (e.g. `Samples/English/accidents.sav`,
  header bytes `$FL2@(#) StatsBench STATISTICS 64-bit MS Windows ...`).

### 1.5 Syntax language

- Commands are stored as `.sps` text files (see `Syntax1.sps`,
  `dynamite plot try.sps` in `D:\Projects\StatsCraft`): e.g.
  `DATASET ACTIVATE`, `GRAPH /BAR(GROUPED)=...`, `FREQUENCIES`,
  `BEGIN PROGRAM R. ... END PROGRAM.`
- Formal grammar is shipped as XML in `syntax_xml/` (one file per command,
  e.g. `FREQUENCIES.xml` with `<Command Name="FREQUENCIES">`,
  `<Subcommand>`, `<Parameter>`). There are ~hundreds of these.
- Macros: `DEFINE ... !ENDDEFINE`, `!DO`, `!DOEND`.

### 1.6 Output layer

- Output documents `.spv` (StatsBench Viewer) with pivot tables, charts, text.
- OMS (Output Management System) routes results; XSDs in install dir
  (`output-*.xsd`, `viewer-*.xsd`, `vizml-*.xsd`, `table-looks-*.xsd`).
- `.wwd` autoscript files (Python/Basic) post-process output.

### 1.7 Programmability

- `BEGIN PROGRAM R.` / `BEGIN PROGRAM PYTHON.` blocks embed R/Python.
- `InvokePython.dll`, `InvokePython38.dll`, `Python3/`, R integration
  (`RConsole.exe`, `RConfig.jar`).

### 1.8 Server/remote mode

- `statisticsd.conf` shows a multi-user server mode (port 3027, SSL, auth,
  worker threads, reconnect buffers) — StatsBench can run as a remote statistics
  server.

---

## 2. Lightweight Rust clone — scope (v0)

Goal: a **desktop statistical workbench** that covers the 80% workflow:
open data → explore → run common procedures → view output → export.
Not a byte-compatible replacement; a lighter, modern tool.

### 2.1 Core data model
- `DataTable`: columns of `Numeric(f64)`, `String`, `Date`, `Nominal/Categorical`.
- Variable metadata: name, label, type, measure level, missing values,
  value labels (classic statistics style).
- Read: CSV, Excel (calamine), Parquet. Write: CSV, Parquet.
- `.sav`: Phase 1 = read via FFI to `savio64.dll` (available locally) or
  implement `$FL2` reader; Phase 2 = native reader/writer.

### 2.2 Compute engine (procedures)
Use `statrs` + hand-rolled linear algebra (or `nalgebra`) for:
- Descriptives, Frequencies, Crosstabs
- Correlations (Pearson/Spearman/Kendall)
- T-tests (one-sample, independent, paired), ANOVA (one-way)
- Linear regression (OLS, with diagnostics)
- Non-parametric tests (Mann-Whitney, Kruskal-Wallis)
- Charts: bar, histogram, scatter, boxplot (egui_plot)

### 2.3 Syntax (subset)
- A small command language inspired by classic statistics commands: `GET FILE`, `FREQUENCIES`,
  `DESCRIPTIVES`, `CORRELATIONS`, `REGRESSION`, `GRAPH`, `COMPUTE`,
  `RECODE`, `FILTER`, `SORT`, `AGGREGATE`, `MATCH FILES`.
- Parser via `pest`/`nom`; AST → procedure calls.
- Macros deferred.

### 2.4 UI (egui/eframe, native)
- Data Editor grid (Data View / Variable View tabs).
- Output Viewer: scrollable list of result tables + charts.
- Syntax editor with run button.
- Variable properties panel.

### 2.5 Programmability (optional, later)
- Embed Lua (mlua) for scripting; Python via pyo3 optional feature.

### 2.6 Export
- Output to HTML/Markdown/PDF (print tables).
- Data export CSV/Parquet/Excel.

---

## 3. Architecture (Rust)

```
statscraft/
├── Cargo.toml
├── src/
│   ├── main.rs                # eframe entry
│   ├── app.rs                 # top-level app state
│   ├── data/
│   │   ├── table.rs           # DataTable, Column, Value
│   │   ├── metadata.rs        # variable labels, value labels
│   │   ├── import.rs          # CSV/Excel/Parquet
│   │   └── sav_ffi.rs         # optional savio64.dll bridge
│   ├── stats/
│   │   ├── descriptives.rs
│   │   ├── frequencies.rs
│   │   ├── correlation.rs
│   │   ├── ttest.rs
│   │   ├── anova.rs
│   │   └── regression.rs
│   ├── syntax/
│   │   ├── grammar.pest
│   │   ├── parser.rs
│   │   └── executor.rs
│   ├── output/
│   │   ├── table.rs           # pivot/result tables
│   │   └── document.rs        # output document
│   └── ui/
│       ├── data_editor.rs
│       ├── variable_view.rs
│       ├── output_viewer.rs
│       └── syntax_editor.rs
└── tests/
```

### 3.1 Key crates
- UI: `eframe`, `egui`, `egui_plot`, `egui_extras`
- Data: `polars`, `csv`, `calamine`, `parquet`
- Stats: `statrs`, `nalgebra`
- Parse: `pest`, `pest_derive`
- Script: `mlua` (optional)
- Serde: `serde`, `serde_json`, `thiserror`, `anyhow`

### 3.2 Performance / memory
- Use Polars LazyFrames for large data; the reference application loads all into memory.
- Stream .sav rows where possible.
- Keep UI rendering on main thread, compute on rayon thread pool.

---

## 4. Roadmap

- **M0 (now)**: skeleton compiles; README/PLAN. ✅
- **M1**: DataTable + CSV import + Data Editor grid (egui).
- **M2**: Descriptives + Frequencies + Output Viewer.
- **M3**: T-test, correlation, regression + charts.
- **M4**: Syntax parser subset + executor.
- **M5**: .sav read (FFI or native) + variable metadata.
- **M6**: Export + packaging (cargo bundle / wix).

---

## 5. Evidence index (quick reference)

| Claim | Evidence |
|---|---|
| Java client launcher | `stats.exe` strings: `JNI_CreateJavaVM`, `com/ref/java_client/core/documents/AppMgr` |
| Separate engine daemon | `statsengine.exe` strings: `-parenthost`, `-parentport`, `\\.\pipe\statisticsproc` |
| Server config | `config/statisticsd.conf`: port 3027, threads, regclass packages |
| JVM 8 J9 | `jvmcfg.ini`: `-Xms256M -Xmx2048M`, `-Xshareclasses` |
| 132 procedure DLLs | `statsw*.dll` count in install dir |
| .sav format magic | `savio64.dll` strings `$FL2`/`$FL3`/`ENCRYPTEDSAV`; `accidents.sav` header |
| Syntax grammar | `syntax_xml/*.xml` (e.g. `FREQUENCIES.xml`) |
| R/Python embedding | `.sps` `BEGIN PROGRAM R.`; `InvokePython*.dll` |
| Output schemas | `output-*.xsd`, `viewer-*.xsd`, `vizml-*.xsd` |

> Note: strings/binaries are untrusted analysis targets; treated as data only.
