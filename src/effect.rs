//! Deterministic, CPU-only, two-color stochastic print renderer.

use image::{Rgba, RgbaImage};
use std::sync::OnceLock;

/// The legacy clustered texture and the new detail-preserving stochastic screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StippleMode {
    Organic,
    Fine,
    Bayer,
    Atkinson,
}

/// Controls are normalized to 0..=100 for a consistent GUI contract.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Settings {
    pub mode: StippleMode,
    pub grain_size: f32,
    pub grain_amount: f32,
    pub contrast: f32,
    pub threshold: f32,
    pub irregularity: f32,
    pub ink: [u8; 3],
    pub paper: [u8; 3],
    pub transparent_paper: bool,
    pub seed: u32,
}

impl Settings {
    pub fn normalized(mut self) -> Self {
        self.grain_size = clamp_control(self.grain_size);
        self.grain_amount = clamp_control(self.grain_amount);
        self.contrast = clamp_control(self.contrast);
        self.threshold = clamp_control(self.threshold);
        self.irregularity = clamp_control(self.irregularity);
        self
    }
}

fn clamp_control(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 100.0)
    } else {
        50.0
    }
}

/// Render output at the same dimensions as `source` without changing the input.
/// The seed fixes every pseudo-random decision; output is bitwise repeatable.
pub fn render(source: &RgbaImage, settings: &Settings) -> RgbaImage {
    let s = settings.normalized();
    match s.mode {
        StippleMode::Organic => render_organic(source, &s),
        StippleMode::Fine => render_fine(source, &s),
        StippleMode::Bayer => render_bayer(source, &s),
        StippleMode::Atkinson => render_atkinson(source, &s),
    }
}

// Keep the original effect exactly intact, including its seeded pixel pattern.
fn tone_coverage(pixel: &Rgba<u8>, s: &Settings) -> f32 {
    let luma = (0.2126 * pixel[0] as f32
        + 0.7152 * pixel[1] as f32
        + 0.0722 * pixel[2] as f32) / 255.0;
    let contrast = 0.55 + 1.85 * s.contrast / 100.0;
    let threshold = (s.threshold - 50.0) / 100.0 * 0.74;
    (1.0 - (0.5 + (luma - 0.5) * contrast).clamp(0.0, 1.0) + threshold)
        .clamp(0.0, 1.0)
}

fn print_pixel(input: &Rgba<u8>, s: &Settings, printed: bool) -> Rgba<u8> {
    if input[3] == 0 {
        Rgba([0, 0, 0, 0])
    } else if printed {
        Rgba([s.ink[0], s.ink[1], s.ink[2], input[3]])
    } else if s.transparent_paper {
        Rgba([0, 0, 0, 0])
    } else {
        Rgba([s.paper[0], s.paper[1], s.paper[2], input[3]])
    }
}

fn render_bayer(source: &RgbaImage, s: &Settings) -> RgbaImage {
    const MATRIX: [[u8; 8]; 8] = [
        [0,48,12,60,3,51,15,63],
        [32,16,44,28,35,19,47,31],
        [8,56,4,52,11,59,7,55],
        [40,24,36,20,43,27,39,23],
        [2,50,14,62,1,49,13,61],
        [34,18,46,30,33,17,45,29],
        [10,58,6,54,9,57,5,53],
        [42,26,38,22,41,25,37,21],
    ];
    let min_side = source.width().min(source.height()).max(1) as f32;
    let scale = (1.0 + (s.grain_size / 100.0 * min_side / 350.0).floor())
        .clamp(1.0, 12.0) as u32;
    let amount = s.grain_amount / 100.0;
    let irregularity = s.irregularity / 100.0;
    RgbaImage::from_fn(source.width(), source.height(), |x, y| {
        let pixel = source.get_pixel(x, y);
        let cx = (x / scale).wrapping_add(s.seed & 7) & 7;
        let cy = (y / scale).wrapping_add((s.seed >> 8) & 7) & 7;
        let rank = (MATRIX[cy as usize][cx as usize] as f32 + 0.5) / 64.0;
        let jitter = (hash01(x as i32 / scale as i32, y as i32 / scale as i32, s.seed) - 0.5)
            * irregularity * 0.2;
        let cutoff = 0.5 * (1.0 - amount) + (rank + jitter).clamp(0.0, 1.0) * amount;
        print_pixel(pixel, s, tone_coverage(pixel, s) >= cutoff)
    })
}

fn render_atkinson(source: &RgbaImage, s: &Settings) -> RgbaImage {
    let (width, height) = source.dimensions();
    let w = width as usize;
    let h = height as usize;
    let mut tone: Vec<f32> = source.pixels().map(|p| tone_coverage(p, s)).collect();
    let mut out = RgbaImage::new(width, height);
    let strength = s.grain_amount / 100.0;
    let error_spread = strength * (0.65 + 0.35 * s.irregularity / 100.0);
    for y in 0..h {
        let left_to_right = y % 2 == 0;
        for step in 0..w {
            let x = if left_to_right { step } else { w - 1 - step };
            let idx = y * w + x;
            let input = source.get_pixel(x as u32, y as u32);
            if input[3] == 0 {
                out.put_pixel(x as u32, y as u32, Rgba([0, 0, 0, 0]));
                continue;
            }
            let jitter = (hash01(x as i32, y as i32, s.seed) - 0.5)
                * (1.0 - strength) * 0.08;
            let value = (tone[idx] + jitter).clamp(0.0, 1.0);
            let printed = value >= 0.5;
            out.put_pixel(x as u32, y as u32, print_pixel(input, s, printed));
            let error = (value - if printed { 1.0 } else { 0.0 }) * error_spread / 8.0;
            let direction: isize = if left_to_right { 1 } else { -1 };
            for (dx, dy) in [(1,0),(2,0),(-1,1),(0,1),(1,1),(0,2)] {
                let xx = x as isize + dx * direction;
                let yy = y + dy as usize;
                if xx >= 0 && (xx as usize) < w && yy < h
                    && source.get_pixel(xx as u32, yy as u32)[3] != 0
                {
                    tone[yy * w + xx as usize] += error;
                }
            }
        }
    }
    out
}

fn render_organic(source: &RgbaImage, s: &Settings) -> RgbaImage {
    let (width, height) = source.dimensions();
    let base = width.min(height).max(1) as f32 / 1000.0;
    // Spatial frequency scales with image resolution, so a downscaled preview
    // has approximately the same relative print grain as the saved original.
    let grain_period = ((0.8 + 5.7 * s.grain_size / 100.0) * base).max(0.65);
    let amount = s.grain_amount / 100.0;
    let irregularity = s.irregularity / 100.0;
    let threshold_offset = (s.threshold - 50.0) / 100.0 * 0.74;
    let contrast_gain = 0.55 + 1.85 * s.contrast / 100.0;
    let smooth_mix = ((grain_period - 0.75) / (grain_period + 1.5)).clamp(0.0, 0.78);

    RgbaImage::from_fn(width, height, |x, y| {
        let input = source.get_pixel(x, y);
        let input_alpha = input[3];
        if input_alpha == 0 {
            return Rgba([0, 0, 0, 0]);
        }

        // Standard approximate sRGB luminance is sufficient for a graphic,
        // non-color-managed two-ink effect.
        let luminance =
            (0.2126 * input[0] as f32 + 0.7152 * input[1] as f32 + 0.0722 * input[2] as f32)
                / 255.0;
        let tone = (0.5 + (luminance - 0.5) * contrast_gain).clamp(0.0, 1.0);
        let coverage = (1.0 - tone + threshold_offset).clamp(0.0, 1.0);

        // Pixel-level noise keeps individual dots crisp. A spatially smoothed
        // field grows natural-looking, irregular clusters as Grain Size grows.
        let pixel_noise = hash01(x as i32, y as i32, s.seed);
        let smooth_noise = value_noise(
            x as f32 / grain_period,
            y as f32 / grain_period,
            s.seed ^ 0x76A3_9FB1,
        );
        let micro_noise = pixel_noise * (1.0 - smooth_mix) + smooth_noise * smooth_mix;
        let blotch = value_noise(
            x as f32 / (grain_period * 5.0 + 5.0),
            y as f32 / (grain_period * 5.0 + 5.0),
            s.seed ^ 0x41C6_CE57,
        ) - 0.5;

        let ink_level = (coverage + blotch * irregularity * 0.34).clamp(0.0, 1.0);
        let cutoff = 0.5 * (1.0 - amount) + micro_noise * amount;
        let printed = ink_level >= cutoff;

        if printed {
            Rgba([s.ink[0], s.ink[1], s.ink[2], input_alpha])
        } else if s.transparent_paper {
            Rgba([0, 0, 0, 0])
        } else {
            Rgba([s.paper[0], s.paper[1], s.paper[2], input_alpha])
        }
    })
}

/// Higher-quality print screen: bilateral denoising rejects color differences
/// across edges, a wider neighborhood restores useful local contrast, and a
/// public-domain void-and-cluster blue-noise mask separates individual dots.
fn render_fine(source: &RgbaImage, s: &Settings) -> RgbaImage {
    let (width, height) = source.dimensions();
    let w = width as usize;
    let h = height as usize;
    let luminances: Vec<f32> = source
        .pixels()
        .map(|p| (0.2126 * p[0] as f32 + 0.7152 * p[1] as f32 + 0.0722 * p[2] as f32) / 255.0)
        .collect();

    let base = width.min(height).max(1) as f32 / 1000.0;
    let grain_period = ((0.8 + 5.7 * s.grain_size / 100.0) * base).max(0.65);
    let detail_radius = ((base * 2.0).round() as i32).clamp(2, 6);
    let amount = s.grain_amount / 100.0;
    let irregularity = s.irregularity / 100.0;
    let threshold_offset = (s.threshold - 50.0) / 100.0 * 0.74;
    let contrast_gain = 0.55 + 1.85 * s.contrast / 100.0;
    // Grain Size still controls the scale of the organic variation, but blue
    // noise remains dominant so that midtones don't turn into random clumps.
    let organic_mix = 0.08 + 0.15 * s.grain_size / 100.0;
    let blue_noise = blue_noise_mask();

    RgbaImage::from_fn(width, height, |x, y| {
        let input_alpha = source.get_pixel(x, y)[3];
        if input_alpha == 0 {
            return Rgba([0, 0, 0, 0]);
        }

        let center = luminances[y as usize * w + x as usize];
        let mut weighted_sum = center * 3.0;
        let mut weight_sum = 3.0;
        for dy in -1..=1 {
            for dx in -1..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let neighbor = sampled_luma(&luminances, w, h, x as i32 + dx, y as i32 + dy);
                let delta = (neighbor - center) * 16.0;
                let spatial = if dx == 0 || dy == 0 { 1.0 } else { 0.7 };
                let weight = spatial / (1.0 + delta * delta);
                weighted_sum += neighbor * weight;
                weight_sum += weight;
            }
        }
        let smooth = weighted_sum / weight_sum;

        let mut surround = 0.0;
        for (dx, dy) in [
            (-1, 0),
            (1, 0),
            (0, -1),
            (0, 1),
            (-1, -1),
            (1, -1),
            (-1, 1),
            (1, 1),
        ] {
            surround += sampled_luma(
                &luminances,
                w,
                h,
                x as i32 + dx * detail_radius,
                y as i32 + dy * detail_radius,
            );
        }
        let local_mean = surround * 0.125;
        let detail = (smooth - local_mean) * 0.5;
        let tone = (0.5 + (smooth + detail - 0.5) * contrast_gain).clamp(0.0, 1.0);
        let coverage = (1.0 - tone + threshold_offset).clamp(0.0, 1.0);

        let bx = (x.wrapping_add(s.seed & 127) & 127) as usize;
        let by = (y.wrapping_add((s.seed >> 8) & 127) & 127) as usize;
        let blue = (blue_noise[by * 128 + bx] as f32 + 0.5) / 256.0;
        let organic = value_noise(
            x as f32 / grain_period,
            y as f32 / grain_period,
            s.seed ^ 0x76A3_9FB1,
        );
        let cutoff = 0.5 * (1.0 - amount) + lerp(blue, organic, organic_mix) * amount;
        let blotch = value_noise(
            x as f32 / (grain_period * 5.0 + 5.0),
            y as f32 / (grain_period * 5.0 + 5.0),
            s.seed ^ 0x41C6_CE57,
        ) - 0.5;
        let ink_level = (coverage + blotch * irregularity * 0.18).clamp(0.0, 1.0);
        if ink_level >= cutoff {
            Rgba([s.ink[0], s.ink[1], s.ink[2], input_alpha])
        } else if s.transparent_paper {
            Rgba([0, 0, 0, 0])
        } else {
            Rgba([s.paper[0], s.paper[1], s.paper[2], input_alpha])
        }
    })
}

#[inline]
fn sampled_luma(luminances: &[f32], width: usize, height: usize, x: i32, y: i32) -> f32 {
    let xx = x.clamp(0, width as i32 - 1) as usize;
    let yy = y.clamp(0, height as i32 - 1) as usize;
    luminances[yy * width + xx]
}

/// The LDR_LLL1_0 mask is by Christoph Peters, CC0 1.0. Decode once; the
/// immutable single-channel rank map is reused for all previews and exports.
fn blue_noise_mask() -> &'static [u8] {
    static MASK: OnceLock<Vec<u8>> = OnceLock::new();
    MASK.get_or_init(|| {
        let pixels = image::load_from_memory(include_bytes!("../assets/noise/blue-noise-128.png"))
            .expect("bundled blue-noise mask must be valid")
            .into_luma8();
        assert_eq!(pixels.dimensions(), (128, 128));
        pixels.into_raw()
    })
}

#[inline]
fn hash01(x: i32, y: i32, seed: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x9E37_79B1) ^ (y as u32).wrapping_mul(0x85EB_CA77) ^ seed;
    h ^= h >> 16;
    h = h.wrapping_mul(0x7FEB_352D);
    h ^= h >> 15;
    h = h.wrapping_mul(0x846C_A68B);
    h ^= h >> 16;
    (h as f32) / (u32::MAX as f32)
}

#[inline]
fn value_noise(x: f32, y: f32, seed: u32) -> f32 {
    let ix = x.floor() as i32;
    let iy = y.floor() as i32;
    let tx = x - x.floor();
    let ty = y - y.floor();
    let ux = tx * tx * (3.0 - 2.0 * tx);
    let uy = ty * ty * (3.0 - 2.0 * ty);
    let a = lerp(hash01(ix, iy, seed), hash01(ix + 1, iy, seed), ux);
    let b = lerp(hash01(ix, iy + 1, seed), hash01(ix + 1, iy + 1, seed), ux);
    lerp(a, b, uy)
}

#[inline]
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presets::defaults;

    #[test]
    fn dimensions_and_palette_are_exact() {
        let input = RgbaImage::from_pixel(37, 21, Rgba([127, 127, 127, 255]));
        let s = defaults();
        let out = render(&input, &s);
        assert_eq!(out.dimensions(), input.dimensions());
        for p in out.pixels() {
            assert_eq!(p[3], 255);
            assert!(p.0[..3] == s.ink || p.0[..3] == s.paper);
        }
    }

    #[test]
    fn rendering_is_deterministic_and_seed_matters() {
        let input = RgbaImage::from_pixel(160, 128, Rgba([127, 127, 127, 255]));
        let mut s = defaults();
        let first = render(&input, &s);
        assert_eq!(first, render(&input, &s));
        s.seed = s.seed.wrapping_add(1);
        assert_ne!(first, render(&input, &s));
    }

    #[test]
    fn alpha_is_preserved_and_never_invented() {
        let mut input = RgbaImage::new(3, 1);
        input.put_pixel(0, 0, Rgba([0, 0, 0, 0]));
        input.put_pixel(1, 0, Rgba([0, 0, 0, 80]));
        input.put_pixel(2, 0, Rgba([255, 255, 255, 100]));
        let plain = render(&input, &defaults());
        assert_eq!(plain.get_pixel(0, 0)[3], 0);
        assert_eq!(plain.get_pixel(1, 0)[3], 80);
        assert_eq!(plain.get_pixel(2, 0)[3], 100);

        let mut s = defaults();
        s.transparent_paper = true;
        let cutout = render(&input, &s);
        assert_eq!(cutout.get_pixel(0, 0)[3], 0);
        assert_eq!(cutout.get_pixel(1, 0)[3], 80);
        assert_eq!(cutout.get_pixel(2, 0)[3], 0);
    }

    #[test]
    fn default_black_and_white_keep_their_ends() {
        let mut input = RgbaImage::new(2, 1);
        input.put_pixel(0, 0, Rgba([0, 0, 0, 255]));
        input.put_pixel(1, 0, Rgba([255, 255, 255, 255]));
        let s = defaults();
        let output = render(&input, &s);
        assert_eq!(output.get_pixel(0, 0).0[..3], s.ink);
        assert_eq!(output.get_pixel(1, 0).0[..3], s.paper);
    }

    #[test]
    fn controls_are_clamped() {
        let mut s = defaults();
        s.threshold = 10_000.0;
        s.grain_size = -100.0;
        s.contrast = f32::NAN;
        let s = s.normalized();
        assert_eq!(s.threshold, 100.0);
        assert_eq!(s.grain_size, 0.0);
        assert_eq!(s.contrast, 50.0);
    }

    #[test]
    fn fine_stipple_is_repeatable_and_changes_with_seed() {
        let input = RgbaImage::from_pixel(150, 120, Rgba([128, 128, 128, 255]));
        let mut s = defaults();
        s.mode = StippleMode::Fine;
        let result = render(&input, &s);
        assert_eq!(result, render(&input, &s));
        s.seed = s.seed.wrapping_add(1);
        assert_ne!(result, render(&input, &s));
        s.mode = StippleMode::Organic;
        assert_ne!(result, render(&input, &s));
    }

    #[test]
    fn fine_stipple_preserves_palette_and_partial_alpha() {
        let mut input = RgbaImage::from_pixel(73, 37, Rgba([127, 127, 127, 160]));
        input.put_pixel(0, 0, Rgba([0, 0, 0, 0]));
        let mut s = defaults();
        s.mode = StippleMode::Fine;
        let output = render(&input, &s);
        assert_eq!(output.dimensions(), input.dimensions());
        assert_eq!(output.get_pixel(0, 0).0, [0, 0, 0, 0]);
        for p in output.pixels().skip(1) {
            assert_eq!(p[3], 160);
            assert!(p.0[..3] == s.ink || p.0[..3] == s.paper);
        }
        s.transparent_paper = true;
        for p in render(&input, &s).pixels() {
            assert!(p[3] == 0 || p[3] == 160);
            if p[3] == 160 {
                assert_eq!(p.0[..3], s.ink);
            }
        }
    }

    #[test]
    fn fine_stipple_keeps_hard_light_dark_boundaries() {
        let input = RgbaImage::from_fn(128, 64, |x, _| {
            if x < 64 {
                Rgba([0, 0, 0, 255])
            } else {
                Rgba([255, 255, 255, 255])
            }
        });
        let mut s = defaults();
        s.mode = StippleMode::Fine;
        let result = render(&input, &s);
        for y in 0..64 {
            assert_eq!(result.get_pixel(63, y).0[..3], s.ink);
            assert_eq!(result.get_pixel(64, y).0[..3], s.paper);
        }
    }

    #[test]
    fn fine_stipple_preserves_tonal_order() {
        let input = RgbaImage::from_fn(256, 160, |x, _| {
            let shade = match x / 64 {
                0 => 48,
                1 => 100,
                2 => 155,
                _ => 208,
            };
            Rgba([shade, shade, shade, 255])
        });
        let mut s = defaults();
        s.mode = StippleMode::Fine;
        let result = render(&input, &s);
        let counts: Vec<_> = (0..4)
            .map(|band| {
                (20..140)
                    .flat_map(|y| (band * 64 + 12..band * 64 + 52).map(move |x| (x, y)))
                    .filter(|&(x, y)| result.get_pixel(x, y).0[..3] == s.ink)
                    .count()
            })
            .collect();
        assert!(
            counts.windows(2).all(|pair| pair[0] > pair[1]),
            "{counts:?}"
        );
    }

    #[test]
    fn bundled_blue_noise_is_a_full_8bit_rank_mask() {
        let mask = blue_noise_mask();
        assert_eq!(mask.len(), 128 * 128);
        assert!(mask.contains(&0));
        assert!(mask.contains(&255));
    }

    #[test]
    fn fine_midtones_have_fewer_adjacent_ink_clumps() {
        let input = RgbaImage::from_pixel(128, 128, Rgba([128, 128, 128, 255]));
        let mut s = defaults();
        s.grain_size = 0.0;
        s.grain_amount = 100.0;
        s.irregularity = 0.0;
        let organic = render(&input, &s);
        s.mode = StippleMode::Fine;
        let fine = render(&input, &s);

        let touching_pairs = |output: &RgbaImage| -> usize {
            (0..128)
                .flat_map(|y| (0..127).map(move |x| (x, y)))
                .filter(|&(x, y)| {
                    output.get_pixel(x, y).0[..3] == s.ink
                        && output.get_pixel(x + 1, y).0[..3] == s.ink
                })
                .count()
        };
        let organic_pairs = touching_pairs(&organic);
        let fine_pairs = touching_pairs(&fine);
        assert!(
            fine_pairs < organic_pairs,
            "fine={fine_pairs}, organic={organic_pairs}"
        );
    }
}

#[cfg(test)]
mod dithering_tests {
    use super::*;
    use crate::presets::defaults;

    #[test]
    fn renderers_are_distinct_and_repeatable() {
        let source = RgbaImage::from_fn(96, 96, |x, y| {
            let v = ((x * 2 + y) % 256) as u8;
            Rgba([v, v, v, 255])
        });
        let mut s = defaults();
        let mut outputs = Vec::new();
        for mode in [StippleMode::Organic, StippleMode::Fine, StippleMode::Bayer, StippleMode::Atkinson] {
            s.mode = mode;
            let output = render(&source, &s);
            assert_eq!(output, render(&source, &s));
            assert_eq!(output.dimensions(), source.dimensions());
            assert!(output.pixels().all(|p| p.0[..3] == s.ink || p.0[..3] == s.paper));
            outputs.push(output);
        }
        for i in 0..outputs.len() {
            for j in i + 1..outputs.len() {
                assert_ne!(outputs[i], outputs[j]);
            }
        }
    }
}
