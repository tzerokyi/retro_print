//! View styling and geometry for the RetroPrint editor.
//!
//! UI geometry lives here; image processing and file operations do not.

use eframe::egui::{
    self, Align2, Color32, FontId, Painter, Rect, RichText, Sense, Stroke, StrokeKind,
    TextureHandle, Vec2, pos2, vec2,
};

pub const BACKGROUND: Color32 = Color32::from_rgb(17, 20, 24);
pub const PANEL: Color32 = Color32::from_rgb(25, 29, 34);
pub const SURFACE: Color32 = Color32::from_rgb(29, 34, 40);
pub const BORDER: Color32 = Color32::from_rgb(58, 64, 72);
pub const TEXT: Color32 = Color32::from_rgb(241, 234, 224);
pub const SUBTLE: Color32 = Color32::from_rgb(157, 160, 166);
pub const ACCENT: Color32 = Color32::from_rgb(240, 157, 96);
pub const ERROR: Color32 = Color32::from_rgb(244, 121, 114);

/// Licensed Lucide icons embedded at compile time; all controls use one icon language.
/// The upstream SVGs are kept in `assets/icons`, with a white stroke so egui
/// can tint them consistently across inactive, hovered, and accent states.
#[derive(Clone, Copy)]
pub enum Icon {
    Open,
    Export,
    Reset,
    Reseed,
    Drop,
    Success,
    Error,
    Controls,
    Palette,
}

impl Icon {
    fn asset(self) -> (&'static str, &'static [u8]) {
        match self {
            Self::Open => (
                "bytes://retroprint/image-up.svg",
                include_bytes!("../assets/icons/image-up.svg"),
            ),
            Self::Export => (
                "bytes://retroprint/download.svg",
                include_bytes!("../assets/icons/download.svg"),
            ),
            Self::Reset => (
                "bytes://retroprint/rotate-ccw.svg",
                include_bytes!("../assets/icons/rotate-ccw.svg"),
            ),
            Self::Reseed => (
                "bytes://retroprint/shuffle.svg",
                include_bytes!("../assets/icons/shuffle.svg"),
            ),
            Self::Drop => (
                "bytes://retroprint/upload.svg",
                include_bytes!("../assets/icons/upload.svg"),
            ),
            Self::Success => (
                "bytes://retroprint/circle-check.svg",
                include_bytes!("../assets/icons/circle-check.svg"),
            ),
            Self::Error => (
                "bytes://retroprint/circle-alert.svg",
                include_bytes!("../assets/icons/circle-alert.svg"),
            ),
            Self::Controls => (
                "bytes://retroprint/sliders-horizontal.svg",
                include_bytes!("../assets/icons/sliders-horizontal.svg"),
            ),
            Self::Palette => (
                "bytes://retroprint/palette.svg",
                include_bytes!("../assets/icons/palette.svg"),
            ),
        }
    }
}

pub fn icon(which: Icon, size: f32, color: Color32) -> egui::Image<'static> {
    let (uri, bytes) = which.asset();
    egui::Image::from_bytes(uri, bytes)
        .fit_to_exact_size(vec2(size, size))
        .tint(color)
}

pub fn icon_button(which: Icon, label: &str, color: Color32) -> egui::Button<'static> {
    egui::Button::image_and_text(icon(which, 16.0, color), label.to_string())
}

pub fn apply_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = BACKGROUND;
    visuals.window_fill = PANEL;
    visuals.window_stroke = Stroke::new(1.0, BORDER);
    visuals.window_shadow = egui::Shadow {
        offset: [0, 8],
        blur: 24,
        spread: 0,
        color: Color32::from_black_alpha(110),
    };
    visuals.override_text_color = Some(TEXT);
    visuals.selection.bg_fill = ACCENT.gamma_multiply(0.32);
    visuals.selection.stroke.color = ACCENT;
    visuals.widgets.inactive.weak_bg_fill = SURFACE;
    visuals.widgets.inactive.bg_fill = SURFACE;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, BORDER);
    visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(49, 54, 60);
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(49, 54, 60);
    visuals.widgets.active.weak_bg_fill = ACCENT;
    visuals.widgets.active.bg_fill = ACCENT;
    visuals.widgets.active.fg_stroke.color = BACKGROUND;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER);
    visuals.slider_trailing_fill = true;
    ctx.set_visuals(visuals);

    ctx.style_mut_of(egui::Theme::Dark, |style| {
        style.spacing.item_spacing = vec2(8.0, 9.0);
        style.spacing.button_padding = vec2(12.0, 8.0);
        style.spacing.slider_width = 190.0;
        style.spacing.scroll = egui::style::ScrollStyle::solid();
        style.spacing.scroll.bar_width = 7.0;
        style.spacing.scroll.bar_inner_margin = 9.0;
        style.spacing.scroll.bar_outer_margin = 3.0;
    });
}

pub fn separator(ui: &mut egui::Ui) {
    ui.add_space(16.0);
    ui.separator();
    ui.add_space(16.0);
}

pub fn section(ui: &mut egui::Ui, text: &str, which: Icon) {
    ui.horizontal(|ui| {
        ui.add(icon(which, 15.0, ACCENT));
        ui.label(RichText::new(text).size(11.0).strong().color(TEXT));
    });
    ui.add_space(12.0);
}

pub fn slider(ui: &mut egui::Ui, label: &str, value: &mut f32) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(12.5).color(TEXT));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            egui::Frame::new()
                .fill(SURFACE)
                .stroke(Stroke::new(1.0, BORDER))
                .corner_radius(5)
                .inner_margin(egui::Margin::symmetric(9, 4))
                .show(ui, |ui| {
                    changed |= ui.add(
                        egui::DragValue::new(value)
                            .range(0.0..=100.0)
                            .speed(0.25)
                            .fixed_decimals(0)
                            .suffix("%"),
                    ).on_hover_text("Drag or double-click to type an exact value").changed();
                });
        });
    });
    changed |= ui
        .add_sized(
            [ui.available_width(), 17.0],
            egui::Slider::new(value, 0.0..=100.0).show_value(false),
        )
        .on_hover_text("Drag to adjust • double-click the value to enter a number")
        .changed();
    ui.add_space(8.0);
    changed
}

pub fn brand(ui: &mut egui::Ui) {
    let origin = ui.cursor().min + vec2(12.0, 8.0);
    for row in 0i32..5 {
        for col in 0i32..5 {
            let center = origin + vec2(col as f32 * 6.5, row as f32 * 6.5);
            if (col - 2).abs() + (row - 2).abs() <= 3 {
                ui.painter().circle_filled(center, 2.15, ACCENT);
            }
        }
    }
    ui.add_space(36.0);
    ui.vertical(|ui| {
        ui.label(
            RichText::new("R E T R O P R I N T")
                .color(TEXT)
                .size(19.0)
                .strong(),
        );
        ui.label(
            RichText::new("I N K   &   G R A I N   S T U D I O")
                .color(SUBTLE)
                .size(9.0),
        );
    });
}

pub fn fit_image(bounds: Rect, natural_size: Vec2) -> Rect {
    let size = vec2(natural_size.x.max(1.0), natural_size.y.max(1.0));
    let scale = (bounds.width() / size.x).min(bounds.height() / size.y);
    let fitted = size * scale.max(0.0);
    Rect::from_center_size(bounds.center(), fitted)
}

fn card(p: &Painter, rect: Rect, title: &str, texture: Option<&TextureHandle>) {
    p.rect_filled(rect, 9, SURFACE);
    p.rect_stroke(rect, 9, Stroke::new(1.0, BORDER), StrokeKind::Inside);
    let content = Rect::from_min_max(
        rect.min + vec2(14.0, 14.0),
        pos2(rect.max.x - 14.0, rect.max.y - 43.0),
    );
    p.rect_filled(content, 3, BACKGROUND);
    if let Some(texture) = texture {
        let image = fit_image(content, texture.size_vec2());
        p.image(
            texture.id(),
            image,
            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            Color32::WHITE,
        );
    } else {
        p.text(
            content.center(),
            Align2::CENTER_CENTER,
            "Rendering preview…",
            FontId::proportional(13.0),
            SUBTLE,
        );
    }
    p.text(
        pos2(rect.center().x, rect.max.y - 20.0),
        Align2::CENTER_CENTER,
        title,
        FontId::proportional(11.0),
        TEXT,
    );
}

/// Paint the preview workspace into its *entire* allocated rectangle.
/// Returns true when the user clicks the empty import surface.
pub fn canvas(
    ui: &mut egui::Ui,
    original: Option<&TextureHandle>,
    printed: Option<&TextureHandle>,
    filename: Option<&str>,
    pending: bool,
    hovering_file: bool,
    wipe: bool,
    wipe_fraction: &mut f32,
) -> bool {
    let bounds = ui.available_rect_before_wrap();
    let response = ui.allocate_rect(bounds, Sense::click_and_drag());
    let p = ui.painter_at(bounds);
    p.rect_filled(bounds, 0, BACKGROUND);

    let margin = if bounds.width() < 670.0 { 15.0 } else { 26.0 };
    let inner = bounds.shrink2(vec2(margin, 18.0));
    p.text(
        inner.min,
        Align2::LEFT_TOP,
        "WORKSPACE",
        FontId::proportional(17.0),
        TEXT,
    );
    if let Some(name) = filename {
        p.text(
            pos2(inner.max.x, inner.min.y + 5.0),
            Align2::RIGHT_TOP,
            name,
            FontId::proportional(11.0),
            SUBTLE,
        );
    }
    let cards = Rect::from_min_max(
        pos2(inner.min.x, inner.min.y + 51.0),
        pos2(inner.max.x, inner.max.y - 3.0),
    );

    if original.is_none() {
        p.rect_filled(cards, 7, PANEL);
        p.rect_stroke(
            cards,
            7,
            Stroke::new(1.5, if hovering_file { ACCENT } else { BORDER }),
            StrokeKind::Inside,
        );
        let middle = cards.center();
        // Use the exact same vector asset as the rest of the app, not a
        // separately hand-drawn pictogram in the large drop target.
        let icon_rect = Rect::from_center_size(middle - vec2(0.0, 58.0), vec2(45.0, 45.0));
        ui.put(icon_rect, icon(Icon::Drop, 45.0, ACCENT));
        p.text(
            middle + vec2(0.0, 13.0),
            Align2::CENTER_CENTER,
            if hovering_file {
                "Release to import"
            } else {
                "Drop an image here"
            },
            FontId::proportional(19.0),
            TEXT,
        );
        p.text(
            middle + vec2(0.0, 43.0),
            Align2::CENTER_CENTER,
            "or click to browse  ·  PNG / JPG / WebP",
            FontId::proportional(12.0),
            SUBTLE,
        );
    } else if wipe && printed.is_some() {
        let original = original.expect("source texture exists");
        let printed = printed.expect("rendered texture exists");
        p.rect_filled(cards, 10, SURFACE);
        let viewport = cards.shrink2(vec2(16.0, 30.0));
        let image_rect = fit_image(viewport, original.size_vec2());
        if response.dragged() {
            if let Some(pointer) = ui.ctx().pointer_interact_pos() {
                if image_rect.width() > 0.0 {
                    *wipe_fraction = ((pointer.x - image_rect.left()) / image_rect.width())
                        .clamp(0.02, 0.98);
                }
            }
        }
        let divider_x = image_rect.left() + image_rect.width() * *wipe_fraction;
        let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
        p.image(original.id(), image_rect, uv, Color32::WHITE);
        let clip = Rect::from_min_max(
            pos2(divider_x, image_rect.top()),
            image_rect.max,
        );
        p.with_clip_rect(clip).image(printed.id(), image_rect, uv, Color32::WHITE);
        p.line_segment(
            [pos2(divider_x, image_rect.top()), pos2(divider_x, image_rect.bottom())],
            Stroke::new(2.0, ACCENT),
        );
        p.circle_filled(pos2(divider_x, image_rect.center().y), 11.0, ACCENT);
        p.line_segment(
            [pos2(divider_x - 4.0, image_rect.center().y),
             pos2(divider_x + 4.0, image_rect.center().y)],
            Stroke::new(2.0, BACKGROUND),
        );
        p.text(
            pos2(cards.left() + 20.0, cards.top() + 17.0),
            Align2::LEFT_CENTER,
            "ORIGINAL",
            FontId::proportional(11.0),
            SUBTLE,
        );
        p.text(
            pos2(cards.right() - 20.0, cards.top() + 17.0),
            Align2::RIGHT_CENTER,
            "PRINT",
            FontId::proportional(11.0),
            TEXT,
        );
    } else {
        let gap = 15.0;
        let width = ((cards.width() - gap) * 0.5).max(0.0);
        let original_rect = Rect::from_min_size(cards.min, vec2(width, cards.height()));
        let print_rect = Rect::from_min_size(
            pos2(cards.min.x + width + gap, cards.min.y),
            vec2(width, cards.height()),
        );
        card(&p, original_rect, "O R I G I N A L", original);
        card(
            &p,
            print_rect,
            if pending {
                "R E N D E R I N G"
            } else {
                "P R I N T"
            },
            printed,
        );
    }

    if hovering_file {
        p.rect_filled(bounds, 0, ACCENT.gamma_multiply(0.11));
        p.rect_stroke(
            bounds.shrink(4.0),
            8,
            Stroke::new(2.0, ACCENT),
            StrokeKind::Inside,
        );
    }
    let is_empty = original.is_none();
    response
        .on_hover_cursor(if is_empty {
            egui::CursorIcon::PointingHand
        } else if wipe {
            egui::CursorIcon::ResizeHorizontal
        } else {
            egui::CursorIcon::Default
        })
        .clicked()
        && is_empty
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fit_centers_and_preserves_aspect() {
        let bounds = Rect::from_min_size(pos2(20.0, 30.0), vec2(300.0, 200.0));
        let fit = fit_image(bounds, vec2(1600.0, 900.0));
        assert!((fit.width() / fit.height() - 16.0 / 9.0).abs() < 0.001);
        assert_eq!(fit.center(), bounds.center());
        assert!(bounds.contains(fit.min) && bounds.contains(fit.max));
    }
}
