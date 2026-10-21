//! File I/O and source-preview preparation.

use std::{collections::HashMap, error::Error, fmt, fs::File, io::{self, BufWriter, Write}, path::Path};

use image::{ImageFormat, RgbaImage, imageops::FilterType};
use png::{BitDepth, ColorType, Compression, Encoder};

const MAX_PIXELS: u64 = 50_000_000;
const PREVIEW_LONG_EDGE: u32 = 1_400;
type IndexedPalette = (Vec<[u8; 4]>, HashMap<[u8; 4], u8>);

#[derive(Debug)]
struct ImageLimitError(u32, u32);

impl fmt::Display for ImageLimitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Image is too large ({} × {}). The limit is 50 megapixels.",
            self.0, self.1
        )
    }
}

impl Error for ImageLimitError {}

pub fn load(path: &Path) -> Result<RgbaImage, Box<dyn Error + Send + Sync>> {
    let decoded = image::open(path)?;
    into_limited_rgba(decoded)
}

/// Fallback for desktop file-drop providers that expose bytes but no usable path.
pub fn load_bytes(bytes: &[u8]) -> Result<RgbaImage, Box<dyn Error + Send + Sync>> {
    let decoded = image::load_from_memory(bytes)?;
    into_limited_rgba(decoded)
}

fn into_limited_rgba(
    decoded: image::DynamicImage,
) -> Result<RgbaImage, Box<dyn Error + Send + Sync>> {
    let (width, height) = (decoded.width(), decoded.height());
    if u64::from(width) * u64::from(height) > MAX_PIXELS {
        return Err(Box::new(ImageLimitError(width, height)));
    }
    Ok(decoded.to_rgba8())
}

pub fn make_preview(source: &RgbaImage) -> RgbaImage {
    let (w, h) = source.dimensions();
    let long_edge = w.max(h);
    if long_edge <= PREVIEW_LONG_EDGE || long_edge == 0 {
        return source.clone();
    }
    let scale = PREVIEW_LONG_EDGE as f64 / long_edge as f64;
    let out_w = ((w as f64 * scale).round() as u32).max(1);
    let out_h = ((h as f64 * scale).round() as u32).max(1);
    image::imageops::resize(source, out_w, out_h, FilterType::Triangle)
}

/// Save the exact RGBA pixels using the smallest lossless indexed color depth
/// when possible. A typical RetroPrint render needs only two palette entries,
/// so even a full-resolution image can be stored at one bit per pixel.
pub fn save_png(path: &Path, image: &RgbaImage) -> Result<(), Box<dyn Error + Send + Sync>> {
    if let Some((colors, indices)) = indexed_palette(image) {
        let depth = match colors.len() {
            0..=2 => BitDepth::One,
            3..=4 => BitDepth::Two,
            5..=16 => BitDepth::Four,
            _ => BitDepth::Eight,
        };
        let bits = match depth {
            BitDepth::One => 1,
            BitDepth::Two => 2,
            BitDepth::Four => 4,
            _ => 8,
        };
        let pixels_per_byte = 8 / bits;
        let stride = (image.width() as usize).div_ceil(pixels_per_byte);
        let mut packed = vec![0u8; stride * image.height() as usize];

        for (y, row) in image.rows().enumerate() {
            for (x, pixel) in row.enumerate() {
                let index = indices[&pixel.0];
                let shift = 8 - bits * (x % pixels_per_byte + 1);
                packed[y * stride + x / pixels_per_byte] |= index << shift;
            }
        }

        let palette: Vec<_> = colors
            .iter()
            .flat_map(|pixel| pixel[..3].iter().copied())
            .collect();
        let alpha: Vec<_> = colors.iter().map(|pixel| pixel[3]).collect();
        let file = BufWriter::new(File::create(path)?);
        let mut encoder = Encoder::new(file, image.width(), image.height());
        encoder.set_color(ColorType::Indexed);
        encoder.set_depth(depth);
        encoder.set_palette(palette);
        if alpha.iter().any(|&a| a != 255) {
            encoder.set_trns(alpha);
        }
        encoder.set_compression(Compression::High);
        encoder.write_header()?.write_image_data(&packed)?;
        Ok(())
    } else {
        // Too many distinct RGBA values (for example a smooth alpha mask):
        // preserve the original pixels instead of quantizing or flattening.
        image.save_with_format(path, ImageFormat::Png)?;
        Ok(())
    }
}

#[derive(Clone, Copy)]
struct SvgRun {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    color: [u8; 4],
}

fn write_rect<W: Write>(writer: &mut W, run: SvgRun) -> io::Result<()> {
    write!(
        writer,
        r##"<rect x="{}" y="{}" width="{}" height="{}" fill="#{:02X}{:02X}{:02X}""##,
        run.x, run.y, run.width, run.height,
        run.color[0], run.color[1], run.color[2]
    )?;
    if run.color[3] != 255 {
        write!(writer, r#" fill-opacity="{:.9}""#, run.color[3] as f64 / 255.0)?;
    }
    writeln!(writer, "/>")
}

pub fn write_svg<W: Write>(mut writer: W, image: &RgbaImage) -> io::Result<()> {
    writeln!(
        writer,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}" shape-rendering="crispEdges">"#,
        image.width(), image.height(), image.width(), image.height()
    )?;

    let mut pending: Vec<SvgRun> = Vec::new();
    for y in 0..image.height() {
        let mut next = Vec::new();
        let mut previous = pending.into_iter().peekable();
        let mut x = 0;
        while x < image.width() {
            let color = image.get_pixel(x, y).0;
            let start = x;
            while x < image.width() && image.get_pixel(x, y).0 == color {
                x += 1;
            }
            if color[3] == 0 { continue; }
            while previous.peek().is_some_and(|p| p.x < start) {
                write_rect(&mut writer, previous.next().expect("run exists"))?;
            }
            if previous.peek().is_some_and(|p| p.x == start && p.width == x - start && p.color == color) {
                let mut run = previous.next().expect("matching run exists");
                run.height += 1;
                next.push(run);
            } else {
                next.push(SvgRun {
                    x: start,
                    y,
                    width: x - start,
                    height: 1,
                    color,
                });
            }
        }
        for run in previous {
            write_rect(&mut writer, run)?;
        }
        pending = next;
    }
    for run in pending { write_rect(&mut writer, run)?; }
    writeln!(writer, "</svg>")
}

pub fn save_svg(path: &Path, image: &RgbaImage) -> Result<(), Box<dyn Error + Send + Sync>> {
    let file = BufWriter::new(File::create(path)?);
    write_svg(file, image)?;
    Ok(())
}

fn indexed_palette(image: &RgbaImage) -> Option<IndexedPalette> {
    let mut colors = Vec::new();
    let mut indices = HashMap::new();
    for pixel in image.pixels() {
        if let std::collections::hash_map::Entry::Vacant(entry) = indices.entry(pixel.0) {
            if colors.len() == 256 {
                return None;
            }
            let index = colors.len() as u8;
            colors.push(pixel.0);
            entry.insert(index);
        }
    }
    Some((colors, indices))
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    #[test]
    fn svg_merges_solid_rectangles_and_preserves_alpha() {
        let image = RgbaImage::from_fn(6, 4, |x, y| {
            if x == 0 { Rgba([0, 0, 0, 0]) }
            else if x < 4 { Rgba([10, 20, 30, 255]) }
            else if y < 2 { Rgba([200, 100, 50, 128]) }
            else { Rgba([10, 20, 30, 255]) }
        });
        let mut output = Vec::new();
        write_svg(&mut output, &image).unwrap();
        let text = String::from_utf8(output).unwrap();
        assert!(text.contains(r#"viewBox="0 0 6 4""#));
        assert!(text.contains(r##"fill="#0A141E""##));
        assert!(text.contains(r#"fill-opacity="0.501960784""#));
        assert!(text.contains(r#"height="4""#));
        assert!(!text.contains(r#"x="0" y="0" width="1""#));
    }

    #[test]
    fn small_preview_is_unchanged() {
        let img = RgbaImage::from_pixel(40, 60, Rgba([10, 20, 30, 40]));
        assert_eq!(make_preview(&img), img);
    }

    #[test]
    fn preview_keeps_aspect_ratio() {
        let img = RgbaImage::new(2_800, 1_400);
        let preview = make_preview(&img);
        assert_eq!(preview.dimensions(), (1_400, 700));
    }

    #[test]
    fn png_export_round_trips_exact_rgba_pixels() {
        let name = format!("retroprint-test-{}.png", std::process::id());
        let path = std::env::temp_dir().join(name);
        let mut original = RgbaImage::new(2, 1);
        original.put_pixel(0, 0, Rgba([12, 35, 188, 255]));
        original.put_pixel(1, 0, Rgba([190, 41, 12, 52]));

        save_png(&path, &original).expect("save_png failed");
        let encoded = std::fs::read(&path).expect("read png bytes");
        assert_eq!(load_bytes(&encoded).expect("load bytes"), original);
        let decoded = load(&path).expect("load failed");
        std::fs::remove_file(path).expect("temporary file cleanup failed");
        assert_eq!(original, decoded);
    }

    #[test]
    fn two_color_png_uses_one_bit_palette_and_is_smaller() {
        let image = RgbaImage::from_fn(1_200, 800, |x, y| {
            let noise = x.wrapping_mul(374_761_393) ^ y.wrapping_mul(668_265_263);
            if noise.count_ones() % 5 == 0 {
                Rgba([21, 56, 88, 255])
            } else {
                Rgba([248, 229, 193, 255])
            }
        });
        let path = std::env::temp_dir().join(format!(
            "retroprint-indexed-test-{}.png",
            std::process::id()
        ));
        save_png(&path, &image).expect("indexed png encoding");
        let indexed = std::fs::read(&path).expect("read indexed file");
        std::fs::remove_file(&path).expect("cleanup indexed file");

        let mut conventional = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(image.clone())
            .write_to(&mut conventional, ImageFormat::Png)
            .expect("ordinary RGBA encoding");
        assert_eq!(indexed[24], 1, "PNG should have 1-bit sample depth");
        assert_eq!(indexed[25], 3, "PNG should use indexed-color mode");
        assert_eq!(load_bytes(&indexed).expect("decode indexed PNG"), image);
        assert!(
            indexed.len() < conventional.get_ref().len() / 2,
            "indexed={} RGBA={}",
            indexed.len(),
            conventional.get_ref().len()
        );
        eprintln!(
            "1-bit indexed {} bytes vs conventional RGBA {} bytes",
            indexed.len(),
            conventional.get_ref().len()
        );
    }

    #[test]
    fn indexed_png_round_trips_padded_rows_and_translucency() {
        let img = RgbaImage::from_fn(13, 5, |x, y| {
            let color = [
                [1, 2, 3, 0],
                [1, 2, 3, 52],
                [1, 2, 3, 255],
                [44, 55, 66, 255],
            ][(x as usize + 3 * y as usize) % 4];
            Rgba(color)
        });
        let path =
            std::env::temp_dir().join(format!("retroprint-alpha-test-{}.png", std::process::id()));
        save_png(&path, &img).expect("indexed alpha encoding");
        let bytes = std::fs::read(&path).expect("read indexed alpha file");
        std::fs::remove_file(&path).expect("cleanup indexed alpha file");
        assert_eq!(bytes[24], 2, "four RGBA colors require two bits");
        assert_eq!(bytes[25], 3, "indexed color type");
        assert_eq!(load_bytes(&bytes).expect("decode alpha PNG"), img);
    }

    #[test]
    fn png_preserves_more_than_256_colors_without_quantization() {
        let image = RgbaImage::from_fn(257, 1, |x, _| {
            Rgba([(x % 256) as u8, (x / 256) as u8, 42, 200])
        });
        let path =
            std::env::temp_dir().join(format!("retroprint-rgba-test-{}.png", std::process::id()));
        save_png(&path, &image).expect("full RGBA encoding");
        let bytes = std::fs::read(&path).expect("read full RGBA file");
        std::fs::remove_file(&path).expect("cleanup full RGBA file");
        assert_eq!(bytes[25], 6, "fallback must preserve full RGBA");
        assert_eq!(load_bytes(&bytes).expect("decode RGBA"), image);
    }
}
