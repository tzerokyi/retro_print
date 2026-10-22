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
    feedback_started: Option<f64>,
    feedback_key: String,
    zoom: f32,
    pan: egui::Vec2,
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
            feedback_started: None,
            feedback_key: String::new(),
            zoom: 1.0,
            pan: egui::Vec2::ZERO,
        };

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
                self.feedback_key.clear();
            }
        }
    }

    fn open_dropped(&mut self, ctx: &egui::Context, file: egui::DroppedFileHandle) {
        let path = file.path().to_path_buf();
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
                self.feedback_key.clear();
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
        self.zoom = 1.0;
        self.pan = egui::Vec2::ZERO;
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
                self.feedback_key.clear();
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
                self.feedback_key.clear();
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
                self.feedback_key.clear();
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
                if self.source.is_some() && !self.exporting {
                    export_svg = ui
                        .add(
                            view::icon_button(view::Icon::Export, "SVG", view::TEXT)
                                .min_size(egui::vec2(84.0, 36.0)),
                        )
                        .clicked();
                    export = ui
                        .add(
                            view::icon_button(view::Icon::Export, "Export PNG", view::BACKGROUND)
                                .fill(view::ACCENT)
                                .min_size(egui::vec2(150.0, 36.0)),
                        )
                        .clicked();
                }
                open = ui
                    .add_sized(
                        [145.0, 36.0],
                        view::icon_button(view::Icon::Open, "Open Image", view::TEXT),
                    )
                    .clicked();
                if self.source.is_some()
                    && ui
                        .add(
                            view::capsule("Compare").selected(self.wipe_view),
                        )
                        .on_hover_text("Wipe Compare")
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
                            view::capsule(look.label).selected(selected),
                        )
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
                    (StippleMode::JarvisJudiceNinke, "Jarvis–Judice–Ninke"),
                    (StippleMode::Stucki, "Stucki"),
                    (StippleMode::Sierra, "Sierra"),
                    (StippleMode::BlueNoise, "Blue Noise"),
                    (StippleMode::ClusteredDot, "Clustered Dots"),
                    (StippleMode::Crosshatch, "Crosshatch"),
                ]
                .into_iter()
                .enumerate()
                {
                    let selected = self.settings.mode == mode;
                    if ui
                        .add_sized(
                            [mode_width, 32.0],
                            view::capsule(label).selected(selected),
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
        ui.add_space(16.0);
        let pixel_mode = !matches!(self.settings.mode, StippleMode::Organic | StippleMode::Fine);
        if pixel_mode {
            changed |= view::slider(ui, "Pixel Size", &mut self.settings.pixel_size);
            if !matches!(
                self.settings.mode,
                StippleMode::Threshold
                    | StippleMode::Halftone
                    | StippleMode::LineScreen
                    | StippleMode::ClusteredDot
                    | StippleMode::Crosshatch
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
                | StippleMode::JarvisJudiceNinke
                | StippleMode::Stucki
                | StippleMode::Sierra
        ) {
            changed |= view::slider(ui, "Irregularity", &mut self.settings.irregularity);
        }

        view::separator(ui);
        view::section(ui, "I N K   &   P A P E R", view::Icon::Palette);
        let swatch_width = (ui.available_width() - 10.0) * 0.5;
        let mut invalid_hex = false;
        ui.horizontal(|ui| {
            let ink = color_selector(
                ui,
                "Ink Color",
                &mut self.settings.ink,
                &mut self.ink_hex,
                swatch_width,
            );
            let paper = color_selector(
                ui,
                "Paper Color",
                &mut self.settings.paper,
                &mut self.paper_hex,
                swatch_width,
            );
            changed |= ink.0 || paper.0;
            invalid_hex = ink.1 || paper.1;
        });
        if invalid_hex {
            self.status = "Invalid HEX color".into();
            self.is_error = true;
            self.feedback_key.clear();
        } else if changed && self.is_error {
            self.is_error = false;
        }
        ui.add_space(15.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            for palette in PALETTES {
                let selected =
                    self.settings.ink == palette.ink && self.settings.paper == palette.paper;
                let color = Color32::from_rgb(palette.ink[0], palette.ink[1], palette.ink[2]);
                let swatch = ui
                    .add_sized(
                        [28.0, 28.0],
                        view::circle_action("")
                            .fill(color)
                            .stroke(egui::Stroke::NONE),
                    )
                    .on_hover_text(palette.label);
                if selected {
                    ui.painter().circle_filled(
                        swatch.rect.right_top() - egui::vec2(6.0, -6.0),
                        3.0,
                        view::ACCENT,
                    );
                }
                if swatch.clicked() {
                    self.settings.ink = palette.ink;
                    self.settings.paper = palette.paper;
                    self.sync_hex_fields();
                    changed = true;
                }
            }
        });
        ui.add_space(10.0);
        if ui
            .add(view::icon_button(
                view::Icon::Swap,
                "Swap Ink / Paper",
                view::TEXT,
            ))
            .on_hover_text("Swap colors")
            .clicked()
        {
            std::mem::swap(&mut self.settings.ink, &mut self.settings.paper);
            self.sync_hex_fields();
            changed = true;
        }
        ui.add_space(16.0);
        if ui
            .add(view::capsule("Transparent Paper").selected(self.settings.transparent_paper))
            .clicked()
        {
            self.settings.transparent_paper = !self.settings.transparent_paper;
            changed = true;
        }
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

    fn status_visible(&self) -> bool {
        self.is_error
            || self.exporting
            || self.preview_pending
            || self.status.starts_with("Saved ")
    }

    fn footer(&self, ui: &mut egui::Ui) {
        let elapsed = self
            .feedback_started
            .map(|started| ui.input(|input| input.time) - started);
        let offset = elapsed.map(view::shake_offset).unwrap_or(0.0);
        ui.horizontal(|ui| {
            ui.add_space(24.0 + offset);
            if let Some(input) = &self.source {
                ui.label(
                    RichText::new(format!("{} × {} px", input.width(), input.height()))
                        .size(12.0)
                        .monospace()
                        .color(view::SUBTLE),
                );
            }
            if self.status_visible() {
                ui.add_space(12.0);
                ui.add(view::icon(
                    if self.is_error {
                        view::Icon::Error
                    } else if self.exporting || self.preview_pending {
                        view::Icon::Controls
                    } else {
                        view::Icon::Success
                    },
                    15.0,
                    if self.is_error { view::ERROR } else { view::ACCENT },
                ));
                ui.label(
                    RichText::new(&self.status)
                        .color(if self.is_error { view::ERROR } else { view::SUBTLE })
                        .size(12.0),
                );
            }
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
) -> (bool, bool) {
    let mut changed = false;
    let mut rejected = false;
    ui.vertical(|ui| {
        ui.label(RichText::new(label).size(11.0).color(view::TEXT));
        let response = ui
            .add_sized(
                [width, 45.0],
                view::capsule("")
                    .fill(Color32::from_rgb(rgb[0], rgb[1], rgb[2])),
            )
            .on_hover_text(label);
        egui::Popup::menu(&response).show(|ui| {
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
            rejected = true;
        }
    });
    (changed, rejected)
}

impl eframe::App for RetroPrintApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.poll_jobs(&ctx);
        if self.is_error {
            if self.feedback_key != self.status {
                self.feedback_started = Some(ctx.input(|input| input.time));
                self.feedback_key.clone_from(&self.status);
            }
        } else {
            self.feedback_started = None;
            self.feedback_key.clear();
        }

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
                view::surface_frame(view::PANEL),
            )
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(12.0);
                    (open, export, export_svg) = self.header(ui);
                });
            });

        if self.source.is_some() || self.status_visible() {
        egui::Panel::bottom("retroprint-status")
            .exact_size(41.0)
            .frame(
                view::surface_frame(view::PANEL)
                    .inner_margin(egui::Margin::symmetric(0, 7)),
            )
            .show(ui, |ui| self.footer(ui));
        }

        let sidebar_width = if ui.available_width() < 1040.0 {
            280.0
        } else {
            330.0
        };
        if self.source.is_some() {
        egui::Panel::left("retroprint-inspector")
            .exact_size(sidebar_width)
            .resizable(false)
            .frame(
                view::surface_frame(view::PANEL)
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
        }

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
                        zoom: &mut self.zoom,
                        pan: &mut self.pan,
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
        if self.is_error
            && self.feedback_started.is_some_and(|started| {
                ctx.input(|input| input.time) - started < 0.5
            })
        {
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
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
