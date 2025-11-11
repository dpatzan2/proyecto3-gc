use nalgebra_glm::{normalize, vec3, Mat4, Vec3, look_at};

pub struct FreeOrbitCamera {
    pub eye: Vec3,
    pub center: Vec3,
    pub up: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub radius: f32,
}

impl FreeOrbitCamera {
    pub fn new(eye: Vec3, center: Vec3) -> Self {
        let forward = normalize(&(center - eye));
        let yaw = forward.z.atan2(forward.x);
        let pitch = (forward.y).asin();
        let up = vec3(0.0, 1.0, 0.0);
        let radius = (eye - center).magnitude();
        Self { eye, center, up, yaw, pitch, radius }
    }
    pub fn view_matrix(&self) -> Mat4 { look_at(&self.eye, &self.center, &self.up) }
    pub fn orbit(&mut self, dyaw: f32, dpitch: f32) {
        self.yaw = (self.yaw + dyaw) % (std::f32::consts::TAU);
        self.pitch = (self.pitch + dpitch).clamp(-1.55, 1.55);
        let cp = self.pitch.cos();
        let dir = vec3(self.yaw.cos() * cp, self.pitch.sin(), self.yaw.sin() * cp);
        self.eye = self.center - dir * self.radius;
    }
    pub fn dolly(&mut self, dr: f32) { self.radius = (self.radius + dr).clamp(1.0, 500.0); self.orbit(0.0, 0.0); }
    pub fn move_local(&mut self, forward: f32, right: f32, up: f32) {
        let forward_vec = normalize(&(self.center - self.eye));
        let right_vec = normalize(&forward_vec.cross(&self.up));
        self.eye += forward_vec * forward + right_vec * right + self.up * up;
        self.center += forward_vec * forward + right_vec * right + self.up * up;
    }
}
