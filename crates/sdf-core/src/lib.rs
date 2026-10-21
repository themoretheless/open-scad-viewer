#![doc = include_str!("../README.md")]
//! Negative-inside implicit fields and bounded marching-tetrahedra extraction.
//! CSG fields preserve the zero set but are not generally exact signed distances.
//! Extraction yields a neutral triangle buffer; mesh inspect lives in the bridge.
use geometry_ops::Triangles;
#[cfg(feature = "codec")]
mod serialization;
pub use math_core::{Acceleration, Error, Result, V3, finite};
use planar_geometry::rings::{self, Rings};
const INVALID_INPUT: &str = "SDF_INVALID_INPUT";
fn error(message: impl Into<String>) -> Error {
    Error::new(INVALID_INPUT, message)
}
#[cfg(feature = "cuda")]
pub mod cuda;
pub mod flat;
#[cfg(feature = "gpu")]
mod gpu;

#[cfg(feature = "gpu")]
pub fn gpu_backend_label() -> Option<&'static str> {
    gpu_backend_report().map(|report| report.label)
}

#[cfg(feature = "gpu")]
pub fn gpu_backend_report() -> Option<gpu_compute::BackendReport> {
    gpu::backend_report()
}

/// The grid-sampling compute shader (WGSL), shared by the native `gpu` feature
/// and the browser WebGPU host path; both must execute the identical text.
pub const SDF_WGSL: &str = r##"
struct Params {
    nx: u32,
    ny: u32,
    nz: u32,
    n_nodes: u32,
    min_x: f32,
    min_y: f32,
    min_z: f32,
    step_x: f32,
    step_y: f32,
    step_z: f32,
}
@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read> kinds: array<u32>;
@group(0) @binding(2) var<storage, read> node_params: array<f32>;
@group(0) @binding(3) var<storage, read> aux: array<u32>;
@group(0) @binding(4) var<storage, read> tris: array<f32>;
@group(0) @binding(5) var<storage, read_write> values: array<f32>;

fn closest_triangle(p: vec3f, a: vec3f, b: vec3f, c: vec3f) -> vec3f {
    let ab = b - a;
    let ac = c - a;
    let n = cross(ab, ac);
    let nn = dot(n, n);
    if (nn > 0.0) {
        let q = p - n * (dot(p - a, n) / nn);
        let aq = q - a;
        let v = dot(cross(aq, ac), n) / nn;
        let w = dot(cross(ab, aq), n) / nn;
        if (v >= 0.0 && w >= 0.0 && v + w <= 1.0) {
            return q;
        }
    }
    var best = a;
    var best_dist = 3.402823466e+38;
    let edges = array<vec3f, 6>(a, b, b, c, c, a);
    for (var e = 0u; e < 3u; e++) {
        let ea = edges[e * 2u];
        let eb = edges[e * 2u + 1u];
        let d = eb - ea;
        var t = 0.0;
        if (dot(d, d) > 0.0) {
            t = clamp(dot(p - ea, d) / dot(d, d), 0.0, 1.0);
        }
        let q = ea + t * d;
        let dist_edge = distance(p, q);
        if (dist_edge < best_dist) {
            best = q;
            best_dist = dist_edge;
        }
    }
    return best;
}

// `WG` is the workgroup-size anchor: the compute-core runtime may substitute
// a per-backend tuned value (powers of two only) before compilation.
const WG: u32 = 256;

// Flat postorder field tree: leaves push, CSG/offset ops combine with a value
// stack. Node kinds match sdf-core/src/flat.rs.
@compute @workgroup_size(WG)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let row = params.nx + 1u;
    let total = row * (params.ny + 1u) * (params.nz + 1u);
    if (id.x >= total) {
        return;
    }
    let x = idx_decompose_x(id.x, row);
    let y = (id.x / row) % (params.ny + 1u);
    let z = id.x / (row * (params.ny + 1u));
    let p = vec3f(
        params.min_x + f32(x) * params.step_x,
        params.min_y + f32(y) * params.step_y,
        params.min_z + f32(z) * params.step_z,
    );
    var stack: array<f32, 33>;
    var sp = 0u;
    for (var n = 0u; n < params.n_nodes; n++) {
        let base = n * 8u;
        let kind = kinds[n];
        if (kind <= 2u || kind == 10u || (kind >= 12u && kind <= 15u)) {
            // Leaves: sphere / box / torus / capsule / cylinder / TPMS with translate-folded parameters.
            var v = 0.0;
            if (kind == 10u) {
                let a = vec3f(node_params[base], node_params[base + 1u], node_params[base + 2u]);
                let b = vec3f(node_params[base + 3u], node_params[base + 4u], node_params[base + 5u]);
                let ab = b - a;
                let ap = p - a;
                var t = 0.0;
                let denom = dot(ab, ab);
                if (denom > 0.0) {
                    t = clamp(dot(ap, ab) / denom, 0.0, 1.0);
                }
                v = length(ap - ab * t) - node_params[base + 6u];
            } else if (kind == 0u) {
                let c = vec3f(node_params[base], node_params[base + 1u], node_params[base + 2u]);
                v = length(p - c) - node_params[base + 3u];
            } else if (kind == 1u) {
                let c = vec3f(node_params[base], node_params[base + 1u], node_params[base + 2u]);
                let h = vec3f(node_params[base + 3u], node_params[base + 4u], node_params[base + 5u]);
                let q = abs(p - c) - h;
                v = length(max(q, vec3f(0.0))) + min(max(q.x, max(q.y, q.z)), 0.0);
            } else if (kind == 2u) {
                let c = vec3f(node_params[base], node_params[base + 1u], node_params[base + 2u]);
                let q = p - c;
                v = length(vec2f(length(q.xy) - node_params[base + 3u], q.z)) - node_params[base + 4u];
            } else if (kind == 12u) {
                let c = vec3f(node_params[base], node_params[base + 1u], node_params[base + 2u]);
                let q = p - c;
                let d = vec2f(length(q.xy) - node_params[base + 3u], abs(q.z) - node_params[base + 4u]);
                v = min(max(d.x, d.y), 0.0) + length(max(d, vec2f(0.0)));
            } else {
                let c = vec3f(node_params[base], node_params[base + 1u], node_params[base + 2u]);
                let period = node_params[base + 3u];
                let thickness = node_params[base + 4u];
                let k = 6.283185307179586 / period;
                let q = (p - c) * k;
                let sx = sin(q.x);
                let sy = sin(q.y);
                let sz = sin(q.z);
                let cx = cos(q.x);
                let cy = cos(q.y);
                let cz = cos(q.z);
                var iso = 0.0;
                if (kind == 13u) {
                    iso = sx * cy + sy * cz + sz * cx;
                } else if (kind == 14u) {
                    iso = cx + cy + cz;
                } else {
                    iso = sx * sy * sz + sx * cy * cz + cx * sy * cz + cx * cy * sz;
                }
                v = abs(iso) / k - 0.5 * thickness;
            }
            stack[sp] = v;
            sp++;
        } else if (kind == 8u || kind == 9u) {
            // Mesh distance: brute-force closest point (and solid angle for
            // signed) over the referenced triangle window, in scan order.
            let start = aux[n * 2u];
            let count = aux[n * 2u + 1u];
            var best = 3.402823466e+38;
            var angle = 0.0;
            for (var t = 0u; t < count; t++) {
                let base_t = (start + t) * 9u;
                let a = vec3f(tris[base_t], tris[base_t + 1u], tris[base_t + 2u]);
                let b = vec3f(tris[base_t + 3u], tris[base_t + 4u], tris[base_t + 5u]);
                let c = vec3f(tris[base_t + 6u], tris[base_t + 7u], tris[base_t + 8u]);
                best = min(best, distance(p, closest_triangle(p, a, b, c)));
                if (kind == 9u) {
                    let qa = a - p;
                    let qb = b - p;
                    let qc = c - p;
                    let la = length(qa);
                    let lb = length(qb);
                    let lc = length(qc);
                    angle += 2.0 * atan2(
                        dot(qa, cross(qb, qc)),
                        la * lb * lc + dot(qa, qb) * lc + dot(qb, qc) * la + dot(qc, qa) * lb,
                    );
                }
            }
            var v = best;
            if (kind == 9u && best > 0.0 && abs(angle) > 6.283185307) {
                v = -best;
            }
            stack[sp] = v;
            sp++;
        } else if (kind == 7u) {
            sp--;
            stack[sp] = stack[sp] - node_params[base];
            sp++;
        } else {
            sp--;
            let b = stack[sp];
            sp--;
            let a = stack[sp];
            var v = 0.0;
            if (kind == 3u) {
                v = min(a, b);
            } else if (kind == 4u) {
                v = max(a, b);
            } else if (kind == 5u) {
                v = max(a, -b);
            } else if (kind == 11u) {
                let k = node_params[base];
                let na = -a;
                let h = clamp(0.5 + 0.5 * (b - na) / k, 0.0, 1.0);
                v = -(b * (1.0 - h) + na * h - k * h * (1.0 - h));
            } else {
                let k = node_params[base];
                let h = clamp(0.5 + 0.5 * (b - a) / k, 0.0, 1.0);
                v = b * (1.0 - h) + a * h - k * h * (1.0 - h);
            }
            stack[sp] = v;
            sp++;
        }
    }
    values[id.x] = stack[0];
}
fn idx_decompose_x(idx: u32, row: u32) -> u32 {
    return idx % row;
}
"##;
pub type Point = V3;
use math_core::{cross, dot, norm as length, sub};
#[derive(Clone, Debug)]
pub enum Field {
    Extrude {
        profile: Rings,
        half_height: f64,
    },
    Revolve {
        profile: Rings,
    },
    Deform {
        input: Box<Field>,
        deformation: geometry_ops::Deformation,
    },
    MeshDistance {
        mesh: Triangles,
        signed: bool,
    },
    Sphere {
        center: Point,
        radius: f64,
    },
    Box {
        center: Point,
        half_size: Point,
    },
    Torus {
        center: Point,
        major_radius: f64,
        minor_radius: f64,
    },
    Cylinder {
        center: Point,
        radius: f64,
        half_height: f64,
    },
    Gyroid {
        origin: Point,
        period: f64,
        thickness: f64,
    },
    SchwarzP {
        origin: Point,
        period: f64,
        thickness: f64,
    },
    Diamond {
        origin: Point,
        period: f64,
        thickness: f64,
    },
    Neovius {
        origin: Point,
        period: f64,
        thickness: f64,
    },
    Lidinoid {
        origin: Point,
        period: f64,
        thickness: f64,
    },
    SchoenIWP {
        origin: Point,
        period: f64,
        thickness: f64,
    },
    Capsule {
        a: Point,
        b: Point,
        radius: f64,
    },
    Octahedron {
        center: Point,
        size: f64,
    },
    HexPrism {
        center: Point,
        radius: f64,
        half_height: f64,
    },
    TriangularPrism {
        center: Point,
        radius: f64,
        half_height: f64,
    },
    Union {
        a: Box<Field>,
        b: Box<Field>,
    },
    Intersection {
        a: Box<Field>,
        b: Box<Field>,
    },
    Difference {
        a: Box<Field>,
        b: Box<Field>,
    },
    SmoothUnion {
        a: Box<Field>,
        b: Box<Field>,
        radius: f64,
    },
    SmoothIntersection {
        a: Box<Field>,
        b: Box<Field>,
        radius: f64,
    },
    SmoothDifference {
        a: Box<Field>,
        b: Box<Field>,
        radius: f64,
    },
    ChamferUnion {
        a: Box<Field>,
        b: Box<Field>,
        radius: f64,
    },
    ChamferIntersection {
        a: Box<Field>,
        b: Box<Field>,
        radius: f64,
    },
    ChamferDifference {
        a: Box<Field>,
        b: Box<Field>,
        radius: f64,
    },
    Offset {
        input: Box<Field>,
        distance: f64,
    },
    Onion {
        input: Box<Field>,
        thickness: f64,
    },
    Elongate {
        input: Box<Field>,
        half_extents: Point,
    },
    Mirror {
        input: Box<Field>,
        axis: [bool; 3],
    },
    Taper {
        input: Box<Field>,
        height: f64,
        factor: f64,
    },
    Translate {
        input: Box<Field>,
        vector: Point,
    },
}

fn smooth_min(a: f64, b: f64, k: f64) -> f64 {
    let h = (0.5 + 0.5 * (b - a) / k).clamp(0., 1.);
    b * (1. - h) + a * h - k * h * (1. - h)
}
fn positive(x: f64) -> bool {
    x.is_finite() && x > 0. && x <= 1e6
}
/// Triangle corner positions as `[x, y, z]` rows: one bounds check per corner
/// instead of three, and the chunk view is hoisted out of the sampling loops.
#[inline(always)]
fn corners(mesh: &Triangles) -> &[[f64; 3]] {
    mesh.positions.as_chunks::<3>().0
}
#[inline(always)]
fn point(mesh: &Triangles, i: usize) -> Point {
    corners(mesh)[i]
}
fn triangles_ok(mesh: &Triangles) -> bool {
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
fn triangles_closed(mesh: &Triangles) -> bool {
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
fn finish_triangles(mesh: Triangles) -> Result<Triangles> {
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
    fn sample(&self, p: Point) -> f64 {
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
    fn sample_work(&self) -> usize {
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
#[derive(Clone, Debug)]
pub struct Grid {
    pub min: Point,
    pub max: Point,
    pub cells: [usize; 3],
}

/// Also accepts user-defined scalar fields. Sampling is bounded; sub-cell features
/// can be missed. A negative boundary sample is rejected rather than capped.
pub fn polygonize_with(field: impl Fn(Point) -> f64, grid: &Grid) -> Result<Triangles> {
    polygonize_tile(field, grid, true)
}

/// Budget and field validation shared by every extraction path.
pub fn check_grid_budget(field: &Field, grid: &Grid) -> Result<()> {
    field.validate()?;
    if grid.cells.iter().any(|&n| n > 64)
        || grid
            .cells
            .iter()
            .map(|n| n + 1)
            .product::<usize>()
            .saturating_mul(field.sample_work())
            > 8_000_000
    {
        return Err(error("SDF extraction exceeds 8000000 sample work units"));
    }
    Ok(())
}
pub fn polygonize(field: &Field, grid: &Grid) -> Result<Triangles> {
    check_grid_budget(field, grid)?;
    #[cfg(not(target_arch = "wasm32"))]
    {
        if finite(grid.min)
            && finite(grid.max)
            && grid.cells.iter().all(|&n| (1..=64).contains(&n))
            && (0..3).all(|i| grid.max[i] > grid.min[i])
        {
            let [nx, ny, nz] = grid.cells;
            let sample_count = (nx + 1) * (ny + 1) * (nz + 1);
            let total_work = sample_count.saturating_mul(field.sample_work());
            let threads = std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1);
            if threads > 1 && nz >= 2 && total_work >= 4_096 {
                let plane_size = (nx + 1) * (ny + 1);
                let z_count = nz + 1;
                let chunk_z = z_count.div_ceil(threads).max(1);
                let mut precomputed = Vec::with_capacity(sample_count);
                std::thread::scope(|s| {
                    let mut handles = Vec::with_capacity(z_count.div_ceil(chunk_z));
                    let mut z_start = 0usize;
                    while z_start < z_count {
                        let z_end = (z_start + chunk_z).min(z_count);
                        handles.push(s.spawn(move || {
                            let mut chunk_vals = Vec::with_capacity((z_end - z_start) * plane_size);
                            for z in z_start..z_end {
                                for y in 0..=ny {
                                    for x in 0..=nx {
                                        let p = std::array::from_fn(|i| {
                                            grid.min[i]
                                                + (grid.max[i] - grid.min[i]) * [x, y, z][i] as f64
                                                    / grid.cells[i] as f64
                                        });
                                        chunk_vals.push(field.sample(p));
                                    }
                                }
                            }
                            chunk_vals
                        }));
                        z_start = z_end;
                    }
                    for handle in handles {
                        precomputed.extend(handle.join().expect("sdf worker panicked"));
                    }
                });
                let cursor = std::cell::Cell::new(0usize);
                return polygonize_with(
                    |_| {
                        let i = cursor.get();
                        cursor.set(i + 1);
                        precomputed[i]
                    },
                    grid,
                );
            }
        }
    }
    polygonize_with(|p| field.sample(p), grid)
}
/// Rebuilds the mesh from externally computed grid samples (GPU shader or the
/// browser WebGPU host path). The snap-to-zero, boundary validation and
/// marching-tetrahedra extraction run here on the CPU, exactly as in
/// `polygonize`; only the raw field values arrive precomputed (f32).
pub fn polygonize_with_values(field: &Field, grid: &Grid, values: &[f32]) -> Result<Triangles> {
    check_grid_budget(field, grid)?;
    let expected = grid.cells.iter().map(|n| n + 1).product::<usize>();
    if values.len() != expected {
        return Err(error("Grid sample count does not match the grid"));
    }
    if values.iter().any(|v| !v.is_finite()) {
        return Err(error("Field returned a non-finite value"));
    }
    let cursor = std::cell::Cell::new(0usize);
    polygonize_with(
        |_| {
            let i = cursor.get();
            cursor.set(i + 1);
            values[i] as f64
        },
        grid,
    )
}

fn grid_sample_count(grid: &Grid) -> usize {
    grid.cells.iter().map(|n| n + 1).product()
}

/// Suggests a placement for [`polygonize_accelerated`] from grid size and SDF
/// field cost. Primitive CSG fields are cheap per point, while mesh-distance
/// fields do a brute-force triangle scan at every grid sample; this combines
/// both into `grid samples * field.sample_work()` units.
///
/// Measured with `examples/bench_gpu.rs` on an NVIDIA RTX 5090 (CUDA 13.4):
/// primitive CSG at 64^3 (~550K work units) is only slightly faster on
/// CUDA/wgpu (67ms CPU vs 62ms device), while a 1088-triangle mesh-distance
/// field at 16^3 (~5.3M work units) is ~200x faster (324ms CPU vs 1.3-1.6ms
/// device). `Auto` therefore stays on CPU below 500K work and prefers CUDA
/// (or portable wgpu, including Metal on macOS) once the device launch cost is
/// likely amortized.
pub fn recommended_for_polygonize(field: &Field, grid: &Grid) -> Acceleration {
    const SDF_GPU_WORK_THRESHOLD: usize = 500_000;
    let work = grid_sample_count(grid).saturating_mul(field.sample_work());
    if work < SDF_GPU_WORK_THRESHOLD {
        Acceleration::Cpu
    } else if cfg!(feature = "cuda") {
        Acceleration::Cuda
    } else {
        Acceleration::Gpu
    }
}

fn resolve_polygonize_acceleration(
    acceleration: Acceleration,
    field: &Field,
    grid: &Grid,
) -> Acceleration {
    match acceleration {
        Acceleration::Auto => recommended_for_polygonize(field, grid),
        explicit => explicit,
    }
}

/// `polygonize` with an optional GPU grid sampler. Eligible fields (primitive
/// and CSG trees) sample the grid on the GPU in f32; the snap-to-zero, boundary
/// validation and marching-tetrahedra extraction stay on the CPU. Everything
/// else — and any failure — falls back to the CPU reference.
///
/// `Acceleration::Auto` uses [`recommended_for_polygonize`].
/// `Acceleration::Cuda` runs the PTX port through the CUDA driver (feature
/// `cuda`), then the wgpu shader (feature `gpu`, Metal on macOS), then the CPU
/// reference.
pub fn polygonize_accelerated(
    field: &Field,
    grid: &Grid,
    #[allow(unused_variables)] acceleration: Acceleration,
) -> Result<Triangles> {
    #[allow(unused_variables)]
    let acceleration = resolve_polygonize_acceleration(acceleration, field, grid);
    #[cfg(feature = "gpu")]
    if acceleration.is_gpu() {
        check_grid_budget(field, grid)?;
        if let Some(flat) = field.to_flat() {
            #[cfg(feature = "cuda")]
            if acceleration == Acceleration::Cuda
                && let Some(values) = cuda::sample_grid_cuda(&flat, grid)
            {
                return polygonize_with_values(field, grid, &values);
            }
            if let Some(values) = gpu::sample_grid_gpu(&flat, grid) {
                return polygonize_with_values(field, grid, &values);
            }
        }
    }
    polygonize(field, grid)
}

impl Field {
    pub fn deform(&self, deformation: geometry_ops::Deformation) -> Result<Self> {
        self.validate()?;
        let out = Self::Deform {
            input: Box::new(self.clone()),
            deformation,
        };
        out.validate()?;
        Ok(out)
    }
    /// Applies one sculpt stroke: a primitive tool shape combined with the field
    /// by hard or smooth-blended add/remove.
    pub fn sculpt(&self, stroke: &SdfStroke) -> Result<Self> {
        self.validate()?;
        stroke.validate()?;
        let tool = stroke.tool.field();
        tool.validate()?;
        let a = Box::new(self.clone());
        let b = Box::new(tool);
        let out = match (stroke.remove, stroke.blend) {
            (false, None) => Self::Union { a, b },
            (true, None) => Self::Difference { a, b },
            (false, Some(radius)) => Self::SmoothUnion { a, b, radius },
            (true, Some(radius)) => Self::SmoothDifference { a, b, radius },
        };
        out.validate()?;
        Ok(out)
    }
    /// Hard sphere add/remove; the historical entry point, now a `sculpt` shorthand.
    pub fn sculpt_sphere(&self, center: Point, radius: f64, remove: bool) -> Result<Self> {
        self.sculpt(&SdfStroke {
            tool: SdfTool::Sphere { center, radius },
            remove,
            blend: None,
        })
    }
}

/// Tool shape of an SDF sculpt stroke.
#[derive(Clone, Debug, PartialEq)]
pub enum SdfTool {
    Sphere { center: Point, radius: f64 },
    Box { center: Point, half_size: Point },
    Capsule { a: Point, b: Point, radius: f64 },
}
impl SdfTool {
    pub fn field(&self) -> Field {
        match self {
            Self::Sphere { center, radius } => Field::Sphere {
                center: *center,
                radius: *radius,
            },
            Self::Box { center, half_size } => Field::Box {
                center: *center,
                half_size: *half_size,
            },
            Self::Capsule { a, b, radius } => Field::Capsule {
                a: *a,
                b: *b,
                radius: *radius,
            },
        }
    }
}

/// One SDF sculpt stroke. `blend` is the smooth-blend radius; `None` keeps hard CSG.
#[derive(Clone, Debug, PartialEq)]
pub struct SdfStroke {
    pub tool: SdfTool,
    pub remove: bool,
    pub blend: Option<f64>,
}
impl SdfStroke {
    pub fn validate(&self) -> Result<()> {
        self.tool.field().validate()?;
        match self.blend {
            Some(k) if !positive(k) => Err(error("Invalid sculpt blend radius")),
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
#[path = "tests/tests_integration.rs"]
mod tests;

impl Field {
    /// Computes the unit outward gradient $\nabla f(p) / \|\nabla f(p)\|$ using a 4-point
    /// tetrahedral finite-difference stencil.
    pub fn gradient(&self, p: Point, eps: f64) -> Point {
        let h = eps.max(1e-7);
        let d0 = self.sample([p[0] + h, p[1] - h, p[2] - h]);
        let d1 = self.sample([p[0] - h, p[1] - h, p[2] + h]);
        let d2 = self.sample([p[0] - h, p[1] + h, p[2] - h]);
        let d3 = self.sample([p[0] + h, p[1] + h, p[2] + h]);
        let g = [
            d0 - d1 - d2 + d3,
            -d0 - d1 + d2 + d3,
            -d0 + d1 - d2 + d3,
        ];
        let n = length(g);
        if n > 1e-15 {
            [g[0] / n, g[1] / n, g[2] / n]
        } else {
            [0.0, 0.0, 1.0]
        }
    }

    /// Sphere-tracing raymarcher along a ray from `origin` in direction `dir` up to `max_dist`
    /// with surface hit tolerance `tol`. Returns `Some((hit_position, hit_normal, distance))`
    /// on convergence to the zero-level surface ($|f(p)| \le \text{tol}$).
    pub fn raycast(
        &self,
        origin: Point,
        dir: Point,
        max_dist: f64,
        tol: f64,
    ) -> Option<(Point, Point, f64)> {
        if !finite(origin) || !finite(dir) || !max_dist.is_finite() || max_dist <= 0.0 || !tol.is_finite() || tol <= 0.0 {
            return None;
        }
        let dir_len = length(dir);
        if dir_len <= 1e-12 {
            return None;
        }
        let d = [dir[0] / dir_len, dir[1] / dir_len, dir[2] / dir_len];
        let mut t = 0.0;
        let eps = tol.max(1e-6);
        for _ in 0..256 {
            let p = [origin[0] + d[0] * t, origin[1] + d[1] * t, origin[2] + d[2] * t];
            let dist = self.sample(p);
            if !dist.is_finite() {
                return None;
            }
            if dist.abs() <= tol {
                let normal = self.gradient(p, eps);
                return Some((p, normal, t));
            }
            t += dist.max(tol * 0.5);
            if t > max_dist {
                break;
            }
        }
        None
    }

    /// Estimates the enclosed interior volume ($f(p) \le 0$) within `bounds_min..=bounds_max`
    /// using stratified voxel midpoint integration with `samples_per_axis` subdivisions per axis.
    pub fn estimate_volume_in_bounds(
        &self,
        bounds_min: Point,
        bounds_max: Point,
        samples_per_axis: usize,
    ) -> f64 {
        let n = samples_per_axis.clamp(1, 128);
        let dx = (bounds_max[0] - bounds_min[0]) / n as f64;
        let dy = (bounds_max[1] - bounds_min[1]) / n as f64;
        let dz = (bounds_max[2] - bounds_min[2]) / n as f64;
        if dx <= 0.0 || dy <= 0.0 || dz <= 0.0 || !dx.is_finite() || !dy.is_finite() || !dz.is_finite() {
            return 0.0;
        }
        let voxel_vol = dx * dy * dz;
        let mut inside = 0usize;
        for iz in 0..n {
            let z = bounds_min[2] + (iz as f64 + 0.5) * dz;
            for iy in 0..n {
                let y = bounds_min[1] + (iy as f64 + 0.5) * dy;
                for ix in 0..n {
                    let x = bounds_min[0] + (ix as f64 + 0.5) * dx;
                    if self.sample([x, y, z]) <= 0.0 {
                        inside += 1;
                    }
                }
            }
        }
        inside as f64 * voxel_vol
    }

    /// Estimates the enclosed interior volume ($f(p) \le 0$) using an automatically inferred bounding box
    /// and `samples_per_axis` subdivisions along each axis.
    pub fn estimate_volume(&self, samples_per_axis: usize) -> f64 {
        let (bmin, bmax) = self.infer_bounds();
        self.estimate_volume_in_bounds(bmin, bmax, samples_per_axis)
    }

    fn infer_bounds(&self) -> (Point, Point) {
        match self {
            Self::Sphere { center, radius } => (
                [center[0] - radius, center[1] - radius, center[2] - radius],
                [center[0] + radius, center[1] + radius, center[2] + radius],
            ),
            Self::Box { center, half_size } => (
                [
                    center[0] - half_size[0],
                    center[1] - half_size[1],
                    center[2] - half_size[2],
                ],
                [
                    center[0] + half_size[0],
                    center[1] + half_size[1],
                    center[2] + half_size[2],
                ],
            ),
            Self::Cylinder {
                center,
                radius,
                half_height,
            }
            | Self::HexPrism {
                center,
                radius,
                half_height,
            }
            | Self::TriangularPrism {
                center,
                radius,
                half_height,
            } => (
                [
                    center[0] - radius,
                    center[1] - radius,
                    center[2] - half_height,
                ],
                [
                    center[0] + radius,
                    center[1] + radius,
                    center[2] + half_height,
                ],
            ),
            Self::Octahedron { center, size } => (
                [center[0] - size, center[1] - size, center[2] - size],
                [center[0] + size, center[1] + size, center[2] + size],
            ),
            Self::Torus {
                center,
                major_radius,
                minor_radius,
            } => {
                let r = major_radius + minor_radius;
                (
                    [center[0] - r, center[1] - r, center[2] - minor_radius],
                    [center[0] + r, center[1] + r, center[2] + minor_radius],
                )
            }
            Self::Capsule { a, b, radius } => (
                [
                    a[0].min(b[0]) - radius,
                    a[1].min(b[1]) - radius,
                    a[2].min(b[2]) - radius,
                ],
                [
                    a[0].max(b[0]) + radius,
                    a[1].max(b[1]) + radius,
                    a[2].max(b[2]) + radius,
                ],
            ),
            Self::Union { a, b }
            | Self::SmoothUnion { a, b, .. }
            | Self::ChamferUnion { a, b, .. } => {
                let (min_a, max_a) = a.infer_bounds();
                let (min_b, max_b) = b.infer_bounds();
                (
                    [
                        min_a[0].min(min_b[0]),
                        min_a[1].min(min_b[1]),
                        min_a[2].min(min_b[2]),
                    ],
                    [
                        max_a[0].max(max_b[0]),
                        max_a[1].max(max_b[1]),
                        max_a[2].max(max_b[2]),
                    ],
                )
            }
            Self::Intersection { a, b }
            | Self::SmoothIntersection { a, b, .. }
            | Self::ChamferIntersection { a, b, .. } => {
                let (min_a, max_a) = a.infer_bounds();
                let (min_b, max_b) = b.infer_bounds();
                (
                    [
                        min_a[0].max(min_b[0]),
                        min_a[1].max(min_b[1]),
                        min_a[2].max(min_b[2]),
                    ],
                    [
                        max_a[0].min(max_b[0]),
                        max_a[1].min(max_b[1]),
                        max_a[2].min(max_b[2]),
                    ],
                )
            }
            Self::Difference { a, .. }
            | Self::SmoothDifference { a, .. }
            | Self::ChamferDifference { a, .. } => a.infer_bounds(),
            Self::Translate { input, vector } => {
                let (bmin, bmax) = input.infer_bounds();
                (
                    [bmin[0] + vector[0], bmin[1] + vector[1], bmin[2] + vector[2]],
                    [bmax[0] + vector[0], bmax[1] + vector[1], bmax[2] + vector[2]],
                )
            }
            Self::Offset { input, distance } => {
                let (bmin, bmax) = input.infer_bounds();
                (
                    [bmin[0] - distance, bmin[1] - distance, bmin[2] - distance],
                    [bmax[0] + distance, bmax[1] + distance, bmax[2] + distance],
                )
            }
            Self::Onion { input, thickness } => {
                let (bmin, bmax) = input.infer_bounds();
                let h = 0.5 * thickness;
                (
                    [bmin[0] - h, bmin[1] - h, bmin[2] - h],
                    [bmax[0] + h, bmax[1] + h, bmax[2] + h],
                )
            }
            Self::Elongate {
                input,
                half_extents,
            } => {
                let (bmin, bmax) = input.infer_bounds();
                (
                    [
                        bmin[0] - half_extents[0],
                        bmin[1] - half_extents[1],
                        bmin[2] - half_extents[2],
                    ],
                    [
                        bmax[0] + half_extents[0],
                        bmax[1] + half_extents[1],
                        bmax[2] + half_extents[2],
                    ],
                )
            }
            Self::Mirror { input, axis } => {
                let (bmin, bmax) = input.infer_bounds();
                let bound = |i: usize| {
                    if axis[i] {
                        let m = bmin[i].abs().max(bmax[i].abs());
                        (-m, m)
                    } else {
                        (bmin[i], bmax[i])
                    }
                };
                let (x0, x1) = bound(0);
                let (y0, y1) = bound(1);
                let (z0, z1) = bound(2);
                ([x0, y0, z0], [x1, y1, z1])
            }
            Self::Taper { input, factor, .. } => {
                let (bmin, bmax) = input.infer_bounds();
                let s = factor.max(1.0);
                (
                    [bmin[0] * s, bmin[1] * s, bmin[2]],
                    [bmax[0] * s, bmax[1] * s, bmax[2]],
                )
            }
            _ => ([-2.0, -2.0, -2.0], [2.0, 2.0, 2.0]),
        }
    }
}



#[path = "surface_polygonization.rs"]
mod surface_polygonization;
pub use surface_polygonization::polygonize_surface_nets;
pub use surface_polygonization::polygonize_tile;
