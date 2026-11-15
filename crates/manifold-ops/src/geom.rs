//! Small vector helpers and an orthonormal plane frame shared by the mesh operations.

pub(crate) type P = [f64; 3];

pub(crate) fn sub(a: P, b: P) -> P {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
pub(crate) fn add(a: P, b: P) -> P {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
pub(crate) fn scale(a: P, s: f64) -> P {
    [a[0] * s, a[1] * s, a[2] * s]
}
pub(crate) fn dot(a: P, b: P) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
pub(crate) fn cross(a: P, b: P) -> P {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
pub(crate) fn norm(a: P) -> f64 {
    dot(a, a).sqrt()
}
pub(crate) fn point(positions: &[f64], i: usize) -> P {
    [positions[3 * i], positions[3 * i + 1], positions[3 * i + 2]]
}
pub(crate) fn tri_normal(a: P, b: P, c: P) -> P {
    cross(sub(b, a), sub(c, a))
}

/// Right-handed orthonormal frame `(u, v, normal)` of a plane through `origin`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlaneFrame {
    pub origin: [f64; 3],
    pub u: [f64; 3],
    pub v: [f64; 3],
    pub normal: [f64; 3],
}

impl PlaneFrame {
    /// `None` when `origin`/`normal` are not finite or `normal` is zero.
    pub fn new(origin: [f64; 3], normal: [f64; 3]) -> Option<Self> {
        let len = norm(normal);
        if !len.is_finite() || len == 0.0 || origin.iter().any(|v| !v.is_finite()) {
            return None;
        }
        let n = scale(normal, 1.0 / len);
        let axis = if n[0].abs() <= n[1].abs() && n[0].abs() <= n[2].abs() {
            [1.0, 0.0, 0.0]
        } else if n[1].abs() <= n[2].abs() {
            [0.0, 1.0, 0.0]
        } else {
            [0.0, 0.0, 1.0]
        };
        let u = cross(n, axis);
        let u = scale(u, 1.0 / norm(u));
        let v = cross(n, u);
        Some(Self {
            origin,
            u,
            v,
            normal: n,
        })
    }
    /// Local coordinates `(x, y, height)`.
    pub fn to_local(&self, p: [f64; 3]) -> [f64; 3] {
        let d = sub(p, self.origin);
        [dot(d, self.u), dot(d, self.v), dot(d, self.normal)]
    }
    pub fn to_world(&self, l: [f64; 3]) -> [f64; 3] {
        add(
            add(add(self.origin, scale(self.u, l[0])), scale(self.v, l[1])),
            scale(self.normal, l[2]),
        )
    }
}

#[cfg(test)]
pub(crate) mod fixtures {
    /// Axis-aligned box, outward CCW winding.
    pub fn cuboid(min: [f64; 3], max: [f64; 3]) -> (Vec<f64>, Vec<usize>) {
        let ([x0, y0, z0], [x1, y1, z1]) = (min, max);
        #[rustfmt::skip]
        let positions = vec![
            x0, y0, z0, x1, y0, z0, x1, y1, z0, x0, y1, z0,
            x0, y0, z1, x1, y0, z1, x1, y1, z1, x0, y1, z1,
        ];
        let indices = vec![
            0, 2, 1, 0, 3, 2, 4, 5, 6, 4, 6, 7, 0, 1, 5, 0, 5, 4, 2, 3, 7, 2, 7, 6, 1, 2, 6, 1, 6,
            5, 3, 0, 4, 3, 4, 7,
        ];
        (positions, indices)
    }

    /// Welded UV sphere (closed manifold), outward winding.
    pub fn sphere(r: f64, rings: usize, sectors: usize) -> (Vec<f64>, Vec<usize>) {
        let mut p = vec![0.0, 0.0, r];
        for i in 1..rings {
            let th = std::f64::consts::PI * i as f64 / rings as f64;
            for j in 0..sectors {
                let ph = 2.0 * std::f64::consts::PI * j as f64 / sectors as f64;
                p.extend([
                    r * th.sin() * ph.cos(),
                    r * th.sin() * ph.sin(),
                    r * th.cos(),
                ]);
            }
        }
        p.extend([0.0, 0.0, -r]);
        let south = p.len() / 3 - 1;
        let ring = |i: usize, j: usize| 1 + (i - 1) * sectors + j % sectors;
        let mut t = Vec::new();
        for j in 0..sectors {
            t.extend([0, ring(1, j), ring(1, j + 1)]);
            t.extend([south, ring(rings - 1, j + 1), ring(rings - 1, j)]);
        }
        for i in 1..rings - 1 {
            for j in 0..sectors {
                let (a, b, c, d) = (
                    ring(i, j),
                    ring(i, j + 1),
                    ring(i + 1, j),
                    ring(i + 1, j + 1),
                );
                t.extend([a, c, d, a, d, b]);
            }
        }
        (p, t)
    }

    pub fn volume(p: &[f64], t: &[usize]) -> f64 {
        polygon_core::Mesh {
            positions: p.to_vec(),
            indices: t.to_vec(),
            uv: None,
        }
        .volume()
    }
    pub fn area(p: &[f64], t: &[usize]) -> f64 {
        polygon_core::Mesh {
            positions: p.to_vec(),
            indices: t.to_vec(),
            uv: None,
        }
        .surface_area()
    }

    /// Deterministic xorshift generator in `[0, 1)`.
    pub struct Rng(pub u64);
    impl Rng {
        pub fn next(&mut self) -> f64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            (self.0 >> 11) as f64 / (1u64 << 53) as f64
        }
    }
}
