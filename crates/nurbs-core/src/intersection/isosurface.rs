//! Adaptive octree isosurface extraction from a signed-distance field
//! (items 941, 967): adaptive marching cubes (tetrahedral decomposition) and
//! dual contouring with a per-cell quadratic error function (QEF) for sharp
//! features, followed by mesh hygiene — Taubin λ/μ smoothing without
//! shrinkage and small connected-component cleanup.
//!
//! The SDF is supplied as a closure `Fn([f64; 3]) -> f64` with an optional
//! analytic gradient; without one, central finite differences are used. The
//! octree subdivides a cell when it carries a sign change and the
//! linearization (curvature) error at the cell center exceeds the configured
//! tolerance; subdivision depth and the total cell count are hard budgets
//! whose exhaustion is a `NURBS_RESOURCE_LIMIT` error, never a silent
//! truncation.
//!
//! Dual contouring places one vertex per sign-changing cell by minimizing
//! the QEF Σ (n_i·(x − p_i))² over the cell's edge-intersection points p_i
//! with surface normals n_i; the small symmetric 3×3 system goes through the
//! pivoted dense solver in `math-core`, with a centroid fallback when the
//! system is degenerate. Mesh edges whose adjacent face normals differ by
//! more than the sharp-angle threshold (default 30°) are reported as sharp.
//!
//! Determinism: children are visited in a fixed lexicographic order, all
//! maps are ordered (`BTreeMap`), and the output index buffers depend only
//! on the inputs — identical inputs produce a byte-identical mesh.

use crate::foundation::guards::{Budget, require_finite_point};
use crate::{Result, check, numeric, resource};
use math_core::{cross, dot, norm, solve};
use std::collections::BTreeMap;

/// Hard cap on octree depth regardless of configuration (grid is 2^d per axis).
pub const HARD_MAX_DEPTH: u32 = 8;
/// Default budget of octree/extraction cells.
pub const DEFAULT_MAX_CELLS: usize = 1 << 20;
/// Hard cap on output triangles.
pub const MAX_TRIANGLES: usize = 4 << 20;
/// Hard cap on Taubin/Laplacian iteration counts.
pub const MAX_SMOOTH_ITERATIONS: usize = 4096;
/// Bisection iterations for root localization (fixed → deterministic).
const BISECTION_ITERS: usize = 32;
/// Default sharp-edge threshold between adjacent face normals, radians (30°).
pub const DEFAULT_SHARP_ANGLE: f64 = std::f64::consts::PI / 6.;

/// Cube corner offset for corner index `c` in `(x, y, z)` bit order.
const CORNER_OFFSET: [[u32; 3]; 8] = [
    [0, 0, 0],
    [1, 0, 0],
    [0, 1, 0],
    [1, 1, 0],
    [0, 0, 1],
    [1, 0, 1],
    [0, 1, 1],
    [1, 1, 1],
];

/// The 12 cube edges as corner-index pairs.
const CUBE_EDGES: [(usize, usize); 12] = [
    (0, 1),
    (1, 3),
    (3, 2),
    (2, 0),
    (4, 5),
    (5, 7),
    (7, 6),
    (6, 4),
    (0, 4),
    (1, 5),
    (2, 6),
    (3, 7),
];

/// Canonical 6-tetrahedra split of a cube around the 0–7 diagonal.
const CUBE_TETS: [[usize; 4]; 6] = [
    [0, 1, 3, 7],
    [0, 3, 2, 7],
    [0, 2, 6, 7],
    [0, 6, 4, 7],
    [0, 4, 5, 7],
    [0, 5, 1, 7],
];

/// Indexed triangle mesh produced by isosurface extraction.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct IsoMesh {
    /// Vertex positions.
    pub points: Vec<[f64; 3]>,
    /// Triangle corners as indices into [`IsoMesh::points`], oriented so the
    /// geometric normal points toward positive SDF (outward).
    pub triangles: Vec<[u32; 3]>,
}

impl IsoMesh {
    /// Signed volume via the divergence theorem (positive for outward
    /// orientation of a closed component).
    pub fn signed_volume(&self) -> f64 {
        let mut volume = 0.;
        for t in &self.triangles {
            let (a, b, c) = (
                self.points[t[0] as usize],
                self.points[t[1] as usize],
                self.points[t[2] as usize],
            );
            volume += dot(a, cross(b, c)) / 6.;
        }
        volume
    }
}

/// Vertex placement strategy inside a sign-changing cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VertexMode {
    /// QEF minimization (sharp-feature preserving); centroid fallback.
    Qef,
    /// Naive centroid of edge intersections (comparison baseline).
    Centroid,
}

/// Surface extraction strategy on the refined grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Extraction {
    /// Dual contouring: one vertex per sign-changing cell, quads across
    /// sign-changing grid edges.
    DualContouring,
    /// Marching cubes via canonical 6-tetrahedra decomposition.
    MarchingCubes,
}

/// Configuration for [`polygonize_sdf`].
#[derive(Clone, Copy, Debug)]
pub struct IsosurfaceConfig {
    /// Maximum octree depth; must be ≤ [`HARD_MAX_DEPTH`].
    pub max_depth: u32,
    /// Budget on octree cells created during refinement and on uniform
    /// extraction cells (2^depth)³; exhaustion is a resource error.
    pub max_cells: usize,
    /// Linearization (curvature) error tolerance driving subdivision.
    pub linearization_tolerance: f64,
    /// Face-normal angle above which a mesh edge is reported sharp, radians.
    pub sharp_angle: f64,
    /// Vertex placement strategy.
    pub vertex_mode: VertexMode,
    /// Extraction strategy.
    pub extraction: Extraction,
}

impl Default for IsosurfaceConfig {
    fn default() -> Self {
        Self {
            max_depth: 6,
            max_cells: DEFAULT_MAX_CELLS,
            linearization_tolerance: 1e-3,
            sharp_angle: DEFAULT_SHARP_ANGLE,
            vertex_mode: VertexMode::Qef,
            extraction: Extraction::DualContouring,
        }
    }
}

/// Statistics gathered during extraction.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IsosurfaceReport {
    /// Octree cells created during adaptive refinement (root included).
    pub octree_cells: usize,
    /// Octree leaves carrying a sign change.
    pub sign_leaves: usize,
    /// Depth at which the surface was extracted.
    pub depth_reached: u32,
    /// Number of SDF evaluations.
    pub sdf_evaluations: usize,
    /// Number of cells where the QEF solve degenerated to the centroid.
    pub qef_fallbacks: usize,
}

/// Extraction result: mesh, sharp edges, and statistics.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct IsosurfaceOutput {
    /// Extracted mesh.
    pub mesh: IsoMesh,
    /// Sharp edges as sorted vertex-index pairs.
    pub sharp_edges: Vec<[u32; 2]>,
    /// Extraction statistics.
    pub report: IsosurfaceReport,
}

/// SDF evaluator with optional analytic gradient and an evaluation counter.
struct Evaluator<'a> {
    sdf: &'a dyn Fn([f64; 3]) -> f64,
    gradient: Option<&'a dyn Fn([f64; 3]) -> [f64; 3]>,
    /// Finite-difference step when no analytic gradient is supplied.
    fd_step: f64,
    evaluations: usize,
}

impl Evaluator<'_> {
    fn value(&mut self, p: [f64; 3]) -> Result<f64> {
        self.evaluations += 1;
        let v = (self.sdf)(p);
        numeric(v.is_finite(), "SDF returned a non-finite value")?;
        Ok(v)
    }

    fn normal(&mut self, p: [f64; 3]) -> Result<Option<[f64; 3]>> {
        let g = match self.gradient {
            Some(grad) => {
                let g = grad(p);
                numeric(
                    g.iter().all(|v| v.is_finite()),
                    "SDF gradient returned a non-finite value",
                )?;
                g
            }
            None => {
                let h = self.fd_step;
                let mut g = [0.; 3];
                for axis in 0..3 {
                    let (mut pp, mut pm) = (p, p);
                    pp[axis] += h;
                    pm[axis] -= h;
                    g[axis] = (self.value(pp)? - self.value(pm)?) / (2. * h);
                }
                g
            }
        };
        let len = norm(g);
        Ok((len > 1e-12).then(|| [g[0] / len, g[1] / len, g[2] / len]))
    }
}

/// Locate the root on the segment a–b (values va, vb of opposite sign) by a
/// fixed number of bisection steps — deterministic by construction.
fn bisect_root(
    eval: &mut Evaluator,
    a: [f64; 3],
    b: [f64; 3],
    mut va: f64,
    mut vb: f64,
) -> Result<[f64; 3]> {
    let (mut lo, mut hi) = (a, b);
    let mut guard = Budget::with_iterations(BISECTION_ITERS)?.guard("isosurface_bisect");
    for _ in 0..BISECTION_ITERS {
        guard.tick()?;
        let mid = [
            (lo[0] + hi[0]) / 2.,
            (lo[1] + hi[1]) / 2.,
            (lo[2] + hi[2]) / 2.,
        ];
        let vm = eval.value(mid)?;
        if (vm < 0.) == (va < 0.) {
            lo = mid;
            va = vm;
        } else {
            hi = mid;
            vb = vm;
        }
    }
    let _ = vb;
    Ok([
        (lo[0] + hi[0]) / 2.,
        (lo[1] + hi[1]) / 2.,
        (lo[2] + hi[2]) / 2.,
    ])
}

mod adaptive;
use adaptive::*;


/// Solve the per-cell QEF Σ (n_i·(x − p_i))² → min. Returns `None` when the
/// normal matrix is degenerate (caller falls back to the centroid).
fn qef_solve(samples: &[([f64; 3], [f64; 3])]) -> Option<[f64; 3]> {
    let mut a = [[0.; 3]; 3];
    let mut b = [0.; 3];
    for &(p, n) in samples {
        let d = dot(n, p);
        for i in 0..3 {
            b[i] += n[i] * d;
            for j in 0..3 {
                a[i][j] += n[i] * n[j];
            }
        }
    }
    // Light Tikhonov regularization keeps flat but solvable configurations
    // stable; genuinely degenerate ones still fail the pivot test.
    let trace = a[0][0] + a[1][1] + a[2][2];
    let reg = 1e-10 * trace.max(1e-12);
    for (i, row) in a.iter_mut().enumerate() {
        row[i] += reg;
    }
    solve(a, b)
}

/// Extract an isosurface of `sdf` inside the axis-aligned box `min`–`max`.
///
/// Negative SDF values denote the interior. Returns an empty mesh when the
/// root cell carries no sign change.
pub fn polygonize_sdf(
    min: [f64; 3],
    max: [f64; 3],
    sdf: &dyn Fn([f64; 3]) -> f64,
    gradient: Option<&dyn Fn([f64; 3]) -> [f64; 3]>,
    config: &IsosurfaceConfig,
) -> Result<IsosurfaceOutput> {
    check(
        min.iter().zip(max.iter()).all(|(a, b)| a < b),
        "isosurface domain must satisfy min < max on every axis",
    )?;
    require_finite_point(&min, "isosurface min")?;
    require_finite_point(&max, "isosurface max")?;
    if config.max_depth > HARD_MAX_DEPTH {
        return Err(resource("requested octree depth exceeds the hard cap"));
    }
    check(config.max_cells >= 8, "cell budget must allow at least one split")?;
    check(
        config.linearization_tolerance.is_finite() && config.linearization_tolerance >= 0.,
        "linearization tolerance must be non-negative and finite",
    )?;
    check(
        config.sharp_angle.is_finite() && config.sharp_angle > 0.,
        "sharp angle must be positive and finite",
    )?;
    let extent = (max[0] - min[0]).max(max[1] - min[1]).max(max[2] - min[2]);
    let mut report = IsosurfaceReport::default();
    let mut eval = Evaluator {
        sdf,
        gradient,
        fd_step: 1e-6 * extent,
        evaluations: 0,
    };
    let Some(depth) = adaptive_depth(&mut eval, min, max, config, &mut report)? else {
        return Ok(IsosurfaceOutput::default());
    };
    report.depth_reached = depth;

    // Uniform extraction grid at the adaptively determined depth.
    let n = 1usize << depth;
    if n.checked_pow(3).is_none_or(|c| c > config.max_cells) {
        return Err(resource("uniform extraction grid exceeds the cell budget"));
    }
    let step = [
        (max[0] - min[0]) / n as f64,
        (max[1] - min[1]) / n as f64,
        (max[2] - min[2]) / n as f64,
    ];
    let gv = n + 1;
    let grid_index = |i: usize, j: usize, k: usize| (k * gv + j) * gv + i;
    let grid_point = |i: usize, j: usize, k: usize| {
        [
            min[0] + i as f64 * step[0],
            min[1] + j as f64 * step[1],
            min[2] + k as f64 * step[2],
        ]
    };
    let mut values = vec![0.; gv * gv * gv];
    let mut grid_guard = Budget::with_iterations(config.max_cells.max(gv * gv * gv))?
        .guard("isosurface_grid_evaluation");
    for k in 0..gv {
        for j in 0..gv {
            for i in 0..gv {
                grid_guard.tick()?;
                values[grid_index(i, j, k)] = eval.value(grid_point(i, j, k))?;
            }
        }
    }

    let mut mesh = IsoMesh::default();
    match config.extraction {
        Extraction::DualContouring => extract_dual(
            &mut eval, &values, n, gv, step, grid_point, config, &mut mesh, &mut report,
        )?,
        Extraction::MarchingCubes => {
            extract_marching(&mut eval, &values, n, gv, grid_point, config, &mut mesh)?
        }
    }
    numeric(
        mesh.points.iter().flatten().all(|v| v.is_finite()),
        "extraction produced a non-finite vertex",
    )?;
    let sharp = sharp_edges(&mesh, config.sharp_angle);
    report.sdf_evaluations = eval.evaluations;
    Ok(IsosurfaceOutput {
        mesh,
        sharp_edges: sharp,
        report,
    })
}

/// Per-cell vertex for dual contouring: QEF minimizer clamped to the cell,
/// centroid of edge intersections as the degenerate fallback.
#[allow(clippy::too_many_arguments)]
fn cell_vertex(
    eval: &mut Evaluator,
    corners: [[f64; 3]; 8],
    values: [f64; 8],
    cell_min: [f64; 3],
    cell_max: [f64; 3],
    mode: VertexMode,
    fallbacks: &mut usize,
) -> Result<[f64; 3]> {
    let mut intersections: Vec<[f64; 3]> = Vec::with_capacity(12);
    for &(ca, cb) in &CUBE_EDGES {
        let (va, vb) = (values[ca], values[cb]);
        if va == 0. {
            intersections.push(corners[ca]);
            continue;
        }
        if vb == 0. {
            intersections.push(corners[cb]);
            continue;
        }
        if (va < 0.) == (vb < 0.) {
            continue;
        }
        intersections.push(bisect_root(eval, corners[ca], corners[cb], va, vb)?);
    }
    if intersections.is_empty() {
        return Ok([
            (cell_min[0] + cell_max[0]) / 2.,
            (cell_min[1] + cell_max[1]) / 2.,
            (cell_min[2] + cell_max[2]) / 2.,
        ]);
    }
    let centroid = {
        let mut c = [0.; 3];
        for p in &intersections {
            c[0] += p[0] / intersections.len() as f64;
            c[1] += p[1] / intersections.len() as f64;
            c[2] += p[2] / intersections.len() as f64;
        }
        c
    };
    if mode == VertexMode::Centroid {
        return Ok(centroid);
    }
    let mut samples = Vec::with_capacity(intersections.len());
    for &p in &intersections {
        if let Some(n) = eval.normal(p)? {
            samples.push((p, n));
        }
    }
    if samples.len() < 3 {
        *fallbacks += 1;
        return Ok(centroid);
    }
    match qef_solve(&samples) {
        Some(x) => {
            // Clamp the minimizer to the cell; a wildly escaping solution is
            // treated as degenerate.
            let mut clamped = x;
            let mut escaped = false;
            for axis in 0..3 {
                let size = cell_max[axis] - cell_min[axis];
                if x[axis] < cell_min[axis] - 0.5 * size || x[axis] > cell_max[axis] + 0.5 * size {
                    escaped = true;
                }
                clamped[axis] = x[axis].clamp(cell_min[axis], cell_max[axis]);
            }
            if escaped {
                *fallbacks += 1;
                Ok(centroid)
            } else {
                Ok(clamped)
            }
        }
        None => {
            *fallbacks += 1;
            Ok(centroid)
        }
    }
}

/// Dual contouring on the uniform grid at the extraction depth.
#[allow(clippy::too_many_arguments)]
fn extract_dual(
    eval: &mut Evaluator,
    values: &[f64],
    n: usize,
    gv: usize,
    step: [f64; 3],
    grid_point: impl Fn(usize, usize, usize) -> [f64; 3],
    config: &IsosurfaceConfig,
    mesh: &mut IsoMesh,
    report: &mut IsosurfaceReport,
) -> Result<()> {
    let grid_index = |i: usize, j: usize, k: usize| (k * gv + j) * gv + i;
    let mut guard =
        Budget::with_iterations(config.max_cells.max(n.saturating_pow(3)))?
            .guard("isosurface_dual_contouring");
    // One vertex per sign-changing cell, created in lexicographic cell order.
    let mut cell_vertex_index: BTreeMap<(usize, usize, usize), u32> = BTreeMap::new();
    for k in 0..n {
        for j in 0..n {
            for i in 0..n {
                guard.tick()?;
                let mut cv = [0.; 8];
                let mut cp = [[0.; 3]; 8];
                for (c, off) in CORNER_OFFSET.iter().enumerate() {
                    let (ci, cj, ck) =
                        (i + off[0] as usize, j + off[1] as usize, k + off[2] as usize);
                    cv[c] = values[grid_index(ci, cj, ck)];
                    cp[c] = grid_point(ci, cj, ck);
                }
                let neg = cv[0] < 0.;
                if !cv.iter().any(|&v| (v < 0.) != neg) {
                    continue;
                }
                let cell_min = grid_point(i, j, k);
                let cell_max = [
                    cell_min[0] + step[0],
                    cell_min[1] + step[1],
                    cell_min[2] + step[2],
                ];
                let v = cell_vertex(
                    eval,
                    cp,
                    cv,
                    cell_min,
                    cell_max,
                    config.vertex_mode,
                    &mut report.qef_fallbacks,
                )?;
                cell_vertex_index.insert((i, j, k), mesh.points.len() as u32);
                mesh.points.push(v);
            }
        }
    }
    // Quads across sign-changing grid edges. Loop nest is fixed: edge axis
    // outermost, then (k, j) or the corresponding fixed permutation.
    for axis in 0..3 {
        let (au, av) = ((axis + 1) % 3, (axis + 2) % 3);
        for a in 0..n {
            for b in 0..gv {
                for c in 0..gv {
                    let (i, j, k) = match axis {
                        0 => (a, b, c),
                        1 => (c, a, b),
                        _ => (b, c, a),
                    };
                    let e0 = [i, j, k];
                    let mut e1 = e0;
                    e1[axis] += 1;
                    let v0 = values[grid_index(e0[0], e0[1], e0[2])];
                    let v1 = values[grid_index(e1[0], e1[1], e1[2])];
                    if (v0 < 0.) == (v1 < 0.) {
                        continue;
                    }
                    // The 4 cells sharing this edge, in cyclic order around it
                    // (fixed (du, dv) sequence in the perpendicular plane).
                    let mut quad = [None; 4];
                    for (slot, &(du, dv)) in
                        [(0usize, 0usize), (1, 0), (1, 1), (0, 1)].iter().enumerate()
                    {
                        let mut cell = e0;
                        if du == 1 {
                            if cell[au] == 0 {
                                continue;
                            }
                            cell[au] -= 1;
                        }
                        if dv == 1 {
                            if cell[av] == 0 {
                                continue;
                            }
                            cell[av] -= 1;
                        }
                        if cell[au] < n && cell[av] < n {
                            quad[slot] = cell_vertex_index
                                .get(&(cell[0], cell[1], cell[2]))
                                .copied();
                        }
                    }
                    if let [Some(q0), Some(q1), Some(q2), Some(q3)] = quad {
                        push_triangle(eval, mesh, [q0, q1, q2])?;
                        push_triangle(eval, mesh, [q0, q2, q3])?;
                    }
                }
            }
        }
    }
    check_mesh_budget(mesh)?;
    Ok(())
}

/// Marching cubes via canonical tetrahedral decomposition on the uniform grid.
fn extract_marching(
    eval: &mut Evaluator,
    values: &[f64],
    n: usize,
    gv: usize,
    grid_point: impl Fn(usize, usize, usize) -> [f64; 3],
    config: &IsosurfaceConfig,
    mesh: &mut IsoMesh,
) -> Result<()> {
    let grid_index = |i: usize, j: usize, k: usize| (k * gv + j) * gv + i;
    let mut guard = Budget::with_iterations(config.max_cells.max(n.saturating_pow(3)))?
        .guard("isosurface_marching_cubes");
    // Vertices on grid edges, keyed canonically (axis, min-corner coordinates).
    let mut edge_vertices: BTreeMap<(usize, usize, usize, usize), u32> = BTreeMap::new();
    for k in 0..n {
        for j in 0..n {
            for i in 0..n {
                guard.tick()?;
                let mut cv = [0.; 8];
                let mut cg = [[0usize; 3]; 8];
                for (c, off) in CORNER_OFFSET.iter().enumerate() {
                    cg[c] = [i + off[0] as usize, j + off[1] as usize, k + off[2] as usize];
                    cv[c] = values[grid_index(cg[c][0], cg[c][1], cg[c][2])];
                }
                let neg = cv[0] < 0.;
                if !cv.iter().any(|&v| (v < 0.) != neg) {
                    continue;
                }
                for tet in &CUBE_TETS {
                    let tv = [cv[tet[0]], cv[tet[1]], cv[tet[2]], cv[tet[3]]];
                    let neg_mask: Vec<usize> = (0..4).filter(|&t| tv[t] < 0.).collect();
                    let pos_mask: Vec<usize> = (0..4).filter(|&t| tv[t] >= 0.).collect();
                    let mut connect = |a: usize, b: usize| -> Result<u32> {
                        let g0 = cg[tet[a]];
                        let g1 = cg[tet[b]];
                        let axis = (0..3).find(|&ax| g0[ax] != g1[ax]).unwrap_or(0);
                        let lo = [
                            g0[0].min(g1[0]),
                            g0[1].min(g1[1]),
                            g0[2].min(g1[2]),
                        ];
                        let key = (axis, lo[0], lo[1], lo[2]);
                        if let Some(&idx) = edge_vertices.get(&key) {
                            return Ok(idx);
                        }
                        let p0 = grid_point(g0[0], g0[1], g0[2]);
                        let p1 = grid_point(g1[0], g1[1], g1[2]);
                        let v0 = values[grid_index(g0[0], g0[1], g0[2])];
                        let v1 = values[grid_index(g1[0], g1[1], g1[2])];
                        let p = if v0 == 0. {
                            p0
                        } else if v1 == 0. {
                            p1
                        } else {
                            bisect_root(eval, p0, p1, v0, v1)?
                        };
                        let idx = mesh.points.len() as u32;
                        mesh.points.push(p);
                        edge_vertices.insert(key, idx);
                        Ok(idx)
                    };
                    match (neg_mask.len(), pos_mask.len()) {
                        (1, 3) => {
                            let a = neg_mask[0];
                            let t = [
                                connect(a, pos_mask[0])?,
                                connect(a, pos_mask[1])?,
                                connect(a, pos_mask[2])?,
                            ];
                            push_triangle(eval, mesh, t)?;
                        }
                        (3, 1) => {
                            let a = pos_mask[0];
                            let t = [
                                connect(a, neg_mask[0])?,
                                connect(a, neg_mask[1])?,
                                connect(a, neg_mask[2])?,
                            ];
                            push_triangle(eval, mesh, t)?;
                        }
                        (2, 2) => {
                            let (a, b) = (neg_mask[0], neg_mask[1]);
                            let (c, d) = (pos_mask[0], pos_mask[1]);
                            let (ac, bc) = (connect(a, c)?, connect(b, c)?);
                            let (ad, bd) = (connect(a, d)?, connect(b, d)?);
                            push_triangle(eval, mesh, [ac, bc, bd])?;
                            push_triangle(eval, mesh, [ac, bd, ad])?;
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    check_mesh_budget(mesh)?;
    Ok(())
}

/// Append a triangle, flipping its winding when the geometric normal points
/// against the SDF gradient at its centroid (outward = positive SDF).
fn push_triangle(eval: &mut Evaluator, mesh: &mut IsoMesh, tri: [u32; 3]) -> Result<()> {
    if tri[0] == tri[1] || tri[1] == tri[2] || tri[0] == tri[2] {
        return Ok(());
    }
    let (a, b, c) = (
        mesh.points[tri[0] as usize],
        mesh.points[tri[1] as usize],
        mesh.points[tri[2] as usize],
    );
    let normal = cross(
        [b[0] - a[0], b[1] - a[1], b[2] - a[2]],
        [c[0] - a[0], c[1] - a[1], c[2] - a[2]],
    );
    if norm(normal) < 1e-18 {
        return Ok(());
    }
    let centroid = [
        (a[0] + b[0] + c[0]) / 3.,
        (a[1] + b[1] + c[1]) / 3.,
        (a[2] + b[2] + c[2]) / 3.,
    ];
    let tri = match eval.normal(centroid)? {
        Some(g) if dot(normal, g) < 0. => [tri[0], tri[2], tri[1]],
        _ => tri,
    };
    mesh.triangles.push(tri);
    check_mesh_budget(mesh)
}

fn check_mesh_budget(mesh: &IsoMesh) -> Result<()> {
    if mesh.triangles.len() > MAX_TRIANGLES {
        return Err(resource("triangle budget exhausted during extraction"));
    }
    Ok(())
}

mod mesh_cleanup;
pub use mesh_cleanup::{sharp_edges,SmoothReport,taubin_smooth,laplacian_smooth,CleanupReport,remove_small_components};


#[cfg(test)]
#[path = "tests/isosurface.rs"]
mod tests;
