use super::*;

pub(super) fn fail(message: impl Into<String>) -> Error {
    Error::new("GEOMETRY_INVALID_INPUT", message)
}
pub(crate) type P = [f64; 3];
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
    pub(super) fn build(mut triangles: Vec<Triangle>) -> Self {
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
    pub(super) fn signed_distance(&self, p: P) -> f64 {
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
pub(super) trait TrianglePoints {
    fn map_array(&self, mesh: &Mesh) -> [P; 3];
}
impl TrianglePoints for [usize] {
    fn map_array(&self, mesh: &Mesh) -> [P; 3] {
        std::array::from_fn(|i| std::array::from_fn(|k| mesh.positions[self[i] * 3 + k]))
    }
}
