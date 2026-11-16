//! Dark industrial palette matching the reference CSS (`--bg`, `--accent`, …).

use iced::Color;

pub const BG: Color = Color::from_rgb(
    0x0d as f32 / 255.0,
    0x11 as f32 / 255.0,
    0x17 as f32 / 255.0,
);
pub const SURFACE: Color = Color::from_rgb(
    0x16 as f32 / 255.0,
    0x1b as f32 / 255.0,
    0x27 as f32 / 255.0,
);
pub const SURFACE2: Color = Color::from_rgb(
    0x1f as f32 / 255.0,
    0x25 as f32 / 255.0,
    0x35 as f32 / 255.0,
);
pub const BORDER: Color = Color::from_rgb(
    0x2a as f32 / 255.0,
    0x30 as f32 / 255.0,
    0x50 as f32 / 255.0,
);
pub const ACCENT: Color = Color::from_rgb(
    0x7c as f32 / 255.0,
    0xb3 as f32 / 255.0,
    0x42 as f32 / 255.0,
);
pub const GREEN: Color = Color::from_rgb(
    0x3d as f32 / 255.0,
    0xdc as f32 / 255.0,
    0x84 as f32 / 255.0,
);
pub const RED: Color = Color::from_rgb(
    0xff as f32 / 255.0,
    0x55 as f32 / 255.0,
    0x55 as f32 / 255.0,
);
pub const GOLD: Color = Color::from_rgb(
    0xf5 as f32 / 255.0,
    0xc8 as f32 / 255.0,
    0x42 as f32 / 255.0,
);
pub const BLUE: Color = Color::from_rgb(
    0x42 as f32 / 255.0,
    0xa5 as f32 / 255.0,
    0xf5 as f32 / 255.0,
);
pub const TEXT: Color = Color::from_rgb(
    0xdd as f32 / 255.0,
    0xe3 as f32 / 255.0,
    0xf0 as f32 / 255.0,
);
pub const TEXT_DIM: Color = Color::from_rgb(
    0x6b as f32 / 255.0,
    0x7a as f32 / 255.0,
    0x99 as f32 / 255.0,
);
pub const HOT: Color = ACCENT;
pub const COLD: Color = Color::from_rgb(
    0x3a as f32 / 255.0,
    0x4a as f32 / 255.0,
    0x68 as f32 / 255.0,
);
pub const CYAN: Color = Color::from_rgb(
    0x00 as f32 / 255.0,
    0xe5 as f32 / 255.0,
    0xff as f32 / 255.0,
);

pub const CELL_W: f32 = 80.0;
pub const CELL_H: f32 = 70.0;
pub const RAIL_X_L: f32 = 40.0;
pub const RUNG_HEADER: f32 = 22.0;
pub const RUNG_GAP: f32 = 12.0;
