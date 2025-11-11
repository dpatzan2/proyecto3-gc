use nalgebra_glm::{Vec2, Vec3};
use tobj;
use crate::vertex::Vertex;

pub struct Obj { meshes: Vec<Mesh> }
struct Mesh { vertices: Vec<Vec3>, normals: Vec<Vec3>, texcoords: Vec<Vec2>, indices: Vec<u32> }

impl Obj {
    pub fn load(filename: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let (models, _mats) = tobj::load_obj(filename, &tobj::LoadOptions { single_index: true, triangulate: true, ..Default::default() })?;
        let meshes = models.into_iter().map(|m| { let mesh = m.mesh; Mesh { vertices: mesh.positions.chunks(3).map(|v| Vec3::new(v[0], v[1], v[2])).collect(), normals: mesh.normals.chunks(3).map(|n| Vec3::new(n[0], n[1], n[2])).collect(), texcoords: mesh.texcoords.chunks(2).map(|t| Vec2::new(t[0], 1.0 - t[1])).collect(), indices: mesh.indices } }).collect();
        Ok(Obj { meshes })
    }
    pub fn get_vertex_array(&self) -> Vec<Vertex> { let mut v = Vec::new(); for mesh in &self.meshes { for &idx in &mesh.indices { let i = idx as usize; let pos = mesh.vertices[i]; let normal = mesh.normals.get(i).copied().unwrap_or_else(|| pos.normalize()); v.push(Vertex::new(pos, normal)); } } v }
}
