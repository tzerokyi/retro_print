use crate::effect::{Settings, StippleMode};

#[derive(Clone, Copy)]
pub struct Palette {
    pub label: &'static str,
    pub ink: [u8; 3],
    pub paper: [u8; 3],
}

pub const PALETTES: [Palette; 6] = [
    Palette {
        label: "Sepia",
        ink: [43, 32, 25],
        paper: [244, 233, 215],
    },
    Palette {
        label: "Copper",
        ink: [171, 78, 42],
        paper: [248, 231, 202],
    },
    Palette {
        label: "Rust",
        ink: [140, 49, 42],
        paper: [247, 228, 189],
    },
    Palette {
        label: "Olive",
        ink: [75, 91, 70],
        paper: [236, 233, 211],
    },
    Palette {
        label: "Indigo",
        ink: [48, 41, 134],
        paper: [239, 217, 220],
    },
    Palette {
        label: "Carbon",
        ink: [35, 36, 39],
        paper: [245, 241, 230],
    },
];

pub fn defaults() -> Settings {
    Settings {
        mode: StippleMode::Organic,
        grain_size: 42.0,
        grain_amount: 65.0,
        contrast: 58.0,
        threshold: 50.0,
        irregularity: 28.0,
        ink: PALETTES[0].ink,
        paper: PALETTES[0].paper,
        transparent_paper: false,
        seed: 0xA61D_24C3,
    }
}

#[derive(Clone, Copy)]
pub struct Look {
    pub label: &'static str,
    pub mode: StippleMode,
    pub grain_size: f32,
    pub grain_amount: f32,
    pub contrast: f32,
    pub threshold: f32,
    pub irregularity: f32,
    pub palette: usize,
}

/// Curated print recipes. Applying one retains the current seed and transparency,
/// so users can compare looks without unexpectedly changing their cutout/export.
pub const LOOKS: [Look; 6] = [
    Look { label: "Newsprint", mode: StippleMode::Bayer, grain_size: 20.0, grain_amount: 94.0, contrast: 64.0, threshold: 51.0, irregularity: 10.0, palette: 5 },
    Look { label: "Risograph", mode: StippleMode::Organic, grain_size: 54.0, grain_amount: 80.0, contrast: 67.0, threshold: 55.0, irregularity: 43.0, palette: 2 },
    Look { label: "Fine Etching", mode: StippleMode::Fine, grain_size: 12.0, grain_amount: 87.0, contrast: 74.0, threshold: 47.0, irregularity: 7.0, palette: 0 },
    Look { label: "Pulp Print", mode: StippleMode::Atkinson, grain_size: 78.0, grain_amount: 92.0, contrast: 76.0, threshold: 58.0, irregularity: 72.0, palette: 1 },
    Look { label: "Soft Grain", mode: StippleMode::Organic, grain_size: 32.0, grain_amount: 48.0, contrast: 42.0, threshold: 49.0, irregularity: 17.0, palette: 3 },
    Look { label: "Blue Ink", mode: StippleMode::Bayer, grain_size: 24.0, grain_amount: 88.0, contrast: 72.0, threshold: 50.0, irregularity: 12.0, palette: 4 },
];

pub fn apply_look(settings: &mut Settings, look: Look) {
    settings.mode = look.mode;
    settings.grain_size = look.grain_size;
    settings.grain_amount = look.grain_amount;
    settings.contrast = look.contrast;
    settings.threshold = look.threshold;
    settings.irregularity = look.irregularity;
    settings.ink = PALETTES[look.palette].ink;
    settings.paper = PALETTES[look.palette].paper;
}

#[cfg(test)]
mod look_tests {
    use super::*;

    #[test]
    fn curated_looks_are_valid_and_preserve_export_options() {
        for look in LOOKS {
            let mut settings = defaults();
            settings.seed = 12345;
            settings.transparent_paper = true;
            apply_look(&mut settings, look);
            assert_eq!(settings, settings.normalized());
            assert_eq!(settings.seed, 12345);
            assert!(settings.transparent_paper);
            assert_ne!(settings.ink, settings.paper);
        }
    }
}
