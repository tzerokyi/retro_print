#[path = "../src/effect.rs"]
mod effect;
#[path = "../src/presets.rs"]
mod presets;

use std::{error::Error, fs, path::PathBuf};

use effect::{Settings, StippleMode, render};
use image::{Rgba, RgbaImage, imageops};
use presets::{LOOKS, apply_look, defaults};

const MODES: [(&str, StippleMode); 15] = [
    ("organic", StippleMode::Organic),
    ("fine-stipple", StippleMode::Fine),
    ("threshold", StippleMode::Threshold),
    ("bayer-4", StippleMode::Bayer4),
    ("bayer-8", StippleMode::Bayer),
    ("halftone", StippleMode::Halftone),
    ("line-screen", StippleMode::LineScreen),
    ("floyd-steinberg", StippleMode::FloydSteinberg),
    ("atkinson", StippleMode::Atkinson),
    ("jarvis-judice-ninke", StippleMode::JarvisJudiceNinke),
    ("stucki", StippleMode::Stucki),
    ("sierra", StippleMode::Sierra),
    ("blue-noise", StippleMode::BlueNoise),
    ("clustered-dots", StippleMode::ClusteredDot),
    ("crosshatch", StippleMode::Crosshatch),
];

fn sample_image() -> RgbaImage {
    const W: u32 = 480;
    const H: u32 = 320;
    RgbaImage::from_fn(W, H, |x, y| {
        let fx = x as f32 / (W - 1) as f32;
        let fy = y as f32 / (H - 1) as f32;
        let wave = ((fx * 32.0).sin() * (fy * 23.0).cos()) * 16.0;
        let dx = x as f32 - 310.0;
        let dy = y as f32 - 151.0;
        let distance = (dx * dx + dy * dy).sqrt();
        let silhouette = if distance < 85.0 { -75.0 } else { 0.0 };
        let edge = if (x > 34 && x < 145 && y > 110 && y < 245)
            || (x > 150 && x < 230 && y > 180 && y < 215)
        {
            60.0
        } else {
            0.0
        };
        let checker = if y > 255 && (x / 8 + y / 8) % 2 == 0 {
            -50.0
        } else {
            0.0
        };
        let value =
            (30.0 + 220.0 * fx + wave + silhouette + edge + checker).clamp(0.0, 255.0) as u8;
        Rgba([value, value, value, 255])
    })
}

fn normalize_for_comparison(settings: &mut Settings) {
    settings.ink = [27, 28, 31];
    settings.paper = [246, 241, 228];
    settings.grain_amount = 100.0;
    settings.pixel_size = 52.0;
    settings.grain_size = 35.0;
    settings.contrast = 50.0;
    settings.threshold = 50.0;
    settings.irregularity = 0.0;
    settings.blur = 0.0;
}

fn make_gallery(
    folder: &std::path::Path,
    source: &RgbaImage,
    presets: bool,
) -> Result<Vec<String>, Box<dyn Error>> {
    let prefix = if presets { "recipe" } else { "engine" };
    let (w, h) = source.dimensions();
    let count = if presets { LOOKS.len() } else { MODES.len() };
    let mut sheet = RgbaImage::from_pixel(w * 3, h * count.div_ceil(3) as u32, Rgba([246, 241, 228, 255]));
    let mut labels = Vec::new();
    let mut metrics = String::from("mode,ink_pixels,visible_pixels,ink_percent\n");
    for index in 0..count {
        let mut settings = defaults();
        let label = if presets {
            let recipe = LOOKS[index];
            apply_look(&mut settings, recipe);
            recipe.label.to_ascii_lowercase().replace(' ', "-")
        } else {
            settings.mode = MODES[index].1;
            normalize_for_comparison(&mut settings);
            MODES[index].0.to_string()
        };
        let result = render(source, &settings);
        imageops::replace(
            &mut sheet,
            &result,
            i64::from((index % 3) as u32 * w),
            i64::from((index / 3) as u32 * h),
        );
        result.save(folder.join(format!("{prefix}-{:02}-{label}.png", index + 1)))?;
        let visible = result.pixels().filter(|p| p[3] != 0).count();
        let ink = result.pixels().filter(|p| p[3] != 0 && p.0[..3] == settings.ink).count();
        let coverage = if visible == 0 { 0.0 } else { ink as f64 * 100.0 / visible as f64 };
        metrics.push_str(&format!("{label},{ink},{visible},{coverage:.4}\n"));
        labels.push(label);
    }
    sheet.save(folder.join(format!("{prefix}-contact-sheet.png")))?;
    fs::write(folder.join(format!("{prefix}-metrics.csv")), metrics)?;
    Ok(labels)
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args_os().skip(1);
    let folder = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/effect-gallery"));
    let source = match args.next() {
        Some(path) => image::open(path)?.to_rgba8(),
        None => sample_image(),
    };
    fs::create_dir_all(&folder)?;
    let engines = make_gallery(&folder, &source, false)?;
    let recipes = make_gallery(&folder, &source, true)?;
    let mut manifest = String::from("# Engine comparison\n\n");
    for row in engines.chunks(3) {
        manifest.push_str(&format!("| {} |\n", row.join(" | ")));
    }
    manifest.push_str("\n# Print recipes\n\n");
    for row in recipes.chunks(3) {
        manifest.push_str(&format!("| {} |\n", row.join(" | ")));
    }
    manifest.push_str("\n# Palette names\n\n");
    for palette in presets::PALETTES {
        manifest.push_str(&format!("- {}\n", palette.label));
    }
    fs::write(folder.join("MANIFEST.md"), manifest)?;
    Ok(())
}
