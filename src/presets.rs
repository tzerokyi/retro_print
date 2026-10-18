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
