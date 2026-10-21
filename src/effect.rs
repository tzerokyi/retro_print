use image::{Rgba, RgbaImage};
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StippleMode {
    Organic,
    Fine,
    Threshold,
    Bayer4,
    Bayer,
    Halftone,
    LineScreen,
    FloydSteinberg,
    Atkinson,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Settings {
    pub mode: StippleMode,
    pub grain_size: f32,
    pub pixel_size: f32,
    pub blur: f32,
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
        self.pixel_size = clamp_control(self.pixel_size);
        self.blur = clamp_control(self.blur);
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

pub fn render(source: &RgbaImage, settings: &Settings) -> RgbaImage {
    let s = settings.normalized();
    match s.mode {
        StippleMode::Organic => render_organic(source, &s),
        StippleMode::Fine => render_fine(source, &s),
        StippleMode::Threshold
        | StippleMode::Bayer4
        | StippleMode::Bayer
        | StippleMode::Halftone
        | StippleMode::LineScreen
        | StippleMode::FloydSteinberg
        | StippleMode::Atkinson => render_pixel_dither(source, &s),
    }
}

fn tone_coverage(pixel: &Rgba<u8>, s: &Settings) -> f32 {
    let luma =
        (0.2126 * pixel[0] as f32 + 0.7152 * pixel[1] as f32 + 0.0722 * pixel[2] as f32) / 255.0;
    let contrast = 0.55 + 1.85 * s.contrast / 100.0;
    let threshold = (s.threshold - 50.0) / 100.0 * 0.74;
    (1.0 - (0.5 + (luma - 0.5) * contrast).clamp(0.0, 1.0) + threshold).clamp(0.0, 1.0)
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

fn pixel_cell_size(image: &RgbaImage, s: &Settings) -> u32 {
    let side = image.width().min(image.height()).max(1) as f32;
    (1.0 + (s.pixel_size / 100.0).powf(1.35) * 36.0 * side / 900.0)
        .round()
        .clamp(1.0, 128.0) as u32
}

fn render_pixel_dither(source: &RgbaImage, s: &Settings) -> RgbaImage {
    let (width, height) = source.dimensions();
    if width == 0 || height == 0 {
        return source.clone();
    }
    let cell = if matches!(s.mode, StippleMode::Halftone | StippleMode::LineScreen) {
        pixel_cell_size(source, s).max(4)
    } else {
        pixel_cell_size(source, s)
    };
    let gw = width.div_ceil(cell) as usize;
    let gh = height.div_ceil(cell) as usize;
    let mut tones = vec![0.0f32; gw * gh];
    let mut active = vec![false; gw * gh];

    for gy in 0..gh {
        for gx in 0..gw {
            let mut weighted = 0.0;
            let mut sum_alpha = 0.0;
            let x0 = gx as u32 * cell;
            let y0 = gy as u32 * cell;
            for y in y0..y0.saturating_add(cell).min(height) {
                for x in x0..x0.saturating_add(cell).min(width) {
                    let p = source.get_pixel(x, y);
                    let alpha = p[3] as f32 / 255.0;
                    weighted += tone_coverage(p, s) * alpha;
                    sum_alpha += alpha;
                }
            }
            let index = gy * gw + gx;
            if sum_alpha > 0.0 {
                active[index] = true;
                tones[index] = weighted / sum_alpha;
            }
        }
    }

    if s.blur > 0.0 {
        let original = tones.clone();
        let radius = if s.blur >= 55.0 { 2isize } else { 1isize };
        let blend = s.blur / 100.0;
        for y in 0..gh {
            for x in 0..gw {
                let index = y * gw + x;
                if !active[index] {
                    continue;
                }
                let mut sum = 0.0;
                let mut count = 0.0;
                for dy in -radius..=radius {
                    for dx in -radius..=radius {
                        let xx = x as isize + dx;
                        let yy = y as isize + dy;
                        if xx >= 0 && (xx as usize) < gw && yy >= 0 && (yy as usize) < gh {
                            let neighbor = yy as usize * gw + xx as usize;
                            if active[neighbor] {
                                sum += original[neighbor];
                                count += 1.0;
                            }
                        }
                    }
                }
                if count > 0.0 {
                    tones[index] += (sum / count - tones[index]) * blend;
                }
            }
        }
    }

    let mut mask = vec![false; tones.len()];
    let strength = s.grain_amount / 100.0;
    let random = s.irregularity / 100.0;
    match s.mode {
        StippleMode::Threshold => {
            for i in 0..mask.len() {
                mask[i] = active[i] && tones[i] >= 0.5;
            }
        }
        StippleMode::Bayer4 | StippleMode::Bayer => {
            const B4: [[u8; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];
            const B8: [[u8; 8]; 8] = [
                [0, 48, 12, 60, 3, 51, 15, 63],
                [32, 16, 44, 28, 35, 19, 47, 31],
                [8, 56, 4, 52, 11, 59, 7, 55],
                [40, 24, 36, 20, 43, 27, 39, 23],
                [2, 50, 14, 62, 1, 49, 13, 61],
                [34, 18, 46, 30, 33, 17, 45, 29],
                [10, 58, 6, 54, 9, 57, 5, 53],
                [42, 26, 38, 22, 41, 25, 37, 21],
            ];
            for y in 0..gh {
                for x in 0..gw {
                    let i = y * gw + x;
                    if !active[i] {
                        continue;
                    }
                    let rank = if s.mode == StippleMode::Bayer4 {
                        (B4[(y + ((s.seed >> 8) as usize & 3)) & 3][(x + (s.seed as usize & 3)) & 3]
                            as f32
                            + 0.5)
                            / 16.0
                    } else {
                        (B8[(y + ((s.seed >> 8) as usize & 7)) & 7][(x + (s.seed as usize & 7)) & 7]
                            as f32
                            + 0.5)
                            / 64.0
                    };
                    let jitter = (hash01(x as i32, y as i32, s.seed) - 0.5) * random * 0.12;
                    let cutoff = 0.5 + (rank - 0.5) * strength + jitter;
                    mask[i] = tones[i] >= cutoff;
                }
            }
        }
        StippleMode::Halftone | StippleMode::LineScreen => {}
        StippleMode::FloydSteinberg | StippleMode::Atkinson => {
            const FLOYD: &[(isize, usize, f32)] = &[
                (1, 0, 7.0 / 16.0),
                (-1, 1, 3.0 / 16.0),
                (0, 1, 5.0 / 16.0),
                (1, 1, 1.0 / 16.0),
            ];
            const ATKINSON: &[(isize, usize, f32)] = &[
                (1, 0, 0.125),
                (2, 0, 0.125),
                (-1, 1, 0.125),
                (0, 1, 0.125),
                (1, 1, 0.125),
                (0, 2, 0.125),
            ];
            let neighbors = if s.mode == StippleMode::FloydSteinberg {
                FLOYD
            } else {
                ATKINSON
            };
            for y in 0..gh {
                let forward = y % 2 == 0;
                let direction: isize = if forward { 1 } else { -1 };
                for step in 0..gw {
                    let x = if forward { step } else { gw - 1 - step };
                    let i = y * gw + x;
                    if !active[i] {
                        continue;
                    }
                    let jitter = (hash01(x as i32, y as i32, s.seed) - 0.5) * random * 0.12;
                    let value = (tones[i] + jitter).clamp(0.0, 1.0);
                    mask[i] = value >= 0.5;
                    let error = (value - if mask[i] { 1.0 } else { 0.0 }) * strength;
                    for &(dx, dy, weight) in neighbors {
                        let nx = x as isize + dx * direction;
                        let ny = y + dy;
                        if nx >= 0 && (nx as usize) < gw && ny < gh {
                            let dest = ny * gw + nx as usize;
                            if active[dest] {
                                tones[dest] += error * weight;
                            }
                        }
                    }
                }
            }
        }
        StippleMode::Organic | StippleMode::Fine => unreachable!(),
    }
    RgbaImage::from_fn(width, height, |x, y| {
        let i = (y / cell) as usize * gw + (x / cell) as usize;
        let ink = match s.mode {
            StippleMode::Halftone => {
                let coverage = tones[i].clamp(0.0, 1.0);
                let px = (x % cell) as f32 + 0.5;
                let py = (y % cell) as f32 + 0.5;
                let nx = px / cell as f32 - 0.5;
                let ny = py / cell as f32 - 0.5;
                nx * nx + ny * ny <= (coverage / std::f32::consts::PI).min(0.5) || coverage >= 0.999
            }
            StippleMode::LineScreen => {
                let period = (cell * 3).max(3) as f32;
                let phase = ((x as f32 + 0.45 * y as f32) / period).rem_euclid(1.0);
                (phase - 0.5).abs() * 2.0 < tones[i].clamp(0.0, 1.0)
            }
            _ => mask[i],
        };
        print_pixel(source.get_pixel(x, y), s, ink)
    })
}

fn render_organic(source: &RgbaImage, s: &Settings) -> RgbaImage {
    let (width, height) = source.dimensions();
    let base = width.min(height).max(1) as f32 / 1000.0;
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

        let luminance =
            (0.2126 * input[0] as f32 + 0.7152 * input[1] as f32 + 0.0722 * input[2] as f32)
                / 255.0;
        let tone = (0.5 + (luminance - 0.5) * contrast_gain).clamp(0.0, 1.0);
        let coverage = (1.0 - tone + threshold_offset).clamp(0.0, 1.0);

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
        s.pixel_size = 250.0;
        s.contrast = f32::NAN;
        let s = s.normalized();
        assert_eq!(s.threshold, 100.0);
        assert_eq!(s.grain_size, 0.0);
        assert_eq!(s.pixel_size, 100.0);
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

    fn gradient() -> RgbaImage {
        RgbaImage::from_fn(192, 128, |x, y| {
            let value = ((x * 2 + y) % 256) as u8;
            Rgba([value, value, value, 255])
        })
    }

    #[test]
    fn every_mode_is_deterministic_and_binary() {
        let input = gradient();
        let mut s = defaults();
        for mode in [
            StippleMode::Organic,
            StippleMode::Fine,
            StippleMode::Threshold,
            StippleMode::Bayer4,
            StippleMode::Bayer,
            StippleMode::Halftone,
            StippleMode::LineScreen,
            StippleMode::FloydSteinberg,
            StippleMode::Atkinson,
        ] {
            s.mode = mode;
            let output = render(&input, &s);
            assert_eq!(output, render(&input, &s));
            assert_eq!(output.dimensions(), input.dimensions());
            assert!(
                output
                    .pixels()
                    .all(|p| p.0[..3] == s.ink || p.0[..3] == s.paper)
            );
        }
    }

    #[test]
    fn pixel_modes_produce_distinct_patterns() {
        let input = gradient();
        let mut s = defaults();
        s.grain_amount = 100.0;
        s.irregularity = 0.0;
        s.pixel_size = 0.0;
        let mut outputs = Vec::new();
        for mode in [
            StippleMode::Threshold,
            StippleMode::Bayer4,
            StippleMode::Bayer,
            StippleMode::FloydSteinberg,
            StippleMode::Atkinson,
        ] {
            s.mode = mode;
            outputs.push(render(&input, &s));
        }
        for i in 0..outputs.len() {
            for j in (i + 1)..outputs.len() {
                assert_ne!(outputs[i], outputs[j], "{i} and {j} match");
            }
        }
    }

    #[test]
    fn pixel_size_changes_pattern_without_resizing() {
        let input = gradient();
        let mut s = defaults();
        s.mode = StippleMode::Bayer;
        s.grain_amount = 100.0;
        s.pixel_size = 0.0;
        let fine = render(&input, &s);
        s.pixel_size = 100.0;
        let coarse = render(&input, &s);
        assert_ne!(fine, coarse);
        assert_eq!(coarse.dimensions(), input.dimensions());
        assert!(pixel_cell_size(&input, &s) > 1);
    }

    #[test]
    fn pixel_mode_alpha_and_cutout_follow_original() {
        let input = RgbaImage::from_fn(24, 20, |x, y| {
            let alpha = if x == 0 {
                0
            } else if y % 2 == 0 {
                64
            } else {
                255
            };
            Rgba([128, 128, 128, alpha])
        });
        let mut s = defaults();
        s.pixel_size = 100.0;
        for mode in [
            StippleMode::Threshold,
            StippleMode::Bayer4,
            StippleMode::Bayer,
            StippleMode::FloydSteinberg,
            StippleMode::Atkinson,
        ] {
            s.mode = mode;
            let opaque = render(&input, &s);
            for (out, original) in opaque.pixels().zip(input.pixels()) {
                assert_eq!(out[3], original[3]);
            }
            s.transparent_paper = true;
            let cutout = render(&input, &s);
            for (out, original) in cutout.pixels().zip(input.pixels()) {
                assert!(out[3] == 0 || out[3] == original[3]);
            }
            s.transparent_paper = false;
        }
    }

    #[test]
    fn screened_modes_have_real_shapes() {
        let input = RgbaImage::from_pixel(96, 96, Rgba([128, 128, 128, 255]));
        let mut s = defaults();
        s.pixel_size = 70.0;
        let mut modes = Vec::new();
        for mode in [StippleMode::Halftone, StippleMode::LineScreen] {
            s.mode = mode;
            let result = render(&input, &s);
            assert_eq!(result.dimensions(), input.dimensions());
            assert!(result.pixels().any(|p| p.0[..3] == s.ink));
            assert!(result.pixels().any(|p| p.0[..3] == s.paper));
            modes.push(result);
        }
        assert_ne!(modes[0], modes[1]);
    }

    #[test]
    fn blur_changes_pixel_grid_output() {
        let input = RgbaImage::from_fn(96, 96, |x, y| {
            let v = if (x / 4 + y / 4) % 2 == 0 { 0 } else { 255 };
            Rgba([v, v, v, 255])
        });
        let mut s = defaults();
        s.mode = StippleMode::Bayer;
        s.pixel_size = 0.0;
        s.grain_amount = 100.0;
        s.blur = 0.0;
        let sharp = render(&input, &s);
        s.blur = 100.0;
        let blurred = render(&input, &s);
        assert_ne!(sharp, blurred);
    }

    #[test]
    fn threshold_preserves_black_and_white_regions() {
        let image = RgbaImage::from_fn(64, 32, |x, _| {
            let value = if x < 32 { 0 } else { 255 };
            Rgba([value, value, value, 255])
        });
        let mut s = defaults();
        s.mode = StippleMode::Threshold;
        s.pixel_size = 0.0;
        let output = render(&image, &s);
        assert_eq!(output.get_pixel(0, 0).0[..3], s.ink);
        assert_eq!(output.get_pixel(63, 0).0[..3], s.paper);
    }
}
