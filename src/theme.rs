//! StatsCraft visual theme: palette, typography, spacing and the icon set.
//!
//! The old UI was SPSS-grey: 1990s flat greys, the system UI font, and
//! hand-painted pictograms. This module is the single place where the modern
//! look lives, so the rest of the app only refers to named colours.

use eframe::egui;

// ---------------------------------------------------------------------------
// Palette
// ---------------------------------------------------------------------------

/// Brand accent — the blue used for focus, selection and primary actions.
// ---------------------------------------------------------------------------
// The palette
//
// The colours live in one struct rather than in constants, because there
// are two of them: the light workbench theme and the dark one, which
// follows the Dracula palette. A theme is chosen at start-up and can be
// switched from the View menu.
//
// The accessors below read the current palette, so a widget writes
// `theme::surface()` and gets the colour of whichever theme is active.
// ---------------------------------------------------------------------------

/// The colours of one theme.
#[derive(Clone, Copy)]
pub struct Palette {
    /// The accent: the primary action colour.
    pub accent: egui::Color32,
    /// A darker accent tint, for the hover states.
    pub accent_soft: egui::Color32,
    /// The secondary accent, for the ready state.
    pub teal: egui::Color32,
    /// The warm accent, for the warnings.
    pub amber: egui::Color32,
    /// The window background.
    pub surface: egui::Color32,
    /// The card and toolbar surface.
    pub surface_alt: egui::Color32,
    /// The data-grid cell background.
    pub cell: egui::Color32,
    /// The data-grid zebra stripe.
    pub cell_alt: egui::Color32,
    /// The column-header band.
    pub header: egui::Color32,
    /// The selected cell or row.
    pub selected: egui::Color32,
    /// The primary text.
    pub text: egui::Color32,
    /// The secondary text.
    pub text_muted: egui::Color32,
    /// The tertiary text.
    pub text_faint: egui::Color32,
    /// The text on an accent-filled surface.
    pub text_on_accent: egui::Color32,
    /// The hairline borders.
    pub border: egui::Color32,
    /// The stronger border, for the focused elements.
    pub border_strong: egui::Color32,
}

/// The light workbench theme.
const LIGHT: Palette = Palette {
    accent: egui::Color32::from_rgb(0x2F, 0x6F, 0xED),
    accent_soft: egui::Color32::from_rgb(0xE8, 0xEF, 0xFD),
    teal: egui::Color32::from_rgb(0x0F, 0xA8, 0x7E),
    amber: egui::Color32::from_rgb(0xE0, 0x8A, 0x0B),
    surface: egui::Color32::from_rgb(0xFA, 0xFB, 0xFD),
    surface_alt: egui::Color32::from_rgb(0xF4, 0xF6, 0xFA),
    cell: egui::Color32::from_rgb(0xFF, 0xFF, 0xFF),
    cell_alt: egui::Color32::from_rgb(0xF7, 0xF9, 0xFC),
    header: egui::Color32::from_rgb(0xEE, 0xF2, 0xF8),
    selected: egui::Color32::from_rgb(0xE4, 0xEE, 0xFF),
    text: egui::Color32::from_rgb(0x1A, 0x1F, 0x2B),
    text_muted: egui::Color32::from_rgb(0x5B, 0x66, 0x78),
    text_faint: egui::Color32::from_rgb(0x93, 0x9E, 0xB0),
    text_on_accent: egui::Color32::WHITE,
    border: egui::Color32::from_rgb(0xDD, 0xE3, 0xED),
    border_strong: egui::Color32::from_rgb(0xC2, 0xCC, 0xDC),
};

/// The dark theme, after the Dracula palette.
///
/// Dracula's foreground is very close to white, which is too harsh in a
/// data grid that is read for hours, so the text is a shade off it and
/// the surfaces are the background and the panel colour.
const DARK: Palette = Palette {
    accent: egui::Color32::from_rgb(0xBD, 0x93, 0xF9),
    accent_soft: egui::Color32::from_rgb(0x39, 0x33, 0x50),
    teal: egui::Color32::from_rgb(0x50, 0xFA, 0x7B),
    amber: egui::Color32::from_rgb(0xFF, 0xB8, 0x6C),
    surface: egui::Color32::from_rgb(0x21, 0x23, 0x2D),
    surface_alt: egui::Color32::from_rgb(0x2A, 0x2C, 0x38),
    cell: egui::Color32::from_rgb(0x25, 0x27, 0x33),
    cell_alt: egui::Color32::from_rgb(0x2A, 0x2C, 0x38),
    header: egui::Color32::from_rgb(0x34, 0x37, 0x45),
    selected: egui::Color32::from_rgb(0x3D, 0x36, 0x5C),
    text: egui::Color32::from_rgb(0xE6, 0xE6, 0xE0),
    text_muted: egui::Color32::from_rgb(0xA0, 0xA6, 0xB8),
    text_faint: egui::Color32::from_rgb(0x62, 0x72, 0xA4),
    text_on_accent: egui::Color32::from_rgb(0x28, 0x2A, 0x36),
    border: egui::Color32::from_rgb(0x3A, 0x3D, 0x4B),
    border_strong: egui::Color32::from_rgb(0x50, 0x54, 0x66),
};

/// The palette in use: 0 for the light theme, 1 for the dark one.
static CURRENT: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

/// Whether the dark theme is active.
pub fn is_dark() -> bool {
    CURRENT.load(std::sync::atomic::Ordering::Relaxed) == 1
}

/// Switch between the light and the dark theme.
///
/// The change is picked up by the widgets on the next frame; the caller
/// re-applies the style.
pub fn set_dark(dark: bool) {
    CURRENT.store(
        if dark { 1 } else { 0 },
        std::sync::atomic::Ordering::Relaxed,
    );
}

/// The palette in use.
pub fn palette() -> Palette {
    if is_dark() {
        DARK
    } else {
        LIGHT
    }
}

macro_rules! colour_accessors {
    ($($name:ident),* $(,)?) => {
        $(
            /// The colour of the current theme.
            pub fn $name() -> egui::Color32 {
                palette().$name
            }
        )*
    };
}

colour_accessors!(
    accent,
    accent_soft,
    teal,
    amber,
    surface,
    surface_alt,
    cell,
    cell_alt,
    header,
    selected,
    text,
    text_muted,
    text_faint,
    text_on_accent,
    border,
    border_strong,
);

// ---------------------------------------------------------------------------
// Metrics
// ---------------------------------------------------------------------------

pub const RADIUS: f32 = 6.0;
pub const RADIUS_SM: f32 = 4.0;
/// Height of a ribbon button, including the icon and its label.
pub const TOOLBAR_H: f32 = 62.0;
/// Width of one ribbon button.
pub const TOOLBAR_BTN_W: f32 = 60.0;
/// The icon inside a ribbon button.
pub const ICON_SIZE: f32 = 32.0;

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

/// Apply the StatsCraft look to an egui context.
///
/// The `dark` flag selects the theme. The style is built from the
/// palette, so the widgets and the style always agree.
pub fn apply_style(ctx: &egui::Context, dark: bool) {
    set_dark(dark);
    let p = palette();
    let mut style = (*ctx.style()).clone();

    style.spacing.item_spacing = egui::vec2(8.0, 5.0);
    style.spacing.button_padding = egui::vec2(10.0, 6.0);
    style.spacing.menu_margin = egui::Margin::same(6.0);
    style.spacing.window_margin = egui::Margin::same(12.0);
    style.spacing.interact_size.y = 22.0;

    style.text_styles.insert(egui::TextStyle::Small, body(11.0));
    style.text_styles.insert(egui::TextStyle::Body, body(13.0));
    style.text_styles.insert(egui::TextStyle::Monospace, font(FONT_MONO, 12.5));
    style.text_styles.insert(egui::TextStyle::Button, medium(13.0));
    style.text_styles.insert(egui::TextStyle::Heading, semibold(19.0));

    let mut v = if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    v.panel_fill = p.surface;
    v.window_fill = p.surface;
    v.extreme_bg_color = p.surface_alt;
    v.faint_bg_color = p.surface_alt;
    v.code_bg_color = p.surface_alt;
    v.override_text_color = None;

    v.selection.bg_fill = p.selected;
    v.selection.stroke = egui::Stroke::new(1.0, p.accent);

    v.hyperlink_color = p.accent;
    v.warn_fg_color = p.amber;
    v.error_fg_color = if dark {
        egui::Color32::from_rgb(0xFF, 0x55, 0x55)
    } else {
        egui::Color32::from_rgb(0xD9, 0x3A, 0x3A)
    };

    v.window_rounding = egui::Rounding::same(RADIUS);
    v.menu_rounding = egui::Rounding::same(RADIUS);
    v.window_stroke = egui::Stroke::new(1.0, p.border);
    v.window_shadow = egui::Shadow {
        offset: egui::vec2(0.0, 6.0),
        blur: 18.0,
        spread: 0.0,
        color: egui::Color32::from_black_alpha(if dark { 90 } else { 28 }),
    };
    v.popup_shadow = v.window_shadow;

    v.widgets.noninteractive.bg_fill = p.surface_alt;
    v.widgets.noninteractive.weak_bg_fill = p.surface;
    v.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, p.border);
    v.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, p.text);
    v.widgets.noninteractive.rounding = egui::Rounding::same(RADIUS_SM);

    v.widgets.inactive.bg_fill = p.surface;
    v.widgets.inactive.weak_bg_fill = p.surface;
    v.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, p.border);
    v.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, p.text);
    v.widgets.inactive.rounding = egui::Rounding::same(RADIUS_SM);

    v.widgets.hovered.bg_fill = p.accent_soft;
    v.widgets.hovered.weak_bg_fill = p.accent_soft;
    v.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, p.accent);
    v.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, p.accent);
    v.widgets.hovered.rounding = egui::Rounding::same(RADIUS_SM);

    v.widgets.active.bg_fill = p.accent;
    v.widgets.active.weak_bg_fill = p.accent;
    v.widgets.active.bg_stroke = egui::Stroke::new(1.0, p.accent);
    v.widgets.active.fg_stroke = egui::Stroke::new(1.0, p.text_on_accent);
    v.widgets.active.rounding = egui::Rounding::same(RADIUS_SM);

    v.widgets.open.bg_fill = p.accent_soft;
    v.widgets.open.weak_bg_fill = p.accent_soft;
    v.widgets.open.bg_stroke = egui::Stroke::new(1.0, p.accent);

    style.visuals = v;
    ctx.set_style(style);
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
        measure_scale => "measure_scale",
        measure_nominal => "measure_nominal",
        data_view => "data_view",
        glm => "glm",
        means => "means",
        explore => "explore",
        delete_case => "delete_case",
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
    /// A ruler — SPSS's measure-level icon for a numeric/scale variable.
    MeasureScale,
    /// A pencil — SPSS's measure-level icon for a string/nominal variable.
    MeasureNominal,
    /// A data grid — the Data View tab.
    DataView,
    /// The general linear model.
    Glm,
    /// Group means.
    Means,
    /// The explore procedure.
    Explore,
    /// A row with a cross through it — deleting a case.
    DeleteCase,
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
            Icon::MeasureScale => icons::measure_scale,
            Icon::MeasureNominal => icons::measure_nominal,
            Icon::DataView => icons::data_view,
            Icon::Glm => icons::glm,
            Icon::Means => icons::means,
            Icon::Explore => icons::explore,
            Icon::DeleteCase => icons::delete_case,
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
            Icon::MeasureScale => "Measure scale",
            Icon::MeasureNominal => "Measure nominal",
            Icon::DataView => "Data View",
            Icon::Glm => "General Linear Model",
            Icon::Means => "Means",
            Icon::Explore => "Explore",
            Icon::DeleteCase => "Delete Cases",
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
            (Icon::DeleteCase, icons::delete_case),
        ];
        for (icon, bytes) in all {
            let svg = String::from_utf8(bytes.to_vec()).expect("icon is utf-8");
            assert!(svg.contains("<svg"), "{} is not an svg", icon.label());
            assert!(svg.contains("<path"), "{} has no path", icon.label());
            assert!(svg.contains("stroke="), "{} is not stroked", icon.label());
        }
    }
}
