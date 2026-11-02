//! CPU implicit field validation, sampling, and triangle mesh distance queries.

use geometry_ops::Triangles;
use math_core::{cross, dot, finite, norm as length, sub};
use planar_geometry::rings;
use crate::{Field, Point, Result, error};

fn smooth_min(a: f64, b: f64, k: f64) -> f64 {
    let h = (0.5 + 0.5 * (b - a) / k).clamp(0., 1.);
    b * (1. - h) + a * h - k * h * (1. - h)
}
pub(crate) fn positive(x: f64) -> bool {
    x.is_finite() && x > 0. && x <= 1e6
}
/// Triangle corner positions as `[x, y, z]` rows: one bounds check per corner
/// instead of three, and the chunk view is hoisted out of the sampling loops.
#[inline(always)]
fn corners(mesh: &Triangles) -> &[[f64; 3]] {
    mesh.positions.as_chunks::<3>().0
}
#[inline(always)]
pub(crate) fn point(mesh: &Triangles, i: usize) -> Point {
    corners(mesh)[i]
}
pub(crate) fn triangles_ok(mesh: &Triangles) -> bool {
    let n = mesh.indices.len() / 3;
    n > 0
        && n <= 4096
        && mesh.indices.len().is_multiple_of(3)
        && mesh.positions.len().is_multiple_of(3)
        && mesh.indices.iter().all(|&i| {
            i.checked_mul(3)
                .is_some_and(|o| o + 2 < mesh.positions.len())
        })
        && mesh
            .positions
            .iter()
            .all(|x| x.is_finite() && x.abs() <= 1e6)
        && mesh.indices.as_chunks::<3>().0.iter().all(|t| {
            length(cross(
                sub(point(mesh, t[1]), point(mesh, t[0])),
                sub(point(mesh, t[2]), point(mesh, t[0])),
            )) > 0.
        })
}
pub(crate) fn triangles_closed(mesh: &Triangles) -> bool {
    let mut edges = rustc_hash::FxHashMap::with_capacity_and_hasher(
        mesh.indices.len() / 2,
        Default::default(),
    );
    for t in mesh.indices.as_chunks::<3>().0 {
        for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            *edges.entry((a.min(b), a.max(b))).or_insert(0) += 1;
        }
    }
    !edges.is_empty() && edges.values().all(|&n| n == 2)
}
pub(crate) fn finish_triangles(mesh: Triangles) -> Result<Triangles> {
    if !mesh.indices.len().is_multiple_of(3)
        || mesh.indices.iter().any(|&i| {
            i.checked_mul(3)
                .is_none_or(|o| o + 2 >= mesh.positions.len())
        })
    {
        return Err(error("Implicit mesh has invalid indices"));
    }
    Ok(mesh)
}
/// Closest point on segment `ab` to `p`; `d` is the precomputed `b - a`.
#[inline(always)]
fn closest_on_segment(p: Point, a: Point, d: Point) -> Point {
    let dd = dot(d, d);
    let t = if dd > 0. {
        (dot(sub(p, a), d) / dd).clamp(0., 1.)
    } else {
        0.
    };
    [a[0] + t * d[0], a[1] + t * d[1], a[2] + t * d[2]]
}
#[inline]
fn closest_triangle(p: Point, a: Point, b: Point, c: Point) -> Point {
    let ab = sub(b, a);
    let ac = sub(c, a);
    let n = cross(ab, ac);
    let nn = dot(n, n);
    if nn > 0. {
        let s = dot(sub(p, a), n);
        let q = [
            p[0] - n[0] * s / nn,
            p[1] - n[1] * s / nn,
            p[2] - n[2] * s / nn,
        ];
        let aq = sub(q, a);
        let v = dot(cross(aq, ac), n) / nn;
        let w = dot(cross(ab, aq), n) / nn;
        if v >= 0. && w >= 0. && v + w <= 1. {
            return q;
        }
    }
    // Unrolled a→b, b→c, c→a scan with the original strict-`<` tie-breaking.
    let mut best = a;
    let mut distance = f64::INFINITY;
    for (a, d) in [(a, ab), (b, sub(c, b)), (c, sub(a, c))] {
        let q = closest_on_segment(p, a, d);
        let dist = length(sub(p, q));
        if dist < distance {
            best = q;
            distance = dist;
        }
    }
    best
}
#[inline]
fn closest_point(mesh: &Triangles, p: Point) -> (Point, f64) {
    let corners = corners(mesh);
    let mut best = p;
    let mut distance = f64::INFINITY;
    for &[i, j, k] in mesh.indices.as_chunks::<3>().0 {
        let q = closest_triangle(p, corners[i], corners[j], corners[k]);
        let d = length(sub(p, q));
        if d < distance {
            best = q;
            distance = d;
        }
    }
    (best, distance)
}
#[inline]
fn signed_distance(mesh: &Triangles, p: Point) -> f64 {
    let (_, d) = closest_point(mesh, p);
    if d == 0. {
        return 0.;
    }
    let corners = corners(mesh);
    let mut angle = 0.;
    for &[i, j, k] in mesh.indices.as_chunks::<3>().0 {
        let a = sub(corners[i], p);
        let b = sub(corners[j], p);
        let c = sub(corners[k], p);
        let la = length(a);
        let lb = length(b);
        let lc = length(c);
        angle += 2.
            * dot(a, cross(b, c))
                .atan2(la * lb * lc + dot(a, b) * lc + dot(b, c) * la + dot(c, a) * lb);
    }
    if angle.abs() > 2. * std::f64::consts::PI {
        -d
    } else {
        d
    }
}
impl Field {
    pub fn validate(&self) -> Result<()> {
        fn walk(f: &Field, depth: usize, budget: &mut usize) -> bool {
            if depth > 32 || *budget >= 256 {
                return false;
            }
            *budget += 1;
            match f {
                Field::Extrude {
                    profile,
                    half_height,
                } => rings::validate_profile(profile).is_ok() && positive(*half_height),
                Field::Revolve { profile } => {
                    rings::validate_profile(profile).is_ok()
                        && profile.iter().flatten().all(|p| p[0] >= 0.)
                }
                Field::Deform { input, deformation } => {
                    matches!(deformation, geometry_ops::Deformation::Twist { .. })
                        && deformation.validate().is_ok()
                        && walk(input, depth + 1, budget)
                }
                Field::MeshDistance { mesh, signed } => {
                    triangles_ok(mesh) && (!*signed || triangles_closed(mesh))
                }
                Field::Sphere { center, radius } => finite(*center) && positive(*radius),
                Field::Box { center, half_size } => {
                    finite(*center) && half_size.iter().all(|&x| positive(x))
                }
                Field::Torus {
                    center,
                    major_radius,
                    minor_radius,
                } => {
                    finite(*center)
                        && positive(*major_radius)
                        && positive(*minor_radius)
                        && minor_radius < major_radius
                }
                Field::Cylinder {
                    center,
                    radius,
                    half_height,
                } => finite(*center) && positive(*radius) && positive(*half_height),
                Field::Gyroid {
                    origin,
                    period,
                    thickness,
                }
                | Field::SchwarzP {
                    origin,
                    period,
                    thickness,
                }
                | Field::Diamond {
                    origin,
                    period,
                    thickness,
                }
                | Field::Neovius {
                    origin,
                    period,
                    thickness,
                }
                | Field::Lidinoid {
                    origin,
                    period,
                    thickness,
                }
                | Field::SchoenIWP {
                    origin,
                    period,
                    thickness,
                } => {
                    finite(*origin)
                        && positive(*period)
                        && positive(*thickness)
                        && thickness < period
                }
                Field::Union { a, b }
                | Field::Intersection { a, b }
                | Field::Difference { a, b } => {
                    walk(a, depth + 1, budget) && walk(b, depth + 1, budget)
                }
                Field::Capsule { a, b, radius } => finite(*a) && finite(*b) && positive(*radius),
                Field::Octahedron { center, size } => finite(*center) && positive(*size),
                Field::HexPrism {
                    center,
                    radius,
                    half_height,
                }
                | Field::TriangularPrism {
                    center,
                    radius,
                    half_height,
                } => finite(*center) && positive(*radius) && positive(*half_height),
                Field::SmoothUnion { a, b, radius }
                | Field::SmoothIntersection { a, b, radius }
                | Field::SmoothDifference { a, b, radius }
                | Field::ChamferUnion { a, b, radius }
                | Field::ChamferIntersection { a, b, radius }
                | Field::ChamferDifference { a, b, radius } => {
                    positive(*radius) && walk(a, depth + 1, budget) && walk(b, depth + 1, budget)
                }
                Field::Offset { input, distance } => {
                    distance.is_finite() && distance.abs() <= 1e6 && walk(input, depth + 1, budget)
                }
                Field::Onion { input, thickness } => {
                    positive(*thickness) && walk(input, depth + 1, budget)
                }
                Field::Elongate {
                    input,
                    half_extents,
                } => {
                    finite(*half_extents)
                        && half_extents.iter().all(|&x| (0.0..=1e6).contains(&x))
                        && walk(input, depth + 1, budget)
                }
                Field::Mirror { input, .. } => walk(input, depth + 1, budget),
                Field::Taper {
                    input,
                    height,
                    factor,
                } => {
                    positive(*height)
                        && factor.is_finite()
                        && (0.05..=20.0).contains(factor)
                        && walk(input, depth + 1, budget)
                }
                Field::Translate { input, vector } => {
                    finite(*vector) && walk(input, depth + 1, budget)
                }
            }
        }
        if walk(self, 0, &mut 0) && self.sample_work() <= 4096 {
            Ok(())
        } else {
            Err(error("Invalid field or depth/node budget exceeded"))
        }
    }
    pub(crate) fn sample(&self, p: Point) -> f64 {
        match self {
            Self::Extrude {
                profile,
                half_height,
            } => {
                let a = rings::signed_distance(profile, [p[0], p[1]]);
                let b = p[2].abs() - half_height;
                a.max(0.).hypot(b.max(0.)) + a.max(b).min(0.)
            }
            Self::Revolve { profile } => rings::signed_distance(profile, [p[0].hypot(p[1]), p[2]]),
            Self::Deform { input, deformation } => match deformation.inverse(p) {
                Ok(q) => input.sample(q),
                Err(_) => f64::NAN,
            },
            Self::MeshDistance { mesh, signed } => {
                if *signed {
                    signed_distance(mesh, p)
                } else {
                    closest_point(mesh, p).1
                }
            }
            Self::Sphere { center, radius } => length(sub(p, *center)) - radius,
            Self::Box { center, half_size } => {
                let q: Point = [
                    (p[0] - center[0]).abs() - half_size[0],
                    (p[1] - center[1]).abs() - half_size[1],
                    (p[2] - center[2]).abs() - half_size[2],
                ];
                length([q[0].max(0.), q[1].max(0.), q[2].max(0.)])
                    + q[0].max(q[1]).max(q[2]).min(0.)
            }
            Self::Torus {
                center,
                major_radius,
                minor_radius,
            } => {
                let q = sub(p, *center);
                (q[0].hypot(q[1]) - major_radius).hypot(q[2]) - minor_radius
            }
            Self::Cylinder {
                center,
                radius,
                half_height,
            } => {
                let q = sub(p, *center);
                let dx = q[0].hypot(q[1]) - radius;
                let dy = q[2].abs() - half_height;
                dx.max(dy).min(0.) + dx.max(0.).hypot(dy.max(0.))
            }
            Self::Gyroid {
                origin,
                period,
                thickness,
            }
            | Self::SchwarzP {
                origin,
                period,
                thickness,
            }
            | Self::Diamond {
                origin,
                period,
                thickness,
            }
            | Self::Neovius {
                origin,
                period,
                thickness,
            }
            | Self::Lidinoid {
                origin,
                period,
                thickness,
            }
            | Self::SchoenIWP {
                origin,
                period,
                thickness,
            } => {
                let k = std::f64::consts::TAU / period;
                let q = [
                    (p[0] - origin[0]) * k,
                    (p[1] - origin[1]) * k,
                    (p[2] - origin[2]) * k,
                ];
                let (sx, cx) = q[0].sin_cos();
                let (sy, cy) = q[1].sin_cos();
                let (sz, cz) = q[2].sin_cos();
                let iso = match self {
                    Self::Gyroid { .. } => sx * cy + sy * cz + sz * cx,
                    Self::SchwarzP { .. } => cx + cy + cz,
                    Self::Diamond { .. } => {
                        sx * sy * sz + sx * cy * cz + cx * sy * cz + cx * cy * sz
                    }
                    Self::Neovius { .. } => 3.0 * (cx + cy + cz) + 4.0 * cx * cy * cz,
                    Self::Lidinoid { .. } => {
                        let s2x = 2.0 * sx * cx;
                        let s2y = 2.0 * sy * cy;
                        let s2z = 2.0 * sz * cz;
                        let c2x = cx * cx - sx * sx;
                        let c2y = cy * cy - sy * sy;
                        let c2z = cz * cz - sz * sz;
                        0.5 * (s2x * cy * sz + s2y * cz * sx + s2z * cx * sy)
                            - 0.5 * (c2x * c2y + c2y * c2z + c2z * c2x)
                            + 0.15
                    }
                    _ => {
                        let c2x = cx * cx - sx * sx;
                        let c2y = cy * cy - sy * sy;
                        let c2z = cz * cz - sz * sz;
                        2.0 * (cx * cy + cy * cz + cz * cx) - (c2x + c2y + c2z)
                    }
                };
                iso.abs() / k - 0.5 * thickness
            }
            Self::Capsule { a, b, radius } => {
                let ab = sub(*b, *a);
                let ap = sub(p, *a);
                let denom = dot(ab, ab);
                let t = if denom > 0. {
                    (dot(ap, ab) / denom).clamp(0., 1.)
                } else {
                    0.
                };
                length(sub(ap, math_core::scale(ab, t))) - radius
            }
            Self::Octahedron { center, size } => {
                let p = [
                    (p[0] - center[0]).abs(),
                    (p[1] - center[1]).abs(),
                    (p[2] - center[2]).abs(),
                ];
                let m = p[0] + p[1] + p[2] - size;
                let q = if 3.0 * p[0] < m {
                    p
                } else if 3.0 * p[1] < m {
                    [p[1], p[2], p[0]]
                } else if 3.0 * p[2] < m {
                    [p[2], p[0], p[1]]
                } else {
                    return m * 0.5773502691896258;
                };
                let k = (0.5 * (q[2] - q[1] + size)).clamp(0.0, *size);
                length([q[0], q[1] - size + k, q[2] - k])
            }
            Self::HexPrism {
                center,
                radius,
                half_height,
            } => {
                let q = [
                    (p[0] - center[0]).abs(),
                    (p[1] - center[1]).abs(),
                    (p[2] - center[2]).abs(),
                ];
                const KX: f64 = -0.8660254037844386;
                const KY: f64 = 0.5;
                const KZ: f64 = 0.5773502691896258;
                let dot_k = (KX * q[0] + KY * q[1]).min(0.0);
                let qx = q[0] - 2.0 * dot_k * KX;
                let qy = q[1] - 2.0 * dot_k * KY;
                let d1 = (qx - qx.clamp(-KZ * radius, KZ * radius)).hypot(qy - radius)
                    * (qy - radius).signum();
                let d2 = q[2] - half_height;
                d1.max(d2).min(0.0) + d1.max(0.0).hypot(d2.max(0.0))
            }
            Self::TriangularPrism {
                center,
                radius,
                half_height,
            } => {
                let q = sub(p, *center);
                let qx = q[0].abs();
                let d1 = (qx * 0.8660254037844386 + q[1] * 0.5).max(-q[1]) - radius * 0.5;
                let d2 = q[2].abs() - half_height;
                d1.max(d2).min(0.0) + d1.max(0.0).hypot(d2.max(0.0))
            }
            Self::Union { a, b } => a.sample(p).min(b.sample(p)),
            Self::Intersection { a, b } => a.sample(p).max(b.sample(p)),
            Self::Difference { a, b } => a.sample(p).max(-b.sample(p)),
            Self::SmoothUnion { a, b, radius: k } => smooth_min(a.sample(p), b.sample(p), *k),
            Self::SmoothIntersection { a, b, radius: k } => {
                -smooth_min(-a.sample(p), -b.sample(p), *k)
            }
            // Smooth max(a, -b): the polynomial smooth-union mirrored through negation.
            Self::SmoothDifference { a, b, radius: k } => {
                -smooth_min(-a.sample(p), b.sample(p), *k)
            }
            Self::ChamferUnion { a, b, radius: r } => {
                let da = a.sample(p);
                let db = b.sample(p);
                da.min(db).min((da - r + db) * std::f64::consts::FRAC_1_SQRT_2)
            }
            Self::ChamferIntersection { a, b, radius: r } => {
                let da = a.sample(p);
                let db = b.sample(p);
                da.max(db).max((da + r + db) * std::f64::consts::FRAC_1_SQRT_2)
            }
            Self::ChamferDifference { a, b, radius: r } => {
                let da = a.sample(p);
                let db = -b.sample(p);
                da.max(db).max((da + r + db) * std::f64::consts::FRAC_1_SQRT_2)
            }
            Self::Offset { input, distance } => input.sample(p) - distance,
            Self::Onion { input, thickness } => input.sample(p).abs() - 0.5 * thickness,
            Self::Elongate {
                input,
                half_extents: h,
            } => {
                let q = [
                    p[0] - p[0].clamp(-h[0], h[0]),
                    p[1] - p[1].clamp(-h[1], h[1]),
                    p[2] - p[2].clamp(-h[2], h[2]),
                ];
                input.sample(q)
            }
            Self::Mirror { input, axis } => {
                let q = [
                    if axis[0] { p[0].abs() } else { p[0] },
                    if axis[1] { p[1].abs() } else { p[1] },
                    if axis[2] { p[2].abs() } else { p[2] },
                ];
                input.sample(q)
            }
            Self::Taper {
                input,
                height,
                factor,
            } => {
                let u = ((p[2] / height) + 0.5).clamp(0.0, 1.0);
                let s = 1.0 + u * (factor - 1.0);
                let inv_s = 1.0 / s.max(1e-6);
                let q = [p[0] * inv_s, p[1] * inv_s, p[2]];
                input.sample(q) * s.min(1.0)
            }
            Self::Translate { input, vector } => input.sample(sub(p, *vector)),
        }
    }
    pub fn from_triangles(mesh: Triangles, signed: bool) -> Result<Self> {
        let field = Self::MeshDistance { mesh, signed };
        field.validate()?;
        Ok(field)
    }
    pub(crate) fn sample_work(&self) -> usize {
        match self {
            Self::Extrude { profile, .. } | Self::Revolve { profile } => rings::work(profile),
            Self::MeshDistance { mesh, .. } => mesh.indices.len() / 3,
            Self::Union { a, b }
            | Self::Intersection { a, b }
            | Self::Difference { a, b }
            | Self::SmoothUnion { a, b, .. }
            | Self::SmoothIntersection { a, b, .. }
            | Self::SmoothDifference { a, b, .. }
            | Self::ChamferUnion { a, b, .. }
            | Self::ChamferIntersection { a, b, .. }
            | Self::ChamferDifference { a, b, .. } => a.sample_work() + b.sample_work(),
            Self::Deform { input, .. }
            | Self::Offset { input, .. }
            | Self::Onion { input, .. }
            | Self::Elongate { input, .. }
            | Self::Mirror { input, .. }
            | Self::Taper { input, .. }
            | Self::Translate { input, .. } => input.sample_work(),
            _ => 1,
        }
    }
    pub fn evaluate(&self, p: Point) -> Result<f64> {
        self.validate()?;
        if !finite(p) {
            return Err(error("Invalid sample point"));
        }
        let value = self.sample(p);
        if value.is_finite() {
            Ok(value)
        } else {
            Err(error("Field evaluation left its numeric domain"))
        }
    }
}
