use crate::{color::Color, fragment::Fragment, vertex::Vertex, Uniforms};
use nalgebra_glm::{mat4_to_mat3, Vec3, Vec4, Mat3};

pub fn vertex_shader(vertex: &Vertex, uniforms: &Uniforms) -> Vertex {
    let pos4 = Vec4::new(vertex.position.x, vertex.position.y, vertex.position.z, 1.0);
    let clip = uniforms.projection_matrix * uniforms.view_matrix * uniforms.model_matrix * pos4;
    let ndc = Vec4::new(clip.x / clip.w, clip.y / clip.w, clip.z / clip.w, 1.0);
    let screen = uniforms.viewport_matrix * ndc;
    let model3 = mat4_to_mat3(&uniforms.model_matrix);
    let normal_matrix: Mat3 = model3.transpose().try_inverse().unwrap_or(Mat3::identity());
    let transformed_normal = normal_matrix * vertex.normal;
    Vertex { position: vertex.position, normal: vertex.normal, color: vertex.color, transformed_position: Vec3::new(screen.x, screen.y, screen.z), transformed_normal }
}

pub fn lambert(base: Color, fragment: &Fragment) -> Color {
    let light_pos = Vec3::new(0.0, 0.0, 20.0);
    let l = (light_pos - fragment.vertex_position).normalize();
    let n = fragment.normal.normalize();
    let diff = n.dot(&l).max(0.0);
    let ambient = 0.2;
    base * (ambient + diff * 0.8)
}

pub fn fragment_solid(color: Color, fragment: &Fragment) -> Color { lambert(color, fragment) }

// Helpers
fn lerp_color(a: Color, b: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    Color {
        r: (a.r as f32 + (b.r as f32 - a.r as f32) * t).round() as u8,
        g: (a.g as f32 + (b.g as f32 - a.g as f32) * t).round() as u8,
        b: (a.b as f32 + (b.b as f32 - a.b as f32) * t).round() as u8,
    }
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn saturate(x: f32) -> f32 { x.clamp(0.0, 1.0) }

fn sph_lon_lat(p: Vec3) -> (f32, f32) {
    // Returns (lon, lat) where:
    // lon in [-PI, PI], lat in [-PI/2, PI/2]
    let n = p.normalize();
    let lon = n.z.atan2(n.x);
    let lat = n.y.asin();
    (lon, lat)
}

fn wrap_pi(x: f32) -> f32 { // wrap to [-PI, PI]
    let mut v = x;
    let two_pi = std::f32::consts::TAU;
    v = (v + std::f32::consts::PI) % two_pi;
    if v < 0.0 { v += two_pi; }
    v - std::f32::consts::PI
}

// Star
pub fn fragment_star(fragment: &Fragment, uniforms: &Uniforms) -> Color {
    // Physically-inspired emissive sun with limb darkening and animated granulation
    let t = uniforms.time * 0.001;

    // Model-space point on the sphere for noise sampling
    let p = fragment.vertex_position;

    // World-space position for view-dependent effects
    let wp4 = uniforms.model_matrix * Vec4::new(p.x, p.y, p.z, 1.0);
    let world_pos = Vec3::new(wp4.x, wp4.y, wp4.z);

    // View direction (towards the camera)
    let view_dir = (uniforms.camera_pos - world_pos).normalize();
    let n = fragment.normal.normalize(); // already in world space
    let mu = n.dot(&view_dir).clamp(0.0, 1.0); // cos(theta) for limb darkening

    // Base blackbody-like colors: hot white core -> orange rim
    let col_core = Color::from_float(1.0, 0.97, 0.90);
    let col_mid  = Color::from_float(1.0, 0.78, 0.35);
    let col_rim  = Color::from_float(1.0, 0.55, 0.12);

    // Limb darkening (simple quadratic law): I(mu) = 1 - u1(1-mu) - u2(1-mu)^2
    let one_minus_mu = 1.0 - mu;
    let u1 = 0.60; let u2 = 0.12;
    let _limb = (1.0 - u1 * one_minus_mu - u2 * one_minus_mu * one_minus_mu).clamp(0.0, 1.0);

    // Color gradient across disk based on viewing angle
    let grad_t = one_minus_mu.powf(0.85);
    let base_grad = lerp_color(col_core, col_mid, grad_t * 0.7);
    let mut col = lerp_color(base_grad, col_rim, grad_t * 0.6);

    // Large-scale convection cells
    let n_base = uniforms.noises[0].get_noise_3d(p.x * 2.0 + t * 0.5, p.y * 2.0, p.z * 2.0 - t * 0.5);
    col = lerp_color(col, Color::from_float(1.0, 0.66, 0.20), ((n_base + 1.0) * 0.5).clamp(0.0, 1.0) * 0.35);

    // Sunspots (cooler/darker areas), sparse and softer near the rim
    if uniforms.noises.len() > 1 {
        let n_spot = uniforms.noises[1].get_noise_3d(p.x * 2.6 - t * 0.35, p.y * 2.6, p.z * 2.6 + t * 0.30);
        let spot_mask = smoothstep(0.25, 0.55, n_spot.abs()) * (0.6 + 0.4 * mu); // fewer at rim
        col = lerp_color(col, Color::from_float(0.15, 0.10, 0.06), spot_mask * 0.55);
    }

    // Fine granulation (bright cells)
    if uniforms.noises.len() > 2 {
        let n_gran = uniforms.noises[2].get_noise_3d(p.x * 22.0, p.y * 22.0, p.z * 22.0);
        let gran_bright = smoothstep(0.40, 0.9, (n_gran + 1.0) * 0.5);
        col = lerp_color(col, Color::from_float(1.0, 0.98, 0.86), gran_bright * 0.40);
    }

    // Prominence hint near limb (still within disk)
    let rim = one_minus_mu.powf(1.4);
    let rim_glow = (0.15 + 0.85 * rim).clamp(0.0, 1.0);
    col = lerp_color(col, Color::from_float(1.0, 0.92, 0.75), rim_glow * 0.25);

    // Subtle pulsation
    let pulse = 0.95 + (t * 0.55).sin() * 0.05;

    // Emissive output (no Lambert shading)
    col * pulse
}

// Rocky generic
pub fn fragment_rocky(fragment: &Fragment, uniforms: &Uniforms) -> Color {
    let p = fragment.vertex_position;
    let base = uniforms.noises[0].get_noise_3d(p.x * 0.7, p.y * 0.7, p.z * 0.7);
    let detail = uniforms.noises[1].get_noise_3d(p.x * 2.0, p.y * 2.0, p.z * 2.0);
    let h = ((base * 0.7 + detail * 0.3) + 1.0) * 0.5;
    let lat = ((p.y + 1.0) * 0.5).clamp(0.0, 1.0);
    let sea = 0.52; let shore = 0.03;
    let ocean_deep = Color::from_float(0.05, 0.10, 0.30);
    let ocean_shallow = Color::from_float(0.10, 0.45, 0.75);
    let mut col;
    if h < sea {
        let d = ((sea - h) / shore).clamp(0.0, 1.0);
        col = lerp_color(ocean_shallow, ocean_deep, d);
    } else {
        let elev = ((h - sea) / (1.0 - sea)).clamp(0.0, 1.0);
        let moisture = if uniforms.noises.len() > 2 { ((uniforms.noises[2].get_noise_3d(p.x * 1.2, p.y * 1.2, p.z * 1.2) + 1.0) * 0.5).clamp(0.0, 1.0) } else { 0.5 };
        let temp = 1.0 - (lat - 0.5).abs() * 2.0;
        let desert_factor = smoothstep(0.4, 0.8, (1.0 - moisture) * temp);
        let grass_factor = smoothstep(0.3, 0.7, moisture * temp) * (1.0 - elev * 0.7);
        let desert = Color::from_float(0.73, 0.64, 0.40);
        let grass = Color::from_float(0.20, 0.50, 0.25);
        let dirt  = Color::from_float(0.42, 0.33, 0.26);
        let land_base = lerp_color(dirt, grass, grass_factor);
        let land_biome = lerp_color(land_base, desert, desert_factor * 0.8);
        let mountain = Color::from_float(0.62, 0.60, 0.58);
        let m_fac = (elev * 1.3).clamp(0.0, 1.0).powf(1.6);
        col = lerp_color(land_biome, mountain, m_fac);
        let snow = Color::from_float(0.96, 0.97, 1.0);
        let polar = smoothstep(0.65, 0.9, (lat - 0.5).abs() * 2.0);
        let snow_alt = smoothstep(0.7, 0.9, elev);
        let s_fac = (polar * 0.7 + snow_alt * 0.6).clamp(0.0, 1.0);
        col = lerp_color(col, snow, s_fac);
        if uniforms.noises.len() > 3 {
            let n_cloud = uniforms.noises[3].get_noise_3d(p.x * 4.0, p.y * 4.0, p.z * 4.0);
            let c = smoothstep(0.55, 0.75, (n_cloud + 1.0) * 0.5);
            col = lerp_color(col, Color::from_float(1.0, 1.0, 1.0), c * 0.20);
        }
    }
    lambert(col, fragment)
}

pub fn fragment_mercury(fragment: &Fragment, uniforms: &Uniforms) -> Color {
    let p = fragment.vertex_position;
    // Basalt/dust palette and roughness
    let base = uniforms.noises[0].get_noise_3d(p.x * 1.3, p.y * 1.3, p.z * 1.3);
    let detail = uniforms.noises[1].get_noise_3d(p.x * 4.0, p.y * 4.0, p.z * 4.0);
    let h = ((base * 0.6 + detail * 0.4) + 1.0) * 0.5;
    let bedrock = Color::from_float(0.40, 0.35, 0.31);
    let dust    = Color::from_float(0.70, 0.62, 0.52);
    let mut col = lerp_color(bedrock, dust, (h * 1.1).clamp(0.0, 1.0));
    // Crater approximation: ridged noise + rim accent
    let ridged = if uniforms.noises.len() > 2 { 1.0 - (uniforms.noises[2].get_noise_3d(p.x * 7.5, p.y * 7.5, p.z * 7.5)).abs() } else { 0.0 };
    let rim = smoothstep(0.65, 0.88, ridged);
    col = lerp_color(col, Color::from_float(0.18, 0.16, 0.15), rim * 0.55);
    // Bright ejecta
    let ejecta = if uniforms.noises.len() > 3 { ((uniforms.noises[3].get_noise_3d(p.x * 6.0, p.y * 6.0, p.z * 6.0) + 1.0) * 0.5).clamp(0.0, 1.0) } else { 0.0 };
    col = lerp_color(col, Color::from_float(0.82, 0.78, 0.70), ejecta * 0.12);
    lambert(col, fragment)
}

pub fn fragment_venus(fragment: &Fragment, uniforms: &Uniforms) -> Color {
    let p = fragment.vertex_position; let tsec = uniforms.time * 0.001;
    // Dense, slowly churning clouds
    let swirl1 = uniforms.noises[3].get_noise_3d(p.x * 2.2 + tsec * 0.10, p.y * 2.2, p.z * 2.2 - tsec * 0.08);
    let swirl2 = if uniforms.noises.len() > 2 { uniforms.noises[2].get_noise_3d(p.x * 3.0 - tsec * 0.06, p.y * 3.0, p.z * 3.0 + tsec * 0.05) } else { 0.0 };
    let t1 = ((swirl1 + 1.0) * 0.5).clamp(0.0, 1.0); let t2 = ((swirl2 + 1.0) * 0.5).clamp(0.0, 1.0);
    let c_lo = Color::from_float(0.88, 0.74, 0.46); let c_hi = Color::from_float(0.97, 0.90, 0.72);
    let mut base = lerp_color(c_lo, c_hi, (t1 * 0.6 + t2 * 0.4).clamp(0.0, 1.0));
    // Soft latitudinal bands
    let (lon, lat) = sph_lon_lat(p); let _ = lon; // suppress unused
    let band = ((lat * 10.0).sin() + 1.0) * 0.5; base = lerp_color(base, Color::from_float(1.0, 0.96, 0.84), band * 0.10);
    // High-altitude haze
    let haze = uniforms.noises[1].get_noise_3d(p.x * 1.2, p.y * 1.2, p.z * 1.2);
    base = lerp_color(base, Color::from_float(1.0, 0.98, 0.92), saturate((haze + 1.0) * 0.5) * 0.12);
    lambert(base, fragment)
}

pub fn fragment_earth(fragment: &Fragment, uniforms: &Uniforms) -> Color {
    let p = fragment.vertex_position;
    let base = uniforms.noises[0].get_noise_3d(p.x * 0.7, p.y * 0.7, p.z * 0.7);
    let detail = uniforms.noises[1].get_noise_3d(p.x * 2.0, p.y * 2.0, p.z * 2.0);
    let h = ((base * 0.7 + detail * 0.3) + 1.0) * 0.5; let lat = ((p.y + 1.0) * 0.5).clamp(0.0, 1.0);
    let sea = 0.54; let shore = 0.035; let ocean_deep = Color::from_float(0.03, 0.08, 0.25); let ocean_shallow = Color::from_float(0.12, 0.52, 0.85);
    let mut col;
    if h < sea { let d = ((sea - h) / shore).clamp(0.0, 1.0); col = lerp_color(ocean_shallow, ocean_deep, d); }
    else {
        let elev = ((h - sea) / (1.0 - sea)).clamp(0.0, 1.0);
        let moisture = if uniforms.noises.len() > 2 { ((uniforms.noises[2].get_noise_3d(p.x * 1.2, p.y * 1.2, p.z * 1.2) + 1.0) * 0.5).clamp(0.0, 1.0) } else { 0.5 };
        let temp = 1.0 - (lat - 0.5).abs() * 2.0; let desert_factor = smoothstep(0.4, 0.85, (1.0 - moisture) * temp); let grass_factor = smoothstep(0.25, 0.65, moisture * temp) * (1.0 - elev * 0.6);
        let desert = Color::from_float(0.85, 0.76, 0.45); let grass = Color::from_float(0.18, 0.55, 0.24); let dirt  = Color::from_float(0.40, 0.33, 0.26);
        let land_base = lerp_color(dirt, grass, grass_factor); let land_biome = lerp_color(land_base, desert, desert_factor * 0.7);
        let mountain = Color::from_float(0.62, 0.60, 0.58); let m_fac = (elev * 1.2).clamp(0.0, 1.0).powf(1.6);
        col = lerp_color(land_biome, mountain, m_fac);
        if uniforms.noises.len() > 3 { let n_cloud = uniforms.noises[3].get_noise_3d(p.x * 4.0, p.y * 4.0, p.z * 4.0); let c = smoothstep(0.55, 0.75, (n_cloud + 1.0) * 0.5); col = lerp_color(col, Color::from_float(1.0, 1.0, 1.0), c * 0.22); }
    }
    lambert(col, fragment)
}

pub fn fragment_mars(fragment: &Fragment, uniforms: &Uniforms) -> Color {
    let p = fragment.vertex_position;
    let base = uniforms.noises[0].get_noise_3d(p.x * 0.9, p.y * 0.9, p.z * 0.9);
    let detail = uniforms.noises[1].get_noise_3d(p.x * 3.0, p.y * 3.0, p.z * 3.0);
    let h = ((base * 0.65 + detail * 0.35) + 1.0) * 0.5;
    let rust1 = Color::from_float(0.60, 0.30, 0.18); let rust2 = Color::from_float(0.82, 0.46, 0.26); let dust  = Color::from_float(0.88, 0.62, 0.44);
    let mut col = lerp_color(lerp_color(rust1, rust2, h), dust, (h * 0.45).clamp(0.0, 1.0));
    // Dark maria
    let maria = if uniforms.noises.len() > 2 { uniforms.noises[2].get_noise_3d(p.x * 1.6, p.y * 1.6, p.z * 1.6) } else { 0.0 };
    let m = smoothstep(0.4, 0.7, (maria + 1.0) * 0.5);
    col = lerp_color(col, Color::from_float(0.35, 0.22, 0.18), m * 0.30);
    // Polar caps
    let (_lon, lat_ang) = sph_lon_lat(p);
    let polar = smoothstep(0.9, 1.2, lat_ang.abs());
    col = lerp_color(col, Color::from_float(0.96, 0.97, 0.99), polar * 0.8);
    // Dust storms
    let storms = if uniforms.noises.len() > 3 { uniforms.noises[3].get_noise_3d(p.x * 1.4, p.y * 1.4, p.z * 1.4) } else { 0.0 };
    let s = smoothstep(0.55, 0.8, (storms + 1.0) * 0.5); col = lerp_color(col, Color::from_float(0.94, 0.78, 0.60), s * 0.22);
    lambert(col, fragment)
}

pub fn fragment_gas(fragment: &Fragment, uniforms: &Uniforms) -> Color {
    let p = fragment.vertex_position;
    let bands = (p.y * 7.0 + uniforms.noises[0].get_noise_3d(p.x * 0.7, p.y * 0.7, p.z * 0.7) * 1.2).sin();
    let t = ((bands + 1.0) * 0.5).clamp(0.0, 1.0);
    let c1 = Color::from_float(0.78, 0.62, 0.48); let c2 = Color::from_float(0.96, 0.88, 0.76);
    let mut col = lerp_color(c1, c2, t);
    let fine = ((p.y * 24.0 + uniforms.noises[0].get_noise_3d(p.x * 0.5, p.y * 0.5, p.z * 0.5) * 0.6).sin() + 1.0) * 0.5; col = lerp_color(col, Color::from_float(1.0, 0.96, 0.88), fine * 0.18);
    let d = ((uniforms.noises[1].get_noise_3d(p.x * 1.7, p.y * 1.4, p.z * 1.6) + 1.0) * 0.5).clamp(0.0, 1.0); col = lerp_color(col, Color::from_float(1.0, 1.0, 1.0), d * 0.12);
    if uniforms.noises.len() > 2 { let s = uniforms.noises[2].get_noise_3d(p.x * 0.9 + 1.3, p.y * 0.7 - 0.7, p.z * 0.9); let mask = smoothstep(0.5, 0.8, s.abs()); col = lerp_color(col, Color::from_float(0.30, 0.27, 0.25), mask * 0.45); }
    lambert(col, fragment)
}

pub fn fragment_jupiter(fragment: &Fragment, uniforms: &Uniforms) -> Color {
    let p = fragment.vertex_position; let t = uniforms.time * 0.001;
    let (lon, lat0) = sph_lon_lat(p);
    // Domain-warped latitude to break straight lines
    let warp1 = uniforms.noises[0].get_noise_3d(p.x * 1.2, p.y * 0.8, p.z * 1.2) * 0.12;
    let warp2 = uniforms.noises[1].get_noise_3d(p.x * 3.0 + 0.7, p.y * 2.5 - 1.1, p.z * 2.8) * 0.05;
    let shear = (lon * 8.0 + t * 0.8).sin() * 0.03;
    let lat = (lat0 + warp1 + warp2 + shear).clamp(-std::f32::consts::FRAC_PI_2, std::f32::consts::FRAC_PI_2);

    // Zonal bands with contrast modulation
    let base_band = (lat * 20.0).sin();
    let contrast = ((uniforms.noises[1].get_noise_3d(p.x * 1.0, p.y * 1.0, p.z * 1.0) + 1.0) * 0.5).clamp(0.0, 1.0);
    let band = base_band * (0.75 + 0.35 * contrast);
    let tband = ((band + 1.0) * 0.5).clamp(0.0, 1.0);
    let light = Color::from_float(0.96, 0.86, 0.72);
    let dark  = Color::from_float(0.66, 0.40, 0.22);
    let mut col = lerp_color(dark, light, tband);

    // Fine meanders and ammonia streaks
    let fine = ((lat * 70.0 + uniforms.noises[1].get_noise_3d(p.x * 0.6, p.y * 0.6, p.z * 0.6) * 2.2 + t * 1.2).sin() + 1.0) * 0.5;
    col = lerp_color(col, Color::from_float(1.0, 0.96, 0.90), fine * 0.18);

    // Great Red Spot (approximate oval with swirl halo)
    let grs_lat = -0.18; let grs_lon = 0.7;
    let dx = wrap_pi(lon - grs_lon) * lat.cos(); let dy = lat0 - grs_lat;
    let e = (dx*dx)/(0.18*0.18) + (dy*dy)/(0.10*0.10);
    let core = (1.0 - smoothstep(0.7, 1.0, e)).clamp(0.0, 1.0);
    let halo = (1.0 - smoothstep(1.1, 1.5, e)).clamp(0.0, 1.0);
    col = lerp_color(col, Color::from_float(0.86, 0.40, 0.18), core * 0.9);
    col = lerp_color(col, Color::from_float(1.0, 0.94, 0.88), halo * 0.25);

    lambert(col, fragment)
}

pub fn fragment_saturn(fragment: &Fragment, uniforms: &Uniforms) -> Color {
    let p = fragment.vertex_position; let t = uniforms.time * 0.001;
    let (lon, lat0) = sph_lon_lat(p); let _ = lon;
    let warp = uniforms.noises[0].get_noise_3d(p.x * 1.0, p.y * 0.8, p.z * 1.0) * 0.08 + (lon * 6.0 + t * 0.5).sin() * 0.02;
    let lat = lat0 + warp;
    let band = (lat * 16.0).sin();
    let t = ((band + 1.0) * 0.5).clamp(0.0, 1.0);
    let pale = Color::from_float(0.94, 0.90, 0.76); let gold = Color::from_float(0.82, 0.74, 0.58);
    let mut col = lerp_color(gold, pale, t * 0.85);
    // Polar hex hint (very subtle)
    let lat_abs = lat0.abs();
    let hex = ((lon * 6.0).cos() * (1.3 - (lat_abs * 6.0))).clamp(0.0, 1.0);
    col = lerp_color(col, Color::from_float(0.80, 0.72, 0.60), hex * 0.03);
    // Soft haze
    let haze = ((uniforms.noises[1].get_noise_3d(p.x * 1.1, p.y * 1.1, p.z * 1.1) + 1.0) * 0.5).clamp(0.0, 1.0);
    col = lerp_color(col, Color::from_float(1.0, 0.98, 0.90), haze * 0.10);
    lambert(col, fragment)
}

pub fn fragment_uranus(fragment: &Fragment, uniforms: &Uniforms) -> Color {
    let p = fragment.vertex_position; let t = uniforms.time * 0.001;
    let (lon, lat0) = sph_lon_lat(p); let _ = lon;
    let lat = lat0 + uniforms.noises[0].get_noise_3d(p.x * 0.8, p.y * 0.8, p.z * 0.8) * 0.04 + (lon * 4.0 + t * 0.4).sin() * 0.01;
    let base = Color::from_float(0.52, 0.86, 0.90);
    let bands = ((lat * 9.0).sin() + 1.0) * 0.5;
    let mut col = lerp_color(base, Color::from_float(0.72, 0.94, 0.97), bands * 0.12);
    let haze = ((uniforms.noises[1].get_noise_3d(p.x * 1.0, p.y * 1.0, p.z * 1.0) + 1.0) * 0.5).clamp(0.0, 1.0);
    col = lerp_color(col, Color::from_float(1.0, 1.0, 1.0), haze * 0.05);
    // Faint polar brightening
    let polar = lat0.abs();
    col = lerp_color(col, Color::from_float(0.85, 0.98, 1.0), smoothstep(1.1, 1.5, polar) * 0.12);
    lambert(col, fragment)
}

pub fn fragment_neptune(fragment: &Fragment, uniforms: &Uniforms) -> Color {
    let p = fragment.vertex_position; let t = uniforms.time * 0.001;
    let (lon, lat0) = sph_lon_lat(p);
    let lat = lat0 + uniforms.noises[0].get_noise_3d(p.x * 0.9, p.y * 0.9, p.z * 0.9) * 0.05 + (lon * 5.0 + t * 0.6).sin() * 0.015;
    let base = Color::from_float(0.06, 0.20, 0.55);
    let bands = (lat * 11.0 + uniforms.noises[1].get_noise_3d(p.x * 0.6, p.y * 0.6, p.z * 0.6) * 0.7).sin();
    let t = ((bands + 1.0) * 0.5).clamp(0.0, 1.0);
    let mut col = lerp_color(base, Color::from_float(0.12, 0.45, 0.95), t * 0.45);
    // Dark spot with bright rim
    let ds_lat = -0.25; let ds_lon = 1.2;
    let dx = wrap_pi(lon - ds_lon) * lat.cos(); let dy = lat0 - ds_lat;
    let e = (dx*dx)/(0.18*0.18) + (dy*dy)/(0.10*0.10);
    let core = (1.0 - smoothstep(0.75, 1.0, e)).clamp(0.0, 1.0);
    let rim  = (1.0 - smoothstep(1.0, 1.3, e)).clamp(0.0, 1.0);
    col = lerp_color(col, Color::from_float(0.02, 0.10, 0.28), core * 0.9);
    col = lerp_color(col, Color::from_float(0.85, 0.95, 1.0), rim * 0.20);
    lambert(col, fragment)
}

pub fn fragment_moon(fragment: &Fragment, uniforms: &Uniforms) -> Color {
    let p = fragment.vertex_position;
    let n1 = ((p.x * 2.0 + p.y * 2.0 + p.z * 2.0).sin() * 0.5 + 0.5).clamp(0.0, 1.0);
    let n = if !uniforms.noises.is_empty() { let v = uniforms.noises[0].get_noise_3d(p.x * 1.2, p.y * 1.2, p.z * 1.2); ((v + 1.0) * 0.5).clamp(0.0, 1.0) } else { n1 };
    let base = Color::from_float(0.65, 0.65, 0.67); let dark = Color::from_float(0.25, 0.25, 0.27);
    let col = lerp_color(dark, base, n);
    lambert(col, fragment)
}

// Distinctive small-body shader for asteroids: dark rocky base with glints and phase brightening
pub fn fragment_asteroid(fragment: &Fragment, uniforms: &Uniforms) -> Color {
    let p = fragment.vertex_position;
    // World position for lighting wrt the Sun at origin
    let wp4 = uniforms.model_matrix * Vec4::new(p.x, p.y, p.z, 1.0);
    let world_pos = Vec3::new(wp4.x, wp4.y, wp4.z);

    let n = fragment.normal.normalize();
    let l = (-world_pos).normalize(); // light from Sun at origin
    let v = (uniforms.camera_pos - world_pos).normalize();

    // Albedo: dark carbonaceous rock with mottling
    let base_n = if !uniforms.noises.is_empty() { uniforms.noises[0].get_noise_3d(p.x * 3.0, p.y * 3.0, p.z * 3.0) } else { 0.0 };
    let detail_n = if uniforms.noises.len() > 1 { uniforms.noises[1].get_noise_3d(p.x * 9.0, p.y * 9.0, p.z * 9.0) } else { 0.0 };
    let h = ((base_n * 0.7 + detail_n * 0.3) + 1.0) * 0.5;
    let dark = Color::from_float(0.16, 0.16, 0.18);
    let mid  = Color::from_float(0.28, 0.28, 0.30);
    let mut col = lerp_color(dark, mid, (h * 1.1).clamp(0.0, 1.0));

    // Lambert diffuse
    let diff = n.dot(&l).max(0.0);
    let ambient = 0.12;

    // Specular glints (sparse, noise-modulated shininess)
    let shininess = 12.0 + (detail_n.abs() * 28.0);
    let r = (n * (2.0 * n.dot(&l)) - l).normalize();
    let spec = v.dot(&r).max(0.0).powf(shininess) * 0.85;

    // Opposition surge (more bright when phase angle small)
    let phase = v.dot(&l).clamp(-1.0, 1.0); // 1 when Sun behind camera
    let opposition = ((phase - 0.8) / 0.2).clamp(0.0, 1.0).powf(2.0) * 0.6;

    // Rim light when backlit
    let rim = (1.0 - n.dot(&v).clamp(0.0, 1.0)).powf(2.0) * (0.4 * (1.0 - diff));

    let light = ambient + diff * 0.9 + spec + opposition + rim;
    col * light.clamp(0.0, 1.6)
}

pub fn fragment_ring(fragment: &Fragment, _uniforms: &Uniforms) -> Color {
    let p = fragment.vertex_position;
    let r = (p.x * p.x + (p.z / 1.2) * (p.z / 1.2)).sqrt();
    let r_norm = ((r - 3.4) / (6.1 - 3.4)).clamp(0.0, 1.0);

    let c_ring = (-((r_norm - 0.18).powi(2)) / (2.0 * 0.05 * 0.05)).exp();
    let b_ring = (-((r_norm - 0.52).powi(2)) / (2.0 * 0.09 * 0.09)).exp();
    let a_ring = (-((r_norm - 0.82).powi(2)) / (2.0 * 0.07 * 0.07)).exp();
    let cassini = smoothstep(0.58, 0.68, r_norm) * (1.0 - smoothstep(0.68, 0.74, r_norm));
    let density = (c_ring * 0.5 + b_ring * 1.2 + a_ring * 0.9) * (1.0 - cassini * 0.8);

    let fine = (r * 40.0).sin() * 0.04 + (r * 95.0).sin() * 0.02;
    let dens = (density + fine).clamp(0.0, 1.0);


    let tint_inner = Color::from_float(0.80, 0.76, 0.68);
    let tint_outer = Color::from_float(0.65, 0.60, 0.54);
    let mut col = lerp_color(tint_inner, tint_outer, r_norm);
    col = lerp_color(Color::from_float(0.45, 0.42, 0.38), col, dens * 0.8);


    let n = fragment.normal.normalize();
    let l = Vec3::new(0.0, 0.0, 1.0);
    let diff = n.dot(&l).abs().max(0.15);
    col * (0.25 + diff * 0.75)
}
