//! The undo history for the data editor.
//!
//! Every change the analyst makes to the dataset can be taken back. The
//! history is a pair of stacks: the changes that can still be undone, and
//! the changes that have been undone and can be redone.
//!
//! Two kinds of change are recorded:
//!
//! * a **cell** change, which stores the value before and after the edit.
//!   This is the common case — one cell, one value — and it costs
//!   nothing to remember.
//! * a **table** change, which stores the whole table before and after.
//!   It is used for the changes that are not one cell: a paste, the
//!   deletion of a case or a variable, a sort, the definition of value
//!   labels. Those change the shape of the dataset, so the shape is what
//!   is remembered.
//!
//! The reference workbench keeps an unlimited history; this one keeps
//! the hundred most recent changes, which is plenty for a working
//! session and keeps the memory bounded.

use crate::data::{DataTable, Value};

/// How many changes are kept.
pub const MAX_HISTORY: usize = 100;

/// One reversible change to the dataset.
#[derive(Debug, Clone)]
pub enum UndoAction {
    /// A single cell changed its value.
    Cell {
        col: usize,
        row: usize,
        before: Value,
        after: Value,
    },
    /// The dataset changed as a whole.
    Table {
        before: Option<Box<DataTable>>,
        after: Option<Box<DataTable>>,
    },
}

impl UndoAction {
    /// A short description of the change, for the status bar.
    pub fn describe(&self) -> String {
        match self {
            UndoAction::Cell { col, row, .. } => {
                format!("Edit of cell at column {}, row {}", col + 1, row + 1)
            }
            UndoAction::Table { .. } => "Change to the dataset".to_string(),
        }
    }
}

/// The undo and redo stacks.
#[derive(Debug, Default)]
pub struct UndoStack {
    /// The changes that can still be undone, oldest first.
    pub undo: Vec<UndoAction>,
    /// The changes that have been undone, oldest first.
    pub redo: Vec<UndoAction>,
}

impl UndoStack {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a change. Recording a new change clears the redo stack,
    /// because the undone changes no longer follow the new one.
    pub fn push(&mut self, action: UndoAction) {
        self.undo.push(action);
        if self.undo.len() > MAX_HISTORY {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    /// Record a change of the whole table.
    ///
    /// `before` is the dataset as it was before the change, and the
    /// dataset in `table` is the state after it.
    pub fn push_table(&mut self, before: Option<DataTable>, table: &Option<DataTable>) {
        self.push(UndoAction::Table {
            before: before.map(Box::new),
            after: table.clone().map(Box::new),
        });
    }

    /// Forget the history.
    ///
    /// A dataset that is loaded is a new document, so the changes of
    /// the previous one are not its own to take back.
    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }

    /// True when there is something to undo.
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    /// True when there is something to redo.
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Take back the most recent change.
    ///
    /// The dataset is changed in place, and the change moves to the redo
    /// stack. The description of the change is returned, so the caller
    /// can say what was taken back.
    pub fn undo(&mut self, table: &mut Option<DataTable>) -> Option<String> {
        let action = self.undo.pop()?;
        let description = action.describe();
        apply(&action, table, false);
        self.redo.push(action);
        Some(description)
    }

    /// Put back the most recently undone change.
    pub fn redo(&mut self, table: &mut Option<DataTable>) -> Option<String> {
        let action = self.redo.pop()?;
        let description = action.describe();
        apply(&action, table, true);
        self.undo.push(action);
        Some(description)
    }
}

/// Apply an action to the dataset, forwards or backwards.
fn apply(action: &UndoAction, table: &mut Option<DataTable>, forward: bool) {
    match action {
        UndoAction::Cell {
            col,
            row,
            before,
            after,
        } => {
            let value = if forward { after } else { before };
            if let Some(t) = table.as_mut() {
                if let Some(c) = t.columns.get_mut(*col) {
                    if *row < c.values.len() {
                        c.values[*row] = value.clone();
                    }
                }
            }
        }
        UndoAction::Table { before, after } => {
            let snapshot = if forward { after } else { before };
            *table = snapshot.as_ref().map(|t| (**t).clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{Column, ColumnKind, DataTable, VariableMeta};

    fn table() -> DataTable {
        DataTable {
            n_rows: 2,
            columns: vec![Column {
                name: "x".into(),
                kind: ColumnKind::Numeric,
                values: vec![Value::Number(1.0), Value::Number(2.0)],
                meta: VariableMeta::default_for(ColumnKind::Numeric),
            }],
        }
    }

    #[test]
    fn a_cell_edit_is_taken_back_and_put_back() {
        let mut t = Some(table());
        let mut h = UndoStack::new();
        let before = t.clone();
        t.as_mut().unwrap().columns[0].values[0] = Value::Number(9.0);
        h.push_table(before, &t);

        assert!(h.can_undo());
        h.undo(&mut t);
        assert_eq!(
            t.as_ref().unwrap().columns[0].values[0],
            Value::Number(1.0),
            "the old value is back"
        );
        assert!(h.can_redo());
        h.redo(&mut t);
        assert_eq!(
            t.as_ref().unwrap().columns[0].values[0],
            Value::Number(9.0),
            "the edit is back"
        );
    }

    #[test]
    fn a_cell_action_restores_one_value() {
        let mut t = Some(table());
        let mut h = UndoStack::new();
        h.push(UndoAction::Cell {
            col: 0,
            row: 1,
            before: Value::Number(2.0),
            after: Value::Number(7.0),
        });
        t.as_mut().unwrap().columns[0].values[1] = Value::Number(7.0);
        h.undo(&mut t);
        assert_eq!(t.as_ref().unwrap().columns[0].values[1], Value::Number(2.0));
        h.redo(&mut t);
        assert_eq!(t.as_ref().unwrap().columns[0].values[1], Value::Number(7.0));
    }

    #[test]
    fn a_new_change_clears_the_redo_stack() {
        let mut t = Some(table());
        let mut h = UndoStack::new();
        h.push(UndoAction::Cell {
            col: 0,
            row: 0,
            before: Value::Number(1.0),
            after: Value::Number(3.0),
        });
        h.undo(&mut t);
        assert!(h.can_redo());
        h.push(UndoAction::Cell {
            col: 0,
            row: 0,
            before: Value::Number(1.0),
            after: Value::Number(4.0),
        });
        assert!(!h.can_redo(), "the redo stack is cleared");
    }

    #[test]
    fn the_history_is_bounded() {
        let mut h = UndoStack::new();
        for i in 0..(MAX_HISTORY + 10) {
            h.push(UndoAction::Cell {
                col: 0,
                row: 0,
                before: Value::Number(0.0),
                after: Value::Number(i as f64),
            });
        }
        assert_eq!(h.undo.len(), MAX_HISTORY);
    }
}
