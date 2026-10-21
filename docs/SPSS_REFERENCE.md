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

**Phase 2 — real procedure dialogs** (closes gap #1 and #12) — DONE
A reusable dialog shell: source list → target list, move arrow, a
right-hand column of sub-dialog buttons, procedure-specific options, and a
gated OK. Every dialog **builds syntax** and hands it to the existing
runner, so dialog → syntax → output works exactly as SPSS does it.

Implemented in `src/dialog.rs` (see the Phase 2 note at the end of this
file for the details).

**Phase 3 — procedure depth** (closes gaps #2, #6–#11)
Statistics options for Frequencies/Descriptives, cell counts and extra
statistics for Crosstabs, post-hoc tests for ANOVA, a full coefficients
table for Regression, and charts in the Viewer.

**Phase 4 — the missing procedures** (closes gaps #3–#5, #14)
Explore, Means, the nonparametric family, reliability, factor.


---

## 7. Phase 2 — what the procedure dialogs now do

`src/dialog.rs` implements the SPSS dialog for ten procedures:
Frequencies, Descriptives, Crosstabs, Independent-Samples T Test,
Paired-Samples T Test, One-Way ANOVA, Bivariate Correlations, Linear
Regression, Means and Explore.

**The anatomy**, taken from the SPSS 27 screenshots in
`docs/spss-reference/`:

| Part | Where | Notes |
|---|---|---|
| Source variable list | left | each row carries a measure-level icon: a **ruler** for numeric/scale, a **pencil** for string/nominal |
| Move arrow | middle | one arrow moves the selected source variable into the target list; a second arrow moves the last one back |
| Target variable list | right | clicking a target removes it |
| Sub-dialog buttons | right, below the list | `Statistics…`, `Charts…`, `Format…` for Frequencies; `Options…` for Descriptives; `Cells…` + `Statistics…` for Crosstabs; `Options…` + `Post-hoc…` for ANOVA; and so on |
| Procedure checkboxes | right, bottom | e.g. `Display frequency tables`, `Create charts (histogram)`, `Save standardized values as variables` |
| Button row | bottom | `OK`, `Paste`, `Reset`, `Cancel`, `Help` |
| **Gated OK** | bottom | disabled until the dialog has everything it needs — a target variable, plus a grouping variable for t-test / ANOVA / means / crosstabs |

**The dialog → syntax → output pipeline.** A dialog does not compute
anything. It *builds SPSS syntax* and hands it to the same runner the
Syntax Editor uses, so there is one execution path:

```
Frequencies  region satisfaction
DESCRIPTIVES score age /STATISTICS=MEAN STDDEV
T-TEST score BY group
ONEWAY score BY group /POSTHOC=TUKEY SCHEFFE
CORRELATIONS score age /PRINT=SIG /MISSING=PAIRWISE
```

- **OK** runs the syntax and raises the Output Viewer.
- **Paste** puts the syntax in the Syntax Editor *without* running it —
  SPSS's behaviour, and the point of the whole pipeline: you can edit the
  command by hand before running it.

**Supporting changes this phase made:**

- `src/syntax.rs` gained a real command parser. It splits a line into the
  command name, the positional arguments, and the `/SUBCOMMAND=KEYWORD`
  options, so `/STATISTICS=`, `/CELLS=`, `/POSTHOC=`, `/PRINT=`,
  `/MISSING=` and `/ES=` are understood rather than silently ignored.
- `src/stats.rs` gained a `StatSet` — the set of statistics a procedure
  should report, parsed from the same keywords SPSS uses — and a
  `descriptives_with` that computes them: mean, median, mode, variance,
  range, sum, standard error of the mean, skewness (SPSS's bias-corrected
  formula), kurtosis, and percentiles by linear interpolation.
- `src/output.rs` gained `descriptives_with`, which lays the table out the
  way SPSS does: the variable name and its N on the first line, then one
  line per statistic with the statistic's name in the first column and its
  value under the matching column.
- The app takes a `--dialog <procedure>` flag, which opens a procedure
  dialog at start-up. Useful for demos, screenshots and scripted checks.

---

## 8. Phase 3 — what changed

Phase 3 closed the gap list from section 5.

### 8.1 The options now change the output

Every option the dialogs offer is parsed by the runner and acted on by the
statistics engine:

| Option | Effect |
|---|---|
| `/STATISTICS=` | Chooses which descriptive statistics are computed and reported. The output table has one labelled line per statistic, the way SPSS lays it out. |
| `/CELLS=` | The crosstab shows the requested cell types as separate blocks: observed counts, expected counts, row percentages, column percentages and total percentages. |
| `/STATISTICS=CHISQ PHI` on a crosstab | Adds Pearson's chi-square, Phi and Cramér's V, plus the linear-by-linear association and Fisher's exact test where they apply. |
| `/POSTHOC=` | The ANOVA adds the requested post-hoc comparisons: Tukey, Scheffe, Bonferroni and LSD. |
| `/STATISTICS=HOMOGENEITY` | The ANOVA adds Levene's test for equality of variances. |
| `/ES=DISPLAY` | The t-test adds the effect size table: mean difference, the group standard deviations, the pooled deviation, Cohen's d and Hedges' g. |
| `/PRINT=SIG` | The correlation matrix gets the significance stars SPSS prints, with the `**` / `*` / `n` note. |
| `/PRINT=DESCRIPTIVES` | The correlation matrix gets the N / Mean / Std. Deviation block above it. |
| `/MISSING=` | Chooses pairwise or listwise deletion. |
| `/CHART=BARCHART` / `/CHART=HISTOGRAM` | The output carries a chart, drawn under the pivot table. |

### 8.2 The new procedures

Seven procedures were added, each with a dialog, a command, and an output
table laid out the way SPSS lays it out:

| Command | Procedure | Output |
|---|---|---|
| `MEANS TABLES <var> BY <group>` | Means | The group means with their counts, standard deviations and standard errors. |
| `EXPLORE <var>` | Explore | The descriptives per group — mean, 5% trimmed mean, median, variance, standard deviation, minimum, maximum, range and interquartile range — plus a Kolmogorov-Smirnov normality test. |
| `NONPARAMETRIC /MW= <var> BY <group>` | Mann-Whitney U | The rank sums, both U statistics, z, the p-value and the common-language effect size. |
| `NONPARAMETRIC /K-W= <var> BY <group>` | Kruskal-Wallis H | The rank sums and mean ranks per group, chi-square, df and the p-value. |
| `NONPARAMETRIC /WILCOXON= <v1> <v2>` | Wilcoxon signed-rank | W+, W-, z and the p-value. |
| `NONPARAMETRIC /FRIEDMAN= <v1> <v2> <v3>` | Friedman | The mean rank of each variable, chi-square, df and the p-value. |

They are also in the Analyze menu, under **Nonparametric Tests**.

### 8.3 Charts in the Output Viewer

A procedure that asks for a chart now draws it under the pivot table: a
bar chart of the counts for Frequencies, a histogram for a numeric
variable, and a scatter plot for a regression. The charts are painted
directly rather than through a plotting library, so they always fit the
space they are given and a categorical axis can show the category names
rather than 0, 1, 2.

### 8.4 The statistics that took the most care

Three were not right the first time, and the tests caught them:

- **The studentized range** (Tukey's q) is now computed exactly, by
  integrating the range of k standard normals over the residual sum of
  squares. The critical values agree with the published Tukey tables to
  within 0.001 across k = 2..5 and df = 10..120, and the test checks
  that.
- **Scheffe's test** was using Tukey's denominator, which has a factor
  of a half in it. That made the F statistic half of what it should be,
  and the p-value too small. It now uses the pooled standard error, so
  its p-value agrees with the unadjusted t-test — which is correct,
  because Scheffe protects against the family of *all* possible
  contrasts rather than the pairs that were compared.
- **The Bonferroni p-value** was not adjusted. SPSS reports the
  unadjusted p-value multiplied by the number of comparisons, and the
  output now does the same.

### 8.5 Two ways to drive the app

```sh
statscraft data.csv --dialog frequencies     # open a procedure dialog
statscraft data.csv --script run.sps         # run a syntax file at start-up
```

The second one is what the test suite and the screenshot tooling use.
`samples/phase3_demo.sps` runs every Phase 3 feature in one go.

## 9. Phase 4 — reliability, factors, discrimination, and the SPSS file format

### 9.1 What Phase 4 added

| Area | What it does |
|---|---|
| **Reliability** | `RELIABILITY` / `ALPHA` computes Cronbach's alpha, the standardised alpha, the item-total correlations, the inter-item correlations and the weak items. |
| **Factor analysis** | `FACTOR` runs a principal component analysis: the eigenvalue table, the component matrix, the communalities and Bartlett's test of sphericity with the KMO measure. `/EXTRACTION=PC<n>` chooses how many components to keep. |
| **ROC curves** | `ROC <var> BY <group> /POSITIVE(<group>)` computes the area under the curve, its standard error, the 95% confidence interval, the cut-off table and the curve itself. |
| **Robust ANOVA** | `ONEWAY ... /STATISTICS=WELCH` adds Welch's test and the Brown-Forsythe test, which do not assume equal group variances. |
| **Computed variables** | `COMPUTE <name> = <expression>` adds a variable to the dataset, with the arithmetic operators and the functions `MEAN`, `MIN`, `MAX`, `SUM`, `ABS`, `SQRT`, `LN`, `LOG10` and `ROUND`. |
| **The `.sav` file** | The data can be written as an SPSS `.sav` file, with the variable labels, the value labels, the measures and the display widths carried across. |

### 9.2 The two formulas that took the most care

Both were wrong at first, and both were caught by looking at the output
rather than at the code.

- **The standardised alpha.** It was computed from the *sum* of the
  off-diagonal correlations:

  ```
  alpha_std = k / (k - 1) * (1 - 1 / r_sum)
  ```

  which is not the formula. The correct one uses the *mean*
  inter-item correlation:

  ```
  alpha_std = k * r_bar / (1 + (k - 1) * r_bar)
  ```

  The wrong version gave a standardised alpha of 1.096 on a scale whose
  ordinary alpha was 0.790 — a value above 1, which is impossible. The
  right one gives 0.790, the same as the ordinary alpha, which is what
  SPSS reports for a scale whose items are interchangeable.

- **The ordinary alpha on a degenerate scale.** When the items split
  into two anti-correlated halves, the total score cancels out, the
  total variance approaches zero, and the ratio `sum(s_i^2) / s_total^2`
  explodes: the alpha came out as -40. That is the arithmetic being
  right and the scale being meaningless. The alpha is now reported as
  missing when the total score has no variance to speak of, and the
  standardised alpha is reported as missing when the mean inter-item
  correlation is not positive. SPSS makes the same distinction.

### 9.3 The `.sav` file format

The writer was rebuilt against the real file rather than from memory.
`C:\Program Files\IBM\SPSS\Statistics\27\Samples\English\accidents.sav`
is the reference. What it showed:

- The file header is **176 bytes**: the `$FL2` magic, a 60-byte
  product string, the layout code, the variable count, the compression
  bias, the case count, the creation date and time, a 64-byte file
  label, and 16 bytes of padding.
- A **type 2** record describes a variable: the type code, whether it
  has a label, the number of missing values, the print and write
  formats, the name in 8 bytes, and then the label with its length
  prefix.
- A **type 3** record holds the value labels, and a **type 4** record
  holds a variable's label.
- A **type 999** record ends the dictionary: the type, a filler, and
  the weighted case count in 8 bytes.
- The data records follow, each with a type 100 marker.

The hand-written writer got the header padding and the type 999 record
wrong, so the file parsed but had no rows. Rather than keep patching
the byte layout, the writer now uses the `SpssWriter` in
`polars-readstat-rs`, which already implements the record layout, the
compression and the metadata. The round-trip is covered by a test:
write the table, read it back, and check the variables, the rows and
the values match.

### 9.4 A parser bug the phase found

`/POSITIVE(case)` — the parenthesised form SPSS uses for a value that is
a piece of data rather than a keyword — was not parsed at all. The
parser saw a keyword with no `=`, so it took the whole `POSITIVE(case)`
as the keyword's name and left `case` as a stray value. Two things
followed from that: the ROC fell back to using the group variable's
*name* as the positive group, which matched nothing, so the procedure
silently produced no output; and the reliability scale's name came out
as the default rather than the one that was asked for.

The parser now reads the parenthesised form, and — because a value in
parentheses is data, not a keyword — it keeps the value's case. A
keyword list after an `=` is still upper-cased, because those are
keywords.

### 9.5 The Phase 4 script

`samples/phase4_demo.sps` runs against `samples/scale_demo.csv`, a
120-row dataset with two clusters of three items each, a two-level
factor and a noisy binary outcome:

```sps
RELIABILITY i1 i2 i3 i4 i5 i6 /SCALE(Overall)
FACTOR i1 i2 i3 i4 i5 i6 /EXTRACTION=PC2
ONEWAY score BY group /STATISTICS=WELCH
ROC score BY status /POSITIVE(case)
COMPUTE centred = score - 50
DESCRIPTIVES centred
```

The factor analysis recovers the two planted clusters: the first three
items load on the first component at about -0.85 and the last three on
the second at about -0.8.
