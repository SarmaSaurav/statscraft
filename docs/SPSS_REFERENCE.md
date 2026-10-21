# IBM SPSS Statistics 27 — reference notes

Captured from the installed copy at
`C:\Program Files\IBM\SPSS\Statistics\27` on 2026-10-21.
Screenshots in `docs/spss-reference/`.

This file is the source of truth for *what to build next*. It records the
observed behaviour of the original, the authoritative command grammar shipped
with it, and the gap list for StatsCraft.

---

## 1. Process architecture (confirmed by running it)

- `stats.exe` is a native launcher that boots a JVM (`JRE/`) and loads the
  Swing client. Its window title is `Untitled1 [DataSet0] - IBM SPSS
  Statistics Data Editor`.
- `spssengine.exe` is a **separate backend process** started alongside it.
  It appears in the process list as its own `spssengine` entry.
- Two process-level entry points are visible in the task list: `stats.exe`
  (the client) and `spssengine.exe` (the compute engine).
- The install ships **329 command-grammar files** in `syntax_xml/*.xml`.
  These are the authoritative definition of every command's parameters and
  are the best reference for correct procedure behaviour — better than
  guessing from a screenshot.

### Measured window geometry (for scripting the menus)

The SPSS window is positioned at `(-8, -8)`, so screen coordinates are
offset from the client area:

| Element | Screen position |
|---|---|
| `Analyze` menu label | `(285, 38)` |
| Analyze item rows | `y = 63, 89, 115, 141, 167, 193` (26 px apart) |
| `Descriptive Statistics` | row 3, `y = 115` |
| Submenu leaves | `x = 572` |

---

## 2. The Analyze menu (full reference)

Top-level items, in order, with their submenus:

| Item | Submenu |
|---|---|
| Power Analysis | (power procedures) |
| Reports | Codebook, OLAF Cubes, Case Summaries, Report Summaries in Rows, Report Summaries in Columns |
| **Descriptive Statistics** | **Frequencies, Descriptives, Distribution Fit, Explore, Crosstabs, TURF Analysis, Column proportions test** |
| Bayesian Statistics | One Sample Normal/Binomial/Poisson, Related Samples Normal, Independent Samples Normal, Pearson Correlation, … |
| Tables | Custom Tables, Multiple Response Sets, Define Category Order |
| Compare Means | Means, One-Sample T Test, Independent-Samples T Test, Summary Independent-Samples T Test, Paired-Samples T Test, … |
| General Linear Model | (GLM procedures) |
| Generalized Linear Models | (GLM procedures) |
| Mixed Models | (mixed procedures) |
| Correlate | Bivariate, Partial, Rank, Distance, Similarities, Canonical, Model Fit |
| Regression | Linear, Quadratic, Cubic, Curve Estimation, … |
| **Compare Means** (2nd) | |
| Explore | |
| Reports | |
| Table | |
| Reliability Analysis | |
| Multivariate | |
| Power Analysis | |
| Classification | |
| Factor | |
| Scaling | |
| Nonparametric Tests | |
| Time Series | |

### The Frequencies dialog (the key reference)

`docs/spss-reference/spss27-frequencies-dialog.png` — 815×556.

Layout:
- **Left**: source variable list, each row with a *measure-level icon*
  (a yellow pencil for string/nominal, a blue ruler for numeric/scale) and
  the variable name.
- **Middle**: `Variable(s):` target list, initially empty.
- **Between them**: a single blue **arrow button** (→) that moves the
  selected source variable into the target list. There is no separate
  "available/target" button pair — one arrow does the move.
- **Right column of buttons**: `Statistics…`, `Charts…`, `Format…`,
  `Style…`, `Bootstrap…`. These are the per-procedure sub-dialogs.
- **Bottom left**: `☑ Display frequency tables` and `☐ Create APA style tables`.
- **Bottom right**: `OK` (greyed until a variable is chosen), `Paste`
  (greyed), `Reset`, `Cancel`, `Help`.
- The OK button is **disabled until at least one variable is in the target
  list** — the dialog validates before it will run.

The `Descriptives` dialog (`docs/spss-reference/spss27-descriptives-dialog.png`)
follows the same template: source list, target list, arrow, and a right-hand
column with `Options…`, `Style…`, `Bootstrap…`; plus a
`☐ Save standardized values as variables` checkbox and the same
OK/Paste/Reset/Cancel/Help button row.

**Design implication for StatsCraft:** a procedure dialog is *not* a small
extra panel. It is a real window with two list boxes, a move button, a
right-hand column of sub-dialog buttons, procedure-specific checkboxes, and
a gated OK. That is the single biggest interaction gap in our app.

---

## 3. The Data Editor

`docs/spss-reference/spss27-data-editor.png`.

- Window title: `Untitled1 [DataSet0] - IBM SPSS Statistics Data Editor`.
  The dataset name and number are part of the title.
- Menu bar: `File Edit View Data Transform Analyze Graphs Utilities
  Extensions Window Help`.
- Toolbar: ~16 small **raster/bitmap icons** in colour (blue/grey/red),
  each ~24×24 — open, save, print, undo/redo, insert-variable, insert-case,
  value-labels, filter, find, chart, data-view, variable-view, … Two of them
  are "switch between Data View and Variable View" grid icons.
- Under the toolbar: a `Visible: 0 of 0 Variables` label on the right, and a
  filter text box in the middle.
- The grid: light grey cells on white, a header band of `var` placeholders,
  a narrow row-number gutter.
- Bottom-left: **`Data View` / `Variable View` document tabs**, the active
  one rendered as a white document attached to the grid.
- Bottom-right status bar: `IBM SPSS Statistics Processor is ready` with a
  green check, and `Unicode: ON`.
- Empty state: the grid still shows the full cell block matrix with `var`
  headers — it does not show an empty-state message.

---

## 4. Command grammar (from `syntax_xml/`)

The install ships one XML per command (329 files). Each declares
`GLOBAL_PARAMS`, optional `Subcommand`s, and the keyword parameters of
each. This is authoritative for what a correct implementation must accept.

### FREQUENCIES
- `GLOBAL_PARAMS`: VARIABLES
- `FORMAT`: NOTABLE, LIMIT, AVALUE, DVALUE, AFREQ, DFREQ
- `MISSING`: INCLUDE
- `BARCHART`: MINIMUM, MAXIMUM, FREQ, PERCENT
- `PIECHART`: MINIMUM, MAXIMUM, FREQ, PERCENT, MISSING, NOMISSING
- `HISTOGRAM`: MINIMUM, MAXIMUM, FREQ, NONORMAL, NORMAL
- `GROUPED` / `NTILES` / `PERCENTILES`: MEAN, STDDEV, MINIMUM, MAXIMUM,
  SEMEAN, VARIANCE, SKEWNESS, SESKEW, RANGE, MODE, KURTOSIS, SEKURT,
  MEDIAN, SUM, ALL, NONE
- `STATISTICS`: same set as above
- `ORDER`: ANALYSIS, VARIABLE

### DESCRIPTIVES
- `GLOBAL_PARAMS`: VARIABLES
- `MISSING`: VARIABLE, LISTWISE, INCLUDE
- `SAVE`: DEFAULT, MEAN, MIN, SKEWNESS, STDDEV, SEMEAN, MAX, KURTOSIS,
  VARIANCE, SUM, RANGE, ALL
- `STATISTICS`: same set
- `SORT`: MEAN, SEMEAN, STDDEV, VARIANCE, KURTOSIS, SKEWNESS, RANGE, MIN,
  MAX, SUM, NAME, A, D

### CROSSTABS (alias `CHISQUARE INDEPENDENCE`)
- `GLOBAL_PARAMS`: TABLES, BY
- `MISSING`: TABLE, INCLUDE, REPORT
- `WRITE`: NONE, CELLS, ALL
- `FORMAT`: AVALUE, DVALUE, TABLES, NOTABLES
- `COUNT`: ASIS, CASE, CELL, ROUND, TRUNCATE
- `CELLS`: COUNT, NONE, ROW, COLUMN, TOTAL, EXPECTED, RESID, SRESID,
  ASRESID, ALL, PROP, BPROP
- `STATISTICS`: CHISQ, PHI, CC, ALL, LAMBDA, UC, RISK, NONE, BTAU, CTAU,
  KAPPA, GAMMA, D, MCNEMAR, ETA, CORR, CMH
- `METHOD`: MC, CIN, SAMPLES, EXACT, TIMER
- `BARCHART`: COUNT
- `HIDESMALLCOUNTS`: COUNT

### T-TEST
- `GLOBAL_PARAMS`: TESTVAL, GROUPS, PAIRS
- `MISSING` / `CRITERIA` (CI) / `ES` (DISPLAY, STANDARDIZER)
- Three forms: one-sample (`TESTVAL`), independent (`GROUPS … BY …`),
  paired (`PAIRS`).

### ONEWAY
- `GLOBAL_PARAMS`: BY
- `CRITERIA`: CILEVEL
- `POSTHOC` / `POLYNOMIAL` / `CONTRAST`: SNK, TUKEY, BTUKEY, DUNCAN,
  SCHEFFE, DUNNETT, BONFERRONI, LSD, SIDAK, GT2, GABRIEL, FREGW, QREGW,
  T2, T3, GH, C, WALLER, ALPHA
- `STATISTICS`: NONE, DESCRIPTIVES, EFFECTS, HOMOGENEITY, ALL, WELCH,
  BROWNFORSYTHE
- `PLOT MEANS`, `MISSING`, `MATRIX`, `TEMPLATE`, `ES`

### REGRESSION
- `DESCRIPTIVES`, `SELECT`/`MISSING` (LISTWISE, PAIRWISE, MEANSUBSTITUTION,
  INCLUDE), `REGWGT`/`STATISTICS` (R, COEFF, ANOVA, OUTS, ZPP, LABEL, CI,
  F, BCOV, SES, XTX, COLLIN, TOL, SELECTION, CHANGE), `CRITERIA`
  (TOLERANCE, MAXSTEPS, PIN, POUT, FIN, FOUT, CIN), `METHOD`
  (STEPWISE, FORWARD, BACKWARD, ENTER, REMOVE, TEST), `RESIDUALS`,
  `CASEWISE`, `OUTFILE` (COVB, MODEL, CORB, PARAMETER), `SAVE` (PRED,
  ADJPRED, SRESID, MAHAL, RESID, ZPRED, SDRESID, COOK, DRESID, ZRESID,
  SEPRED, LEVER, DFBETA, …).

### CORRELATIONS
- `GLOBAL_PARAMS`: VARIABLES, WITH
- `MISSING`: PAIRWISE, LISTWISE, INCLUDE, EXCLUDE
- `PRINT`: TWOTAIL, ONETAIL, SIG, NOSIG, FULL, LOWER, LNODIAG, NOMATRIX
- `STATISTICS`: DESCRIPTIVES, XPROD, ALL
- `CI`: BIAS, CILEVEL

### MEANS
- `GLOBAL_PARAMS`: TABLES, BY, ALL
- `CELLS`: MEAN, COUNT, STDDEV, MEDIAN, GMEDIAN, SEMEAN, SUM, MIN, MAX,
  RANGE, VAR, KURT, SEKURT, SKEW, SESKEW, FIRST, LAST, NPCT, SPCT,
  HARMONIC, GEOMETRIC, DEFAULT, ALL
- `STATISTICS`: ANOVA, LINEARITY, ALL, NONE

---

## 5. Gap list for StatsCraft

Ordered by how much it costs the user.

| # | Gap | Why it matters | Reference |
|---|---|---|---|
| 1 | **No procedure dialogs.** Menu items auto-pick variables and run. | The user cannot choose which variables go into an analysis, cannot see the options, and cannot paste the syntax. This is the single biggest interaction gap. | Frequencies / Descriptives dialogs |
| 2 | **No `Statistics…` / `Format…` / `Charts…` sub-dialogs.** | Frequencies can only produce a bare count/percent table — no mean/median/quartiles, no histogram, no charts. | FREQUENCIES grammar |
| 3 | **No `Explore…`, `Means…`, `Table` / Custom Tables.** | Descriptive work is limited to the two procedures we have. | Analyze menu |
| 4 | **No nonparametric tests** (Mann-Whitney, Kruskal-Wallis, Wilcoxon, Friedman). | Standard for ordinal/Nonparametric data. | Analyze menu |
| 5 | **No reliability analysis, factor analysis, ROC curves.** | Common in survey research. | Analyze menu |
| 6 | **Descriptives has no statistic selection** — always the same set. | SPSS lets you tick mean/median/stddev/… | DESCRIPTIVES grammar |
| 7 | **Crosstabs has no cell counts (expected/row%/column%) and no other statistics** (Phi, Cramér's V, Fisher's exact). | We only emit χ². | CROSSTABS grammar |
| 8 | **One-way ANOVA has no post-hoc comparisons and no Welch/Brown-Forsythe.** | Comparing >2 groups needs them. | ONEWAY grammar |
| 9 | **Regression has no coefficients table, ANOVA table, R², diagnostics.** | We produce a bare summary. | REGRESSION grammar |
| 10 | **Correlations has no significance stars or pairwise-delete option.** | | CORRELATIONS grammar |
| 11 | **No chart output at all** (histogram, bar, scatter, boxplot in the Viewer). | Charts are a first-class SPSS output. | FREQUENCIES `BARCHART`/`HISTOGRAM` |
| 12 | **No `Paste` / syntax round-trip from a dialog.** | The whole point of SPSS's pipeline is dialog → syntax → runner → output. | OK/Paste buttons |
| 13 | **Data Editor**: no `Visible: N of M Variables` in the right place, no cell-formatting, no value-label editor. | | Data Editor screenshot |
| 14 | **No `.sav` writing, no `SAVE` of computed variables.** | | DESCRIPTIVES `SAVE` |

---

## 6. Plan

**Phase 1 — modernise the chrome** (no behaviour change)
Match the SPSS structure but modern: the Data Editor chrome, tab strip,
status bar, and a proper window title. Already largely done in the
"Modern UI" commit; this phase closes the remaining gaps (visible-variable
counter placement, dataset title, empty-state grid).

**Phase 2 — real procedure dialogs** (closes gap #1 and #12)
A reusable dialog shell: source list → target list, move arrow, a
right-hand column of sub-dialog buttons, procedure-specific options, and a
gated OK. Every dialog **builds syntax** and hands it to the existing
runner, so dialog → syntax → output works exactly as SPSS does it.

**Phase 3 — procedure depth** (closes gaps #2, #6–#11)
Statistics options for Frequencies/Descriptives, cell counts and extra
statistics for Crosstabs, post-hoc tests for ANOVA, a full coefficients
table for Regression, and charts in the Viewer.

**Phase 4 — the missing procedures** (closes gaps #3–#5, #14)
Explore, Means, the nonparametric family, reliability, factor.
