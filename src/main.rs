mod color; mod framebuffer; mod fragment; mod vertex; mod triangle; mod obj; mod camera; mod shaders; mod skybox;

use color::Color; use framebuffer::Framebuffer; use fragment::Fragment; use vertex::Vertex; use triangle::triangle_stream; use obj::Obj; use camera::FreeOrbitCamera; use shaders::lambert; use skybox::Skybox;
use fastnoise_lite::{FastNoiseLite, FractalType, NoiseType};
use minifb::{Key, Window, WindowOptions};
use nalgebra_glm::{Mat4, Vec3, Vec4, vec3};
use image::{ImageBuffer, Rgb};


const ASTEROID_MATCH_VENUS_SCALE: f32 = 1.90;

pub struct Uniforms<'a> { pub model_matrix: Mat4, pub view_matrix: Mat4, pub projection_matrix: Mat4, pub viewport_matrix: Mat4, pub time: f32, pub noises: Vec<&'a FastNoiseLite>, pub camera_pos: Vec3 }

fn create_viewport_matrix(width: f32, height: f32) -> Mat4 { Mat4::new(width/2.0,0.0,0.0,width/2.0, 0.0,-height/2.0,0.0,height/2.0, 0.0,0.0,1.0,0.0, 0.0,0.0,0.0,1.0) }
fn create_model_matrix(translation: Vec3, scale: f32, rotation_y: f32) -> Mat4 { let (s,c) = rotation_y.sin_cos(); let rot_y = Mat4::new(c,0.0,s,0.0, 0.0,1.0,0.0,0.0, -s,0.0,c,0.0, 0.0,0.0,0.0,1.0); let transform = Mat4::new(scale,0.0,0.0,translation.x, 0.0,scale,0.0,translation.y, 0.0,0.0,scale,translation.z, 0.0,0.0,0.0,1.0); transform*rot_y }
fn create_model_matrix_euler(translation: Vec3, scale: f32, rx: f32, ry: f32, rz: f32) -> Mat4 { let (sx,cx) = rx.sin_cos(); let (sy,cy) = ry.sin_cos(); let (sz,cz) = rz.sin_cos(); let rxm = Mat4::new(1.0,0.0,0.0,0.0, 0.0,cx,-sx,0.0, 0.0,sx,cx,0.0, 0.0,0.0,0.0,1.0); let rym = Mat4::new(cy,0.0,sy,0.0, 0.0,1.0,0.0,0.0, -sy,0.0,cy,0.0, 0.0,0.0,0.0,1.0); let rzm = Mat4::new(cz,-sz,0.0,0.0, sz,cz,0.0,0.0, 0.0,0.0,1.0,0.0, 0.0,0.0,0.0,1.0); let s = Mat4::new(scale,0.0,0.0,translation.x, 0.0,scale,0.0,translation.y, 0.0,0.0,scale,translation.z, 0.0,0.0,0.0,1.0); s*rzm*rym*rxm }
fn create_noise_fbmn(seed: i32, freq: f32, octaves: i32) -> FastNoiseLite { let mut n = FastNoiseLite::with_seed(seed); n.set_noise_type(Some(NoiseType::Perlin)); n.set_fractal_type(Some(FractalType::FBm)); n.set_fractal_octaves(Some(octaves)); n.set_frequency(Some(freq)); n }


struct Lcg(u64);
impl Lcg { fn new(seed: u64) -> Self { Self(seed) } fn next_u32(&mut self) -> u32 { self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1); (self.0 >> 32) as u32 } fn next_f32(&mut self) -> f32 { (self.next_u32() as f32) / (u32::MAX as f32) } }

fn render<F: Fn(&Fragment) -> Color>(fb: &mut Framebuffer, uniforms: &Uniforms, vertex_array: &[Vertex], shader_fn: F) {
    // Vertex stage
    let mut transformed = Vec::with_capacity(vertex_array.len());
    for v in vertex_array { transformed.push(shaders::vertex_shader(v, uniforms)); }

    for i in (0..transformed.len()).step_by(3) {
        if i+2 < transformed.len() {
            let a=&transformed[i]; let b=&transformed[i+1]; let c=&transformed[i+2];
            let fbw = fb.width; let fbh = fb.height; let zbuf_ptr: *const f32 = fb.zbuffer.as_ptr();
            triangle_stream(a,b,c, fbw, fbh, |frag| {
                let x = frag.position.x as i32; let y = frag.position.y as i32;
                if x >= 0 && y >= 0 && (x as usize) < fbw && (y as usize) < fbh {
                    let idx = y as usize * fbw + x as usize;
                    let current_z = unsafe { *zbuf_ptr.add(idx) };
                    if frag.depth < current_z {
                        let color = shader_fn(frag).to_hex();
                        fb.set_current_color(color);
                        fb.point(x, y, frag.depth);
                    }
                }
            });
        }
    }
}

fn planet_color(index: usize) -> Color {
    match index {
        1 => Color::from_float(0.60, 0.54, 0.46), // Mercury
        2 => Color::from_float(0.93, 0.84, 0.62), // Venus
        3 => Color::from_float(0.25, 0.55, 0.28), // Earth 
        4 => Color::from_float(0.78, 0.42, 0.28), // Mars
        5 => Color::from_float(0.86, 0.74, 0.58), // Jupiter
        6 => Color::from_float(0.92, 0.86, 0.72), // Saturn
        7 => Color::from_float(0.56, 0.84, 0.88), // Uranus
        8 => Color::from_float(0.10, 0.36, 0.80), // Neptune
        _ => Color::from_float(0.7,0.7,0.7),
    }
}


struct Ship {
    pos: Vec3,
    yaw: f32,
    pitch: f32,
    roll: f32,
    vel: Vec3,
    yaw_vel: f32,
    roll_vel: f32,
}

struct Asteroid { pos: Vec3, scale: f32, rot_y: f32, vel: Vec3, alive: bool, exploding: bool, t: f32 }
impl Asteroid {
    fn new(pos: Vec3, scale: f32, rot_y: f32, vel: Vec3) -> Self { Self { pos, scale, rot_y, vel, alive: true, exploding: false, t: 0.0 } }
}

// Spawn an asteroid that will cross the player's view near the ship
fn spawn_asteroid_crossing_ship(ship: &Ship, rng: &mut Lcg) -> Asteroid {
    let (fwd, right, up) = ship.axes();
    // Spawn ahead of the ship, with lateral offset so it crosses the screen
    let ahead = 30.0 + rng.next_f32() * 20.0; // 30..50 units ahead (más cerca y visible)
    let off_x = (rng.next_f32() - 0.5) * 30.0; 
    let off_y = (rng.next_f32() - 0.5) * 4.0; 
    let pos = ship.pos + fwd * ahead + right * off_x + up * off_y;
    // Mismo tamaño que Venus
    let scale = ASTEROID_MATCH_VENUS_SCALE;
    let rot_y = rng.next_f32() * std::f32::consts::TAU;
    // Velocity toward the ship (so it passes by), with slight drift
    // Slightly slower to remain on screen longer
    let speed = 0.08 + rng.next_f32() * 0.06; // 0.08..0.14
    let drift_r = (rng.next_f32() - 0.5) * 0.025; // small sideways drift
    let drift_u = (rng.next_f32() - 0.5) * 0.015; // small vertical drift
    let vel = (-fwd * speed) + right * drift_r + up * drift_u;
    Asteroid::new(pos, scale, rot_y, vel)
}

impl Ship {
    fn new(pos: Vec3) -> Self { Self { pos, yaw: 0.0, pitch: 0.0, roll: 0.0, vel: vec3(0.0,0.0,0.0), yaw_vel: 0.0, roll_vel: 0.0 } }
    fn axes(&self) -> (Vec3, Vec3, Vec3) {
        let cp = self.pitch.cos();
        // Forward based on yaw/pitch
        let forward = vec3(self.yaw.cos()*cp, self.pitch.sin(), self.yaw.sin()*cp).normalize();
        let world_up = vec3(0.0,1.0,0.0);
        // Stable right even if forward ~ world_up
        let mut right = forward.cross(&world_up);
        if right.magnitude() < 1e-3 { right = forward.cross(&vec3(0.0,0.0,1.0)); }
        let right = right.normalize();
        let up = right.cross(&forward).normalize();
        // Apply roll: rotate right/up around forward by roll
        let right = rotate_around_axis(right, forward, self.roll);
        let up = rotate_around_axis(up, forward, self.roll);
        (forward, right, up)
    }
    fn update_controls(&mut self, window: &Window) {
        let (forward, right, up_axis) = self.axes();
        let mut acc = vec3(0.0,0.0,0.0);
        // Thrust forward/back
        if window.is_key_down(Key::W) { acc += forward * 0.02; }
        if window.is_key_down(Key::S) { acc -= forward * 0.02; }
        // Strafe left/right
        if window.is_key_down(Key::D) { acc += right * 0.015; }
        if window.is_key_down(Key::A) { acc -= right * 0.015; }
        // Up/Down
        if window.is_key_down(Key::R) { acc += up_axis * 0.015; }
        if window.is_key_down(Key::F) { acc -= up_axis * 0.015; }
        // Yaw smoothing and banking with arrows
    let yaw_accel = 0.0028; // softer lateral acceleration
        let mut yaw_acc = 0.0;
        if window.is_key_down(Key::Left)  { yaw_acc -= yaw_accel; }
        if window.is_key_down(Key::Right) { yaw_acc += yaw_accel; }
        // Integrate yaw velocity with damping
    self.yaw_vel = self.yaw_vel * 0.94 + yaw_acc; // a bit more damping
    let max_yaw_vel = 0.028; // lower cap for smoother turns
        if self.yaw_vel > max_yaw_vel { self.yaw_vel = max_yaw_vel; }
        if self.yaw_vel < -max_yaw_vel { self.yaw_vel = -max_yaw_vel; }
        self.yaw = (self.yaw + self.yaw_vel) % (std::f32::consts::TAU);
        // Auto-bank proportional to yaw rate
    let bank_target = (-self.yaw_vel * 9.5).clamp(-0.6, 0.6); // slightly softer banking
    let bank_resp = 0.06; let bank_damp = 0.90; // smoother response and damping
        self.roll_vel += (bank_target - self.roll) * bank_resp;
        self.roll_vel *= bank_damp;
        self.roll += self.roll_vel;
    // Pitch with arrows: Up increases pitch (nose up), Down decreases
    if window.is_key_down(Key::Up)    { self.pitch = (self.pitch + 0.015).clamp(-1.2, 1.2); }
    if window.is_key_down(Key::Down)  { self.pitch = (self.pitch - 0.015).clamp(-1.2, 1.2); }
        // Boost
        if window.is_key_down(Key::LeftShift) { acc *= 2.0; }
        self.vel += acc;
        // Damp to avoid runaway speeds
        self.vel *= 0.992;
       
        let speed = self.vel.magnitude();
        let max_speed = 1.2;
        if speed > max_speed { self.vel = self.vel / speed * max_speed; }
        self.pos += self.vel;
    }
}

fn clamp_ship_sphere(ship: &mut Ship, center: Vec3, radius: f32, margin: f32) {
    let to_center = ship.pos - center;
    let dist = to_center.magnitude();
    let min_dist = radius + margin;
    if dist < min_dist && dist > 1e-4 {
        let n = to_center / dist;
        ship.pos = center + n * min_dist;
        let inward = -n.dot(&ship.vel).max(0.0);
        ship.vel += n * inward;
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (w,h) = (900usize, 700usize);
    let mut window = Window::new("Proyecto 3 - Sistema", w, h, WindowOptions::default())?;
    let mut fb = Framebuffer::new(w,h);
    let viewport = create_viewport_matrix(w as f32, h as f32);
    let aspect = w as f32 / h as f32; let base_fov_deg = 45.0f32; let near = 0.1f32; let far = 2000.0f32;

    let mut camera = FreeOrbitCamera::new(vec3(0.0, 6.0, 24.0), vec3(0.0, 0.0, 0.0));
    // Ship 
    let mut ship = Ship::new(vec3(0.0, 0.0, 26.0));

    // Skybox
    let sky = Skybox::new(w,h, 1000, 12345);

    // Models
    let args: Vec<String> = std::env::args().collect();

    let sphere_path = if args.len() > 1 { args[1].clone() } else { 
        "assets/models/sphere.obj".to_string()
    };
    let ship_path = if args.len() > 2 { args[2].clone() } else { 
        "assets/models/SpaceShip.obj".to_string()
    };
    let sphere = Obj::load(&sphere_path)?; let sphere_vertices = sphere.get_vertex_array();
    let ship_mesh = Obj::load(&ship_path)?; let ship_vertices = ship_mesh.get_vertex_array();
    let asteroid_path_try = "assets/models/Asteoid.obj".to_string();
    let asteroid_path_fallback = "assets/models/Asteroid.obj".to_string();
    let asteroid_mesh = Obj::load(&asteroid_path_try).or_else(|_| Obj::load(&asteroid_path_fallback))?;
    let asteroid_vertices = asteroid_mesh.get_vertex_array();

    let mut asteroid_max_r = 0.0f32;
    for v in &asteroid_vertices { let l = v.position.magnitude(); if l > asteroid_max_r { asteroid_max_r = l; } }
    let asteroid_unit_scale = if asteroid_max_r > 1e-6 { 1.0 / asteroid_max_r } else { 1.0 };

    // Noises
    let star_base = create_noise_fbmn(42, 0.005, 6); let star_spots = create_noise_fbmn(43, 0.02, 5); let star_gran  = create_noise_fbmn(44, 0.08, 4);
    let rocky_base = create_noise_fbmn(7, 1.0, 5); let rocky_detail = create_noise_fbmn(8, 3.0, 3); let rocky_biome = create_noise_fbmn(9, 0.6, 3); let rocky_clouds = create_noise_fbmn(10, 0.9, 5);
    let gas_bands = create_noise_fbmn(99, 2.0, 2); let gas_detail = create_noise_fbmn(100, 1.2, 3); let gas_storms = create_noise_fbmn(101, 0.9, 4);

  
    let au_scale = 10.0f32; 
    let planets_au = [
        ("Sun",     0.00f32, 2.8f32, 0.0f32),
        ("Mercury", 0.39,    0.76,   0.95),
        ("Venus",   0.72,    1.90,   0.75),
        ("Earth",   1.00,    2.00,   0.62),
        ("Mars",    1.52,    1.06,   0.50),
        ("Jupiter", 3.20,    3.50,   0.35),
        ("Saturn",  5.28,    3.00,   0.28),
        ("Uranus",  7.20,   2.20,   0.22),
        ("Neptune", 10.05,   2.10,   0.20),
    ];
    let planets: Vec<(&str, f32, f32, f32)> = planets_au
        .iter()
        .map(|(n, au, s, spd)| (*n, au * au_scale, *s, *spd))
        .collect();

    let mut time = 0.0f32; let mut rotation = 0.0f32; let mut animate_orbits = true;

    // Asteroid field (keep only a couple at once)
    let mut rng = Lcg::new(0xC0FFEE12);
    let mut asteroids: Vec<Asteroid> = Vec::new();
    let max_asteroids = 2usize;
    for _ in 0..max_asteroids { asteroids.push(spawn_asteroid_crossing_ship(&ship, &mut rng)); }

    let mut cam_detached = false;
    let mut cam_warp_target: Option<(Vec3, Vec3)> = None; 
    let mut cam_warp_origin_eye = camera.eye; let mut cam_warp_origin_center = camera.center; let mut cam_warp_t = 0.0f32;
    let mut cam_follow_after_warp = false;
    let mut cam_follow_planet: Option<usize> = None;

    while window.is_open() && !window.is_key_down(Key::Escape) {
        time += 16.0; rotation += 0.01; fb.clear(0x000000);
    sky.render(&mut fb);
   
    ship.update_controls(&window);

 
        for (i, (_name, r, s, _spd)) in planets.iter().enumerate() {
            let key = match i {0=>Key::Key0,1=>Key::Key1,2=>Key::Key2,3=>Key::Key3,4=>Key::Key4,5=>Key::Key5,6=>Key::Key6,7=>Key::Key7,8=>Key::Key8,_=>Key::Unknown};
            if window.is_key_pressed(key, minifb::KeyRepeat::No) {
                let tsec = time*0.001; let a = tsec * *_spd;
                let center_t = if i==0 { vec3(0.0,0.0,0.0) } else { vec3(a.cos()* *r, 0.0, a.sin()* *r) };
                let eye_offset = vec3(0.0, s*2.5 + 4.0, s*3.5 + 7.0);
                let eye_t = center_t + eye_offset;
                cam_detached = true; cam_follow_after_warp = false; camera.up = vec3(0.0,1.0,0.0);
                cam_follow_planet = Some(i);
                cam_warp_origin_eye = camera.eye; cam_warp_origin_center = camera.center; cam_warp_target = Some((eye_t, center_t)); cam_warp_t = 0.0;
            }
        }
      
        let (fwd, _right, up_axis) = ship.axes();
        let cam_dist = 6.0; let cam_height = 2.2; let lookahead = 6.0;
        let follow_eye = ship.pos - fwd*cam_dist + up_axis*cam_height;
        let follow_center = ship.pos + fwd*lookahead;
        if window.is_key_pressed(Key::C, minifb::KeyRepeat::No) {
            if cam_detached { cam_warp_origin_eye = camera.eye; cam_warp_origin_center = camera.center; cam_warp_target = Some((follow_eye, follow_center)); cam_warp_t = 0.0; cam_follow_after_warp = true; }
            cam_follow_planet = None; // switch to following the ship after warp
        }
       
        if let Some((eye_t, center_t)) = cam_warp_target {
            cam_warp_t += 0.08; let t = cam_warp_t.min(1.0);
            let u = ease_in_out_cubic(t);
            let world_up = vec3(0.0,1.0,0.0);
        
            let eye_start = cam_warp_origin_eye; let eye_end = eye_t;
            let dir = eye_end - eye_start;
            let dist = dir.magnitude().max(1e-3);
            let dir_n = dir / dist;
            let mut right = dir_n.cross(&world_up);
            if right.magnitude() < 1e-3 { right = vec3(1.0,0.0,0.0); } else { right = right.normalize(); }
            let sign = if (eye_start.x + eye_end.z).sin() >= 0.0 { 1.0 } else { -1.0 };
            let amp = (dist * 0.25).clamp(5.0, 40.0);
            let eye_c1 = eye_start + dir_n * (amp*0.30) + right * (amp * 1.00 * sign);
            let eye_c2 = eye_end   - dir_n * (amp*0.30) + right * (amp * 0.60 * sign);
            let eye_pos = bezier3(eye_start, eye_c1, eye_c2, eye_end, u);

            let cen_start = cam_warp_origin_center; let cen_end = center_t;
            let cdir = cen_end - cen_start;
            let cdist = cdir.magnitude().max(1e-3);
            let cdir_n = cdir / cdist;
            let mut cright = cdir_n.cross(&world_up);
            if cright.magnitude() < 1e-3 { cright = vec3(1.0,0.0,0.0); } else { cright = cright.normalize(); }
            let camp = (cdist * 0.20).clamp(3.0, 25.0);
            let cen_c1 = cen_start + cdir_n * (camp*0.35) + cright * (camp * 0.6 * sign);
            let cen_c2 = cen_end   - cdir_n * (camp*0.35) + cright * (camp * 0.4 * sign);
            let cen_pos = bezier3(cen_start, cen_c1, cen_c2, cen_end, u);


            let look = (cen_pos - eye_pos).normalize();
            let roll = (std::f32::consts::PI * u).sin() * 0.4 * sign as f32;
            let up = rotate_around_axis(world_up, look, roll).normalize();

            camera.eye = eye_pos; camera.center = cen_pos; camera.up = up;
            if cam_warp_t >= 1.0 { cam_warp_target = None; if cam_follow_after_warp { cam_detached = false; cam_follow_after_warp = false; } }
        } else if !cam_detached {
            camera.eye = follow_eye; camera.center = follow_center; camera.up = vec3(0.0,1.0,0.0);
        } else if let Some(pi) = cam_follow_planet {
            // Follow the currently selected planet
            if pi < planets.len() {
                let tsec = time*0.001; let (_n, r, size, spd) = planets[pi];
                let a = tsec * spd;
                let center_t = if pi==0 { vec3(0.0,0.0,0.0) } else { vec3(a.cos()* r, 0.0, a.sin()* r) };
                let eye_offset = vec3(0.0, size*2.5 + 4.0, size*3.5 + 7.0);
                camera.eye = center_t + eye_offset; camera.center = center_t; camera.up = vec3(0.0,1.0,0.0);
            }
        }


    let fov_deg = if cam_warp_target.is_some() {
        let t = cam_warp_t.min(1.0);
        let u = ease_in_out_cubic(t);
        let bell = (u * (1.0 - u)) * 4.0; // 0..1..0
        let overshoot = (ease_out_back(u) - u).max(0.0);
        base_fov_deg + 28.0 * bell + 6.0 * overshoot
    } else { base_fov_deg };
    let projection = nalgebra_glm::perspective(fov_deg.to_radians(), aspect, near, far);
    let view = camera.view_matrix();

    let tsec = time*0.001;

    draw_orbit_trails(&mut fb, &view, &projection, &viewport, &planets, tsec);

    let star_pos = vec3(0.0,0.0,0.0); let star_scale = planets[0].2; let noises = vec![&star_base, &star_spots, &star_gran];
    let u = Uniforms { model_matrix: create_model_matrix(star_pos, star_scale, rotation), view_matrix: view, projection_matrix: projection, viewport_matrix: viewport, time, noises, camera_pos: camera.eye };
    render(&mut fb, &u, &sphere_vertices, |frag| shaders::fragment_star(frag, &u));


    let tsec = time*0.001;
        for (i, (_name, r, s, spd)) in planets.iter().enumerate().skip(1) {
            let a = tsec * *spd; let pos = vec3(a.cos()* *r, 0.0, a.sin()* *r);
            let model = create_model_matrix(pos, *s, rotation*0.3);
            let noises = if i <= 4 { vec![&rocky_base, &rocky_detail, &rocky_biome, &rocky_clouds] } else { vec![&gas_bands, &gas_detail, &gas_storms] };
            let u = Uniforms { model_matrix: model, view_matrix: view, projection_matrix: projection, viewport_matrix: viewport, time, noises, camera_pos: camera.eye };
       
            let radius_px = screen_radius_px(&view, &projection, &viewport, pos, *s, rotation*0.3).unwrap_or(0.0);
            if radius_px < 2.0 { continue; } 
            if radius_px < 7.0 {
                let base = planet_color(i);
                render(&mut fb, &u, &sphere_vertices, |frag| lambert(base, frag));
            } else {
                render(&mut fb, &u, &sphere_vertices, |frag| {
                    match i {
                        1 => shaders::fragment_mercury(frag, &u),
                        2 => shaders::fragment_venus(frag, &u),
                        3 => shaders::fragment_earth(frag, &u),
                        4 => shaders::fragment_mars(frag, &u),
                        5 => shaders::fragment_jupiter(frag, &u),
                        6 => shaders::fragment_saturn(frag, &u),
                        7 => shaders::fragment_uranus(frag, &u),
                        8 => shaders::fragment_neptune(frag, &u),
                        _ => lambert(planet_color(i), frag),
                    }
                });
            }
    
            clamp_ship_sphere(&mut ship, pos, *s, 0.6);
     
            if i == 3 { let ma = tsec*2.5; let moon_pos = pos + vec3(ma.cos()* (s*1.3), 0.5*(ma*0.7).sin(), ma.sin()* (s*1.3)); let u = Uniforms { model_matrix: create_model_matrix(moon_pos, s*0.35, rotation*0.6), view_matrix: view, projection_matrix: projection, viewport_matrix: viewport, time, noises: vec![&rocky_detail], camera_pos: camera.eye }; render(&mut fb, &u, &sphere_vertices, |frag| shaders::fragment_moon(frag, &u)); }
         
            if i == 6 {
                let segs = if radius_px < 12.0 { 32 } else if radius_px < 40.0 { 64 } else { 128 };
                render_saturn_ring_with_segments(&mut fb, &view, &projection, &viewport, pos, *s, rotation*0.2, segs);
            }
        }

    clamp_ship_sphere(&mut ship, star_pos, star_scale, 1.2);

    // --- Asteroids update/render ---
    let dt = 0.016f32; 
    for a in asteroids.iter_mut() {
        if !a.alive { continue; }
        if !a.exploding {
            let d = (ship.pos - a.pos).magnitude();
            let trigger = (a.scale * 6.0).clamp(1.0, 8.0);
            if d < trigger { a.exploding = true; a.t = 0.0; }
        }
        if a.exploding {
            a.t += dt;
            if let Some((sx,sy)) = project_point(&view, &projection, &viewport, a.pos) {
                let rp = (8.0 + 90.0 * (a.t)).min(120.0);
                let k = 1.0 - (a.t / 1.0).min(1.0);
                sun_glow_layer(&mut fb, sx, sy, rp*1.1, rp*0.3, Color::new(255, 180, 80), 0.28 * k);
                sun_glow_layer(&mut fb, sx, sy, rp*0.7, rp*0.2, Color::new(255, 230, 160), 0.22 * k);
            }
            if a.t >= 1.0 { a.alive = false; }
            continue; 
        }
        a.rot_y += 0.004;
        a.pos += a.vel;

    let dist = (a.pos).magnitude();
    let passed_ship = (a.pos - ship.pos).dot(&fwd) < -120.0; 
    if dist > 500.0 || a.pos.y.abs() > 80.0 || passed_ship { a.alive = false; continue; }
    let model = create_model_matrix(a.pos, a.scale * asteroid_unit_scale, a.rot_y);
    let u = Uniforms { model_matrix: model, view_matrix: view, projection_matrix: projection, viewport_matrix: viewport, time, noises: vec![&rocky_base, &rocky_detail], camera_pos: camera.eye };

        render(&mut fb, &u, &asteroid_vertices, |frag| shaders::fragment_asteroid(frag, &u));

        if let Some((sx,sy)) = project_point(&view, &projection, &viewport, a.pos) {
            if let Some(rad_px) = screen_radius_px(&view, &projection, &viewport, a.pos, a.scale, a.rot_y) {
                let rp = rad_px.max(2.0).min(7.0);
                sun_glow_layer(&mut fb, sx, sy, rp*1.6, rp*0.7, Color::new(200, 230, 255), 0.10);
            }
        }
    }

    // Keep at most two asteroids alive; respawn replacements when they disappear
    asteroids.retain(|a| a.alive || a.exploding);
    let mut alive_count = asteroids.iter().filter(|a| a.alive).count();
    while alive_count < max_asteroids {
        asteroids.push(spawn_asteroid_crossing_ship(&ship, &mut rng));
        alive_count += 1;
    }


    let ship_rot_y = ship.yaw + std::f32::consts::FRAC_PI_2;
    let u = Uniforms { model_matrix: create_model_matrix_euler(ship.pos, 0.25, -ship.pitch, ship_rot_y, ship.roll), view_matrix: view, projection_matrix: projection, viewport_matrix: viewport, time, noises: vec![], camera_pos: camera.eye };
    render(&mut fb, &u, &ship_vertices, |_frag| Color::from_float(0.85,0.85,0.9));

        {
            let star_pos = vec3(0.0,0.0,0.0);
            let star_scale = planets[0].2;
            if let Some(rad_px) = screen_radius_px(&view, &projection, &viewport, star_pos, star_scale, rotation) {
                if let Some((sx,sy)) = project_point(&view, &projection, &viewport, star_pos) {
                    if rad_px > 2.0 {
                        let max_dim = fb.width.max(fb.height) as f32;
                        let huge = rad_px > max_dim * 0.45; 
                        let rp = if huge { max_dim * 0.45 } else { rad_px };
             
                        if huge {
                            sun_glow_layer(&mut fb, sx, sy, rp*1.6, rp*0.7, Color::new(255, 210, 120), 0.22);
                        } else {
                            sun_glow_layer(&mut fb, sx, sy, rp*1.8, rp*0.6, Color::new(255, 210, 120), 0.50);
                            sun_glow_layer(&mut fb, sx, sy, rp*2.6, rp*1.2, Color::new(255, 180, 90), 0.28);
                            sun_glow_layer(&mut fb, sx, sy, rp*3.8, rp*2.4, Color::new(255, 140, 60), 0.12);
                      
                            sun_streak_horizontal(&mut fb, sx, sy, rp*4.0, Color::new(255, 190, 100), 0.06);
                        }
                    }
                }
            }
        }

        window.update_with_buffer(&fb.buffer, w, h)?;

   
    if window.is_key_pressed(Key::O, minifb::KeyRepeat::No) { animate_orbits = !animate_orbits; }
    
        if window.is_key_pressed(Key::S, minifb::KeyRepeat::No) {
            let mut img = ImageBuffer::<Rgb<u8>, Vec<u8>>::new(w as u32, h as u32);
            for y in 0..h { for x in 0..w { let px = fb.buffer[y*w + x]; let r=((px>>16)&0xFF) as u8; let g=((px>>8)&0xFF) as u8; let b=(px&0xFF) as u8; img.put_pixel(x as u32, y as u32, Rgb([r,g,b])); } }
            let _ = img.save("screenshot.png");
        }
    }
    Ok(())
}

fn draw_orbit_trails(fb: &mut Framebuffer, view: &Mat4, proj: &Mat4, vp: &Mat4, planets: &[(&str, f32, f32, f32)], tsec: f32) {
    let star = vec3(0.0,0.0,0.0);
    for (_name, r, _s, spd) in planets.iter().skip(1) { 
        let a_now = tsec * *spd;
        let segments = 72;
        let tail_len = 1.4;
        let step = tail_len / segments as f32;
        let base = Color::new(110,110,110);
        let mut last: Option<(i32,i32)> = None;
        for i in 0..=segments {
            let ang = a_now - i as f32 * step;
            let p = star + vec3(ang.cos()* *r, 0.0, ang.sin()* *r);
            if let Some((x,y)) = project_point(view, proj, vp, p) {
              
                let t = 1.0 - (i as f32 / segments as f32);
                let col = scale_color(base, 0.35 + 0.65 * t);
                fb.set_current_color(col.to_hex());
                if let Some((lx,ly)) = last { fb.draw_line(lx,ly,x,y); }
                last = Some((x,y));
            } else {
                last = None; 
            }
        }
    }
}

fn project_point(view: &Mat4, proj: &Mat4, vp: &Mat4, p: Vec3) -> Option<(i32,i32)> {
    let hp = Vec4::new(p.x,p.y,p.z,1.0);
    let clip = *proj * *view * hp;

    if clip.w <= 1e-6 { return None; }
    let ndc = Vec4::new(clip.x/clip.w, clip.y/clip.w, clip.z/clip.w, 1.0);

    if ndc.z < -1.0 || ndc.z > 1.0 { return None; }
    let screen = *vp * ndc; Some((screen.x as i32, screen.y as i32))
}

fn draw_circle_world(fb: &mut Framebuffer, view: &Mat4, proj: &Mat4, vp: &Mat4, center: Vec3, radius: f32, segments: i32, color: Color) {
    let mut last: Option<(i32,i32)> = None; fb.set_current_color(color.to_hex());
    for i in 0..=segments {
        let t = i as f32 / segments as f32 * std::f32::consts::TAU;
        let p = center + vec3(t.cos()*radius, 0.0, t.sin()*radius);
        if let Some((x,y)) = project_point(view, proj, vp, p) {
            if let Some((lx,ly)) = last { fb.draw_line(lx,ly,x,y); }
            last = Some((x,y));
        } else {
     
            last = None;
        }
    }
}

fn scale_color(c: Color, k: f32) -> Color {
    let k = k.clamp(0.0, 1.0);
    Color::new(
        ((c.r as f32)*k) as u8,
        ((c.g as f32)*k) as u8,
        ((c.b as f32)*k) as u8,
    )
}

// ---------- Screen-space sun glow helpers ----------
fn add_color_to_pixel(fb: &mut Framebuffer, x: i32, y: i32, add: Color) {
    if x < 0 || y < 0 || x as usize >= fb.width || y as usize >= fb.height { return; }
    let idx = y as usize * fb.width + x as usize;
    let px = fb.buffer[idx];
    let r = ((px >> 16) & 0xFF) as u8; let g = ((px >> 8) & 0xFF) as u8; let b = (px & 0xFF) as u8;
    let cur = Color::new(r,g,b);
    let out = cur + add;
    fb.buffer[idx] = out.to_hex();
}

fn sun_glow_layer(fb: &mut Framebuffer, cx: i32, cy: i32, r_outer: f32, r_inner: f32, color: Color, strength: f32) {
    if r_outer <= 0.0 { return; }
    let r0 = r_inner.max(0.0); let r1 = r_outer.max(r0+1.0);
    let mut min_x = (cx as f32 - r1).floor() as i32; let mut max_x = (cx as f32 + r1).ceil() as i32;
    let mut min_y = (cy as f32 - r1).floor() as i32; let mut max_y = (cy as f32 + r1).ceil() as i32;
    
    min_x = min_x.max(0); min_y = min_y.max(0);
    max_x = max_x.min(fb.width as i32 - 1); max_y = max_y.min(fb.height as i32 - 1);
    if min_x > max_x || min_y > max_y { return; }
   
    let area = (max_x - min_x + 1) as i64 * (max_y - min_y + 1) as i64;
    let budget: f32 = 180_000.0; 
    let stride = ((area as f32 / budget).sqrt().ceil() as i32).max(1);
    let mut y = min_y;
    while y <= max_y {
        let mut x = min_x;
        while x <= max_x {
            let dx = x as f32 - cx as f32; let dy = y as f32 - cy as f32;
            let d = (dx*dx + dy*dy).sqrt(); if d <= r1 {
                let t = ((d - r0) / (r1 - r0)).clamp(0.0, 1.0);
                let s = (1.0 - (t*t*(3.0 - 2.0*t))) * strength; 
                if s > 0.001 {
                    add_color_to_pixel(fb, x, y, color * s);
                 
                    if stride > 1 {
                        add_color_to_pixel(fb, x+1, y, color * (s*0.7));
                        add_color_to_pixel(fb, x, y+1, color * (s*0.7));
                    }
                }
            }
            x += stride;
        }
        y += stride;
    }
}

fn sun_streak_horizontal(fb: &mut Framebuffer, cx: i32, cy: i32, half_len: f32, color: Color, strength: f32) {
    let mut y0 = cy - 1; let mut y1 = cy + 1; 
    y0 = y0.max(0); y1 = y1.min(fb.height as i32 - 1);
    let mut min_x = (cx as f32 - half_len).floor() as i32; let mut max_x = (cx as f32 + half_len).ceil() as i32;
    min_x = min_x.max(0); max_x = max_x.min(fb.width as i32 - 1);
    if min_x > max_x || y0 > y1 { return; }
   
    let len = (max_x - min_x + 1).max(1) as i32;
    let budget: f32 = 12_000.0;
    let stride = ((len as f32 / budget).ceil() as i32).max(1);
    for y in y0..=y1 {
        let mut x = min_x;
        while x <= max_x {
            let dx = (x - cx) as f32; let fall = 1.0 / (1.0 + (dx*dx)/(half_len*half_len*0.25));
            let s = strength * fall;
            if s > 0.001 {
                add_color_to_pixel(fb, x, y, color * s);
                if stride > 1 { add_color_to_pixel(fb, x+1, y, color * (s*0.7)); }
            }
            x += stride;
        }
    }
}


fn generate_ring_vertices(segments: usize, inner_r: f32, outer_r: f32, ellipse_z: f32) -> Vec<Vertex> {
    let mut verts = Vec::with_capacity(segments * 6);
    for i in 0..segments {
        let t0 = i as f32 / segments as f32 * std::f32::consts::TAU;
        let t1 = (i as f32 + 1.0) / segments as f32 * std::f32::consts::TAU;
    let (s0, c0) = t0.sin_cos();
    let (s1, c1) = t1.sin_cos();
        let n = vec3(0.0, 1.0, 0.0);
        let o0 = vec3(c0 * outer_r, 0.0, s0 * outer_r * ellipse_z);
        let i0 = vec3(c0 * inner_r, 0.0, s0 * inner_r * ellipse_z);
        let o1 = vec3(c1 * outer_r, 0.0, s1 * outer_r * ellipse_z);
        let i1 = vec3(c1 * inner_r, 0.0, s1 * inner_r * ellipse_z);
        
        verts.push(Vertex::new(o0, n));
        verts.push(Vertex::new(i0, n));
        verts.push(Vertex::new(i1, n));
   
        verts.push(Vertex::new(o0, n));
        verts.push(Vertex::new(i1, n));
        verts.push(Vertex::new(o1, n));
    }
    verts
}

fn render_saturn_ring_with_segments(fb: &mut Framebuffer, view: &Mat4, proj: &Mat4, vp: &Mat4, center: Vec3, planet_scale: f32, rotate_y: f32, segments: usize) {
 
    let inner_r = planet_scale * 1.2;
    let outer_r = planet_scale * 2.0;
    let ellipse_z = 1.2;
    let ring = generate_ring_vertices(segments, inner_r, outer_r, ellipse_z);
    let model = create_model_matrix(center, 1.0, rotate_y);
    let u = Uniforms { model_matrix: model, view_matrix: *view, projection_matrix: *proj, viewport_matrix: *vp, time: 0.0, noises: vec![], camera_pos: vec3(0.0,0.0,0.0) };
    render(fb, &u, &ring, |frag| shaders::fragment_ring(frag, &u));
}

fn render_saturn_ring(fb: &mut Framebuffer, view: &Mat4, proj: &Mat4, vp: &Mat4, center: Vec3, planet_scale: f32, rotate_y: f32) {
    render_saturn_ring_with_segments(fb, view, proj, vp, center, planet_scale, rotate_y, 128);
}

fn screen_radius_px(view: &Mat4, proj: &Mat4, vp: &Mat4, center: Vec3, scale: f32, rotate_y: f32) -> Option<f32> {
    let c = rotate_y.cos(); let s = rotate_y.sin();
    let offset = vec3(scale, 0.0, 0.0);
    let rot_off = vec3(offset.x*c + offset.z*s, offset.y, -offset.x*s + offset.z*c);
    let p0 = project_point(view, proj, vp, center)?;
    let p1 = project_point(view, proj, vp, center + rot_off)?;
    let dx = (p1.0 - p0.0) as f32; let dy = (p1.1 - p0.1) as f32;
    Some((dx*dx + dy*dy).sqrt())
}

// ---------- Creative camera warp helpers ----------
fn ease_in_out_cubic(t: f32) -> f32 {
    if t < 0.5 { 4.0 * t * t * t } else { 1.0 - (-2.0 * t + 2.0).powi(3) / 2.0 }
}

fn ease_out_back(t: f32) -> f32 {
    let c1 = 1.70158f32; let c3 = c1 + 1.0;
    1.0 + c3 * (t - 1.0).powi(3) + c1 * (t - 1.0).powi(2)
}

fn bezier3(p0: Vec3, c1: Vec3, c2: Vec3, p3: Vec3, t: f32) -> Vec3 {
    let it = 1.0 - t;
    let b0 = it*it*it; let b1 = 3.0*it*it*t; let b2 = 3.0*it*t*t; let b3 = t*t*t;
    p0*b0 + c1*b1 + c2*b2 + p3*b3
}

fn rotate_around_axis(v: Vec3, axis: Vec3, angle: f32) -> Vec3 {
    let a = axis.normalize();
    let c = angle.cos(); let s = angle.sin();
    v*c + a.cross(&v)*s + a*(a.dot(&v))*(1.0 - c)
}
