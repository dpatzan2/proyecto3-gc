use crate::{framebuffer::Framebuffer, color::Color};

pub struct Star { pub x: i32, pub y: i32, pub c: u32 }

pub struct Skybox { stars: Vec<Star>, w: usize, h: usize }

impl Skybox {
    pub fn new(w: usize, h: usize, count: usize, seed: u64) -> Self {
        let mut rng = fastrand::Rng::with_seed(seed);
        let mut stars = Vec::with_capacity(count);
        for _ in 0..count { let x = rng.i32(0..w as i32); let y = rng.i32(0..h as i32); let g = rng.u8(180..255); let c = Color::new(g,g,g).to_hex(); stars.push(Star { x, y, c }); }
        Self { stars, w, h }
    }
    pub fn render(&self, fb: &mut Framebuffer) {
        // gradient background
        for y in 0..self.h { let t = y as f32 / (self.h as f32 - 1.0); let c = Color::from_float(0.02*(1.0-t)+0.0*t, 0.02*(1.0-t)+0.0*t, 0.05*(1.0-t)+0.0*t).to_hex(); fb.set_current_color(c); for x in 0..self.w { fb.point_no_depth(x as i32, y as i32); } }
        // stars
        for s in &self.stars { fb.set_current_color(s.c); fb.point_no_depth(s.x, s.y); }
    }
}
