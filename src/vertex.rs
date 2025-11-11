use nalgebra_glm::Vec3;
use crate::color::Color;

#[derive(Clone, Debug)]
pub struct Vertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub transformed_position: Vec3,
    pub transformed_normal: Vec3,
    pub color: Color,
}

impl Vertex { pub fn new(position: Vec3, normal: Vec3) -> Self { Self { position, normal, transformed_position: position, transformed_normal: normal, color: Color::new(0,0,0) } } }
