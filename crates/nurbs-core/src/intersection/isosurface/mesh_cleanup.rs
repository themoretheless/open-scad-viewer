use super::*;

/// Sharp edges: mesh edges whose two adjacent face normals differ by more
/// than `angle_threshold` radians. Deterministic (sorted by vertex pair).
pub fn sharp_edges(mesh: &IsoMesh, angle_threshold: f64) -> Vec<[u32; 2]> {
    let mut adjacency: BTreeMap<(u32, u32), Vec<usize>> = BTreeMap::new();
    for (t, tri) in mesh.triangles.iter().enumerate() {
        for e in 0..3 {
            let (a, b) = (tri[e], tri[(e + 1) % 3]);
            adjacency.entry((a.min(b), a.max(b))).or_default().push(t);
        }
    }
    let mut result = Vec::new();
    for (&(a, b), tris) in &adjacency {
        if tris.len() != 2 {
            continue;
        }
        if let (Some(u), Some(v)) = (triangle_normal(mesh, tris[0]), triangle_normal(mesh, tris[1]))
        {
            let cos = dot(u, v).clamp(-1., 1.);
            if cos.acos() > angle_threshold {
                result.push([a, b]);
            }
        }
    }
    result
}

pub(super) fn triangle_normal(mesh: &IsoMesh, t: usize) -> Option<[f64; 3]> {
    let tri = mesh.triangles[t];
    let (a, b, c) = (
        mesh.points[tri[0] as usize],
        mesh.points[tri[1] as usize],
        mesh.points[tri[2] as usize],
    );
    let n = cross(
        [b[0] - a[0], b[1] - a[1], b[2] - a[2]],
        [c[0] - a[0], c[1] - a[1], c[2] - a[2]],
    );
    let len = norm(n);
    (len > 1e-18).then(|| [n[0] / len, n[1] / len, n[2] / len])
}

/// Report of a smoothing run.
#[derive(Clone, Copy, Debug, Default)]
pub struct SmoothReport {
    /// Iterations actually performed.
    pub iterations: usize,
    /// Signed volume before smoothing.
    pub volume_before: f64,
    /// Signed volume after smoothing.
    pub volume_after: f64,
    /// `volume_after − volume_before`; near zero for well-tuned Taubin.
    pub volume_drift: f64,
}

/// Vertex adjacency (sorted, deduplicated) built from the triangle list.
pub(super) fn vertex_adjacency(mesh: &IsoMesh) -> Vec<Vec<u32>> {
    let mut sets: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for tri in &mesh.triangles {
        for e in 0..3 {
            let (a, b) = (tri[e], tri[(e + 1) % 3]);
            sets.entry(a).or_default().push(b);
            sets.entry(b).or_default().push(a);
        }
    }
    let mut adjacency = vec![Vec::new(); mesh.points.len()];
    for (v, mut nbrs) in sets {
        nbrs.sort_unstable();
        nbrs.dedup();
        adjacency[v as usize] = nbrs;
    }
    adjacency
}

/// One umbrella-operator pass: `p += w · (mean(neighbors) − p)`.
pub(super) fn smooth_pass(points: &mut [[f64; 3]], adjacency: &[Vec<u32>], w: f64) {
    let mut next = points.to_vec();
    for (i, nbrs) in adjacency.iter().enumerate() {
        if nbrs.is_empty() {
            continue;
        }
        let mut mean = [0.; 3];
        for &j in nbrs {
            let p = points[j as usize];
            mean[0] += p[0] / nbrs.len() as f64;
            mean[1] += p[1] / nbrs.len() as f64;
            mean[2] += p[2] / nbrs.len() as f64;
        }
        for axis in 0..3 {
            next[i][axis] = points[i][axis] + w * (mean[axis] - points[i][axis]);
        }
    }
    points.copy_from_slice(&next);
}

/// Taubin λ/μ smoothing: each iteration applies a shrink pass with weight
/// `lambda` followed by an inflation pass with weight `mu` of opposite sign,
/// `|mu| > lambda`, which cancels the Laplacian shrinkage (item 941). The
/// report carries the signed volume (divergence theorem) before and after.
pub fn taubin_smooth(
    mesh: &mut IsoMesh,
    lambda: f64,
    mu: f64,
    iterations: usize,
) -> Result<SmoothReport> {
    check(
        lambda.is_finite() && (0. ..1.).contains(&lambda),
        "Taubin lambda must lie in (0, 1)",
    )?;
    check(
        mu.is_finite() && mu < 0. && mu.abs() > lambda,
        "Taubin mu must be negative with |mu| > lambda",
    )?;
    if iterations > MAX_SMOOTH_ITERATIONS {
        return Err(resource("Taubin iteration budget exceeded"));
    }
    let mut guard =
        Budget::with_iterations(MAX_SMOOTH_ITERATIONS)?.guard("isosurface_taubin_smooth");
    let volume_before = mesh.signed_volume();
    let adjacency = vertex_adjacency(mesh);
    for _ in 0..iterations {
        guard.tick()?;
        smooth_pass(&mut mesh.points, &adjacency, lambda);
        smooth_pass(&mut mesh.points, &adjacency, mu);
    }
    let volume_after = mesh.signed_volume();
    Ok(SmoothReport {
        iterations,
        volume_before,
        volume_after,
        volume_drift: volume_after - volume_before,
    })
}

/// Pure Laplacian smoothing (shrink pass only) — the baseline Taubin
/// improves upon; exposed for comparison and validation.
pub fn laplacian_smooth(
    mesh: &mut IsoMesh,
    lambda: f64,
    iterations: usize,
) -> Result<SmoothReport> {
    check(
        lambda.is_finite() && (0. ..1.).contains(&lambda),
        "Laplacian lambda must lie in (0, 1)",
    )?;
    if iterations > MAX_SMOOTH_ITERATIONS {
        return Err(resource("Laplacian iteration budget exceeded"));
    }
    let mut guard =
        Budget::with_iterations(MAX_SMOOTH_ITERATIONS)?.guard("isosurface_laplacian_smooth");
    let volume_before = mesh.signed_volume();
    let adjacency = vertex_adjacency(mesh);
    for _ in 0..iterations {
        guard.tick()?;
        smooth_pass(&mut mesh.points, &adjacency, lambda);
    }
    let volume_after = mesh.signed_volume();
    Ok(SmoothReport {
        iterations,
        volume_before,
        volume_after,
        volume_drift: volume_after - volume_before,
    })
}

/// Report of small-component cleanup.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CleanupReport {
    /// Connected components present before cleanup.
    pub components_before: usize,
    /// Components removed (volume below the threshold).
    pub components_removed: usize,
    /// Volumes of the removed components, in first-triangle order.
    pub removed_volumes: Vec<f64>,
    /// Triangles removed in total.
    pub triangles_removed: usize,
}

/// Union-find with path compression over triangle indices; unions always
/// toward the smaller root for determinism.
struct UnionFind {
    parent: Vec<usize>,
}

impl UnionFind {
    fn new(n: usize) -> Self {
        Self {
            parent: (0..n).collect(),
        }
    }
    fn find(&mut self, mut x: usize) -> usize {
        while self.parent[x] != x {
            self.parent[x] = self.parent[self.parent[x]];
            x = self.parent[x];
        }
        x
    }
    fn union(&mut self, a: usize, b: usize) {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra != rb {
            self.parent[rb.max(ra)] = ra.min(rb);
        }
    }
}

/// Remove connected components whose absolute signed volume is below
/// `min_volume` (item 941). Components are linked through shared vertices via
/// union-find; the surviving mesh keeps the original triangle order with
/// vertices renumbered in first-use order — fully deterministic.
pub fn remove_small_components(mesh: &mut IsoMesh, min_volume: f64) -> Result<CleanupReport> {
    check(
        min_volume.is_finite() && min_volume >= 0.,
        "component volume threshold must be non-negative and finite",
    )?;
    let ntris = mesh.triangles.len();
    let mut uf = UnionFind::new(ntris);
    let mut first_triangle: BTreeMap<u32, usize> = BTreeMap::new();
    for (t, tri) in mesh.triangles.iter().enumerate() {
        for &v in tri {
            match first_triangle.get(&v) {
                Some(&prev) => uf.union(prev, t),
                None => {
                    first_triangle.insert(v, t);
                }
            }
        }
    }
    let mut volume_by_root: BTreeMap<usize, f64> = BTreeMap::new();
    for t in 0..ntris {
        let root = uf.find(t);
        let tri = mesh.triangles[t];
        let (a, b, c) = (
            mesh.points[tri[0] as usize],
            mesh.points[tri[1] as usize],
            mesh.points[tri[2] as usize],
        );
        *volume_by_root.entry(root).or_default() += dot(a, cross(b, c)) / 6.;
    }
    let components_before = volume_by_root.len();
    // Roots are minimum triangle indices, so ascending root order is the
    // deterministic discovery order.
    let mut kept_roots = Vec::new();
    let mut removed_volumes = Vec::new();
    for (&root, &volume) in &volume_by_root {
        let volume = volume.abs();
        if volume < min_volume {
            removed_volumes.push(volume);
        } else {
            kept_roots.push(root);
        }
    }
    let components_removed = removed_volumes.len();
    if components_removed == 0 {
        return Ok(CleanupReport {
            components_before,
            components_removed: 0,
            removed_volumes: Vec::new(),
            triangles_removed: 0,
        });
    }
    let kept: Vec<[u32; 3]> = mesh
        .triangles
        .iter()
        .copied()
        .enumerate()
        .filter(|(t, _)| kept_roots.contains(&uf.find(*t)))
        .map(|(_, tri)| tri)
        .collect();
    let triangles_removed = ntris - kept.len();
    let mut remap = vec![u32::MAX; mesh.points.len()];
    let mut points = Vec::new();
    let mut triangles = Vec::with_capacity(kept.len());
    for tri in kept {
        let mut out = [0u32; 3];
        for (e, o) in out.iter_mut().enumerate() {
            let v = tri[e] as usize;
            if remap[v] == u32::MAX {
                remap[v] = points.len() as u32;
                points.push(mesh.points[v]);
            }
            *o = remap[v];
        }
        triangles.push(out);
    }
    mesh.points = points;
    mesh.triangles = triangles;
    Ok(CleanupReport {
        components_before,
        components_removed,
        removed_volumes,
        triangles_removed,
    })
}
