//! The output document: the saved form of the Output Viewer.
//!
//! The reference workbench keeps its results in a document that can be
//! saved and opened again. This module writes that document in a plain
//! text form, so a saved file can be read back without a serialization
//! dependency.
//!
//! The format keeps what the reader sees: the title of each table, its
//! columns, its rows, its footnote, and the Log of the commands that
//! produced it. A table that carries a pivot structure is written as the
//! table it is shown as, because the flattened form is the one the
//! reader sees.

use crate::output::OutputTable;

/// The first line of every output document.
const MAGIC: &str = "STATSCRAFT-OUTPUT 1";

/// Write the output document.
pub fn save(
    path: &std::path::Path,
    outputs: &[OutputTable],
    log: &[String],
) -> Result<(), String> {
    let mut text = String::new();
    text.push_str(MAGIC);
    text.push('\n');

    // The Log: the commands that were run.
    text.push_str("LOG\n");
    for command in log {
        text.push_str(&escape(command));
        text.push('\n');
    }
    text.push_str("END LOG\n");

    // The tables, in the order they were produced.
    for t in outputs {
        text.push_str("TABLE\n");
        text.push_str(&format!("TITLE\t{}\n", escape(&t.title)));
        // The cells are escaped one by one and joined with a real tab,
        // so a tab inside a cell does not look like a column break.
        let columns: Vec<String> = t.headers.iter().map(|h| escape(h)).collect();
        text.push_str(&format!("COLUMNS\t{}\n", columns.join("\t")));
        for row in &t.rows {
            let cells: Vec<String> = row.iter().map(|c| escape(c)).collect();
            text.push_str(&format!("ROW\t{}\n", cells.join("\t")));
        }
        if !t.note.is_empty() {
            text.push_str(&format!("NOTE\t{}\n", escape(&t.note)));
        }
        text.push_str("END TABLE\n");
    }

    std::fs::write(path, text).map_err(|e| format!("write {}: {}", path.display(), e))
}

/// Read an output document back.
pub fn load(path: &std::path::Path) -> Result<(Vec<OutputTable>, Vec<String>), String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("read {}: {}", path.display(), e))?;
    parse(&text)
}

/// Read an output document from a string.
pub fn parse(text: &str) -> Result<(Vec<OutputTable>, Vec<String>), String> {
    let mut lines = text.lines();
    let magic = lines.next().unwrap_or("").trim();
    if magic != MAGIC {
        return Err("This is not a StatsCraft output document.".to_string());
    }
    let mut outputs: Vec<OutputTable> = Vec::new();
    let mut log: Vec<String> = Vec::new();
    let mut in_log = false;
    let mut in_table = false;
    let mut title = String::new();
    let mut headers: Vec<String> = Vec::new();
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut note = String::new();

    for raw in lines {
        let line = raw.trim_end_matches(['\r', '\n']);
        if in_log {
            if line == "END LOG" {
                in_log = false;
                continue;
            }
            log.push(unescape(line));
            continue;
        }
        if in_table {
            if line == "END TABLE" {
                outputs.push(OutputTable {
                    title: std::mem::take(&mut title),
                    headers: std::mem::take(&mut headers),
                    rows: std::mem::take(&mut rows),
                    pivot: None,
                    note: std::mem::take(&mut note),
                    chart: None,
                });
                in_table = false;
                continue;
            }
            if let Some(rest) = line.strip_prefix("TITLE\t") {
                title = unescape(rest);
            } else if let Some(rest) = line.strip_prefix("COLUMNS\t") {
                headers = rest.split('\t').map(unescape).collect();
            } else if let Some(rest) = line.strip_prefix("ROW\t") {
                let cells: Vec<String> = rest.split('\t').map(unescape).collect();
                rows.push(cells);
            } else if let Some(rest) = line.strip_prefix("NOTE\t") {
                note = unescape(rest);
            }
            continue;
        }
        match line {
            "LOG" => in_log = true,
            "TABLE" => in_table = true,
            _ => {}
        }
    }
    Ok((outputs, log))
}

/// Escape the characters that would break the line structure.
fn escape(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

/// Undo the escaping.
fn unescape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('t') => out.push('\t'),
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('\\') => out.push('\\'),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> OutputTable {
        OutputTable {
            title: "Descriptive Statistics".into(),
            headers: vec!["Variable".into(), "Mean".into()],
            rows: vec![vec!["score".into(), "12.5".into()]],
            pivot: None,
            note: "Based on 6 cases.".into(),
            chart: None,
        }
    }

    #[test]
    fn a_document_round_trips_with_its_tables_and_its_log() {
        let dir = std::env::temp_dir();
        let path = dir.join("sc_out_doc.scdoc");
        let outputs = vec![table()];
        let log = vec!["LOAD \"data.csv\"".to_string(), "DESCRIPTIVES score".to_string()];
        save(&path, &outputs, &log).expect("the document is written");

        let (back_outputs, back_log) = load(&path).expect("the document is read");
        assert_eq!(back_outputs.len(), 1);
        assert_eq!(back_outputs[0].title, "Descriptive Statistics");
        assert_eq!(back_outputs[0].headers, vec!["Variable", "Mean"]);
        assert_eq!(back_outputs[0].rows, vec![vec!["score", "12.5"]]);
        assert_eq!(back_outputs[0].note, "Based on 6 cases.");
        assert_eq!(back_log, log);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn a_document_with_several_tables_keeps_them_in_order() {
        let dir = std::env::temp_dir();
        let path = dir.join("sc_out_doc2.scdoc");
        let mut second = table();
        second.title = "Crosstabulation".into();
        save(&path, &[table(), second], &[]).expect("written");
        let (back, log) = load(&path).expect("read");
        assert_eq!(back.len(), 2);
        assert_eq!(back[0].title, "Descriptive Statistics");
        assert_eq!(back[1].title, "Crosstabulation");
        assert!(log.is_empty());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn a_document_that_is_not_one_is_refused() {
        let err = parse("hello").expect_err("it is not a document");
        assert!(err.contains("not a StatsCraft output document"));
    }

    #[test]
    fn tabs_and_newlines_survive_the_round_trip() {
        let dir = std::env::temp_dir();
        let path = dir.join("sc_out_doc3.scdoc");
        let mut t = table();
        t.rows = vec![vec!["a\tb".into(), "c\nd".into()]];
        save(&path, &[t], &[]).expect("written");
        let (back, _) = load(&path).expect("read");
        assert_eq!(back[0].rows[0][0], "a\tb");
        assert_eq!(back[0].rows[0][1], "c\nd");
        let _ = std::fs::remove_file(path);
    }
}
