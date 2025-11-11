use crate::{fragment::Fragment, vertex::Vertex};
use nalgebra_glm::{dot, Vec2, Vec3};

fn edge(a: &Vec3, b: &Vec3, c: &Vec3) -> f32 { (c.x - a.x) * (b.y - a.y) - (c.y - a.y) * (b.x - a.x) }

pub fn triangle_stream<F: FnMut(&Fragment) -> ()>(v1: &Vertex, v2: &Vertex, v3: &Vertex, screen_w: usize, screen_h: usize, mut emit: F) {
    let a = v1.transformed_position; let b = v2.transformed_position; let c = v3.transformed_position;
   
    if !a.x.is_finite() || !a.y.is_finite() || !b.x.is_finite() || !b.y.is_finite() || !c.x.is_finite() || !c.y.is_finite() { return; }
    let area = edge(&a,&b,&c);
    if area.abs() < 1e-6 { return; }
   
    if area < 0.0 { return; }
  
    let mut min_x = a.x.min(b.x).min(c.x).floor() as i32;
    let mut min_y = a.y.min(b.y).min(c.y).floor() as i32;
    let mut max_x = a.x.max(b.x).max(c.x).ceil() as i32;
    let mut max_y = a.y.max(b.y).max(c.y).ceil() as i32;
    if max_x < 0 || max_y < 0 || min_x as usize >= screen_w || min_y as usize >= screen_h { return; }
    if min_x < 0 { min_x = 0; } if min_y < 0 { min_y = 0; }
    if max_x >= screen_w as i32 { max_x = screen_w as i32 - 1; }
    if max_y >= screen_h as i32 { max_y = screen_h as i32 - 1; }
    if min_x > max_x || min_y > max_y { return; }
    let light_dir = Vec3::new(0.0, 0.0, 1.0);
    for y in min_y..=max_y { for x in min_x..=max_x {
        let p = Vec3::new(x as f32 + 0.5, y as f32 + 0.5, 0.0);
        let w1 = edge(&b, &c, &p) / area; let w2 = edge(&c, &a, &p) / area; let w3 = edge(&a, &b, &p) / area;
        if w1 >= 0.0 && w2 >= 0.0 && w3 >= 0.0 {
            let normal = (v1.transformed_normal * w1 + v2.transformed_normal * w2 + v3.transformed_normal * w3).normalize();
            let intensity = dot(&normal, &light_dir).max(0.0);
            let depth = a.z * w1 + b.z * w2 + c.z * w3;
            let vertex_position = v1.position * w1 + v2.position * w2 + v3.position * w3;
            emit(&Fragment::new(Vec2::new(x as f32, y as f32), depth, normal, intensity, vertex_position));
        }
    }}
}
