pub struct Framebuffer {
    pub width: usize,
    pub height: usize,
    pub buffer: Vec<u32>,
    pub zbuffer: Vec<f32>,
    current_color: u32,
}

impl Framebuffer {
    pub fn new(width: usize, height: usize) -> Self {
        Self { width, height, buffer: vec![0; width*height], zbuffer: vec![f32::INFINITY; width*height], current_color: 0x000000 }
    }
    pub fn clear(&mut self, color: u32) {
        self.buffer.fill(color); self.zbuffer.fill(f32::INFINITY);
    }
    pub fn set_current_color(&mut self, color: u32) { self.current_color = color; }
    #[inline]
    pub fn point(&mut self, x: i32, y: i32, depth: f32) {
        if x < 0 || y < 0 || x as usize >= self.width || y as usize >= self.height { return; }
        let idx = y as usize * self.width + x as usize;
        if depth < self.zbuffer[idx] { self.zbuffer[idx] = depth; self.buffer[idx] = self.current_color; }
    }
    #[inline]
    pub fn point_no_depth(&mut self, x: i32, y: i32) {
        if x < 0 || y < 0 || x as usize >= self.width || y as usize >= self.height { return; }
        let idx = y as usize * self.width + x as usize; self.buffer[idx] = self.current_color;
    }
    pub fn draw_line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32) {
        let mut x0 = x0; let mut y0 = y0; let dx = (x1 - x0).abs(); let sx = if x0 < x1 {1} else {-1}; let dy = -(y1 - y0).abs(); let sy = if y0 < y1 {1} else {-1}; let mut err = dx + dy; loop { self.point_no_depth(x0, y0); if x0 == x1 && y0 == y1 { break; } let e2 = 2*err; if e2 >= dy { err += dy; x0 += sx; } if e2 <= dx { err += dx; y0 += sy; } }
    }
}
