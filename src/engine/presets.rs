//! Built-in background gradients. Colours are our own picks.

pub struct Preset {
    pub name: &'static str,
    pub stops: &'static [(f32, [u8; 3])],
    pub angle: f32,
}

pub const PRESETS: &[Preset] = &[
    Preset {
        name: "Brasa",
        stops: &[(0.0, [255, 94, 58]), (1.0, [255, 196, 87])],
        angle: 135.0,
    },
    Preset {
        name: "Oceano",
        stops: &[(0.0, [15, 76, 129]), (1.0, [63, 193, 201])],
        angle: 135.0,
    },
    Preset {
        name: "Menta",
        stops: &[(0.0, [190, 240, 214]), (1.0, [61, 153, 112])],
        angle: 160.0,
    },
    Preset {
        name: "Pôr do sol",
        stops: &[
            (0.0, [255, 126, 95]),
            (0.55, [254, 180, 123]),
            (1.0, [255, 221, 140]),
        ],
        angle: 120.0,
    },
    Preset {
        name: "Grafite",
        stops: &[(0.0, [32, 34, 38]), (1.0, [78, 83, 94])],
        angle: 145.0,
    },
    Preset {
        name: "Areia",
        stops: &[(0.0, [246, 238, 226]), (1.0, [214, 192, 160])],
        angle: 135.0,
    },
    Preset {
        name: "Floresta",
        stops: &[(0.0, [16, 66, 50]), (1.0, [86, 171, 122])],
        angle: 135.0,
    },
    Preset {
        name: "Noite",
        stops: &[(0.0, [8, 14, 36]), (1.0, [44, 62, 128])],
        angle: 160.0,
    },
    Preset {
        name: "Coral",
        stops: &[
            (0.0, [255, 154, 139]),
            (0.5, [255, 106, 136]),
            (1.0, [255, 153, 172]),
        ],
        angle: 135.0,
    },
    Preset {
        name: "Limão",
        stops: &[(0.0, [212, 252, 121]), (1.0, [120, 214, 160])],
        angle: 120.0,
    },
    Preset {
        name: "Céu",
        stops: &[(0.0, [161, 196, 253]), (1.0, [210, 236, 250])],
        angle: 180.0,
    },
    Preset {
        name: "Ameixa",
        stops: &[
            (0.0, [58, 28, 113]),
            (0.5, [215, 109, 119]),
            (1.0, [255, 175, 123]),
        ],
        angle: 135.0,
    },
];

/// Colour of a gradient at t in 0..1.
pub fn sample(stops: &[(f32, [u8; 3])], t: f32) -> [u8; 3] {
    let t = t.clamp(0.0, 1.0);
    let mut prev = stops[0];
    for &s in stops {
        if t <= s.0 {
            let span = (s.0 - prev.0).max(1e-6);
            let k = ((t - prev.0) / span).clamp(0.0, 1.0);
            let mix = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * k).round() as u8;
            return [
                mix(prev.1[0], s.1[0]),
                mix(prev.1[1], s.1[1]),
                mix(prev.1[2], s.1[2]),
            ];
        }
        prev = s;
    }
    prev.1
}
