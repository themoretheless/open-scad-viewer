//! Sampled inward shell for closed triangle meshes. BVHs keep grid queries bounded.
use polygon_core::{BuiltMesh, Error, Mesh, Result, solid::proximity::closest_triangle};
fn fail(message: impl Into<String>) -> Error {
    Error::new("GEOMETRY_INVALID_INPUT", message)
}
pub(crate) type P = [f64; 3];
use crate::Acceleration;
use math_core::{cross, dot, sub};
#[derive(Clone)]
pub(crate) struct Triangle {
    pub(crate) p: [P; 3],
    pub(crate) min: P,
    pub(crate) max: P,
}
pub(crate) struct Node {
    pub(crate) min: P,
    pub(crate) max: P,
    pub(crate) triangles: Vec<Triangle>,
    pub(crate) children: Option<Box<[Node; 2]>>,
}
impl Node {
    fn build(mut triangles: Vec<Triangle>) -> Self {
        let min = std::array::from_fn(|k| {
            triangles
                .iter()
                .map(|t| t.min[k])
                .fold(f64::INFINITY, f64::min)
        });
        let max = std::array::from_fn(|k| {
            triangles
                .iter()
                .map(|t| t.max[k])
                .fold(f64::NEG_INFINITY, f64::max)
        });
        if triangles.len() <= 8 {
            return Self {
                min,
                max,
                triangles,
                children: None,
            };
        }
        let axis = (0..3)
            .max_by(|&a, &b| (max[a] - min[a]).total_cmp(&(max[b] - min[b])))
            .unwrap();
        triangles
            .sort_by(|a, b| (a.min[axis] + a.max[axis]).total_cmp(&(b.min[axis] + b.max[axis])));
        let right = triangles.split_off(triangles.len() / 2);
        Self {
            min,
            max,
            triangles: vec![],
            children: Some(Box::new([Self::build(triangles), Self::build(right)])),
        }
    }
    fn bound(&self, p: P) -> f64 {
        (0..3)
            .map(|k| (self.min[k] - p[k]).max(p[k] - self.max[k]).max(0.).powi(2))
            .sum()
    }
    fn nearest(&self, p: P, best: &mut f64) {
        if self.bound(p) > *best {
            return;
        }
        if let Some(children) = &self.children {
            let first = usize::from(children[1].bound(p) < children[0].bound(p));
            children[first].nearest(p, best);
            children[1 - first].nearest(p, best)
        } else {
            for t in &self.triangles {
                let q = closest_triangle(p, t.p[0], t.p[1], t.p[2]);
                *best = best.min(dot(sub(p, q), sub(p, q)))
            }
        }
    }
    fn distance(&self, p: P) -> f64 {
        let mut best = f64::INFINITY;
        self.nearest(p, &mut best);
        best.sqrt()
    }
    fn hits(&self, p: P, d: P, hits: &mut Vec<f64>) {
        let mut low: f64 = 0.;
        let mut high = f64::INFINITY;
        for k in 0..3 {
            let a = (self.min[k] - p[k]) / d[k];
            let b = (self.max[k] - p[k]) / d[k];
            low = low.max(a.min(b));
            high = high.min(a.max(b));
        }
        if high < low {
            return;
        }
        if let Some(children) = &self.children {
            children[0].hits(p, d, hits);
            children[1].hits(p, d, hits)
        } else {
            for t in &self.triangles {
                let e1 = sub(t.p[1], t.p[0]);
                let e2 = sub(t.p[2], t.p[0]);
                let h = cross(d, e2);
                let det = dot(e1, h);
                if det.abs() < 1e-13 {
                    continue;
                }
                let s = sub(p, t.p[0]);
                let u = dot(s, h) / det;
                if !(-1e-10..=1. + 1e-10).contains(&u) {
                    continue;
                }
                let q = cross(s, e1);
                let v = dot(d, q) / det;
                if v < -1e-10 || u + v > 1. + 1e-10 {
                    continue;
                }
                let along = dot(e2, q) / det;
                if along > 1e-10 {
                    hits.push(along)
                }
            }
        }
    }
    fn signed_distance(&self, p: P) -> f64 {
        let distance = self.distance(p);
        if distance < 1e-12 {
            return 0.;
        }
        let mut hits = vec![];
        self.hits(p, [1., 0.3713906763541037, 0.127831], &mut hits);
        hits.sort_by(f64::total_cmp);
        hits.dedup_by(|a, b| (*a - *b).abs() < 1e-8 * (1. + a.abs().max(b.abs())));
        if hits.len() % 2 == 1 {
            -distance
        } else {
            distance
        }
    }
}

/// Preorder flattening of the recursive BVH: min/max (6 f32), left/right
/// (i32, -1 for leaves), triangle window (start, count as u32 bits). Shared
/// by the wgpu and CUDA lattice kernels.
#[cfg(any(feature = "gpu", feature = "cuda"))]
pub(crate) fn flatten_lattice_bvh(root: &Node) -> (Vec<f32>, Vec<f32>) {
    let mut nodes: Vec<f32> = Vec::new();
    let mut triangles: Vec<f32> = Vec::new();
    fn emit(node: &Node, nodes: &mut Vec<f32>, triangles: &mut Vec<f32>) -> u32 {
        let index = (nodes.len() / 10) as u32;
        nodes.resize(nodes.len() + 10, 0.);
        for k in 0..3 {
            nodes[index as usize * 10 + k] = node.min[k] as f32;
            nodes[index as usize * 10 + 3 + k] = node.max[k] as f32;
        }
        if let Some(children) = &node.children {
            let left = emit(&children[0], nodes, triangles);
            let right = emit(&children[1], nodes, triangles);
            nodes[index as usize * 10 + 6] = f32::from_bits(left);
            nodes[index as usize * 10 + 7] = f32::from_bits(right);
        } else {
            nodes[index as usize * 10 + 6] = f32::from_bits(u32::MAX);
            nodes[index as usize * 10 + 7] = f32::from_bits(u32::MAX);
            let start = (triangles.len() / 9) as u32;
            for t in &node.triangles {
                for point in t.p {
                    for k in 0..3 {
                        triangles.push(point[k] as f32);
                    }
                }
            }
            nodes[index as usize * 10 + 8] = f32::from_bits(start);
            nodes[index as usize * 10 + 9] = f32::from_bits(triangles.len() as u32 / 9 - start);
        }
        index
    }
    emit(root, &mut nodes, &mut triangles);
    (nodes, triangles)
}

pub fn shell(mesh: &Mesh, openings: &[usize], thickness: f64, step: f64) -> Result<BuiltMesh> {
    shell_options(mesh, openings, thickness, step, false)
}
pub fn shell_options(
    mesh: &Mesh,
    openings: &[usize],
    thickness: f64,
    step: f64,
    adaptive: bool,
) -> Result<BuiltMesh> {
    let report = mesh.inspect()?;
    if !report.closed || report.signed_volume_mm3 <= 0. || report.degenerate_triangles > 0 {
        return Err(fail(
            "Sampled Shell requires a closed, outward-oriented, nondegenerate mesh",
        ));
    }
    if !thickness.is_finite()
        || !step.is_finite()
        || thickness <= 0.
        || step <= 0.
        || step > thickness / 3. + 1e-9
    {
        return Err(fail(
            "Shell grid step must be positive and no larger than one third of wall thickness",
        ));
    }
    let count = mesh.indices.len() / 3;
    if count > 30000
        || openings.is_empty()
        || openings.len() >= count
        || openings.iter().any(|&t| t >= count)
    {
        return Err(fail(
            "Select valid openings; sampled Shell supports up to 30000 source triangles",
        ));
    }
    let triangles: Vec<_> = mesh
        .indices
        .as_chunks::<3>()
        .0
        .iter()
        .map(|ids| {
            let p = ids.map_array(mesh);
            Triangle {
                min: std::array::from_fn(|k| p.iter().map(|p| p[k]).fold(f64::INFINITY, f64::min)),
                max: std::array::from_fn(|k| {
                    p.iter().map(|p| p[k]).fold(f64::NEG_INFINITY, f64::max)
                }),
                p,
            }
        })
        .collect();
    let removed: std::collections::BTreeSet<_> = openings.iter().copied().collect();
    let retained = Node::build(
        triangles
            .iter()
            .enumerate()
            .filter(|(i, _)| !removed.contains(i))
            .map(|(_, t)| t.clone())
            .collect(),
    );
    if !openings.iter().any(|&i| {
        let t = &triangles[i];
        let center = std::array::from_fn(|k| (t.p[0][k] + t.p[1][k] + t.p[2][k]) / 3.);
        retained.distance(center) > thickness + step / 2.
    }) {
        return Err(fail(
            "Opening is too small for the wall thickness and grid resolution; select a larger connected face patch",
        ));
    }
    let all = Node::build(triangles);
    let min = std::array::from_fn(|k| all.min[k] - (2.123 + 0.07 * k as f64) * step);
    let max = std::array::from_fn(|k| all.max[k] + (2.413 + 0.11 * k as f64) * step);
    let cells: [usize; 3] = std::array::from_fn(|k| ((max[k] - min[k]) / step).ceil() as usize);
    if cells.iter().any(|&n| n > if adaptive { 256 } else { 64 }) {
        return Err(fail(if adaptive {
            "Shell exceeds 256 adaptive cells per axis; increase grid step"
        } else {
            "Shell exceeds the 64-cell grid per axis; increase grid step or wall thickness"
        }));
    }
    let field = |p| all.signed_distance(p).max(retained.distance(p) - thickness);
    let grid = sdf_core::Grid { min, max, cells };
    let output = if adaptive {
        adaptive_tiles(field, &grid)?
    } else {
        crate::mesh_from_triangles(sdf_core::polygonize_with(field, &grid)?)
    };
    let report = output.inspect()?;
    if !report.closed || output.indices.is_empty() || report.signed_volume_mm3 <= 0. {
        return Err(fail(
            "Sampled Shell produced an invalid boundary; try a different grid step",
        ));
    }
    Ok(BuiltMesh {
        mesh: output,
        report,
    })
}
/// Sparse subdivision skips blocks whose Lipschitz distance bound excludes the zero set.
/// All active leaf tiles share one requested spacing, avoiding coarse/fine cracks.
fn adaptive_tiles(field: impl Fn(P) -> f64, grid: &sdf_core::Grid) -> Result<Mesh> {
    use std::collections::BTreeMap;
    let delta: P = std::array::from_fn(|k| (grid.max[k] - grid.min[k]) / grid.cells[k] as f64);
    let coord = |i: [usize; 3]| std::array::from_fn(|k| grid.min[k] + delta[k] * i[k] as f64);
    let mut pending = vec![([0usize; 3], grid.cells)];
    let mut output = Mesh {
        positions: vec![],
        indices: vec![],
        uv: None,
    };
    let mut welded = BTreeMap::<[i64; 3], usize>::new();
    let quantum = delta.iter().copied().fold(f64::INFINITY, f64::min) * 1e-7;
    let mut samples = 0usize;
    while let Some((lo, hi)) = pending.pop() {
        let min = coord(lo);
        let max = coord(hi);
        let center = std::array::from_fn(|k| (min[k] + max[k]) / 2.);
        let radius = dot(sub(max, min), sub(max, min)).sqrt() / 2.;
        if field(center).abs() > radius + quantum {
            continue;
        }
        let cells: [usize; 3] = std::array::from_fn(|k| hi[k] - lo[k]);
        let axis = (0..3).max_by_key(|&k| cells[k]).unwrap();
        if cells[axis] > 16 {
            let mid = (lo[axis] + hi[axis]) / 2;
            let mut a = hi;
            a[axis] = mid;
            let mut b = lo;
            b[axis] = mid;
            pending.push((lo, a));
            pending.push((b, hi));
            continue;
        }
        samples += cells.iter().map(|n| n + 1).product::<usize>();
        if samples > 4_000_000 {
            return Err(fail(
                "Adaptive Shell exceeds four million samples; increase grid step",
            ));
        }
        let tile = sdf_core::polygonize_tile(&field, &sdf_core::Grid { min, max, cells }, false)?;
        let ids: Vec<usize> = tile
            .positions
            .as_chunks::<3>()
            .0
            .iter()
            .map(|p| {
                let key = std::array::from_fn(|k| ((p[k] - grid.min[k]) / quantum).round() as i64);
                *welded.entry(key).or_insert_with(|| {
                    let id = output.positions.len() / 3;
                    output.positions.extend_from_slice(p);
                    id
                })
            })
            .collect();
        for t in tile.indices.as_chunks::<3>().0 {
            let tri = t.iter().map(|&i| ids[i]).collect::<Vec<_>>();
            if tri[0] != tri[1] && tri[1] != tri[2] && tri[0] != tri[2] {
                output.indices.extend(tri)
            }
        }
        if output.indices.len() > 300_000 {
            return Err(fail(
                "Adaptive Shell exceeds 100000 triangles; increase grid step",
            ));
        }
    }
    Ok(output)
}
trait TrianglePoints {
    fn map_array(&self, mesh: &Mesh) -> [P; 3];
}
impl TrianglePoints for [usize] {
    fn map_array(&self, mesh: &Mesh) -> [P; 3] {
        std::array::from_fn(|i| std::array::from_fn(|k| mesh.positions[self[i] * 3 + k]))
    }
}

/// The lattice field compute shader (WGSL), shared by the native `gpu` feature
/// and the browser WebGPU host path; both must execute the identical text.
pub const LATTICE_WGSL: &str = r##"
struct Params {
    nx: u32,
    ny: u32,
    nz: u32,
    n_segments: u32,
    n_nodes: u32,
    organic: u32,
    open_top: u32,
    keep_core: u32,
    min_x: f32,
    min_y: f32,
    min_z: f32,
    step_x: f32,
    step_y: f32,
    step_z: f32,
    skin: f32,
    radius_blend: f32,
    wall_depth: f32,
    top_z: f32,
    _pad0: f32,
    _pad1: f32,
}
@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read> nodes_flat: array<f32>;
@group(0) @binding(2) var<storage, read> tris: array<f32>;
@group(0) @binding(3) var<storage, read> segments: array<f32>;
@group(0) @binding(4) var<storage, read_write> values: array<f32>;

fn node_min(n: u32) -> vec3f { return vec3f(nodes_flat[n*10u], nodes_flat[n*10u+1u], nodes_flat[n*10u+2u]); }
fn node_max(n: u32) -> vec3f { return vec3f(nodes_flat[n*10u+3u], nodes_flat[n*10u+4u], nodes_flat[n*10u+5u]); }
fn node_left(n: u32) -> i32 { return bitcast<i32>(nodes_flat[n*10u+6u]); }
fn node_right(n: u32) -> i32 { return bitcast<i32>(nodes_flat[n*10u+7u]); }
fn node_start(n: u32) -> u32 { return bitcast<u32>(nodes_flat[n*10u+8u]); }
fn node_count(n: u32) -> u32 { return bitcast<u32>(nodes_flat[n*10u+9u]); }

fn tri_point(t: u32, k: u32) -> vec3f {
    let base = t * 9u + k * 3u;
    return vec3f(tris[base], tris[base+1u], tris[base+2u]);
}

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
        let de = distance(p, q);
        if (de < best_dist) {
            best = q;
            best_dist = de;
        }
    }
    return best;
}

// Min-reduction over squared distances is order-free; the iterative walk
// prunes any node whose bound exceeds the running best, exactly like the
// recursive reference.
fn nearest2(p: vec3f) -> f32 {
    var best = 3.402823466e+38;
    var stack: array<u32, 64>;
    var sp = 0u;
    stack[sp] = 0u;
    sp++;
    while (sp > 0u) {
        sp--;
        let n = stack[sp];
        let bmin = node_min(n) - p;
        let bmax = p - node_max(n);
        let d2v = max(max(bmin, bmax), vec3f(0.0));
        let bound = dot(d2v, d2v);
        if (bound > best) {
            continue;
        }
        let count = node_count(n);
        if (count > 0u) {
            for (var i = 0u; i < count; i++) {
                let t = node_start(n) + i;
                let q = closest_triangle(p, tri_point(t, 0u), tri_point(t, 1u), tri_point(t, 2u));
                let dq = p - q;
                best = min(best, dot(dq, dq));
            }
        } else {
            let l = u32(node_left(n));
            let r = u32(node_right(n));
            if (sp + 2u > 64u) { return -1.0; }
            stack[sp] = l;
            stack[sp + 1u] = r;
            sp += 2u;
        }
    }
    return best;
}

fn ray_hits(p: vec3f) -> vec2u {
    let dir = vec3f(1.0, 0.3713906763541037, 0.127831);
    var hits: array<f32, 128>;
    var count = 0u;
    var stack: array<u32, 64>;
    var sp = 0u;
    stack[sp] = 0u;
    sp++;
    while (sp > 0u) {
        sp--;
        let n = stack[sp];
        let nmin = node_min(n);
        let nmax = node_max(n);
        var low = 0.0;
        var high = 3.402823466e+38;
        for (var k = 0u; k < 3u; k++) {
            let a = (nmin[k] - p[k]) / dir[k];
            let b = (nmax[k] - p[k]) / dir[k];
            low = max(low, min(a, b));
            high = min(high, max(a, b));
        }
        if (high < low) {
            continue;
        }
        let tc = node_count(n);
        if (tc > 0u) {
            for (var i = 0u; i < tc; i++) {
                let t = node_start(n) + i;
                let a = tri_point(t, 0u);
                let b = tri_point(t, 1u);
                let c = tri_point(t, 2u);
                let e1 = b - a;
                let e2 = c - a;
                let h = cross(dir, e2);
                let det = dot(e1, h);
                if (abs(det) < 1e-13) {
                    continue;
                }
                let sv = p - a;
                let u = dot(sv, h) / det;
                if (u < -1e-10 || u > 1.0 + 1e-10) {
                    continue;
                }
                let q = cross(sv, e1);
                let v = dot(dir, q) / det;
                if (v < -1e-10 || u + v > 1.0 + 1e-10) {
                    continue;
                }
                let along = dot(e2, q) / det;
                if (along > 1e-10) {
                    if (count >= 128u) { return vec2u(0u, 1u); }
                    hits[count] = along;
                    count++;
                }
            }
        } else {
            let l = u32(node_left(n));
            let r = u32(node_right(n));
            if (sp + 2u > 64u) { return vec2u(0u, 1u); }
            stack[sp] = l;
            stack[sp + 1u] = r;
            sp += 2u;
        }
    }
    // Insertion sort over the few collected crossings, then the same relative
    // dedup and parity as the CPU reference (in f32 here).
    for (var i = 1u; i < count; i++) {
        let key = hits[i];
        var j = i;
        while (j > 0u && hits[j - 1u] > key) {
            hits[j] = hits[j - 1u];
            j--;
        }
        hits[j] = key;
    }
    var unique = 0u;
    for (var i = 0u; i < count; i++) {
        if (i == 0u || abs(hits[i] - hits[i - 1u]) >= 1e-8 * (1.0 + max(abs(hits[i]), abs(hits[i - 1u])))) {
            unique++;
        }
    }
    return vec2u(unique, 0u);
}

fn signed_distance(p: vec3f) -> f32 {
    let d2 = nearest2(p);
    if (d2 < 0.0) { return bitcast<f32>(0x7fc00000u); }
    let distance = sqrt(d2);
    if (distance < 1e-12) { return 0.0; }
    let h = ray_hits(p);
    if (h.y > 0u) { return bitcast<f32>(0x7fc00000u); }
    if (h.x % 2u == 1u) { return -distance; }
    return distance;
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let row = params.nx + 1u;
    let total = row * (params.ny + 1u) * (params.nz + 1u);
    if (id.x >= total) { return; }
    let x = id.x % row;
    let y = (id.x / row) % (params.ny + 1u);
    let z = id.x / (row * (params.ny + 1u));
    let p = vec3f(
        params.min_x + f32(x) * params.step_x,
        params.min_y + f32(y) * params.step_y,
        params.min_z + f32(z) * params.step_z,
    );
    let source = signed_distance(p);
    if (source != source) {
        values[id.x] = bitcast<f32>(0x7fc00000u);
        return;
    }
    var graph = 3.402823466e+38;
    for (var s = 0u; s < params.n_segments; s++) {
        let base = s * 8u;
        let a = vec3f(segments[base], segments[base + 1u], segments[base + 2u]);
        let d = vec3f(segments[base + 3u], segments[base + 4u], segments[base + 5u]);
        let length2 = segments[base + 6u];
        let r = segments[base + 7u];
        let q = p - a;
        let t = clamp(dot(q, d) / length2, 0.0, 1.0);
        let delta = q - t * d;
        let distance = length(delta) - r;
        if (params.organic > 0u) {
            let h = max((params.radius_blend - abs(graph - distance)) / params.radius_blend, 0.0);
            graph = min(graph, distance) - h * h * params.radius_blend / 4.0;
        } else {
            graph = min(graph, distance);
        }
    }
    var skin_field = 3.402823466e+38;
    if (params.skin > 0.0) {
        var cap = -3.402823466e+38;
        if (params.open_top > 0u) {
            cap = p.z - (params.top_z - params.skin);
        }
        skin_field = max(-source - params.skin, cap);
    }
    var material = min(graph, skin_field);
    if (params.wall_depth > 0.0) {
        let band = -source - params.wall_depth;
        var walls = max(material, band);
        if (params.keep_core > 0u) {
            walls = min(walls, source + params.wall_depth);
        }
        material = walls;
    }
    values[id.x] = max(source, material);
}
"##;
/// A bounded spatial graph of rounded struts, optionally blended into a skin.
pub fn lattice(
    mesh: &Mesh,
    nodes: Vec<P>,
    edges: Vec<[usize; 2]>,
    radius: f64,
    skin: f64,
    step: f64,
    organic: bool,
    open_top: bool,
    wall_depth: f64,
    keep_core: bool,
) -> Result<BuiltMesh> {
    lattice_accelerated(
        mesh,
        nodes,
        edges,
        radius,
        skin,
        step,
        organic,
        open_top,
        wall_depth,
        keep_core,
        Acceleration::Cpu,
    )
}

/// Size-based placement recommendation for the spatial lattice field sampler.
/// Work scales with sampled grid points times the source shell and graph
/// complexity; extraction/audit remain CPU-bound, so small grids stay on CPU.
pub fn recommended_for_lattice(
    grid_points: usize,
    source_triangles: usize,
    segment_count: usize,
) -> Acceleration {
    const GPU_WORK_THRESHOLD: usize = 500_000;
    let complexity = source_triangles.saturating_add(segment_count).max(1);
    let work = grid_points.saturating_mul(complexity);
    if cfg!(feature = "cuda") && work >= GPU_WORK_THRESHOLD {
        Acceleration::Cuda
    } else if cfg!(feature = "gpu") && work >= GPU_WORK_THRESHOLD {
        Acceleration::Gpu
    } else {
        Acceleration::Cpu
    }
}

/// `lattice` with an optional GPU field sampler: the implicit field (BVH signed
/// distance plus capsule graph) is evaluated on the GPU in f32; snap, boundary
/// validation, marching-tetrahedra and the final mesh audit stay on the CPU.
/// Anything ineligible or unavailable falls back to the CPU reference.
pub fn lattice_accelerated(
    mesh: &Mesh,
    nodes: Vec<P>,
    edges: Vec<[usize; 2]>,
    radius: f64,
    skin: f64,
    step: f64,
    organic: bool,
    open_top: bool,
    wall_depth: f64,
    keep_core: bool,
    acceleration: Acceleration,
) -> Result<BuiltMesh> {
    let report = mesh.inspect()?;
    if !report.closed || report.signed_volume_mm3 <= 0. || mesh.indices.len() / 3 > 30000 {
        return Err(fail(
            "Spatial lattice requires a closed outward solid with at most 30000 source triangles",
        ));
    }
    if nodes.is_empty()
        || nodes.len() > 125
        || edges.is_empty()
        || edges.len() > 400
        || nodes.iter().flatten().any(|v| !v.is_finite())
        || edges
            .iter()
            .any(|e| e[0] >= nodes.len() || e[1] >= nodes.len() || e[0] == e[1])
    {
        return Err(fail(
            "Spatial lattice graph exceeds limits or has invalid nodes",
        ));
    }
    if !radius.is_finite()
        || !skin.is_finite()
        || !step.is_finite()
        || radius <= 0.
        || skin < 0.
        || step <= 0.
        || step > radius * 0.8
        || skin > 0. && step > skin / 2.
    {
        return Err(fail(
            "Grid step must resolve the strut diameter and skin: step <= diameter/2.5 and skin/2",
        ));
    }
    let triangles = mesh
        .indices
        .as_chunks::<3>()
        .0
        .iter()
        .map(|ids| {
            let p = ids.map_array(mesh);
            Triangle {
                min: std::array::from_fn(|k| p.iter().map(|p| p[k]).fold(f64::INFINITY, f64::min)),
                max: std::array::from_fn(|k| {
                    p.iter().map(|p| p[k]).fold(f64::NEG_INFINITY, f64::max)
                }),
                p,
            }
        })
        .collect();
    let all = Node::build(triangles);
    let segments: Vec<_> = edges
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let a = nodes[e[0]];
            let d = sub(nodes[e[1]], a);
            let length2 = dot(d, d);
            (
                a,
                d,
                length2,
                radius
                    * if organic {
                        1.0 + 0.3 * ((i * 73 % 101) as f64 / 100.)
                    } else {
                        1.
                    },
            )
        })
        .collect();
    if segments.iter().any(|s| s.2 < 1e-12) {
        return Err(fail("Coincident lattice nodes"));
    }
    let min: P = std::array::from_fn(|k| all.min[k] - (2.123 + 0.07 * k as f64) * step);
    let max: P = std::array::from_fn(|k| all.max[k] + (2.413 + 0.11 * k as f64) * step);
    let cells = std::array::from_fn(|k| ((max[k] - min[k]) / step).ceil() as usize);
    if cells.iter().any(|&n| n > 64) {
        return Err(fail(
            "Spatial lattice exceeds 64 grid cells per axis. Increase strut thickness and grid step.",
        ));
    }
    let grid_points = (cells[0] + 1) * (cells[1] + 1) * (cells[2] + 1);
    let acceleration = match acceleration {
        Acceleration::Auto => {
            recommended_for_lattice(grid_points, mesh.indices.len() / 3, segments.len())
        }
        explicit => explicit,
    };
    if !wall_depth.is_finite() || wall_depth < 0. || wall_depth > 0. && wall_depth < step * 2. {
        return Err(fail("Wall depth must be at least two sampling steps"));
    }
    let blend = radius * 0.7;
    let field = |p: P| {
        let source = all.signed_distance(p);
        let mut graph = f64::INFINITY;
        for &(a, d, length2, r) in &segments {
            let q = sub(p, a);
            let t = (dot(q, d) / length2).clamp(0., 1.);
            let delta: P = std::array::from_fn(|k| q[k] - t * d[k]);
            let distance = dot(delta, delta).sqrt() - r;
            if organic && graph.is_finite() {
                let h = ((blend - (graph - distance).abs()) / blend).max(0.);
                graph = graph.min(distance) - h * h * blend / 4.
            } else {
                graph = graph.min(distance)
            }
        }
        let skin_field = if skin > 0. {
            (-source - skin).max(if open_top {
                p[2] - (all.max[2] - skin)
            } else {
                f64::NEG_INFINITY
            })
        } else {
            f64::INFINITY
        };
        let material = graph.min(skin_field);
        let material = if wall_depth > 0. {
            let band = -source - wall_depth;
            let walls = material.max(band);
            if keep_core {
                walls.min(source + wall_depth)
            } else {
                walls
            }
        } else {
            material
        };
        source.max(material)
    };
    let mut output = None;
    #[cfg(feature = "cuda")]
    if acceleration == Acceleration::Cuda {
        output = crate::lattice_cuda::try_cuda(
            &all, &segments, min, max, cells, skin, organic, open_top, wall_depth, keep_core,
            blend, &field,
        )?;
    }
    #[cfg(feature = "gpu")]
    if output.is_none() && acceleration.is_gpu() {
        output = crate::lattice_gpu::try_gpu(
            &all, &segments, min, max, cells, skin, organic, open_top, wall_depth, keep_core,
            blend, &field,
        )?;
    }
    let output = match output {
        Some(mesh) => mesh,
        None => crate::mesh_from_triangles(sdf_core::polygonize_with(
            &field,
            &sdf_core::Grid { min, max, cells },
        )?),
    };
    #[cfg(not(feature = "gpu"))]
    let _ = acceleration;
    let result = output.inspect()?;
    if !result.closed
        || result.degenerate_triangles > 0
        || result.signed_volume_mm3 <= 0.
        || result.signed_volume_mm3 >= report.signed_volume_mm3
    {
        return Err(fail(
            "Spatial lattice did not produce a valid lighter closed surface; adjust cell size or resolution",
        ));
    }
    Ok(BuiltMesh {
        mesh: output,
        report: result,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lattice_auto_recommendation_tracks_work_size() {
        assert_eq!(recommended_for_lattice(1_000, 8, 8), Acceleration::Cpu);
        let large = recommended_for_lattice(100_000, 12, 54);
        if cfg!(feature = "cuda") {
            assert_eq!(large, Acceleration::Cuda);
        } else if cfg!(feature = "gpu") {
            assert_eq!(large, Acceleration::Gpu);
        } else {
            assert_eq!(large, Acceleration::Cpu);
        }
    }

    #[test]
    fn lattice_auto_matches_cpu_on_small_grid() {
        let mesh = polygon_core::solid::primitives::cube([20.; 3], false).unwrap();
        let nodes = vec![[5., 5., 5.], [15., 5., 5.], [5., 15., 5.], [5., 5., 15.]];
        let edges = vec![[0, 1], [0, 2], [0, 3]];
        let reference = lattice(
            &mesh,
            nodes.clone(),
            edges.clone(),
            4.,
            0.,
            3.,
            false,
            false,
            0.,
            false,
        )
        .unwrap();
        let automatic = lattice_accelerated(
            &mesh,
            nodes,
            edges,
            4.,
            0.,
            3.,
            false,
            false,
            0.,
            false,
            Acceleration::Auto,
        )
        .unwrap();
        assert_eq!(automatic.mesh.indices.len(), reference.mesh.indices.len());
        assert!(
            (automatic.report.signed_volume_mm3 - reference.report.signed_volume_mm3).abs() < 1e-9
        );
    }

    #[cfg(feature = "gpu")]
    #[test]
    fn gpu_lattice_matches_cpu_reference() {
        let mesh = polygon_core::solid::primitives::cube([30.; 3], false).unwrap();
        let nodes = vec![[5., 5., 5.], [25., 5., 5.], [5., 25., 5.], [5., 5., 25.]];
        let edges = vec![[0, 1], [0, 2], [0, 3]];
        let reference = lattice(
            &mesh,
            nodes.clone(),
            edges.clone(),
            2.,
            0.,
            1.,
            false,
            false,
            0.,
            false,
        )
        .unwrap();
        let accelerated = lattice_accelerated(
            &mesh,
            nodes,
            edges,
            2.,
            0.,
            1.,
            false,
            false,
            0.,
            false,
            Acceleration::Gpu,
        )
        .unwrap();
        if crate::lattice_gpu::try_gpu_available() {
            let dt =
                (accelerated.mesh.indices.len() as f64 - reference.mesh.indices.len() as f64).abs();
            assert!(
                dt <= 0.01 * reference.mesh.indices.len() as f64,
                "triangle counts diverge: {} vs {}",
                reference.mesh.indices.len(),
                accelerated.mesh.indices.len()
            );
            let dv = (accelerated.report.signed_volume_mm3 - reference.report.signed_volume_mm3)
                .abs()
                / reference.report.signed_volume_mm3;
            assert!(dv < 0.001, "volume diverges: {dv}");
        }
    }

    #[cfg(feature = "cuda")]
    #[test]
    fn cuda_lattice_matches_cpu_reference() {
        let mesh = polygon_core::solid::primitives::cube([30.; 3], false).unwrap();
        let nodes = vec![[5., 5., 5.], [25., 5., 5.], [5., 25., 5.], [5., 5., 25.]];
        let edges = vec![[0, 1], [0, 2], [0, 3]];
        let reference = lattice(
            &mesh,
            nodes.clone(),
            edges.clone(),
            2.,
            0.,
            1.,
            false,
            false,
            0.,
            false,
        )
        .unwrap();
        let accelerated = lattice_accelerated(
            &mesh,
            nodes,
            edges,
            2.,
            0.,
            1.,
            false,
            false,
            0.,
            false,
            Acceleration::Cuda,
        )
        .unwrap();
        if crate::lattice_cuda::try_cuda_available() {
            let dt =
                (accelerated.mesh.indices.len() as f64 - reference.mesh.indices.len() as f64).abs();
            assert!(
                dt <= 0.01 * reference.mesh.indices.len() as f64,
                "triangle counts diverge: {} vs {}",
                reference.mesh.indices.len(),
                accelerated.mesh.indices.len()
            );
            let dv = (accelerated.report.signed_volume_mm3 - reference.report.signed_volume_mm3)
                .abs()
                / reference.report.signed_volume_mm3;
            assert!(dv < 0.001, "volume diverges: {dv}");
        }
    }

    #[test]
    fn bvh_distance_and_sign_agree_with_reference() {
        let mesh = polygon_core::solid::primitives::cube([10.; 3], false).unwrap();
        let triangles = mesh
            .indices
            .as_chunks::<3>()
            .0
            .iter()
            .map(|ids| {
                let p = ids.map_array(&mesh);
                Triangle {
                    min: std::array::from_fn(|k| {
                        p.iter().map(|p| p[k]).fold(f64::INFINITY, f64::min)
                    }),
                    max: std::array::from_fn(|k| {
                        p.iter().map(|p| p[k]).fold(f64::NEG_INFINITY, f64::max)
                    }),
                    p,
                }
            })
            .collect();
        let bvh = Node::build(triangles);
        for i in 0..250 {
            let p = [
                ((i * 73) % 197) as f64 / 10. - 5.,
                ((i * 31) % 193) as f64 / 10. - 5.,
                ((i * 17) % 191) as f64 / 10. - 5.,
            ];
            let actual = bvh.signed_distance(p);
            let expected = polygon_core::solid::proximity::signed_distance(&mesh, p);
            assert!(
                (actual - expected).abs() < 1e-7,
                "{p:?}: {actual} != {expected}"
            )
        }
    }
}
