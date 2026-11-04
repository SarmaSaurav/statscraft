//! A DrawingML chart, packaged the way Office puts a chart on the clipboard.
//!
//! The package is a zip. Word and PowerPoint read `Art::GVML ClipFormat`
//! and turn it into a native chart. The numbers live in the chart
//! (`c:numLit`), so the paste does not need an embedded workbook.
//! Bars, histograms, lines and scatters map onto chart types. A box plot
//! does not, and stays a picture.

use crate::output::{Chart, ChartKind, FigureTheme, LegendPlace};

/// The GVML package, or `None` when this chart has no Office chart type.
pub fn gvml(chart: &Chart) -> Option<Vec<u8>> {
    let body = chart_xml(chart)?;
    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut cursor);
        let opts = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        let mut add = |name: &str, text: &str| -> Result<(), String> {
            zip.start_file(name, opts).map_err(|e| e.to_string())?;
            use std::io::Write;
            zip.write_all(text.as_bytes()).map_err(|e| e.to_string())
        };
        add("[Content_Types].xml", CONTENT_TYPES).ok()?;
        add("_rels/.rels", RELS).ok()?;
        add("clipboard/drawings/_rels/drawing1.xml.rels", DRAWING_RELS).ok()?;
        add("clipboard/drawings/drawing1.xml", DRAWING).ok()?;
        add("clipboard/charts/chart1.xml", &body).ok()?;
        zip.finish().ok()?;
    }
    Some(cursor.into_inner())
}

fn chart_xml(chart: &Chart) -> Option<String> {
    if chart.series.is_empty() || chart.kind == ChartKind::BoxPlot {
        return None;
    }
    let plot = match chart.kind {
        ChartKind::Bar => bar_chart(chart, 80),
        ChartKind::Histogram => bar_chart(chart, 0),
        ChartKind::Line => line_chart(chart),
        ChartKind::Scatter => scatter_chart(chart),
        ChartKind::BoxPlot => return None,
    }?;
    let legend = if show_legend(chart) {
        "<c:legend><c:legendPos val=\"b\"/><c:overlay val=\"0\"/></c:legend>"
    } else {
        ""
    };
    let title = if chart.title.is_empty() {
        "<c:autoTitleDeleted val=\"1\"/>".to_string()
    } else {
        format!(
            "<c:title><c:overlay val=\"0\"/>{}</c:title><c:autoTitleDeleted val=\"0\"/>",
            rich(&chart.title, true, 1200, 0)
        )
    };
    let text_style = latin(1000, false, 0);
    Some(format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
         <c:chartSpace xmlns:c=\"http://schemas.openxmlformats.org/drawingml/2006/chart\" \
         xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\">\
         <c:roundedCorners val=\"0\"/>\
         <c:chart>{title}<c:plotArea><c:layout/>{plot}{x_axis}{y_axis}</c:plotArea>\
         {legend}<c:plotVisOnly val=\"1\"/></c:chart>\
         <c:spPr><a:solidFill><a:srgbClr val=\"FFFFFF\"/></a:solidFill><a:ln><a:noFill/></a:ln></c:spPr>\
         <c:txPr>{text_style}</c:txPr>\
         </c:chartSpace>",
        x_axis = axis("catAx", 1, 2, "b", &chart.x_label, 0),
        y_axis = axis("valAx", 2, 1, "l", &chart.y_label, -5_400_000),
    ))
}

fn bar_chart(chart: &Chart, gap: u32) -> Option<String> {
    let n = categories(chart).len();
    if n == 0 {
        return None;
    }
    let cats = categories(chart);
    let mut s = String::from(
        "<c:barChart><c:barDir val=\"col\"/><c:grouping val=\"clustered\"/><c:varyColors val=\"0\"/>",
    );
    for (i, series) in chart.series.iter().enumerate() {
        s.push_str(&series_open(i, &series_name(series, i), ink(chart.theme, i)));
        s.push_str(&str_lit(&cats));
        s.push_str("<c:val>");
        s.push_str(&num_lit(&pad(&series.y, n)));
        s.push_str("</c:val></c:ser>");
    }
    s.push_str(&format!(
        "<c:gapWidth val=\"{gap}\"/><c:overlap val=\"0\"/><c:axId val=\"1\"/><c:axId val=\"2\"/></c:barChart>"
    ));
    Some(s)
}

fn line_chart(chart: &Chart) -> Option<String> {
    let cats = categories(chart);
    let n = cats.len().max(chart.series.iter().map(|s| s.y.len()).max().unwrap_or(0));
    if n == 0 {
        return None;
    }
    let cats = if cats.len() == n {
        cats
    } else {
        (0..n).map(|i| format!("{}", i + 1)).collect()
    };
    let symbols = ["circle", "diamond", "triangle", "square", "x"];
    let mut s = String::from(
        "<c:lineChart><c:grouping val=\"standard\"/><c:varyColors val=\"0\"/>",
    );
    for (i, series) in chart.series.iter().enumerate() {
        s.push_str(&series_open(i, &series_name(series, i), ink(chart.theme, i)));
        s.push_str(&format!(
            "<c:marker><c:symbol val=\"{}\"/><c:size val=\"6\"/>{}</c:marker>",
            symbols[i % symbols.len()],
            sp_pr(if i == 0 { "FFFFFF" } else { ink(chart.theme, i) }, edge(chart.theme, i))
        ));
        s.push_str(&str_lit(&cats));
        s.push_str("<c:val>");
        s.push_str(&num_lit(&pad(&series.y, n)));
        s.push_str("</c:val></c:ser>");
    }
    s.push_str("<c:smooth val=\"0\"/><c:axId val=\"1\"/><c:axId val=\"2\"/></c:lineChart>");
    Some(s)
}

fn scatter_chart(chart: &Chart) -> Option<String> {
    if chart.series.iter().all(|s| s.x.is_empty() || s.y.is_empty()) {
        return None;
    }
    let mut s = String::from(
        "<c:scatterChart><c:scatterStyle val=\"marker\"/><c:varyColors val=\"0\"/>",
    );
    for (i, series) in chart.series.iter().enumerate() {
        let n = series.x.len().min(series.y.len());
        if n == 0 {
            continue;
        }
        s.push_str(&series_open(i, &series_name(series, i), ink(chart.theme, i)));
        let fill = if i == 0 { "FFFFFF" } else { ink(chart.theme, i) };
        s.push_str(&format!(
            "<c:marker><c:symbol val=\"circle\"/><c:size val=\"6\"/>{}</c:marker>",
            sp_pr(fill, edge(chart.theme, i))
        ));
        s.push_str("<c:xVal>");
        s.push_str(&num_lit(&series.x[..n]));
        s.push_str("</c:xVal><c:yVal>");
        s.push_str(&num_lit(&series.y[..n]));
        s.push_str("</c:yVal></c:ser>");
    }
    s.push_str("<c:axId val=\"1\"/><c:axId val=\"2\"/></c:scatterChart>");
    Some(s)
}

fn series_open(i: usize, name: &str, fill: &str) -> String {
    format!(
        "<c:ser><c:idx val=\"{i}\"/><c:order val=\"{i}\"/>\
         <c:tx><c:v>{}</c:v></c:tx>{}",
        xml(name),
        sp_pr(fill, "000000")
    )
}

fn sp_pr(fill: &str, line: &str) -> String {
    format!(
        "<c:spPr><a:solidFill><a:srgbClr val=\"{fill}\"/></a:solidFill>\
         <a:ln w=\"9525\"><a:solidFill><a:srgbClr val=\"{line}\"/></a:solidFill></a:ln></c:spPr>"
    )
}

fn categories(chart: &Chart) -> Vec<String> {
    let labelled = chart.series.iter().find(|s| !s.x_labels.is_empty());
    if let Some(s) = labelled {
        return s.x_labels.clone();
    }
    let n = chart.series.iter().map(|s| s.y.len()).max().unwrap_or(0);
    (0..n).map(|i| format!("{}", i + 1)).collect()
}

fn series_name(series: &crate::output::ChartSeries, i: usize) -> String {
    if series.name.is_empty() {
        format!("Series {}", i + 1)
    } else {
        series.name.clone()
    }
}

fn pad(values: &[f64], n: usize) -> Vec<f64> {
    let mut v = values.to_vec();
    v.resize(n, 0.0);
    v
}

fn num_lit(values: &[f64]) -> String {
    let mut s = format!(
        "<c:numLit><c:formatCode>General</c:formatCode><c:ptCount val=\"{}\"/>",
        values.len()
    );
    for (i, v) in values.iter().enumerate() {
        s.push_str(&format!("<c:pt idx=\"{i}\"><c:v>{v}</c:v></c:pt>"));
    }
    s.push_str("</c:numLit>");
    s
}

fn str_lit(values: &[String]) -> String {
    let mut s = format!("<c:cat><c:strLit><c:ptCount val=\"{}\"/>", values.len());
    for (i, v) in values.iter().enumerate() {
        s.push_str(&format!(
            "<c:pt idx=\"{i}\"><c:v>{}</c:v></c:pt>",
            xml(v)
        ));
    }
    s.push_str("</c:strLit></c:cat>");
    s
}

/// An axis with outward ticks, a black spine, and no grid.
fn axis(tag: &str, id: u32, cross: u32, pos: &str, title: &str, rot: i32) -> String {
    let title = if title.is_empty() {
        String::new()
    } else {
        format!("<c:title>{}<c:overlay val=\"0\"/></c:title>", rich(title, false, 1000, rot))
    };
    format!(
        "<c:{tag}><c:axId val=\"{id}\"/><c:scaling><c:orientation val=\"minMax\"/></c:scaling>\
         <c:delete val=\"0\"/><c:axPos val=\"{pos}\"/>\
         <c:majorTickMark val=\"out\"/><c:minorTickMark val=\"none\"/>\
         <c:tickLblPos val=\"nextTo\"/><c:crossAx val=\"{cross}\"/><c:crosses val=\"autoZero\"/>\
         <c:auto val=\"1\"/><c:lblOffset val=\"100\"/>\
         <c:numFmt formatCode=\"General\" sourceLinked=\"0\"/>\
         <c:spPr><a:ln w=\"9525\"><a:solidFill><a:srgbClr val=\"000000\"/></a:solidFill></a:ln></c:spPr>\
         {title}{tx}</c:{tag}>",
        tx = latin(1000, false, 0)
    )
}

fn rich(text: &str, italic: bool, sz: u32, rot: i32) -> String {
    format!(
        "<c:tx><c:rich>{}<a:bodyPr rot=\"{rot}\"/><a:lstStyle/>\
         <a:p><a:pPr><a:defRPr/></a:pPr>\
         <a:r><a:rPr lang=\"en-US\" sz=\"{sz}\" i=\"{}\"><a:latin typeface=\"Calibri\"/></a:rPr>\
         <a:t>{}</a:t></a:r></a:p></c:rich></c:tx>",
        "",
        if italic { 1 } else { 0 },
        xml(text)
    )
}

fn latin(sz: u32, italic: bool, rot: i32) -> String {
    format!(
        "<c:txPr><a:bodyPr rot=\"{rot}\"/><a:lstStyle/>\
         <a:p><a:pPr><a:defRPr sz=\"{sz}\" i=\"{}\"><a:latin typeface=\"Calibri\"/></a:defRPr></a:pPr>\
         <a:endParaRPr lang=\"en-US\"/></a:p></c:txPr>",
        if italic { 1 } else { 0 }
    )
}

fn show_legend(chart: &Chart) -> bool {
    match chart.legend {
        LegendPlace::None => false,
        LegendPlace::Bottom => !chart.series.is_empty(),
        LegendPlace::Auto => chart.series.len() > 1,
    }
}

fn ink(theme: FigureTheme, i: usize) -> &'static str {
    match theme {
        FigureTheme::Apa => ["000000", "595959", "969696", "C8C8C8", "E6E6E6"][i % 5],
        FigureTheme::Color => ["0072B2", "E69F00", "009E73", "D55E00", "56B4E9"][i % 5],
    }
}

fn edge(theme: FigureTheme, i: usize) -> &'static str {
    match theme {
        FigureTheme::Apa => "000000",
        FigureTheme::Color => ink(theme, i),
    }
}

fn xml(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

const CONTENT_TYPES: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\">\
<Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/>\
<Default Extension=\"xml\" ContentType=\"application/xml\"/>\
<Override PartName=\"/clipboard/charts/chart1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.drawingml.chart+xml\"/>\
<Override PartName=\"/clipboard/drawings/drawing1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.drawing+xml\"/>\
</Types>";

const RELS: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\
<Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/drawing\" Target=\"clipboard/drawings/drawing1.xml\"/>\
</Relationships>";

const DRAWING_RELS: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\
<Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart\" Target=\"../charts/chart1.xml\"/>\
</Relationships>";

const DRAWING: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<a:graphic xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" \
xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" \
xmlns:c=\"http://schemas.openxmlformats.org/drawingml/2006/chart\">\
<a:graphicData uri=\"http://schemas.openxmlformats.org/drawingml/2006/chart\">\
<c:chart r:id=\"rId1\"/>\
</a:graphicData></a:graphic>";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::ChartSeries;

    fn bar() -> Chart {
        Chart {
            kind: ChartKind::Bar,
            title: "Counts <group>".into(),
            x_label: "Group".into(),
            y_label: "Count".into(),
            series: vec![ChartSeries {
                name: "Count".into(),
                x: vec![0.0, 1.0],
                y: vec![4.0, 7.0],
                quartiles: None,
                x_labels: vec!["A".into(), "B".into()],
            }],
            reference_line: None,
            reference_label: String::new(),
            ..Chart::default()
        }
    }

    #[test]
    fn a_bar_chart_is_an_office_chart_without_gridlines() {
        let bytes = gvml(&bar()).unwrap();
        assert_eq!(&bytes[..2], b"PK");
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
        let mut xml = String::new();
        std::io::Read::read_to_string(&mut archive.by_name("clipboard/charts/chart1.xml").unwrap(), &mut xml).unwrap();
        assert!(xml.contains("<c:barChart>"));
        assert!(xml.contains("Counts &lt;group&gt;"));
        assert!(xml.contains("val=\"out\""));
        assert!(!xml.contains("majorGridlines"));
        assert!(xml.contains("Calibri"));
    }

    #[test]
    fn a_histogram_has_no_gap_and_a_box_plot_is_not_a_chart() {
        let mut hist = bar();
        hist.kind = ChartKind::Histogram;
        let bytes = gvml(&hist).unwrap();
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
        let mut xml = String::new();
        std::io::Read::read_to_string(&mut archive.by_name("clipboard/charts/chart1.xml").unwrap(), &mut xml).unwrap();
        assert!(xml.contains("gapWidth val=\"0\""));
        hist.kind = ChartKind::BoxPlot;
        assert!(gvml(&hist).is_none());
    }
}
