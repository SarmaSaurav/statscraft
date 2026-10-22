# StatsBench 27 — Feature Analysis, Mechanics, and Evidence

> Internal reference note. Third-party product names have been replaced with
> the neutral label **StatsBench**.

**Product analysed:** StatsBench 27 (build 27.0.1.0), installed at `C:\Program Files\StatsBench\Statistics\27`
**Analysis date:** 22 October 2026
**Method:** direct observation of the running application, installation-tree inspection, command-descriptor XML, output schemas, sample files, and extension bundles
**Deliverable location:** `D:\Projects\StatsCraft\REFERENCE_FEATURES.md`

---

## 0. How to read this document

Every feature entry answers four questions:

| Question | Meaning |
|---|---|
| **What** | The user-visible behaviour and purpose of the feature |
| **How** | The mechanism: the command syntax it generates, the subcommands/parameters, the module or extension that executes it, and the output objects it produces |
| **Why** | The design reason: why the feature exists, what problem it solves, what assumption or workflow it protects |
| **Evidence** | References of the form `[E1]` to `[E25]`, resolved in Appendix A with exact file paths, menu captures, and screenshots |

Claims are limited to what is visible in the shipped artefacts or observable in the running product. Where the mechanism is an inference from naming or structure, it is marked **(inferred)**. No source code was available.

The application was launched and used live. The full pipeline **dialog → generated syntax → execution → pivot-table output** was exercised with `Samples\English\accidents.sav`, and is documented in §5 and §6 with screenshots.

---

## 1. Executive summary

StatsBench 27 is a **metadata-driven statistical workbench** built as a Java Swing client over a separate native compute engine, with per-procedure native modules and an R/Python extension layer.

Six design decisions explain almost every feature in the product:

1. **The dataset is a dictionary, not just a grid.** Every variable carries a name, type, width, decimals, label, value labels, missing-value rules, display width, alignment, measurement level, and modelling role. Dialogs, charts, and procedures read that metadata and change their behaviour accordingly.
2. **Dialogs are syntax generators.** A dialog assembles a command, and its **Paste** button writes the exact command into a Syntax Editor. The Output Viewer's log shows the commands that actually ran. Nothing is hidden in a proprietary "dialog state".
3. **Analysis is command-oriented.** The install ships 328 command descriptors (`syntax_xml\*.xml`). Every menu procedure maps to one of these commands and to a native module (`statsw*.dll`, 132 modules; most are procedures, a few are format handlers) or an R/Python extension.
4. **Output is a structured document, not a screenshot.** Results are a tree of headings, notes, pivot tables, charts, trees, and models, serialised as `.spv` XML conforming to the shipped `output-1.8.xsd`. Pivot tables stay editable after the analysis.
5. **The front end is separate from the engine.** `stats.exe` starts a JVM/Swing client; `statsengine.exe` performs the computation. The same engine can be reached through a server daemon (`statisticsd.conf`, port 3027), which is why StatsBench can run locally or in client/server mode.
6. **New statistics ship as extensions, not core rebuilds.** Many StatsBench 27 additions (Distribution Fit, Relative Importance, Ridge Regression, Canonical Correlation, Cluster Silhouettes, Nonparametric Regression, Propensity Score Matching, TURF, Column Proportion Test, Weibull Plot) are R-based bundles under `ProgramData\StatsBench\Statistics\27\extensions`, each with a dialog spec (`.spd`), syntax spec (`.xml`/`.spe`), and R source (`.R`).

The rest of this document expands those points feature by feature.

---

## 2. Environment and evidence base

### 2.1 Product facts

| Property | Observed value | Evidence |
|---|---|---|
| Product name | StatsBench 27 | [E1] |
| Version | 27.0.1.0 (VersionMajor 27, VersionMinor 0, FixPack 0, VersionPatch 1) | [E1] |
| Install root | `C:\Program Files\StatsBench\Statistics\27` | [E1], [E5] |
| Licence feature code | 1200 (Base); module codes listed in `productnames.ini` | [E1], [E2] |
| Character encoding | CodePage 65001 (UTF-8); status bar reports `Unicode: ON` | [E1], [E13] |
| Java runtime | J9 (`JRE\bin\j9vm\jvm.dll`), `-Xms256M -Xmx2048M`, compressed references, TLS 1.2 default | [E3] |
| Compute binaries | `stats.exe` (client launcher, 359 KB) and `statsengine.exe` (engine, 86 KB) | [E5] |
| Server daemon config | port 3027, `win32` authentication, 4 user / 4 worker threads, SSL mode 0 (disabled; 2/3/4 enable TLS 1.0/1.1/1.2), client data access `FULL` | [E4] |
| Native `statsw*` modules | 132 files | [E5] |
| Java components | 142 JARs, including `StatsBenchClientUI.jar` (16.5 MB), `StatsBenchClientUIRes.jar` (15.0 MB), `StatsBenchSyntaxEditor.jar` | [E5] |
| Command descriptors | 328 XML files in `syntax_xml\` | [E6] |
| Help topics | 8,992 IDs in `helpids.csv` | [E9] |
| Chart templates | 50 `.viztemplate` files | [E8] |
| Sample assets | 1,328 `.sav`, 209 `.sps`, 22 `.spv`, 55 `.csplan`, 33 `.csaplan` | [E7] |
| Bundled Python | Python 3.8.2 (`Python3\python.exe`) | [E12] |
| R integration | present; R-based extension bundles and `R 3.6 Configuration` bundle | [E12] |
| Output schemas | `output-1.4` … `1.8.xsd`, `dictionary-1.0/1.1.xsd`, `oms-log-1.1/1.2.xsd`, PMML 2.1–4.3 | [E10] |

### 2.2 What "evidence" means here

The analysis used five independent kinds of evidence, so that no single source has to be trusted alone:

1. **Live UI captures** — top-level menus, submenus, dialogs, the Data Editor, the Syntax Editor, and the Output Viewer (screenshots listed in Appendix D).
2. **Shipped declarative metadata** — the 328 command XMLs, each listing subcommands, parameters, value types, and enumerations.
3. **Binary/module inventory** — names and sizes of the client, engine, I/O libraries, procedure DLLs, and JARs.
4. **Schemas and samples** — output XSDs, `.sav`/`.sps`/`.spv` samples, and the command syntax reference PDF.
5. **A real end-to-end run** — `accidents.sav` → Frequencies dialog → Paste → Run → Viewer, which proves the dialog/syntax/output relationship directly.

### 2.3 Scope

The document covers the shipped Base client plus the modules and extension bundles visible in this installation: Data Editor, syntax, output, all eleven top-level menus, the procedures reachable from Analyze, Graphs, Data, Transform, Utilities, and Extensions, file I/O, automation, and the extension architecture.

It does not claim to document every option of all 328 commands at parameter level; it documents the feature surface and the mechanics, with representative subcommand lists and the exact locations where the full definitions live.

---

## 3. Architecture: how StatsBench is put together

### 3.1 Layer model

```
+--------------------------------------------------------------+
|  Java Swing client                                           |
|  stats.exe -> JVM (J9) -> StatsBenchClientUI.jar / ClientCore.jar  |
|  Data Editor | Syntax Editor | Output Viewer | Dialogs       |
+-----------------------------+--------------------------------+
                              | commands, data, output objects
+-----------------------------v--------------------------------+
|  Compute engine  (statsengine.exe)                            |
|  statsstat.dll  (core statistics, 5.0 MB)                     |
|  132 statsw*.dll modules (procedures + format handlers)       |
|  savio64.dll + savodbc/savxls/refxd/... (data access)    |
|  statswout.dll, statswoms.dll  (output and OMS)                |
+-----------------------------+--------------------------------+
                              |
+-----------------------------v--------------------------------+
|  Persistent artefacts                                        |
|  .sav data | .spv output | .sps syntax | .spj production     |
|  .csplan/.csaplan samples | .xml/.spd/.spe/.R extensions    |
+--------------------------------------------------------------+
```

### 3.2 The client

`stats.exe` is a small native launcher (359 KB). It starts the bundled J9 JVM (`jvmcfg.ini`) and the Java client. The client is by far the largest part of the product: `StatsBenchClientUI.jar` is 16.5 MB and `StatsBenchClientUIRes.jar` is 15.0 MB, with additional JARs for the syntax editor, chart editor, pivot-table editor, model viewer, and custom-dialog builder. The measured behaviour matches a Swing application: Java-style menus, a Java Look-and-Feel, keyboard mnemonics, modal dialogs over MDI child windows, and a separate window per Data Editor, Syntax Editor, and Viewer.

The client is responsible for **presentation and command construction**, not for the statistics. It knows which variables exist and what their metadata is, which is why dialogs can grey out invalid choices and why the Frequencies dialog can show `Age category [agecat]` rather than only `agecat`.

**Why:** the split lets the original developer ship a rich cross-platform UI once, keep the numerical engine native and fast, and offer the same engine in local, server, and batch modes.

### 3.3 The engine and the procedure modules

`statsengine.exe` (86 KB) is the separate compute executable in the install; the client is designed to drive it as the engine, as the product's client/server layout and daemon configuration indicate **(inferred from the binary layout; the exact IPC is not exposed)**. The heavy numerical code is in `statsstat.dll` (5.0 MB) and in **132 `statsw*.dll` modules**. The naming is systematic and matches commands:

| Module | Command family (inferred from name) |
|---|---|
| `statswfre.dll` | FREQUENCIES |
| `statswreg.dll` | REGRESSION |
| `statswglm.dll` | GLM / UNIANOVA |
| `statswmix.dll` | MIXED |
| `statswgen.dll` | GENLIN |
| `statswfac.dll` | FACTOR |
| `statswrel.dll` | RELIABILITY |
| `statswclu.dll` | CLUSTER / QUICK CLUSTER |
| `statswcox.dll` | COXREG |
| `statswkm.dll` | KM (Kaplan-Meier) |
| `statswmva.dll` | MVA (Missing Value Analysis) |
| `statswmim.dll` | MULTIPLE IMPUTATION |
| `statswneu.dll` | Neural Networks (MLP/RBF) |
| `statswggr.dll` | GGRAPH (Chart Builder graphics) |
| `statswstm.dll` | Spatial and Temporal Modeling |
| `statswcs*.dll` | Complex Samples family |

Modules are loaded on demand, which is why the install can carry 132 of them without a 132-module startup cost. The engine also enforces the licence: `productnames.ini` maps feature codes (Base, Tables, Regression Models, Advanced Models, Exact Tests, Categories, Missing Values, Custom Tables, Complex Samples, Classification Trees, Data Validation, Programmability, Advanced Visualization, Trends) to modules. **(inferred)** from the feature-code table and the per-module layout.

### 3.4 Data access

Data I/O is a separate layer with format-specific libraries:

| Library | Role |
|---|---|
| `savio64.dll` (1.3 MB) | Native StatsBench `.sav`/`.zsav` reader/writer |
| `savodbc.dll` | ODBC database access |
| `savxls.dll`, `refexcel.jar` | Excel workbooks |
| `refxd.dll`, `refxdlm.dll` | Delimited text and data access |
| `statswsas.dll` | SAS transport files (`GET SAS`) |
| `savjson.dll` | JSON |
| `refdeds.dll`, `refdemg.dll`, `refdnet.dll` | Data-editor and data-management support |
| `StatsbenchClientProxy.dll` | Client-side proxy into the engine |

The command catalog confirms the formats: `GET SAS`, `GET STATA`, `GET TRANSLATE`, `SAVE TRANSLATE`, `IMPORT`, `EXPORT`, `SAVE CODEPAGE`, `MATRIX DATA`, `OMS` destinations, and the `OUTPUT EXPORT` subcommands `DOC, HTML, PDF, PPT, TEXT, XLS, XLSX, XLSM, BMP, EMF, EPS, JPG, PNG, TIF` [E6].

### 3.5 Output and OMS

`statswout.dll` (678 KB) and `statswoms.dll` (203 KB) handle output and the Output Management System. The output model is defined publicly by `output-1.8.xsd`, whose root `outputTree` contains `command`, `pageTitle`, `textBlock`, `heading`, `pivotTable`, `chartTitle`/`chart`, `treeTitle`/`treeView`, and `modelTitle`/`model` elements [E10]. That is exactly what the Viewer displays in its outline (Log, Title, Notes, Active Dataset, Statistics, Age category), which confirms that the Viewer is rendering the same structured document that `.spv` stores [E17].

### 3.6 Server mode

`statisticsd.conf` is a full daemon configuration: listen port 3027, SSL mode setting (0 disables SSL; 2/3/4 select TLS 1.0/1.1/1.2), 4 user and 4 worker threads, `win32` authentication, `client-data-access FULL|NONE`, reconnect timeout, COP registration on port 80, and GSK keystore settings [E4]. The File menu's **Switch Server...** and **Repository** entries expose this mode in the client. **Why:** the same statistical engine can serve many users, keep data on a server, and run long jobs independently of a desktop session.

### 3.7 Why this architecture matters for feature behaviour

- **Commands are the contract.** A menu action, a pasted syntax line, a production job, and an extension all reduce to the same command language.
- **Metadata flows end to end.** Measurement level and value labels set in Variable View change dialog defaults, chart suggestions, and procedure output.
- **Output survives the session.** Because results are structured XML, they can be reopened, edited, exported, and processed by OMS long after the engine has finished.
- **Extensions do not fork the product.** New procedures are new commands with declarative specs, so the same dialog framework, syntax editor, and Viewer can host them.

---

## 4. The Data Editor and the data model

### 4.1 Two views of one dataset

The Data Editor is the default window (`Untitled1 [DataSet0] - StatsBench Data Editor`, later `accidents.sav [DataSet1]`). It has two tabs:

- **Data View** — a case-by-variable grid. Rows are cases (respondents, records); columns are variables. Cells show either raw values or value labels depending on **View > Value Labels**.
- **Variable View** — one row per variable and one column per metadata property. This is the dictionary that drives every dialog.

Switching is done with the tabs or **View > Data** (`Ctrl+T`). **Why:** analysts need both a spreadsheet-like view of the data and a schema-like view of the metadata; separating them avoids overloading the grid with 11 extra columns per variable.

Evidence: [E13] (Variable View), [E14] (Data View with value labels), [E22] (View menu).

### 4.2 Variable View columns (observed with `accidents.sav`)

| Column | Meaning | Observed example |
|---|---|---|
| **Name** | Short identifier used in syntax; unique, no spaces | `agecat` |
| **Type** | Storage type: Numeric, Comma, Dot, Scientific, Date, Dollar, Custom currency, String | `Numeric` |
| **Width** | Display width (and for strings, maximum length) | `4` for `agecat`, `8` for `accid` |
| **Decimals** | Number of decimal places displayed | `0` |
| **Label** | Human-readable description used in dialogs and output | `Age category` |
| **Values** | Value labels for coded categories | `{1, Under 21...}` |
| **Missing** | Missing-value rules: none, discrete values, range plus optional discrete | `None` |
| **Columns** | Column width in the Data View | `8` |
| **Align** | Cell alignment: Left, Right, Center | `Right` |
| **Measure** | Measurement level: Scale, Ordinal, Nominal (with icons) | `Ordinal` |
| **Role** | Intended modelling use (e.g. Input, Target) | `Input` |

Observed rows: `agecat` (Ordinal), `gender` (Nominal), `accid` (Scale, "Accidents"), `pop` (Scale, "Population at risk"). The Measure column uses a ruler icon for scale, a bar-chart icon for ordinal, and three circles for nominal [E13].

**Why:** these are not cosmetic. Measurement level determines which procedures and chart types are offered, how categories are treated, and which summary statistics are sensible. Value labels determine what appears in output tables. Missing rules determine case exclusion. Role tells modelling procedures which variable is the outcome and which are predictors.

### 4.3 Measurement level

- **Scale** — continuous/interval/ratio. Means, standard deviations, Pearson correlations, linear models.
- **Ordinal** — ordered categories. Medians, rank tests, ordinal regression, bar charts.
- **Nominal** — unordered categories. Frequencies, chi-square, logistic models, pie/bar charts.

The level can be set manually in Variable View, inferred on import, or fixed later with **Data > Set Measurement Level for Unknown**. The Chart Builder explicitly warns when levels are not set properly and offers **Define Variable Properties** before charting [E19]. That warning is direct evidence that chart construction is level-aware, not a generic plotting tool.

### 4.4 Value labels and labelled data

`accidents.sav` stores `agecat` as 1/2/3 with labels `Under 21`, `21-25`, `26-30`, and `gender` as 0/1 with `Male`/`Female`. In Data View the labels are shown (`Under 21`, `Female`); in output the frequency table row labels are the value labels, not the codes [E14], [E17].

Value labels can be entered in Variable View, defined in bulk through **Define Variable Properties**, derived from string values with **Data > Create Value Labels from Data** (extension `STATS_VALLBLS_FROMDATA`), or attached with the `VALUE LABELS` command. **Why:** analysis output should be readable without a codebook, while storage stays compact and numeric.

### 4.5 Missing values

Variable View's **Missing** column supports:

- **No missing values** (the default),
- **Discrete missing values** (up to three specific codes, e.g. 9, 99),
- **Range** (a low–high range) plus an optional discrete value.

Missing values are declared, not guessed. Procedures exclude them listwise or pairwise depending on their options; the Frequencies output separates `N Valid` from `Missing` (observed: Valid 6, Missing 0) [E17]. **Why:** StatsBench distinguishes "no answer" codes from real data, so a code such as 99 does not silently inflate means.

### 4.6 Roles and custom attributes

**Role** labels a variable for modelling: Input, Target, Both, None, Partition, Split. This is not used by classical procedures; it is consumed by modelling procedures and by data preparation. The `ADP` command descriptor makes the connection explicit: its `FIELDS` subcommand takes `TARGET`, `INPUT`, and `ANALYSISWEIGHT` parameters [E6]. In other words, the Role column is the UI for the target/input contract used by Automatic Data Preparation and Automatic Linear Modeling.

**Data > New Custom Attribute** adds arbitrary name/value metadata to variables, stored in the dictionary and available to syntax (`VARIABLE ATTRIBUTE`). **Why:** organisations attach study-specific metadata (question wording, scale source, wave) without abusing variable labels.

### 4.7 Multiple datasets and pending transformations

A session can hold several datasets (`DataSet0`, `DataSet1`, ...). The Frequencies **Paste** produced:

```
DATASET ACTIVATE DataSet1.
FREQUENCIES VARIABLES=agecat
  /ORDER=ANALYSIS.
```

This shows that the client tracks an active dataset and that dialogs explicitly activate it before running a command [E16]. The File menu's **Rename Dataset**, **Copy Dataset**, **Save All Data**, and the `DATASET` command family (`DECLARE`, `NAME`, `COPY`, `ACTIVATE`, `CLOSE`, `DISPLAY`) implement this model [E6], [E22].

Transformations can remain **pending** until the data are read or a procedure forces execution; **Transform > Run Pending Transforms** (`EXECUTE`) flushes them. **Why:** deferring work avoids recomputing after every cell edit and lets the engine optimise a batch of transformations.

### 4.8 Status, Unicode, and auto-recovery

The status bar reports `StatsBench Processor is ready` and `Unicode: ON`. The Unicode flag reflects the UTF-8 code page (65001) and affects how text and labels are stored and displayed [E1], [E13].

The product maintains **restore points**. On startup in this environment it presented a **Session Management** dialog: "Please select a restore point to open", with a table of restore points by type (Data Editor, Output, Syntax), file name, and date, plus **Restore**, **Cancel**, and **Delete** [E20]. The File menu exposes **Open Restore Point...**, **Save Restore Point...**, and a checked **File Enabled for Auto-Recovery** item [E22]. **Why:** statistical sessions can represent hours of work; auto-recovery is a first-class data-protection feature, not an afterthought.

A separate **Error Reporting Service** dialog appeared at startup ("StatsBench encountered an unexpected error"), offering **More Information**, **Send**, and **Cancel** [E21]. Cancelling it allowed the application to continue normally. This is the crash-reporting/telemetry path, and it is independent of the session-recovery path.

---

## 5. Dialog mechanics: the front door to the command language

### 5.1 The standard dialog anatomy

Two dialogs were captured in detail: **Frequencies** [E15] and **Linear Regression** [E18]. They share a consistent structure:

| Element | Frequencies | Linear Regression | Purpose |
|---|---|---|---|
| Source list | All four variables, with labels and `[names]` | Same | Variables available in the active dataset |
| Target list(s) | `Variable(s)` | `Dependent`, `Independent(s)`, `Selection Variable`, `Case Labels`, `WLS Weight` | Slots that define the command |
| Move control | Single arrow | Per-slot arrows | Moves selected variables between lists |
| Method/options | `Statistics`, `Charts`, `Format`, `Style`, `Bootstrap` | `Statistics`, `Plots`, `Save`, `Options`, `Style`, `Bootstrap` | Sub-dialogs that add subcommands |
| Switches | `Display frequency tables`, `Create APA style tables` | `Method: Enter`, `Block 1 of 1`, `Previous`/`Next` | Behavioural choices |
| Commit row | `OK`, `Paste`, `Reset`, `Cancel`, `Help` | Same | Execute, generate syntax, reset, dismiss |

Three behaviours are important:

1. **`OK` and `Paste` are disabled until the dialog is valid.** In the initial Frequencies dialog both were grey; after moving `agecat` into `Variable(s)`, both became active [E15].
2. **The source list shows labels and names.** `Age category [agecat]` is one row; the icon in front encodes the measurement level. This is the metadata model surfacing in the UI.
3. **Sub-dialogs are additive.** `Statistics...`, `Charts...`, `Format...`, `Style...`, and `Bootstrap...` do not run anything; they attach subcommands such as `/STATISTICS=MEAN MEDIAN` or `/BARCHART` to the pending command.

### 5.2 What the buttons actually do

- **OK** builds the command and submits it to the engine, then writes the result objects into the designated Output Viewer.
- **Paste** builds the same command but writes it into a new Syntax Editor instead of running it. This is the single most important affordance for reproducibility.
- **Reset** clears the dialog back to its initial state.
- **Cancel** closes without building anything.
- **Help** opens the context help topic (one of the 8,992 help IDs) [E9].

### 5.3 The observed dialog-to-syntax pipeline

With `accidents.sav` loaded and `agecat` selected, pressing **Paste** produced this Syntax Editor content [E16]:

```
DATASET ACTIVATE DataSet1.
FREQUENCIES VARIABLES=agecat
  /ORDER=ANALYSIS.
```

Notes:

- The dialog automatically prefixed `DATASET ACTIVATE DataSet1.` so the command runs against the intended dataset even in a multi-dataset session.
- `VARIABLES=agecat` is the main specification; the dialog defaulted the ordering subcommand to `/ORDER=ANALYSIS` (sort categories by value order rather than by frequency).
- The syntax is free-format: the command ends with a period, subcommands begin with `/`, and keywords are case-insensitive.

Running the pasted syntax (`Ctrl+A`, `Ctrl+R`) produced the Viewer document described in §7. The Viewer's **Log** recorded the *actual* commands, including the ones the GUI had issued earlier:

```
GET  FILE='C:\Program Files\StatsBench\Statistics\27\Samples\English\accidents.sav'.
DATASET NAME DataSet1 WINDOW=FRONT.
DATASET ACTIVATE DataSet1.
FREQUENCIES VARIABLES=agecat
  /ORDER=ANALYSIS.
```

This is direct proof that the GUI, the syntax editor, and the engine share one command language: opening a file, naming the dataset, activating it, and running the analysis are all commands in the log [E17].

### 5.4 Dialog families and what they generate

The dialog set is organised by the same command families as the menu:

| Family | Representative dialogs | Command generated |
|---|---|---|
| Data definition | Define Variable Properties, Define Dates, Multiple Response Sets | `VARIABLE LEVEL`, `DATE`, `MRSETS` |
| Data manipulation | Sort Cases, Merge Files, Aggregate, Restructure, Select Cases | `SORT CASES`, `ADD FILES`/`MATCH FILES`, `AGGREGATE`, `VARSTOCASES`/`CASESTOVARS`, `SELECT IF`/`FILTER` |
| Transformation | Compute, Recode, Count, Rank, Time Series | `COMPUTE`, `RECODE`, `COUNT`, `RANK`, `CREATE` |
| Analysis | Frequencies, Descriptives, Crosstabs, Regression, GLM | `FREQUENCIES`, `DESCRIPTIVES`, `CROSSTABS`, `REGRESSION`, `GLM`/`UNIANOVA` |
| Graphics | Chart Builder, Legacy Dialogs | `GGRAPH`, `GRAPH` |
| Output | OMS Control Panel, Export | `OMS`, `OUTPUT EXPORT` |

**Why:** a single dialog framework (Java classes, resource bundles, variable-list widgets, validation) can host hundreds of procedures because each dialog only has to describe its slots and options; the command layer does the rest.

### 5.5 Dialog validation and metadata awareness

Dialogs are not generic forms. They read the dictionary:

- Variable lists show labels, names, and measurement icons.
- Procedures restrict slots by type. For example, a scale-only slot will not accept a string variable, and a factor slot expects categorical variables.
- Chart Builder checks measurement level and value labels up front (see §11.2).
- The Linear Regression dialog exposes **Block 1 of 1** with **Previous/Next**, so predictors can be entered in blocks and the method chosen per block [E18].

**Why:** most statistical errors are data-model errors (using a string ID as a covariate, treating a nominal code as continuous). Enforcing the model in the dialog prevents a class of mistakes before the engine sees the command.

---

## 6. The Syntax Editor

### 6.1 Window anatomy

The pasted command opened a window titled `*Syntax1 - StatsBench Syntax Editor` [E16]. Its parts:

- **Menu bar** specific to syntax: `File, Edit, View, Data, Transform, Analyze, Graphs, Utilities, Run, Tools, Extensions, Window, Help`. The presence of **Run** and **Tools** distinguishes it from the Data Editor.
- **Navigation pane** on the left, listing the commands in the file (`DATASET ACTIVATE`, `FREQUENCIES`). This is an outline for long syntax files.
- **Line-number gutter** on the left of the editing area.
- **Editing pane** with syntax colouring: command keywords in blue, values in green, punctuation in black.
- **Toolbar** with Run (green triangle), Run Selection, and a dropdown showing the **Active DataSet** (`DataSet1`).

### 6.2 Running syntax

The **Run** menu and the toolbar support running the whole file, the current command, a selection, or up to the end. The observed run used `Ctrl+A` then `Ctrl+R`. **Why:** analysts iterate: run a data step, inspect, adjust, run the model. Command-level execution makes that loop fast and avoids rerunning everything.

### 6.3 Syntax features

- **Free-format commands**, terminated by `.`; subcommands prefixed by `/`.
- **Macros and includes**: the sample `automaton.sps` uses `@location`, `@datafile`, and `@varlist` placeholders, showing the macro style used by the shipped samples [E23]. The command catalog includes `DEFINE`, `INCLUDE`, `INSERT`, `INSERT EXEC`, and `BEGIN PROGRAM`.
- **Programmability blocks**: `BEGIN PROGRAM` / `END PROGRAM` for Python or R; `BEGIN GPL` for the Graphics Production Language (used by Chart Builder templates); `BEGIN MATRIX` / `END MATRIX` for the matrix language.
- **Command descriptors**: autocomplete and syntax help draw on the 328 XML definitions [E6].

**Why:** syntax is the archival form of an analysis. It is diffable, reviewable, reusable, and independent of the GUI version.

---

## 7. The Output Viewer

### 7.1 Document structure

The Viewer window for the Frequencies run was titled `*Output1 [Document1] - StatsBench Viewer` [E17]. Its left pane is an **outline tree**:

```
Output
  Log
  Frequencies
    Title
    Notes
    Active Dataset
    Statistics
    Age category
```

The right pane renders the corresponding objects: the log text, the heading `Frequencies`, the dataset path, the `Statistics` pivot table (N Valid 6, Missing 0), and the `Age category` frequency table with columns Frequency, Percent, Valid Percent, and Cumulative Percent.

### 7.2 The Log object

The Log is not a console dump; it is a first-class output object containing the commands that ran. That is what makes an `.spv` file self-documenting: anyone can see the exact `GET FILE`, `DATASET NAME`, `DATASET ACTIVATE`, and `FREQUENCIES` calls behind the tables [E17].

### 7.3 Pivot tables

The output tables are **pivot tables**, not images. The `output-1.8.xsd` defines a `pivotTable` node with caption, corner text, dimensions, categories, and cells [E10]. In the Viewer, a pivot table can be:

- transposed (rows and columns swapped),
- pivoted (dimensions moved between layers, rows, and columns),
- edited (labels, decimals, widths, footnotes),
- restyled (table looks, fonts, borders),
- exported or copied into Word/Excel/PowerPoint.

**Why:** the same result often needs to be read in different orientations, and a table that can be reshaped is more useful than a frozen screenshot. It also makes the output machine-processable (OMS, XML, StatsBench Server).

### 7.4 Charts, trees, and models

The schema also defines `chart`, `treeView`, and `model` nodes [E10]. The Viewer can therefore host not only tables but also:

- charts from Chart Builder / Legacy Dialogs,
- decision trees from Tree,
- model views from Regression, Neural Networks, and forecasting procedures.

The right-hand panel of the Viewer changes with the selected object (chart editor, table editor, model viewer), which is why the install carries separate chart-editor, pivot-table-editor, and model-viewer JARs [E5].

### 7.5 Export and printing

Export is a command, not a screen scrape. `OUTPUT EXPORT` supports `DOC`, `HTML`, `PDF`, `PPT`, `TEXT`, `XLS`, `XLSX`, `XLSM`, `BMP`, `EMF`, `EPS`, `JPG`, `PNG`, and `TIF` [E6]. The Viewer's File menu exposes the same operation, and the Output Management System can route selected objects to files or datasets without opening the Viewer at all.

### 7.6 Why the output model is designed this way

- **Reproducibility:** the log makes the output self-describing.
- **Reusability:** pivot tables can be reshaped and exported to Office formats.
- **Automation:** OMS can capture tables as XML/HTML/`.sav` for downstream systems.
- **Extensibility:** a new procedure only has to emit standard output objects; the Viewer will display them without new UI code.

---

## 8. Menu-by-menu feature reference

The eleven top-level menus are the product's feature map. Each subsection documents the observed items, what they do, how they are implemented, why they exist, and where the evidence is.

Menu captures: File, Edit, View, Data, Transform, Analyze, Graphs, Utilities, Extensions, Window, Help [E22]; Analyze submenus [E6], [E22]; Data submenus [E24]; Graphs submenus [E25].

### 8.1 File

The File menu is the document and session layer: create/open/save data and output, import/export, print, manage datasets, and leave the application.

| Item | What it does | How it works | Why it exists |
|---|---|---|---|
| **New ▸** | Creates a new document: Data Editor, Syntax, Output, or Script | Client-side document factory; new Data Editor is an empty dataset (`Untitled1 [DataSet0]`) | Start a fresh analysis without leaving the session |
| **Open ▸** | Opens a data file, syntax file, output file, or script | Issues `GET FILE` for data; `INSERT`/`INCLUDE` for syntax; opens `.spv` in the Viewer | Re-enter prior work |
| **Import Data ▸** | Imports Excel, CSV, text, or database data | Text Import Wizard / `GET DATA`, `GET TRANSLATE`, ODBC | Bring external data into the `.sav` model without conversion tools |
| **Open Restore Point...** | Reopens an auto-saved recovery point | Reads the restore-point store shown by Session Management | Recover work after a crash |
| **Close** | Closes the active document (`Ctrl+F4`) | Client document lifecycle | Tidy the workspace |
| **Get R Workspace** | Pulls an R workspace into the session | R integration bridge | Continue work started in R |
| **Save** (`Ctrl+S`) | Saves the active dataset in `.sav` form | `SAVE OUTFILE`; preserves dictionary and data | Persist work |
| **Save As...** | Saves under a new name/format | `SAVE OUTFILE` with format options (`/COMPRESSED`, `/UNSELECTED`) | Branch a dataset; export a copy |
| **Save All Data** | Saves every open dataset | Iterates `DATASET` objects | Multi-dataset sessions |
| **Save Restore Point...** | Writes a recovery point on demand | Serialises editor state and data | Checkpoint before a risky operation |
| **Export ▸** | Exports data or output to external formats | `SAVE TRANSLATE` (CSV/Excel/SAS/Stata), `OUTPUT EXPORT` | Deliver results to other tools |
| **Mark File Read Only** | Protects the dataset from writes | Client flag plus file attributes | Prevent accidental edits |
| **Revert to Saved File** | Discards unsaved changes and reloads | Reissues the original `GET FILE` | Undo a bad editing session |
| **File Enabled for Auto-Recovery** | Toggles automatic restore points (checked by default) | Periodic state capture | Crash protection |
| **Collect Variable Information** | Gathers metadata for the dataset | Reads the dictionary | Documentation and audit |
| **Rename Dataset...** | Renames the in-memory dataset (`DataSet1`) | `DATASET NAME` | Distinguish datasets in syntax and dialogs |
| **Display Data File Information ▸** | Shows the dictionary and file attributes (working file or external file) | `DISPLAY` / `SYSFILE INFO` | Verify metadata before analysis |
| **Cache Data...** | Copies data into memory for faster access | Engine-side cache | Speed up repeated analyses on large files |
| **Stop Processor** (`Ctrl+Period`) | Interrupts a running command | Engine cancellation | Escape a long or mistaken run |
| **Switch Server...** | Connects the client to a Statistics Server | Uses `statisticsd` (port 3027) | Client/server deployments |
| **Repository ▸** | Connects to a model/data repository | Repository commands (`REPOSITORY CONNECT`, `COPY`, `ATTRIBUTES`) | Share models and assets |
| **Print Preview / Print...** | Previews or prints the active document | Viewer print pipeline (`Ctrl+P`) | Paper and PDF workflows |
| **Welcome Dialog...** | Reopens the start dialog | Client dialog | Discover tutorials and recent files |
| **Recently Used Data / Files ▸** | Reopens recent items | Client MRU list | Fast return to work |
| **Exit** | Quits the application | Closes documents, prompts to save | Normal shutdown |

**Why the File menu looks like this:** StatsBench treats data, syntax, output, and scripts as separate document types with separate lifecycles. That is why New/Open/Save each have submenus rather than a single "file" concept. It also explains why **Save** (data) and **Export** (other formats) are distinct: `.sav` preserves the dictionary, while export formats generally do not.

**Evidence:** [E22] File menu capture; [E1] version/build; [E4] server config; [E6] `SAVE`, `SAVE TRANSLATE`, `OUTPUT EXPORT`, `DATASET`, `DISPLAY`, `SYSFILE INFO`, `REPOSITORY` commands; [E20] restore-point dialog.

### 8.2 Edit

The Edit menu is the clipboard, search, navigation, and options layer for the active document (Data Editor in Variable View shown).

| Item | What it does | How it works | Why it exists |
|---|---|---|---|
| **Undo / Redo** (`Ctrl+Z`, `Ctrl+Y`) | Reverses or reapplies the last edit | Client-side undo stack | Safe experimentation |
| **Cut / Copy / Paste** | Standard clipboard operations on cells, variables, or output | Client clipboard with StatsBench-aware formats | Move data between StatsBench and Office |
| **Copy with Variable Names** | Copies data with variable names as headers | Clipboard serialiser | Paste into a tool that needs machine names |
| **Copy with Variable Labels** | Copies data with labels as headers | Clipboard serialiser | Readable pastes for reports |
| **Paste Variables...** | Pastes a list of variables into the current dataset | Reads clipboard variable list | Build a dataset from a variable plan |
| **Paste with Variable Names** | Pastes while preserving names | Clipboard serialiser | Round-trip tables |
| **Clear** (`Del`) | Deletes the selected cells/rows/variables | Data-editor edit | Remove unwanted content |
| **Insert Variable** | Adds a new variable row in Variable View | Dictionary edit | Extend the schema |
| **Insert Cases** | Adds new case rows | Data-editor edit | Make room for new records |
| **Search Data Files** | Searches across data files for a variable/value | Metadata index | Find where a variable lives |
| **Search** (`Ctrl+Shift+S`) | Global search in the current document | Client search | Locate text/values |
| **Find...** (`Ctrl+F`) | Finds text in the active view | Client search | Navigate large grids/output |
| **Find Next** (`F3`) | Repeats the last find | Client search | Iterate matches |
| **Replace...** (`Ctrl+H`) | Find and replace | Client edit | Bulk corrections |
| **Go to Case...** | Jumps to a case number | Data-editor navigation | Inspect a specific record |
| **Go to Variable...** | Jumps to a variable column | Data-editor navigation | Inspect a specific field |
| **Go to Imputation...** | Jumps to imputed values | Multiple-imputation metadata | Audit imputations |
| **Options...** | Opens global preferences (view, output, charts, currency, etc.) | Client settings store | Personalise the environment |

**Why:** the Data Editor is an editor as well as a viewer. Analysts spend a large share of time locating, correcting, and copying values, so standard editing plus StatsBench-specific copy modes (names vs labels) are first-class features.

**Evidence:** [E22] Edit menu capture; [E6] `VARIABLE`, `FILTER`, `WEIGHT` command families; [E5] client JARs.

### 8.3 View

The View menu controls what the current window shows. In Variable View the observed items were:

| Item | What it does | How it works | Why it exists |
|---|---|---|---|
| **Status Bar** (checked) | Shows/hides the bottom status bar ("Processor is ready", "Unicode: ON") | Client chrome toggle | Reclaim screen space / monitor state |
| **Toolbars ▸** | Shows/hides and customises toolbars | Client toolbar registry | Adapt the UI to the task |
| **Menu Editor...** | Customises menus and shortcuts | Client menu model | Personal workflows |
| **Fonts...** | Changes the display font | Client look-and-feel | Readability and projection |
| **Grid Lines** (checked) | Shows/hides cell grid lines | Data-editor renderer | Visual density |
| **Value Labels** (checked) | Toggles display of value labels vs raw codes | Reads variable value-label metadata | Readable data entry vs code inspection |
| **Mark Imputed Data** | Highlights imputed values (disabled when no imputations exist) | Multiple-imputation metadata | Transparency about completed data |
| **Customize Variable View...** | Chooses which metadata columns are visible | Column model | Focus on relevant metadata |
| **Data** (`Ctrl+T`) | Switches to Data View | View switch | Move between schema and cases |

**Why:** StatsBench distinguishes **presentation** from **data**. Value labels can be shown or hidden without changing stored values; grid lines and fonts are display-only; imputation marking is a visual audit aid. That separation is what lets the same dataset serve data entry, analysis, and reporting.

**Evidence:** [E22] View menu capture; [E13] Variable View; [E14] Data View with labels.

### 8.4 Data

The Data menu is the data-management layer: it changes the dictionary, joins and reshapes datasets, selects and weights cases, and prepares data for modelling. Observed items (in order) [E22], [E24]:

#### Define Variable Properties...

- **What:** A spreadsheet-like dialog for editing metadata in bulk: name, label, type, width, decimals, value labels, missing values, measurement level, and role. It can scan the actual data and suggest value labels and measurement levels.
- **How:** Writes dictionary commands (`VARIABLE LABELS`, `VALUE LABELS`, `MISSING VALUES`, `VARIABLE LEVEL`, `VARIABLE WIDTH`, `VARIABLE ALIGN`) without changing the data.
- **Why:** After importing raw codes (1/2/3, 0/1), the analyst needs to turn them into a documented, analysis-ready dictionary. Doing it in one dialog is faster and less error-prone than a dozen separate commands.
- **Evidence:** [E22]; command families in [E6].

#### Set Measurement Level for Unknown...

- **What:** Assigns Scale/Ordinal/Nominal to variables whose level is unknown (typically after import).
- **How:** `VARIABLE LEVEL`.
- **Why:** Level drives which procedures and charts are valid; unknown levels block or misconfigure analyses.
- **Evidence:** [E6], [E19] (Chart Builder's level warning).

#### Create Value Labels from Data

- **What:** Derives value labels from the string values present in the data.
- **How:** Extension command `STATS VALLBLS FROMDATA` (`STATS_VALLBLS_FROMDATA` bundle).
- **Why:** Imported files often contain text categories but no label metadata; this reconstructs the dictionary automatically.
- **Evidence:** [E11] extension list; [E6] command catalog.

#### Copy Data Properties...

- **What:** Copies the dictionary (labels, levels, missing rules, formats) from another dataset or file onto the current data.
- **How:** `APPLY DICTIONARY`.
- **Why:** Recurring surveys and panels share a schema; copying properties avoids re-documenting each wave.
- **Evidence:** [E6].

#### New Custom Attribute...

- **What:** Defines an arbitrary name/value attribute for variables or the data file.
- **How:** `VARIABLE ATTRIBUTE` / `DATAFILE ATTRIBUTE`.
- **Why:** Study-specific metadata (question text, wave, source) can travel with the `.sav` file without abusing labels.
- **Evidence:** [E6].

#### Define date and time...

- **What:** Wizard for building date/time variables from parts and extracting parts from dates.
- **How:** `DATE` command (e.g. `DATE ... /FORMAT`, `DATE ... /EXTRACT`).
- **Why:** Dates are stored as seconds since 1582-10-14 in StatsBench; the wizard hides the conversion arithmetic and avoids off-by-one errors.
- **Evidence:** [E6]; DateTime type in Variable View [E13].

#### Define Multiple Response Sets...

- **What:** Declares a set of variables as a multiple-dichotomy or multiple-category response set.
- **How:** `MRSETS` command; the set then becomes available to Frequencies/Crosstabs and `MULT RESPONSE`.
- **Why:** Survey questions such as "which brands do you know?" are stored as many binary columns; the set abstraction lets procedures treat them as one question.
- **Evidence:** [E6]; the Analyze > Multiple Response submenu showed Frequencies/Crosstabs greyed out until a set is defined [E22].

#### Validation ▸

Submenu observed: **Load Predefined Rules...**, **Define Rules...**, **Validate Data...** [E24].

- **What:** Defines data-quality rules (ranges, allowed values, consistency checks) and validates cases against them.
- **How:** `VALIDATEDATA` command; rules can be loaded from a predefined rule set or defined interactively; output lists violations and can flag/filter cases.
- **Why:** Statistical results are only as good as the data; validation turns ad-hoc checking into a repeatable, documented step.
- **Evidence:** [E24]; `VALIDATEDATA` in [E6]; `validatedata_stroke.sps` sample [E7].

#### Identify Duplicate Cases...

- **What:** Finds duplicate records by key variables and creates an indicator/filter.
- **How:** The dialog sorts by the key and derives a duplicate indicator; the exact generated command was not captured, and the install has no standalone `DUPLICATES` command in the 328-command catalog, so it is implemented with the data-management command set (sort/aggregate/filter) rather than a dedicated procedure. **(inferred)**
- **Why:** Duplicates bias counts and models; a systematic identification step is safer than manual sorting.
- **Evidence:** [E22]; command catalog [E6].

#### Identify Unusual Cases...

- **What:** Anomaly detection: flags cases that deviate from the typical pattern across variables.
- **How:** `DETECTANOMALY`; the procedure clusters cases, computes a deviation index, and reports the variables contributing to each anomaly.
- **Why:** Outliers can dominate means, regressions, and clusters; anomaly detection surfaces them before modelling.
- **Evidence:** [E6]; `detectanomaly_stroke.sps` sample [E7].

#### Compare Datasets...

- **What:** Compares two datasets case-by-case and variable-by-variable.
- **How:** `COMPARE DATASETS`.
- **Why:** After merging, restructuring, or re-importing, the analyst needs to prove that the data did not change unexpectedly.
- **Evidence:** [E6].

#### Sort Cases...

- **What:** Sorts cases by one or more keys, ascending or descending.
- **How:** `SORT CASES BY key (A|D) ...`.
- **Why:** Order matters for lag/lead operations, duplicate detection, reports, and reproducible file comparisons.
- **Evidence:** [E6].

#### Sort Variables...

- **What:** Reorders variables in the dataset.
- **How:** `SORT VARIABLES`.
- **Why:** Wide survey files become navigable when variables are grouped logically; order also affects default listings.
- **Evidence:** [E6].

#### Transpose...

- **What:** Swaps rows and columns (cases become variables and vice versa).
- **How:** `FLIP`.
- **Why:** Some data arrive in the wrong orientation (variables as rows); transposition fixes the shape without external tools.
- **Evidence:** [E6].

#### Adjust String Widths Across Files

- **What:** Harmonises string variable widths across files before merging.
- **How:** Extension `STATS_ADJUST_WIDTHS`.
- **Why:** `MATCH FILES`/`ADD FILES` require compatible string widths; mismatches are a common merge failure.
- **Evidence:** [E11], [E6].

#### Merge Files ▸

Submenu observed: **Add Cases...**, **Add Variables...** [E24].

- **Add Cases:** appends cases from another file; maps variables, excludes unwanted ones, and can add a source indicator. Command: `ADD FILES`.
- **Add Variables:** joins variables by a key (or by order); supports one-to-one, one-to-many, and table lookup. Command: `MATCH FILES`.
- **Why:** Most real analyses combine waves (cases) or sources (variables); merge semantics (key matching, missing keys, duplicate keys) need explicit control.
- **Evidence:** [E24]; [E6].

#### Restructure...

- **What:** Converts between wide and long formats.
- **How:** `VARSTOCASES` (wide → long) and `CASESTOVARS` (long → wide), with index variables and ID variables.
- **Why:** Repeated measures, panel data, and multi-response data are often stored in the opposite shape from what a procedure expects.
- **Evidence:** [E6].

#### Propensity Score Matching... (observed twice)

- **What:** Matches treated and control cases on a propensity score so groups are comparable.
- **How:** Extension-based (`STATS_PSM` / `PSM` bundles); estimates the propensity model, performs matching (nearest neighbour, caliper, etc.), and reports balance.
- **Why:** Observational studies need covariate balance; matching is a standard causal-inference workflow.
- **Evidence:** [E11], [E22]. The menu showed the entry twice; that is recorded as observed without inferring a cause.

#### Rake Weights...

- **What:** Adjusts survey weights so marginal totals match known population margins (raking/iterative proportional fitting).
- **How:** Extension `EXT_RAKE`.
- **Why:** Post-stratification weighting reduces non-response bias in surveys.
- **Evidence:** [E11].

#### Case Control Matching...

- **What:** Matches cases and controls on specified characteristics.
- **How:** Extension-based matching tools.
- **Why:** Case-control designs require comparable groups; manual matching does not scale.
- **Evidence:** [E11], [E22].

#### Aggregate...

- **What:** Collapses cases into groups and computes aggregate statistics (mean, sum, N, min, max, SD, median, percentiles, first, last, etc.).
- **How:** `AGGREGATE /BREAK=... /OUTFILE=... /variables=functions`.
- **Why:** Produces one row per group for reporting or for merging group-level context back onto individual cases.
- **Evidence:** [E6].

#### Orthogonal Design ▸

Submenu observed: **Generate...**, **Display...** [E24].

- **What:** Generates fractional factorial/orthogonal designs for conjoint and screening experiments, and displays existing designs.
- **How:** `ORTHOPLAN`.
- **Why:** Full factorial designs explode combinatorially; orthogonal designs retain estimable main effects with far fewer profiles.
- **Evidence:** [E24]; [E6]; `conjoint.sps` sample [E7].

#### Split into Files

- **What:** Writes one file per group defined by a variable.
- **How:** Extension `EXT_SPLIT_DATASET`.
- **Why:** Distributing or processing data by region/site/wave is a common batch need.
- **Evidence:** [E11].

#### Copy Dataset

- **What:** Duplicates the current dataset in memory.
- **How:** `DATASET COPY`.
- **Why:** Experiment with recodes or selections without destroying the original.
- **Evidence:** [E6].

#### Split File...

- **What:** Runs subsequent analyses separately for each group (or in one table with group layers).
- **How:** `SPLIT FILE BY ...`; can compare groups or organise output by group.
- **Why:** Subgroup analysis without writing separate commands for every group.
- **Evidence:** [E6].

#### Select Cases...

- **What:** Filters cases by condition, range, random sample, or time/date; can copy selected cases to a new dataset.
- **How:** `SELECT IF`, `FILTER BY`, `SAMPLE`.
- **Why:** Analyses often target a subset (complete cases, a region, a random holdout); selection must be explicit and reversible.
- **Evidence:** [E6].

#### Weight Cases...

- **What:** Applies frequency or analytic weights.
- **How:** `WEIGHT BY variable`; weight state is shown in the status bar and affects subsequent procedures.
- **Why:** Survey and aggregate data are often stored as counts; weighting restores the intended population.
- **Evidence:** [E6].

**Why the Data menu is so large:** data preparation is usually the longest part of an analysis. StatsBench keeps it in the same command language and dictionary model as the analysis, so a prepared dataset is fully reproducible rather than the result of undocumented spreadsheet edits.

### 8.5 Transform

Transformations create or modify variables. Observed items [E22]:

#### Compute Variable...

- **What:** Creates a new variable (or replaces one) from an arithmetic/logical expression, with optional conditional case selection.
- **How:** `COMPUTE target = expression` plus `IF (condition) ...`; uses the built-in function library (arithmetic, string, date, statistical, missing-value functions).
- **Why:** Derived measures (BMI, totals, ratios, flags) are the backbone of analysis; computing them in syntax keeps the derivation auditable.
- **Evidence:** [E6]; Transform menu [E22].

#### Programmability Transformation...

- **What:** Runs a transformation written in Python or R against the active dataset.
- **How:** `BEGIN PROGRAM` / `END PROGRAM` with the StatsBench Python or R API, or `StatsBenchINC TRANS`.
- **Why:** When the built-in function library is insufficient, the analyst can extend it without leaving the data step.
- **Evidence:** [E12], [E11] (`EXT_TRANS`).

#### Count Values within Cases...

- **What:** Counts how many variables in a list satisfy a value condition, per case.
- **How:** `COUNT target = var1 var2 ... (value/range)`.
- **Why:** Symptom checklists, multi-select items, and risk scores are naturally "count of positives".
- **Evidence:** [E6].

#### Shift Values

- **What:** Creates lagged or led values of a variable.
- **How:** `SHIFT VALUES`.
- **Why:** Time-series models and change scores need the previous/next observation on the same row.
- **Evidence:** [E6].

#### Recode into Same Variables...

- **What:** Changes values in place using old→new mappings, ranges, and ELSE.
- **How:** `RECODE var (old=new) ... (ELSE=...)`.
- **Why:** Fix coding errors and collapse categories without adding variables; value labels can be updated at the same time.
- **Evidence:** [E6].

#### Recode into Different Variables...

- **What:** Creates new variables from recoded values, leaving the originals intact.
- **How:** `RECODE old (old=new) INTO new`.
- **Why:** The safer default for analysis: the original data remain available for verification.
- **Evidence:** [E6].

#### Automatic Recode...

- **What:** Converts string or numeric values into consecutive integer codes and writes value labels.
- **How:** `AUTORECODE`.
- **Why:** Procedures need numeric categories; automatic recoding preserves the mapping in labels instead of losing it.
- **Evidence:** [E6].

#### Create Dummy Variables

- **What:** Creates k−1 indicator variables for a categorical variable.
- **How:** Extension `EXT_CREATE_DUMMIES`.
- **Why:** Regression and other models need numeric indicators; manual dummy creation is error-prone (wrong reference category, missed level).
- **Evidence:** [E11].

#### Visual Binning...

- **What:** Interactive binning of a continuous variable into categories, with cutpoints, labels, and a live histogram.
- **How:** Generates `RECODE`/`COMPUTE` plus `VALUE LABELS`.
- **Why:** Age bands, income bands, and risk tiers are usually analyst-defined; visual cutpoints make the choice transparent.
- **Evidence:** [E22]; [E6] (`RECODE`).

#### Optimal Binning...

- **What:** Supervised binning that chooses cutpoints to maximise association with a target variable.
- **How:** `OPTIMAL BINNING`.
- **Why:** Equal-width bins can destroy signal; optimal bins preserve monotonic relationships for scorecards and models.
- **Evidence:** [E6].

#### Prepare Data for Modeling ▸

- **What:** Automated data preparation for predictive modelling: target/input roles, date handling, missing-value treatment, outlier handling, and transformations.
- **How:** `ADP` command with `FIELDS TARGET=... INPUT=... ANALYSISWEIGHT=...`, plus `PREPDATETIME` options; produces a prepared dataset and a transformation log.
- **Why:** Model quality depends more on preparation than on the algorithm; automation makes the steps consistent and repeatable.
- **Evidence:** [E6] (`ADP.xml` explicitly defines TARGET/INPUT/ANALYSISWEIGHT); Transform menu [E22].

#### Rank Cases...

- **What:** Ranks cases by one or more variables; can produce normal scores, percentile ranks, and ntiles.
- **How:** `RANK`.
- **Why:** Nonparametric tests, decile analysis, and rank-based scores need well-defined tie handling.
- **Evidence:** [E6].

#### Date and Time Wizard...

- **What:** Builds date/time variables and extracts components (year, month, day, weekday, duration).
- **How:** `DATE`.
- **Why:** Dates are stored as a numeric epoch; the wizard prevents format and timezone mistakes.
- **Evidence:** [E6].

#### Create Time Series...

- **What:** Creates lag, difference, moving average, cumulative, and seasonal variables.
- **How:** `CREATE`.
- **Why:** Forecasting and trend analysis need a stable set of derived series; doing it in syntax keeps the transformations reproducible.
- **Evidence:** [E6].

#### Replace Missing Values...

- **What:** Fills missing values using series mean, mean of nearby points, median, linear interpolation, or linear trend.
- **How:** `RMV`.
- **Why:** Some procedures cannot handle gaps; simple imputation is a transparent first step (with the caveat that it understates uncertainty).
- **Evidence:** [E6].

#### Random Number Generators...

- **What:** Selects the RNG (e.g. Mersenne Twister) and sets/initialises the seed.
- **How:** `SET RNG`.
- **Why:** Random sampling, bootstrap, and simulation must be reproducible; an unseeded RNG makes results irreproducible.
- **Evidence:** [E6].

#### Run Pending Transforms (`Ctrl+G`)

- **What:** Forces execution of queued transformations.
- **How:** `EXECUTE`.
- **Why:** The Data Editor defers transformations until needed; this gives the analyst explicit control over when the data are materialised.
- **Evidence:** [E22].

---

### 8.6 Analyze

Analyze is the statistical core. The menu contained 27 entries [E22]:

Power Analysis, Reports, Descriptive Statistics, Bayesian Statistics, Tables, Compare Means, General Linear Model, Generalized Linear Models, Mixed Models, Correlate, Regression, Loglinear, Neural Networks, Classify, Dimension Reduction, Scale, Nonparametric Tests, Forecasting, Survival, Multiple Response, Missing Value Analysis, Multiple Imputation, Complex Samples, Simulation, Quality Control, Spatial and Temporal Modeling, Direct Marketing.

#### 8.6.1 Descriptive Statistics

##### Frequencies...

- **What:** Frequency tables for categorical/short-scale variables, with optional statistics, charts, percentiles, and missing-value handling.
- **How:** `FREQUENCIES` with subcommands `FORMAT`, `MISSING`, `BARCHART`, `PIECHART`, `HISTOGRAM`, `GROUPED`, `NTILES`, `PERCENTILES`, `STATISTICS`, `ORDER` [E6]. Produces a `Statistics` table and a frequency table (Frequency, Percent, Valid Percent, Cumulative Percent) [E17].
- **Why:** The first look at any categorical variable: counts, percentages, and the shape of the distribution. It separates valid from missing cases so percentages are interpretable.
- **Evidence:** [E6], [E15] dialog, [E16] generated syntax, [E17] output.

##### Descriptives...

- **What:** Summary statistics for continuous variables: mean, sum, SD, variance, range, min/max, SEM, skewness, kurtosis, and optional standardised (z) scores.
- **How:** `DESCRIPTIVES` with `STATISTICS`, `SAVE`, `SORT`, `MISSING` [E6]; `SAVE` writes z-scores; `SORT` orders variables by a chosen statistic.
- **Why:** Faster and more compact than Frequencies for many continuous variables; the z-score save is a common preprocessing step.
- **Evidence:** [E6].

##### Distribution Fit

- **What:** Fits candidate probability distributions to a variable and tests goodness of fit.
- **How:** Extension command `STATS DISTFIT` (bundle `STATS_DISTFIT`, R); parameters `VARIABLE`, `DISTRIBUTION`, `SHAPE1`, `SHAPE2`, `DF1`, `DF2`, `DF`, and `OPTIONS SIMULATEP/MINVALUE/MAXVALUE/QQPLOT` [E11].
- **Why:** Many models assume a distribution (normal, Poisson, gamma, Weibull); fitting and testing it is a principled precursor to modelling. Shipping it as an R extension shows how StatsBench 27 adds specialised methods.
- **Evidence:** [E11] extension descriptor and `Readme.md`.

##### Explore...

- **What:** Group-wise descriptive statistics, outlier identification, normality tests, homogeneity-of-variance tests, and diagnostic plots.
- **How:** `EXAMINE` with `COMPARE`, `TOTAL`, `ID`, `PERCENTILES`, `PLOT`, `STATISTICS`, `CINTERVAL`, `MESTIMATORS`, `MISSING` [E6]. Produces boxplots, stem-and-leaf, histograms, normality plots, spread-vs-level plots, and tests (Shapiro-Wilk, Kolmogorov-Smirnov/Lilliefors, Levene).
- **Why:** Before a t-test or ANOVA, the analyst needs to see distributions, outliers, and variance homogeneity. Explore is the diagnostic companion to the inferential procedures.
- **Evidence:** [E6].

##### Crosstabs...

- **What:** Contingency tables with chi-square tests, association measures, residual analysis, and layered tables.
- **How:** `CROSSTABS` with `TABLES`, `CELLS`, `STATISTICS`, `METHOD`, `COUNT`, `FORMAT`, `BARCHART`, `HIDESMALLCOUNTS`, `SHOWDIM` [E6]. Supports Pearson chi-square, likelihood ratio, Fisher exact, Yates correction, phi/Cramér's V, lambda, gamma, tau-b/c, Somers' d, kappa, McNemar, Mantel-Haenszel, and risk estimates.
- **Why:** The standard tool for two categorical variables; the association measures and residuals prevent the common mistake of reading a significant chi-square as a strong relationship.
- **Evidence:** [E6].

##### TURF Analysis

- **What:** Total Unduplicated Reach and Frequency: finds combinations of items (e.g. products, media, features) that maximise unique reach and frequency.
- **How:** Extension `EXT_TURF`; works on multiple dichotomy sets and searches/evaluates combinations.
- **Why:** Portfolio and media planning ask "which set of options reaches the most people?"; TURF answers it directly instead of by trial and error.
- **Evidence:** [E11], [E22].

##### Column proportions test

- **What:** Compares column proportions in a table and marks significant differences (with multiple-comparison adjustment).
- **How:** Extension `ColPropTest`, designed for Custom Tables / categorical tables.
- **Why:** A chi-square says "something differs"; the column-proportions test says *which* columns differ, with controlled false positives.
- **Evidence:** [E11], [E22].

##### Ratio...

- **What:** Summary statistics for ratios (e.g. cost per case, revenue per employee), including mean, median, weighted mean, coefficient of variation, and confidence intervals.
- **How:** `RATIO STATISTICS`.
- **Why:** Ratios have skewed distributions and a different sampling theory from ordinary means; naive averaging of ratios is biased.
- **Evidence:** [E6].

##### P-P Plots... / Q-Q Plots...

- **What:** Probability-probability and quantile-quantile plots to assess whether data follow a theoretical distribution.
- **How:** `PPLOT` command with distribution choice and transformations.
- **Why:** Visual distribution checks complement formal tests, which are sensitive to sample size.
- **Evidence:** [E6].

#### 8.6.2 Compare Means

##### Means...

- **What:** Subgroup means with counts, SD, SEM, sum, and an ANOVA table; optionally tests for linearity and reports eta/eta-squared.
- **How:** `MEANS` with `CELLS` and `STATISTICS`.
- **Why:** A compact group-comparison table for reporting, distinct from the hypothesis-test-focused t-test/ANOVA dialogs.
- **Evidence:** [E6].

##### One-Sample T Test...

- **What:** Compares a sample mean with a hypothesised test value.
- **How:** `T-TEST /TESTVAL=... /VARIABLES=...`; reports t, df, p, mean difference, 95% CI, and effect size (`ES`).
- **Why:** The canonical test for a single mean (e.g. quality target, national benchmark).
- **Evidence:** [E6].

##### Independent-Samples T Test...

- **What:** Compares means of two independent groups.
- **How:** `T-TEST /GROUPS=group(1 2) /VARIABLES=...`; Levene's test chooses equal-variance or Welch/Satterthwaite results; reports CI and effect size.
- **Why:** The most common two-group comparison (treatment vs control, male vs female). Levene prevents the classic mistake of ignoring unequal variances.
- **Evidence:** [E6].

##### Summary Independent-Samples T Test

- **What:** Performs the same test from summary statistics (n, mean, SD) rather than raw data.
- **How:** `T-TEST` with summary input (or the corresponding dialog that accepts summary values).
- **Why:** Meta-analysis and published tables often provide only summaries; this avoids re-entering raw data.
- **Evidence:** [E22], [E6].

##### Paired-Samples T Test...

- **What:** Compares two related measurements (before/after, matched pairs).
- **How:** `T-TEST /PAIRS=var1 WITH var2`; reports paired differences, correlation, CI, and effect size.
- **Why:** Repeated measurements on the same unit are correlated; an independent-samples test would be wrong.
- **Evidence:** [E6].

##### One-Way ANOVA...

- **What:** Compares means across three or more groups defined by one factor, with post hoc tests, contrasts, and homogeneity tests.
- **How:** `ONEWAY` with `POSTHOC`, `CONTRAST`, `STATISTICS`, `PLOT MEANS`, `MISSING`, `MATRIX` [E6]. Post hoc options include LSD, Bonferroni, Sidak, Scheffé, Tukey, Tukey b, Duncan, R-E-G-W F/Q, Gabriel, Waller-Duncan, and Dunnett.
- **Why:** Extends the two-group t-test; post hoc control is essential because multiple pairwise tests inflate Type I error.
- **Evidence:** [E6].

##### One-Sample / Independent-Samples / Paired-Samples Proportions

- **What:** Hypothesis tests and confidence intervals for one proportion, two independent proportions, and paired proportions.
- **How:** `PROPORTIONS` command; reports z tests, CIs, and effect sizes.
- **Why:** Binary outcomes (conversion, defect, vote) are common, and proportion tests are the correct analogue of t-tests for them.
- **Evidence:** [E6], [E22].

#### 8.6.3 Reports

##### Codebook...

- **What:** A data-dictionary report: variable names, labels, types, value labels, missing rules, and summary statistics (frequencies/percentiles) in a printable document.
- **How:** `CODEBOOK` command.
- **Why:** Before sharing data, the analyst must document what every variable means; the codebook is the deliverable that makes a dataset usable by others.
- **Evidence:** [E6].

##### OLAP Cubes...

- **What:** Online Analytical Processing summary tables: statistics (mean, sum, count, etc.) crossed by grouping variables, with layers for interactive exploration.
- **How:** `OLAP CUBES`.
- **Why:** Executives and analysts need multi-dimensional summaries without writing a table procedure; OLAP cubes provide drill-down structure.
- **Evidence:** [E6].

##### Case Summaries...

- **What:** Lists cases (or group summaries) with selected variables and statistics; useful for auditing individual records.
- **How:** `SUMMARIZE`.
- **Why:** Sometimes the deliverable is the raw case list, not an aggregate; case summaries preserve case-level detail.
- **Evidence:** [E6].

##### Report Summaries in Rows... / Report Summaries in Columns...

- **What:** Formatted reporting with titles, footnotes, column arrangements, and summary breaks.
- **How:** `REPORT`.
- **Why:** Pre-Custom-Tables reporting; still useful for precisely laid-out text reports.
- **Evidence:** [E6].

#### 8.6.4 Power Analysis

Observed structure: **Means ▸**, **Proportions ▸**, **Correlations ▸**, **Regression ▸** [E22]. The command catalog contains a full `POWER` family: `POWER MEANS ONESAMPLE`, `POWER MEANS INDEPENDENT`, `POWER MEANS RELATED`, `POWER ONEWAY ANOVA`, `POWER PROPORTIONS ONESAMPLE/INDEPENDENT/RELATED`, `POWER PEARSON ONESAMPLE`, `POWER PARTIALCORR`, `POWER SPEARMAN ONESAMPLE`, `POWER UNIVARIATE LINEAR` [E6].

- **What:** Prospective and retrospective power calculations: given effect size, alpha, and n, compute power; or given power, compute the required n.
- **How:** Dedicated `POWER` commands with distributional approximations; output includes power, critical values, and sample-size tables/curves.
- **Why:** Studies that are too small waste resources and mislead; power analysis is the design-stage counterpart to significance testing.
- **Evidence:** [E6], [E22].

#### 8.6.5 Bayesian Statistics

Observed items: **One Sample Normal**, **One Sample Binomial**, **One Sample Poisson**, **Related Samples Normal**, **Independent Samples Normal**, **Pearson Correlation**, **Linear Regression**, **One-way ANOVA**, **Loglinear Models**, **One-way Repeated Measures ANOVA** [E22]. Commands: `BAYES ONESAMPLE`, `BAYES RELATED`, `BAYES INDEPENDENT`, `BAYES CORRELATION`, `BAYES REGRESSION`, `BAYES ANOVA`, `BAYES LOGLINEAR`, `BAYES REPEATED` [E6].

- **What:** Bayesian inference with default or user-specified priors; reports posterior summaries, credible intervals, Bayes factors, and diagnostic plots.
- **How:** MCMC sampling (the install ships posterior-plot templates such as `bayes.viztemplate`, `BYSPosteriorPlot.viztemplate`) and the `BAYES` command family.
- **Why:** Bayesian methods answer "how probable is the hypothesis given the data?" and remain valid with small samples; the menu brings them to analysts who do not write MCMC code.
- **Evidence:** [E6], [E8] templates, [E22].

#### 8.6.6 Tables

- **What:** Custom Tables: build multi-dimensional tables with statistics, totals, subtotals, nesting, significance tests, and table looks.
- **How:** `CTABLES` command; the dialog is the "Tables" entry (not to be confused with the legacy `TABLES` command).
- **Why:** Standard procedures produce fixed table layouts; Custom Tables produces publication-ready tables without exporting and rebuilding them.
- **Evidence:** [E6] (`CTABLES`, `TABLES`), [E22].

#### 8.6.7 General Linear Model

Observed submenu: **Univariate...**, **Multivariate...**, **Repeated Measures...**, **Variance Components...** [E22].

- **Univariate:** one dependent variable, factors and covariates, full-factorial or custom model, Type I–IV sums of squares, contrasts, post hoc tests, estimated marginal means, Levene's test, residual plots, and save options. Command: `UNIANOVA` / `GLM`.
- **Multivariate:** several dependent variables; reports Pillai's trace, Wilks' lambda, Hotelling's trace, and Roy's largest root, with the same factor/covariate structure. Command: `GLM` with multiple dependent variables (`MANOVA` for the legacy syntax).
- **Repeated Measures:** within-subject factors, between-subject factors, Mauchly's sphericity test, Greenhouse-Geisser/Huynh-Feldt corrections, contrasts, and profile plots. Command: `GLM` with `WSFACTOR`/`WSDESIGN`.
- **Variance Components:** random-effects models with REML/ML/ANOVA estimation of variance components. Command: `VARCOMP`.

**Why:** ANOVA/ANCOVA/MANOVA/repeated measures are the classical experimental designs; one framework with custom contrasts and effect sizes covers most between- and within-subject experiments.

**How (shared mechanics):** the GLM family partitions variance into model and error sums of squares, forms F ratios, and estimates marginal means at specified factor levels. Subcommands control the design (`DESIGN`), assumptions (`CRITERIA`), post hoc comparisons (`POSTHOC`), and saved diagnostics (`SAVE`, `OUTFILE`).

**Evidence:** [E6] (`GLM` subcommands include `WSFACTOR`, `MEASURE`, `RANDOM`, `METHOD`, `CRITERIA`, `PRINT`, `PLOT`, `LMATRIX`, `MMATRIX`, `KMATRIX`, `POSTHOC`, `EMMEANS`, `ROBUST`, `SAVE`, `OUTFILE`, `DESIGN`), [E22].

#### 8.6.8 Generalized Linear Models

Observed submenu: **Generalized Linear Models...**, **Generalized Estimating Equations...** [E22].

- **GLM (GENLIN):** extends linear models to non-normal outcomes via a distribution (normal, binomial, Poisson, negative binomial, gamma, Tweedie, ordinal, multinomial) and a link function (identity, log, logit, probit, complementary log-log, power). Reports parameter estimates, Wald/LR tests, deviance, AIC/BIC, estimated marginal means, and predicted/residual values. Command: `GENLIN` with `MODEL`, `CRITERIA`, `REPEATED`, `EMMEANS`, `PRINT`, `SAVE`, `OUTFILE`.
- **GEE:** for correlated observations (repeated measures, clustered samples) with working correlation structures (independent, exchangeable, AR(1), unstructured) and robust sandwich standard errors. Uses the `REPEATED` subcommand **(inferred: the command catalog has no separate GEE command)**.
- **Why:** Real outcomes are often counts, proportions, or skewed costs; forcing them into a linear model violates assumptions. GENLIN/GEE keep the regression framework while respecting the outcome distribution and dependence structure.
- **Evidence:** [E6], [E22].

#### 8.6.9 Mixed Models

Observed submenu: **Linear...**, **Generalized Linear...** [E22].

- **Linear mixed models:** fixed and random effects, repeated/covariance structures (compound symmetry, AR(1), unstructured), REML/ML estimation, Satterthwaite/Kenward-Roger degrees of freedom, estimated marginal means, and save options. Command: `MIXED`.
- **Generalized linear mixed models:** non-normal outcomes plus random effects. Command: `GENLINMIXED`.
- **Why:** Clustered and longitudinal data violate independence; mixed models model the clustering explicitly instead of aggregating it away.
- **Evidence:** [E6], [E22].

#### 8.6.10 Correlate

Observed submenu: **Bivariate...**, **Partial...**, **Distances...**, **Canonical Correlation** [E22].

- **Bivariate:** Pearson, Kendall's tau-b, or Spearman correlations; one- or two-tailed tests; flag significant correlations; matrix and confidence intervals. Commands: `CORRELATIONS`, `NONPAR CORR`.
- **Partial:** partial correlations controlling for one or more variables, with zero-order and part correlations. Command: `PARTIAL CORR`.
- **Distances:** similarity/dissimilarity matrices using Euclidean, squared Euclidean, Chebyshev, block, Minkowski, cosine, Pearson, chi-square, or phi-square measures. Command: `PROXIMITIES`.
- **Canonical Correlation:** correlates two sets of variables by finding linear combinations that maximise their correlation; reports canonical correlations, significance tests, and structure coefficients. Extension `STATS_CANCORR` (27).
- **Why:** Correlation is the entry point to relationship analysis; partial correlation addresses confounding, distances feed clustering/MDS, and canonical correlation handles multivariate association.
- **Evidence:** [E6], [E11], [E22].

#### 8.6.11 Regression

Observed submenu (16 entries) [E22]: Automatic Linear Modeling, Linear, Curve Estimation, Regression Relative Importance, Partial Least Squares, Ridge Regression, Binary Logistic, Multinomial Logistic, Ordinal, Probit, Nonparametric Regression, Nonlinear, Weight Estimation, 2-Stage Least Squares, Quantile, Optimal Scaling (CATREG).

##### Automatic Linear Modeling...

- **What:** Automated regression modelling: variable selection, transformation, handling of missing values/outliers, and validation, with a model-selection summary.
- **How:** `AIM` command; uses roles (target/input) and can split data into training/testing; produces a model, importance chart, and diagnostics.
- **Why:** Gives non-specialists a defensible model quickly, while exposing the steps so they can be reviewed.
- **Evidence:** [E6], [E22].

##### Linear...

- **What:** Ordinary least squares regression with multiple predictors, entry methods (Enter, Stepwise, Remove, Backward, Forward), blockwise entry, and extensive diagnostics.
- **How:** `REGRESSION` with `VARIABLES`, `DEPENDENT`, `METHOD`, `STATISTICS`, `CRITERIA`, `RESIDUALS`, `CASEWISE`, `SCATTERPLOT`, `PARTIALPLOT`, `SAVE`, `OUTFILE` [E6]. Reports R, R², adjusted R², ANOVA, coefficients, collinearity statistics, Durbin-Watson, residuals, and influence measures.
- **Why:** The workhorse of applied statistics; blockwise entry and diagnostics make the assumptions auditable.
- **Evidence:** [E6], [E18] dialog.

##### Curve Estimation...

- **What:** Fits and compares 11 curve types: linear, logarithmic, inverse, quadratic, cubic, power, compound, S-curve, logistic, growth, exponential.
- **How:** `CURVEFIT`.
- **Why:** Quick exploration of nonlinear trends without specifying a full nonlinear model.
- **Evidence:** [E6].

##### Regression Relative Importance

- **What:** Decomposes R² into the relative contribution of each predictor (dominance/relative-weights style).
- **How:** Extension `STATS_RELIMP` (R).
- **Why:** Standardised coefficients and p-values do not answer "which predictor matters most", especially with correlated predictors.
- **Evidence:** [E11].

##### Partial Least Squares...

- **What:** PLS regression for many/collinear predictors; extracts latent components that maximise covariance with the outcome.
- **How:** `PLS` command / extension.
- **Why:** Chemometrics, spectroscopy, and wide survey data often have more predictors than cases or severe collinearity.
- **Evidence:** [E6], [E11] (`PLS` bundle).

##### Ridge Regression

- **What:** Penalised regression that shrinks coefficients to stabilise estimates under multicollinearity.
- **How:** Extension `RRegRidge` (R); ridge trace and selection of the penalty parameter.
- **Why:** OLS coefficients can be unstable or sign-flipped when predictors are highly correlated; ridge trades bias for variance.
- **Evidence:** [E11], [E22].

##### Binary Logistic...

- **What:** Logistic regression for a two-category outcome; odds ratios, classification table, Hosmer-Lemeshow, ROC, and stepwise methods.
- **How:** `LOGISTIC REGRESSION` with `CATEGORICAL`, `CONTRAST`, `METHOD`, `SELECT`, `PRINT`, `CRITERIA`, `CLASSPLOT`, `SAVE`, `OUTFILE`.
- **Why:** Linear regression is invalid for a 0/1 outcome; logistic regression models the log-odds and yields interpretable odds ratios.
- **Evidence:** [E6].

##### Multinomial Logistic...

- **What:** Logistic regression for an unordered outcome with three or more categories.
- **How:** `NOMREG`.
- **Why:** Nominal outcomes (brand choice, diagnosis category) need a model that does not impose an order.
- **Evidence:** [E6].

##### Ordinal...

- **What:** Cumulative logit/probit/cloglog regression for ordered categories.
- **How:** `PLUM`; includes the test of parallel lines.
- **Why:** Ordinal outcomes (Likert, severity) contain more information than a binary split but less than a continuous measure.
- **Evidence:** [E6].

##### Probit...

- **What:** Binary probit regression (normal link).
- **How:** `PROBIT`.
- **Why:** In some disciplines (toxicology, economics) the probit link is the convention; results can differ from logit in the tails.
- **Evidence:** [E6].

##### Nonparametric Regression

- **What:** Smoothing-based regression that does not assume a global functional form.
- **How:** Extension `STATS_NONPAR_REGR`.
- **Why:** Exploratory analysis of curved relationships without committing to a polynomial.
- **Evidence:** [E11].

##### Nonlinear...

- **What:** User-specified nonlinear regression with starting values, parameter constraints, and loss functions.
- **How:** `NLR`.
- **Why:** Mechanistic models (growth curves, pharmacokinetics) are nonlinear in parameters and cannot be expressed as a linear model.
- **Evidence:** [E6].

##### Weight Estimation...

- **What:** Weighted least squares / estimation of weights to address heteroscedasticity.
- **How:** `WLS`.
- **Why:** When error variance changes with the predictors, OLS standard errors are wrong; weighting restores efficiency.
- **Evidence:** [E6].

##### 2-Stage Least Squares...

- **What:** Instrumental-variable regression for endogenous predictors.
- **How:** `2SLS`.
- **Why:** OLS is biased when a predictor is correlated with the error (simultaneity, omitted variables); instruments can identify the effect.
- **Evidence:** [E6].

##### Quantile...

- **What:** Quantile regression (median and other quantiles) rather than mean regression.
- **How:** `QUANTILE REGRESSION`.
- **Why:** Effects can differ across the distribution (e.g. policy helps low earners but not high earners); quantile regression reveals that.
- **Evidence:** [E6].

##### Optimal Scaling (CATREG)...

- **What:** Categorical regression with optimal scaling of nominal, ordinal, and numeric predictors.
- **How:** `CATREG`.
- **Why:** Converts categorical predictors into quantified scores that maximise fit, avoiding arbitrary dummy coding.
- **Evidence:** [E6].

#### 8.6.12 Loglinear

Observed submenu: **General...**, **Logit...**, **Model Selection...** [E22].

- **General:** Hierarchical loglinear models for multi-way contingency tables; tests of partial and marginal association, expected counts, residuals, and parameter estimates. Commands: `GENLOG`, legacy `HILOGLINEAR`.
- **Logit:** Logit models for a categorical response with categorical predictors, expressed as a loglinear model. Command: `GENLOG`.
- **Model Selection:** Automatic selection (e.g. backward elimination) of the loglinear model terms. Command: `HILOGLINEAR`.
- **Why:** Chi-square handles two variables; loglinear models handle three or more categorical variables and their interactions without treating one as the outcome.
- **Evidence:** [E6], [E22].

#### 8.6.13 Neural Networks

Observed submenu: **Multilayer Perceptron...**, **Radial Basis Function...** [E22].

- **Multilayer Perceptron (MLP):** feed-forward network with one or more hidden layers, activation functions, batch/online training, validation and stopping rules, ROC/classification output, and sensitivity analysis. Command: `MLP`.
- **Radial Basis Function (RBF):** radial-basis hidden units with centres learned from the data; fast training for function approximation and classification. Command: `RBF`.
- **Why:** Neural networks capture nonlinear interactions that regression misses; the dialogs expose validation and importance measures so the model is not a black box.
- **Evidence:** [E6], [E22].

#### 8.6.14 Classify

Observed submenu: **TwoStep Cluster...**, **K-Means Cluster...**, **Hierarchical Cluster...**, **Cluster Silhouettes**, **Tree...**, **Discriminant...**, **Nearest Neighbor...**, **ROC Curve...**, **ROC Analysis...** [E22].

##### TwoStep Cluster

- **What:** Clustering for large datasets with mixed continuous and categorical variables; can choose the number of clusters automatically (BIC/AIC) and reports cluster profiles and silhouette.
- **How:** `TWOSTEP CLUSTER`.
- **Why:** Classical hierarchical clustering does not scale and k-means assumes numeric data; TwoStep handles both.
- **Evidence:** [E6].

##### K-Means Cluster

- **What:** Partitioning into a user-specified k by iteratively minimising within-cluster variance.
- **How:** `QUICK CLUSTER`.
- **Why:** Fast and interpretable for large numeric datasets.
- **Evidence:** [E6].

##### Hierarchical Cluster

- **What:** Agglomerative clustering with linkage methods (between-groups, within-groups, Ward, centroid, median, single, complete) and distance measures; dendrogram and icicle plots.
- **How:** `CLUSTER`.
- **Why:** The dendrogram shows the entire merge history, which helps choose k and spot outliers.
- **Evidence:** [E6].

##### Cluster Silhouettes

- **What:** Per-case silhouette values measuring cohesion and separation, with plots.
- **How:** Extension `STATS_CLUS_SIL`.
- **Why:** A single clustering run can look plausible while containing poorly separated cases; silhouettes make that visible.
- **Evidence:** [E11].

##### Tree...

- **What:** Decision trees (CHAID, CRT, QUEST) for classification or regression, with pruning, surrogate splits, and gains/lift/risk tables.
- **How:** `TREE`.
- **Why:** Trees handle nonlinearity and interactions and produce human-readable rules; useful for segmentation and prediction.
- **Evidence:** [E6].

##### Discriminant...

- **What:** Linear/Fisher discriminant analysis with stepwise selection, canonical functions, classification table, and leave-one-out validation.
- **How:** `DISCRIMINANT`.
- **Why:** Predicts group membership from continuous predictors and tests which variables separate groups.
- **Evidence:** [E6].

##### Nearest Neighbor...

- **What:** k-nearest-neighbour classification/regression with distance weighting.
- **How:** `KNN`.
- **Why:** A nonparametric baseline that makes no functional-form assumption.
- **Evidence:** [E6].

##### ROC Curve...

- **What:** Receiver-operating-characteristic analysis for a single predictor and a binary state; AUC, coordinates, standard error, and optimal cutpoint.
- **How:** `ROC`.
- **Why:** Measures discrimination independent of prevalence and cutpoint.
- **Evidence:** [E6].

##### ROC Analysis...

- **What:** Compares multiple ROC curves and tests differences between AUCs.
- **How:** `ROC ANALYSIS`; extension-assisted in 27.
- **Why:** Model comparison needs a significance test, not just two AUC numbers.
- **Evidence:** [E6], [E22].

#### 8.6.15 Dimension Reduction

Observed submenu: **Factor...**, **Correspondence Analysis...**, **Optimal Scaling...** [E22].

- **Factor:** Principal components and common-factor extraction (principal axis, maximum likelihood, alpha), rotation (varimax, promax, oblimin, quartimax, equamax), KMO and Bartlett's test, communalities, factor scores, and scree plots. Command: `FACTOR`.
- **Correspondence Analysis:** Simple and multiple correspondence analysis for categorical contingency tables, with row/column coordinates and biplots. Commands: `ANACOR`, `MULTIPLE CORRESPONDENCE`.
- **Optimal Scaling:** Categorical principal components (`CATPCA`), homogeneity analysis (`HOMALS`), and nonlinear canonical correlation (`OVERALS`).
- **Why:** Many variables are correlated; dimension reduction finds the latent structure without choosing one outcome. Correspondence analysis does the same for categorical data.
- **Evidence:** [E6], [E22].

#### 8.6.16 Scale

Observed submenu: **Reliability Analysis...**, **Weighted Kappa...**, **Multidimensional Unfolding (PREFSCAL)...**, **Multidimensional Scaling (PROXSCAL)...**, **Multidimensional Scaling (ALSCAL)...** [E22].

- **Reliability Analysis:** Cronbach's alpha, split-half, Guttman, parallel and strict-parallel models; item-total statistics, inter-item correlations, and intraclass correlation. Command: `RELIABILITY`.
- **Weighted Kappa:** Agreement between raters with linear or quadratic weights. Command: `WEIGHTED KAPPA` / `FLEISS MULTIRATER KAPPA`.
- **PREFSCAL:** Multidimensional unfolding for preference data.
- **PROXSCAL:** Individual-differences/ordinal multidimensional scaling.
- **ALSCAL:** Classical multidimensional scaling.
- **Why:** Before summing items into a scale, the analyst must show the items measure one thing reliably; kappa does the same for inter-rater agreement. MDS recovers perceptual maps from similarity/preference data.
- **Evidence:** [E6], [E22].

#### 8.6.17 Nonparametric Tests

Observed submenu: **One Sample...**, **Independent Samples...**, **Related Samples...**, **Legacy Dialogs ▸** [E22].

- **One Sample:** Binomial test, chi-square goodness-of-fit, Kolmogorov-Smirnov, and runs test. Commands: `NPAR TESTS`, `NPTESTS`.
- **Independent Samples:** Mann-Whitney U, Kolmogorov-Smirnov, Moses, Kruskal-Wallis, median, and Jonckheere-Terpstra tests.
- **Related Samples:** Sign, Wilcoxon signed-rank, McNemar, Cochran's Q, Friedman, and Kendall's W.
- **Legacy Dialogs:** The older chi-square, binomial, runs, 1-sample K-S, 2-independent/related, and k-independent/related dialogs.
- **Why:** Parametric tests assume normality and interval data; rank/permutation tests are valid for ordinal data, small samples, and outliers. The new dialogs organise them by design (one/independent/related), matching the t-test family.
- **Evidence:** [E6] (`NPAR TESTS` subcommands include `CHISQUARE`, `K-S`, `RUNS`, `BINOMIAL`, `MCNEMAR`, `SIGN`, `WILCOXON`, `MH`, `COCHRAN`, `FRIEDMAN`, `KENDALL`, `M-W`, `W-W`, `MOSES`, `K-W`, `J-T`, `MEDIAN`), [E22].

#### 8.6.18 Forecasting

Observed submenu [E22]: **Create Temporal Causal Models...**, **Create Traditional Models...**, **Apply Temporal Causal Models...**, **Apply Traditional Models...**, **Seasonal Decomposition...**, **Spectral Analysis...**, **Sequence Charts...**, **Autocorrelations...**, **Cross-Correlations...**

- **Create Temporal Causal Models:** Builds models that combine ARIMA/regression structure with time-varying predictors (interventions, promotions, covariates), optionally with automatic model selection. Commands: `TCM ANALYSIS`, `TCM MODEL`.
- **Create Traditional Models:** Exponential smoothing (simple, Holt, Winters) and ARIMA models, with expert modeler, stationarity tests, and forecasts. Commands: `TSMODEL`, `EXSMOOTH`, `ARIMA`.
- **Apply Temporal/Traditional Models:** Scores new periods or new data with a saved model. Commands: `TCM APPLY`, `TSAPPLY`.
- **Seasonal Decomposition:** Splits a series into trend, seasonal, and error components (additive or multiplicative). Command: `SEASON`.
- **Spectral Analysis:** Periodogram/spectral density for cyclical structure. Command: `SPECTRA`.
- **Sequence Charts:** Time-series line plots. Command: `TSPLOT`.
- **Autocorrelations:** ACF with Box-Ljung statistics. Command: `ACF`.
- **Cross-Correlations:** CCF between two series. Command: `CCF`.
- **Why:** Time-ordered data violate independence; forecasting tools model trend, seasonality, autocorrelation, and interventions explicitly, and separate model building from scoring.
- **Evidence:** [E6], [E22]; sample `catalog_seasfac.sav` [E7].

#### 8.6.19 Survival

Observed submenu [E22]: **Life Tables...**, **Kaplan-Meier...**, **Cox Regression...**, **Cox w Time-Dep Cov...**, **Parametric Regression**, **Cox Regression Extension**.

- **Life Tables:** Actuarial life-table estimates of survival and hazard by interval. Command: `SURVIVAL`.
- **Kaplan-Meier:** Product-limit survival curves, median survival, log-rank (Mantel-Cox) and Breslow tests, and plots. Command: `KM`.
- **Cox Regression:** Proportional-hazards regression with covariates, categorical predictors, strata, and survival plots; reports −2 log likelihood, model chi-square, and hazard ratios. Command: `COXREG`.
- **Cox w Time-Dep Cov:** Cox models with time-dependent covariates.
- **Parametric Regression:** Accelerated-failure-time models (exponential, Weibull, lognormal, log-logistic, gamma) with a chosen survival distribution. Extension `STATS_SURVREG`.
- **Cox Regression Extension:** Extension-based Cox modelling (`STATS_COXREGR`).
- **Why:** Survival data are censored: some subjects have not experienced the event by the end of follow-up. Ordinary regression would discard or bias those cases; survival procedures use the censored information correctly.
- **Evidence:** [E6], [E11], [E22].

#### 8.6.20 Multiple Response

Observed submenu [E22]: **Define Variable Sets...**, **Frequencies...** (greyed until a set exists), **Crosstabs...** (greyed until a set exists).

- **Define Variable Sets:** Declares a multiple-dichotomy set (each variable is a yes/no item) or a multiple-category set (each variable is a category slot) with a name, label, and value/category labels. Command: `MRSETS`.
- **Frequencies:** Counts responses and cases for the set, with percentages of cases and responses. Command: `MULT RESPONSE`.
- **Crosstabs:** Crosses a multiple-response set with a categorical variable.
- **Why:** A survey question such as "which of these 12 brands do you know?" is stored as 12 binary columns. Treating each column separately answers the wrong question; the set treats them as one multiple-response question.
- **Evidence:** [E6], [E22] (the disabled items are direct evidence that sets gate the procedures).

#### 8.6.21 Missing Value Analysis...

- **What:** Examines missing-data patterns, describes missingness, tests whether data are missing completely at random (Little's MCAR test), and optionally imputes via EM or regression.
- **How:** `MVA` command with `DESCRIPTIVES`, `PATTERN`, `EM`, `REGRESSION`, `TTEST`, `MCAR`, `TABLES`.
- **Why:** Missingness can bias results; understanding the mechanism (MCAR/MAR/MNAR) determines whether deletion, single imputation, or multiple imputation is appropriate.
- **Evidence:** [E6].

#### 8.6.22 Multiple Imputation

Observed submenu: **Analyze Patterns...**, **Impute Missing Data Values...** [E22].

- **Analyze Patterns:** Displays missing-data patterns, counts, and variable combinations.
- **Impute Missing Data Values:** Fully conditional specification (MICE) or monotone imputation; chooses models per variable, the number of imputations, and constraints; produces an imputed dataset with imputation indicators and pooled results. Command: `MULTIPLE IMPUTATION`.
- **Why:** Single imputation underestimates uncertainty; multiple imputation creates several plausible datasets, analyses each, and pools estimates with correct standard errors.
- **Evidence:** [E6], [E22].

#### 8.6.23 Complex Samples

Observed submenu [E22]: **Select a Sample...**, **Prepare for Analysis...**, **Frequencies...**, **Descriptives...**, **Crosstabs...**, **Ratios...**, **General Linear Model...**, **Logistic Regression...**, **Ordinal Regression...**, **Cox Regression...**

- **Select a Sample:** Defines a complex sampling design (stratification, clustering, unequal probabilities/PPS) and draws a sample; the design is saved as a plan (`.csplan`).
- **Prepare for Analysis:** Applies a sampling plan to an existing dataset for analysis.
- **Frequencies/Descriptives/Crosstabs/Ratios:** Survey-adjusted estimates with design effects and correct standard errors.
- **GLM/Logistic/Ordinal/Cox:** Survey-weighted regression models.
- **Why:** Standard errors assume simple random sampling. Complex designs (clusters, strata, weights) change the variance; ignoring the design produces overconfident conclusions.
- **Evidence:** [E6] (`CSPLAN SAMPLE`, `CSPLAN ANALYSIS`, `CSDESCRIPTIVES`, `CSTABULATE`, `CSGLM`, `CSLOGISTIC`, `CSORDINAL`, `CSCOXREG`), [E7] (55 `.csplan`, 33 `.csaplan` samples), [E22].

#### 8.6.24 Simulation...

- **What:** Builds and runs simulation plans to study the behaviour of statistical models under specified assumptions (sample size, distributions, missingness, effect sizes).
- **How:** `SIMPLAN CREATE`, `SIMPLAN VIEW`, `SIMRUN`; the plan is a separate document.
- **Why:** Analytical power formulas cover simple designs; simulation covers realistic, messy scenarios and verifies that a planned analysis will work.
- **Evidence:** [E6].

#### 8.6.25 Quality Control

Observed submenu: **Control Charts...**, **Pareto Charts...** [E22].

- **Control Charts:** X-bar/R/S, individuals/moving range, p/np/c/u charts, with runs rules and Phase I/II limits. Command: `SPCHART`.
- **Pareto Charts:** Sorted frequency bars with a cumulative-percentage line, separating the "vital few" from the "trivial many".
- **Why:** Manufacturing and service quality depend on distinguishing common-cause from special-cause variation; control charts formalise that distinction.
- **Evidence:** [E6], [E22].

#### 8.6.26 Spatial and Temporal Modeling...

Observed submenu: **Spatial Modeling** [E22].

- **What:** Spatial association rules and spatial-temporal prediction: finds co-location patterns and forecasts values over space and time.
- **How:** Commands `SPATIAL ASSOCIATION RULES`, `SPATIAL MAPSPEC`, `SPATIAL TEMPORAL PREDICTION`; works with map specifications and geo-coded data.
- **Why:** Location and time interact; standard regression ignores spatial dependence and temporal drift.
- **Evidence:** [E6], [E22].

#### 8.6.27 Direct Marketing

Observed submenu: **Choose Technique** [E22].

- **What:** A guided entry point to direct-marketing analyses: RFM (recency/frequency/monetary) segmentation, customer clustering, prospect profiling, and response modelling.
- **How:** Commands `DMCLUSTER`, `DMLOGISTIC`, `DMROC`, `DMTABLES`, `DMTREE`; the "Choose Technique" dialog routes to the appropriate one.
- **Why:** Marketing teams need workflow-level guidance ("what analysis fits my question?") rather than a list of statistical procedures.
- **Evidence:** [E6], [E22].

### 8.7 Graphs

The Graphs menu builds charts. Observed items [E22], [E25]: **Chart Builder...**, **Graphboard Template Chooser...**, **Weibull Plot...**, **Fluctuation Chart**, **Compare Subgroups**, **Regression Variable Plots**, **Regression Residual Plot** (greyed in the observed state), **Boxplot Family**, **Legacy Dialogs ▸**.

#### Chart Builder...

- **What:** The modern charting environment. A gallery of chart types on the right/bottom, a drag-and-drop canvas in the centre, and element properties on the right. Variables are dragged onto axes, panels, and groupings; the chart previews with the real data or example data.
- **How:** Generates `GGRAPH` commands using the Graphics Production Language (GPL) templates. The dialog's **Paste** writes the `GGRAPH` command and its `BEGIN GPL` block. The install ships 50 `.viztemplate` files that supply chart layouts and styles [E8].
- **Why:** Charts are part of the analysis, not decoration. A drag-and-drop builder with live preview lets the analyst explore encodings (bar vs line vs boxplot, grouping, faceting) and then keep the exact chart as reproducible syntax.
- **Observed anatomy:** On first use it shows an advisory: measurement level should be set properly for each variable; **OK** proceeds, **Define Variable Properties** opens the metadata editor, and **Don't show this dialog again** suppresses it [E19]. The builder itself has **Gallery** (Favorites, Bar, Line, Area, Pie/Polar, Scatter/Dot, Histogram, High-Low, Boxplot, Dual Axes), **Basic Elements**, **Groups/Point ID**, and **Titles/Footnotes** tabs, with **Element Properties**, **Chart Appearance**, and **Options** on the right, and **OK/Paste/Reset/Cancel/Help** at the bottom [E19].
- **Evidence:** [E19] two Chart Builder screenshots; [E8] templates; [E6] `GGRAPH`, `BEGIN GPL`.

#### Graphboard Template Chooser...

- **What:** A template-driven chooser that presents chart types as a browsable \"graphboard\" and creates a chart from the selected template and data.
- **How:** Uses chart templates (`.viztemplate`) and the graphboard engine; produces the corresponding graphics command.
- **Why:** Gives users a catalogue of professionally designed chart types without requiring them to understand GPL or the Chart Builder canvas.
- **Evidence:** [E8], [E22].

#### Weibull Plot...

- **What:** A Weibull probability plot for reliability/survival data, with shape/scale estimates and goodness-of-fit.
- **How:** Extension `STATS_WEIBULL_PLOT`.
- **Why:** Weibull analysis is the standard reliability model for time-to-failure; a dedicated plot makes the fit visually checkable.
- **Evidence:** [E11], [E22].

#### Fluctuation Chart

- **What:** A control-chart-style plot of a process over time, highlighting unusual fluctuation.
- **How:** Extension `Fluctuation_Plot`.
- **Why:** Quick visual process monitoring without building a full control chart.
- **Evidence:** [E11].

#### Compare Subgroups

- **What:** Produces subgroup comparison plots (e.g. group means with intervals, or distributions by subgroup).
- **How:** Extension `STATS_SUBGROUP_PLOTS`.
- **Why:** Group comparisons are easier to interpret visually before/alongside the formal test.
- **Evidence:** [E11].

#### Regression Variable Plots / Regression Residual Plot

- **What:** Diagnostic plots for regression: partial plots, residual-vs-predicted, residual distributions, and influence plots.
- **How:** Extension `STATS_REGRESS_PLOT` / `STATS_RESIDUAL_BOXPLOTS`; the linear regression dialog also has its own **Plots...** sub-dialog.
- **Why:** Regression assumptions (linearity, homoscedasticity, normality of residuals, influential cases) are diagnosed visually; the dedicated menu item puts the plots one click away.
- **Evidence:** [E11], [E18].

#### Boxplot Family

- **What:** A family of boxplot variants (simple, clustered, stacked; with/without outliers; horizontal/vertical).
- **How:** Chart Builder templates / `GGRAPH` boxplot geometry.
- **Why:** Boxplots are the default distribution comparison; offering the family avoids re-building each variant.
- **Evidence:** [E8], [E25].

#### Legacy Dialogs ▸

Observed submenu [E25]: **Bar...**, **3-D Bar...**, **Line...**, **Area...**, **Pie...**, **High-Low...**, **Boxplot...**, **Error Bar...**, **Population Pyramid...**, **Scatter/Dot...**, **Histogram...**.

- **What:** The classic chart dialogs. Each has a simple/stacked/clustered variant, a data mode (summaries of groups or values of individual cases), and titles/options.
- **How:** Generates the older `GRAPH` command (e.g. `GRAPH /BAR(SIMPLE)=COUNT BY category`), which is still fully supported.
- **Why:** The legacy dialogs are faster for standard charts and are the syntax most existing StatsBench code uses. Keeping them preserves backward compatibility while Chart Builder serves advanced needs.
- **Evidence:** [E25]; [E6] `GRAPH`.

**Why the Graphs menu has three generations:** Legacy Dialogs (`GRAPH`), Chart Builder (`GGRAPH`/GPL), and Graphboard templates. Each targets a different trade-off between speed, control, and design. The command layer supports all three, so old syntax keeps working while new charts gain richer geometry.

### 8.8 Utilities

Observed items [E22]: **Variables...**, **OMS Control Panel...**, **OMS Identifiers...**, **Scoring Wizard...**, **Merge Model XML...**, **Calculate with Pivot Table** (disabled), **Data File Comments...**, **Define Variable Macro**, **Define Variable Sets...**, **Censor Table**, **Use Variable Sets...**, **Show All Variables**, **Create Text Output**, **Spelling...** (disabled), **Process Data Files**, **Run Script...**, **Modify Table Appearance**, **Production Facility...**, **Map Conversion Utility...**, **Python2 Conversion**.

| Item | What it does | How it works | Why it exists |
|---|---|---|---|
| **Variables...** | Shows the variables in the active dataset and their metadata | Reads the dictionary | Quick inventory without switching to Variable View |
| **OMS Control Panel...** | Selects output objects and routes them to files/datasets; can generate the `OMS` command | `OMS` with `SELECT`, `DESTINATION`, `IF`, `COLUMNS`, `LABELS` | Capture results for downstream systems without manual export |
| **OMS Identifiers...** | Lists the table/column identifiers available to OMS | Reads the output schema | You cannot select an object you cannot name |
| **Scoring Wizard...** | Applies a saved predictive model (PMML) to new data | `MODEL HANDLE` / scoring commands | Reuse models without rebuilding them |
| **Merge Model XML...** | Merges PMML model fragments | PMML/XML processing | Assemble models from multiple sources |
| **Calculate with Pivot Table** | Computes new values inside a pivot table | Pivot-table editor | Add derived rows/columns to output without returning to the data |
| **Data File Comments...** | Documents the dataset | `DOCUMENT` / `ADD DOCUMENT` | Provenance travels with the file |
| **Define Variable Macro** | Names a list of variables for reuse | `DEFINE` macro | Avoid repeating long variable lists |
| **Define Variable Sets...** | Groups variables into named sets | `VARIABLE SET` | Tame wide datasets in dialogs |
| **Censor Table** | Hides small-count cells | `OMS`/table options | Disclosure control |
| **Use Variable Sets / Show All Variables** | Filters the variable list in dialogs | Client variable-list model | Focus on relevant variables |
| **Create Text Output** | Produces plain-text output | Text output driver | Logs and legacy pipelines |
| **Spelling...** | Spell-checks text | Client spell checker | Presentation quality |
| **Process Data Files** | Runs a set of commands against many files | Batch/data-file processing | Production-style repeated runs |
| **Run Script...** | Runs a Basic/Python script | Scripting host | Automate the client itself |
| **Modify Table Appearance** | Applies table looks/styles | Pivot-table styling | Consistent reporting |
| **Production Facility...** | Runs production jobs (`.spj`) unattended | Production job runner | Scheduled, repeatable reporting |
| **Map Conversion Utility...** | Converts map files for spatial analysis | Map format converter | Prepare geography for mapping |
| **Python2 Conversion** | Converts Python 2 extension code to Python 3 | Extension migration tool | The bundled Python is 3.8; old extensions need porting |

**Why Utilities exists:** these are the \"meta\" features that make StatsBench operable at scale: naming variables, capturing output, reusing models, running jobs unattended, and migrating extensions. They are not statistical, but they determine whether an analysis can be repeated by someone else.

**Evidence:** [E22]; [E6] (`OMS`, `DOCUMENT`, `DEFINE`, `VARIABLE SET`, `MODEL HANDLE`, production commands); [E12] Python 3.8.

### 8.9 Extensions

Observed items [E22]: **My Extensions**, **Extension Hub...**, **Install Local Extension Bundle...**, **Custom Dialog Builder for Extensions**, **R 3.6 Configuration**, **Correlation Plot**, **Utilities ▸**, **Dynamite Plot**, **String 2 Nominal**, **Custom_Descriptive_Tables**.

- **My Extensions:** Lists installed extension bundles and their commands/dialogs.
- **Extension Hub...:** Browses and installs bundles from the extension hub.
- **Install Local Extension Bundle...:** Installs a downloaded bundle.
- **Custom Dialog Builder for Extensions:** Builds a dialog (`.spd`) for an extension command, so an R/Python routine gets a full StatsBench dialog.
- **R 3.6 Configuration:** Configures the R installation used by R-based bundles.
- **Correlation Plot / Dynamite Plot / String 2 Nominal / Custom_Descriptive_Tables:** Installed example bundles that add menu items and commands.
- **Utilities ▸:** A submenu of extension utilities.

**How extensions work (observed anatomy):** each bundle under `C:\ProgramData\StatsBench\Statistics\27\extensions\<NAME>` contains:

- one or more `.R` files (the implementation),
- a `.spd` dialog specification (a ZIP of dialog resources),
- an `.xml` command definition (syntax),
- a `.spe` extension specification,
- a `Readme.md` and licence.

The Distribution Fit bundle, for example, declares the command `STATS DISTFIT` with parameters `VARIABLE`, `DISTRIBUTION`, `SHAPE1`, `SHAPE2`, `DF1`, `DF2`, `DF`, and options such as `SIMULATEP` and `QQPLOT` [E11]. The Custom Dialog Builder produces the `.spd`; the command XML teaches the syntax editor and the engine about the command.

**Why:** statistics evolves faster than a desktop release cycle. The extension layer lets the original developer and third parties ship new procedures (and their dialogs) as bundles, while keeping the core product stable. It also explains a visible StatsBench 27 pattern: many of the new menu items are extensions, not core DLL procedures.

**Evidence:** [E11] bundle inventory and Distribution Fit descriptor; [E12] R/Python; [E22] Extensions menu.

### 8.10 Window

Observed items [E22]: **Split**, **Minimize All Windows**, **Go to Designated Viewer Window**, **Go to Designated Syntax Window**, **Reset Dialog Sizes and Positions**, and a numbered list of open windows (e.g. `1 Untitled1 [DataSet0] - StatsBench Data Editor`).

- **Split:** Tiles the MDI child windows so the Data Editor, Syntax, and Viewer can be seen together.
- **Minimize All Windows:** Clears the desktop.
- **Go to Designated Viewer/Syntax Window:** Jumps to the window marked as the output or syntax destination, regardless of which one is active.
- **Reset Dialog Sizes and Positions:** Restores dialogs that were moved off-screen or resized badly (a real recovery feature on multi-monitor setups).
- **Window list:** Switches between open documents; the active one is checked.

**Why:** StatsBench is a multi-document application (data, syntax, output, scripts, models). Window management is part of the workflow, not a cosmetic extra.

**Evidence:** [E22].

### 8.11 Help

Observed items [E22]: **Topics**, **StatsBench Support**, **StatsBench Forums**, **StatsBench Community**, **Documentation in PDF Format**, **Command Syntax Reference**, **About...**, **Diagnostic Tool...**, **Give Feedback**, **Report an Issue**.

- **Topics:** Opens the installed help system. The `helpids.csv` file maps 8,992 dialog/help IDs to topics [E9].
- **StatsBench Support / Forums / Community:** Web destinations for support and peer help.
- **Documentation in PDF Format:** The manuals, including `syntaxreference.pdf` (the command syntax reference).
- **Command Syntax Reference:** Direct entry to the syntax manual.
- **About...:** Version, build, licence, and installed modules.
- **Diagnostic Tool...:** Collects environment/log information for support.
- **Give Feedback / Report an Issue:** Feedback channels, including the error-reporting path observed at startup [E21].

**Why:** A product with 328 commands and 8,992 help topics cannot rely on tooltips. Context help, a searchable syntax reference, and a diagnostic bundle are how users and support staff resolve problems.

**Evidence:** [E22]; [E9]; [E21].

---

## 9. Statistical mechanics: what the procedures compute

The menu entries in §8 say *what* each procedure is for. This section documents the computational core: the quantities computed, the assumptions, and the diagnostics the product reports. The formulas are the standard definitions that the command descriptors and output labels imply; they are included so the behaviour of each dialog option is unambiguous.

### 9.1 Descriptives and the shape of a variable

For a variable \(x\) with \(n\) valid cases:

- mean \(\bar{x} = \frac{1}{n}\sum x_i\)
- variance \(s^2 = \frac{1}{n-1}\sum (x_i-\bar{x})^2\), standard deviation \(s=\sqrt{s^2}\)
- standard error of the mean \(SE = s/\sqrt{n}\)
- skewness \(g_1 = \frac{m_3}{m_2^{3/2}}\), kurtosis \(g_2 = \frac{m_4}{m_2^{2}} - 3\), with \(m_k=\frac{1}{n}\sum(x_i-\bar{x})^k\)
- range, minimum, maximum, sum
- percentiles by the selected definition (weighted average, empirical distribution, or nearest rank)

`DESCRIPTIVES` reports these; `FREQUENCIES` adds counts and cumulative percentages; `EXAMINE` adds robust location (trimmed mean, M-estimators), normality tests, and plots. **Why the distinctions matter:** the mean and SD are only meaningful for roughly symmetric interval data; skewness/kurtosis and the Explore plots reveal when a median or a transformation is more appropriate.

### 9.2 t-tests

- **One-sample:** \(t = \frac{\bar{x}-\mu_0}{s/\sqrt{n}}\), \(df=n-1\). The CI is \(\bar{x} \pm t_{1-\alpha/2,df}\,SE\).
- **Independent samples:** \(t = \frac{\bar{x}_1-\bar{x}_2}{\sqrt{s_1^2/n_1 + s_2^2/n_2}}\). Levene's test decides whether the pooled-variance or the Welch/Satterthwaite version is reported. Effect size: Cohen's \(d = \frac{\bar{x}_1-\bar{x}_2}{s_p}\).
- **Paired samples:** compute \(d_i=x_{i1}-x_{i2}\), then \(t=\frac{\bar{d}}{s_d/\sqrt{n}}\), \(df=n-1\). The procedure also reports the correlation between the paired variables, because a high correlation is what makes the paired design powerful.

**Assumptions:** independence of observations; approximate normality of the sampling distribution (robust for large \(n\)); for the pooled test, equal variances. **Why the dialog offers Levene and both t rows:** the equal-variance assumption is often false, and choosing the wrong row is one of the most common applied errors.

### 9.3 One-way ANOVA and the GLM family

One-way ANOVA partitions the total sum of squares:

\[
SS_{total}=\sum_i\sum_j (x_{ij}-\bar{x})^2,\quad
SS_{between}=\sum_i n_i(\bar{x}_i-\bar{x})^2,\quad
SS_{within}=SS_{total}-SS_{between}
\]

with \(F=\frac{SS_{between}/(k-1)}{SS_{within}/(N-k)}\). Post hoc tests (Tukey, Bonferroni, Scheffé, Dunnett, ...) control the family-wise error rate; contrasts test planned comparisons.

The GLM generalises this to:

\[
\mathbf{y} = \mathbf{X}\boldsymbol{\beta} + \boldsymbol{\varepsilon},\qquad
\boldsymbol{\varepsilon}\sim N(0,\sigma^2\mathbf{I})
\]

with factors coded as design columns, covariates as continuous predictors, and sums of squares of Type I–IV depending on the hypothesis. Repeated measures add a within-subject design matrix and a covariance structure; sphericity (Mauchly) determines whether the univariate F is valid or a Greenhouse-Geisser/Huynh-Feldt correction is needed.

**Why the options exist:** the same model can be parameterised in many ways; Type III SS, contrast coding, and estimated marginal means determine what \"the effect\" means. Reporting them explicitly prevents the \"different StatsBench output for the same data\" confusion.

### 9.4 Correlation and regression

- Pearson \(r=\frac{\sum(x_i-\bar{x})(y_i-\bar{y})}{\sqrt{\sum(x_i-\bar{x})^2\sum(y_i-\bar{y})^2}}\).
- Spearman is Pearson on ranks; Kendall's tau-b is based on concordant/discordant pairs and handles ties.
- Partial correlation removes the linear effect of control variables from both \(x\) and \(y\) before correlating the residuals.

OLS regression estimates \(\hat{\boldsymbol{\beta}}=(\mathbf{X}'\mathbf{X})^{-1}\mathbf{X}'\mathbf{y}\), with \(R^2=1-\frac{SS_{res}}{SS_{tot}}\) and adjusted \(R^2=1-(1-R^2)\frac{n-1}{n-p-1}\). The coefficient covariance is \(\hat{\sigma}^2(\mathbf{X}'\mathbf{X})^{-1}\), giving standard errors, t tests, and CIs. Collinearity is diagnosed by VIF \(=1/(1-R_j^2)\) and tolerance. Durbin-Watson detects first-order autocorrelation. Casewise diagnostics flag standardised residuals beyond \(\pm3\) and influence measures (Cook's distance, leverage, DFBETAS).

**Why the diagnostics are part of the dialog:** an OLS fit is easy; knowing whether its assumptions hold is the actual work. StatsBench puts the diagnostics in the same dialog so they are not forgotten.

### 9.5 Logistic, ordinal, and loglinear models

- **Binary logistic:** \(\log\frac{p}{1-p}=\beta_0+\sum\beta_j x_j\); odds ratio \(OR_j=e^{\beta_j}\). Estimation is by maximum likelihood; the likelihood-ratio chi-square compares the model with the intercept-only model. Hosmer-Lemeshow groups predicted probabilities and compares observed with expected counts. The classification table uses a cutoff (default 0.5).
- **Multinomial:** \(\log\frac{P(Y=k)}{P(Y=K)}=\beta_{0k}+\sum\beta_{jk}x_j\) for \(k=1,\dots,K-1\).
- **Ordinal (PLUM):** cumulative logits \(\log\frac{P(Y\le k)}{P(Y>k)}=\theta_k-\boldsymbol{\beta}'\mathbf{x}\), with a parallel-lines assumption (one \(\boldsymbol{\beta}\) across cutpoints). The test of parallel lines checks it.
- **Loglinear:** models expected cell counts in a contingency table: \(\log \mu_{ijk}=\lambda+\lambda_i^A+\lambda_j^B+\lambda_k^C+\lambda_{ij}^{AB}+\dots\); hierarchical models set higher-order terms to zero when lower-order terms are absent.

**Why:** these models respect the measurement scale of the outcome. Treating a 0/1 outcome or an ordered rating as continuous produces impossible predictions and invalid tests.

### 9.6 Factor analysis and dimension reduction

- **Principal components:** eigen-decomposition of the correlation (or covariance) matrix; component \(k\) explains \(\lambda_k/\sum\lambda\) of the variance.
- **Common factor:** \(\mathbf{\Sigma}=\mathbf{\Lambda}\mathbf{\Phi}\mathbf{\Lambda}'+\mathbf{\Psi}\), estimated by principal axis, maximum likelihood, or alpha factoring; rotated by varimax (orthogonal) or promax/oblimin (oblique).
- **KMO** measures sampling adequacy; **Bartlett's test** tests whether the correlation matrix is an identity matrix.
- **Correspondence analysis:** singular-value decomposition of the standardised residual matrix of a contingency table.
- **CATPCA:** alternates optimal scaling of categorical variables with principal-components extraction.

**Why rotation and KMO matter:** unrotated factors are hard to interpret, and factor analysis on data that are not factorable produces spurious dimensions. The diagnostics are the guard rails.

### 9.7 Clustering and classification

- **K-means:** minimise \(W=\sum_{c}\sum_{x\in c}\|x-\mu_c\|^2\) by alternating assignment and centroid updates.
- **Hierarchical:** merge the closest clusters according to a linkage rule (single, complete, average, Ward, centroid, median); the dendrogram records the merge distances.
- **TwoStep:** pre-clusters cases into sub-clusters, then merges them using a likelihood/BIC criterion; handles categorical and continuous variables.
- **Discriminant:** finds linear combinations \(\mathbf{a}'\mathbf{x}\) that maximise between-group relative to within-group variance.
- **Trees:** recursive partitioning that chooses splits maximising impurity reduction (CHAID uses chi-square, CRT uses Gini/variance, QUEST uses quadratic discriminant tests).

**Why validation is offered:** clustering always returns clusters, and trees always return splits. Silhouettes, classification tables, and cross-validation are what distinguish structure from noise.

### 9.8 Reliability

Cronbach's alpha for \(k\) items:

\[
\alpha=\frac{k}{k-1}\left(1-\frac{\sum_{i=1}^k \sigma_i^2}{\sigma_{total}^2}\right)
\]

Split-half reliability correlates two halves and applies the Spearman-Brown correction. Intraclass correlation generalises agreement to multiple raters. **Why item-total statistics are shown:** an item with a low item-total correlation or a large alpha-if-deleted increase is a candidate for removal, which is the practical use of the coefficient.

### 9.9 Nonparametric tests

- **Mann-Whitney U:** rank all observations, sum ranks per group; \(U\) counts pairwise wins.
- **Wilcoxon signed-rank:** rank the absolute paired differences, then sum the positive and negative ranks.
- **Kruskal-Wallis:** rank-based one-way ANOVA; \(H=\frac{12}{N(N+1)}\sum n_i\bar{R}_i^2-3(N+1)\).
- **Friedman:** rank within blocks; tests whether treatments differ across repeated measures.
- **Chi-square goodness-of-fit:** \(\chi^2=\sum (O-E)^2/E\).
- **McNemar:** compares paired proportions using the off-diagonal cells.

**Why:** these tests replace distributional assumptions with rank or permutation logic. They are the correct choice for ordinal outcomes and small samples, at the cost of some power when the parametric assumptions do hold.

### 9.10 Survival analysis

- **Kaplan-Meier:** \(S(t)=\prod_{t_i\le t}\left(1-\frac{d_i}{n_i}\right)\), where \(d_i\) is the number of events and \(n_i\) the number at risk at time \(t_i\). The log-rank test compares observed and expected events across groups.
- **Cox model:** \(h(t|\mathbf{x})=h_0(t)\exp(\boldsymbol{\beta}'\mathbf{x})\). The baseline hazard \(h_0\) is unspecified; estimation is by partial likelihood, so the model targets hazard ratios without assuming a distribution.
- **Parametric AFT:** \(\log T=\boldsymbol{\beta}'\mathbf{x}+\sigma\varepsilon\) with a chosen distribution for \(\varepsilon\) (extreme value → Weibull, normal → lognormal, logistic → log-logistic).

**Why censoring matters:** a subject who has not had the event by the end of follow-up contributes information about survival up to that time. Survival procedures use that partial information; ordinary regression would either drop the case (bias) or treat it as an event (bias in the other direction).

### 9.11 Time series

- **ARIMA(p,d,q):** \(\phi(B)(1-B)^d y_t=\theta(B)\varepsilon_t\), where \(B\) is the backshift operator.
- **Exponential smoothing:** weighted averages with level, trend, and seasonal components; Holt adds trend, Winters adds seasonality.
- **Seasonal decomposition:** \(y_t=T_t+S_t+E_t\) (additive) or \(y_t=T_t\cdot S_t\cdot E_t\) (multiplicative).
- **ACF/PACF:** autocorrelation and partial autocorrelation identify the orders p and q.

**Why stationarity tests and ACF plots are in the dialog:** ARIMA requires a stationary series; differencing (d) and the order selection are the modelling decisions, not afterthoughts.

### 9.12 Missing data and multiple imputation

- **Little's MCAR test** compares the observed means across missingness patterns; a non-significant result is consistent with MCAR, not proof of it.
- **EM** alternates expected sufficient statistics and maximum-likelihood parameter updates under a multivariate normal model.
- **MICE/FCS:** for each variable with missingness, regress it on the other variables (with an appropriate model), draw a value from the predictive distribution, and iterate. Repeat \(m\) times to create \(m\) completed datasets.
- **Pooling (Rubin's rules):** \(\bar{Q}=\frac{1}{m}\sum Q_j\), \(T=\bar{U}+(1+1/m)B\), where \(U\) is the within-imputation variance and \(B\) the between-imputation variance.

**Why multiple imputation is offered next to MVA:** MVA diagnoses the mechanism; MI provides valid inference under MAR. Single imputation (Replace Missing Values) is convenient but understates uncertainty, which is why it is a separate Transform tool.

### 9.13 Complex samples

For a sampling design with weights \(w_i\), strata \(h\), and clusters \(a\):

- weighted estimate \(\hat{\theta}=\sum w_i y_i / \sum w_i\),
- variance estimated by linearisation or jackknife, accounting for stratification and clustering,
- design effect \(deff = V_{design}/V_{SRS}\),
- effective sample size \(n_{eff}=n/deff\).

**Why the design must be declared:** the naive variance assumes independent simple-random observations. Clustered samples are less informative than their nominal \(n\), and ignoring the design produces confidence intervals that are too narrow.

### 9.14 Bayesian procedures

Bayes' theorem gives the posterior:

\[
p(\theta\mid y)\propto p(y\mid\theta)\,p(\theta)
\]

The procedures use MCMC to sample from the posterior, then report posterior means/medians, credible intervals, and Bayes factors. **Why MCMC:** closed-form posteriors exist only for conjugate models; sampling handles the general case. The diagnostic plots (trace, autocorrelation, posterior density) are how the analyst checks that the sampler converged.

### 9.15 Procedure-to-artefact map

| Procedure family | Command | Engine artefact | Primary output objects |
|---|---|---|---|
| Frequencies | `FREQUENCIES` | `statswfre.dll` | Statistics, frequency table, charts |
| Descriptives | `DESCRIPTIVES` | procedure module | Descriptive statistics, z-scores |
| Crosstabs | `CROSSTABS` | procedure module | Crosstab, chi-square, association measures |
| t tests | `T-TEST` | procedure module | Group statistics, test table, CI |
| ANOVA | `ONEWAY` | procedure module | ANOVA table, post hoc, means plot |
| GLM | `GLM`/`UNIANOVA` | `statswglm.dll` | Tests of effects, EMMs, contrasts |
| Mixed | `MIXED` | `statswmix.dll` | Fixed/random effects, covariance parameters |
| GENLIN/GEE | `GENLIN` | `statswgen.dll` | Parameter estimates, model fit |
| Regression | `REGRESSION` | `statswreg.dll` | Model summary, ANOVA, coefficients, diagnostics |
| Logistic | `LOGISTIC REGRESSION` | procedure module | Omnibus tests, classification, ORs |
| Factor | `FACTOR` | `statswfac.dll` | Communalities, variance explained, rotated matrix |
| Reliability | `RELIABILITY` | `statswrel.dll` | Alpha, item-total statistics |
| Cluster | `CLUSTER`/`QUICK CLUSTER` | `statswclu.dll` | Cluster memberships, dendrogram, centres |
| Survival | `KM`/`COXREG` | `statswkm.dll`/`statswcox.dll` | Survival table, log-rank, hazard ratios |
| Missing data | `MVA`/`MULTIPLE IMPUTATION` | `statswmva.dll`/`statswmim.dll` | Patterns, imputed datasets, pooled tests |
| Complex samples | `CS*` | `statswcs*.dll` | Design-adjusted estimates and tests |
| Graphics | `GGRAPH`/`GRAPH` | `statswggr.dll` | Charts |
| Output control | `OMS`/`OUTPUT EXPORT` | `statswoms.dll`/`statswout.dll` | Routed tables, exported files |

The DLL names are the observed module names; the mapping to commands is by naming convention **(inferred)** and by the command families in the XML catalog.

---

## 10. Data I/O: getting data in and out

### 10.1 The native format: `.sav`

`.sav` is not a flat table. It stores:

- the case data (numeric, string, and date values),
- the variable dictionary (names, types, widths, decimals, labels, value labels, missing rules, measurement level, role, custom attributes),
- file-level attributes and documents,
- optional compression (`.zsav` is the compressed variant).

The native reader/writer is `savio64.dll` (1.3 MB) [E5]. Because the dictionary travels with the data, a `.sav` opened on another machine keeps its labels, missing codes, and measurement levels. That is why `accidents.sav` displayed `Age category` and `Under 21` without any extra setup [E14].

**Why:** reproducible analysis requires the metadata to be part of the data file, not a separate codebook that can be lost.

### 10.2 Text and CSV

The Text Import Wizard and `GET DATA` read delimited or fixed-width text. The wizard lets the analyst:

- choose a delimiter (comma, tab, semicolon, space, other),
- set the first row as variable names,
- define variable ranges and types,
- preview the resulting structure,
- save the import specification for reuse.

Command form: `GET DATA /TYPE=TXT /FILE=... /DELIMITERS=... /QUALIFIER=... /ARRANGEMENT=... /VARIABLES=...`. **Why a wizard:** text files have no schema; the import step is where types, decimals, and missing codes are decided, and mistakes there poison every downstream analysis.

### 10.3 Excel and other spreadsheets

Excel files are read through the Excel data-access layer (`savxls.dll`, `refexcel.jar`) and the `GET DATA /TYPE=XLS/XLSX` command. The dialog lets the analyst pick a sheet, a range, and whether the first row contains names. **Why:** Excel is where most real-world data live; direct import avoids a lossy CSV round-trip.

### 10.4 Databases

ODBC access (`savodbc.dll`) supports `GET DATA /TYPE=ODBC` and the Database Wizard: choose a data source, write or build a query, preview, and import. **Why:** enterprise data usually lives in a database; pulling a query result into StatsBench is more reliable than exporting to CSV first.

### 10.5 SAS and Stata

- `GET SAS` reads SAS transport files (`.xpt`) through `statswsas.dll`.
- `GET STATA` reads Stata `.dta` files.
- `SAVE TRANSLATE` writes SAS, Stata, Excel, CSV, and tab-delimited files.

**Why:** collaboration across statistical packages is common; native readers preserve labels and missing values better than flat exports.

### 10.6 Output formats

`OUTPUT EXPORT` supports `DOC`, `HTML`, `PDF`, `PPT`, `TEXT`, `XLS`, `XLSX`, `XLSM`, `BMP`, `EMF`, `EPS`, `JPG`, `PNG`, and `TIF` [E6]. Tables can also be copied from the Viewer into Word/Excel/PowerPoint as editable objects. **Why:** the analysis is rarely the final artefact; it has to become a report, a slide deck, or a data feed.

### 10.7 Project, plan, and model files

| Extension | Content | Producer |
|---|---|---|
| `.spv` | Output document (ZIP container of XML parts and binary table-data chunks) | Viewer |
| `.spj` | Production job (commands, files, output destinations) | Production Facility |
| `.csplan` | Complex-samples sampling plan | Complex Samples |
| `.csaplan` | Complex-samples analysis plan | Complex Samples |
| `.viztemplate` | Chart template | Chart Builder / Graphboard |
| `.xml` (PMML) | Predictive model | Scoring Wizard / model export |
| `.spd` | Custom dialog definition | Custom Dialog Builder |
| `.spe` | Extension specification | Extension bundle |

**Why:** each artefact captures a decision that must be repeatable: how the sample was drawn, how the chart looks, how the model scores, how the dialog behaves. Storing them as files makes the workflow portable.

### 10.8 Unicode and code pages

The installation uses code page 65001 (UTF-8), and the status bar reports `Unicode: ON` [E1], [E13]. `SAVE CODEPAGE` and the import dialogs can transcode legacy files. **Why:** labels and string data in non-Latin scripts must survive the round trip; Unicode mode is the default in modern StatsBench.

---

## 11. Automation and extensibility

StatsBench has five automation layers, from least to most programmatic:

1. **Syntax** — the command language; every dialog can paste it.
2. **Scripts** — Basic or Python scripts that drive the client (`Run Script...`).
3. **Programmability blocks** — `BEGIN PROGRAM` for Python/R inside a syntax stream.
4. **Extension bundles** — new commands with dialogs, implemented in R or Python.
5. **Server/production** — `statisticsd` and the Production Facility for unattended runs.

### 11.1 The Output Management System (OMS)

OMS intercepts output objects as they are produced and routes them:

```
OMS /SELECT TABLES /IF COMMANDS=['Frequencies'] SUBTYPES=['Frequencies']
    /DESTINATION FORMAT=SAV OUTFILE='freq.sav'.
FREQUENCIES VARIABLES=agecat.
OMSEND.
```

- **SELECT** chooses by command, subtype, or object identifier (the `OMS Identifiers...` utility lists them).
- **DESTINATION** can be a file (HTML, XML, SAV, StatsBench data), a dataset, or the Viewer.
- **COLUMNS/LABELS** reshape the captured table.
- **OMSEND** closes the capture.

**Why:** OMS turns output into data. A report can be generated, a table can feed a dashboard, or a set of results can be accumulated across models without manual copying.

### 11.2 Scripting and programmability

- **`BEGIN PROGRAM Python3.` / `END PROGRAM.`** runs Python with the StatsBench Python API (`ref` module): read/write datasets, run commands, manipulate output.
- **`BEGIN PROGRAM R.` / `END PROGRAM.`** does the same for R.
- **Basic scripts** (`.sbs`) drive the client UI: open dialogs, set fields, run commands.
- **`StatsBenchINC TRANS`** applies an R function to StatsBench variables as a transformation.

The bundled Python is 3.8.2 [E12]. **Why:** the built-in procedures cannot cover every method; programmability lets analysts add a method in the language they know, with the data and output plumbing already handled.

### 11.3 Extension bundles in detail

A bundle is a directory containing:

```
<BUNDLE>/
  <name>.R            implementation (or Python)
  <name>.spd          custom dialog (ZIP of XML/resources)
  <name>.xml          command syntax definition
  <name>.spe          extension specification
  Readme.md           description and licence
```

Installation is through **Extensions > Extension Hub** or **Install Local Extension Bundle**. Once installed, the command appears in the syntax editor and the dialog appears in the menu. **Why this is powerful:** a statistician can ship a new method with a native-looking dialog and syntax, without rebuilding the product.

The observed bundles include both the original StatsBench 27 features (`STATS_DISTFIT`, `STATS_RELIMP`, `STATS_CANCORR`, `STATS_CLUS_SIL`, `STATS_NONPAR_REGR`, `STATS_PSM`, `STATS_SURVREG`, `STATS_WEIBULL_PLOT`, `STATS_VALLBLS_FROMDATA`, `STATS_ADJUST_WIDTHS`, `STATS_SPLIT_DATASET`) and R packages used by them (`ggplot2`, `dplyr`, `tidyr`, `rstatix`, `ggpubr`, `ggcorrplot`) [E11].

### 11.4 PMML scoring

Models can be exported as PMML (Predictive Model Markup Language) and applied to new data with the **Scoring Wizard** and `MODEL HANDLE`. The install ships PMML schemas 2.1–4.3 [E10]. **Why:** a model built once should be reusable in production without the analyst re-running the original procedure.

### 11.5 Server mode and production

`statisticsd.conf` defines a server on port 3027 with TLS, authentication, thread limits, and data-access policy [E4]. The **Production Facility** runs `.spj` jobs unattended, with logging and scheduled execution. **Why:** monthly reports, scoring runs, and large analyses should not depend on a person clicking dialogs.

---

## 12. Why StatsBench is designed this way

### 12.1 Reproducibility over convenience

Every dialog can paste syntax; every run is logged; every dataset carries its dictionary. The design assumes that an analysis will be repeated, reviewed, or challenged. This is why the product has survived in regulated and academic environments: the GUI is a generator for an auditable command stream, not a replacement for it.

### 12.2 Metadata as the contract

Measurement level, value labels, missing rules, and roles are not documentation niceties. They determine which procedures run, how categories are treated, which charts are valid, and what the output says. The Chart Builder's measurement-level advisory [E19] is the clearest expression of this philosophy: fix the dictionary before drawing conclusions.

### 12.3 Separation of concerns

The client, engine, procedure modules, data-access layer, and output layer are separate. That separation enables:

- local and server deployment from the same engine,
- long-running jobs that survive a UI restart,
- multiple front-ends (Desktop, Server, batch),
- per-module licensing,
- extensions that add commands without touching the core.

### 12.4 Backward compatibility

The product still runs `GRAPH`, `HILOGLINEAR`, `MANOVA`, and the legacy dialogs alongside `GGRAPH`, `GENLOG`, and Chart Builder. The 328-command catalog includes both old and new commands. **Why:** decades of saved syntax and published analyses depend on the old commands continuing to work. New features are added beside the old ones, not on top of them.

### 12.5 Editable, structured output

Pivot tables and the `.spv` schema make output a first-class object. Analysts can reshape, restyle, and export results; OMS can capture them programmatically. **Why:** a static screenshot cannot be audited, reformatted, or fed into another system.

### 12.6 Trade-offs and criticisms visible in the product

- **Menu sprawl.** Analyze has 27 entries and Regression alone has 17. The breadth is powerful but hard to navigate for occasional users.
- **Extension dependence.** Several StatsBench 27 \"new features\" are R bundles; they require a working R configuration and add moving parts outside the core installer.
- **Dialog complexity.** A single procedure can have six option sub-dialogs (Statistics, Plots, Save, Options, Style, Bootstrap). The defaults are sensible, but the surface is large.
- **Two charting generations.** Legacy Dialogs and Chart Builder coexist, which is good for compatibility but confusing for learners.
- **Startup diagnostics.** In this environment the product showed an unexpected-error report and a session-restore dialog before the Data Editor was usable [E20], [E21]. The recovery features work, but the first-run experience is not clean.
- **Performance.** Large files benefit from **Cache Data** and server mode; the desktop client is not designed for massive data in the way a columnar database is.

These are design consequences, not bugs: the product optimises for breadth, auditability, and compatibility rather than for a minimal, opinionated workflow.

---

## 13. Verification log

The following was actually performed during this analysis (not inferred):

1. Located and inventoried the installation at `C:\Program Files\StatsBench\Statistics\27` (binaries, 132 `statsw` DLLs, 142 JARs, 328 command XMLs, 50 templates, samples, schemas) [E1]–[E12].
2. Launched the application; observed and dismissed the **Error Reporting Service** dialog and the **Session Management** restore-point dialog; confirmed the Data Editor was usable [E20], [E21].
3. Captured the top-level menus (File, Edit, View, Data, Transform, Analyze, Graphs, Utilities, Extensions, Window, Help) and the Analyze/Data/Graphs/Transform submenus [E22], [E24], [E25].
4. Captured Variable View and Data View, including the 11 metadata columns and value-label display [E13], [E14].
5. Opened `Samples\English\accidents.sav` through **File > Open > Data** (`Ctrl+O`), confirming the dictionary and the dataset title `accidents.sav [DataSet1]` [E14].
6. Opened **Analyze > Descriptive Statistics > Frequencies**, captured the dialog, moved `agecat` into `Variable(s)`, and confirmed `OK`/`Paste` enabled only after a valid selection [E15].
7. Pressed **Paste**, captured the generated syntax (`DATASET ACTIVATE DataSet1.` + `FREQUENCIES VARIABLES=agecat /ORDER=ANALYSIS.`) [E16].
8. Ran the syntax (`Ctrl+A`, `Ctrl+R`) and captured the Viewer: Log with the executed commands, Statistics table (N Valid 6, Missing 0), and the `Age category` frequency table [E17].
9. Opened **Analyze > Regression > Linear**, captured the full dialog (Dependent, Independent(s), Method, Blocks, Statistics/Plots/Save/Options/Style/Bootstrap) [E18].
10. Opened **Graphs > Chart Builder**, captured the measurement-level advisory and the builder itself (Gallery, Basic Elements, Groups/Point ID, Titles/Footnotes; Element Properties/Chart Appearance/Options) [E19].
11. Read the command descriptors for `FREQUENCIES`, `DESCRIPTIVES`, `CROSSTABS`, `REGRESSION`, `T-TEST`, `ONEWAY`, `GLM`, `MIXED`, `GENLIN`, `CLUSTER`, `FACTOR`, `RELIABILITY`, `NPAR TESTS`, `EXAMINE`, `MEANS`, `CORRELATIONS`, `LOGISTIC REGRESSION`, `DISCRIMINANT`, `QUICK CLUSTER`, `OMS`, `OUTPUT EXPORT`, `GRAPH`, `GGRAPH`, and `ADP`, extracting their subcommand lists [E6].
12. Inventoried the extension bundles and confirmed the `.R`/`.spd`/`.spe`/`.xml` anatomy and the Apache 2.0 Readme [E11].
13. Verified the bundled Python version (3.8.2) and the JVM/server configuration [E3], [E4], [E12].

---

## 14. Limitations of this analysis

- **No source code.** Statements about internal control flow are inferences from module names, command descriptors, schemas, and observable behaviour; they are marked **(inferred)** where they go beyond direct evidence.
- **One end-to-end run.** The dialog → syntax → output pipeline was exercised with Frequencies on `accidents.sav`. Other procedures were documented from their dialogs and command XML, not each run to completion.
- **Parameter coverage.** The 328 command descriptors contain far more parameters than can be listed here; the document gives representative subcommands and points to `syntax_xml\<COMMAND>.xml` for the authoritative definition.
- **UI state is environment-specific.** Some items were greyed out because of the current state (e.g. Multiple Response Frequencies before a set is defined; Regression Residual Plot; Calculate with Pivot Table). Greyed state is reported as observed, not as a permanent property.
- **File submenus.** File > New/Open/Import/Export submenu contents were not individually screenshotted; their descriptions rely on the command catalog and standard document types. The top-level File items are captured evidence.
- **Duplicate menu entry.** The Data menu showed `Propensity Score Matching...` twice in the captured session. It is recorded as observed without attributing a cause.
- **Module-to-command mapping.** The DLL-to-command table uses naming conventions; the exact loader mapping is not exposed in the install.
- **Startup error.** The unexpected-error dialog may be specific to this machine/session; it is documented as an observed behaviour, not as a universal StatsBench 27 defect.

---

## Appendix A — Evidence register

All paths are on the analysed machine. \"Capture\" means a PNG screenshot saved during the live session; the capture directory is `C:\Users\Saurav\AppData\Local\Temp\opencode`.

| ID | Source | What it establishes |
|---|---|---|
| **E1** | `C:\Program Files\StatsBench\Statistics\27\refprod.inf` | ProductName `StatsBench 27`; Version `27.0.1.0`; VersionMajor 27; VersionMinor 0; FixPack 0; VersionPatch 1; FeatureCode 1200; CodePage 65001; RegistryRoot `StatsBench\27` |
| **E2** | `...\productnames.ini` | Feature codes and module names: Base, Tables, Regression Models, Advanced Models, Exact Tests, Categories, Missing Values, Custom Tables, Complex Samples, Classification Trees, Data Validation, Programmability, Advanced Visualization, Trends |
| **E3** | `...\jvmcfg.ini` | J9 JVM (`JRE\bin\j9vm\jvm.dll`); `-Xms256M -Xmx2048M`; `-Xcompressedrefs`; `-Xshareclasses`; default TLS 1.2; `application.home`/`statistics.home` |
| **E4** | `...\statisticsd.conf` (155 lines) | Server daemon: listen port 3027; `win32` authentication; 4 user / 4 worker threads; `ssl` value 0 (SSL disabled; 2=TLS 1.0, 3=TLS 1.1, 4=TLS 1.2); client data access FULL/NONE; reconnect timeout 100 min; COP port 80; GSK keystore settings |
| **E5** | Install directory inventory | 132 `statsw*.dll`; `statsstat.dll` 5.0 MB; `statsengine.exe` 86 KB; `stats.exe` 359 KB; `savio64.dll` 1.3 MB; `statswout.dll` 678 KB; `statswoms.dll` 203 KB; 142 JARs incl. `StatsBenchClientUI.jar` 16.5 MB and `StatsBenchClientUIRes.jar` 15.0 MB; format libraries `savodbc.dll`, `savxls.dll`, `statswsas.dll`, `savjson.dll` |
| **E6** | `...\syntax_xml\` (328 XML files) | Authoritative command definitions. Examples inspected: `FREQUENCIES` (FORMAT, MISSING, BARCHART, PIECHART, HISTOGRAM, GROUPED, NTILES, PERCENTILES, STATISTICS, ORDER); `DESCRIPTIVES` (MISSING, SAVE, STATISTICS, SORT); `CROSSTABS` (TABLES, CELLS, STATISTICS, METHOD, COUNT, FORMAT, BARCHART, HIDESMALLCOUNTS, SHOWDIM); `REGRESSION` (VARIABLES, DEPENDENT, METHOD, STATISTICS, CRITERIA, RESIDUALS, CASEWISE, SCATTERPLOT, PARTIALPLOT, SAVE, OUTFILE, TEMPLATE); `GLM` (WSFACTOR, MEASURE, WSDESIGN, RANDOM, METHOD, CRITERIA, PRINT, PLOT, TEST, LMATRIX, MMATRIX, KMATRIX, POSTHOC, EMMEANS, ROBUST, SAVE, OUTFILE, DESIGN); `GENLIN` (MODEL, CRITERIA, REPEATED, EMMEANS, PRINT, SAVE, OUTFILE); `MIXED`; `ADP` (FIELDS with TARGET, INPUT, ANALYSISWEIGHT); `OMS`; `OUTPUT EXPORT` (DOC, HTML, PDF, PPT, TEXT, XLS, XLSX, XLSM, BMP, EMF, EPS, JPG, PNG, TIF). copyright headers dated 1989–2011+ |
| **E7** | `...\Samples\English\` | 1,328 `.sav`; 209 `.sps`; 22 `.spv`; 55 `.csplan`; 33 `.csaplan`. Used: `accidents.sav` (agecat/gender/accid/pop). Also `automaton.sps`, `validatedata_stroke.sps`, `detectanomaly_stroke.sps`, `conjoint.sps`, `prefscal_*.sps`, `Ridge regression.sps`, `scoring*.sps` |
| **E8** | `...\template\` | 50 `.viztemplate` chart templates, including Bayesian posterior plots (`bayes.viztemplate`, `BYSPosteriorPlot.viztemplate`) |
| **E9** | `...\helpids.csv` | 8,992 rows mapping `help_id` to `ref/base/...` help topics |
| **E10** | `...\output-1.8.xsd` (and 1.4–1.7), `dictionary-1.0/1.1.xsd`, `oms-log-1.1/1.2.xsd`, PMML 2.1–4.3 | Output document model: `outputTree` with `command`, `pageTitle`, `textBlock`, `heading`, `pivotTable`, `chart`, `treeView`, `model`; OMS log and dictionary schemas |
| **E11** | `C:\ProgramData\StatsBench\Statistics\27\extensions\` and `%LOCALAPPDATA%\StatsBench\Statistics\27\extensions\` | Installed bundles: `STATS_DISTFIT`, `STATS_RELIMP`, `STATS_CANCORR`, `STATS_CLUS_SIL`, `STATS_NONPAR_REGR`, `STATS_PSM`, `STATS_SURVREG`, `STATS_WEIBULL_PLOT`, `STATS_VALLBLS_FROMDATA`, `STATS_ADJUST_WIDTHS`, `STATS_SPLIT_DATASET`, `STATS_GET_R`, `STATS_COXREGR`, `STATS_REGRESS_PLOT`, `STATS_SUBGROUP_PLOTS`, `EXT_TURF`, `EXT_RAKE`, `EXT_CREATE_DUMMIES`, `EXT_TRANS`, `ColPropTest`, `RRegRidge`, `PLS`, `PSM`, `Correlation_Plot`, `Dynamite_Plot`, `Fluctuation_Plot`, `RBoxplotFamily`, `String_2_Nominal`, `Custom_Desriptive_Tables`, `Variable_Tables`, plus R libraries (`ggplot2`, `dplyr`, `tidyr`, `rstatix`, `ggpubr`, `ggcorrplot`, `stringr`). Bundle anatomy: `.R` implementation, `.spd` dialog (ZIP), `.xml` command definition, `.spe` extension spec, `Readme.md` (Apache 2.0). `STATS_DISTFIT` declares `STATS DISTFIT` with VARIABLE, DISTRIBUTION, SHAPE1, SHAPE2, DF1, DF2, DF, OPTIONS SIMULATEP/MINVALUE/MAXVALUE/QQPLOT |
| **E12** | `...\Python3\python.exe --version`; `savdxcfg.ini`; extension R config | Bundled Python 3.8.2; Python/R integration configured; `R 3.6 Configuration` bundle present |
| **E13** | Capture `ref_data.png` / `ref_de2.png` | Data Editor; Variable View columns Name, Type, Width, Decimals, Label, Values, Missing, Columns, Align, Measure, Role; rows agecat/gender/accid/pop with labels and measurement levels; status bar \"StatsBench Processor is ready\" and \"Unicode: ON\"; title `Untitled1 [DataSet0]` |
| **E14** | Capture `ref_loaded.png`, `ref_dataview.png` | `accidents.sav [DataSet1]` with 4 variables; Data View displaying value labels (`Under 21`, `Female`); toolbar enabled after load |
| **E15** | Capture `ref_freq_dialog.png`, `ref_freq_selected.png` | Frequencies dialog anatomy; variable list with `label [name]`; `Statistics/Charts/Format/Style/Bootstrap`; `Display frequency tables`; `Create APA style tables`; `OK`/`Paste` disabled until valid |
| **E16** | Capture `ref_syntax_pasted.png` | Syntax Editor with `DATASET ACTIVATE DataSet1.` and `FREQUENCIES VARIABLES=agecat /ORDER=ANALYSIS.`; navigation pane; line numbers; syntax colouring; Active DataSet dropdown |
| **E17** | Capture `ref_output.png` | Viewer outline (Log, Frequencies > Title, Notes, Active Dataset, Statistics, Age category); executed log with `GET FILE`, `DATASET NAME`, `DATASET ACTIVATE`, `FREQUENCIES`; Statistics table (N Valid 6, Missing 0); frequency table with Frequency/Percent/Valid Percent/Cumulative Percent |
| **E18** | Capture `ref_regression_dialog.png` | Linear Regression dialog: Dependent, Independent(s), Block 1 of 1, Previous/Next, Method Enter, Selection Variable, Case Labels, WLS Weight, Statistics/Plots/Save/Options/Style/Bootstrap |
| **E19** | Captures `ref_chartbuilder.png`, `ref_chartbuilder2.png` | Chart Builder measurement-level advisory with OK / Define Variable Properties / Don't show again; builder with Gallery (Favorites, Bar, Line, Area, Pie/Polar, Scatter/Dot, Histogram, High-Low, Boxplot, Dual Axes), Basic Elements, Groups/Point ID, Titles/Footnotes, Element Properties/Chart Appearance/Options |
| **E20** | Capture `ref_after_dismiss.png` (Session Management) | Auto-recovery dialog: restore points by Type (Data Editor, Output, Syntax), File, Date/Time; Restore/Cancel/Delete; File menu's Open/Save Restore Point and Auto-Recovery toggle corroborate the feature |
| **E21** | Capture `ref_de2.png` (Error Reporting Service) | Unexpected-error dialog with More Information / Send / Cancel; dismissed to continue; separate from session recovery |
| **E22** | Captures under `...\refmenus2\` and `...\refsub2\` | Top-level menus File, Edit, View, Data, Transform, Analyze, Graphs, Utilities, Extensions, Window, Help, with item lists as documented in §8; Analyze submenus for Power, Reports, Descriptive, Bayesian, Tables, Compare Means, GLM, GenLin, Mixed, Correlate, Regression, Loglinear, Neural, Classify, Dimension Reduction, Scale, Nonparametric, Forecasting, Survival, Multiple Response, MVA, MI, Complex, Simulation, Quality, Spatial, Direct Marketing |
| **E23** | `...\Samples\English\automaton.sps` | Macro-style sample syntax: `CD @location.`, `GET FILE=@datafile.`, `FREQUENCIES VARIABLES=@varlist /BARCHART.` |
| **E24** | Captures under `...\refsub3\` | Data > Validation (Load Predefined Rules, Define Rules, Validate Data); Data > Merge Files (Add Cases, Add Variables); Data > Orthogonal Design (Generate, Display) |
| **E25** | Capture `...\refsub3\graph_legacy.png` | Graphs > Legacy Dialogs: Bar, 3-D Bar, Line, Area, Pie, High-Low, Boxplot, Error Bar, Population Pyramid, Scatter/Dot, Histogram |

**Note on captures:** screenshots are session-local artefacts; the permanent evidence is the shipped metadata (E1–E12) plus the reproducible commands in the document. The screenshots are cited to show the live UI behind the descriptions.

---

## Appendix B — Complete command catalog (328 commands)

The following names are the base names of the 328 XML files in `syntax_xml\` [E6]. Each file defines the command's subcommands, parameters, types, and enumerations.

```
2SLS; ACF; ADD DOCUMENT; ADD FILES; ADD VALUE LABELS; ADP; AGGREGATE; AIM; ALSCAL;
ALTER TYPE; ANACOR; ANOVA; APPLY DICTIONARY; AREG; ARIMA; AUTORECODE; BAYES ANOVA;
BAYES CORRELATION; BAYES INDEPENDENT; BAYES LOGLINEAR; BAYES ONESAMPLE; BAYES REGRESSION;
BAYES RELATED; BAYES REPEATED; BEGIN DATA; BEGIN EXPR; BEGIN GPL; BEGIN GRAPHSRV;
BEGIN PROGRAM; BOOTSTRAP; BREAK; CACHE; CASEPLOT; CASESTOVARS; CATPCA; CATREG; CCF; CD;
CLEAR TIME PROGRAM; CLEAR TRANSFORMATIONS; CLUSTER; CNLR; CODEBOOK; COMMENT;
COMPARE DATASETS; COMPUTE; CONJOINT; CORRELATIONS; CORRESPONDENCE; COUNT; COXREG; CREATE;
CROSSTABS; CSCOXREG; CSDESCRIPTIVES; CSGLM; CSLOGISTIC; CSORDINAL; CSPLAN ANALYSIS;
CSPLAN SAMPLE; CSPLAN VIEW; CSSELECT; CSTABULATE; CTABLES; CURVEFIT; DATA LIST;
DATAFILE ATTRIBUTE; DATASET ACTIVATE; DATASET CLOSE; DATASET COPY; DATASET DECLARE;
DATASET DISPLAY; DATASET NAME; DATE; DEFINE; DELETE VARIABLES; DERIVATIVES; DESCRIPTIVES;
DETECTANOMALY; DISCRIMINANT; DISPLAY; DMCLUSTER; DMLOGISTIC; DMROC; DMTABLES; DMTREE;
DO IF; DO REPEAT; DOCUMENT; DROP DOCUMENTS; ECHO; ELSE IF; ELSE; END CASE; END EXPR;
END FILE TYPE; END FILE; END IF; END INPUT PROGRAM; END LOOP; END MATRIX; END PROGRAM;
END REPEAT; ERASE; EXAMINE; EXECUTE; EXPORT; EXSMOOTH; EXTENSION; FACTOR; FILE HANDLE;
FILE LABEL; FILE TYPE; FILTER; FINISH; FIT; FLEISS MULTIRATER KAPPA; FLIP; FORMATS;
FREQUENCIES; GENLIN; GENLINMIXED; GENLOG; GET CAPTURE; GET DATA; GET SAS; GET STATA;
GET TRANSLATE; GET; GETCOGNOS; GETTM1; GGRAPH; GLM; GRAPH; HILOGLINEAR; HOMALS; HOST;
IF; IGRAPH; IMPORT; INCLUDE; INPUT PROGRAM; INSERT EXEC; INSERT HIDDEN; INSERT;
KEYED DATA LIST; KM; KNN; LEAVE; LINEAR; LIST; LOGISTIC REGRESSION; LOGLINEAR; LOOP;
MANOVA; MATCH FILES; MATRIX DATA; MATRIX; MCONVERT; MEANS; MISSING VALUES; MIXED; MLP;
MODEL CLOSE; MODEL HANDLE; MODEL LIST; MODEL NAME; MODEL PROGRAM; MRSETS; MULT RESPONSE;
MULTIPLE CORRESPONDENCE; MULTIPLE IMPUTATION; MVA; N OF CASES; NAIVEBAYES; NEW FILE;
NLR; NOMREG; NONPAR CORR; NPAR TESTS; NPTESTS; NUMERIC; OLAP CUBES; OMS; OMSEND;
OMSINFO; OMSLOG; ONEWAY; OPTIMAL BINNING; ORTHOPLAN; OUTPUT ACTIVATE; OUTPUT CLOSE;
OUTPUT DISPLAY; OUTPUT EXPORT; OUTPUT MODIFY; OUTPUT NAME; OUTPUT NEW; OUTPUT OPEN;
OUTPUT SAVE; OVERALS; PACF; PARTIAL CORR; PERMISSIONS; PLANCARDS; PLS; PLUM; POINT;
POWER MEANS INDEPENDENT; POWER MEANS ONESAMPLE; POWER MEANS RELATED; POWER ONEWAY ANOVA;
POWER PARTIALCORR; POWER PEARSON ONESAMPLE; POWER PROPORTIONS INDEPENDENT;
POWER PROPORTIONS ONESAMPLE; POWER PROPORTIONS RELATED; POWER SPEARMAN ONESAMPLE;
POWER UNIVARIATE LINEAR; PPLOT; PREDICT; PREFSCAL; PRESERVE; PRINCALS; PRINT EJECT;
PRINT FORMATS; PRINT SPACE; PRINT; PROBIT; PROCEDURE OUTPUT; PROPORTIONS; PROXIMITIES;
PROXSCAL; QUANTILE REGRESSION; QUICK CLUSTER; RANK; RATIO STATISTICS; RBF; READ MODEL;
RECODE; RECORD TYPE; REGRESSION; RELIABILITY; RENAME VARIABLES; REPEATING DATA; REPORT;
REPOSITORY ATTRIBUTES; REPOSITORY CONNECT; REPOSITORY COPY; REREAD; RESPONSE RATE;
RESTORE; RMV; ROC ANALYSIS; ROC; SAMPLE; SAVE CODEPAGE; SAVE DATA COLLECTION;
SAVE MODEL; SAVE TRANSLATE; SAVE; SAVETM1; SCRIPT; SEASON; SELECT IF; SELECTPRED;
SET; SHIFT VALUES; SHOW; SIMPLAN CREATE; SIMPLAN VIEW; SIMRUN; SORT CASES;
SORT VARIABLES; SPATIAL ASSOCIATION RULES; SPATIAL MAPSPEC;
SPATIAL TEMPORAL PREDICTION; SPCHART; SPECTRA; SPLIT FILE; STAR JOIN; STRING; SUBTITLE;
SUMMARIZE; SURVIVAL; SYSFILE INFO; TABLES; TCM ANALYSIS; TCM APPLY; TCM MODEL;
TDISPLAY; TEMPORARY; TIME PROGRAM; TITLE; TMS BEGIN; TMS END; TMS IMPORT; TMS MERGE;
TREE; TSAPPLY; TSET; TSHOW; TSMODEL; TSPLOT; T-TEST; TWOSTEP CLUSTER; UNIANOVA;
UPDATE; USE; VALIDATEDATA; VALUE LABELS; VARCOMP; VARIABLE ALIGNMENT; VARIABLE ATTRIBUTE;
VARIABLE LABELS; VARIABLE LEVEL; VARIABLE ROLE; VARIABLE WIDTH; VARSTOCASES; VECTOR;
VERIFY; VGRAPH; WEIGHT; WEIGHTED KAPPA; WLS; WRITE FORMATS; WRITE; XGRAPH; XSAVE
```

**Reading the catalog:**

- Commands with `CS` prefixes are Complex Samples.
- `DM*` commands are Direct Marketing.
- `BAYES *` are the Bayesian procedures.
- `POWER *` are the power-analysis procedures.
- `SPATIAL *` are spatial/temporal modelling.
- `TCM *` are temporal causal models; `TS*` are traditional time series.
- `BEGIN *` / `END *` are block constructs (data, program, GPL, matrix).
- `OUTPUT *` and `OMS*` are output management.
- `X*`/`V*`/`I*` `GRAPH` variants are legacy graphics back-ends.
- `MATRIX` is the matrix language; `DEFINE`/`INSERT`/`INCLUDE` are macro/programmability.

---

## Appendix C — Menu tree (as observed)

Items marked `▸` open a submenu. Items in *italics* were greyed out in the observed session. See [E22], [E24], [E25] for the captures.

### File
- New ▸
- Open ▸
- Import Data ▸
- Open Restore Point...
- Close
- Get R Workspace
- Save
- Save As...
- Save All Data
- Save Restore Point...
- Export ▸
- Mark File Read Only
- Revert to Saved File
- ✔ File Enabled for Auto-Recovery
- Collect Variable Information
- Rename Dataset...
- Display Data File Information ▸
- Cache Data...
- Stop Processor (Ctrl+Period)
- Switch Server...
- Repository ▸
- Print Preview
- Print... (Ctrl+P)
- Welcome Dialog...
- Recently Used Data ▸
- Recently Used Files ▸
- Exit

### Edit (Variable View state)
- Undo
- Redo
- Cut
- Copy
- Copy with Variable Names
- Copy with Variable Labels
- Paste
- Paste Variables...
- Paste with Variable Names
- Clear
- Insert Variable
- Insert Cases
- Search Data Files
- Search (Ctrl+Shift+S)
- Find... (Ctrl+F)
- Find Next (F3)
- Replace... (Ctrl+H)
- Go to Case...
- Go to Variable...
- Go to Imputation...
- Options...

### View (Variable View state)
- ✔ Status Bar
- Toolbars ▸
- Menu Editor...
- Fonts...
- ✔ Grid Lines
- ✔ Value Labels
- Mark Imputed Data
- Customize Variable View...
- Data (Ctrl+T)

### Data
- Define Variable Properties...
- Set Measurement Level for Unknown...
- Create Value Labels from Data
- Copy Data Properties...
- New Custom Attribute...
- Define date and time...
- Define Multiple Response Sets...
- Validation ▸
  - Load Predefined Rules...
  - Define Rules...
  - Validate Data...
- Identify Duplicate Cases...
- Identify Unusual Cases...
- Compare Datasets...
- Sort Cases...
- Sort Variables...
- Transpose...
- Adjust String Widths Across Files
- Merge Files ▸
  - Add Cases...
  - Add Variables...
- Restructure...
- Propensity Score Matching...
- Rake Weights...
- Propensity Score Matching... *(observed twice)*
- Case Control Matching...
- Aggregate...
- Orthogonal Design ▸
  - Generate...
  - Display...
- Split into Files
- Copy Dataset
- Split File...
- Select Cases...
- Weight Cases...

### Transform
- Compute Variable...
- Programmability Transformation...
- Count Values within Cases...
- Shift Values
- Recode into Same Variables...
- Recode into Different Variables...
- Automatic Recode...
- Create Dummy Variables
- Visual Binning...
- Optimal Binning...
- Prepare Data for Modeling ▸
- Rank Cases...
- Date and Time Wizard...
- Create Time Series...
- Replace Missing Values...
- Random Number Generators...
- Run Pending Transforms (Ctrl+G)

### Analyze
- Power Analysis ▸
  - Means ▸
  - Proportions ▸
  - Correlations ▸
  - Regression ▸
- Reports ▸
  - Codebook...
  - OLAP Cubes...
  - Case Summaries...
  - Report Summaries in Rows...
  - Report Summaries in Columns...
- Descriptive Statistics ▸
  - Frequencies...
  - Descriptives...
  - Distribution Fit
  - Explore...
  - Crosstabs...
  - TURF Analysis
  - Column proportions test
  - Ratio...
  - P-P Plots...
  - Q-Q Plots...
- Bayesian Statistics ▸
  - One Sample Normal
  - One Sample Binomial
  - One Sample Poisson
  - Related Samples Normal
  - Independent Samples Normal
  - Pearson Correlation
  - Linear Regression
  - One-way ANOVA
  - Loglinear Models
  - One-way Repeated Measures ANOVA
- Tables
- Compare Means ▸
  - Means...
  - One-Sample T Test...
  - Independent-Samples T Test...
  - Summary Independent-Samples T Test
  - Paired-Samples T Test...
  - One-Way ANOVA...
  - One-Sample Proportions...
  - Independent-Samples Proportions...
  - Paired-Samples Proportions...
- General Linear Model ▸
  - Univariate...
  - Multivariate...
  - Repeated Measures...
  - Variance Components...
- Generalized Linear Models ▸
  - Generalized Linear Models...
  - Generalized Estimating Equations...
- Mixed Models ▸
  - Linear...
  - Generalized Linear...
- Correlate ▸
  - Bivariate...
  - Partial...
  - Distances...
  - Canonical Correlation
- Regression ▸
  - Automatic Linear Modeling...
  - Linear...
  - Curve Estimation...
  - Regression Relative Importance
  - Partial Least Squares...
  - Ridge Regression...
  - Binary Logistic...
  - Multinomial Logistic...
  - Ordinal...
  - Probit...
  - Nonparametric Regression
  - Nonlinear...
  - Weight Estimation...
  - 2-Stage Least Squares...
  - Quantile...
  - Optimal Scaling (CATREG)...
- Loglinear ▸
  - General...
  - Logit...
  - Model Selection...
- Neural Networks ▸
  - Multilayer Perceptron...
  - Radial Basis Function...
- Classify ▸
  - TwoStep Cluster...
  - K-Means Cluster...
  - Hierarchical Cluster...
  - Cluster Silhouettes
  - Tree...
  - Discriminant...
  - Nearest Neighbor...
  - ROC Curve...
  - ROC Analysis...
- Dimension Reduction ▸
  - Factor...
  - Correspondence Analysis...
  - Optimal Scaling...
- Scale ▸
  - Reliability Analysis...
  - Weighted Kappa...
  - Multidimensional Unfolding (PREFSCAL)...
  - Multidimensional Scaling (PROXSCAL)...
  - Multidimensional Scaling (ALSCAL)...
- Nonparametric Tests ▸
  - One Sample...
  - Independent Samples...
  - Related Samples...
  - Legacy Dialogs ▸
- Forecasting ▸
  - Create Temporal Causal Models...
  - Create Traditional Models...
  - Apply Temporal Causal Models...
  - Apply Traditional Models...
  - Seasonal Decomposition...
  - Spectral Analysis...
  - Sequence Charts...
  - Autocorrelations...
  - Cross-Correlations...
- Survival ▸
  - Life Tables...
  - Kaplan-Meier...
  - Cox Regression...
  - Cox w Time-Dep Cov...
  - Parametric Regression
  - Cox Regression Extension
- Multiple Response ▸
  - Define Variable Sets...
  - *Frequencies...*
  - *Crosstabs...*
- Missing Value Analysis...
- Multiple Imputation ▸
  - Analyze Patterns...
  - Impute Missing Data Values...
- Complex Samples ▸
  - Select a Sample...
  - Prepare for Analysis...
  - Frequencies...
  - Descriptives...
  - Crosstabs...
  - Ratios...
  - General Linear Model...
  - Logistic Regression...
  - Ordinal Regression...
  - Cox Regression...
- Simulation...
- Quality Control ▸
  - Control Charts...
  - Pareto Charts...
- Spatial and Temporal Modeling... ▸
  - Spatial Modeling
- Direct Marketing ▸
  - Choose Technique...

### Graphs
- Chart Builder...
- Graphboard Template Chooser...
- Weibull Plot...
- Fluctuation Chart
- Compare Subgroups
- Regression Variable Plots
- *Regression Residual Plot*
- Boxplot Family
- Legacy Dialogs ▸
  - Bar...
  - 3-D Bar...
  - Line...
  - Area...
  - Pie...
  - High-Low...
  - Boxplot...
  - Error Bar...
  - Population Pyramid...
  - Scatter/Dot...
  - Histogram...

### Utilities
- Variables...
- OMS Control Panel...
- OMS Identifiers...
- Scoring Wizard...
- Merge Model XML...
- *Calculate with Pivot Table*
- Data File Comments...
- Define Variable Macro
- Define Variable Sets...
- Censor Table
- Use Variable Sets...
- Show All Variables
- Create Text Output
- *Spelling...*
- Process Data Files
- Run Script...
- Modify Table Appearance
- Production Facility...
- Map Conversion Utility...
- Python2 Conversion

### Extensions
- My Extensions
- Extension Hub...
- Install Local Extension Bundle...
- Custom Dialog Builder for Extensions
- R 3.6 Configuration
- Correlation Plot
- Utilities ▸
- Dynamite Plot
- String 2 Nominal
- Custom_Descriptive_Tables

### Window
- Split
- Minimize All Windows
- Go to Designated Viewer Window
- Go to Designated Syntax Window
- Reset Dialog Sizes and Positions
- 1 Untitled1 [DataSet0] - StatsBench Data Editor

### Help
- Topics
- StatsBench Support
- StatsBench Forums
- StatsBench Community
- Documentation in PDF Format
- Command Syntax Reference
- About...
- Diagnostic Tool...
- Give Feedback
- Report an Issue

---

## Appendix D — Screenshot index

| File (under `C:\Users\Saurav\AppData\Local\Temp\opencode`) | Content |
|---|---|
| `ref_data.png`, `ref_de2.png` | Data Editor; Variable View; error dialog |
| `ref_after_dismiss.png` | Session Management / restore-point dialog |
| `ref_open_dialog.png` | File > Open Data dialog |
| `ref_loaded.png` | `accidents.sav` Variable View |
| `ref_dataview.png` | Data View with value labels |
| `ref_freq_dialog.png` | Frequencies dialog, initial state |
| `ref_freq_selected.png` | Frequencies dialog with `agecat` selected; OK/Paste enabled |
| `ref_syntax_pasted.png` | Syntax Editor with generated `FREQUENCIES` command |
| `ref_output.png` | Viewer with Log, Statistics, and frequency table |
| `ref_regression_dialog.png` | Linear Regression dialog |
| `ref_chartbuilder.png` | Chart Builder measurement-level advisory |
| `ref_chartbuilder2.png` | Chart Builder main window |
| `refmenus2\*.png` | Top-level menu captures (File, Edit, View, Data, Transform, Analyze, Graphs, Utilities, Extensions, Window, Help) |
| `refsub2\*.png` | Analyze submenu captures (27 entries) |
| `refsub3\*.png` | Data > Validation/Merge/Orthogonal, Graphs > Legacy, Transform menu captures |

---

## Appendix E — Glossary

| Term | Meaning in this document |
|---|---|
| **`.sav`** | Native StatsBench data file: data plus dictionary plus file metadata |
| **`.spv`** | StatsBench output document: a ZIP container of XML parts (conforming to `output-1.8.xsd`) plus binary table-data chunks |
| **`.sps`** | StatsBench command syntax file |
| **`.spj`** | Production job file |
| **`.csplan` / `.csaplan`** | Complex-samples sampling / analysis plan |
| **`.viztemplate`** | Chart template used by Chart Builder and Graphboard |
| **`.spd` / `.spe`** | Extension dialog specification / extension specification |
| **PMML** | Predictive Model Markup Language; portable model format |
| **Pivot table** | Editable, multi-dimensional output table (not an image) |
| **OMS** | Output Management System: selects and routes output objects |
| **GPL** | Graphics Production Language used by `GGRAPH`/Chart Builder |
| **Procedure module** | One of the 132 `statsw*.dll` native libraries; most implement commands, a few handle formats |
| **Dictionary** | Variable metadata: names, types, labels, value labels, missing rules, level, role |
| **Measurement level** | Scale, ordinal, or nominal; drives procedure and chart validity |
| **Role** | Modelling intent (Input, Target, Both, None, Partition, Split) |
| **Restore point** | Auto-saved recovery snapshot of a Data Editor, Output, or Syntax window |
| **Extension bundle** | Installable R/Python command with dialog and syntax specification |

---

## Closing note

The strongest conclusion from the evidence is that StatsBench 27 is **one command language with several faces**: a Java client that constructs commands, a native engine that executes them, a structured output document that records them, and an extension layer that adds new commands without changing the core.

That design is why a dialog can be turned into syntax, why an old `GRAPH` command still runs beside a new Chart Builder chart, why an R package can appear as a native menu item, and why an `.spv` file remains readable and editable years later. The menus are large and sometimes repetitive, and the extension layer adds operational dependencies, but the underlying model is coherent: **metadata in, commands through, structured output out**.

*End of document.*













