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
    egui::Id::new("statecraft_icons")
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
            format!("statecraft/icon/{}", name),
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

/// A flat, modern toolbar button: no chrome until hover, tinted icon.
pub fn icon_button(ui: &mut egui::Ui, icon: Icon, tip: &str) -> egui::Response {
    icon_button_enabled(ui, icon, tip, true)
}

/// A toolbar button that can be disabled (rendered dimmed, not clickable).
pub fn icon_button_enabled(
    ui: &mut egui::Ui,
    icon: Icon,
    tip: &str,
    enabled: bool,
) -> egui::Response {
    let size = egui::vec2(theme::TOOLBAR_H, theme::TOOLBAR_H);
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());

    let hovered = resp.hovered() && enabled;
    let pressed = resp.is_pointer_button_down_on() && enabled;

    let p = ui.painter();
    if pressed {
        p.rect_filled(
            rect.shrink(2.0),
            theme::RADIUS_SM,
            theme::ACCENT.gamma_multiply(0.22),
        );
    } else if hovered {
        p.rect_filled(rect.shrink(2.0), theme::RADIUS_SM, theme::ACCENT_SOFT);
    }

    let tint = if !enabled {
        theme::TEXT_FAINT
    } else if hovered {
        theme::ACCENT
    } else {
        theme::TEXT_MUTED
    };

    let ctx = ui.ctx().clone();
    if let Some(tex) = icon_texture(&ctx, icon) {
        let inner = egui::Rect::from_center_size(
            rect.center(),
            egui::vec2(theme::ICON_SIZE, theme::ICON_SIZE),
        );
        p.image(
            tex,
            inner,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            tint,
        );
    }

    let resp = resp.on_hover_text(format!("{}\n{}", icon.label(), tip));
    if enabled {
        resp
    } else {
        // Swallow the click so a disabled button does nothing.
        resp.interact(egui::Sense::hover())
    }
}

/// A vertical separator between groups of toolbar buttons.
pub fn separator(ui: &mut egui::Ui) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(9.0, 24.0), egui::Sense::hover());
    let p = ui.painter();
    p.rect_filled(
        egui::Rect::from_center_size(rect.center(), egui::vec2(1.0, 20.0)),
        0.5,
        theme::BORDER,
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

/// A menu entry with a leading icon, used across the menu bar.
pub fn menu_item(ui: &mut egui::Ui, icon: Icon, label: &str) -> egui::Response {
    let ctx = ui.ctx().clone();
    let (rect, resp) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), 24.0),
        egui::Sense::click(),
    );

    let p = ui.painter();
    if resp.hovered() {
        p.rect_filled(rect, theme::RADIUS_SM, theme::ACCENT_SOFT);
    }

    let icon_rect = egui::Rect::from_center_size(
        egui::pos2(rect.left() + 14.0, rect.center().y),
        egui::vec2(16.0, 16.0),
    );
    let tint = if resp.hovered() {
        theme::ACCENT
    } else {
        theme::TEXT_MUTED
    };
    paint_icon(p, &ctx, icon, icon_rect, tint);

    let text_color = if resp.hovered() {
        theme::ACCENT
    } else {
        theme::TEXT
    };
    let galley = p.layout_no_wrap(label.to_owned(), theme::medium(12.5), text_color);
    p.galley(
        egui::pos2(
            rect.left() + 28.0,
            rect.center().y - galley.size().y / 2.0,
        ),
        galley,
        text_color,
    );

    resp
}
