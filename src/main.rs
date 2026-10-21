#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod effect;
mod image_io;
mod presets;
mod ui;

use std::{
    path::{Path, PathBuf},
    sync::{Arc, mpsc},
    thread,
};

use eframe::egui::{self, Color32, RichText, TextureHandle};
use image::RgbaImage;

use crate::ui as view;
use crate::{
    effect::{Settings, StippleMode, render},
    image_io::{load, load_bytes, make_preview, save_png, save_svg},
    presets::{LOOKS, PALETTES, apply_look, defaults},
};

struct PreviewRequest {
    revision: u64,
    input: Arc<RgbaImage>,
    settings: Settings,
}

struct PreviewResult {
    revision: u64,
    rendered: RgbaImage,
}

struct ExportResult {
    path: PathBuf,
    result: Result<u64, String>,
}

struct RetroPrintApp {
    settings: Settings,
    ink_hex: String,
    paper_hex: String,
    source_path: Option<PathBuf>,
    source_name: Option<String>,
    source: Option<Arc<RgbaImage>>,
    preview_source: Option<Arc<RgbaImage>>,
    original_texture: Option<TextureHandle>,
    print_texture: Option<TextureHandle>,
    preview_sender: mpsc::Sender<PreviewRequest>,
    preview_receiver: mpsc::Receiver<PreviewResult>,
    export_sender: mpsc::Sender<ExportResult>,
    export_receiver: mpsc::Receiver<ExportResult>,
    revision: u64,
    preview_pending: bool,
    exporting: bool,
    status: String,
    is_error: bool,
    wipe_view: bool,
    wipe_fraction: f32,
}

impl RetroPrintApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        view::apply_theme(&cc.egui_ctx);
        egui_extras::install_image_loaders(&cc.egui_ctx);

        let (preview_sender, job_rx) = mpsc::channel::<PreviewRequest>();
        let (result_tx, preview_receiver) = mpsc::channel::<PreviewResult>();
        thread::Builder::new()
            .name("retroprint-preview".to_string())
            .spawn(move || {
                while let Ok(mut job) = job_rx.recv() {
                    // A slider may queue several jobs while the worker renders.
                    // Render only the latest one waiting in the queue.
                    while let Ok(next) = job_rx.try_recv() {
                        job = next;
                    }
                    let rendered = render(&job.input, &job.settings);
                    if result_tx
                        .send(PreviewResult {
                            revision: job.revision,
                            rendered,
                        })
                        .is_err()
                    {
                        break;
                    }
                }
            })
            .expect("failed to create preview worker");

        let (export_sender, export_receiver) = mpsc::channel();

        let settings = defaults();
        let mut app = Self {
            ink_hex: format_hex_color(settings.ink),
            paper_hex: format_hex_color(settings.paper),
            settings,
            source_path: None,
            source_name: None,
            source: None,
            preview_source: None,
            original_texture: None,
            print_texture: None,
            preview_sender,
            preview_receiver,
            export_sender,
            export_receiver,
            revision: 0,
            preview_pending: false,
            exporting: false,
            status: "Drop an image or select Open Image".into(),
            is_error: false,
            wipe_view: false,
            wipe_fraction: 0.5,
        };

        // Opening an image on startup also makes the entire render path
        // reproducible in desktop smoke tests without a native file picker.
        if let Some(path) = std::env::args_os().nth(1) {
            app.open_path(&cc.egui_ctx, Path::new(&path));
        }
        app
    }

    fn open_path(&mut self, ctx: &egui::Context, path: &Path) {
        match load(path) {
            Ok(pixels) => {
                self.install_source(ctx, pixels, Some(path.to_path_buf()), file_label(path))
            }
            Err(err) => {
                self.status = format!("Cannot open image: {err}");
                self.is_error = true;
            }
        }
    }

    fn open_dropped(&mut self, ctx: &egui::Context, file: egui::DroppedFileHandle) {
        let path = file.path().to_path_buf();
        // Native Wayland/X11 drag-and-drop usually provides an absolute path.
        // Some portals/providers offer only a readable file handle instead.
        let image = if path.is_file() {
            match load(&path) {
                Ok(image) => Ok(image),
                Err(_) => file
                    .bytes()
                    .and_then(|bytes| load_bytes(&bytes).map_err(|e| e.to_string())),
            }
        } else {
            file.bytes()
                .and_then(|bytes| load_bytes(&bytes).map_err(|e| e.to_string()))
        };

        match image {
            Ok(image) => {
                let name = file_label(&path);
                let usable_path = path.is_file().then_some(path);
                self.install_source(ctx, image, usable_path, name);
            }
            Err(err) => {
                self.status = format!("Cannot open dropped image: {err}");
                self.is_error = true;
            }
        }
    }

    fn install_source(
        &mut self,
        ctx: &egui::Context,
        pixels: RgbaImage,
        path: Option<PathBuf>,
        name: String,
    ) {
        let preview = Arc::new(make_preview(&pixels));
        self.original_texture = Some(texture_from_image(ctx, "original", &preview));
        self.print_texture = None;
        self.preview_source = Some(preview);
        self.source = Some(Arc::new(pixels));
        self.source_path = path;
        self.source_name = Some(name.clone());
        self.status = format!("Loaded {name}");
        self.is_error = false;
        self.queue_preview();
    }

    fn open_dialog(&mut self, ctx: &egui::Context) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Images", &["png", "jpg", "jpeg", "webp"])
            .pick_file()
        {
            self.open_path(ctx, &path);
        }
    }

    fn queue_preview(&mut self) {
        if let Some(input) = &self.preview_source {
            self.revision = self.revision.wrapping_add(1);
            let job = PreviewRequest {
                revision: self.revision,
                input: Arc::clone(input),
                settings: self.settings,
            };
            if self.preview_sender.send(job).is_ok() {
                self.preview_pending = true;
            } else {
                self.status = "Preview worker is unavailable".into();
                self.is_error = true;
            }
        }
    }

    fn poll_jobs(&mut self, ctx: &egui::Context) {
        while let Ok(result) = self.preview_receiver.try_recv() {
            if result.revision == self.revision {
                self.print_texture = Some(texture_from_image(ctx, "print", &result.rendered));
                self.preview_pending = false;
                if !self.is_error && !self.exporting {
                    self.status = "Preview ready".into();
                }
            }
        }
        while let Ok(done) = self.export_receiver.try_recv() {
            self.exporting = false;
            match done.result {
                Ok(bytes) => {
                    self.status = format!(
                        "Saved {} · {:.1} KB",
                        file_label(&done.path),
                        bytes as f64 / 1024.0
                    );
                    self.is_error = false;
                }
                Err(err) => {
                    self.status = format!("Export failed: {err}");
                    self.is_error = true;
                }
            }
        }
    }

    fn export_dialog(&mut self, svg: bool) {
        if self.exporting {
            return;
        }
        let Some(source) = self.source.as_ref() else {
            return;
        };

        let extension = if svg { "svg" } else { "png" };
        let label = if svg { "SVG Image" } else { "PNG Image" };
        let suggested = self
            .source_name
            .as_deref()
            .and_then(|name| Path::new(name).file_stem())
            .map(|stem| format!("{}_retroprint.{extension}", stem.to_string_lossy()))
            .unwrap_or_else(|| format!("retroprint.{extension}"));
        let mut picker = rfd::FileDialog::new()
            .add_filter(label, &[extension])
            .set_file_name(&suggested);
        if let Some(parent) = self.source_path.as_ref().and_then(|p| p.parent()) {
            picker = picker.set_directory(parent);
        }
        let Some(mut output_path) = picker.save_file() else {
            return;
        };
        output_path.set_extension(extension);
        if self.source_path.as_deref() == Some(output_path.as_path()) {
            self.status = "Choose a new filename: overwriting the source is disabled".into();
            self.is_error = true;
            return;
        }

        let source = Arc::clone(source);
        let settings = self.settings;
        let sender = self.export_sender.clone();
        self.exporting = true;
        self.status = format!("Rendering full-resolution {}…", extension.to_uppercase());
        self.is_error = false;
        thread::spawn(move || {
            let rendered = render(&source, &settings);
            let outcome = (if svg {
                save_svg(&output_path, &rendered)
            } else {
                save_png(&output_path, &rendered)
            })
            .and_then(|()| Ok(std::fs::metadata(&output_path)?.len()))
            .map_err(|err| err.to_string());
            let _ = sender.send(ExportResult {
                path: output_path,
                result: outcome,
            });
        });
    }

    fn header(&mut self, ui: &mut egui::Ui) -> (bool, bool, bool) {
        let mut open = false;
        let mut export = false;
        let mut export_svg = false;
        ui.horizontal(|ui| {
            ui.add_space(23.0);
            view::brand(ui);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(20.0);
                export_svg = ui
                    .add_enabled(
                        self.source.is_some() && !self.exporting,
                        view::icon_button(view::Icon::Export, "SVG", view::TEXT)
                            .min_size(egui::vec2(84.0, 36.0)),
                    )
                    .clicked();
                export = ui
                    .add_enabled(
                        self.source.is_some() && !self.exporting,
                        view::icon_button(view::Icon::Export, "Export PNG", view::BACKGROUND)
                            .fill(view::ACCENT)
                            .min_size(egui::vec2(150.0, 36.0)),
                    )
                    .clicked();
                open = ui
                    .add_sized(
                        [145.0, 36.0],
                        view::icon_button(view::Icon::Open, "Open Image", view::TEXT),
                    )
                    .clicked();
                if self.source.is_some()
                    && ui
                        .add(
                            egui::Button::new("Wipe Compare")
                                .selected(self.wipe_view)
                                .corner_radius(16),
                        )
                        .on_hover_text("Drag the divider across the image to compare")
                        .clicked()
                {
                    self.wipe_view = !self.wipe_view;
                }
            });
        });
        (open, export, export_svg)
    }

    fn inspector(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        ui.add_space(10.0);
        view::section(ui, "P R I N T   C O N T R O L S", view::Icon::Controls);
        ui.label(RichText::new("Print Recipes").size(12.0).color(view::TEXT));
        ui.label(
            RichText::new("Start with a look, then fine-tune the sliders below.")
                .size(10.0)
                .color(view::SUBTLE),
        );
        let recipe_columns = if ui.available_width() >= 320.0 { 3 } else { 2 };
        let recipe_width =
            (ui.available_width() - (recipe_columns - 1) as f32 * 7.0) / recipe_columns as f32;
        egui::Grid::new("print_recipes")
            .num_columns(recipe_columns)
            .spacing([7.0, 7.0])
            .show(ui, |ui| {
                for (index, look) in LOOKS.iter().enumerate() {
                    let selected = self.settings.mode == look.mode
                        && self.settings.blur == look.blur
                        && self.settings.pixel_size == look.pixel_size
                        && self.settings.grain_size == look.grain_size
                        && self.settings.grain_amount == look.grain_amount
                        && self.settings.contrast == look.contrast
                        && self.settings.threshold == look.threshold
                        && self.settings.irregularity == look.irregularity
                        && self.settings.ink == PALETTES[look.palette].ink
                        && self.settings.paper == PALETTES[look.palette].paper;
                    if ui
                        .add_sized(
                            [recipe_width.max(60.0), 34.0],
                            egui::Button::new(RichText::new(look.label).size(11.0))
                                .selected(selected),
                        )
                        .on_hover_text("Apply texture, contrast and palette")
                        .clicked()
                    {
                        apply_look(&mut self.settings, *look);
                        self.sync_hex_fields();
                        changed = true;
                    }
                    if (index + 1) % recipe_columns == 0 {
                        ui.end_row();
                    }
                }
            });
        ui.add_space(14.0);
        ui.label(RichText::new("Texture Mode").size(12.0).color(view::TEXT));
        let mode_width = (ui.available_width() - 7.0) * 0.5;
        egui::Grid::new("texture_modes")
            .num_columns(2)
            .spacing([7.0, 7.0])
            .show(ui, |ui| {
                for (index, (mode, label)) in [
                    (StippleMode::Organic, "Organic"),
                    (StippleMode::Fine, "Fine Stipple"),
                    (StippleMode::Threshold, "Threshold"),
                    (StippleMode::Bayer4, "Bayer 4×4"),
                    (StippleMode::Bayer, "Bayer 8×8"),
                    (StippleMode::Halftone, "Halftone Dots"),
                    (StippleMode::LineScreen, "Line Screen"),
                    (StippleMode::FloydSteinberg, "Floyd–Steinberg"),
                    (StippleMode::Atkinson, "Atkinson"),
                ]
                .into_iter()
                .enumerate()
                {
                    let selected = self.settings.mode == mode;
                    if ui
                        .add_sized(
                            [mode_width, 32.0],
                            egui::Button::new(RichText::new(label).size(
                                if mode == StippleMode::FloydSteinberg {
                                    9.5
                                } else {
                                    11.5
                                },
                            ))
                            .selected(selected),
                        )
                        .clicked()
                        && !selected
                    {
                        self.settings.mode = mode;
                        changed = true;
                    }
                    if index % 2 == 1 {
                        ui.end_row();
                    }
                }
            });
        let mode_hint = match self.settings.mode {
            StippleMode::Organic => "Soft, clustered analog grain",
            StippleMode::Fine => "Fine blue-noise dots and sharp details",
            StippleMode::Threshold => "Hard black-and-white shapes",
            StippleMode::Bayer4 => "Tight 4×4 ordered pattern",
            StippleMode::Bayer => "Detailed 8×8 ordered pattern",
            StippleMode::Halftone => "Round ink dots grow with shadow density",
            StippleMode::LineScreen => "Engraved diagonal parallel lines",
            StippleMode::FloydSteinberg => "Continuous-tone error diffusion",
            StippleMode::Atkinson => "High-contrast retro error diffusion",
        };
        ui.label(RichText::new(mode_hint).size(10.0).color(view::SUBTLE));
        ui.add_space(12.0);
        let pixel_mode = !matches!(self.settings.mode, StippleMode::Organic | StippleMode::Fine);
        if pixel_mode {
            changed |= view::slider(ui, "Pixel Size", &mut self.settings.pixel_size);
            if !matches!(
                self.settings.mode,
                StippleMode::Threshold | StippleMode::Halftone | StippleMode::LineScreen
            ) {
                changed |= view::slider(ui, "Dither Strength", &mut self.settings.grain_amount);
            }
            changed |= view::slider(ui, "Blur", &mut self.settings.blur);
        } else {
            changed |= view::slider(ui, "Grain Size", &mut self.settings.grain_size);
            changed |= view::slider(ui, "Grain Amount", &mut self.settings.grain_amount);
        }
        changed |= view::slider(ui, "Contrast", &mut self.settings.contrast);
        changed |= view::slider(ui, "Threshold", &mut self.settings.threshold);
        if matches!(
            self.settings.mode,
            StippleMode::Organic
                | StippleMode::Fine
                | StippleMode::Bayer4
                | StippleMode::Bayer
                | StippleMode::FloydSteinberg
                | StippleMode::Atkinson
        ) {
            changed |= view::slider(ui, "Irregularity", &mut self.settings.irregularity);
        }

        view::separator(ui);
        view::section(ui, "I N K   &   P A P E R", view::Icon::Palette);
        let swatch_width = (ui.available_width() - 10.0) * 0.5;
        ui.horizontal(|ui| {
            changed |= color_selector(
                ui,
                "Ink Color",
                &mut self.settings.ink,
                &mut self.ink_hex,
                swatch_width,
            );
            changed |= color_selector(
                ui,
                "Paper Color",
                &mut self.settings.paper,
                &mut self.paper_hex,
                swatch_width,
            );
        });
        ui.label(
            RichText::new("Click a swatch for spectrum · or type HEX")
                .size(10.0)
                .color(view::SUBTLE),
        );
        ui.add_space(15.0);
        ui.label(RichText::new("Quick Palette").size(12.0).color(view::TEXT));
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            for palette in PALETTES {
                let selected =
                    self.settings.ink == palette.ink && self.settings.paper == palette.paper;
                let color = Color32::from_rgb(palette.ink[0], palette.ink[1], palette.ink[2]);
                let stroke = if selected {
                    egui::Stroke::new(2.0, view::ACCENT)
                } else {
                    egui::Stroke::new(1.0, view::BORDER)
                };
                if ui
                    .add_sized(
                        [28.0, 28.0],
                        egui::Button::new("").fill(color).stroke(stroke),
                    )
                    .on_hover_text(palette.label)
                    .clicked()
                {
                    self.settings.ink = palette.ink;
                    self.settings.paper = palette.paper;
                    self.sync_hex_fields();
                    changed = true;
                }
            }
        });
        ui.add_space(10.0);
        if ui
            .button("⇄  Swap Ink / Paper")
            .on_hover_text("Reverse foreground and background colors without changing the texture")
            .clicked()
        {
            std::mem::swap(&mut self.settings.ink, &mut self.settings.paper);
            self.sync_hex_fields();
            changed = true;
        }
        ui.add_space(16.0);
        changed |= ui
            .checkbox(&mut self.settings.transparent_paper, "Transparent Paper")
            .changed();
        ui.add_space(16.0);
        ui.horizontal(|ui| {
            let button_width = (ui.available_width() - 8.0) * 0.5;
            if ui
                .add_sized(
                    [button_width, 34.0],
                    view::icon_button(view::Icon::Reset, "Reset", view::TEXT),
                )
                .clicked()
            {
                self.settings = defaults();
                self.sync_hex_fields();
                changed = true;
            }
            if ui
                .add_sized(
                    [button_width, 34.0],
                    view::icon_button(view::Icon::Reseed, "Reseed", view::TEXT),
                )
                .clicked()
            {
                self.settings.seed = self.settings.seed.wrapping_add(0x9E37_79B9);
                changed = true;
            }
        });
        ui.add_space(8.0);
        ui.label(
            RichText::new(format!("Seed  {:08X}", self.settings.seed))
                .monospace()
                .size(10.0)
                .color(view::SUBTLE),
        );
        changed
    }

    fn sync_hex_fields(&mut self) {
        self.ink_hex = format_hex_color(self.settings.ink);
        self.paper_hex = format_hex_color(self.settings.paper);
    }

    fn footer(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.add_space(24.0);
            if let Some(input) = &self.source {
                ui.label(
                    RichText::new(format!("{} × {} px", input.width(), input.height()))
                        .size(12.0)
                        .monospace()
                        .color(view::SUBTLE),
                );
                ui.separator();
            }
            ui.add(view::icon(
                if self.is_error {
                    view::Icon::Error
                } else {
                    view::Icon::Success
                },
                15.0,
                if self.is_error {
                    view::ERROR
                } else {
                    view::ACCENT
                },
            ));
            ui.label(
                RichText::new(&self.status)
                    .color(if self.is_error {
                        view::ERROR
                    } else {
                        view::SUBTLE
                    })
                    .size(12.0),
            );
        });
    }
}

fn format_hex_color(rgb: [u8; 3]) -> String {
    format!("#{:02X}{:02X}{:02X}", rgb[0], rgb[1], rgb[2])
}

fn parse_hex_color(input: &str) -> Option<[u8; 3]> {
    let hex = input.trim().strip_prefix('#').unwrap_or(input.trim());
    if hex.len() != 6 {
        return None;
    }
    Some([
        u8::from_str_radix(&hex[0..2], 16).ok()?,
        u8::from_str_radix(&hex[2..4], 16).ok()?,
        u8::from_str_radix(&hex[4..6], 16).ok()?,
    ])
}

fn color_selector(
    ui: &mut egui::Ui,
    label: &str,
    rgb: &mut [u8; 3],
    hex: &mut String,
    width: f32,
) -> bool {
    let mut changed = false;
    ui.vertical(|ui| {
        ui.label(RichText::new(label).size(11.0).color(view::TEXT));
        let response = ui
            .add_sized(
                [width, 45.0],
                egui::Button::new("")
                    .fill(Color32::from_rgb(rgb[0], rgb[1], rgb[2]))
                    .stroke(egui::Stroke::new(1.0, view::BORDER)),
            )
            .on_hover_text(format!("Edit {label} using the color spectrum"));
        egui::Popup::menu(&response).show(|ui| {
            ui.label(format!("{label} · Spectrum"));
            let mut picked = Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
            if egui::color_picker::color_picker_color32(
                ui,
                &mut picked,
                egui::color_picker::Alpha::Opaque,
            ) {
                *rgb = [picked.r(), picked.g(), picked.b()];
                *hex = format_hex_color(*rgb);
                changed = true;
            }
        });
        let input = ui.add_sized(
            [width, 27.0],
            egui::TextEdit::singleline(hex)
                .font(egui::TextStyle::Monospace)
                .hint_text("#RRGGBB"),
        );
        if input.changed()
            && let Some(color) = parse_hex_color(hex)
            && *rgb != color
        {
            *rgb = color;
            changed = true;
        }
        if input.lost_focus() && parse_hex_color(hex).is_none() {
            *hex = format_hex_color(*rgb);
        }
    });
    changed
}

impl eframe::App for RetroPrintApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.poll_jobs(&ctx);

        if let Some(file) = ctx.input(|input| input.raw.dropped_files.first().cloned()) {
            self.open_dropped(&ctx, file);
        }
        let hovering_file = ctx.input(|input| !input.raw.hovered_files.is_empty());
        let keyboard_open =
            ctx.input(|input| input.modifiers.command && input.key_pressed(egui::Key::O));
        let keyboard_save =
            ctx.input(|input| input.modifiers.command && input.key_pressed(egui::Key::S));

        let mut changed = false;
        let mut open = false;
        let mut export = false;
        let mut export_svg = false;
        egui::Panel::top("retroprint-toolbar")
            .exact_size(72.0)
            .frame(
                egui::Frame::new()
                    .fill(view::PANEL)
                    .stroke(egui::Stroke::new(1.0, view::BORDER)),
            )
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(12.0);
                    (open, export, export_svg) = self.header(ui);
                });
            });

        egui::Panel::bottom("retroprint-status")
            .exact_size(41.0)
            .frame(
                egui::Frame::new()
                    .fill(view::PANEL)
                    .stroke(egui::Stroke::new(1.0, view::BORDER))
                    .inner_margin(egui::Margin::symmetric(0, 7)),
            )
            .show(ui, |ui| self.footer(ui));

        let sidebar_width = if ui.available_width() < 1040.0 {
            280.0
        } else {
            330.0
        };
        egui::Panel::left("retroprint-inspector")
            .exact_size(sidebar_width)
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(view::PANEL)
                    .stroke(egui::Stroke::new(1.0, view::BORDER))
                    .inner_margin(egui::Margin {
                        left: 20,
                        right: 5,
                        top: 16,
                        bottom: 16,
                    }),
            )
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("retroprint-inspector-scroll")
                    .auto_shrink([false, false])
                    .scroll_bar_visibility(
                        egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded,
                    )
                    .show(ui, |ui| {
                        changed = self.inspector(ui);
                    });
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(view::BACKGROUND))
            .show(ui, |ui| {
                open |= view::canvas(
                    ui,
                    self.original_texture.as_ref(),
                    self.print_texture.as_ref(),
                    self.source_name.as_deref(),
                    self.preview_pending,
                    hovering_file,
                    view::CompareView {
                        enabled: self.wipe_view,
                        fraction: &mut self.wipe_fraction,
                    },
                );
            });

        if changed {
            self.queue_preview();
        }
        if open || keyboard_open {
            self.open_dialog(&ctx);
        }
        if export || keyboard_save {
            self.export_dialog(false);
        } else if export_svg {
            self.export_dialog(true);
        }

        if self.preview_pending || self.exporting {
            ctx.request_repaint_after(std::time::Duration::from_millis(40));
        }
    }
}

fn texture_from_image(ctx: &egui::Context, name: &str, pixels: &RgbaImage) -> TextureHandle {
    let rgba = egui::ColorImage::from_rgba_unmultiplied(
        [pixels.width() as usize, pixels.height() as usize],
        pixels.as_raw(),
    );
    ctx.load_texture(
        name,
        rgba,
        if name == "print" {
            egui::TextureOptions::NEAREST
        } else {
            egui::TextureOptions::LINEAR
        },
    )
}

fn file_label(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

fn main() -> eframe::Result {
    let mut options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("RetroPrint")
            .with_inner_size([1320.0, 820.0])
            .with_min_inner_size([900.0, 580.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };

    // KDE Plasma / Dolphin on Wayland: prefer the XWayland/X11 event loop
    // when DISPLAY is available. On Wayland-only systems (no XWayland
    // DISPLAY), keep winit's normal native backend so the app starts.
    // winit 0.30 no longer recognizes WINIT_UNIX_BACKEND, hence the builder.
    #[cfg(target_os = "linux")]
    if std::env::var_os("DISPLAY").is_some_and(|display| !display.is_empty()) {
        options.event_loop_builder = Some(Box::new(|builder| {
            use winit::platform::x11::EventLoopBuilderExtX11;
            builder.with_x11();
        }));
    }
    eframe::run_native(
        "RetroPrint",
        options,
        Box::new(|cc| Ok(Box::new(RetroPrintApp::new(cc)))),
    )
}

#[cfg(test)]
mod color_tests {
    use super::*;

    #[test]
    fn hex_color_accepts_upper_and_lower_case() {
        assert_eq!(parse_hex_color("#5cA0fF"), Some([92, 160, 255]));
        assert_eq!(parse_hex_color("112233"), Some([17, 34, 51]));
        assert_eq!(parse_hex_color("#FFFF"), None);
        assert_eq!(parse_hex_color("#12345G"), None);
        assert_eq!(format_hex_color([0, 127, 255]), "#007FFF");
    }
}
