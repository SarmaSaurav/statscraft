//! Drawing the SVG icon set inside egui.
//!
//! egui has no SVG renderer of its own, so each icon is rasterised once with
//! `egui_extras` (which uses resvg) and uploaded to the GPU as a texture.
//! The cache lives in egui's memory, keyed by icon name, so an icon is
//! decoded once per session rather than on every frame.

use crate::theme::{self, Icon};
use eframe::egui;

/// A cache entry: the GPU texture an icon was uploaded to.
#[derive(Clone)]
struct IconEntry {
    texture: egui::TextureId,
}

/// All decoded icons, in one egui memory slot.
#[derive(Default, Clone)]
struct IconCache {
    entries: std::collections::HashMap<&'static str, IconEntry>,
}

/// The egui memory slot the cache lives in.
fn cache_id() -> egui::Id {
    egui::Id::new("statscraft_icons")
}

/// Get (and cache) the texture for an icon.
fn icon_texture(ctx: &egui::Context, icon: Icon) -> Option<egui::TextureId> {
    let name = icon.label();

    // Fast path: already decoded.
    if let Some(entry) = ctx.memory(|m| m.data.get_temp::<IconCache>(cache_id())) {
        if let Some(e) = entry.entries.get(name) {
            return Some(e.texture);
        }
    }

    // Decode the SVG at a generous resolution so it stays crisp when scaled.
    let img = egui_extras::image::load_svg_bytes_with_size(
        icon.bytes(),
        Some(egui::load::SizeHint::Height(64)),
    )
    .ok()?;

    let tex_id = {
        let tex_manager = ctx.tex_manager();
        let mut tex_manager = tex_manager.write();
        tex_manager.alloc(
            format!("statscraft/icon/{}", name),
            egui::ImageData::Color(std::sync::Arc::new(img)),
            egui::TextureOptions::LINEAR,
        )
    };

    ctx.memory_mut(|m| {
        let mut cache = m.data.get_temp_mut_or_default::<IconCache>(cache_id());
        cache.entries.insert(name, IconEntry { texture: tex_id });
    });

    Some(tex_id)
}

/// A ribbon button: a large line icon with its label underneath.
///
/// The icon is a monochrome line drawing tinted from the active theme,
/// so it reads cleanly in both the light and the dark palette. The label
/// is part of the button, not just the tooltip, which is what makes the
/// ribbon understandable at a glance.
pub fn icon_button(ui: &mut egui::Ui, icon: Icon, label: &str, tip: &str) -> egui::Response {
    icon_button_enabled(ui, icon, label, tip, true)
}

/// A ribbon button that can be disabled (rendered dimmed, not clickable).
pub fn icon_button_enabled(
    ui: &mut egui::Ui,
    icon: Icon,
    label: &str,
    tip: &str,
    enabled: bool,
) -> egui::Response {
    let size = egui::vec2(theme::TOOLBAR_BTN_W, theme::TOOLBAR_H);
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());

    let hovered = resp.hovered() && enabled;
    let pressed = resp.is_pointer_button_down_on() && enabled;

    let p = ui.painter();
    if pressed {
        p.rect_filled(
            rect.shrink(2.0),
            theme::RADIUS_SM,
            theme::accent().gamma_multiply(0.22),
        );
    } else if hovered {
        p.rect_filled(rect.shrink(2.0), theme::RADIUS_SM, theme::accent_soft());
    }

    let active_colour = if hovered || pressed {
        theme::accent()
    } else {
        theme::text_muted()
    };
    let tint = if enabled {
        active_colour
    } else {
        theme::text_faint().gamma_multiply(0.55)
    };

    let ctx = ui.ctx().clone();
    if let Some(tex) = icon_texture(&ctx, icon) {
        let inner = egui::Rect::from_center_size(
            egui::pos2(rect.center().x, rect.top() + 22.0),
            egui::vec2(theme::ICON_SIZE, theme::ICON_SIZE),
        );
        p.image(
            tex,
            inner,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            tint,
        );
    }

    let label_colour = if enabled { active_colour } else { tint };
    p.text(
        egui::pos2(rect.center().x, rect.bottom() - 9.0),
        egui::Align2::CENTER_CENTER,
        label,
        theme::medium(10.5),
        label_colour,
    );

    let resp = resp.on_hover_text(format!("{}\n{}", label, tip));
    if enabled {
        resp
    } else {
        // Swallow the click so a disabled button does nothing.
        resp.interact(egui::Sense::hover())
    }
}

/// A vertical separator between groups of toolbar buttons.
pub fn separator(ui: &mut egui::Ui) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(11.0, theme::TOOLBAR_H), egui::Sense::hover());
    let p = ui.painter();
    p.rect_filled(
        egui::Rect::from_center_size(rect.center(), egui::vec2(1.0, theme::TOOLBAR_H - 16.0)),
        0.5,
        theme::border(),
    );
}

/// Paint an icon at a given rect, tinted — used inside menus and headings.
pub fn paint_icon(
    p: &egui::Painter,
    ctx: &egui::Context,
    icon: Icon,
    rect: egui::Rect,
    tint: egui::Color32,
) {
    if let Some(tex) = icon_texture(ctx, icon) {
        p.image(
            tex,
            rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            tint,
        );
    }
}

/// A small icon-only button, used in dense grids and table rows.
///
/// The icon is tinted with `colour`, a hover wash uses the same colour at
/// low alpha, and the tooltip carries the full description.
pub fn small_icon_button(
    ui: &mut egui::Ui,
    icon: Icon,
    tip: &str,
    colour: egui::Color32,
) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(22.0, 22.0), egui::Sense::click());
    let p = ui.painter();
    if resp.hovered() {
        p.rect_filled(rect, theme::RADIUS_SM, colour.gamma_multiply(0.14));
    }
    let tint = if resp.hovered() {
        colour
    } else {
        colour.gamma_multiply(0.85)
    };
    let ctx = ui.ctx().clone();
    paint_icon(
        p,
        &ctx,
        icon,
        egui::Rect::from_center_size(rect.center(), egui::vec2(15.0, 15.0)),
        tint,
    );
    resp.on_hover_text(tip)
}

/// A menu entry with a leading icon, used across the menu bar.
pub fn menu_item(ui: &mut egui::Ui, icon: Icon, label: &str) -> egui::Response {
    menu_item_enabled(ui, icon, label, true)
}

/// A menu entry that cannot be chosen.
///
/// A command that is not implemented yet is shown greyed out and says so
/// when the reader hovers over it, rather than pretending to work and
/// reporting a failure when it is used.
pub fn menu_item_disabled(ui: &mut egui::Ui, icon: Icon, label: &str, planned: &str) -> egui::Response {
    let resp = menu_item_enabled(ui, icon, label, false);
    resp.on_hover_text(format!(
        "{} is planned for a later release ({}).",
        label, planned
    ))
}

/// A menu entry with a leading icon, enabled or not.
pub fn menu_item_enabled(
    ui: &mut egui::Ui,
    icon: Icon,
    label: &str,
    enabled: bool,
) -> egui::Response {
    let ctx = ui.ctx().clone();
    let (rect, resp) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), 24.0),
        egui::Sense::click(),
    );

    let p = ui.painter();
    if resp.hovered() && enabled {
        p.rect_filled(rect, theme::RADIUS_SM, theme::accent_soft());
    }

    let icon_rect = egui::Rect::from_center_size(
        egui::pos2(rect.left() + 14.0, rect.center().y),
        egui::vec2(16.0, 16.0),
    );
    // The line glyph is tinted from the theme so it stays legible on
    // both the light and the dark palette.
    let text_color = if !enabled {
        theme::text_faint()
    } else if resp.hovered() {
        theme::accent()
    } else {
        theme::text()
    };
    paint_icon(p, &ctx, icon, icon_rect, text_color);
    let galley = p.layout_no_wrap(label.to_owned(), theme::medium(12.5), text_color);
    p.galley(
        egui::pos2(
            rect.left() + 28.0,
            rect.center().y - galley.size().y / 2.0,
        ),
        galley,
        text_color,
    );

    if enabled {
        resp
    } else {
        // Swallow the click so a disabled entry does nothing.
        resp.interact(egui::Sense::hover())
    }
}
