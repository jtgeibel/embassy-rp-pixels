#![allow(dead_code)]

use smart_leds::RGB8;

pub const BLACK: RGB8 = RGB8 { r: 0, g: 0, b: 0 };
pub const WHITE: RGB8 = RGB8 {
    r: 255,
    g: 255,
    b: 255,
};
pub const GRAY: RGB8 = RGB8 {
    r: 0x7F,
    g: 0x7F,
    b: 0x7F,
};

pub const RED: RGB8 = RGB8 { r: 255, g: 0, b: 0 };
pub const GREEN: RGB8 = RGB8 { r: 0, g: 255, b: 0 };
pub const BLUE: RGB8 = RGB8 { r: 0, g: 0, b: 255 };

pub const YELLOW: RGB8 = RGB8 {
    r: 255,
    g: 255,
    b: 0,
};
pub const AQUA: RGB8 = RGB8 {
    r: 0,
    g: 255,
    b: 255,
};
pub const FUCHSIA: RGB8 = RGB8 {
    r: 255,
    g: 0,
    b: 255,
};
