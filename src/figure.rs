//! An APA figure.
//!
//! One layout feeds the screen, the SVG export and the PNG that other
//! programs paste. The figure is black ink on white: an L-shaped pair of
//! axes, ticks pointing out, no grid, and a gray ramp so a series still
//! reads after the figure is printed in black and white. The title sits
//! above the axes, in italic, which is where a journal caption goes.

use crate::output::{Chart, ChartKind, FigureTheme, LegendPlace};

#[derive(Clone, Copy)]
pub struct Rgb(pub u8, pub u8, pub u8);

pub const BLACK: Rgb = Rgb(0, 0, 0);
pub const WHITE: Rgb = Rgb(255, 255, 255);
const FRAME: Rgb = Rgb(255, 255, 255);

/// Five print-safe inks. The first is black; the rest step up the gray ramp.
const APA: [Rgb; 5] = [
    Rgb(0, 0, 0),
    Rgb(90, 90, 90),
    Rgb(150, 150, 150),
    Rgb(200, 200, 200),
    Rgb(230, 230, 230),
];

/// Wong's colorblind-safe series colors. The axes stay black.
const COLOR: [Rgb; 5] = [
    Rgb(0, 114, 178),
    Rgb(230, 159, 0),
    Rgb(0, 158, 115),
    Rgb(213, 94, 0),
    Rgb(86, 180, 233),
];

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Anchor {
    Left,
    Center,
    Right,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Marker {
    Circle,
    Square,
    Triangle,
}

#[derive(Clone)]
pub enum Mark {
    Rect {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        fill: Rgb,
        stroke: Option<f32>,
    },
    Line {
        x0: f32,
        y0: f32,
        x1: f32,
        y1: f32,
        width: f32,
        dashed: bool,
    },
    Marker {
        kind: Marker,
        x: f32,
        y: f32,
        r: f32,
        fill: Option<Rgb>,
        stroke: Rgb,
    },
    Text {
        x: f32,
        y: f32,
        text: String,
        size: f32,
        anchor: Anchor,
        /// Radians, counter-clockwise. The y-axis title is -pi/2.
        angle: f32,
    },
}

pub struct Figure {
    pub width: f32,
    pub height: f32,
    pub marks: Vec<Mark>,
}

pub fn layout(chart: &Chart, width: f32, height: f32) -> Figure {
    let w = width.max(280.0);
    let h = height.max(200.0);
    let mut marks = Vec::new();
    marks.push(Mark::Rect {
        x: 0.0,
        y: 0.0,
        w,
        h,
        fill: FRAME,
        stroke: None,
    });

    let title_band = if chart.title.is_empty() { 4.0 } else { 26.0 };
    if !chart.title.is_empty() {
        marks.push(Mark::Text {
            x: 8.0,
            y: 16.0,
            text: chart.title.clone(),
            size: 13.0,
            anchor: Anchor::Left,
            angle: 0.0,
        });
    }

    let legend = show_legend(chart);
    let caption = !chart.caption.is_empty();
    let caption_h = if caption { 16.0 } else { 0.0 };
    let left = 64.0;
    let right = w - 18.0;
    let top = title_band + 8.0;
    let bottom = h - caption_h - if legend { 58.0 } else { 40.0 };
    let plot_w = (right - left).max(1.0);
    let plot_h = (bottom - top).max(1.0);

    let (min_x, max_x, min_y, max_y) = ranges(chart);
    let x_of = |x: f64| left + ((x - min_x) / (max_x - min_x) * plot_w as f64) as f32;
    let y_of = |y: f64| bottom - ((y - min_y) / (max_y - min_y) * plot_h as f64) as f32;

    // Y-axis title, reading upward.
    if !chart.y_label.is_empty() {
        marks.push(Mark::Text {
            x: 14.0,
            y: (top + bottom) / 2.0,
            text: chart.y_label.clone(),
            size: 11.0,
            anchor: Anchor::Center,
            angle: -std::f32::consts::FRAC_PI_2,
        });
    }
    if !chart.x_label.is_empty() {
        marks.push(Mark::Text {
            x: (left + right) / 2.0,
            y: h - 8.0,
            text: chart.x_label.clone(),
            size: 11.0,
            anchor: Anchor::Center,
            angle: 0.0,
        });
    }

    // L-shaped axes. No top spine, no right spine, no grid.
    marks.push(Mark::Line {
        x0: left,
        y0: top,
        x1: left,
        y1: bottom,
        width: 1.25,
        dashed: false,
    });
    marks.push(Mark::Line {
        x0: left,
        y0: bottom,
        x1: right,
        y1: bottom,
        width: 1.25,
        dashed: false,
    });

    let y_step = nice_step(max_y - min_y, 4);
    let mut yv = (min_y / y_step).floor() * y_step;
    if yv < min_y - y_step * 0.01 {
        yv += y_step;
    }
    let mut guard = 0;
    while yv <= max_y + y_step * 0.01 && guard < 8 {
        if yv >= min_y - y_step * 0.01 {
            let py = y_of(yv);
            marks.push(Mark::Line {
                x0: left - 4.0,
                y0: py,
                x1: left,
                y1: py,
                width: 1.0,
                dashed: false,
            });
            marks.push(Mark::Text {
                x: left - 6.0,
                y: py,
                text: fmt_tick(yv, y_step),
                size: 10.0,
                anchor: Anchor::Right,
                angle: 0.0,
            });
        }
        yv += y_step;
        guard += 1;
    }

    match chart.kind {
        ChartKind::Bar | ChartKind::Histogram => draw_bars(chart, &mut marks, left, bottom, plot_w, y_of, min_y),
        ChartKind::Scatter => draw_points(chart, &mut marks, x_of, y_of, false),
        ChartKind::Line => draw_points(chart, &mut marks, x_of, y_of, true),
        ChartKind::BoxPlot => draw_boxes(chart, &mut marks, left, bottom, plot_w, y_of),
    }

    if matches!(chart.kind, ChartKind::Scatter | ChartKind::Line) {
        let x_step = nice_step(max_x - min_x, 4);
        let mut xv = (min_x / x_step).floor() * x_step;
        if xv < min_x - x_step * 0.01 {
            xv += x_step;
        }
        let mut guard = 0;
        while xv <= max_x + x_step * 0.01 && guard < 8 {
            if xv >= min_x - x_step * 0.01 {
                let px = x_of(xv);
                marks.push(Mark::Line {
                    x0: px,
                    y0: bottom,
                    x1: px,
                    y1: bottom + 4.0,
                    width: 1.0,
                    dashed: false,
                });
                marks.push(Mark::Text {
                    x: px,
                    y: bottom + 8.0,
                    text: fmt_tick(xv, x_step),
                    size: 10.0,
                    anchor: Anchor::Center,
                    angle: 0.0,
                });
            }
            xv += x_step;
            guard += 1;
        }
    }

    if let Some(value) = chart.reference_line {
        if value >= min_y && value <= max_y {
            let py = y_of(value);
            marks.push(Mark::Line {
                x0: left,
                y0: py,
                x1: right,
                y1: py,
                width: 1.0,
                dashed: true,
            });
            if !chart.reference_label.is_empty() {
                marks.push(Mark::Text {
                    x: right - 2.0,
                    y: py - 4.0,
                    text: chart.reference_label.clone(),
                    size: 10.0,
                    anchor: Anchor::Right,
                    angle: 0.0,
                });
            }
        }
    }

    if legend {
        let mut x = left;
        let y = h - caption_h - if chart.x_label.is_empty() { 16.0 } else { 26.0 };
        for (i, series) in chart.series.iter().enumerate() {
            let ink = series_ink(chart, i);
            marks.push(Mark::Rect {
                x,
                y: y - 5.0,
                w: 10.0,
                h: 10.0,
                fill: ink,
                stroke: Some(1.0),
            });
            let name = if series.name.is_empty() {
                format!("Series {}", i + 1)
            } else {
                series.name.clone()
            };
            marks.push(Mark::Text {
                x: x + 14.0,
                y,
                text: name,
                size: 10.0,
                anchor: Anchor::Left,
                angle: 0.0,
            });
            x += 28.0 + series.name.chars().count() as f32 * 6.2;
        }
    }

    if caption {
        marks.push(Mark::Text {
            x: left,
            y: h - 8.0,
            text: chart.caption.clone(),
            size: 10.0,
            anchor: Anchor::Left,
            angle: 0.0,
        });
    }

    Figure {
        width: w,
        height: h,
        marks,
    }
}

fn show_legend(chart: &Chart) -> bool {
    match chart.legend {
        LegendPlace::None => false,
        LegendPlace::Bottom => !chart.series.is_empty() && chart.kind != ChartKind::BoxPlot,
        LegendPlace::Auto => chart.series.len() > 1 && chart.kind != ChartKind::BoxPlot,
    }
}

fn series_ink(chart: &Chart, i: usize) -> Rgb {
    let palette: &[Rgb] = match chart.theme {
        FigureTheme::Apa => &APA,
        FigureTheme::Color => &COLOR,
    };
    palette[i % palette.len()]
}

fn draw_bars(
    chart: &Chart,
    marks: &mut Vec<Mark>,
    left: f32,
    bottom: f32,
    plot_w: f32,
    y_of: impl Fn(f64) -> f32,
    min_y: f64,
) {
    let n = chart
        .series
        .iter()
        .map(|s| s.y.len().max(s.x_labels.len()))
        .max()
        .unwrap_or(0)
        .max(1);
    let nser = chart.series.len().max(1);
    let slot = plot_w / n as f32;
    let touching = chart.kind == ChartKind::Histogram;
    let inner = if touching { slot } else { slot * 0.72 };
    let bar_w = inner / nser as f32;
    let labels = chart
        .series
        .iter()
        .find(|s| !s.x_labels.is_empty())
        .map(|s| s.x_labels.clone())
        .unwrap_or_default();
    let base = y_of(min_y.min(0.0));
    for i in 0..n {
        let cluster = left + i as f32 * slot + (slot - inner) / 2.0;
        for (s, series) in chart.series.iter().enumerate() {
            let Some(y) = series.y.get(i).copied() else {
                continue;
            };
            let ink = series_ink(chart, s);
            let top = y_of(y);
            let x = cluster + s as f32 * bar_w;
            let (y0, h) = if top <= base {
                (top, (base - top).max(0.5))
            } else {
                (base, (top - base).max(0.5))
            };
            marks.push(Mark::Rect {
                x,
                y: y0,
                w: bar_w.max(0.5),
                h,
                fill: ink,
                stroke: Some(0.75),
            });
        }
        if let Some(label) = labels.get(i) {
            marks.push(Mark::Text {
                x: left + (i as f32 + 0.5) * slot,
                y: bottom + 8.0,
                text: label.clone(),
                size: 10.0,
                anchor: Anchor::Center,
                angle: 0.0,
            });
        }
    }
}

fn draw_points(
    chart: &Chart,
    marks: &mut Vec<Mark>,
    x_of: impl Fn(f64) -> f32,
    y_of: impl Fn(f64) -> f32,
    connect: bool,
) {
    for (s, series) in chart.series.iter().enumerate() {
        let ink = series_ink(chart, s);
        let marker = match s % 3 {
            0 => Marker::Circle,
            1 => Marker::Square,
            _ => Marker::Triangle,
        };
        let pts: Vec<(f32, f32)> = series
            .x
            .iter()
            .zip(series.y.iter())
            .map(|(&x, &y)| (x_of(x), y_of(y)))
            .collect();
        if connect {
            for w in pts.windows(2) {
                marks.push(Mark::Line {
                    x0: w[0].0,
                    y0: w[0].1,
                    x1: w[1].0,
                    y1: w[1].1,
                    width: 1.5,
                    dashed: false,
                });
            }
        }
        let open = s == 0;
        for (x, y) in pts {
            marks.push(Mark::Marker {
                kind: marker,
                x,
                y,
                r: 3.5,
                fill: if open { Some(WHITE) } else { Some(ink) },
                stroke: if chart.theme == FigureTheme::Apa { BLACK } else { ink },
            });
        }
    }
}

fn draw_boxes(
    chart: &Chart,
    marks: &mut Vec<Mark>,
    left: f32,
    bottom: f32,
    plot_w: f32,
    y_of: impl Fn(f64) -> f32,
) {
    let n = chart.series.len().max(1);
    let slot = plot_w / n as f32;
    let box_w = (slot * 0.45).clamp(12.0, 36.0);
    for (i, series) in chart.series.iter().enumerate() {
        let Some((min, q1, median, q3)) = series.quartiles else {
            continue;
        };
        // `y` holds the maximum when the builder stored one. The tuple
        // is (minimum, first quartile, median, third quartile).
        let max = series.y.last().copied().unwrap_or(q3);
        let cx = left + (i as f32 + 0.5) * slot;
        let y_min = y_of(min);
        let y_q1 = y_of(q1);
        let y_med = y_of(median);
        let y_q3 = y_of(q3);
        let y_max = y_of(max);
        for (y_a, y_b) in [(y_min, y_q1), (y_q3, y_max)] {
            marks.push(Mark::Line {
                x0: cx,
                y0: y_a,
                x1: cx,
                y1: y_b,
                width: 1.0,
                dashed: false,
            });
        }
        for y in [y_min, y_max] {
            marks.push(Mark::Line {
                x0: cx - box_w * 0.35,
                y0: y,
                x1: cx + box_w * 0.35,
                y1: y,
                width: 1.0,
                dashed: false,
            });
        }
        let top = y_q1.min(y_q3);
        marks.push(Mark::Rect {
            x: cx - box_w / 2.0,
            y: top,
            w: box_w,
            h: (y_q1 - y_q3).abs().max(1.0),
            fill: WHITE,
            stroke: Some(1.25),
        });
        marks.push(Mark::Line {
            x0: cx - box_w / 2.0,
            y0: y_med,
            x1: cx + box_w / 2.0,
            y1: y_med,
            width: 1.75,
            dashed: false,
        });
        let label = series
            .x_labels
            .first()
            .cloned()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| series.name.clone());
        if !label.is_empty() {
            marks.push(Mark::Text {
                x: cx,
                y: bottom + 8.0,
                text: label,
                size: 10.0,
                anchor: Anchor::Center,
                angle: 0.0,
            });
        }
    }
}

fn ranges(chart: &Chart) -> (f64, f64, f64, f64) {
    let mut min_y = f64::MAX;
    let mut max_y = f64::MIN;
    let mut min_x = f64::MAX;
    let mut max_x = f64::MIN;
    let mut any = false;
    for s in &chart.series {
        for &y in &s.y {
            any = true;
            min_y = min_y.min(y);
            max_y = max_y.max(y);
        }
        if matches!(chart.kind, ChartKind::Scatter | ChartKind::Line) {
            for &x in &s.x {
                min_x = min_x.min(x);
                max_x = max_x.max(x);
            }
        }
        if let Some((min, q1, median, q3)) = s.quartiles {
            any = true;
            for v in [min, q1, median, q3] {
                min_y = min_y.min(v);
                max_y = max_y.max(v);
            }
        }
    }
    if !any {
        return (0.0, 1.0, 0.0, 1.0);
    }
    if matches!(chart.kind, ChartKind::Bar | ChartKind::Histogram | ChartKind::BoxPlot) {
        if min_y > 0.0 {
            min_y = 0.0;
        }
        if max_y < 0.0 {
            max_y = 0.0;
        }
    }
    if max_y <= min_y {
        max_y = min_y + 1.0;
    }
    if !matches!(chart.kind, ChartKind::Scatter | ChartKind::Line) {
        min_x = 0.0;
        max_x = 1.0;
    } else if max_x <= min_x {
        min_x -= 0.5;
        max_x += 0.5;
    } else {
        let pad = (max_x - min_x) * 0.04;
        min_x -= pad;
        max_x += pad;
    }
    if matches!(chart.kind, ChartKind::Scatter | ChartKind::Line) {
        let pad = (max_y - min_y) * 0.06;
        min_y -= pad;
        max_y += pad;
    } else if max_y > 0.0 {
        max_y += (max_y - min_y) * 0.04;
    }
    (min_x, max_x, min_y, max_y)
}

fn nice_step(span: f64, ticks: i32) -> f64 {
    let span = span.abs().max(1e-9);
    let rough = span / ticks.max(1) as f64;
    let pow = 10f64.powf(rough.log10().floor());
    let err = rough / pow;
    let nice = if err <= 1.0 {
        1.0
    } else if err <= 2.0 {
        2.0
    } else if err <= 5.0 {
        5.0
    } else {
        10.0
    };
    nice * pow
}

fn fmt_tick(v: f64, step: f64) -> String {
    if step >= 1.0 && (v - v.round()).abs() < 1e-6 {
        format!("{}", v.round() as i64)
    } else if step >= 0.1 {
        format!("{:.1}", v)
    } else {
        format!("{:.2}", v)
    }
}

pub fn to_svg(chart: &Chart, width: f32, height: f32) -> String {
    let fig = layout(chart, width, height);
    let mut s = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{:.0}\" height=\"{:.0}\" viewBox=\"0 0 {:.0} {:.0}\">",
        fig.width, fig.height, fig.width, fig.height
    );
    for mark in &fig.marks {
        match mark {
            Mark::Rect { x, y, w, h, fill, stroke } => {
                s.push_str(&format!(
                    "<rect x=\"{x:.1}\" y=\"{y:.1}\" width=\"{w:.1}\" height=\"{h:.1}\" fill=\"{}\" {}/>",
                    rgb(*fill),
                    stroke
                        .map(|sw| format!("stroke=\"#000000\" stroke-width=\"{sw:.2}\""))
                        .unwrap_or_else(|| "stroke=\"none\"".into())
                ));
            }
            Mark::Line { x0, y0, x1, y1, width, dashed } => {
                let dash = if *dashed { " stroke-dasharray=\"5 4\"" } else { "" };
                s.push_str(&format!(
                    "<line x1=\"{x0:.1}\" y1=\"{y0:.1}\" x2=\"{x1:.1}\" y2=\"{y1:.1}\" stroke=\"#000000\" stroke-width=\"{width:.2}\"{dash}/>"
                ));
            }
            Mark::Marker { kind, x, y, r, fill, stroke } => {
                let fill = fill.map(rgb).unwrap_or_else(|| "none".into());
                let edge = rgb(*stroke);
                match kind {
                    Marker::Circle => s.push_str(&format!(
                        "<circle cx=\"{x:.1}\" cy=\"{y:.1}\" r=\"{r:.1}\" fill=\"{fill}\" stroke=\"{edge}\" stroke-width=\"1.1\"/>"
                    )),
                    Marker::Square => s.push_str(&format!(
                        "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" fill=\"{fill}\" stroke=\"{edge}\" stroke-width=\"1.1\"/>",
                        x - r, y - r, r * 2.0, r * 2.0
                    )),
                    Marker::Triangle => s.push_str(&format!(
                        "<polygon points=\"{:.1},{:.1} {:.1},{:.1} {:.1},{:.1}\" fill=\"{fill}\" stroke=\"{edge}\" stroke-width=\"1.1\"/>",
                        x, y - r, x - r, y + r * 0.8, x + r, y + r * 0.8
                    )),
                }
            }
            Mark::Text { x, y, text, size, anchor, angle } => {
                let anchor = match anchor {
                    Anchor::Left => "start",
                    Anchor::Center => "middle",
                    Anchor::Right => "end",
                };
                let rot = if angle.abs() > 0.01 {
                    format!(" transform=\"rotate({:.1} {:.1} {:.1})\"", angle.to_degrees(), x, y)
                } else {
                    String::new()
                };
                let style = if *size >= 12.5 { " font-style=\"italic\"" } else { "" };
                s.push_str(&format!(
                    "<text x=\"{x:.1}\" y=\"{y:.1}\" font-family=\"Calibri,Arial,sans-serif\" font-size=\"{size:.1}\" text-anchor=\"{anchor}\" fill=\"#000000\"{style}{rot}>{}</text>",
                    xml(&text)
                ));
            }
        }
    }
    s.push_str("</svg>");
    s
}

fn rgb(c: Rgb) -> String {
    format!("#{:02x}{:02x}{:02x}", c.0, c.1, c.2)
}

fn xml(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Raster of the figure, top-down RGBA.
pub fn raster(chart: &Chart, width: u32, height: u32) -> (u32, u32, Vec<u8>) {
    let fig = layout(chart, width as f32, height as f32);
    let w = fig.width.ceil() as usize;
    let h = fig.height.ceil() as usize;
    let mut px = vec![255u8; w * h * 4];
    let font = apa_font();
    for mark in &fig.marks {
        match mark {
            Mark::Rect { x, y, w: rw, h: rh, fill, stroke } => {
                fill_rect(&mut px, w, h, *x, *y, *rw, *rh, *fill);
                if stroke.is_some() {
                    stroke_rect(&mut px, w, h, *x, *y, *rw, *rh, BLACK);
                }
            }
            Mark::Line { x0, y0, x1, y1, width: sw, dashed } => {
                if *dashed {
                    dash_line(&mut px, w, h, *x0, *y0, *x1, *y1);
                } else {
                    line(&mut px, w, h, *x0, *y0, *x1, *y1, sw.max(1.0), BLACK);
                }
            }
            Mark::Marker { kind, x, y, r, fill, .. } => {
                let ink = fill.unwrap_or(WHITE);
                match kind {
                    Marker::Circle => disc(&mut px, w, h, *x, *y, *r, ink, true),
                    Marker::Square => {
                        fill_rect(&mut px, w, h, x - r, y - r, r * 2.0, r * 2.0, ink);
                        stroke_rect(&mut px, w, h, x - r, y - r, r * 2.0, r * 2.0, BLACK);
                    }
                    Marker::Triangle => disc(&mut px, w, h, *x, *y, *r, ink, true),
                }
            }
            Mark::Text { x, y, text, size, anchor, angle } => {
                blit_text(&mut px, w, h, font, text, *size, *x, *y, *anchor, *angle);
            }
        }
    }
    (w as u32, h as u32, px)
}

pub fn png_bytes(chart: &Chart, width: u32, height: u32) -> Result<Vec<u8>, String> {
    let (w, h, px) = raster(chart, width, height);
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, w, h);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
    writer.write_image_data(&px).map_err(|e| e.to_string())?;
    drop(writer);
    Ok(out)
}

fn apa_font() -> &'static fontdue::Font {
    use std::sync::OnceLock;
    static FONT: OnceLock<fontdue::Font> = OnceLock::new();
    FONT.get_or_init(|| {
        fontdue::Font::from_bytes(
            include_bytes!("../assets/fonts/IBMPlexSans-Regular.ttf") as &[u8],
            fontdue::FontSettings::default(),
        )
        .expect("IBM Plex Sans is embedded")
    })
}

fn blit_text(
    px: &mut [u8],
    w: usize,
    h: usize,
    font: &fontdue::Font,
    text: &str,
    size: f32,
    x: f32,
    y: f32,
    anchor: Anchor,
    angle: f32,
) {
    let px_size = size.max(8.0);
    let mut glyphs = Vec::new();
    let mut total = 0i32;
    for ch in text.chars() {
        let (metrics, bitmap) = font.rasterize(ch, px_size);
        total += metrics.advance_width.round() as i32;
        glyphs.push((metrics, bitmap));
    }
    let mut pen = match anchor {
        Anchor::Left => x,
        Anchor::Center => x - total as f32 / 2.0,
        Anchor::Right => x - total as f32,
    };
    let rotate = angle.abs() > 0.01;
    for (metrics, bitmap) in glyphs {
        let gw = metrics.width;
        let gh = metrics.height;
        let gx = pen + metrics.xmin as f32;
        let gy = y - metrics.height as f32 - metrics.ymin as f32;
        for row in 0..gh {
            for col in 0..gw {
                let cov = bitmap[row * gw + col];
                if cov < 8 {
                    continue;
                }
                let (dx, dy) = if rotate {
                    // -90°: the pen runs along y, glyphs stand upright-turned.
                    let ox = (col as f32) - gw as f32 / 2.0;
                    let oy = (row as f32) - gh as f32 / 2.0;
                    (x + oy, pen + ox)
                } else {
                    (gx + col as f32, gy + row as f32)
                };
                blend(px, w, h, dx as i32, dy as i32, cov);
            }
        }
        pen += metrics.advance_width;
    }
}

fn blend(px: &mut [u8], w: usize, h: usize, x: i32, y: i32, cov: u8) {
    if x < 0 || y < 0 || x as usize >= w || y as usize >= h {
        return;
    }
    let i = (y as usize * w + x as usize) * 4;
    let a = cov as u16;
    for c in 0..3 {
        let dst = px[i + c] as u16;
        px[i + c] = ((dst * (255 - a) + 0 * a) / 255) as u8;
    }
    px[i + 3] = 255;
}

fn fill_rect(px: &mut [u8], w: usize, h: usize, x: f32, y: f32, rw: f32, rh: f32, ink: Rgb) {
    let x0 = x.floor().max(0.0) as usize;
    let y0 = y.floor().max(0.0) as usize;
    let x1 = ((x + rw).ceil() as usize).min(w);
    let y1 = ((y + rh).ceil() as usize).min(h);
    for yy in y0..y1 {
        for xx in x0..x1 {
            put(px, w, xx, yy, ink);
        }
    }
}

fn stroke_rect(px: &mut [u8], w: usize, h: usize, x: f32, y: f32, rw: f32, rh: f32, ink: Rgb) {
    line(px, w, h, x, y, x + rw, y, 1.0, ink);
    line(px, w, h, x, y + rh, x + rw, y + rh, 1.0, ink);
    line(px, w, h, x, y, x, y + rh, 1.0, ink);
    line(px, w, h, x + rw, y, x + rw, y + rh, 1.0, ink);
}

fn put(px: &mut [u8], w: usize, x: usize, y: usize, ink: Rgb) {
    let i = (y * w + x) * 4;
    if i + 3 < px.len() {
        px[i] = ink.0;
        px[i + 1] = ink.1;
        px[i + 2] = ink.2;
        px[i + 3] = 255;
    }
}

fn line(px: &mut [u8], w: usize, h: usize, x0: f32, y0: f32, x1: f32, y1: f32, width: f32, ink: Rgb) {
    let n = ((x1 - x0).abs().max((y1 - y0).abs()) as i32).max(1);
    let r = (width * 0.5).max(0.5);
    for i in 0..=n {
        let t = i as f32 / n as f32;
        let x = x0 + (x1 - x0) * t;
        let y = y0 + (y1 - y0) * t;
        disc(px, w, h, x, y, r, ink, false);
    }
}

fn dash_line(px: &mut [u8], w: usize, h: usize, x0: f32, y0: f32, x1: f32, y1: f32) {
    let len = ((x1 - x0).powi(2) + (y1 - y0).powi(2)).sqrt().max(1.0);
    let steps = len as i32;
    for i in 0..=steps {
        if (i / 5) % 2 == 1 {
            continue;
        }
        let t = i as f32 / steps as f32;
        disc(px, w, h, x0 + (x1 - x0) * t, y0 + (y1 - y0) * t, 0.7, BLACK, false);
    }
}

fn disc(px: &mut [u8], w: usize, h: usize, cx: f32, cy: f32, r: f32, ink: Rgb, ring: bool) {
    let r2 = r * r;
    let inner = (r - 1.1).max(0.0).powi(2);
    let x0 = (cx - r - 1.0).floor().max(0.0) as usize;
    let y0 = (cy - r - 1.0).floor().max(0.0) as usize;
    let x1 = ((cx + r + 1.0).ceil() as usize).min(w);
    let y1 = ((cy + r + 1.0).ceil() as usize).min(h);
    for y in y0..y1 {
        for x in x0..x1 {
            let d = (x as f32 + 0.5 - cx).powi(2) + (y as f32 + 0.5 - cy).powi(2);
            if d <= r2 {
                let color = if ring && d >= inner { BLACK } else { ink };
                put(px, w, x, y, color);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::{Chart, ChartKind, ChartSeries, FigureTheme, LegendPlace};

    fn bar() -> Chart {
        Chart {
            kind: ChartKind::Bar,
            title: "Counts by group".into(),
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
    fn an_apa_figure_has_no_grid_and_black_ink() {
        let svg = to_svg(&bar(), 640.0, 400.0);
        assert!(svg.contains("Counts by group"));
        assert!(svg.contains("font-style=\"italic\""));
        assert!(!svg.contains("#dde3ed"), "the old grid colour is gone");
        assert!(!svg.contains("#2f6fed"), "the theme accent is not the bar");
        assert!(svg.contains("fill=\"#000000\""));
        // The two spines are the only full-width rules; a grid would add several.
        let rules = svg.matches("stroke=\"#000000\"").count();
        assert!(rules < 12, "grid lines leaked into the figure: {rules}");
    }

    #[test]
    fn a_color_theme_and_a_caption_reach_the_figure() {
        let mut chart = bar();
        chart.series.push(ChartSeries {
            name: "Treated".into(),
            x: vec![0.0, 1.0],
            y: vec![1.0, 2.0],
            quartiles: None,
            x_labels: Vec::new(),
        });
        chart.theme = FigureTheme::Color;
        chart.caption = "Figure 1. Counts in two groups.".into();
        chart.legend = LegendPlace::None;
        let hidden = to_svg(&chart, 640.0, 400.0);
        assert!(hidden.contains("fill=\"#0072b2\""));
        assert!(hidden.contains("Figure 1. Counts in two groups."));
        assert!(!hidden.contains("Treated"));
        chart.legend = LegendPlace::Bottom;
        assert!(to_svg(&chart, 640.0, 400.0).contains("Treated"));
    }

    #[test]
    fn the_png_is_a_png() {
        let bytes = png_bytes(&bar(), 320, 200).unwrap();
        assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
    }
}
