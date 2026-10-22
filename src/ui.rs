use eframe::egui::{
    self, Align2, Color32, FontId, Painter, Rect, RichText, Sense, Stroke, TextureHandle, Vec2,
    pos2, vec2,
};

pub const BACKGROUND: Color32 = Color32::from_rgb(17, 20, 24);
pub const PANEL: Color32 = Color32::from_rgb(25, 29, 34);
pub const SURFACE: Color32 = Color32::from_rgb(29, 34, 40);
pub const TEXT: Color32 = Color32::from_rgb(241, 234, 224);
pub const SUBTLE: Color32 = Color32::from_rgb(157, 160, 166);
pub const ACCENT: Color32 = Color32::from_rgb(240, 157, 96);
pub const ERROR: Color32 = Color32::from_rgb(244, 121, 114);

pub const CAPSULE_RADIUS: u8 = 20;
pub const CARD_RADIUS: u8 = 12;

pub fn matte_shadow() -> egui::Shadow {
    egui::Shadow {
        offset: [0, 4],
        blur: 16,
        spread: 0,
        color: Color32::from_black_alpha(75),
    }
}

pub fn surface_frame(fill: Color32) -> egui::Frame {
    egui::Frame::new()
        .fill(fill)
        .corner_radius(CARD_RADIUS)
        .stroke(Stroke::NONE)
        .shadow(matte_shadow())
}

pub fn capsule(label: &str) -> egui::Button<'static> {
    egui::Button::new(label.to_owned())
        .corner_radius(CAPSULE_RADIUS)
        .stroke(Stroke::NONE)
}

pub fn circle_action(label: &str) -> egui::Button<'static> {
    egui::Button::new(label.to_owned())
        .corner_radius(CAPSULE_RADIUS)
        .stroke(Stroke::NONE)
        .min_size(vec2(30.0, 30.0))
}

pub fn shake_offset(elapsed: f64) -> f32 {
    if !(0.0..0.50).contains(&elapsed) {
        return 0.0;
    }
    let envelope = 1.0 - (elapsed / 0.50) as f32;
    (elapsed as f32 * 85.0).sin() * 9.0 * envelope
}

pub fn matte_card(painter: &Painter, rect: Rect, fill: Color32) {
    painter.rect_filled(
        rect.translate(vec2(0.0, 4.0)).expand(2.0),
        CARD_RADIUS + 2,
        Color32::from_black_alpha(26),
    );
    painter.rect_filled(rect, CARD_RADIUS, fill);
}

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
    Swap,
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
            Self::Swap => (
                "bytes://retroprint/arrow-left-right.svg",
                include_bytes!("../assets/icons/arrow-left-right.svg"),
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
        .corner_radius(CAPSULE_RADIUS)
        .stroke(Stroke::NONE)
}

pub fn apply_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = BACKGROUND;
    visuals.window_fill = PANEL;
    visuals.window_stroke = Stroke::NONE;
    visuals.window_shadow = matte_shadow();
    visuals.override_text_color = Some(TEXT);
    visuals.selection.bg_fill = ACCENT.gamma_multiply(0.32);
    visuals.selection.stroke = Stroke::NONE;
    visuals.widgets.inactive.weak_bg_fill = SURFACE;
    visuals.widgets.inactive.bg_fill = SURFACE;
    visuals.widgets.inactive.bg_stroke = Stroke::NONE;
    visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(49, 54, 60);
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(49, 54, 60);
    visuals.widgets.active.weak_bg_fill = ACCENT;
    visuals.widgets.active.bg_fill = ACCENT;
    visuals.widgets.active.fg_stroke.color = BACKGROUND;
    visuals.widgets.noninteractive.bg_stroke = Stroke::NONE;
    for widget in [
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.open,
        &mut visuals.widgets.noninteractive,
    ] {
        widget.corner_radius = egui::CornerRadius::same(CAPSULE_RADIUS);
        widget.bg_stroke = Stroke::NONE;
    }
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
    ui.add_space(24.0);
}

pub fn section(ui: &mut egui::Ui, text: &str, which: Icon) {
    ui.horizontal(|ui| {
        ui.add(icon(which, 19.0, ACCENT)).on_hover_text(text);
    });
    ui.add_space(14.0);
}

pub fn slider(ui: &mut egui::Ui, label: &str, value: &mut f32) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(12.5).color(TEXT));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            egui::Frame::new()
                .fill(SURFACE)
                .stroke(Stroke::NONE)
                .corner_radius(9)
                .inner_margin(egui::Margin::symmetric(9, 4))
                .show(ui, |ui| {
                    changed |= ui
                        .add(
                            egui::DragValue::new(value)
                                .range(0.0..=100.0)
                                .speed(0.25)
                                .fixed_decimals(0)
                                .suffix("%"),
                        )
                        .changed();
                });
        });
    });
    changed |= ui
        .add_sized(
            [ui.available_width(), 17.0],
            egui::Slider::new(value, 0.0..=100.0).show_value(false),
        )
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
    ui.label(
        RichText::new("R E T R O P R I N T")
            .color(TEXT)
            .size(19.0)
            .strong(),
    );
}

pub fn fit_image(bounds: Rect, natural_size: Vec2) -> Rect {
    let size = vec2(natural_size.x.max(1.0), natural_size.y.max(1.0));
    let scale = (bounds.width() / size.x).min(bounds.height() / size.y);
    let fitted = size * scale.max(0.0);
    Rect::from_center_size(bounds.center(), fitted)
}

fn wipe_divider_x(image: Rect, viewport: Rect, fraction: f32) -> f32 {
    (image.left() + image.width() * fraction.clamp(0.0, 1.0))
        .clamp(viewport.left(), viewport.right())
}

pub fn zoomed_image(bounds: Rect, natural_size: Vec2, zoom: f32, pan: Vec2) -> Rect {
    let base = fit_image(bounds, natural_size);
    let size = base.size() * zoom.clamp(1.0, 8.0);
    let max_x = ((size.x - bounds.width()) * 0.5).max(0.0);
    let max_y = ((size.y - bounds.height()) * 0.5).max(0.0);
    let offset = vec2(pan.x.clamp(-max_x, max_x), pan.y.clamp(-max_y, max_y));
    Rect::from_center_size(bounds.center() + offset, size)
}

fn card(
    p: &Painter,
    rect: Rect,
    title: &str,
    texture: Option<&TextureHandle>,
    zoom: f32,
    pan: Vec2,
) {
    matte_card(p, rect, SURFACE);
    let content = Rect::from_min_max(
        rect.min + vec2(14.0, 14.0),
        pos2(rect.max.x - 14.0, rect.max.y - 43.0),
    );
    p.rect_filled(content, CARD_RADIUS, BACKGROUND);
    if let Some(texture) = texture {
        let image = zoomed_image(content, texture.size_vec2(), zoom, pan);
        p.with_clip_rect(content).image(
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

pub struct CompareView<'a> {
    pub enabled: bool,
    pub fraction: &'a mut f32,
    pub zoom: &'a mut f32,
    pub pan: &'a mut Vec2,
}

pub fn canvas(
    ui: &mut egui::Ui,
    original: Option<&TextureHandle>,
    printed: Option<&TextureHandle>,
    filename: Option<&str>,
    pending: bool,
    hovering_file: bool,
    compare: CompareView<'_>,
) -> bool {
    let bounds = ui.available_rect_before_wrap();
    let response = ui.allocate_rect(bounds, Sense::click_and_drag());
    let p = ui.painter_at(bounds);
    p.rect_filled(bounds, 0, BACKGROUND);

    if original.is_some() && response.hovered() {
        let scroll = ui.ctx().input(|input| input.smooth_scroll_delta.y);
        if scroll.abs() > 0.5 {
            *compare.zoom = (*compare.zoom * (scroll * 0.0015).exp()).clamp(1.0, 8.0);
        }
    }
    if original.is_some() && !compare.enabled && *compare.zoom > 1.0 && response.dragged() {
        *compare.pan += ui.ctx().input(|input| input.pointer.delta());
    }
    if *compare.zoom <= 1.0 {
        *compare.pan = Vec2::ZERO;
    }

    let margin = if bounds.width() < 670.0 { 15.0 } else { 26.0 };
    let inner = bounds.shrink2(vec2(margin, 18.0));
    if let Some(name) = filename {
        p.text(
            pos2(inner.max.x, inner.min.y + 5.0),
            Align2::RIGHT_TOP,
            name,
            FontId::proportional(11.0),
            SUBTLE,
        );
    }
    if original.is_some() && inner.width() >= 480.0 {
        let y = inner.min.y - 1.0;
        let x = inner.min.x;
        if ui
            .put(
                Rect::from_min_size(pos2(x, y), vec2(30.0, 30.0)),
                circle_action("−"),
            )
            .on_hover_text("Zoom out")
            .clicked()
        {
            *compare.zoom = (*compare.zoom / 1.5).max(1.0);
        }
        if ui
            .put(
                Rect::from_min_size(pos2(x + 36.0, y), vec2(43.0, 30.0)),
                capsule("Fit"),
            )
            .on_hover_text("Fit to workspace")
            .clicked()
        {
            *compare.zoom = 1.0;
            *compare.pan = Vec2::ZERO;
        }
        if ui
            .put(
                Rect::from_min_size(pos2(x + 85.0, y), vec2(30.0, 30.0)),
                circle_action("+"),
            )
            .on_hover_text("Zoom in")
            .clicked()
        {
            *compare.zoom = (*compare.zoom * 1.5).min(8.0);
        }
        p.text(
            pos2(x + 128.0, y + 15.0),
            Align2::LEFT_CENTER,
            format!("{:.0}%", *compare.zoom * 100.0),
            FontId::proportional(11.0),
            SUBTLE,
        );
    }
    let cards = Rect::from_min_max(
        pos2(inner.min.x, inner.min.y + 51.0),
        pos2(inner.max.x, inner.max.y - 3.0),
    );

    if original.is_none() {
        matte_card(&p, cards, if hovering_file { SURFACE } else { PANEL });
        let middle = cards.center();
        let icon_rect = Rect::from_center_size(middle - vec2(0.0, 58.0), vec2(45.0, 45.0));
        ui.put(icon_rect, icon(Icon::Drop, 45.0, ACCENT));
        p.text(
            middle + vec2(0.0, 13.0),
            Align2::CENTER_CENTER,
            if hovering_file {
                "Import image"
            } else {
                "Open image"
            },
            FontId::proportional(19.0),
            TEXT,
        );
    } else if let (Some(original), Some(printed), true) = (original, printed, compare.enabled) {
        matte_card(&p, cards, SURFACE);
        let viewport = cards.shrink2(vec2(16.0, 30.0));
        let image_rect = zoomed_image(viewport, original.size_vec2(), *compare.zoom, *compare.pan);
        if response.dragged()
            && let Some(pointer) = ui.ctx().pointer_interact_pos()
            && image_rect.width() > 0.0
        {
            *compare.fraction =
                ((pointer.x - image_rect.left()) / image_rect.width()).clamp(0.02, 0.98);
        }
        let divider_x = wipe_divider_x(image_rect, viewport, *compare.fraction);
        let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
        p.with_clip_rect(viewport)
            .image(original.id(), image_rect, uv, Color32::WHITE);
        let clip = Rect::from_min_max(pos2(divider_x, viewport.top()), viewport.max);
        p.with_clip_rect(clip)
            .image(printed.id(), image_rect, uv, Color32::WHITE);
        p.line_segment(
            [
                pos2(divider_x, viewport.top()),
                pos2(divider_x, viewport.bottom()),
            ],
            Stroke::new(2.0, ACCENT),
        );
        p.circle_filled(pos2(divider_x, viewport.center().y), 11.0, ACCENT);
        p.line_segment(
            [
                pos2(divider_x - 4.0, viewport.center().y),
                pos2(divider_x + 4.0, viewport.center().y),
            ],
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
        card(
            &p,
            original_rect,
            "O R I G I N A L",
            original,
            *compare.zoom,
            *compare.pan,
        );
        card(
            &p,
            print_rect,
            if pending {
                "R E N D E R I N G"
            } else {
                "P R I N T"
            },
            printed,
            *compare.zoom,
            *compare.pan,
        );
    }

    if hovering_file {
        p.rect_filled(bounds.shrink(4.0), CARD_RADIUS, ACCENT.gamma_multiply(0.09));
    }
    let is_empty = original.is_none();
    response
        .on_hover_cursor(if is_empty {
            egui::CursorIcon::PointingHand
        } else if compare.enabled {
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
    fn shared_theme_disables_widget_and_window_outlines() {
        let ctx = egui::Context::default();
        apply_theme(&ctx);
        let visuals = &ctx.style().visuals;
        assert_eq!(visuals.window_stroke, Stroke::NONE);
        for widget in [
            &visuals.widgets.inactive,
            &visuals.widgets.hovered,
            &visuals.widgets.active,
            &visuals.widgets.open,
            &visuals.widgets.noninteractive,
        ] {
            assert_eq!(widget.bg_stroke, Stroke::NONE);
            assert_eq!(
                widget.corner_radius,
                egui::CornerRadius::same(CAPSULE_RADIUS)
            );
        }
    }

    #[test]
    fn error_shake_decays_and_stops() {
        assert_eq!(shake_offset(-0.1), 0.0);
        assert_eq!(shake_offset(0.0), 0.0);
        assert!(shake_offset(0.02).abs() > 0.0);
        assert_eq!(shake_offset(0.5), 0.0);
        assert_eq!(shake_offset(1.0), 0.0);
    }

    #[test]
    fn wipe_divider_is_clipped_to_visible_workspace() {
        let viewport = Rect::from_min_size(pos2(100.0, 100.0), vec2(300.0, 200.0));
        let zoomed = Rect::from_min_size(pos2(-100.0, -50.0), vec2(800.0, 600.0));
        assert_eq!(wipe_divider_x(zoomed, viewport, 0.0), viewport.left());
        assert_eq!(wipe_divider_x(zoomed, viewport, 1.0), viewport.right());
        assert_eq!(wipe_divider_x(zoomed, viewport, 0.5), 300.0);
    }

    #[test]
    fn zoom_keeps_image_clamped_to_viewport() {
        let bounds = Rect::from_min_size(pos2(0.0, 0.0), vec2(300.0, 200.0));
        let fitted = zoomed_image(bounds, vec2(1200.0, 800.0), 1.0, vec2(900.0, 900.0));
        assert_eq!(fitted.center(), bounds.center());
        let scaled = zoomed_image(bounds, vec2(1200.0, 800.0), 2.0, vec2(900.0, 900.0));
        assert_eq!(scaled.size(), vec2(600.0, 400.0));
        assert_eq!(scaled.left(), bounds.left());
        assert_eq!(scaled.top(), bounds.top());
    }

    #[test]
    fn fit_centers_and_preserves_aspect() {
        let bounds = Rect::from_min_size(pos2(20.0, 30.0), vec2(300.0, 200.0));
        let fit = fit_image(bounds, vec2(1600.0, 900.0));
        assert!((fit.width() / fit.height() - 16.0 / 9.0).abs() < 0.001);
        assert_eq!(fit.center(), bounds.center());
        assert!(bounds.contains(fit.min) && bounds.contains(fit.max));
    }
}
