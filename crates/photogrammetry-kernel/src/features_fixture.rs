//! Synthetic image fixture shared by unit tests and the external-reference benchmark.
use crate::Image;
struct Random(u64);
impl Random {
    fn next(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 % 1000000) as f64 / 1000000.
    }
}
pub fn texture(seed: u64, angle: f64, zoom: f64, dx: f64, dy: f64) -> Image {
    let (w, h) = (320, 256);
    let mut rng = Random(seed);
    let lattice = (0..100 * 100).map(|_| rng.next()).collect::<Vec<_>>();
    let noise = |x: f64, y: f64| {
        let (x, y) = ((x / 3. + 30.).clamp(0., 98.), (y / 3. + 30.).clamp(0., 98.));
        let (ix, iy) = (x.floor() as usize, y.floor() as usize);
        let (fx, fy) = (x - ix as f64, y - iy as f64);
        (1. - fy) * ((1. - fx) * lattice[iy * 100 + ix] + fx * lattice[iy * 100 + ix + 1])
            + fy * ((1. - fx) * lattice[(iy + 1) * 100 + ix]
                + fx * lattice[(iy + 1) * 100 + ix + 1])
    };
    let (c, s) = (angle.cos(), angle.sin());
    let mut rgb = Vec::new();
    for y in 0..h {
        for x in 0..w {
            let (u, v) = ((x as f64 - 160. - dx) / zoom, (y as f64 - 128. - dy) / zoom);
            let (xx, yy) = (c * u + s * v, -s * u + c * v);
            let value = (noise(xx, yy) * 230. + 12.).round() as u8;
            rgb.extend([value; 3]);
        }
    }
    Image {
        width: w,
        height: h,
        rgb,
        focal: 300.,
    }
}
