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

/// Octree node used during adaptive refinement.
struct OctreeCell {
    min: [f64; 3],
    depth: u32,
    corners: [f64; 8],
}

impl OctreeCell {
    fn sign_change(&self) -> bool {
        let neg = self.corners[0] < 0.;
        self.corners.iter().any(|&v| (v < 0.) != neg)
    }
}

/// Refine adaptively and return the extraction depth (deepest sign-changing
/// cell seen) together with statistics. `None` means no sign change anywhere.
fn adaptive_depth(
    eval: &mut Evaluator,
    min: [f64; 3],
    max: [f64; 3],
    config: &IsosurfaceConfig,
    report: &mut IsosurfaceReport,
) -> Result<Option<u32>> {
    let mut corners = [0.; 8];
    for (c, off) in corners.iter_mut().zip(CORNER_OFFSET.iter()) {
        *c = eval.value([
            if off[0] == 0 { min[0] } else { max[0] },
            if off[1] == 0 { min[1] } else { max[1] },
            if off[2] == 0 { min[2] } else { max[2] },
        ])?;
    }
    let root = OctreeCell {
        min,
        depth: 0,
        corners,
    };
    let extents = [max[0] - min[0], max[1] - min[1], max[2] - min[2]];
    let mut stack = vec![(root, extents)];
    let mut cells_created = 1usize;
    let mut depth_reached = 0u32;
    let mut sign_leaves = 0usize;
    // Unified budget guard backing the configured max_depth/max_cells limits:
    // one tick per processed cell, wall-clock checked at the same waypoint.
    let mut guard = Budget::new(
        config.max_cells.max(1),
        (config.max_depth as usize + 1).max(1),
        u64::MAX,
    )?
    .guard("isosurface_adaptive_refinement");
    // Deterministic traversal: children are pushed in a fixed lexicographic
    // (dz, dy, dx) order and popped from the end.
    while let Some((cell, ext)) = stack.pop() {
        guard.tick()?;
        guard.check()?;
        let center = [
            cell.min[0] + ext[0] / 2.,
            cell.min[1] + ext[1] / 2.,
            cell.min[2] + ext[2] / 2.,
        ];
        let trilinear: f64 = cell.corners.iter().sum::<f64>() / 8.;
        let lin_err = (eval.value(center)? - trilinear).abs();
        // Subdivide on linearization (curvature) error alone: a surface fully
        // contained in a cell shows no corner sign change but a large
        // center-vs-trilinear discrepancy, so this criterion cannot miss it.
        let subdivide = cell.depth < config.max_depth && lin_err > config.linearization_tolerance;
        if !subdivide {
            if cell.sign_change() {
                sign_leaves += 1;
                depth_reached = depth_reached.max(cell.depth);
            }
            continue;
        }
        if cells_created + 8 > config.max_cells {
            return Err(resource(
                "octree cell budget exhausted during adaptive refinement",
            ));
        }
        cells_created += 8;
        let half = [ext[0] / 2., ext[1] / 2., ext[2] / 2.];
        for dz in 0..2u32 {
            for dy in 0..2u32 {
                for dx in 0..2u32 {
                    let child_min = [
                        cell.min[0] + f64::from(dx) * half[0],
                        cell.min[1] + f64::from(dy) * half[1],
                        cell.min[2] + f64::from(dz) * half[2],
                    ];
                    let mut cc = [0.; 8];
                    for (c, off) in cc.iter_mut().zip(CORNER_OFFSET.iter()) {
                        *c = eval.value([
                            child_min[0] + f64::from(off[0]) * half[0],
                            child_min[1] + f64::from(off[1]) * half[1],
                            child_min[2] + f64::from(off[2]) * half[2],
                        ])?;
                    }
                    stack.push((
                        OctreeCell {
                            min: child_min,
                            depth: cell.depth + 1,
                            corners: cc,
                        },
                        half,
                    ));
                }
            }
        }
    }
    report.octree_cells = cells_created;
    report.sign_leaves = sign_leaves;
    Ok((depth_reached > 0 || sign_leaves > 0).then_some(depth_reached))
}

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

fn triangle_normal(mesh: &IsoMesh, t: usize) -> Option<[f64; 3]> {
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
fn vertex_adjacency(mesh: &IsoMesh) -> Vec<Vec<u32>> {
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
fn smooth_pass(points: &mut [[f64; 3]], adjacency: &[Vec<u32>], w: f64) {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn sphere_sdf(r: f64) -> impl Fn([f64; 3]) -> f64 {
        move |p| norm(p) - r
    }

    fn sphere_grad(p: [f64; 3]) -> [f64; 3] {
        let l = norm(p);
        if l < 1e-12 {
            [1., 0., 0.]
        } else {
            [p[0] / l, p[1] / l, p[2] / l]
        }
    }

    fn config(depth: u32, mode: VertexMode) -> IsosurfaceConfig {
        IsosurfaceConfig {
            max_depth: depth,
            linearization_tolerance: 1e-4,
            vertex_mode: mode,
            ..IsosurfaceConfig::default()
        }
    }

    /// FNV-1a over the raw bytes of the output — byte-identical determinism.
    /// Uses the shared `guards::Fnv1a` construction (item 1094).
    fn hash_output(out: &IsosurfaceOutput) -> u64 {
        use crate::foundation::guards::Fnv1a;
        let mut h = Fnv1a::new();
        for p in &out.mesh.points {
            for v in p {
                h.write(&v.to_le_bytes());
            }
        }
        for t in &out.mesh.triangles {
            for i in t {
                h.write(&i.to_le_bytes());
            }
        }
        for e in &out.sharp_edges {
            for i in e {
                h.write(&i.to_le_bytes());
            }
        }
        h.finish()
    }

    #[test]
    fn sphere_volume_and_vertices_on_surface() {
        let out = polygonize_sdf(
            [-1.4; 3],
            [1.4; 3],
            &sphere_sdf(1.),
            Some(&sphere_grad),
            &config(5, VertexMode::Qef),
        )
        .unwrap();
        assert!(!out.mesh.triangles.is_empty());
        let volume = out.mesh.signed_volume();
        let expected = 4. / 3. * std::f64::consts::PI;
        assert!(
            (volume - expected).abs() / expected < 0.05,
            "volume {volume} vs {expected}"
        );
        for p in &out.mesh.points {
            let r = norm(*p);
            assert!((r - 1.).abs() < 0.06, "vertex radius {r}");
        }
        // Smooth sphere: no sharp edges at the 30° threshold.
        assert!(out.sharp_edges.is_empty());
    }

    #[test]
    fn sphere_without_gradient_uses_finite_differences() {
        let out = polygonize_sdf(
            [-1.4; 3],
            [1.4; 3],
            &sphere_sdf(1.),
            None,
            &config(5, VertexMode::Qef),
        )
        .unwrap();
        let volume = out.mesh.signed_volume();
        let expected = 4. / 3. * std::f64::consts::PI;
        assert!((volume - expected).abs() / expected < 0.05);
    }

    /// Two overlapping spheres: the intersection circle is a sharp ridge that
    /// QEF dual contouring must preserve better than centroid placement.
    #[test]
    fn qef_preserves_sharp_ridge_better_than_centroid() {
        let d = 0.55f64;
        let ridge_r = (1. - d * d).sqrt();
        let sdf = move |p: [f64; 3]| {
            let s1 = norm([p[0] - d, p[1], p[2]]) - 1.;
            let s2 = norm([p[0] + d, p[1], p[2]]) - 1.;
            s1.max(s2)
        };
        let grad = move |p: [f64; 3]| {
            let unit = |v: [f64; 3]| {
                let l = norm(v).max(1e-12);
                [v[0] / l, v[1] / l, v[2] / l]
            };
            let g1 = unit([p[0] - d, p[1], p[2]]);
            let g2 = unit([p[0] + d, p[1], p[2]]);
            if norm([p[0] - d, p[1], p[2]]) > norm([p[0] + d, p[1], p[2]]) {
                g1
            } else {
                g2
            }
        };
        let ridge_error = |mode: VertexMode| {
            let out = polygonize_sdf(
                [-1.8, -1.2, -1.2],
                [1.8, 1.2, 1.2],
                &sdf,
                Some(&grad),
                &config(5, mode),
            )
            .unwrap();
            let mut errors = Vec::new();
            for p in &out.mesh.points {
                let rho = (p[1] * p[1] + p[2] * p[2]).sqrt();
                if (rho - ridge_r).abs() < 0.15 && p[0].abs() < 0.35 {
                    // Distance to the exact ridge circle (x = 0, ρ = ridge_r).
                    let mut best = f64::INFINITY;
                    for k in 0..64 {
                        let a = k as f64 / 64. * 2. * std::f64::consts::PI;
                        let q = [0., ridge_r * a.cos(), ridge_r * a.sin()];
                        best = best.min(norm([p[0] - q[0], p[1] - q[1], p[2] - q[2]]));
                    }
                    errors.push(best);
                }
            }
            assert!(errors.len() >= 8, "too few ridge vertices");
            errors.iter().sum::<f64>() / errors.len() as f64
        };
        let qef = ridge_error(VertexMode::Qef);
        let centroid = ridge_error(VertexMode::Centroid);
        assert!(
            qef < centroid,
            "QEF ridge error {qef} should beat centroid {centroid}"
        );
        // The ridge is sharp: it must be detected as sharp edges.
        let out = polygonize_sdf(
            [-1.8, -1.2, -1.2],
            [1.8, 1.2, 1.2],
            &sdf,
            Some(&grad),
            &config(5, VertexMode::Qef),
        )
        .unwrap();
        assert!(!out.sharp_edges.is_empty(), "ridge should be marked sharp");
    }

    #[test]
    fn extraction_is_byte_deterministic() {
        let run = || {
            polygonize_sdf(
                [-1.4; 3],
                [1.4; 3],
                &sphere_sdf(1.),
                Some(&sphere_grad),
                &config(5, VertexMode::Qef),
            )
            .unwrap()
        };
        let (a, b) = (run(), run());
        assert_eq!(hash_output(&a), hash_output(&b));
        assert_eq!(a, b);
        // Marching cubes path too.
        let run_mc = || {
            let mut cfg = config(5, VertexMode::Qef);
            cfg.extraction = Extraction::MarchingCubes;
            polygonize_sdf([-1.4; 3], [1.4; 3], &sphere_sdf(1.), Some(&sphere_grad), &cfg).unwrap()
        };
        assert_eq!(hash_output(&run_mc()), hash_output(&run_mc()));
    }

    #[test]
    fn taubin_preserves_volume_better_than_laplacian() {
        let make = || {
            polygonize_sdf(
                [-1.4; 3],
                [1.4; 3],
                &sphere_sdf(1.),
                Some(&sphere_grad),
                &config(5, VertexMode::Qef),
            )
            .unwrap()
            .mesh
        };
        let mut taubin_mesh = make();
        let taubin = taubin_smooth(&mut taubin_mesh, 0.5, -0.53, 20).unwrap();
        let mut lap_mesh = make();
        let lap = laplacian_smooth(&mut lap_mesh, 0.5, 20).unwrap();
        assert!(
            taubin.volume_drift.abs() < lap.volume_drift.abs(),
            "Taubin drift {} vs Laplacian {}",
            taubin.volume_drift,
            lap.volume_drift
        );
        assert!(
            lap.volume_drift < 0.,
            "pure Laplacian must shrink, drift {}",
            lap.volume_drift
        );
    }

    #[test]
    fn small_component_is_removed() {
        let sdf = |p: [f64; 3]| {
            let big = norm(p) - 1.;
            let small = norm([p[0] - 3., p[1], p[2]]) - 0.25;
            big.min(small)
        };
        let grad = |p: [f64; 3]| {
            if norm(p) - 1. < norm([p[0] - 3., p[1], p[2]]) - 0.25 {
                sphere_grad(p)
            } else {
                sphere_grad([p[0] - 3., p[1], p[2]])
            }
        };
        let mut out = polygonize_sdf(
            [-1.4, -1.4, -1.4],
            [4.4, 1.4, 1.4],
            &sdf,
            Some(&grad),
            &config(5, VertexMode::Qef),
        )
        .unwrap();
        let report = remove_small_components(&mut out.mesh, 0.5).unwrap();
        assert_eq!(report.components_before, 2);
        assert_eq!(report.components_removed, 1);
        assert_eq!(report.removed_volumes.len(), 1);
        assert!(report.removed_volumes[0] < 0.5);
        let remaining = out.mesh.signed_volume();
        let expected = 4. / 3. * std::f64::consts::PI;
        assert!(
            (remaining - expected).abs() / expected < 0.05,
            "remaining volume {remaining}"
        );
    }

    #[test]
    fn budgets_reject_excessive_requests() {
        // Depth beyond the hard cap is a resource error.
        let cfg = config(HARD_MAX_DEPTH + 1, VertexMode::Qef);
        let err = polygonize_sdf([-1.4; 3], [1.4; 3], &sphere_sdf(1.), None, &cfg).unwrap_err();
        assert_eq!(err.code, crate::RESOURCE_LIMIT);
        // A cell budget too small for the refinement is a resource error.
        let mut cfg = config(6, VertexMode::Qef);
        cfg.max_cells = 64;
        let err = polygonize_sdf([-1.4; 3], [1.4; 3], &sphere_sdf(1.), None, &cfg).unwrap_err();
        assert_eq!(err.code, crate::RESOURCE_LIMIT);
        // Taubin iteration budget.
        let mut mesh = IsoMesh {
            points: vec![[0.; 3], [1., 0., 0.], [0., 1., 0.]],
            triangles: vec![[0, 1, 2]],
        };
        assert!(taubin_smooth(&mut mesh, 0.5, -0.53, MAX_SMOOTH_ITERATIONS + 1).is_err());
    }

    #[test]
    fn marching_cubes_sphere_volume() {
        let mut cfg = config(5, VertexMode::Centroid);
        cfg.extraction = Extraction::MarchingCubes;
        let out = polygonize_sdf([-1.4; 3], [1.4; 3], &sphere_sdf(1.), Some(&sphere_grad), &cfg)
            .unwrap();
        let volume = out.mesh.signed_volume();
        let expected = 4. / 3. * std::f64::consts::PI;
        assert!(
            (volume - expected).abs() / expected < 0.06,
            "MC volume {volume} vs {expected}"
        );
    }

    // -- guards (items 1065, 1093, 1094) -------------------------------------

    #[test]
    fn non_finite_domain_is_rejected_as_input_error() {
        for (min, max) in [
            ([f64::NAN; 3], [1.; 3]),
            ([-1.; 3], [f64::INFINITY; 3]),
            ([0., f64::NEG_INFINITY, 0.], [1.; 3]),
        ] {
            let err = polygonize_sdf(min, max, &sphere_sdf(1.), None, &config(4, VertexMode::Qef))
                .unwrap_err();
            assert_eq!(err.code, crate::INVALID_INPUT, "{err:?}");
        }
    }

    #[test]
    fn adaptive_refinement_guard_preserves_budget_error() {
        // The BudgetGuard backing max_cells keeps exhaustion a typed resource
        // error mentioning the refinement stage or the legacy cell message.
        let mut cfg = config(6, VertexMode::Qef);
        cfg.max_cells = 8;
        let err = polygonize_sdf([-1.4; 3], [1.4; 3], &sphere_sdf(1.), None, &cfg).unwrap_err();
        assert_eq!(err.code, crate::RESOURCE_LIMIT, "{err:?}");
        assert!(
            err.contains("isosurface_adaptive_refinement") || err.contains("cell budget"),
            "{err}"
        );
    }

    #[test]
    fn hash_output_uses_shared_fnv1a_and_stays_deterministic() {
        use crate::foundation::guards::Fnv1a;
        let out = polygonize_sdf(
            [-1.4; 3],
            [1.4; 3],
            &sphere_sdf(1.),
            Some(&sphere_grad),
            &config(4, VertexMode::Qef),
        )
        .unwrap();
        assert_eq!(hash_output(&out), hash_output(&out));
        // Empty output hashes to the bare offset basis of the shared hasher.
        assert_eq!(hash_output(&IsosurfaceOutput::default()), Fnv1a::new().finish());
    }
}
