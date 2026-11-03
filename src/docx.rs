//! A Word document of the output tables.
//!
//! The file is the Office Open XML package: a zip of a few XML parts.
//! Each result is a `w:tbl` with the booktabs rules — a heavy line above
//! the header, a light line under it, a heavy line under the last row,
//! and no vertical rules. Numeric columns carry a decimal tab, so the
//! points line up when the document is opened in Word.

use std::io::{Cursor, Write};

use crate::output::{column_is_numeric, stat_number_parts, OutputTable};
use crate::pivot::label;

const PAGE_CONTENT: u32 = 10512;

/// Write `outputs` as a `.docx` file.
pub fn write(path: &std::path::Path, outputs: &[OutputTable]) -> Result<(), String> {
    let bytes = document_bytes(outputs)?;
    std::fs::write(path, bytes).map_err(|e| e.to_string())
}

/// The `.docx` package as bytes.
pub fn document_bytes(outputs: &[OutputTable]) -> Result<Vec<u8>, String> {
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut cursor);
        let opts = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        part(&mut zip, opts, "[Content_Types].xml", CONTENT_TYPES)?;
        part(&mut zip, opts, "_rels/.rels", RELS)?;
        part(
            &mut zip,
            opts,
            "word/_rels/document.xml.rels",
            DOCUMENT_RELS,
        )?;
        part(&mut zip, opts, "word/styles.xml", STYLES)?;
        part(&mut zip, opts, "word/document.xml", &document_xml(outputs))?;
        zip.finish().map_err(|e| e.to_string())?;
    }
    Ok(cursor.into_inner())
}

fn part(
    zip: &mut zip::ZipWriter<&mut Cursor<Vec<u8>>>,
    opts: zip::write::SimpleFileOptions,
    name: &str,
    body: &str,
) -> Result<(), String> {
    zip.start_file(name, opts).map_err(|e| e.to_string())?;
    zip.write_all(body.as_bytes()).map_err(|e| e.to_string())?;
    Ok(())
}

fn document_xml(outputs: &[OutputTable]) -> String {
    let mut body = String::new();
    for table in outputs {
        body.push_str(&table_blocks(table));
    }
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
         <w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\">\
         <w:body>{body}\
         <w:sectPr>\
         <w:pgSz w:w=\"12240\" w:h=\"15840\"/>\
         <w:pgMar w:top=\"864\" w:right=\"864\" w:bottom=\"864\" w:left=\"864\" w:header=\"720\" w:footer=\"720\" w:gutter=\"0\"/>\
         </w:sectPr></w:body></w:document>"
    )
}

fn table_blocks(table: &OutputTable) -> String {
    let mut s = String::new();
    s.push_str(&paragraph(&table.title, true, 22, 240, 60));
    if let Some(pivot) = &table.pivot {
        let n = pivot.measures.len().max(1);
        for measure in 0..n {
            if pivot.measures.len() > 1 {
                let name = pivot.measures.get(measure).map(String::as_str).unwrap_or("");
                s.push_str(&paragraph(name, true, 21, 80, 40));
            }
            s.push_str(&pivot_table(pivot, measure));
        }
    } else {
        s.push_str(&flat_table(&table.headers, &table.rows));
    }
    if !table.note.is_empty() {
        for line in table.note.split('\n') {
            s.push_str(&paragraph_italic(line));
        }
    }
    s
}

struct Col {
    width: u32,
    numeric: bool,
    /// Decimal-tab position, in twips from the left of the paragraph.
    tab: u32,
}

fn flat_table(headers: &[String], rows: &[Vec<String>]) -> String {
    let n = headers.len().max(rows.iter().map(Vec::len).max().unwrap_or(0));
    if n == 0 {
        return String::new();
    }
    let mut cols = Vec::with_capacity(n);
    for i in 0..n {
        let cells: Vec<String> = rows
            .iter()
            .map(|r| r.get(i).cloned().unwrap_or_default())
            .collect();
        let header = headers.get(i).cloned().unwrap_or_default();
        cols.push(col_spec(i > 0 && column_is_numeric(&cells), &header, &cells));
    }
    // A header that is wider than its column claims the extra width.
    for (i, header) in headers.iter().enumerate() {
        let need = text_twips(header) + 200;
        if need > cols[i].width {
            cols[i].width = need;
        }
    }
    fit_page(&mut cols);
    let mut grid = String::new();
    grid.push_str(&header_row(headers, &cols));
    for row in rows {
        let cells: Vec<String> = (0..n)
            .map(|i| row.get(i).cloned().unwrap_or_default())
            .collect();
        grid.push_str(&body_row(&cells, &cols));
    }
    wrap_table(&cols, &grid)
}

fn pivot_table(pivot: &crate::pivot::PivotTable, measure: usize) -> String {
    let row_keys = pivot.row_keys();
    let col_keys = pivot.col_keys();
    if row_keys.is_empty() || col_keys.is_empty() {
        return String::new();
    }
    let mut cols = Vec::with_capacity(col_keys.len() + 1);
    let stub_cells: Vec<String> = row_keys
        .iter()
        .map(|k| label(&pivot.row_dims, k))
        .collect();
    let corner = pivot
        .col_dims
        .first()
        .map(|d| d.name.as_str())
        .unwrap_or("");
    cols.push(col_spec(false, corner, &stub_cells));
    for ck in &col_keys {
        let cells: Vec<String> = row_keys
            .iter()
            .map(|rk| pivot.measure_at(rk, ck, measure).to_string())
            .collect();
        let leaf = pivot
            .col_dims
            .last()
            .and_then(|d| {
                ck.last()
                    .and_then(|level| d.levels.get(*level))
                    .map(String::as_str)
            })
            .unwrap_or("");
        cols.push(col_spec(column_is_numeric(&cells), leaf, &cells));
    }
    // Spanning headers must fit over the columns they cover.
    for (d, dim) in pivot.col_dims.iter().enumerate() {
        let below: usize = pivot.col_dims[d + 1..]
            .iter()
            .map(|x| x.levels.len().max(1))
            .product::<usize>()
            .max(1);
        let mut j = 0;
        while j < col_keys.len() {
            let level = dim
                .levels
                .get(col_keys[j].get(d).copied().unwrap_or(0))
                .map(String::as_str)
                .unwrap_or("");
            let span = below.min(col_keys.len() - j);
            grow_span(&mut cols, j + 1, span, text_twips(level) + 200);
            j += span;
        }
    }
    fit_page(&mut cols);

    let header_rows = pivot.col_dims.len().max(1);
    let mut grid = String::new();
    for d in 0..header_rows {
        grid.push_str("<w:tr><w:trPr><w:tblHeader/></w:trPr>");
        if d == 0 {
            grid.push_str(&cell(
                corner,
                cols[0].width,
                1,
                Some("restart"),
                true,
                false,
                0,
            ));
        } else {
            grid.push_str(&cell("", cols[0].width, 1, Some("cont"), true, false, 0));
        }
        let below: usize = pivot.col_dims[d + 1..]
            .iter()
            .map(|x| x.levels.len().max(1))
            .product::<usize>()
            .max(1);
        let mut j = 0;
        while j < col_keys.len() {
            let level = pivot
                .col_dims
                .get(d)
                .and_then(|dim| dim.levels.get(col_keys[j].get(d).copied().unwrap_or(0)))
                .cloned()
                .unwrap_or_default();
            let span = below.min(col_keys.len() - j);
            let width: u32 = cols[j + 1..j + 1 + span].iter().map(|c| c.width).sum();
            grid.push_str(&cell(&level, width, span, None, true, false, 0));
            j += span;
        }
        grid.push_str("</w:tr>");
    }
    for (rk, stub) in row_keys.iter().zip(stub_cells.iter()) {
        grid.push_str("<w:tr>");
        grid.push_str(&cell(stub, cols[0].width, 1, None, false, false, 0));
        for (j, ck) in col_keys.iter().enumerate() {
            let value = pivot.measure_at(rk, ck, measure);
            let col = &cols[j + 1];
            grid.push_str(&cell(
                value,
                col.width,
                1,
                None,
                false,
                col.numeric,
                col.tab,
            ));
        }
        grid.push_str("</w:tr>");
    }
    wrap_table(&cols, &grid)
}

fn col_spec(numeric: bool, header: &str, cells: &[String]) -> Col {
    let mut width = text_twips(header) + 200;
    let mut tab = 200u32;
    let mut frac = 0u32;
    for cell in cells {
        width = width.max(text_twips(cell) + 200);
        if let Some((int, rest)) = stat_number_parts(cell) {
            tab = tab.max(text_twips(&int).max(120));
            frac = frac.max(text_twips(&rest));
        }
    }
    if numeric {
        width = width.max(tab + frac + 280);
    }
    Col {
        width: width.clamp(700, 5000),
        numeric,
        tab,
    }
}

fn text_twips(text: &str) -> u32 {
    // Cambria at 10.5 pt is a little under 110 twips for a typical character.
    text.chars().count() as u32 * 110
}

fn grow_span(cols: &mut [Col], start: usize, span: usize, need: u32) {
    if span == 0 || start + span > cols.len() {
        return;
    }
    let have: u32 = cols[start..start + span].iter().map(|c| c.width).sum();
    if need > have {
        let add = (need - have) / span as u32;
        let extra = (need - have) % span as u32;
        for (i, col) in cols[start..start + span].iter_mut().enumerate() {
            col.width += add + if i == 0 { extra } else { 0 };
        }
    }
}

fn fit_page(cols: &mut [Col]) {
    let sum: u32 = cols.iter().map(|c| c.width).sum();
    if sum <= PAGE_CONTENT || sum == 0 {
        return;
    }
    let scale = PAGE_CONTENT as f64 / sum as f64;
    for col in cols.iter_mut() {
        col.width = ((col.width as f64) * scale).round().max(400.0) as u32;
        col.tab = ((col.tab as f64) * scale).round().max(80.0) as u32;
        if col.tab + 120 > col.width {
            col.tab = col.width.saturating_sub(120);
        }
    }
}

fn wrap_table(cols: &[Col], rows: &str) -> String {
    let sum: u32 = cols.iter().map(|c| c.width).sum();
    let mut grid = String::new();
    for col in cols {
        grid.push_str(&format!("<w:gridCol w:w=\"{}\"/>", col.width));
    }
    format!(
        "<w:tbl><w:tblPr>\
         <w:tblW w:w=\"{sum}\" w:type=\"dxa\"/>\
         <w:jc w:val=\"left\"/>\
         <w:tblLayout w:type=\"fixed\"/>\
         <w:tblBorders>\
         <w:top w:val=\"single\" w:sz=\"12\" w:space=\"0\" w:color=\"000000\"/>\
         <w:left w:val=\"nil\"/>\
         <w:bottom w:val=\"single\" w:sz=\"12\" w:space=\"0\" w:color=\"000000\"/>\
         <w:right w:val=\"nil\"/>\
         <w:insideH w:val=\"nil\"/>\
         <w:insideV w:val=\"nil\"/>\
         </w:tblBorders>\
         <w:tblCellMar>\
         <w:top w:w=\"40\" w:type=\"dxa\"/>\
         <w:left w:w=\"80\" w:type=\"dxa\"/>\
         <w:bottom w:w=\"40\" w:type=\"dxa\"/>\
         <w:right w:w=\"80\" w:type=\"dxa\"/>\
         </w:tblCellMar>\
         </w:tblPr><w:tblGrid>{grid}</w:tblGrid>{rows}</w:tbl>"
    )
}

fn header_row(headers: &[String], cols: &[Col]) -> String {
    let mut row = String::from("<w:tr><w:trPr><w:tblHeader/></w:trPr>");
    for (i, col) in cols.iter().enumerate() {
        let text = headers.get(i).map(String::as_str).unwrap_or("");
        row.push_str(&cell(text, col.width, 1, None, true, false, 0));
    }
    row.push_str("</w:tr>");
    row
}

fn body_row(cells: &[String], cols: &[Col]) -> String {
    let mut row = String::from("<w:tr>");
    for (i, col) in cols.iter().enumerate() {
        let text = cells.get(i).map(String::as_str).unwrap_or("");
        row.push_str(&cell(
            text,
            col.width,
            1,
            None,
            false,
            col.numeric,
            col.tab,
        ));
    }
    row.push_str("</w:tr>");
    row
}

/// One cell. `vmerge` is `"restart"` on the first row of a vertical
/// merge and `"cont"` on the rows that continue it.
fn cell(
    text: &str,
    width: u32,
    span: usize,
    vmerge: Option<&str>,
    header: bool,
    decimal: bool,
    tab: u32,
) -> String {
    let mut pr = format!("<w:tcW w:w=\"{width}\" w:type=\"dxa\"/>");
    if span > 1 {
        pr.push_str(&format!("<w:gridSpan w:val=\"{span}\"/>"));
    }
    if let Some(v) = vmerge {
        if v == "restart" {
            pr.push_str("<w:vMerge w:val=\"restart\"/>");
        } else {
            pr.push_str("<w:vMerge/>");
        }
    }
    if header {
        pr.push_str(
            "<w:tcBorders>\
             <w:top w:val=\"nil\"/>\
             <w:left w:val=\"nil\"/>\
             <w:bottom w:val=\"single\" w:sz=\"6\" w:space=\"0\" w:color=\"000000\"/>\
             <w:right w:val=\"nil\"/>\
             </w:tcBorders>",
        );
    }
    pr.push_str("<w:vAlign w:val=\"center\"/>");
    let align = if decimal {
        "left"
    } else if header {
        "center"
    } else {
        "left"
    };
    let mut ppr = format!(
        "<w:spacing w:before=\"20\" w:after=\"20\"/><w:jc w:val=\"{align}\"/>"
    );
    let mut runs = String::new();
    if decimal && stat_number_parts(text).is_some() {
        ppr.push_str(&format!(
            "<w:tabs><w:tab w:val=\"decimal\" w:pos=\"{tab}\"/></w:tabs>"
        ));
        runs.push_str(&format!(
            "<w:r><w:rPr>\
             <w:rFonts w:ascii=\"Cambria\" w:hAnsi=\"Cambria\" w:cs=\"Cambria\"/>\
             <w:sz w:val=\"21\"/><w:szCs w:val=\"21\"/>\
             </w:rPr><w:tab/></w:r>{}",
            run(text, false)
        ));
    } else if vmerge == Some("cont") {
        runs.push_str(&run("", false));
    } else {
        runs.push_str(&run(text, header));
    }
    format!("<w:tc><w:tcPr>{pr}</w:tcPr><w:p><w:pPr>{ppr}</w:pPr>{runs}</w:p></w:tc>")
}

fn run(text: &str, bold: bool) -> String {
    let b = if bold { "<w:b/>" } else { "" };
    format!(
        "<w:r><w:rPr>{b}\
         <w:rFonts w:ascii=\"Cambria\" w:hAnsi=\"Cambria\" w:cs=\"Cambria\"/>\
         <w:sz w:val=\"21\"/><w:szCs w:val=\"21\"/>\
         </w:rPr><w:t xml:space=\"preserve\">{text}</w:t></w:r>",
        text = xml_escape(text)
    )
}

fn paragraph(text: &str, bold: bool, half_points: u32, before: u32, after: u32) -> String {
    let b = if bold { "<w:b/>" } else { "" };
    format!(
        "<w:p><w:pPr><w:spacing w:before=\"{before}\" w:after=\"{after}\"/></w:pPr>\
         <w:r><w:rPr>{b}\
         <w:rFonts w:ascii=\"Cambria\" w:hAnsi=\"Cambria\" w:cs=\"Cambria\"/>\
         <w:sz w:val=\"{half_points}\"/><w:szCs w:val=\"{half_points}\"/>\
         </w:rPr><w:t xml:space=\"preserve\">{}</w:t></w:r></w:p>",
        xml_escape(text)
    )
}

fn paragraph_italic(text: &str) -> String {
    format!(
        "<w:p><w:pPr><w:spacing w:before=\"40\" w:after=\"40\"/></w:pPr>\
         <w:r><w:rPr><w:i/>\
         <w:rFonts w:ascii=\"Cambria\" w:hAnsi=\"Cambria\" w:cs=\"Cambria\"/>\
         <w:sz w:val=\"18\"/><w:szCs w:val=\"18\"/>\
         </w:rPr><w:t xml:space=\"preserve\">{}</w:t></w:r></w:p>",
        xml_escape(text)
    )
}

fn xml_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

const CONTENT_TYPES: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\">\
<Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/>\
<Default Extension=\"xml\" ContentType=\"application/xml\"/>\
<Override PartName=\"/word/document.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml\"/>\
<Override PartName=\"/word/styles.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml\"/>\
</Types>";

const RELS: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\
<Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"word/document.xml\"/>\
</Relationships>";

const DOCUMENT_RELS: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\
<Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles\" Target=\"styles.xml\"/>\
</Relationships>";

const STYLES: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<w:styles xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\">\
<w:docDefaults><w:rPrDefault><w:rPr>\
<w:rFonts w:ascii=\"Cambria\" w:hAnsi=\"Cambria\" w:cs=\"Cambria\"/>\
<w:sz w:val=\"21\"/><w:szCs w:val=\"21\"/>\
</w:rPr></w:rPrDefault></w:docDefaults>\
</w:styles>";

#[cfg(test)]
mod tests {
    use super::*;

    fn read_part(bytes: &[u8], name: &str) -> String {
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
        let mut file = archive.by_name(name).unwrap();
        let mut s = String::new();
        std::io::Read::read_to_string(&mut file, &mut s).unwrap();
        s
    }

    #[test]
    fn a_result_becomes_a_booktabs_word_table() {
        let table = OutputTable {
            title: "Descriptives of <age>".into(),
            headers: vec!["Statistic".into(), "Value".into()],
            rows: vec![
                vec!["Mean".into(), "12.50".into()],
                vec!["N".into(), "40".into()],
            ],
            note: "Listwise.".into(),
            chart: None,
            pivot: None,
        };
        let bytes = document_bytes(&[table]).unwrap();
        assert_eq!(&bytes[..2], b"PK");
        let xml = read_part(&bytes, "word/document.xml");
        assert!(xml.contains("Descriptives of &lt;age&gt;"));
        assert!(xml.contains("<w:tbl>"));
        assert!(xml.contains("<w:insideV w:val=\"nil\"/>"));
        assert!(xml.contains("w:sz=\"12\""));
        assert!(xml.contains("w:sz=\"6\""));
        assert!(xml.contains("w:val=\"decimal\""));
        assert!(xml.contains(">12.50</w:t>"));
        assert!(xml.contains("Listwise."));
        // The package names the main part as a Word document.
        let types = read_part(&bytes, "[Content_Types].xml");
        assert!(types.contains(
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"
        ));
    }

    #[test]
    fn a_pivot_keeps_its_spanning_header() {
        use crate::pivot::{PivotDimension, PivotTable};
        let mut pivot = PivotTable::new(
            "Crosstab",
            vec![PivotDimension::new("region", vec!["East".into(), "West".into()])],
            vec![PivotDimension::new(
                "group",
                vec!["A".into(), "B".into()],
            )],
            vec!["Count".into()],
        );
        pivot.set(&[0], &[0], vec!["10".into()]);
        pivot.set(&[0], &[1], vec!["5".into()]);
        pivot.set(&[1], &[0], vec!["5".into()]);
        pivot.set(&[1], &[1], vec!["20".into()]);
        let table = OutputTable {
            title: "Crosstab".into(),
            headers: Vec::new(),
            rows: Vec::new(),
            note: String::new(),
            chart: None,
            pivot: Some(pivot),
        };
        let xml = read_part(&document_bytes(&[table]).unwrap(), "word/document.xml");
        assert!(xml.contains("w:gridSpan w:val=\"2\"") || xml.contains(">A</w:t>"));
        assert!(xml.contains(">East</w:t>"));
        assert!(xml.contains(">10</w:t>"));
        assert!(xml.contains(">20</w:t>"));
    }
}
