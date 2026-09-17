//! Negative-inside implicit fields and bounded marching-tetrahedra extraction.
//! CSG fields preserve the zero set but are not generally exact signed distances.
//! Extraction yields a neutral triangle buffer; mesh inspect lives in the bridge.
#![feature(
    try_blocks,
    gen_blocks,
    yield_expr,
    super_let,
    deref_patterns,
    yeet_expr
)]
#![allow(unused_features)]
use geometry_ops::Triangles;
pub use math_core::{Acceleration, Error, Result, V3, finite};
use planar_geometry::rings::{self, Rings};
use std::collections::BTreeMap;
const INVALID_INPUT: &str = "SDF_INVALID_INPUT";
fn error(message: impl Into<String>) -> Error {
    Error::new(INVALID_INPUT, message)
}
#[cfg(feature = "cuda")]
pub mod cuda;
pub mod flat;
#[cfg(feature = "gpu")]
mod gpu;

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

// Flat postorder field tree: leaves push, CSG/offset ops combine with a value
// stack. Node kinds match sdf-core/src/flat.rs.
@compute @workgroup_size(256)
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
        if (kind <= 2u) {
            // Leaves: sphere / box / torus with translate-folded parameters.
            var v = 0.0;
            if (kind == 0u) {
                let c = vec3f(node_params[base], node_params[base + 1u], node_params[base + 2u]);
                v = length(p - c) - node_params[base + 3u];
            } else if (kind == 1u) {
                let c = vec3f(node_params[base], node_params[base + 1u], node_params[base + 2u]);
                let h = vec3f(node_params[base + 3u], node_params[base + 4u], node_params[base + 5u]);
                let q = abs(p - c) - h;
                v = length(max(q, vec3f(0.0))) + min(max(q.x, max(q.y, q.z)), 0.0);
            } else {
                let c = vec3f(node_params[base], node_params[base + 1u], node_params[base + 2u]);
                let q = p - c;
                v = length(vec2f(length(q.xy) - node_params[base + 3u], q.z)) - node_params[base + 4u];
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
    Offset {
        input: Box<Field>,
        distance: f64,
    },
    Translate {
        input: Box<Field>,
        vector: Point,
    },
}
fn profile_to_value(rings: &Rings) -> value_codec::Value {
    let mut object = value_codec::Map::new();
    let (outer, holes) = match rings.split_first() {
        Some((outer, holes)) => (outer.clone(), holes.to_vec()),
        None => (Vec::new(), Vec::new()),
    };
    object.insert("outer".into(), value_codec::Serialize::to_value(&outer));
    object.insert("holes".into(), value_codec::Serialize::to_value(&holes));
    value_codec::Value::Object(object)
}
fn profile_from_value(value: value_codec::Value) -> value_codec::Result<Rings> {
    let mut object = value
        .as_object()
        .ok_or_else(|| value_codec::error("Expected object"))?
        .clone();
    let outer: Vec<[f64; 2]> = value_codec::Deserialize::from_value(
        object
            .remove("outer")
            .ok_or_else(|| value_codec::error("Missing field outer"))?,
    )?;
    let holes: Vec<Vec<[f64; 2]>> = if let Some(v) = object.remove("holes") {
        value_codec::Deserialize::from_value(v)?
    } else {
        Default::default()
    };
    Ok(rings::from_outer_holes(outer, holes))
}
impl value_codec::Serialize for Field {
    fn to_value(&self) -> value_codec::Value {
        match self {
            Self::Extrude {
                profile,
                half_height,
            } => {
                let mut object = value_codec::Map::new();
                object.insert("profile".into(), profile_to_value(profile));
                object.insert(
                    "half_height".into(),
                    value_codec::Serialize::to_value(half_height),
                );
                object.insert("kind".into(), value_codec::Value::String("extrude".into()));
                value_codec::Value::Object(object)
            }
            Self::Revolve { profile } => {
                let mut object = value_codec::Map::new();
                object.insert("profile".into(), profile_to_value(profile));
                object.insert("kind".into(), value_codec::Value::String("revolve".into()));
                value_codec::Value::Object(object)
            }
            Self::Deform { input, deformation } => {
                let mut object = value_codec::Map::new();
                object.insert("input".into(), value_codec::Serialize::to_value(input));
                object.insert(
                    "deformation".into(),
                    value_codec::Serialize::to_value(deformation),
                );
                object.insert("kind".into(), value_codec::Value::String("deform".into()));
                value_codec::Value::Object(object)
            }
            Self::MeshDistance { mesh, signed } => {
                let mut object = value_codec::Map::new();
                object.insert("mesh".into(), value_codec::Serialize::to_value(mesh));
                object.insert("signed".into(), value_codec::Serialize::to_value(signed));
                object.insert(
                    "kind".into(),
                    value_codec::Value::String("mesh_distance".into()),
                );
                value_codec::Value::Object(object)
            }
            Self::Sphere { center, radius } => {
                let mut object = value_codec::Map::new();
                object.insert("center".into(), value_codec::Serialize::to_value(center));
                object.insert("radius".into(), value_codec::Serialize::to_value(radius));
                object.insert("kind".into(), value_codec::Value::String("sphere".into()));
                value_codec::Value::Object(object)
            }
            Self::Box { center, half_size } => {
                let mut object = value_codec::Map::new();
                object.insert("center".into(), value_codec::Serialize::to_value(center));
                object.insert(
                    "half_size".into(),
                    value_codec::Serialize::to_value(half_size),
                );
                object.insert("kind".into(), value_codec::Value::String("box".into()));
                value_codec::Value::Object(object)
            }
            Self::Torus {
                center,
                major_radius,
                minor_radius,
            } => {
                let mut object = value_codec::Map::new();
                object.insert("center".into(), value_codec::Serialize::to_value(center));
                object.insert(
                    "major_radius".into(),
                    value_codec::Serialize::to_value(major_radius),
                );
                object.insert(
                    "minor_radius".into(),
                    value_codec::Serialize::to_value(minor_radius),
                );
                object.insert("kind".into(), value_codec::Value::String("torus".into()));
                value_codec::Value::Object(object)
            }
            Self::Union { a, b } => {
                let mut object = value_codec::Map::new();
                object.insert("a".into(), value_codec::Serialize::to_value(a));
                object.insert("b".into(), value_codec::Serialize::to_value(b));
                object.insert("kind".into(), value_codec::Value::String("union".into()));
                value_codec::Value::Object(object)
            }
            Self::Intersection { a, b } => {
                let mut object = value_codec::Map::new();
                object.insert("a".into(), value_codec::Serialize::to_value(a));
                object.insert("b".into(), value_codec::Serialize::to_value(b));
                object.insert(
                    "kind".into(),
                    value_codec::Value::String("intersection".into()),
                );
                value_codec::Value::Object(object)
            }
            Self::Difference { a, b } => {
                let mut object = value_codec::Map::new();
                object.insert("a".into(), value_codec::Serialize::to_value(a));
                object.insert("b".into(), value_codec::Serialize::to_value(b));
                object.insert(
                    "kind".into(),
                    value_codec::Value::String("difference".into()),
                );
                value_codec::Value::Object(object)
            }
            Self::SmoothUnion { a, b, radius } => {
                let mut object = value_codec::Map::new();
                object.insert("a".into(), value_codec::Serialize::to_value(a));
                object.insert("b".into(), value_codec::Serialize::to_value(b));
                object.insert("radius".into(), value_codec::Serialize::to_value(radius));
                object.insert(
                    "kind".into(),
                    value_codec::Value::String("smooth_union".into()),
                );
                value_codec::Value::Object(object)
            }
            Self::Offset { input, distance } => {
                let mut object = value_codec::Map::new();
                object.insert("input".into(), value_codec::Serialize::to_value(input));
                object.insert(
                    "distance".into(),
                    value_codec::Serialize::to_value(distance),
                );
                object.insert("kind".into(), value_codec::Value::String("offset".into()));
                value_codec::Value::Object(object)
            }
            Self::Translate { input, vector } => {
                let mut object = value_codec::Map::new();
                object.insert("input".into(), value_codec::Serialize::to_value(input));
                object.insert("vector".into(), value_codec::Serialize::to_value(vector));
                object.insert(
                    "kind".into(),
                    value_codec::Value::String("translate".into()),
                );
                value_codec::Value::Object(object)
            }
        }
    }
}
impl<'de> value_codec::Deserialize<'de> for Field {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        match value["kind"].as_str().unwrap_or("") {
            "extrude" => {
                let mut object = value
                    .as_object()
                    .ok_or_else(|| value_codec::error("Expected object"))?
                    .clone();
                let profile = profile_from_value(
                    object
                        .remove("profile")
                        .ok_or_else(|| value_codec::error("Missing field profile"))?,
                )?;
                let half_height: f64 = value_codec::Deserialize::from_value(
                    object
                        .remove("half_height")
                        .ok_or_else(|| value_codec::error("Missing field half_height"))?,
                )?;
                Ok(Self::Extrude {
                    profile,
                    half_height,
                })
            }
            "revolve" => {
                let mut object = value
                    .as_object()
                    .ok_or_else(|| value_codec::error("Expected object"))?
                    .clone();
                let profile = profile_from_value(
                    object
                        .remove("profile")
                        .ok_or_else(|| value_codec::error("Missing field profile"))?,
                )?;
                Ok(Self::Revolve { profile })
            }
            "deform" => {
                let mut object = value
                    .as_object()
                    .ok_or_else(|| value_codec::error("Expected object"))?
                    .clone();
                let input: Box<Field> = value_codec::Deserialize::from_value(
                    object
                        .remove("input")
                        .ok_or_else(|| value_codec::error("Missing field input"))?,
                )?;
                let deformation: geometry_ops::Deformation = value_codec::Deserialize::from_value(
                    object
                        .remove("deformation")
                        .ok_or_else(|| value_codec::error("Missing field deformation"))?,
                )?;
                Ok(Self::Deform { input, deformation })
            }
            "mesh_distance" => {
                let mut object = value
                    .as_object()
                    .ok_or_else(|| value_codec::error("Expected object"))?
                    .clone();
                let mesh: Triangles = value_codec::Deserialize::from_value(
                    object
                        .remove("mesh")
                        .ok_or_else(|| value_codec::error("Missing field mesh"))?,
                )?;
                let signed: bool = value_codec::Deserialize::from_value(
                    object
                        .remove("signed")
                        .ok_or_else(|| value_codec::error("Missing field signed"))?,
                )?;
                Ok(Self::MeshDistance { mesh, signed })
            }
            "sphere" => {
                let mut object = value
                    .as_object()
                    .ok_or_else(|| value_codec::error("Expected object"))?
                    .clone();
                let center: Point = value_codec::Deserialize::from_value(
                    object
                        .remove("center")
                        .ok_or_else(|| value_codec::error("Missing field center"))?,
                )?;
                let radius: f64 = value_codec::Deserialize::from_value(
                    object
                        .remove("radius")
                        .ok_or_else(|| value_codec::error("Missing field radius"))?,
                )?;
                Ok(Self::Sphere { center, radius })
            }
            "box" => {
                let mut object = value
                    .as_object()
                    .ok_or_else(|| value_codec::error("Expected object"))?
                    .clone();
                let center: Point = value_codec::Deserialize::from_value(
                    object
                        .remove("center")
                        .ok_or_else(|| value_codec::error("Missing field center"))?,
                )?;
                let half_size: Point = value_codec::Deserialize::from_value(
                    object
                        .remove("half_size")
                        .ok_or_else(|| value_codec::error("Missing field half_size"))?,
                )?;
                Ok(Self::Box { center, half_size })
            }
            "torus" => {
                let mut object = value
                    .as_object()
                    .ok_or_else(|| value_codec::error("Expected object"))?
                    .clone();
                let center: Point = value_codec::Deserialize::from_value(
                    object
                        .remove("center")
                        .ok_or_else(|| value_codec::error("Missing field center"))?,
                )?;
                let major_radius: f64 = value_codec::Deserialize::from_value(
                    object
                        .remove("major_radius")
                        .ok_or_else(|| value_codec::error("Missing field major_radius"))?,
                )?;
                let minor_radius: f64 = value_codec::Deserialize::from_value(
                    object
                        .remove("minor_radius")
                        .ok_or_else(|| value_codec::error("Missing field minor_radius"))?,
                )?;
                Ok(Self::Torus {
                    center,
                    major_radius,
                    minor_radius,
                })
            }
            "union" => {
                let mut object = value
                    .as_object()
                    .ok_or_else(|| value_codec::error("Expected object"))?
                    .clone();
                let a: Box<Field> = value_codec::Deserialize::from_value(
                    object
                        .remove("a")
                        .ok_or_else(|| value_codec::error("Missing field a"))?,
                )?;
                let b: Box<Field> = value_codec::Deserialize::from_value(
                    object
                        .remove("b")
                        .ok_or_else(|| value_codec::error("Missing field b"))?,
                )?;
                Ok(Self::Union { a, b })
            }
            "intersection" => {
                let mut object = value
                    .as_object()
                    .ok_or_else(|| value_codec::error("Expected object"))?
                    .clone();
                let a: Box<Field> = value_codec::Deserialize::from_value(
                    object
                        .remove("a")
                        .ok_or_else(|| value_codec::error("Missing field a"))?,
                )?;
                let b: Box<Field> = value_codec::Deserialize::from_value(
                    object
                        .remove("b")
                        .ok_or_else(|| value_codec::error("Missing field b"))?,
                )?;
                Ok(Self::Intersection { a, b })
            }
            "difference" => {
                let mut object = value
                    .as_object()
                    .ok_or_else(|| value_codec::error("Expected object"))?
                    .clone();
                let a: Box<Field> = value_codec::Deserialize::from_value(
                    object
                        .remove("a")
                        .ok_or_else(|| value_codec::error("Missing field a"))?,
                )?;
                let b: Box<Field> = value_codec::Deserialize::from_value(
                    object
                        .remove("b")
                        .ok_or_else(|| value_codec::error("Missing field b"))?,
                )?;
                Ok(Self::Difference { a, b })
            }
            "smooth_union" => {
                let mut object = value
                    .as_object()
                    .ok_or_else(|| value_codec::error("Expected object"))?
                    .clone();
                let a: Box<Field> = value_codec::Deserialize::from_value(
                    object
                        .remove("a")
                        .ok_or_else(|| value_codec::error("Missing field a"))?,
                )?;
                let b: Box<Field> = value_codec::Deserialize::from_value(
                    object
                        .remove("b")
                        .ok_or_else(|| value_codec::error("Missing field b"))?,
                )?;
                let radius: f64 = value_codec::Deserialize::from_value(
                    object
                        .remove("radius")
                        .ok_or_else(|| value_codec::error("Missing field radius"))?,
                )?;
                Ok(Self::SmoothUnion { a, b, radius })
            }
            "offset" => {
                let mut object = value
                    .as_object()
                    .ok_or_else(|| value_codec::error("Expected object"))?
                    .clone();
                let input: Box<Field> = value_codec::Deserialize::from_value(
                    object
                        .remove("input")
                        .ok_or_else(|| value_codec::error("Missing field input"))?,
                )?;
                let distance: f64 = value_codec::Deserialize::from_value(
                    object
                        .remove("distance")
                        .ok_or_else(|| value_codec::error("Missing field distance"))?,
                )?;
                Ok(Self::Offset { input, distance })
            }
            "translate" => {
                let mut object = value
                    .as_object()
                    .ok_or_else(|| value_codec::error("Expected object"))?
                    .clone();
                let input: Box<Field> = value_codec::Deserialize::from_value(
                    object
                        .remove("input")
                        .ok_or_else(|| value_codec::error("Missing field input"))?,
                )?;
                let vector: Point = value_codec::Deserialize::from_value(
                    object
                        .remove("vector")
                        .ok_or_else(|| value_codec::error("Missing field vector"))?,
                )?;
                Ok(Self::Translate { input, vector })
            }
            _ => Err(value_codec::error("Unknown enum variant")),
        }
    }
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
    let mut edges = BTreeMap::new();
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
                Field::Union { a, b }
                | Field::Intersection { a, b }
                | Field::Difference { a, b } => {
                    walk(a, depth + 1, budget) && walk(b, depth + 1, budget)
                }
                Field::SmoothUnion { a, b, radius } => {
                    positive(*radius) && walk(a, depth + 1, budget) && walk(b, depth + 1, budget)
                }
                Field::Offset { input, distance } => {
                    distance.is_finite() && distance.abs() <= 1e6 && walk(input, depth + 1, budget)
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
            Self::Union { a, b } => a.sample(p).min(b.sample(p)),
            Self::Intersection { a, b } => a.sample(p).max(b.sample(p)),
            Self::Difference { a, b } => a.sample(p).max(-b.sample(p)),
            Self::SmoothUnion { a, b, radius: k } => {
                let a = a.sample(p);
                let b = b.sample(p);
                let h = (0.5 + 0.5 * (b - a) / k).clamp(0., 1.);
                b * (1. - h) + a * h - k * h * (1. - h)
            }
            Self::Offset { input, distance } => input.sample(p) - distance,
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
            | Self::SmoothUnion { a, b, .. } => a.sample_work() + b.sample_work(),
            Self::Deform { input, .. }
            | Self::Offset { input, .. }
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
impl value_codec::Serialize for Grid {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert("min".into(), value_codec::Serialize::to_value(&self.min));
        object.insert("max".into(), value_codec::Serialize::to_value(&self.max));
        object.insert(
            "cells".into(),
            value_codec::Serialize::to_value(&self.cells),
        );
        value_codec::Value::Object(object)
    }
}
impl<'de> value_codec::Deserialize<'de> for Grid {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let mut object = value
            .as_object()
            .ok_or_else(|| value_codec::error("Expected object"))?
            .clone();
        let min: Point = value_codec::Deserialize::from_value(
            object
                .remove("min")
                .ok_or_else(|| value_codec::error("Missing field min"))?,
        )?;
        let max: Point = value_codec::Deserialize::from_value(
            object
                .remove("max")
                .ok_or_else(|| value_codec::error("Missing field max"))?,
        )?;
        let cells: [usize; 3] = value_codec::Deserialize::from_value(
            object
                .remove("cells")
                .ok_or_else(|| value_codec::error("Missing field cells"))?,
        )?;
        Ok(Self { min, max, cells })
    }
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
    let mut points = Vec::new();
    let mut values = Vec::new();
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
    let mut mesh = Triangles {
        positions: Vec::new(),
        indices: Vec::new(),
    };
    let mut cache = BTreeMap::new();
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
                    let mut polygon = Vec::new();
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
                        if !polygon.contains(&id) {
                            polygon.push(id);
                        }
                    }
                    if polygon.len() < 3 {
                        continue;
                    }
                    let point = |i: usize| {
                        [
                            mesh.positions[i * 3],
                            mesh.positions[i * 3 + 1],
                            mesh.positions[i * 3 + 2],
                        ]
                    };
                    let mut center = [0.; 3];
                    for &i in &polygon {
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
/// `polygonize` with an optional GPU grid sampler. Eligible fields (primitive
/// and CSG trees) sample the grid on the GPU in f32; the snap-to-zero, boundary
/// validation and marching-tetrahedra extraction stay on the CPU. Everything
/// else — and any failure — falls back to the CPU reference.
///
/// `Acceleration::Cuda` runs the PTX port through the CUDA driver (feature
/// `cuda`), then the wgpu shader (feature `gpu`), then the CPU reference.
pub fn polygonize_accelerated(
    field: &Field,
    grid: &Grid,
    #[allow(unused_variables)] acceleration: Acceleration,
) -> Result<Triangles> {
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
    pub fn sculpt_sphere(&self, center: Point, radius: f64, remove: bool) -> Result<Self> {
        self.validate()?;
        let sphere = Self::Sphere { center, radius };
        sphere.validate()?;
        let out = if remove {
            Self::Difference {
                a: Box::new(self.clone()),
                b: Box::new(sphere),
            }
        } else {
            Self::Union {
                a: Box::new(self.clone()),
                b: Box::new(sphere),
            }
        };
        out.validate()?;
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
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

    use super::*;
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
}
