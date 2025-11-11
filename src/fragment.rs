use nalgebra_glm::{Vec2, Vec3};
use crate::color::Color;

#[derive(Clone, Debug)]
pub struct Fragment {
    pub position: Vec2,
    pub depth: f32,
    pub normal: Vec3,
    pub intensity: f32,
    pub vertex_position: Vec3,
    pub color: Color,
}

impl Fragment { pub fn new(position: Vec2, depth: f32, normal: Vec3, intensity: f32, vertex_position: Vec3) -> Self { Self { position, depth, normal, intensity, vertex_position, color: Color::new(0,0,0) } } }
