# StatsCraft feature gap plan

**Audit date:** 22 October 2026
**Compared against:** [`REFERENCE_FEATURES.md`](../REFERENCE_FEATURES.md)
**StatsCraft version:** 0.2.0
**Purpose:** separate what is finished from what only looks finished, then put
the remaining work in a deliberate order.

## How to read this

- **Done** means the behaviour exists, is reachable from the UI or syntax, and
  is covered by a test where the result is numeric.
- **Partial** means the core path works but an important reference behaviour
  does not.
- **Stub** means the menu item exists and reports "not implemented yet".
- **Missing** means there is no command, dialog, or data model for it.

Priority is about user harm, not about how interesting the feature is:

| Priority | Meaning |
|---|---|
| **P0** | Can lose work, mislead the analyst, or make a core workflow untrustworthy. |
| **P1** | Needed for ordinary data preparation and analysis; the absence blocks real projects. |
| **P2** | Important breadth or interoperability; can wait for a later release. |
| **P3** | Specialist product areas; out of scope for the current workbench. |

## Executive summary

StatsCraft 0.2.0 is a credible small statistics workbench, not a reduced copy
of the reference product. It has a real data grid, a real variable dictionary,
18 procedures, one syntax language, and a pivot-table output model. The
statistics that exist are tested against closed-form values rather than against
the app itself.

The largest remaining problems are not missing algorithms. They are:

1. **The UI advertises work it cannot do.** Undo, Redo, Cut, Select Cases,
   Curve Estimation, Reports, and almost the whole Transform menu are visible
   commands that only set a status message. That is worse than not shipping
   them, because it teaches the user not to trust the menus.
2. **There is no undo.** Every cell edit, paste, case deletion, and variable
   deletion is immediate. A confirmation guards variable deletion, but there
   is no way to recover from a wrong edit.
3. **Value labels are editable but not visible in Data View.** The dictionary
   now stores them; the grid still prints raw codes and there is no View >
   Value Labels switch.
4. **The output is a result, not a record.** There is no Log object, no saved
   output document, and no general pivot. The current `.doc`/`.xls` exports
   are Office-flavoured HTML, not native Office files.
5. **The syntax runner is all-or-nothing.** It runs the whole buffer. There is
   no Run Selection, Run Current Command, command outline, line-number gutter,
   or active-dataset indicator.
6. **Data preparation is the biggest functional hole.** There is no merge,
   reshape, aggregate, select, weight, split, recode, count, rank, date, or
   missing-value replacement. Those are not advanced features; they are the
   middle of a normal project.

## What 0.2.0 actually fixed

| Area | State | Evidence |
|---|---|---|
| Variable View Values column | **Done.** Clickable `None.` / `Defined (n).` control opens a real value-label editor for numeric and string variables. | `src/dialog.rs`, `src/data.rs`; `value_labels` tests |
| Value-label input layout | **Done.** The old Grid collapsed the label field. Rows now use a 100 px value field and a 240 px label field. | `src/dialog.rs`, `show_value_labels` |
| Delete variable | **Done.** Trash icon, warning dialog, no immediate removal. | `assets/icons/delete_variable.svg`, `src/icon.rs`, `src/app.rs` |
| Unsaved close | **Done.** Window close and File > Exit show Save / Don't Save / Cancel. | `src/app.rs`, `show_close_prompt`, `save_and_close` |
| Output viewer layout | **Done.** Fixed 236 px outline, wide result pane, stable scroll-area IDs. | `src/app.rs`, Output viewport |
| Toolbar Variable / Add Case / Labels | **Done in 0.2.0.** They now call the real actions. | `src/app.rs`, toolbar |
| Branding and showcase | **Done.** Fresh screenshots; stale `showcase/` duplicate and pivot shots removed. | `README.md`, `docs/showcase/` |
| Test suite | **Done.** 107 tests pass. | `cargo test` |

## What StatsCraft does wrong

### P0: trust and data safety

1. **Visible stubs masquerade as features.**
   `Edit > Undo`, `Edit > Redo`, `Edit > Cut`, `Data > Select Cases`,
   `Analyze > Regression > Curve Estimation`, `Analyze > Reports`, and every
   item in `Transform` only write a "not implemented yet" status. The user has
   no way to tell a working command from a placeholder until they try it.
   **Fix:** either implement the command or disable it with a "planned for
   0.3" tooltip. Do not leave an enabled menu item that cannot act.

2. **No undo/redo stack.**
   The data editor mutates the live table. A mis-typed cell, a bad paste, or a
   wrong case deletion is permanent once the confirmation is passed.
   **Fix:** snapshot-based undo for the table plus selection, or command
   journaling. Minimum: one undo level for the last cell/block edit and one
   for case/variable deletion. This is the first 0.3 feature.

3. **Data View ignores the dictionary it now edits.**
   `Data View` renders `DataTable::cell_text`, which returns the stored value.
   Value labels are not substituted, and there is no `View > Value Labels`
   toggle. The reference product treats label display as a presentation
   setting separate from stored data (§4.1, §4.4, §8.3).
   **Fix:** add `show_value_labels: bool` to the app, format numeric and string
   cells through the dictionary, and add the checked menu item. Editing must
   still operate on the underlying value, not on the label text.

4. **Case deletion is less protected than variable deletion.**
   `Data > Delete Cases` removes the selected row immediately. The reference
   confirms destructive data edits, and the 0.2.0 variable path already has
   the pattern to reuse.
   **Fix:** reuse the confirmation window, with "Delete case N?" and a clear
   statement that undo is not available until 0.3.

### P1: misleading or incomplete core workflows

5. **Syntax execution has no scope control.**
   The editor has one `Run Syntax` button and runs the whole buffer. There is
   no Run Selection, Run Current Command, line-number gutter, command outline,
   or active-dataset display (§6.1–6.3).
   **Fix:** add a command splitter at statement boundaries, Run Selection and
   Run Current Command, and a read-only command list beside the editor. This
   is more valuable than syntax colouring.

6. **Output is not a document.**
   The Viewer shows result tables but has no Log object, no Notes, no saved
   `.spv` equivalent, and no print path. The export is a one-way copy of the
   currently selected table or the whole list. The reference output is a
   document whose log records the real commands (§7.1–7.6).
   **Fix:** make the output list a numbered document, add a Log object per run,
   save/load that document, and print it. Keep Markdown/HTML as export, not as
   the document model.

7. **Pivot support is only a transpose.**
   The `Pivot` button swaps row and column dimensions. The reference model lets
   dimensions move between layers, rows, and columns, and lets cells and labels
   be edited (§7.3). StatsCraft's `PivotTable` is a good start but its public
   surface is one operation.
   **Fix:** expose layer/row/column assignment in the pivot model before
   adding more table types.

8. **The Office exports are HTML with Office extensions.**
   `export_output_office_to` writes an HTML document with Word and Excel
   namespaces to a `.doc` or `.xls` path. Word and Excel may open it, but it is
   not a native document, and the extension overstates what was produced.
   **Fix now:** label the menu entries "Word-compatible HTML" and
   "Excel-compatible HTML", or write real `.docx`/`.xlsx` packages.

9. **Measurement level is stored but weakly enforced.**
   `ColumnKind` is only Numeric or String. `Measure` is persisted and shown,
   but procedures mostly check numeric versus string, not Scale versus Nominal.
   A nominal code can be dropped into a scale-only slot. The reference uses
   the measure level to shape dialogs and chart choices (§4.3, §5.5).
   **Fix:** add a central `accepts(role, measure)` check used by every dialog,
   and make the scale slots reject nominal/ordinal variables with an
   explanation rather than silently computing a mean.

10. **The Transform menu is a façade.**
    `COMPUTE` exists in syntax, but there is no Compute Variable dialog, no
    target/expression builder, no conditional `IF`, and no function browser.
    Recode, Count, Rank, Automatic Recode, Date/Time, Replace Missing Values,
    and Create Dummy Variables are absent. This is the largest gap for real
    data work (§8.5).
    **Fix order:** Compute dialog → Recode (same/different) → Count → Rank →
    Automatic Recode → date/time and missing-value replacement.

### P2: breadth and interoperability

11. **Data management stops at sort.**
    There is no Select Cases, Weight Cases, Split File, Aggregate, Merge Files,
    Restructure (wide/long), Transpose, Sort Variables, multi-key sort, or
    Create Value Labels from Data (§8.4). `Sort Cases` handles one key only.
    **Fix order:** Select Cases/filter → Weight → Split File → Aggregate →
    Add/Match Files → Restructure.

12. **Only CSV and `.sav` move data.**
    Excel, text import wizard, databases, SAS, and Stata are missing (§10).
    `.sav` preservation is good and should stay the native path; Excel import
    is the highest-value addition.
    **Fix:** add `.xlsx` read via a small, well-tested crate before adding any
    database driver.

13. **No restore points or auto-recovery.**
    The reference treats session recovery as a first-class feature (§4.8).
    StatsCraft has only the close prompt.
    **Fix:** periodic serialized snapshot next to the settings file, offered at
    startup when the previous session did not close cleanly.

14. **Multiple datasets are not modelled.**
    `DATASET` is acknowledged but ignored. There is one active table, one
    output list, and one syntax buffer (§4.7). This blocks split/merge
    workflows and makes `DATASET ACTIVATE` syntax a lie.
    **Fix:** introduce a dataset registry before adding merge/reshape, because
    those commands need a source and destination dataset.

15. **No custom attributes, validation rules, duplicate detection, or
    multiple-response sets.**
    These are dictionary-level features that make surveys and panel data
    manageable (§4.6, §8.4). Lower priority than preparation, but they belong
    with the metadata work rather than with new algorithms.

### P3: specialist procedures

16. **Large areas of the reference Analyze menu are absent.**
    Logistic/ordinal/multinomial regression, generalized linear and mixed
    models, clustering, discriminant analysis, survival, time series and
    forecasting, multiple imputation, complex samples, power analysis,
    Bayesian procedures, neural networks, decision trees, simulation, quality
    control, and direct marketing are not implemented (§8.6, §9).
    These should not be attempted as a list. Add a procedure only when its
    data model, diagnostics, and output contract are understood.

## Missing feature matrix

| Reference area | StatsCraft 0.2.0 | Gap | Priority |
|---|---|---|---|
| Data View / Variable View | Both exist | Value labels not displayed; no column chooser | P0/P1 |
| Variable metadata | Name, type, width, decimals, label, values, missing, columns, align, measure, role | Type is only Numeric/String; role is always Input; no custom attributes | P1 |
| Value labels | Editable, numeric and string | No bulk editor, no labels-from-data | P1 |
| Missing values | Discrete + range, excluded listwise | No per-procedure missing options or pairwise deletion | P1 |
| Multiple datasets | No | Single table only | P2 |
| Undo/redo, cut | No | Enabled stubs | P0 |
| Find/replace, go to case/variable | No | Search absent | P2 |
| Sort cases | One key | No multi-key, no sort variables | P1 |
| Merge, aggregate, reshape, transpose, split, select, weight | No | Entire preparation layer | P1 |
| Compute | Syntax only | No dialog, no IF, limited function set | P1 |
| Recode/count/rank/autorecode/date/missing replacement | No | Entire Transform layer | P1 |
| 18 analysis procedures | Frequencies, Descriptives, Crosstabs, t-tests, ANOVA, GLM, Correlations, Regression, Means, Explore, Reliability, Factor, ROC, four nonparametric tests | No one-sample t, proportions, logistic, clustering, survival, time series, imputation | P2/P3 |
| Dialogs | 18 procedures | No Reset/Help; limited validation by measure; no block entry for regression | P1 |
| Syntax editor | Whole-buffer run, monospace editor | No selection/current run, line numbers, outline, macros/includes | P1 |
| Output viewer | Outline, pivot transpose, copy, export | No Log, no document save/print, no general pivot, no cell editing | P1 |
| Charts | Chart Builder, bar/histogram/scatter/box/ROC | No chart editor, no legacy dialog family, limited styling | P2 |
| Data I/O | CSV, `.sav` read/write | No Excel, databases, SAS/Stata | P2 |
| Output export | Markdown, HTML, Office-HTML | No PDF, real DOCX/XLSX, PPT, print | P2 |
| Automation | One syntax language | No macros, includes, Python/R, OMS, scripting | P3 |
| Recovery | Close prompt | No restore points or auto-recovery | P2 |

## Recommended sequence

### 0.3 — Trustworthy editing — **Done (0.3.0)**

1. Undo/redo for table mutations, with tests for cell edit, paste, case
   deletion, and variable deletion.
2. Value-label display in Data View plus the View toggle.
3. Confirmation for case deletion.
4. Disable every remaining stub or replace it with a working command.
5. Syntax Run Selection / Run Current Command and a command outline.
6. Log object in the Output Viewer.

This release should not add a single new statistical procedure. Its job is to
make the existing surface honest.

**What shipped.** The undo history covers cell edits, pastes, cuts, case
additions and deletions, and variable additions and deletions; `Ctrl+Z` and
`Ctrl+Y` drive it, and the toolbar buttons show when there is nothing to
undo. `View > Value Labels` shows the dictionary's labels in Data View
without changing the stored codes. `Data > Delete Cases` confirms first. The
Transform menu, `Data > Select Cases`, `Curve Estimation` and `Reports` are
greyed out and say which release will bring them. The Syntax Editor runs a
selection or the command under the cursor and lists the commands of the
buffer. The Output Viewer opens with a **Log** object, and the output can be
saved to a `.scdoc` document and opened again.

### 0.4 — Preparation and output — **Done (0.4.0)**

1. Compute Variable dialog with `IF`.
2. Recode into Same/Different Variables, Count, Rank, Automatic Recode.
3. Select Cases, Weight Cases, Split File.
4. Aggregate, then Add/Match Files, then Restructure.
5. Save/load an output document (0.3); print; Office exports labelled as
   Word/Excel-compatible HTML (not native OOXML).
6. Restore points (`.sav` snapshot after unclean exit).

### 0.5 — Analytical breadth

1. One-sample t and proportions.
2. Regression diagnostics: residuals, collinearity, plots.
3. Binary logistic regression.
4. K-means and hierarchical clustering.
5. Kaplan-Meier survival.
6. Multiple imputation only after the missing-data model is designed.

Anything beyond that is a separate product decision, not a backlog item.

## Acceptance tests for the next release

- Edit a cell, press Ctrl+Z, and get the previous value back; press Ctrl+Y and
  get the edit again.
- Delete a variable, undo, and recover its metadata and every cell value.
- Switch value labels on in Data View; edit a labelled cell and verify the
  stored code is unchanged.
- Delete a case and see the confirmation before the row count changes.
- Select three lines of syntax and run only those lines; the Log records only
  those commands.
- Close with unsaved edits, choose Cancel, and verify the document is still
  open and dirty.
- Export an output table and reopen the saved output document without losing
  the table, title, or log.

## Bottom line

The 0.2.0 release fixed the embarrassing parts of the metadata editor and the
destructive-action flow. The next release must not chase the reference
product's long Analyse menu. It must make the existing editor safe, make the
menus truthful, and make the output reproducible. Data preparation comes after
that; advanced modelling comes last.
