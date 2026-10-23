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
/// Extract an open tile for a caller that welds and validates the complete surface.
pub fn polygonize_tile(
    field: impl Fn(Point) -> f64,
    grid: &Grid,
    require_closed_bounds: bool,
) -> Result<Triangles> {
    if !finite(grid.min)
        || !finite(grid.max)
        || grid.cells.iter().any(|&n| n == 0 || n > 64)
        || (0..3).any(|i| grid.max[i] <= grid.min[i])
    {
        return Err(error(
            "Invalid grid: each axis requires 1..64 cells and ordered finite bounds",
        ));
    }
    let [nx, ny, nz] = grid.cells;
    let index = |x: usize, y: usize, z: usize| (z * (ny + 1) + y) * (nx + 1) + x;
    let sample_count = (nx + 1) * (ny + 1) * (nz + 1);
    let mut points = Vec::with_capacity(sample_count);
    let mut values = Vec::with_capacity(sample_count);
    for z in 0..=nz {
        for y in 0..=ny {
            for x in 0..=nx {
                let p = std::array::from_fn(|i| {
                    grid.min[i]
                        + (grid.max[i] - grid.min[i]) * [x, y, z][i] as f64 / grid.cells[i] as f64
                });
                let v = field(p);
                let v = if v.abs()
                    <= 32.
                        * f64::EPSILON
                        * (0..3).map(|i| grid.max[i] - grid.min[i]).fold(0., f64::max)
                {
                    0.
                } else {
                    v
                };
                if !v.is_finite() {
                    return Err(error("Field returned a non-finite value"));
                }
                if require_closed_bounds
                    && (x == 0 || y == 0 || z == 0 || x == nx || y == ny || z == nz)
                    && v <= 0.
                {
                    return Err(error("Surface touches grid boundary; enlarge bounds"));
                }
                points.push(p);
                values.push(v);
            }
        }
    }
    let cell_estimate = (nx * ny + ny * nz + nz * nx) * 2;
    let mut mesh = Triangles {
        positions: Vec::with_capacity(cell_estimate * 3),
        indices: Vec::with_capacity(cell_estimate * 6),
    };
    // Edge cache is only ever probed by key (entry API); iteration order never
    // reaches the mesh, so a hash map keeps the output bit-identical while
    // avoiding per-entry tree comparisons.
    let mut cache = rustc_hash::FxHashMap::with_capacity_and_hasher(
        cell_estimate,
        Default::default(),
    );
    for z in 0..nz {
        for y in 0..ny {
            for x in 0..nx {
                let cube = [
                    index(x, y, z),
                    index(x + 1, y, z),
                    index(x + 1, y + 1, z),
                    index(x, y + 1, z),
                    index(x, y, z + 1),
                    index(x + 1, y, z + 1),
                    index(x + 1, y + 1, z + 1),
                    index(x, y + 1, z + 1),
                ];
                for tet in [
                    [0, 1, 2, 6],
                    [0, 2, 3, 6],
                    [0, 3, 7, 6],
                    [0, 7, 4, 6],
                    [0, 4, 5, 6],
                    [0, 5, 1, 6],
                ] {
                    let t = tet.map(|i| cube[i]);
                    let mut polygon = [0usize; 4];
                    let mut poly_len = 0usize;
                    let mut outward = [0.; 3];
                    let mut ni = 0.;
                    let mut no = 0.;
                    let mut inside = [0.; 3];
                    let mut outside = [0.; 3];
                    for &i in &t {
                        if values[i] < 0. {
                            ni += 1.;
                            for (k, v) in inside.iter_mut().enumerate() {
                                *v += points[i][k];
                            }
                        } else {
                            no += 1.;
                            for (k, v) in outside.iter_mut().enumerate() {
                                *v += points[i][k];
                            }
                        }
                    }
                    if ni == 0. || no == 0. {
                        continue;
                    }
                    for k in 0..3 {
                        outward[k] = outside[k] / no - inside[k] / ni;
                    }
                    for [a, b] in [[0, 1], [0, 2], [0, 3], [1, 2], [1, 3], [2, 3]] {
                        let a = t[a];
                        let b = t[b];
                        if (values[a] < 0.) == (values[b] < 0.) {
                            continue;
                        }
                        let key = if values[a] == 0. {
                            (a, a)
                        } else if values[b] == 0. {
                            (b, b)
                        } else {
                            (a.min(b), a.max(b))
                        };
                        let id = *cache.entry(key).or_insert_with(|| {
                            let ratio = values[a] / (values[a] - values[b]);
                            let id = mesh.positions.len() / 3;
                            mesh.positions.extend(
                                points[a]
                                    .iter()
                                    .zip(points[b])
                                    .map(|(&a, b)| a + ratio * (b - a)),
                            );
                            id
                        });
                        if !polygon[..poly_len].contains(&id) {
                            polygon[poly_len] = id;
                            poly_len += 1;
                        }
                    }
                    if poly_len < 3 {
                        continue;
                    }
                    let polygon = &mut polygon[..poly_len];
                    let point = |i: usize| {
                        [
                            mesh.positions[i * 3],
                            mesh.positions[i * 3 + 1],
                            mesh.positions[i * 3 + 2],
                        ]
                    };
                    let mut center = [0.; 3];
                    for &i in &*polygon {
                        for (k, v) in center.iter_mut().enumerate() {
                            *v += point(i)[k] / polygon.len() as f64;
                        }
                    }
                    let u = sub(point(polygon[0]), center);
                    let v = cross(outward, u);
                    polygon.sort_by(|&a, &b| {
                        let a = sub(point(a), center);
                        let b = sub(point(b), center);
                        dot(a, v)
                            .atan2(dot(a, u))
                            .total_cmp(&dot(b, v).atan2(dot(b, u)))
                    });
                    for i in 1..polygon.len() - 1 {
                        let a = polygon[0];
                        let mut b = polygon[i];
                        let mut c = polygon[i + 1];
                        let normal = cross(sub(point(b), point(a)), sub(point(c), point(a)));
                        if length(normal) == 0. {
                            continue;
                        }
                        if dot(normal, outward) < 0. {
                            std::mem::swap(&mut b, &mut c);
                        }
                        mesh.indices.extend([a, b, c]);
                    }
                    if mesh.indices.len() / 3 > 100_000 {
                        return Err(error("Implicit mesh exceeds 100000 triangles"));
                    }
                }
            }
        }
    }
    finish_triangles(mesh)
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
mod tests {
    use super::*;

    #[test]
    fn polygonize_auto_recommendation_tracks_work_size() {
        let sphere = Field::Sphere {
            center: [0.; 3],
            radius: 1.,
        };
        let tiny = Grid {
            min: [-1.5; 3],
            max: [1.5; 3],
            cells: [8, 8, 8],
        };
        assert_eq!(
            recommended_for_polygonize(&sphere, &tiny),
            Acceleration::Cpu
        );
        let large = Grid {
            cells: [64, 64, 64],
            ..tiny.clone()
        };
        let smooth = Field::SmoothUnion {
            a: Box::new(sphere.clone()),
            b: Box::new(Field::Box {
                center: [0.5; 3],
                half_size: [0.75; 3],
            }),
            radius: 0.25,
        };
        assert!(recommended_for_polygonize(&smooth, &large).is_gpu());
        let mut mesh = Triangles {
            positions: vec![
                0., 0., 0., //
                1., 0., 0., 0., 1., 0.,
            ],
            indices: Vec::new(),
        };
        for _ in 0..1024 {
            mesh.indices.extend([0, 1, 2]);
        }
        let mesh_field = Field::from_triangles(mesh, false).unwrap();
        assert!(recommended_for_polygonize(&mesh_field, &tiny).is_gpu());
    }

    #[test]
    fn polygonize_auto_matches_cpu_on_small_grid() {
        let field = Field::Sphere {
            center: [0.; 3],
            radius: 1.,
        };
        let grid = Grid {
            min: [-1.5; 3],
            max: [1.5; 3],
            cells: [8, 8, 8],
        };
        let cpu = polygonize(&field, &grid).unwrap();
        let automatic = polygonize_accelerated(&field, &grid, Acceleration::Auto).unwrap();
        assert_eq!(cpu.indices, automatic.indices);
        assert_eq!(cpu.positions, automatic.positions);
    }

    #[cfg(feature = "gpu")]
    #[test]
    fn gpu_sampling_matches_cpu_field_within_tolerance() {
        let field = Field::Translate {
            input: Box::new(Field::SmoothUnion {
                a: Box::new(Field::Sphere {
                    center: [0., 0., 0.],
                    radius: 10.,
                }),
                b: Box::new(Field::Box {
                    center: [8., 0., 0.],
                    half_size: [6., 6., 6.],
                }),
                radius: 3.,
            }),
            vector: [1., -2., 0.5],
        };
        let Some(flat) = field.to_flat() else {
            panic!("CSG tree should flatten")
        };
        let grid = Grid {
            min: [-12., -12., -12.],
            max: [16., 12., 12.],
            cells: [24, 16, 16],
        };
        let Some(values) = gpu::sample_grid_gpu(&flat, &grid) else {
            eprintln!("no GPU adapter; skipping");
            return;
        };
        let [nx, ny, nz] = grid.cells;
        let mut max_diff = 0f64;
        for z in 0..=nz {
            for y in 0..=ny {
                for x in 0..=nx {
                    let p = std::array::from_fn(|i| {
                        grid.min[i]
                            + (grid.max[i] - grid.min[i]) * [x, y, z][i] as f64
                                / grid.cells[i] as f64
                    });
                    let gpu = values[(z * (ny + 1) + y) * (nx + 1) + x] as f64;
                    max_diff = max_diff.max((gpu - field.sample(p)).abs());
                }
            }
        }
        assert!(max_diff < 0.01, "GPU field diverges: {max_diff}");
        // Full extraction: same topology class and volume within f32 tolerance.
        let cpu_mesh = polygonize(&field, &grid).unwrap();
        let gpu_mesh = polygonize_accelerated(&field, &grid, Acceleration::Gpu).unwrap();
        let cpu_tris = cpu_mesh.indices.len() / 3;
        let gpu_tris = gpu_mesh.indices.len() / 3;
        assert!(
            (cpu_tris as f64 - gpu_tris as f64).abs() <= 0.02 * cpu_tris as f64,
            "triangle counts diverge: {cpu_tris} vs {gpu_tris}"
        );
    }

    #[cfg(feature = "gpu")]
    #[test]
    fn gpu_mesh_distance_matches_cpu_signed_and_unsigned() {
        // Closed outward tetrahedron around the origin-ish region.
        let mesh = Triangles {
            positions: vec![
                10., 10., 10., //
                30., 10., 10., 10., 30., 10., 10., 10., 30.,
            ],
            indices: vec![0, 2, 1, 0, 1, 3, 1, 2, 3, 2, 0, 3],
        };
        let field = Field::from_triangles(mesh, true).unwrap();
        let Some(flat) = field.to_flat() else {
            panic!("mesh field should flatten")
        };
        assert_eq!(flat.triangles.len(), 4 * 9);
        let grid = Grid {
            min: [0., 0., 0.],
            max: [40., 40., 40.],
            cells: [8, 8, 8],
        };
        let Some(values) = gpu::sample_grid_gpu(&flat, &grid) else {
            eprintln!("no GPU adapter; skipping");
            return;
        };
        let [nx, ny, nz] = grid.cells;
        let mut max_diff = 0f64;
        let mut inside_cpu = 0;
        let mut inside_gpu = 0;
        for z in 0..=nz {
            for y in 0..=ny {
                for x in 0..=nx {
                    let p = std::array::from_fn(|i| {
                        grid.min[i]
                            + (grid.max[i] - grid.min[i]) * [x, y, z][i] as f64
                                / grid.cells[i] as f64
                    });
                    let cpu = field.sample(p);
                    let gpu = values[(z * (ny + 1) + y) * (nx + 1) + x] as f64;
                    max_diff = max_diff.max((gpu - cpu).abs());
                    if cpu < 0. {
                        inside_cpu += 1;
                    }
                    if gpu < 0. {
                        inside_gpu += 1;
                    }
                }
            }
        }
        assert!(inside_cpu > 0, "tetrahedron should contain grid points");
        assert_eq!(
            inside_cpu, inside_gpu,
            "inside/outside classification diverges"
        );
        assert!(max_diff < 0.01, "signed distance diverges: {max_diff}");
    }

    /// CSG tree plus a signed mesh-distance leaf: every node kind the flat
    /// interpreter implements, so the CUDA port is exercised end to end.
    #[cfg(feature = "cuda")]
    fn cuda_fixture() -> (Field, Grid) {
        let tetra = Triangles {
            positions: vec![
                -6., -6., -6., //
                6., -6., -6., -6., 6., -6., -6., -6., 6.,
            ],
            indices: vec![0, 2, 1, 0, 1, 3, 1, 2, 3, 2, 0, 3],
        };
        let field = Field::Translate {
            input: Box::new(Field::Difference {
                a: Box::new(Field::SmoothUnion {
                    a: Box::new(Field::Sphere {
                        center: [0., 0., 0.],
                        radius: 8.,
                    }),
                    b: Box::new(Field::Offset {
                        input: Box::new(Field::Torus {
                            center: [4., 0., 0.],
                            major_radius: 4.,
                            minor_radius: 1.5,
                        }),
                        distance: 0.5,
                    }),
                    radius: 3.,
                }),
                b: Box::new(Field::Intersection {
                    a: Box::new(Field::Box {
                        center: [0., 0., 0.],
                        half_size: [5., 5., 5.],
                    }),
                    b: Box::new(Field::from_triangles(tetra, true).unwrap()),
                }),
            }),
            vector: [1., -2., 0.5],
        };
        let grid = Grid {
            min: [-12., -12., -12.],
            max: [16., 12., 12.],
            cells: [28, 24, 24],
        };
        (field, grid)
    }

    #[cfg(feature = "cuda")]
    #[test]
    fn cuda_sampling_matches_cpu_field_within_tolerance() {
        let (field, grid) = cuda_fixture();
        let flat = field.to_flat().expect("fixture should flatten");
        let Some(values) = cuda::sample_grid_cuda(&flat, &grid) else {
            eprintln!("no CUDA device; skipping");
            return;
        };
        let [nx, ny, nz] = grid.cells;
        assert_eq!(values.len(), (nx + 1) * (ny + 1) * (nz + 1));
        let mut max_diff = 0f64;
        let (mut inside_cpu, mut inside_cuda) = (0, 0);
        for z in 0..=nz {
            for y in 0..=ny {
                for x in 0..=nx {
                    let p = std::array::from_fn(|i| {
                        grid.min[i]
                            + (grid.max[i] - grid.min[i]) * [x, y, z][i] as f64
                                / grid.cells[i] as f64
                    });
                    let cpu = field.sample(p);
                    let cuda = values[(z * (ny + 1) + y) * (nx + 1) + x] as f64;
                    max_diff = max_diff.max((cuda - cpu).abs());
                    inside_cpu += (cpu < 0.) as usize;
                    inside_cuda += (cuda < 0.) as usize;
                }
            }
        }
        assert!(inside_cpu > 0);
        assert_eq!(
            inside_cpu, inside_cuda,
            "inside/outside classification diverges"
        );
        assert!(max_diff < 0.01, "CUDA field diverges: {max_diff}");
        let cpu_mesh = polygonize(&field, &grid).unwrap();
        let cuda_mesh = polygonize_accelerated(&field, &grid, Acceleration::Cuda).unwrap();
        let cpu_tris = cpu_mesh.indices.len() / 3;
        let cuda_tris = cuda_mesh.indices.len() / 3;
        assert!(
            (cpu_tris as f64 - cuda_tris as f64).abs() <= 0.02 * cpu_tris as f64,
            "triangle counts diverge: {cpu_tris} vs {cuda_tris}"
        );
    }

    /// The PTX and WGSL kernels are ports of the same text; on the same device
    /// class they should agree to f32 rounding, far tighter than the CPU bound.
    #[cfg(feature = "cuda")]
    #[test]
    fn cuda_and_wgpu_samplers_agree() {
        let (field, grid) = cuda_fixture();
        let flat = field.to_flat().expect("fixture should flatten");
        let (Some(cuda), Some(wgpu)) = (
            cuda::sample_grid_cuda(&flat, &grid),
            gpu::sample_grid_gpu(&flat, &grid),
        ) else {
            eprintln!("CUDA or wgpu unavailable; skipping");
            return;
        };
        assert_eq!(cuda.len(), wgpu.len());
        let max_diff = cuda
            .iter()
            .zip(&wgpu)
            .map(|(a, b)| (a - b).abs())
            .fold(0f32, f32::max);
        assert!(
            max_diff < 1e-3,
            "CUDA and wgpu samplers diverge: {max_diff}"
        );
    }

    #[cfg(feature = "cuda")]
    #[test]
    fn cuda_request_without_device_falls_back() {
        // Whatever the host has, `Cuda` must never fail where the CPU succeeds.
        let (field, grid) = cuda_fixture();
        let cpu = polygonize(&field, &grid).unwrap();
        let cuda = polygonize_accelerated(&field, &grid, Acceleration::Cuda).unwrap();
        assert!(!cuda.indices.is_empty());
        let flat = field.to_flat().unwrap();
        if !cuda::available() && gpu::sample_grid_gpu(&flat, &grid).is_none() {
            assert_eq!(
                cpu.indices, cuda.indices,
                "CPU fallback must be bit-identical"
            );
        }
    }

    fn sphere() -> Field {
        Field::Sphere {
            center: [0.; 3],
            radius: 1.,
        }
    }
    fn grid(n: usize) -> Grid {
        Grid {
            min: [-1.5; 3],
            max: [1.5; 3],
            cells: [n; 3],
        }
    }
    #[test]
    fn primitives_and_csg() {
        assert_eq!(sphere().evaluate([0.; 3]).unwrap(), -1.);
        let f = Field::Difference {
            a: Box::new(sphere()),
            b: Box::new(Field::Sphere {
                center: [0.; 3],
                radius: 0.5,
            }),
        };
        assert_eq!(f.evaluate([0.; 3]).unwrap(), 0.5);
        assert!(f.evaluate([0.75, 0., 0.]).unwrap() < 0.);
    }
    fn signed_volume(mesh: &Triangles) -> f64 {
        mesh.indices
            .as_chunks::<3>()
            .0
            .iter()
            .map(|t| {
                let a = point(mesh, t[0]);
                let b = point(mesh, t[1]);
                let c = point(mesh, t[2]);
                dot(a, cross(b, c)) / 6.
            })
            .sum()
    }
    #[test]
    fn sculpt_sphere_adds_and_removes_material() {
        let base = sphere();
        let added = base.sculpt_sphere([1.5, 0., 0.], 0.5, false).unwrap();
        assert!(matches!(added, Field::Union { .. }));
        assert_eq!(base.evaluate([1.5, 0., 0.]).unwrap(), 0.5);
        assert_eq!(added.evaluate([1.5, 0., 0.]).unwrap(), -0.5);
        assert_eq!(added.evaluate([0.; 3]).unwrap(), -1.);
        let removed = base.sculpt_sphere([0.; 3], 0.5, true).unwrap();
        assert!(matches!(removed, Field::Difference { .. }));
        assert_eq!(removed.evaluate([0.; 3]).unwrap(), 0.5);
        assert!(removed.evaluate([0.75, 0., 0.]).unwrap() < 0.);
        // Chained strokes compose and the result still polygonizes to a closed shell.
        let strokes = removed
            .sculpt_sphere([0., 1., 0.], 0.3, false)
            .unwrap()
            .sculpt_sphere([0., -1., 0.], 0.3, true)
            .unwrap();
        assert!(strokes.evaluate([0., 1., 0.]).unwrap() < 0.);
        assert!(strokes.evaluate([0., -1., 0.]).unwrap() > 0.);
        let mesh = polygonize(&strokes, &grid(16)).unwrap();
        assert!(triangles_closed(&mesh));
        assert!(triangles_ok(&mesh));
    }
    #[test]
    fn capsule_and_smooth_difference_sample_correctly() {
        let capsule = Field::Capsule {
            a: [0.; 3],
            b: [2., 0., 0.],
            radius: 0.5,
        };
        assert_eq!(capsule.evaluate([1., 0., 0.]).unwrap(), -0.5);
        assert_eq!(capsule.evaluate([3., 0., 0.]).unwrap(), 0.5);
        assert_eq!(capsule.evaluate([1., 1., 0.]).unwrap(), 0.5);
        let degenerate = Field::Capsule {
            a: [0.; 3],
            b: [0.; 3],
            radius: 1.,
        };
        assert_eq!(
            degenerate.evaluate([0., 2., 0.]).unwrap(),
            1.,
            "zero-length capsule is a sphere"
        );
        let hard = Field::Difference {
            a: Box::new(sphere()),
            b: Box::new(Field::Sphere {
                center: [1., 0., 0.],
                radius: 0.5,
            }),
        };
        let smooth = Field::SmoothDifference {
            a: Box::new(sphere()),
            b: Box::new(Field::Sphere {
                center: [1., 0., 0.],
                radius: 0.5,
            }),
            radius: 0.3,
        };
        // Far from the seam the smooth result equals the hard result; near it, material is removed more.
        assert!(
            (smooth.evaluate([-0.9, 0., 0.]).unwrap() - hard.evaluate([-0.9, 0., 0.]).unwrap())
                .abs()
                < 1e-12
        );
        assert!(
            (smooth.evaluate([1., 0., 0.]).unwrap() - hard.evaluate([1., 0., 0.]).unwrap()).abs()
                < 1e-12
        );
        // On the seam circle both distances vanish; the blend lifts the value by k/4.
        let seam = [0.875, (1. - 0.875f64 * 0.875).sqrt(), 0.];
        assert!(hard.evaluate(seam).unwrap().abs() < 1e-9);
        assert!((smooth.evaluate(seam).unwrap() - 0.3 / 4.).abs() < 1e-9);
        assert!(triangles_closed(&polygonize(&smooth, &grid(16)).unwrap()));
    }
    #[test]
    fn sculpt_strokes_compose_tools_and_blend_modes() {
        let base = sphere();
        let add = base
            .sculpt(&SdfStroke {
                tool: SdfTool::Box {
                    center: [1.2, 0., 0.],
                    half_size: [0.3; 3],
                },
                remove: false,
                blend: None,
            })
            .unwrap();
        assert!(matches!(add, Field::Union { .. }));
        assert!(add.evaluate([1.2, 0., 0.]).unwrap() < 0.);
        let smooth_add = base
            .sculpt(&SdfStroke {
                tool: SdfTool::Capsule {
                    a: [0., 0., 0.8],
                    b: [0., 0., 1.6],
                    radius: 0.2,
                },
                remove: false,
                blend: Some(0.2),
            })
            .unwrap();
        assert!(matches!(smooth_add, Field::SmoothUnion { .. }));
        assert!(smooth_add.evaluate([0., 0., 1.6]).unwrap() < 0.);
        let hard_remove = base
            .sculpt(&SdfStroke {
                tool: SdfTool::Sphere {
                    center: [0.; 3],
                    radius: 0.5,
                },
                remove: true,
                blend: None,
            })
            .unwrap();
        assert_eq!(hard_remove.evaluate([0.; 3]).unwrap(), 0.5);
        assert!(matches!(hard_remove, Field::Difference { .. }));
        let smooth_remove = base
            .sculpt(&SdfStroke {
                tool: SdfTool::Sphere {
                    center: [1., 0., 0.],
                    radius: 0.5,
                },
                remove: true,
                blend: Some(0.25),
            })
            .unwrap();
        assert!(matches!(smooth_remove, Field::SmoothDifference { .. }));
        assert!(smooth_remove.evaluate([0.6, 0., 0.]).unwrap() > 0.);
        let mesh = polygonize(&smooth_remove, &grid(16)).unwrap();
        assert!(triangles_closed(&mesh) && triangles_ok(&mesh));
        #[cfg(feature = "codec")]
        {
            let round: SdfStroke = value_codec::from_value(
                value_codec::to_value(SdfStroke {
                    tool: SdfTool::Capsule {
                        a: [0.; 3],
                        b: [1., 0., 0.],
                        radius: 0.2,
                    },
                    remove: true,
                    blend: Some(0.1),
                })
                .unwrap(),
            )
            .unwrap();
            assert_eq!(round.blend, Some(0.1));
            assert!(round.remove);
        }
        for bad in [
            SdfStroke {
                tool: SdfTool::Sphere {
                    center: [0.; 3],
                    radius: 0.,
                },
                remove: false,
                blend: None,
            },
            SdfStroke {
                tool: SdfTool::Box {
                    center: [0.; 3],
                    half_size: [1., -1., 1.],
                },
                remove: false,
                blend: None,
            },
            SdfStroke {
                tool: SdfTool::Capsule {
                    a: [f64::NAN; 3],
                    b: [0.; 3],
                    radius: 1.,
                },
                remove: false,
                blend: None,
            },
            SdfStroke {
                tool: SdfTool::Sphere {
                    center: [0.; 3],
                    radius: 1.,
                },
                remove: false,
                blend: Some(0.),
            },
        ] {
            assert!(base.sculpt(&bad).is_err());
        }
    }
    #[test]
    fn sculpt_sphere_rejects_invalid_strokes() {
        let base = sphere();
        for radius in [0., -1., f64::NAN, f64::INFINITY] {
            assert!(
                base.sculpt_sphere([0.; 3], radius, false).is_err(),
                "{radius}"
            );
        }
        assert!(base.sculpt_sphere([f64::NAN, 0., 0.], 1., true).is_err());
        let invalid = Field::Sphere {
            center: [0.; 3],
            radius: -1.,
        };
        assert!(invalid.sculpt_sphere([0.; 3], 1., false).is_err());
    }
    #[test]
    fn closed_outward_sphere_and_convergence() {
        let coarse = polygonize(&sphere(), &grid(8)).unwrap();
        let fine = polygonize(&sphere(), &grid(16)).unwrap();
        assert!(triangles_closed(&fine));
        assert!(triangles_ok(&fine));
        let exact = 4. * std::f64::consts::PI / 3.;
        assert!((signed_volume(&fine) - exact).abs() < (signed_volume(&coarse) - exact).abs());
        assert!((signed_volume(&fine) - exact).abs() < 0.1);
    }
    #[test]
    fn exact_grid_vertices_and_empty() {
        let mut g = grid(12);
        g.min = [-2.; 3];
        g.max = [2.; 3];
        assert!(triangles_closed(&polygonize(&sphere(), &g).unwrap()));
        assert!(polygonize_with(|_| 1., &g).unwrap().indices.is_empty());
        assert!(polygonize_with(|_| f64::NAN, &g).is_err());
        assert!(polygonize_with(|_| -1., &g).is_err());
    }
    #[test]
    fn tpms_chamfer_onion_elongate_and_surface_nets() {
        let neovius = Field::Neovius {
            origin: [0.; 3],
            period: 4.0,
            thickness: 0.5,
        };
        let lidinoid = Field::Lidinoid {
            origin: [0.; 3],
            period: 4.0,
            thickness: 0.5,
        };
        let iwp = Field::SchoenIWP {
            origin: [0.; 3],
            period: 4.0,
            thickness: 0.5,
        };
        for tpms in [&neovius, &lidinoid, &iwp] {
            assert!(tpms.validate().is_ok());
            assert!(tpms.evaluate([0.5, 0.7, 1.1]).unwrap().is_finite());
        }

        let onion_sphere = Field::Onion {
            input: Box::new(sphere()),
            thickness: 0.2,
        };
        // On the original sphere surface (r=1), onion field is -0.1 (inside shell); at origin it's +0.9 (outside shell).
        assert!((onion_sphere.evaluate([1., 0., 0.]).unwrap() + 0.1).abs() < 1e-12);
        assert!((onion_sphere.evaluate([0., 0., 0.]).unwrap() - 0.9).abs() < 1e-12);

        let elongated = Field::Elongate {
            input: Box::new(sphere()),
            half_extents: [1.0, 0.0, 0.0],
        };
        // Elongating a unit sphere by 1.0 along +-X produces a capsule of total length 4 along X.
        assert!((elongated.evaluate([2.0, 0., 0.]).unwrap()).abs() < 1e-12);
        assert!(elongated.evaluate([1.5, 0., 0.]).unwrap() < 0.);

        let chamfer = Field::ChamferUnion {
            a: Box::new(sphere()),
            b: Box::new(Field::Box {
                center: [0.8, 0., 0.],
                half_size: [0.6; 3],
            }),
            radius: 0.2,
        };
        assert!(chamfer.validate().is_ok());

        let sn_mesh = polygonize_surface_nets(&chamfer, &grid(16)).unwrap();
        assert!(triangles_closed(&sn_mesh));
        assert!(triangles_ok(&sn_mesh));
    }

    #[test]
    fn octahedron_prisms_mirror_taper_raycast_and_volume() {
        let oct = Field::Octahedron {
            center: [0.; 3],
            size: 1.0,
        };
        assert!(oct.validate().is_ok());
        assert!(oct.evaluate([0.; 3]).unwrap() < 0.0);
        assert!((oct.evaluate([1.0, 0.0, 0.0]).unwrap()).abs() < 1e-12);

        let hex = Field::HexPrism {
            center: [0.; 3],
            radius: 1.0,
            half_height: 0.8,
        };
        assert!(hex.validate().is_ok());
        assert!(hex.evaluate([0.; 3]).unwrap() < 0.0);

        let tri_prism = Field::TriangularPrism {
            center: [0.; 3],
            radius: 1.0,
            half_height: 0.8,
        };
        assert!(tri_prism.validate().is_ok());
        assert!(tri_prism.evaluate([0.; 3]).unwrap() < 0.0);

        let s_inter = Field::SmoothIntersection {
            a: Box::new(sphere()),
            b: Box::new(Field::Box {
                center: [0.; 3],
                half_size: [0.8; 3],
            }),
            radius: 0.2,
        };
        assert!(s_inter.validate().is_ok());
        assert!(s_inter.evaluate([0.; 3]).unwrap() < 0.0);

        let mirrored = Field::Mirror {
            input: Box::new(Field::Sphere {
                center: [1.5, 0.0, 0.0],
                radius: 0.5,
            }),
            axis: [true, false, false],
        };
        assert!((mirrored.evaluate([-1.5, 0.0, 0.0]).unwrap() + 0.5).abs() < 1e-12);

        let tapered = Field::Taper {
            input: Box::new(Field::Cylinder {
                center: [0.; 3],
                radius: 1.0,
                half_height: 1.0,
            }),
            height: 2.0,
            factor: 0.5,
        };
        assert!(tapered.validate().is_ok());
        assert!(tapered.evaluate([0.0, 0.0, 0.8]).unwrap() < 0.0);
        assert!(tapered.evaluate([0.8, 0.0, 0.8]).unwrap() > 0.0);

        // Raycast from (-3, 0, 0) along +X against unit sphere at origin
        let (hit_p, hit_n, dist) = sphere()
            .raycast([-3.0, 0.0, 0.0], [1.0, 0.0, 0.0], 10.0, 1e-5)
            .unwrap();
        assert!((dist - 2.0).abs() < 1e-4);
        assert!((hit_p[0] + 1.0).abs() < 1e-4);
        assert!((hit_n[0] + 1.0).abs() < 1e-3);

        // Volume estimation of unit sphere (~4/3 * pi = 4.18879)
        let vol = sphere().estimate_volume(32);
        let exact = 4.0 * std::f64::consts::PI / 3.0;
        assert!((vol - exact).abs() < 0.15, "estimated {vol} vs exact {exact}");

        // Mean curvature on a sphere of radius R=2.0 is H = 1/R = 0.5
        let r2_sphere = Field::Sphere {
            center: [0.; 3],
            radius: 2.0,
        };
        let h_curv = r2_sphere.mean_curvature([2.0, 0.0, 0.0], 1e-3);
        assert!((h_curv - 0.5).abs() < 1e-3, "mean curvature got {h_curv}");

        // Ambient occlusion: unoccluded sphere surface -> 1.0; occluded by an overhanging box -> < 0.8
        let ao_open = r2_sphere.ambient_occlusion([2.0, 0.0, 0.0], [1.0, 0.0, 0.0], 5, 0.2);
        assert!((ao_open - 1.0).abs() < 1e-6, "expected 1.0, got {ao_open}");
        let occluded_scene = Field::Union {
            a: Box::new(r2_sphere),
            b: Box::new(Field::Box {
                center: [2.8, 0.0, 0.0],
                half_size: [0.5, 2.0, 2.0],
            }),
        };
        let ao_occluded = occluded_scene.ambient_occlusion([2.0, 0.0, 0.0], [1.0, 0.0, 0.0], 5, 0.2);
        assert!(ao_occluded < 0.6, "expected occluded AO < 0.6, got {ao_occluded}");
    }
}

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

    /// Estimates ambient occlusion in $[0, 1]$ at surface point `p` along outward `normal`
    /// using an $N$-step exponentially weighted SDF cone/normal march ($k = 1 \dots \text{steps}$
    /// at distances $d_k = k \cdot \text{step\_dist}$). Returns $1.0$ for unoccluded convex/flat
    /// surfaces and approaches $0.0$ inside deep cavities or crevices.
    pub fn ambient_occlusion(&self, p: Point, normal: Point, steps: usize, step_dist: f64) -> f64 {
        if !finite(p) || !finite(normal) || !step_dist.is_finite() || step_dist <= 0.0 || steps == 0 {
            return 1.0;
        }
        let n_len = length(normal);
        if n_len <= 1e-12 {
            return 1.0;
        }
        let n = [normal[0] / n_len, normal[1] / n_len, normal[2] / n_len];
        let count = steps.min(32);
        let mut occlusion = 0.0;
        let mut weight = 0.5;
        let mut norm_sum = 0.0;
        for i in 1..=count {
            let expected_dist = (i as f64) * step_dist;
            let q = [
                p[0] + n[0] * expected_dist,
                p[1] + n[1] * expected_dist,
                p[2] + n[2] * expected_dist,
            ];
            let actual_dist = self.sample(q);
            if actual_dist.is_finite() {
                let deficit = (expected_dist - actual_dist).max(0.0) / expected_dist;
                occlusion += weight * deficit.clamp(0.0, 1.0);
            }
            norm_sum += weight;
            weight *= 0.5;
        }
        if norm_sum > 0.0 {
            (1.0 - occlusion / norm_sum).clamp(0.0, 1.0)
        } else {
            1.0
        }
    }

    /// Computes the mean curvature $H = \frac{1}{2} \nabla \cdot \left(\frac{\nabla f}{\|\nabla f\|}\right)$
    /// at point `p` via central differences of the normalized SDF gradient with step `eps`.
    /// For a sphere of radius $R$, $H = 1 / R$ on the surface.
    pub fn mean_curvature(&self, p: Point, eps: f64) -> f64 {
        if !finite(p) || !eps.is_finite() {
            return 0.0;
        }
        let h = eps.abs().max(1e-5);
        let g_eps = h * 0.5;
        let nx_pos = self.gradient([p[0] + h, p[1], p[2]], g_eps)[0];
        let nx_neg = self.gradient([p[0] - h, p[1], p[2]], g_eps)[0];
        let ny_pos = self.gradient([p[0], p[1] + h, p[2]], g_eps)[1];
        let ny_neg = self.gradient([p[0], p[1] - h, p[2]], g_eps)[1];
        let nz_pos = self.gradient([p[0], p[1], p[2] + h], g_eps)[2];
        let nz_neg = self.gradient([p[0], p[1], p[2] - h], g_eps)[2];
        let div = ((nx_pos - nx_neg) + (ny_pos - ny_neg) + (nz_pos - nz_neg)) / (2.0 * h);
        0.5 * div
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

/// Dual Surface Nets / Dual Contouring extraction with gradient-guided QEF vertex projection.
/// Places one dual vertex per active grid cell at the mass point of edge crossings and projects
/// it onto the tangent planes defined by the SDF gradients $\nabla f$, then emits two outward
/// triangles per sign-changing interior grid edge.
pub fn polygonize_surface_nets(field: &Field, grid: &Grid) -> Result<Triangles> {
    check_grid_budget(field, grid)?;
    if !finite(grid.min)
        || !finite(grid.max)
        || grid.cells.iter().any(|&n| n == 0 || n > 64)
        || (0..3).any(|i| grid.max[i] <= grid.min[i])
    {
        return Err(error(
            "Invalid grid: each axis requires 1..64 cells and ordered finite bounds",
        ));
    }
    let [nx, ny, nz] = grid.cells;
    let index = |x: usize, y: usize, z: usize| (z * (ny + 1) + y) * (nx + 1) + x;
    let sample_count = (nx + 1) * (ny + 1) * (nz + 1);
    let mut points = Vec::with_capacity(sample_count);
    let mut values = Vec::with_capacity(sample_count);
    let snap = 32.0
        * f64::EPSILON
        * (0..3)
            .map(|i| grid.max[i] - grid.min[i])
            .fold(0.0, f64::max);
    for z in 0..=nz {
        for y in 0..=ny {
            for x in 0..=nx {
                let p = std::array::from_fn(|i| {
                    grid.min[i]
                        + (grid.max[i] - grid.min[i]) * [x, y, z][i] as f64 / grid.cells[i] as f64
                });
                let v = field.sample(p);
                let v = if v.abs() <= snap { 0.0 } else { v };
                if !v.is_finite() {
                    return Err(error("Field returned a non-finite value"));
                }
                if (x == 0 || y == 0 || z == 0 || x == nx || y == ny || z == nz) && v <= 0.0 {
                    return Err(error("Surface touches grid boundary; enlarge bounds"));
                }
                points.push(p);
                values.push(v);
            }
        }
    }

    let cell_idx = |cx: usize, cy: usize, cz: usize| (cz * ny + cy) * nx + cx;
    let mut cell_vert = vec![usize::MAX; nx * ny * nz];
    let cell_estimate = (nx * ny + ny * nz + nz * nx) * 2;
    let mut positions = Vec::with_capacity(cell_estimate * 3);
    let step = [
        (grid.max[0] - grid.min[0]) / nx as f64,
        (grid.max[1] - grid.min[1]) / ny as f64,
        (grid.max[2] - grid.min[2]) / nz as f64,
    ];
    let grad_eps = step[0].min(step[1]).min(step[2]) * 0.05;

    const CUBE_EDGES: [[usize; 2]; 12] = [
        [0, 1],
        [1, 2],
        [2, 3],
        [3, 0],
        [4, 5],
        [5, 6],
        [6, 7],
        [7, 4],
        [0, 4],
        [1, 5],
        [2, 6],
        [3, 7],
    ];

    for cz in 0..nz {
        for cy in 0..ny {
            for cx in 0..nx {
                let c = [
                    index(cx, cy, cz),
                    index(cx + 1, cy, cz),
                    index(cx + 1, cy + 1, cz),
                    index(cx, cy + 1, cz),
                    index(cx, cy, cz + 1),
                    index(cx + 1, cy, cz + 1),
                    index(cx + 1, cy + 1, cz + 1),
                    index(cx, cy + 1, cz + 1),
                ];
                let mut crossings_buf = [[0.0; 3]; 12];
                let mut cross_len = 0usize;
                for &[ea, eb] in &CUBE_EDGES {
                    let va = values[c[ea]];
                    let vb = values[c[eb]];
                    if (va < 0.0) != (vb < 0.0) {
                        let t = (va / (va - vb)).clamp(0.0, 1.0);
                        let pa = points[c[ea]];
                        let pb = points[c[eb]];
                        crossings_buf[cross_len] = [
                            pa[0] + t * (pb[0] - pa[0]),
                            pa[1] + t * (pb[1] - pa[1]),
                            pa[2] + t * (pb[2] - pa[2]),
                        ];
                        cross_len += 1;
                    }
                }
                if cross_len == 0 {
                    continue;
                }
                let crossings = &crossings_buf[..cross_len];
                let inv_len = 1.0 / cross_len as f64;
                let mut v_pos = [0.0; 3];
                for p in crossings {
                    for k in 0..3 {
                        v_pos[k] += p[k] * inv_len;
                    }
                }
                // Gradient-descent / QEF tangent-plane projection clamped strictly inside cell bounds
                let c_min = points[c[0]];
                let c_max = points[c[6]];
                for _ in 0..2 {
                    let mut disp = [0.0; 3];
                    for p in crossings {
                        let n = field.gradient(*p, grad_eps);
                        let d = dot(sub(*p, v_pos), n);
                        for k in 0..3 {
                            disp[k] += d * n[k] * inv_len;
                        }
                    }
                    for k in 0..3 {
                        let margin = step[k] * 0.05;
                        v_pos[k] = (v_pos[k] + 0.5 * disp[k])
                            .clamp(c_min[k] + margin, c_max[k] - margin);
                    }
                }
                let vid = positions.len() / 3;
                positions.extend_from_slice(&v_pos);
                cell_vert[cell_idx(cx, cy, cz)] = vid;
            }
        }
    }

    let mut indices = Vec::with_capacity(positions.len() * 2);
    let mut emit_quad = |v0: usize, v1: usize, v2: usize, v3: usize, flip: bool| {
        if flip {
            indices.extend_from_slice(&[v0, v2, v1, v0, v3, v2]);
        } else {
            indices.extend_from_slice(&[v0, v1, v2, v0, v2, v3]);
        }
    };

    // X-parallel edges: shared by cells (x, y-1, z-1), (x, y, z-1), (x, y, z), (x, y-1, z)
    for z in 1..nz {
        for y in 1..ny {
            for x in 0..nx {
                let va = values[index(x, y, z)];
                let vb = values[index(x + 1, y, z)];
                if (va < 0.0) != (vb < 0.0) {
                    let c0 = cell_vert[cell_idx(x, y - 1, z - 1)];
                    let c1 = cell_vert[cell_idx(x, y, z - 1)];
                    let c2 = cell_vert[cell_idx(x, y, z)];
                    let c3 = cell_vert[cell_idx(x, y - 1, z)];
                    emit_quad(c0, c1, c2, c3, va >= 0.0);
                }
            }
        }
    }
    // Y-parallel edges: shared by cells (x-1, y, z-1), (x-1, y, z), (x, y, z), (x, y, z-1)
    for z in 1..nz {
        for y in 0..ny {
            for x in 1..nx {
                let va = values[index(x, y, z)];
                let vb = values[index(x, y + 1, z)];
                if (va < 0.0) != (vb < 0.0) {
                    let c0 = cell_vert[cell_idx(x - 1, y, z - 1)];
                    let c1 = cell_vert[cell_idx(x - 1, y, z)];
                    let c2 = cell_vert[cell_idx(x, y, z)];
                    let c3 = cell_vert[cell_idx(x, y, z - 1)];
                    emit_quad(c0, c1, c2, c3, va >= 0.0);
                }
            }
        }
    }
    // Z-parallel edges: shared by cells (x-1, y-1, z), (x, y-1, z), (x, y, z), (x-1, y, z)
    for z in 0..nz {
        for y in 1..ny {
            for x in 1..nx {
                let va = values[index(x, y, z)];
                let vb = values[index(x, y, z + 1)];
                if (va < 0.0) != (vb < 0.0) {
                    let c0 = cell_vert[cell_idx(x - 1, y - 1, z)];
                    let c1 = cell_vert[cell_idx(x, y - 1, z)];
                    let c2 = cell_vert[cell_idx(x, y, z)];
                    let c3 = cell_vert[cell_idx(x - 1, y, z)];
                    emit_quad(c0, c1, c2, c3, va >= 0.0);
                }
            }
        }
    }

    finish_triangles(Triangles { positions, indices })
}
