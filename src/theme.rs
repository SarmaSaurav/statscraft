//! StateCraft visual theme: palette, typography, spacing and the icon set.
//!
//! The old UI was SPSS-grey: 1990s flat greys, the system UI font, and
//! hand-painted pictograms. This module is the single place where the modern
//! look lives, so the rest of the app only refers to named colours.

use eframe::egui;

// ---------------------------------------------------------------------------
// Palette
// ---------------------------------------------------------------------------

/// Brand accent — the blue used for focus, selection and primary actions.
pub const ACCENT: egui::Color32 = egui::Color32::from_rgb(0x2F, 0x6F, 0xED);
/// Lighter accent tint for hover states.
pub const ACCENT_SOFT: egui::Color32 = egui::Color32::from_rgb(0xE8, 0xEF, 0xFD);
/// Secondary accent used for positive/ready states.
pub const TEAL: egui::Color32 = egui::Color32::from_rgb(0x0F, 0xA8, 0x7E);
/// Warm accent used for warnings / unsigned state.
pub const AMBER: egui::Color32 = egui::Color32::from_rgb(0xE0, 0x8A, 0x0B);

/// Window / panel background.
pub const SURFACE: egui::Color32 = egui::Color32::from_rgb(0xFA, 0xFB, 0xFD);
/// Card and toolbar surface — a touch cooler than `SURFACE`.
pub const SURFACE_ALT: egui::Color32 = egui::Color32::from_rgb(0xF4, 0xF6, 0xFA);
/// Data-grid cell background.
pub const CELL: egui::Color32 = egui::Color32::from_rgb(0xFF, 0xFF, 0xFF);
/// Data-grid zebra stripe.
pub const CELL_ALT: egui::Color32 = egui::Color32::from_rgb(0xF7, 0xF9, 0xFC);
/// Column-header band.
pub const HEADER: egui::Color32 = egui::Color32::from_rgb(0xEE, 0xF2, 0xF8);
/// Selected cell / row.
pub const SELECTED: egui::Color32 = egui::Color32::from_rgb(0xE4, 0xEE, 0xFF);

/// Primary text.
pub const TEXT: egui::Color32 = egui::Color32::from_rgb(0x1A, 0x1F, 0x2B);
/// Secondary text (labels, captions).
pub const TEXT_MUTED: egui::Color32 = egui::Color32::from_rgb(0x5B, 0x66, 0x78);
/// Tertiary text (placeholders, disabled).
pub const TEXT_FAINT: egui::Color32 = egui::Color32::from_rgb(0x93, 0x9E, 0xB0);
/// Text on an accent-filled surface.
pub const TEXT_ON_ACCENT: egui::Color32 = egui::Color32::WHITE;

/// Hairline borders.
pub const BORDER: egui::Color32 = egui::Color32::from_rgb(0xDD, 0xE3, 0xED);
/// Slightly stronger border for focused/selected elements.
pub const BORDER_STRONG: egui::Color32 = egui::Color32::from_rgb(0xC2, 0xCC, 0xDC);

// ---------------------------------------------------------------------------
// Metrics
// ---------------------------------------------------------------------------

pub const RADIUS: f32 = 6.0;
pub const RADIUS_SM: f32 = 4.0;
pub const TOOLBAR_H: f32 = 44.0;
pub const ICON_SIZE: f32 = 20.0;

/// Row height of the data grid.
pub const ROW_H: f32 = 22.0;
/// Width of the row-number gutter.
pub const ROWNUM_W: f32 = 52.0;
/// Width of one data column.
pub const COL_W: f32 = 92.0;
/// Gap between cells.
pub const GAP: f32 = 1.0;
/// Minimum number of columns drawn beyond the real ones.
pub const MIN_COLS: usize = 20;

// ---------------------------------------------------------------------------
// Fonts
// ---------------------------------------------------------------------------

/// Font family names registered with egui. Each family is a separate
/// `FontDefinitions` entry so egui can fall back through them in order.
pub const FONT_REGULAR: &str = "Inter";
pub const FONT_MEDIUM: &str = "Inter Medium";
pub const FONT_SEMIBOLD: &str = "Inter SemiBold";
pub const FONT_BOLD: &str = "Inter Bold";
/// Kept as a named family so code blocks stay monospaced.
pub const FONT_MONO: &str = "Inter Mono";

/// Register the bundled Inter family (and a monospace fallback) with egui.
///
/// The fonts are embedded in the binary with `include_bytes!`, so the app
/// looks the same on every machine and needs no system font.
///
/// Each weight is registered as its own *named* family. `RichText::strong`
/// works by asking the font stack for the bold face, so the bold face has to
/// be reachable from the proportional stack — it is, as the last entry.
pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    fonts.font_data.insert(
        FONT_REGULAR.to_owned(),
        egui::FontData::from_static(include_bytes!("../assets/fonts/Inter-Regular.ttf")),
    );
    fonts.font_data.insert(
        FONT_MEDIUM.to_owned(),
        egui::FontData::from_static(include_bytes!("../assets/fonts/Inter-Medium.ttf")),
    );
    fonts.font_data.insert(
        FONT_SEMIBOLD.to_owned(),
        egui::FontData::from_static(include_bytes!("../assets/fonts/Inter-SemiBold.ttf")),
    );
    fonts.font_data.insert(
        FONT_BOLD.to_owned(),
        egui::FontData::from_static(include_bytes!("../assets/fonts/Inter-Bold.ttf")),
    );

    // Every named family resolves to its own face, so `font(FONT_BOLD, n)`
    // really is bold.
    for name in [FONT_REGULAR, FONT_MEDIUM, FONT_SEMIBOLD, FONT_BOLD] {
        fonts
            .families
            .entry(egui::FontFamily::Name(name.into()))
            .or_default()
            .insert(0, name.to_owned());
    }

    // The named monospace family points at the same face; the syntax editor
    // uses the fixed-width advance of the regular face.
    fonts
        .families
        .entry(egui::FontFamily::Name(FONT_MONO.into()))
        .or_default()
        .insert(0, FONT_REGULAR.to_owned());

    // Proportional stack: Regular first (the everyday face), then Medium,
    // SemiBold and Bold, so `RichText::strong` finds a real bold face.
    let prop = fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default();
    prop.insert(0, FONT_REGULAR.to_owned());
    prop.insert(1, FONT_MEDIUM.to_owned());
    prop.insert(2, FONT_SEMIBOLD.to_owned());
    prop.insert(3, FONT_BOLD.to_owned());

    // Monospace: the Inter Regular face is close enough and keeps the
    // syntax editor aligned without shipping a second typeface.
    let mono = fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default();
    mono.insert(0, FONT_REGULAR.to_owned());

    ctx.set_fonts(fonts);
}

/// Font id for a given family, size and weight.
pub fn font(family: &str, size: f32) -> egui::FontId {
    egui::FontId::new(size, egui::FontFamily::Name(family.into()))
}

/// The app's regular text.
pub fn body(size: f32) -> egui::FontId {
    font(FONT_REGULAR, size)
}

/// Medium weight — for toolbars, labels and table headers.
pub fn medium(size: f32) -> egui::FontId {
    font(FONT_MEDIUM, size)
}

/// Semi-bold — for headings and the active tab.
pub fn semibold(size: f32) -> egui::FontId {
    font(FONT_SEMIBOLD, size)
}

// ---------------------------------------------------------------------------
// Style
// ---------------------------------------------------------------------------

/// Apply the StateCraft look to an egui context.
///
/// `dark` selects between the light workbench theme and a dark variant used
/// by the Output Viewer.
pub fn apply_style(ctx: &egui::Context, dark: bool) {
    let mut style = (*ctx.style()).clone();

    style.spacing.item_spacing = egui::vec2(8.0, 5.0);
    style.spacing.button_padding = egui::vec2(10.0, 6.0);
    style.spacing.menu_margin = egui::Margin::same(6.0);
    style.spacing.window_margin = egui::Margin::same(12.0);
    style.spacing.interact_size.y = 22.0;

    style.text_styles.insert(
        egui::TextStyle::Small,
        body(11.0),
    );
    style.text_styles.insert(
        egui::TextStyle::Body,
        body(13.0),
    );
    style.text_styles.insert(
        egui::TextStyle::Monospace,
        font(FONT_MONO, 12.5),
    );
    style.text_styles.insert(
        egui::TextStyle::Button,
        medium(13.0),
    );
    style.text_styles.insert(
        egui::TextStyle::Heading,
        semibold(19.0),
    );

    if dark {
        apply_dark_visuals(&mut style);
    } else {
        apply_light_visuals(&mut style);
    }

    ctx.set_style(style);
}

fn apply_light_visuals(style: &mut egui::Style) {
    let mut v = egui::Visuals::light();
    v.panel_fill = SURFACE;
    v.window_fill = SURFACE;
    v.extreme_bg_color = SURFACE_ALT;
    v.faint_bg_color = SURFACE_ALT;
    v.code_bg_color = SURFACE_ALT;
    v.override_text_color = None;

    v.selection.bg_fill = ACCENT_SOFT;
    v.selection.stroke = egui::Stroke::new(1.0, ACCENT);

    v.hyperlink_color = ACCENT;
    v.warn_fg_color = AMBER;
    v.error_fg_color = egui::Color32::from_rgb(0xD9, 0x3A, 0x3A);

    v.window_rounding = egui::Rounding::same(RADIUS);
    v.menu_rounding = egui::Rounding::same(RADIUS);
    v.window_stroke = egui::Stroke::new(1.0, BORDER);
    v.window_shadow = egui::Shadow {
        offset: egui::vec2(0.0, 6.0),
        blur: 18.0,
        spread: 0.0,
        color: egui::Color32::from_black_alpha(28),
    };
    v.popup_shadow = v.window_shadow;

    v.widgets.noninteractive.bg_fill = SURFACE_ALT;
    v.widgets.noninteractive.weak_bg_fill = SURFACE;
    v.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, BORDER);
    v.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, TEXT);
    v.widgets.noninteractive.rounding = egui::Rounding::same(RADIUS_SM);

    v.widgets.inactive.bg_fill = SURFACE;
    v.widgets.inactive.weak_bg_fill = SURFACE;
    v.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, BORDER);
    v.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, TEXT);
    v.widgets.inactive.rounding = egui::Rounding::same(RADIUS_SM);

    v.widgets.hovered.bg_fill = ACCENT_SOFT;
    v.widgets.hovered.weak_bg_fill = ACCENT_SOFT;
    v.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, ACCENT);
    v.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, ACCENT);
    v.widgets.hovered.rounding = egui::Rounding::same(RADIUS_SM);

    v.widgets.active.bg_fill = ACCENT;
    v.widgets.active.weak_bg_fill = ACCENT;
    v.widgets.active.bg_stroke = egui::Stroke::new(1.0, ACCENT);
    v.widgets.active.fg_stroke = egui::Stroke::new(1.0, TEXT_ON_ACCENT);
    v.widgets.active.rounding = egui::Rounding::same(RADIUS_SM);

    v.widgets.open.bg_fill = ACCENT_SOFT;
    v.widgets.open.weak_bg_fill = ACCENT_SOFT;
    v.widgets.open.bg_stroke = egui::Stroke::new(1.0, ACCENT);

    style.visuals = v;
}

fn apply_dark_visuals(style: &mut egui::Style) {
    let mut v = egui::Visuals::dark();
    v.panel_fill = egui::Color32::from_rgb(0x1B, 0x1F, 0x27);
    v.window_fill = egui::Color32::from_rgb(0x1B, 0x1F, 0x27);
    v.extreme_bg_color = egui::Color32::from_rgb(0x14, 0x18, 0x1F);
    v.faint_bg_color = egui::Color32::from_rgb(0x22, 0x27, 0x31);
    v.override_text_color = None;

    v.selection.bg_fill = egui::Color32::from_rgb(0x24, 0x3A, 0x66);
    v.selection.stroke = egui::Stroke::new(1.0, ACCENT);

    v.hyperlink_color = egui::Color32::from_rgb(0x7A, 0xA8, 0xFF);
    v.window_rounding = egui::Rounding::same(RADIUS);
    v.menu_rounding = egui::Rounding::same(RADIUS);
    v.window_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(0x33, 0x3B, 0x4A));
    v.window_shadow = egui::Shadow {
        offset: egui::vec2(0.0, 8.0),
        blur: 24.0,
        spread: 0.0,
        color: egui::Color32::from_black_alpha(90),
    };
    v.popup_shadow = v.window_shadow;

    let fg = egui::Color32::from_rgb(0xE6, 0xEA, 0xF2);
    v.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(0x22, 0x27, 0x31);
    v.widgets.noninteractive.weak_bg_fill = egui::Color32::from_rgb(0x1B, 0x1F, 0x27);
    v.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(0x33, 0x3B, 0x4A));
    v.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, fg);
    v.widgets.noninteractive.rounding = egui::Rounding::same(RADIUS_SM);

    v.widgets.inactive.bg_fill = egui::Color32::from_rgb(0x26, 0x2C, 0x37);
    v.widgets.inactive.weak_bg_fill = egui::Color32::from_rgb(0x1B, 0x1F, 0x27);
    v.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(0x3A, 0x43, 0x54));
    v.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, fg);
    v.widgets.inactive.rounding = egui::Rounding::same(RADIUS_SM);

    v.widgets.hovered.bg_fill = egui::Color32::from_rgb(0x2B, 0x36, 0x4C);
    v.widgets.hovered.weak_bg_fill = egui::Color32::from_rgb(0x24, 0x2C, 0x3C);
    v.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, ACCENT);
    v.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, fg);
    v.widgets.hovered.rounding = egui::Rounding::same(RADIUS_SM);

    v.widgets.active.bg_fill = ACCENT;
    v.widgets.active.weak_bg_fill = ACCENT;
    v.widgets.active.bg_stroke = egui::Stroke::new(1.0, ACCENT);
    v.widgets.active.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
    v.widgets.active.rounding = egui::Rounding::same(RADIUS_SM);

    v.widgets.open.bg_fill = egui::Color32::from_rgb(0x24, 0x3A, 0x66);
    v.widgets.open.weak_bg_fill = egui::Color32::from_rgb(0x24, 0x3A, 0x66);
    v.widgets.open.bg_stroke = egui::Stroke::new(1.0, ACCENT);

    style.visuals = v;
}

// ---------------------------------------------------------------------------
// Icons
// ---------------------------------------------------------------------------

/// The icon set, embedded in the binary as SVG bytes.
pub mod icons {
    macro_rules! icon_set {
        ($($name:ident => $file:literal),* $(,)?) => {
            $(pub const $name: &[u8] = include_bytes!(concat!("../assets/icons/", $file, ".svg"));)*
        };
    }
    icon_set!(
        open => "open",
        save => "save",
        export => "export",
        print => "print",
        undo => "undo",
        redo => "redo",
        cut => "cut",
        copy => "copy",
        paste => "paste",
        insert_variable => "insert_variable",
        insert_case => "insert_case",
        value_labels => "value_labels",
        find => "find",
        filter => "filter",
        chart => "chart",
        histogram => "histogram",
        regression => "regression",
        crosstabs => "crosstabs",
        describe => "describe",
        correlation => "correlation",
        anova => "anova",
        ttest => "ttest",
        output => "output",
        syntax => "syntax",
        variables => "variables",
        refresh => "refresh",
        settings => "settings",
        check => "check",
    );
}

/// A named icon, so the call sites read as intent rather than as bytes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Icon {
    Open,
    Save,
    Export,
    Print,
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    InsertVariable,
    InsertCase,
    ValueLabels,
    Find,
    Filter,
    Chart,
    Histogram,
    Regression,
    Crosstabs,
    Describe,
    Correlation,
    Anova,
    TTest,
    Output,
    Syntax,
    Variables,
    Refresh,
    Settings,
    Check,
}

impl Icon {
    /// The raw SVG bytes for this icon.
    pub fn bytes(self) -> &'static [u8] {
        match self {
            Icon::Open => icons::open,
            Icon::Save => icons::save,
            Icon::Export => icons::export,
            Icon::Print => icons::print,
            Icon::Undo => icons::undo,
            Icon::Redo => icons::redo,
            Icon::Cut => icons::cut,
            Icon::Copy => icons::copy,
            Icon::Paste => icons::paste,
            Icon::InsertVariable => icons::insert_variable,
            Icon::InsertCase => icons::insert_case,
            Icon::ValueLabels => icons::value_labels,
            Icon::Find => icons::find,
            Icon::Filter => icons::filter,
            Icon::Chart => icons::chart,
            Icon::Histogram => icons::histogram,
            Icon::Regression => icons::regression,
            Icon::Crosstabs => icons::crosstabs,
            Icon::Describe => icons::describe,
            Icon::Correlation => icons::correlation,
            Icon::Anova => icons::anova,
            Icon::TTest => icons::ttest,
            Icon::Output => icons::output,
            Icon::Syntax => icons::syntax,
            Icon::Variables => icons::variables,
            Icon::Refresh => icons::refresh,
            Icon::Settings => icons::settings,
            Icon::Check => icons::check,
        }
    }

    /// A human-readable name, used for the tooltip and the accessible label.
    pub fn label(self) -> &'static str {
        match self {
            Icon::Open => "Open",
            Icon::Save => "Save",
            Icon::Export => "Export",
            Icon::Print => "Print",
            Icon::Undo => "Undo",
            Icon::Redo => "Redo",
            Icon::Cut => "Cut",
            Icon::Copy => "Copy",
            Icon::Paste => "Paste",
            Icon::InsertVariable => "Insert Variable",
            Icon::InsertCase => "Insert Cases",
            Icon::ValueLabels => "Value Labels",
            Icon::Find => "Find",
            Icon::Filter => "Filter",
            Icon::Chart => "Chart",
            Icon::Histogram => "Histogram",
            Icon::Regression => "Regression",
            Icon::Crosstabs => "Crosstabs",
            Icon::Describe => "Descriptives",
            Icon::Correlation => "Correlation",
            Icon::Anova => "ANOVA",
            Icon::TTest => "T-Test",
            Icon::Output => "Output Viewer",
            Icon::Syntax => "Syntax Editor",
            Icon::Variables => "Variable View",
            Icon::Refresh => "Refresh",
            Icon::Settings => "Settings",
            Icon::Check => "Check",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The app must render in Inter, not egui's default Ubuntu-Light.
    ///
    /// A missing font silently falls back, so assert on the *width* of a
    /// word: Inter and Ubuntu-Light measure the same string differently.
    #[test]
    fn inter_font_is_installed_and_used() {
        let ctx = egui::Context::default();
        install_fonts(&ctx);
        // egui only builds its font atlas on the first frame.
        let _ = ctx.run(egui::RawInput::default(), |_| {});

        let word = "Transform";
        let app_width = ctx
            .fonts(|f| {
                f.layout_no_wrap(word.to_owned(), body(13.0), egui::Color32::WHITE)
            })
            .size()
            .x;

        // Measure the same word with the Inter file directly, and with the
        // default font, and require the app's width to sit with Inter.
        //
        // `body()` is the Regular face, so compare against Inter Regular.
        let inter_ctx = egui::Context::default();
        let mut defs = egui::FontDefinitions::default();
        defs.font_data.insert(
            "Inter".to_owned(),
            egui::FontData::from_static(include_bytes!("../assets/fonts/Inter-Regular.ttf")),
        );
        defs.families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "Inter".to_owned());
        inter_ctx.set_fonts(defs);
        let _ = inter_ctx.run(egui::RawInput::default(), |_| {});
        let inter_width = inter_ctx
            .fonts(|f| {
                f.layout_no_wrap(
                    word.to_owned(),
                    egui::FontId::proportional(13.0),
                    egui::Color32::WHITE,
                )
            })
            .size()
            .x;

        let default_ctx = egui::Context::default();
        let _ = default_ctx.run(egui::RawInput::default(), |_| {});
        let default_width = default_ctx
            .fonts(|f| {
                f.layout_no_wrap(
                    word.to_owned(),
                    egui::FontId::proportional(13.0),
                    egui::Color32::WHITE,
                )
            })
            .size()
            .x;

        assert!(
            (app_width - inter_width).abs() < 0.5,
            "app width {app_width} should match Inter {inter_width} (default was {default_width})"
        );
        assert!(
            (app_width - default_width).abs() > 0.5,
            "app width {app_width} should differ from the default font {default_width}"
        );
    }

    /// Every icon in the set must be a real, non-empty SVG.
    #[test]
    fn every_icon_is_present_and_wellformed() {
        let all = [
            (Icon::Open, icons::open),
            (Icon::Save, icons::save),
            (Icon::Export, icons::export),
            (Icon::Print, icons::print),
            (Icon::Undo, icons::undo),
            (Icon::Redo, icons::redo),
            (Icon::Cut, icons::cut),
            (Icon::Copy, icons::copy),
            (Icon::Paste, icons::paste),
            (Icon::InsertVariable, icons::insert_variable),
            (Icon::InsertCase, icons::insert_case),
            (Icon::ValueLabels, icons::value_labels),
            (Icon::Find, icons::find),
            (Icon::Filter, icons::filter),
            (Icon::Chart, icons::chart),
            (Icon::Histogram, icons::histogram),
            (Icon::Regression, icons::regression),
            (Icon::Crosstabs, icons::crosstabs),
            (Icon::Describe, icons::describe),
            (Icon::Correlation, icons::correlation),
            (Icon::Anova, icons::anova),
            (Icon::TTest, icons::ttest),
            (Icon::Output, icons::output),
            (Icon::Syntax, icons::syntax),
            (Icon::Variables, icons::variables),
            (Icon::Refresh, icons::refresh),
            (Icon::Settings, icons::settings),
            (Icon::Check, icons::check),
        ];
        for (icon, bytes) in all {
            let svg = String::from_utf8(bytes.to_vec()).expect("icon is utf-8");
            assert!(svg.contains("<svg"), "{} is not an svg", icon.label());
            assert!(svg.contains("<path"), "{} has no path", icon.label());
            assert!(svg.contains("stroke="), "{} is not stroked", icon.label());
        }
    }
}
