#[derive(Debug, Clone, Copy)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub fn new(r: u8, g: u8, b: u8) -> Self { Self { r, g, b } }
    pub fn from_float(r: f32, g: f32, b: f32) -> Self {
        Self {
            r: (r.clamp(0.0, 1.0) * 255.0) as u8,
            g: (g.clamp(0.0, 1.0) * 255.0) as u8,
            b: (b.clamp(0.0, 1.0) * 255.0) as u8,
        }
    }
    pub fn to_hex(self) -> u32 { ((self.r as u32) << 16) | ((self.g as u32) << 8) | (self.b as u32) }
}

use std::ops::{Add, Mul};
impl Add for Color { type Output = Color; fn add(self, o: Color) -> Color { Color { r: self.r.saturating_add(o.r), g: self.g.saturating_add(o.g), b: self.b.saturating_add(o.b) } } }
impl Mul<f32> for Color { type Output = Color; fn mul(self, s: f32) -> Color { Color { r: (self.r as f32 * s).clamp(0.0, 255.0) as u8, g: (self.g as f32 * s).clamp(0.0, 255.0) as u8, b: (self.b as f32 * s).clamp(0.0, 255.0) as u8 } } }
