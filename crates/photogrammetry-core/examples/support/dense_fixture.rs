//! Analytic images and true visibility; scene scale is arbitrary, not millimeters.
use super::{Camera, Image, Point, Reconstruction, V3, add, dot, mv, norm, scale, sub, tr};
pub const SIDE: usize = 80;
pub const FOCAL: f64 = 110.;
fn texture(p: V3, object: usize) -> u8 {
    let v = 128.
        + 44. * (p[0] * 19. + (p[1] * 7.).sin() + usize::from(object > 0) as f64 * 1.7).sin()
        + 35. * (p[1] * 23. + (p[0] * 11.).cos()).cos()
        + 17. * (p[0] * 37. + p[1] * 29.).sin();
    v.clamp(0., 255.) as u8
}
#[derive(Clone, Copy)]
pub struct Scene {
    pub angle: f64,
    pub thin: bool,
}
impl Scene {
    pub fn hit(self, camera: &Camera, uv: [f64; 2]) -> (V3, usize) {
        let center = camera.center();
        let ray = mv(tr(camera.rotation), camera.ray(uv));
        let n = [
            self.angle.to_radians().sin(),
            0.,
            self.angle.to_radians().cos(),
        ];
        let t = (4. * n[2] - dot(n, center)) / dot(n, ray);
        let mut p = add(center, scale(ray, t));
        let mut object = 0;
        if self.thin {
            let z = 3.;
            let q = add(center, scale(ray, (z - center[2]) / ray[2]));
            // A thin vertical ribbon plus a larger occluder, separated from the
            // rear plane. Analytic visibility is used to render every camera.
            if ((q[0].abs() < 0.07 && q[1].abs() < 0.9)
                || ((q[0] - 0.55).abs() < 0.22 && (q[1] + 0.3).abs() < 0.35))
                && q[2] < p[2]
            {
                p = q;
                object = if q[0].abs() < 0.07 { 1 } else { 2 };
            }
        }
        (p, object)
    }
    #[allow(dead_code)]
    pub fn distance(self, p: V3) -> f64 {
        let n = [
            self.angle.to_radians().sin(),
            0.,
            self.angle.to_radians().cos(),
        ];
        let back = (dot(n, p) - 4. * n[2]).abs();
        if !self.thin {
            return back;
        }
        let rectangle = |cx: f64, cy: f64, hx: f64, hy: f64| {
            let dx = ((p[0] - cx).abs() - hx).max(0.);
            let dy = ((p[1] - cy).abs() - hy).max(0.);
            (dx * dx + dy * dy + (p[2] - 3.).powi(2)).sqrt()
        };
        back.min(rectangle(0., 0., 0.07, 0.9))
            .min(rectangle(0.55, -0.3, 0.22, 0.35))
    }
}
pub fn fixture(scene: Scene) -> (Vec<Image>, Reconstruction, Vec<(V3, usize)>) {
    let cameras: Vec<_> = [
        [-0.35, 0., 0.],
        [0., 0., 0.],
        [0.35, 0., 0.],
        [0., 0.25, 0.],
    ]
    .into_iter()
    .map(|center| {
        let mut c = Camera::identity(FOCAL, SIDE as f64 / 2., SIDE as f64 / 2.);
        c.translation = scale(center, -1.);
        Some(c)
    })
    .collect();
    let images = cameras
        .iter()
        .map(|c| {
            let c = c.as_ref().unwrap();
            let mut rgb = Vec::new();
            for y in 0..SIDE {
                for x in 0..SIDE {
                    let (p, obj) = scene.hit(c, [x as f64, y as f64]);
                    let v = texture(p, obj);
                    rgb.extend([v, v, v]);
                }
            }
            Image {
                width: SIDE,
                height: SIDE,
                rgb,
                focal: FOCAL,
            }
        })
        .collect();
    let mut points = Vec::new();
    for y in (8..SIDE - 8).step_by(6) {
        for x in (8..SIDE - 8).step_by(6) {
            let (position, _) = scene.hit(cameras[1].as_ref().unwrap(), [x as f64, y as f64]);
            let observations = cameras
                .iter()
                .enumerate()
                .filter_map(|(i, c)| {
                    let c = c.as_ref().unwrap();
                    let uv = c.project(position)?;
                    if uv[0] < 3.
                        || uv[1] < 3.
                        || uv[0] > SIDE as f64 - 4.
                        || uv[1] > SIDE as f64 - 4.
                    {
                        return None;
                    }
                    let (visible, _) = scene.hit(c, uv);
                    (norm(sub(visible, position)) < 1e-6).then_some((i, points.len()))
                })
                .collect();
            points.push(Point {
                position,
                color: [128; 3],
                observations,
            });
        }
    }
    let mut ground = Vec::new();
    // All views, equal sampling density; all target pixels remain in the
    // completeness denominator even if reconstruction rejects them.
    for c in cameras.iter().flatten() {
        for y in (5..SIDE - 5).step_by(2) {
            for x in (5..SIDE - 5).step_by(2) {
                ground.push(scene.hit(c, [x as f64, y as f64]));
            }
        }
    }
    (
        images,
        Reconstruction {
            cameras,
            points,
            input_images: 4,
            reprojection_rmse: 0.,
        },
        ground,
    )
}
