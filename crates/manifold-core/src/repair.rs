//! Best-effort mesh repair toward a manifold surface.
//!
//! Two modes:
//!
//! **Conservative** (default, [`repair`]) — only operations that cannot
//! silently destroy geometry intent:
//! 1. **Weld** — merge vertices closer than `epsilon` (exact weld with
//!    `epsilon = 0.0`). The tolerant weld hashes points into a grid of cell
//!    side `epsilon` and searches the 27 neighbouring cells, so near
//!    duplicates straddling a cell boundary still merge; the representative
//!    is always the lowest-index vertex of its cluster (O(n) expected).
//! 2. **Drop degenerates** — remove triangles that collapsed to a point or
//!    segment (after welding, or already degenerate in the input).
//! 3. **Unify orientation** — BFS over the face adjacency graph, flipping
//!    triangles so shared edges are traversed in opposite directions.
//!
//! **Full** ([`repair_with_mode`] with [`RepairMode::Full`]) — additionally:
//! 4. **Split non-manifold edges** — triangles around a 3+-use edge are
//!    grouped into manifold shells; extra shells get duplicated endpoints.
//! 5. **Stitch and fill boundary loops** — boundary walks that touch
//!    themselves (figure-eight / pinched boundaries) are cut into simple
//!    loops at the repeated vertex and filled before pinch splitting, so the
//!    touching vertex is not duplicated.
//! 6. **Split pinched vertices** — each disconnected fan component gets its
//!    own copy of the shared vertex.
//! 7. **Fill boundary loops** — simple boundary loops are triangulated
//!    (ear clipping on the best-fit plane projection) with orientation
//!    opposite to the surface boundary direction.
//!
//! [`repair_with_options`] adds work budgets (typed
//! [`RepairBudgetExhausted`] error carrying a deterministic partial result),
//! hole-filling limits (loop length, area, planarity), duplicate-face
//! removal, outward orientation by signed volume and a bounded fixed-point
//! iteration. What repair never does: remove self-intersections. Leftovers
//! are reported in [`RepairReport::residual`].

use crate::{EdgeKey, ManifoldReport, check, edge_key};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

/// Result of a repair pass.
#[derive(Debug, Clone)]
pub struct RepairOutcome {
    /// Repaired mesh positions (welded; length shrinks when duplicates merge).
    pub positions: Vec<f64>,
    /// Repaired mesh indices (degenerates removed, orientation unified).
    pub indices: Vec<usize>,
    /// For every output triangle, the input triangle it came from; `None`
    /// for triangles added by hole filling.
    pub source_triangles: Vec<Option<usize>>,
    pub report: RepairReport,
}

/// Repair aggressiveness.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RepairMode {
    /// Weld + de-degenerate + orientation unify only.
    #[default]
    Conservative,
    /// Additionally split non-manifold edges, stitch non-simple boundary
    /// loops, split pinched vertices and fill boundary loops.
    Full,
}

/// Work limits for one repair call. `None` means unlimited.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RepairBudget {
    /// Abstract work units (vertex visits, triangle visits, ear-clipping
    /// candidate tests). Deterministic for a given input and options.
    pub max_work: Option<u64>,
    /// Vertices duplicated by non-manifold edge and pinch splitting.
    pub max_split_vertices: Option<usize>,
    /// Triangles added by hole filling.
    pub max_filled_triangles: Option<usize>,
}

/// Limits on which boundary loops hole filling may close. Loops outside the
/// limits are left open and counted in [`RepairReport::refused_holes`].
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct HoleFillLimits {
    /// Maximum number of loop vertices.
    pub max_loop_vertices: Option<usize>,
    /// Maximum loop area (magnitude of the Newell vector area).
    pub max_area: Option<f64>,
    /// Maximum distance of a loop vertex from the loop's best-fit plane
    /// (Newell normal through the centroid), in mesh units.
    pub max_plane_deviation: Option<f64>,
}

/// Full repair configuration for [`repair_with_options`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RepairOptions {
    pub mode: RepairMode,
    /// Weld tolerance in mesh units (`0.0` = exact bitwise weld; negative is
    /// treated as `0.0`).
    pub epsilon: f64,
    pub budget: RepairBudget,
    pub hole_limits: HoleFillLimits,
    /// Cut self-touching boundary walks into simple loops and fill them
    /// (Full mode only).
    pub stitch_non_simple_loops: bool,
    /// Remove repeated triangles (same vertices, same orientation keep one;
    /// an opposite-orientation pair cancels and both are removed).
    pub remove_duplicate_faces: bool,
    /// Flip closed components with negative signed volume so normals point
    /// outward.
    pub orient_outward: bool,
    /// Upper bound on repeated passes until the mesh stops changing
    /// (`1` = single pass).
    pub max_iterations: usize,
}

impl Default for RepairOptions {
    fn default() -> Self {
        Self {
            mode: RepairMode::Conservative,
            epsilon: 0.0,
            budget: RepairBudget::default(),
            hole_limits: HoleFillLimits::default(),
            stitch_non_simple_loops: true,
            remove_duplicate_faces: false,
            orient_outward: false,
            max_iterations: 1,
        }
    }
}

/// Which budget ran out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepairBudgetKind {
    Work,
    SplitVertices,
    FilledTriangles,
}

/// A budget ran out. `partial` is the mesh after the last completed atomic
/// step, with its residual report; it is identical across runs.
#[derive(Debug, Clone)]
pub struct RepairBudgetExhausted {
    pub kind: RepairBudgetKind,
    pub partial: Box<RepairOutcome>,
}

impl std::fmt::Display for RepairBudgetExhausted {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "repair budget exhausted: {:?}", self.kind)
    }
}

impl std::error::Error for RepairBudgetExhausted {}

/// What repair changed and what it could not fix.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RepairReport {
    /// Vertices merged by welding.
    pub welded_vertices: usize,
    /// Triangles removed as degenerate (input or post-weld).
    pub removed_degenerate_triangles: usize,
    /// Triangles removed as duplicates (`remove_duplicate_faces`).
    pub removed_duplicate_triangles: usize,
    /// Triangles flipped to unify orientation (and outward orientation).
    pub flipped_triangles: usize,
    /// Closed components flipped because their signed volume was negative.
    pub inverted_components: usize,
    /// Vertices duplicated when splitting non-manifold edge shells and
    /// pinched fans (Full mode; 0 in Conservative).
    pub split_vertices: usize,
    /// Simple loops cut out of self-touching boundary walks.
    pub stitched_loops: usize,
    /// Boundary loops triangulated (Full mode; 0 in Conservative).
    pub filled_holes: usize,
    /// Triangles added by hole filling (Full mode; 0 in Conservative).
    pub filled_triangles: usize,
    /// Boundary loops left open because they exceed [`HoleFillLimits`].
    pub refused_holes: usize,
    /// Work units spent (see [`RepairBudget::max_work`]).
    pub work: u64,
    /// Passes run.
    pub iterations: usize,
    /// The last pass changed nothing (fixed point reached).
    pub converged: bool,
    /// A later pass reproduced an earlier mesh state.
    pub oscillation_detected: bool,
    /// Manifoldness issues that repair does not attempt: residual report
    /// computed on the repaired mesh.
    pub residual: ManifoldReport,
}

impl RepairOutcome {
    /// Strict 2-manifold after repair.
    pub fn is_manifold(&self) -> bool {
        self.report.residual.is_manifold()
    }
}

/// Weld + de-degenerate + orientation-unify `positions`/`indices`
/// (Conservative mode; see [`repair_with_mode`]).
///
/// `epsilon` is the weld tolerance in mesh units; use `0.0` for an exact
/// bitwise weld. Negative epsilon is treated as `0.0`.
pub fn repair(positions: &[f64], indices: &[usize], epsilon: f64) -> RepairOutcome {
    repair_with_mode(positions, indices, epsilon, RepairMode::Conservative)
}

/// Repair `positions`/`indices` with the given aggressiveness and no budget.
///
/// `epsilon` is the weld tolerance in mesh units; use `0.0` for an exact
/// bitwise weld. Negative epsilon is treated as `0.0`.
pub fn repair_with_mode(
    positions: &[f64],
    indices: &[usize],
    epsilon: f64,
    mode: RepairMode,
) -> RepairOutcome {
    let options = RepairOptions {
        mode,
        epsilon,
        ..RepairOptions::default()
    };
    match repair_with_options(positions, indices, &options) {
        Ok(out) => out,
        Err(exhausted) => *exhausted.partial, // unreachable: no limits set
    }
}

/// Repair with explicit options. Returns [`RepairBudgetExhausted`] (with a
/// deterministic partial result) when a budget limit is hit.
pub fn repair_with_options(
    positions: &[f64],
    indices: &[usize],
    options: &RepairOptions,
) -> Result<RepairOutcome, RepairBudgetExhausted> {
    let mut budget = Budget::new(options.budget);
    let max_iterations = options.max_iterations.max(1);
    let mut out = repair_pass(positions, indices, options, &mut budget);
    let mut report = out.report.clone();
    report.iterations = 1;
    let mut seen: Vec<(Vec<u64>, Vec<usize>)> = vec![state_key(positions, indices)];
    let mut converged = state_key(&out.positions, &out.indices) == seen[0];
    while !converged && budget.exhausted.is_none() && report.iterations < max_iterations {
        let key = state_key(&out.positions, &out.indices);
        seen.push(key);
        let next = repair_pass(&out.positions, &out.indices, options, &mut budget);
        let next_key = state_key(&next.positions, &next.indices);
        report.iterations += 1;
        accumulate(&mut report, &next.report);
        let source = next
            .source_triangles
            .iter()
            .map(|s| s.and_then(|j| out.source_triangles[j]))
            .collect();
        converged = next_key == *seen.last().unwrap();
        if !converged && seen.contains(&next_key) {
            report.oscillation_detected = true;
        }
        out = RepairOutcome {
            source_triangles: source,
            ..next
        };
        if report.oscillation_detected {
            break;
        }
    }
    report.converged = converged;
    report.work = budget.work;
    report.residual = out.report.residual.clone();
    out.report = report;
    match budget.exhausted {
        Some(kind) => Err(RepairBudgetExhausted {
            kind,
            partial: Box::new(out),
        }),
        None => Ok(out),
    }
}

fn state_key(positions: &[f64], indices: &[usize]) -> (Vec<u64>, Vec<usize>) {
    (
        positions.iter().map(|x| x.to_bits()).collect(),
        indices.to_vec(),
    )
}

fn accumulate(total: &mut RepairReport, pass: &RepairReport) {
    total.welded_vertices += pass.welded_vertices;
    total.removed_degenerate_triangles += pass.removed_degenerate_triangles;
    total.removed_duplicate_triangles += pass.removed_duplicate_triangles;
    total.flipped_triangles += pass.flipped_triangles;
    total.inverted_components += pass.inverted_components;
    total.split_vertices += pass.split_vertices;
    total.stitched_loops += pass.stitched_loops;
    total.filled_holes += pass.filled_holes;
    total.filled_triangles += pass.filled_triangles;
    total.refused_holes = pass.refused_holes;
}

/// Running budget state shared by all passes of one repair call.
struct Budget {
    limits: RepairBudget,
    work: u64,
    split_vertices: usize,
    filled_triangles: usize,
    exhausted: Option<RepairBudgetKind>,
}

impl Budget {
    fn new(limits: RepairBudget) -> Self {
        Self {
            limits,
            work: 0,
            split_vertices: 0,
            filled_triangles: 0,
            exhausted: None,
        }
    }

    fn ok(&self) -> bool {
        self.exhausted.is_none()
    }

    /// Spend `units` of work; on overflow mark exhaustion and spend nothing.
    fn charge(&mut self, units: u64) -> bool {
        if !self.ok() {
            return false;
        }
        let next = self.work.saturating_add(units);
        if self.limits.max_work.is_some_and(|max| next > max) {
            self.exhausted = Some(RepairBudgetKind::Work);
            return false;
        }
        self.work = next;
        true
    }

    fn take_split(&mut self, n: usize) -> bool {
        if !self.ok() {
            return false;
        }
        if self
            .limits
            .max_split_vertices
            .is_some_and(|max| self.split_vertices + n > max)
        {
            self.exhausted = Some(RepairBudgetKind::SplitVertices);
            return false;
        }
        self.split_vertices += n;
        true
    }

    fn take_fill(&mut self, n: usize) -> bool {
        if !self.ok() {
            return false;
        }
        if self
            .limits
            .max_filled_triangles
            .is_some_and(|max| self.filled_triangles + n > max)
        {
            self.exhausted = Some(RepairBudgetKind::FilledTriangles);
            return false;
        }
        self.filled_triangles += n;
        true
    }
}

/// Triangle soup with per-triangle source ids.
struct Tris {
    tris: Vec<[usize; 3]>,
    source: Vec<Option<usize>>,
}

fn repair_pass(
    positions: &[f64],
    indices: &[usize],
    options: &RepairOptions,
    budget: &mut Budget,
) -> RepairOutcome {
    let epsilon = options.epsilon.max(0.0);
    let mut report = RepairReport::default();
    let vertex_count = positions.len() / 3;

    // --- 1. Weld vertices -------------------------------------------------
    let (mut pos, remap) = if budget.charge(vertex_count as u64) {
        let (p, r, merged) = weld(positions, epsilon);
        report.welded_vertices = merged;
        (p, r)
    } else {
        (positions.to_vec(), (0..vertex_count).collect())
    };

    // --- 2. Remap indices, drop degenerates -------------------------------
    let mut mesh = Tris {
        tris: Vec::with_capacity(indices.len() / 3),
        source: Vec::with_capacity(indices.len() / 3),
    };
    for (ti, t) in indices.as_chunks::<3>().0.iter().enumerate() {
        let tri = [remap[t[0]], remap[t[1]], remap[t[2]]];
        if tri[0] == tri[1] || tri[1] == tri[2] || tri[0] == tri[2] {
            report.removed_degenerate_triangles += 1;
            continue;
        }
        mesh.tris.push(tri);
        mesh.source.push(Some(ti));
    }

    // --- 2b. Duplicate faces ----------------------------------------------
    if options.remove_duplicate_faces && budget.charge(mesh.tris.len() as u64) {
        report.removed_duplicate_triangles = remove_duplicate_faces(&mut mesh);
    }

    // --- 3. Unify orientation ---------------------------------------------
    if budget.charge(mesh.tris.len() as u64) {
        report.flipped_triangles = unify_orientation(&mut mesh.tris);
    }

    // --- 4-7. Full mode: splits + stitching + hole filling ----------------
    if options.mode == RepairMode::Full {
        report.split_vertices += split_non_manifold_edges(&mut pos, &mut mesh.tris, budget);
        if options.stitch_non_simple_loops {
            fill_boundary_loops(&pos, &mut mesh, options, true, budget, &mut report);
        }
        let pinched = split_pinched_vertices(&mut pos, &mut mesh.tris, budget);
        report.split_vertices += pinched;
        fill_boundary_loops(&pos, &mut mesh, options, false, budget, &mut report);
    }

    // --- 8. Outward orientation -------------------------------------------
    if options.orient_outward && budget.charge(mesh.tris.len() as u64) {
        let (components, flipped) = orient_outward(&pos, &mut mesh.tris);
        report.inverted_components = components;
        report.flipped_triangles += flipped;
    }

    let mut flat_indices = Vec::with_capacity(mesh.tris.len() * 3);
    for t in &mesh.tris {
        flat_indices.extend_from_slice(t);
    }
    report.residual = check(&pos, &flat_indices);
    RepairOutcome {
        positions: pos,
        indices: flat_indices,
        source_triangles: mesh.source,
        report,
    }
}

/// Weld vertices within `epsilon`. Returns (positions, remap, merged).
///
/// Every vertex joins the lowest-index earlier representative within
/// `epsilon` (searching the 27 neighbouring grid cells), otherwise becomes a
/// representative itself. Representatives keep their original coordinates,
/// so the result is independent of hash order and never drifts by chaining.
fn weld(positions: &[f64], epsilon: f64) -> (Vec<f64>, Vec<usize>, usize) {
    let vertex_count = positions.len() / 3;
    let mut remap: Vec<usize> = (0..vertex_count).collect();
    let mut out: Vec<f64> = Vec::with_capacity(positions.len());

    if epsilon == 0.0 {
        // Exact weld on bit patterns.
        let mut canonical: HashMap<[u64; 3], usize> = HashMap::new();
        for v in 0..vertex_count {
            let key = [
                positions[v * 3].to_bits(),
                positions[v * 3 + 1].to_bits(),
                positions[v * 3 + 2].to_bits(),
            ];
            let next = out.len() / 3;
            let id = *canonical.entry(key).or_insert_with(|| {
                out.extend_from_slice(&positions[v * 3..v * 3 + 3]);
                next
            });
            remap[v] = id;
        }
    } else {
        let inv = 1.0 / epsilon;
        let eps2 = epsilon * epsilon;
        let cell_of = |p: &[f64]| -> (i64, i64, i64) {
            (
                (p[0] * inv).floor() as i64,
                (p[1] * inv).floor() as i64,
                (p[2] * inv).floor() as i64,
            )
        };
        // Cell -> representative output ids, in creation order.
        let mut grid: HashMap<(i64, i64, i64), Vec<usize>> = HashMap::new();
        for v in 0..vertex_count {
            let p = &positions[v * 3..v * 3 + 3];
            let c = cell_of(p);
            let mut best: Option<usize> = None;
            for dx in -1..=1 {
                for dy in -1..=1 {
                    for dz in -1..=1 {
                        let Some(bucket) = grid.get(&(c.0 + dx, c.1 + dy, c.2 + dz)) else {
                            continue;
                        };
                        for &u in bucket {
                            let q = &out[u * 3..u * 3 + 3];
                            let d2 = (p[0] - q[0]).powi(2)
                                + (p[1] - q[1]).powi(2)
                                + (p[2] - q[2]).powi(2);
                            if d2 <= eps2 && best.is_none_or(|b| u < b) {
                                best = Some(u);
                            }
                        }
                    }
                }
            }
            remap[v] = match best {
                Some(u) => u,
                None => {
                    let id = out.len() / 3;
                    out.extend_from_slice(p);
                    grid.entry(c).or_default().push(id);
                    id
                }
            };
        }
    }
    let merged = vertex_count - out.len() / 3;
    (out, remap, merged)
}

/// Remove repeated triangles. Same cyclic vertex order: keep the first.
/// Opposite orientation: the pair cancels (both removed). Returns the number
/// of removed triangles.
fn remove_duplicate_faces(mesh: &mut Tris) -> usize {
    // Canonical rotation (smallest vertex first) keeps orientation.
    let canon = |t: [usize; 3]| -> [usize; 3] {
        let r = (0..3).min_by_key(|&i| t[i]).unwrap();
        [t[r], t[(r + 1) % 3], t[(r + 2) % 3]]
    };
    let mut keep = vec![true; mesh.tris.len()];
    // Oriented key -> indices of live triangles with that key.
    let mut live: HashMap<[usize; 3], Vec<usize>> = HashMap::new();
    for (ti, &t) in mesh.tris.iter().enumerate() {
        let k = canon(t);
        let rev = canon([t[0], t[2], t[1]]);
        if live.get(&k).is_some_and(|v| !v.is_empty()) {
            keep[ti] = false; // exact duplicate
            continue;
        }
        if let Some(other) = live.get_mut(&rev).and_then(|v| v.pop()) {
            keep[ti] = false; // cancels an opposite twin
            keep[other] = false;
            continue;
        }
        live.entry(k).or_default().push(ti);
    }
    let removed = keep.iter().filter(|k| !**k).count();
    let mut i = 0;
    mesh.tris.retain(|_| {
        i += 1;
        keep[i - 1]
    });
    let mut i = 0;
    mesh.source.retain(|_| {
        i += 1;
        keep[i - 1]
    });
    removed
}

/// Flip every closed component (all edges used exactly twice within it) whose
/// signed volume is negative. Returns (flipped components, flipped triangles).
fn orient_outward(positions: &[f64], triangles: &mut [[usize; 3]]) -> (usize, usize) {
    let n = triangles.len();
    let mut parent: Vec<usize> = (0..n).collect();
    fn find(parent: &mut [usize], x: usize) -> usize {
        let mut r = x;
        while parent[r] != r {
            r = parent[r];
        }
        let mut c = x;
        while parent[c] != r {
            let next = parent[c];
            parent[c] = r;
            c = next;
        }
        r
    }
    let mut edge_tris: BTreeMap<EdgeKey, Vec<usize>> = BTreeMap::new();
    for (ti, t) in triangles.iter().enumerate() {
        for i in 0..3 {
            edge_tris
                .entry(edge_key(t[i], t[(i + 1) % 3]))
                .or_default()
                .push(ti);
        }
    }
    let mut open_root: HashSet<usize> = HashSet::new();
    let mut open_tris: Vec<usize> = Vec::new();
    for tris in edge_tris.values() {
        for w in tris.windows(2) {
            let (a, b) = (find(&mut parent, w[0]), find(&mut parent, w[1]));
            if a != b {
                parent[a] = b;
            }
        }
        if tris.len() != 2 {
            open_tris.extend_from_slice(tris);
        }
    }
    for t in open_tris {
        open_root.insert(find(&mut parent, t));
    }
    let mut volume: BTreeMap<usize, f64> = BTreeMap::new();
    for ti in 0..n {
        let root = find(&mut parent, ti);
        let p = |v: usize| [positions[v * 3], positions[v * 3 + 1], positions[v * 3 + 2]];
        let [a, b, c] = triangles[ti].map(p);
        let det = a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0])
            + a[2] * (b[0] * c[1] - b[1] * c[0]);
        *volume.entry(root).or_default() += det / 6.0;
    }
    let inverted: HashSet<usize> = volume
        .iter()
        .filter(|(r, v)| **v < 0.0 && !open_root.contains(r))
        .map(|(r, _)| *r)
        .collect();
    let mut flipped = 0;
    for ti in 0..n {
        if inverted.contains(&find(&mut parent, ti)) {
            triangles[ti].swap(1, 2);
            flipped += 1;
        }
    }
    (inverted.len(), flipped)
}

/// Flip triangles so every edge shared by exactly two triangles is
/// traversed in opposite directions. Returns the flip count.
///
/// BFS over the dual graph: an edge used twice imposes a constraint
/// between its two triangles; single-use (boundary) and 3+-use edges
/// impose none. Each connected component is oriented from an arbitrary
/// seed triangle; conflicting components cannot exist because constraints
/// are only checked, never forced.
fn unify_orientation(triangles: &mut [[usize; 3]]) -> usize {
    // Edge -> incident (triangle, direction of traversal: true if a->b
    // with a < b... store actual (from, to) of the directed half-edge).
    // Ordered edges make adjacency insertion and BFS traversal reproducible.
    let mut half_edges: BTreeMap<EdgeKey, Vec<(usize, usize, usize)>> = BTreeMap::new();
    for (ti, t) in triangles.iter().enumerate() {
        for i in 0..3 {
            let (a, b) = (t[i], t[(i + 1) % 3]);
            half_edges
                .entry(edge_key(a, b))
                .or_default()
                .push((ti, a, b));
        }
    }
    // Dual adjacency with the constraint: for edge used by triangles
    // (t1, a, b) and (t2, c, d), orientations agree iff the directed
    // half-edges are opposite, i.e. a == d && b == c.
    let mut dual: HashMap<usize, Vec<(usize, bool)>> = HashMap::new();
    for uses in half_edges.values() {
        if uses.len() != 2 {
            continue;
        }
        let (t1, a, b) = uses[0];
        let (t2, c, d) = uses[1];
        let agree = a == d && b == c;
        dual.entry(t1).or_default().push((t2, agree));
        dual.entry(t2).or_default().push((t1, agree));
    }

    // BFS assigning desired flip parity per component.
    let mut flip = vec![false; triangles.len()];
    let mut visited = vec![false; triangles.len()];
    let mut stack = Vec::new();
    let mut flipped = 0usize;
    for seed in 0..triangles.len() {
        if visited[seed] {
            continue;
        }
        visited[seed] = true;
        stack.push((seed, false));
        while let Some((t, parity)) = stack.pop() {
            if flip[t] != parity && visited[t] && t != seed {
                // Constraint conflict within a component (e.g. mobius-like
                // configuration). Keep first assignment; residual report
                // will flag the edge.
                continue;
            }
            flip[t] = parity;
            visited[t] = true;
            if let Some(neighbors) = dual.get(&t) {
                for &(n, agree) in neighbors {
                    // If we flip t, neighbor must flip too to keep the
                    // (dis)agreement.
                    let n_parity = if agree { parity } else { !parity };
                    if !visited[n] {
                        stack.push((n, n_parity));
                    }
                }
            }
        }
    }
    for (t, tri) in triangles.iter_mut().enumerate() {
        if flip[t] {
            tri.swap(1, 2);
            flipped += 1;
        }
    }
    flipped
}

// ---------------------------------------------------------------------------
// Full mode stages
// ---------------------------------------------------------------------------

/// Append a copy of vertex `v` to `positions`; returns the new vertex id.
fn duplicate_vertex(positions: &mut Vec<f64>, v: usize) -> usize {
    let point = [positions[v * 3], positions[v * 3 + 1], positions[v * 3 + 2]];
    positions.extend_from_slice(&point);
    positions.len() / 3 - 1
}

/// Group the triangles incident to a non-manifold edge (3+ uses) into
/// manifold shells: triangles connected through any *other* shared edge
/// belong to the same shell. Every shell beyond the first receives its own
/// copies of the edge endpoints. Returns the number of duplicated vertices.
/// Stops (atomically per shell) when the budget runs out.
fn split_non_manifold_edges(
    positions: &mut Vec<f64>,
    triangles: &mut [[usize; 3]],
    budget: &mut Budget,
) -> usize {
    if !budget.charge(triangles.len() as u64) {
        return 0;
    }
    // Ordered traversal keeps shell ownership and new vertex IDs stable.
    let mut edge_tris: BTreeMap<EdgeKey, Vec<usize>> = BTreeMap::new();
    for (ti, t) in triangles.iter().enumerate() {
        for i in 0..3 {
            edge_tris
                .entry(edge_key(t[i], t[(i + 1) % 3]))
                .or_default()
                .push(ti);
        }
    }
    let mut duplicated = 0usize;
    for (&edge, tris) in &edge_tris {
        if tris.len() <= 2 {
            continue;
        }
        if !budget.charge(tris.len() as u64) {
            return duplicated;
        }
        // Union-find over incident triangles, joined by shared edges other
        // than `edge` itself.
        let mut parent: BTreeMap<usize, usize> = tris.iter().map(|&t| (t, t)).collect();
        fn find(parent: &mut BTreeMap<usize, usize>, x: usize) -> usize {
            let mut r = x;
            while parent[&r] != r {
                r = parent[&r];
            }
            let mut c = x;
            while parent[&c] != r {
                let n = parent[&c];
                parent.insert(c, r);
                c = n;
            }
            r
        }
        let tri_set: BTreeSet<usize> = tris.iter().copied().collect();
        let mut link_owner: BTreeMap<EdgeKey, usize> = BTreeMap::new();
        for &ti in &tri_set {
            for i in 0..3 {
                let key = edge_key(triangles[ti][i], triangles[ti][(i + 1) % 3]);
                if key == edge {
                    continue;
                }
                if let Some(&other) = link_owner.get(&key) {
                    if tri_set.contains(&other) {
                        let (ra, rb) = (find(&mut parent, other), find(&mut parent, ti));
                        if ra != rb {
                            parent.insert(ra, rb);
                        }
                    }
                } else {
                    link_owner.insert(key, ti);
                }
            }
        }
        // Group triangle ids by shell root, preserving first-shell-wins order.
        let mut shells: Vec<(usize, Vec<usize>)> = Vec::new();
        for &ti in tris {
            let root = find(&mut parent, ti);
            match shells.iter_mut().find(|(r, _)| *r == root) {
                Some((_, group)) => group.push(ti),
                None => shells.push((root, vec![ti])),
            }
        }
        // Shells beyond the first get private copies of both endpoints.
        for (_, group) in shells.iter().skip(1) {
            if !budget.take_split(2) {
                return duplicated;
            }
            let (new_a, new_b) = (
                duplicate_vertex(positions, edge.0),
                duplicate_vertex(positions, edge.1),
            );
            duplicated += 2;
            for &ti in group {
                for corner in triangles[ti].iter_mut() {
                    if *corner == edge.0 {
                        *corner = new_a;
                    } else if *corner == edge.1 {
                        *corner = new_b;
                    }
                }
            }
        }
    }
    duplicated
}

/// Give every disconnected fan component its own copy of the shared vertex.
/// Returns the number of duplicated vertices. Stops when the budget runs out
/// (the current vertex keeps its remaining corners).
fn split_pinched_vertices(
    positions: &mut Vec<f64>,
    triangles: &mut [[usize; 3]],
    budget: &mut Budget,
) -> usize {
    if !budget.charge(triangles.len() as u64 * 3) {
        return 0;
    }
    // Corners per vertex: (triangle, slot, next, prev) link edge endpoints.
    let vertex_count = positions.len() / 3;
    let mut corners: Vec<Vec<(usize, usize, usize, usize)>> = vec![Vec::new(); vertex_count];
    for (ti, t) in triangles.iter().enumerate() {
        for slot in 0..3 {
            corners[t[slot]].push((ti, slot, t[(slot + 1) % 3], t[(slot + 2) % 3]));
        }
    }
    let mut duplicated = 0usize;
    for (v, list) in corners.iter().enumerate() {
        if list.len() < 2 {
            continue;
        }
        // Union corners whose link edges share a node.
        let mut parent: Vec<usize> = (0..list.len()).collect();
        fn find(parent: &mut [usize], x: usize) -> usize {
            let mut r = x;
            while parent[r] != r {
                r = parent[r];
            }
            let mut c = x;
            while parent[c] != r {
                let n = parent[c];
                parent[c] = r;
                c = n;
            }
            r
        }
        let mut node_owner: HashMap<usize, usize> = HashMap::new();
        for (ci, &(_, _, next, prev)) in list.iter().enumerate() {
            for node in [next, prev] {
                if let Some(&other) = node_owner.get(&node) {
                    let (ra, rb) = (find(&mut parent, other), find(&mut parent, ci));
                    if ra != rb {
                        parent[ra] = rb;
                    }
                } else {
                    node_owner.insert(node, ci);
                }
            }
        }
        // Components beyond the first get a private vertex copy.
        let mut seen_roots: HashMap<usize, usize> = HashMap::new();
        for (ci, &(ti, slot, _, _)) in list.iter().enumerate() {
            let root = find(&mut parent, ci);
            if seen_roots.is_empty() {
                seen_roots.insert(root, v);
            }
            let target = match seen_roots.get(&root) {
                Some(&t) => t,
                None => {
                    if !budget.take_split(1) {
                        return duplicated;
                    }
                    let copy = duplicate_vertex(positions, v);
                    duplicated += 1;
                    seen_roots.insert(root, copy);
                    copy
                }
            };
            if target != v {
                triangles[ti][slot] = target;
            }
        }
    }
    duplicated
}

/// Extract simple directed boundary loops. Self-touching walks are cut into
/// simple loops at the repeated vertex; the flag says whether the loop passes
/// through a vertex with several outgoing boundary edges (a pinched
/// boundary).
fn boundary_loops(triangles: &[[usize; 3]]) -> Vec<(Vec<usize>, bool)> {
    let flat: Vec<usize> = triangles.iter().flatten().copied().collect();
    let uses = mesh_topology::EdgeUses::new(&flat);
    let mut out_edges: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for [a, b] in uses.boundary() {
        out_edges.entry(a).or_default().push(b);
    }
    for targets in out_edges.values_mut() {
        targets.sort_unstable();
    }
    let mut cursor: BTreeMap<usize, usize> = out_edges.keys().map(|&v| (v, 0)).collect();
    let mut loops = Vec::new();
    let starts: Vec<usize> = out_edges.keys().copied().collect();
    for start in starts {
        loop {
            // Walk from `start`; cut a simple cycle whenever a vertex repeats.
            let mut path = vec![start];
            let mut on_path: HashMap<usize, usize> = HashMap::from([(start, 0)]);
            let mut cur = start;
            let mut walked = false;
            loop {
                let c = cursor.get_mut(&cur);
                let Some(n) = c.and_then(|c| {
                    let t = out_edges[&cur].get(*c).copied();
                    *c += 1;
                    t
                }) else {
                    break;
                };
                walked = true;
                if let Some(&pos) = on_path.get(&n) {
                    let cycle: Vec<usize> = path[pos..].to_vec();
                    for v in &cycle[1..] {
                        on_path.remove(v);
                    }
                    path.truncate(pos + 1);
                    let touches = cycle.iter().any(|v| out_edges[v].len() > 1);
                    if cycle.len() >= 3 {
                        loops.push((cycle, touches));
                    }
                } else {
                    on_path.insert(n, path.len());
                    path.push(n);
                }
                cur = n;
            }
            if !walked {
                break;
            }
        }
    }
    loops
}

/// Newell vector of a loop (twice the vector area).
fn newell(positions: &[f64], loop_vs: &[usize]) -> [f64; 3] {
    let mut n = [0.0f64; 3];
    for i in 0..loop_vs.len() {
        let p = &positions[loop_vs[i] * 3..loop_vs[i] * 3 + 3];
        let j = loop_vs[(i + 1) % loop_vs.len()];
        let q = &positions[j * 3..j * 3 + 3];
        n[0] += (p[1] - q[1]) * (p[2] + q[2]);
        n[1] += (p[2] - q[2]) * (p[0] + q[0]);
        n[2] += (p[0] - q[0]) * (p[1] + q[1]);
    }
    n
}

/// Whether a loop is within the hole-filling limits.
fn within_limits(positions: &[f64], loop_vs: &[usize], limits: &HoleFillLimits) -> bool {
    if limits
        .max_loop_vertices
        .is_some_and(|max| loop_vs.len() > max)
    {
        return false;
    }
    let n = newell(positions, loop_vs);
    let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
    if limits.max_area.is_some_and(|max| !(len / 2.0 <= max)) {
        return false;
    }
    if let Some(max) = limits.max_plane_deviation {
        if !(len > 0.0) {
            return false;
        }
        let k = loop_vs.len() as f64;
        let mut c = [0.0f64; 3];
        for &v in loop_vs {
            for (a, ca) in c.iter_mut().enumerate() {
                *ca += positions[v * 3 + a] / k;
            }
        }
        let deviation = loop_vs
            .iter()
            .map(|&v| {
                (0..3)
                    .map(|a| (positions[v * 3 + a] - c[a]) * n[a] / len)
                    .sum::<f64>()
                    .abs()
            })
            .fold(0.0f64, f64::max);
        if !(deviation <= max) {
            return false;
        }
    }
    true
}

/// Triangulate boundary loops and append the fill triangles. With
/// `stitch_pass`, only loops cut out of self-touching walks are filled;
/// otherwise only loops that never touch a pinched boundary vertex. Loops
/// beyond [`HoleFillLimits`] are refused; fills whose diagonals already
/// exist in the mesh are skipped.
fn fill_boundary_loops(
    positions: &[f64],
    mesh: &mut Tris,
    options: &RepairOptions,
    stitch_pass: bool,
    budget: &mut Budget,
    report: &mut RepairReport,
) {
    if !budget.charge(mesh.tris.len() as u64) {
        return;
    }
    let loops: Vec<Vec<usize>> = boundary_loops(&mesh.tris)
        .into_iter()
        .filter(|(_, touches)| *touches == stitch_pass)
        .map(|(l, _)| l)
        .collect();
    if loops.is_empty() {
        return;
    }
    if stitch_pass {
        report.stitched_loops += loops.len();
    }
    // Hole loops wind opposite to the exterior contour on a consistently
    // oriented surface. Filling the exterior contour would double-cover the
    // sheet (and clash with interior diagonals), so with several loops only
    // the ones wound against the largest-area loop are filled. A single
    // loop is always filled: it is the hole by definition.
    let areas: Vec<f64> = loops
        .iter()
        .map(|l| loop_signed_area(positions, l))
        .collect();
    let exterior = areas
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.abs().total_cmp(&b.1.abs()))
        .map(|(i, _)| i)
        .unwrap();
    let mut edges: HashSet<EdgeKey> = mesh
        .tris
        .iter()
        .flat_map(|t| (0..3).map(move |i| edge_key(t[i], t[(i + 1) % 3])))
        .collect();
    for (li, loop_vs) in loops.iter().enumerate() {
        if loops.len() > 1 && (li == exterior || areas[li] * areas[exterior] > 0.0) {
            continue; // exterior contour, or another sheet's outside
        }
        if !within_limits(positions, loop_vs, &options.hole_limits) {
            report.refused_holes += 1;
            continue;
        }
        let Some(tris) = triangulate_loop(positions, loop_vs, budget) else {
            if !budget.ok() {
                return;
            }
            continue;
        };
        // A diagonal that already exists would become a 3-use edge.
        let boundary: HashSet<EdgeKey> = (0..loop_vs.len())
            .map(|k| edge_key(loop_vs[k], loop_vs[(k + 1) % loop_vs.len()]))
            .collect();
        let clash = tris.iter().any(|t| {
            (0..3).any(|e| {
                let key = edge_key(loop_vs[t[e]], loop_vs[t[(e + 1) % 3]]);
                !boundary.contains(&key) && edges.contains(&key)
            })
        });
        if clash {
            continue;
        }
        if !budget.take_fill(tris.len()) {
            return;
        }
        // Orient fill triangles so boundary edges are traversed against the
        // surface direction (ear clipping yields a consistently wound strip,
        // so one probe orients them all).
        let boundary_dir: HashSet<(usize, usize)> = (0..loop_vs.len())
            .map(|k| (loop_vs[k], loop_vs[(k + 1) % loop_vs.len()]))
            .collect();
        let needs_flip = tris.iter().any(|t| {
            (0..3).any(|e| boundary_dir.contains(&(loop_vs[t[e]], loop_vs[t[(e + 1) % 3]])))
        });
        report.filled_holes += 1;
        report.filled_triangles += tris.len();
        for mut t in tris {
            if needs_flip {
                t.swap(1, 2);
            }
            let tri = [loop_vs[t[0]], loop_vs[t[1]], loop_vs[t[2]]];
            for i in 0..3 {
                edges.insert(edge_key(tri[i], tri[(i + 1) % 3]));
            }
            mesh.tris.push(tri);
            mesh.source.push(None);
        }
    }
}

/// Signed projected loop area along its dominant normal axis (half the
/// Newell normal component). Sign distinguishes hole loops from the
/// exterior contour on a consistently oriented surface.
fn loop_signed_area(positions: &[f64], loop_vs: &[usize]) -> f64 {
    let n = newell(positions, loop_vs);
    n[dominant_axis(n)] / 2.0
}

fn dominant_axis(n: [f64; 3]) -> usize {
    if n[0].abs() >= n[1].abs() && n[0].abs() >= n[2].abs() {
        0
    } else if n[1].abs() >= n[2].abs() {
        1
    } else {
        2
    }
}

/// Triangulate a simple 3D loop, returning triangle indices into `loop_vs`
/// with the same winding as the polygon orientation on its best-fit plane.
/// `None` when the loop is degenerate (zero-area or unclippable) or the work
/// budget ran out (ear clipping is O(n²) per loop, so large loops are
/// refused by budget rather than stalling).
fn triangulate_loop(
    positions: &[f64],
    loop_vs: &[usize],
    budget: &mut Budget,
) -> Option<Vec<[usize; 3]>> {
    let n = newell(positions, loop_vs);
    let len2 = n[0] * n[0] + n[1] * n[1] + n[2] * n[2];
    if !len2.is_finite() || len2 <= 1e-24 {
        return None;
    }
    // Project onto the plane perpendicular to the dominant axis.
    let axis = dominant_axis(n);
    let (u, v) = ((axis + 1) % 3, (axis + 2) % 3);
    let pts: Vec<(f64, f64)> = loop_vs
        .iter()
        .map(|&i| (positions[i * 3 + u], positions[i * 3 + v]))
        .collect();
    let signed_area: f64 = (0..pts.len())
        .map(|i| {
            let (x0, y0) = pts[i];
            let (x1, y1) = pts[(i + 1) % pts.len()];
            x0 * y1 - x1 * y0
        })
        .sum::<f64>()
        / 2.0;
    if signed_area.abs() <= 1e-24 || !signed_area.is_finite() {
        return None;
    }
    // Ear clipping on a CCW-normalised index list.
    let mut idx: Vec<usize> = (0..pts.len()).collect();
    if signed_area < 0.0 {
        idx.reverse();
    }
    let cross = |a: usize, b: usize, c: usize| -> f64 {
        (pts[b].0 - pts[a].0) * (pts[c].1 - pts[a].1)
            - (pts[b].1 - pts[a].1) * (pts[c].0 - pts[a].0)
    };
    let inside = |p: usize, a: usize, b: usize, c: usize| -> bool {
        cross(a, b, p) >= 0.0 && cross(b, c, p) >= 0.0 && cross(c, a, p) >= 0.0
    };
    let mut out = Vec::with_capacity(idx.len() - 2);
    while idx.len() > 3 {
        let m = idx.len();
        let mut clipped = false;
        for i in 0..m {
            if !budget.charge(m as u64) {
                return None;
            }
            let (a, b, c) = (idx[(i + m - 1) % m], idx[i], idx[(i + 1) % m]);
            if cross(a, b, c) <= 0.0 {
                continue; // reflex vertex
            }
            if idx
                .iter()
                .any(|&p| p != a && p != b && p != c && inside(p, a, b, c))
            {
                continue;
            }
            out.push([a, b, c]);
            idx.remove(i);
            clipped = true;
            break;
        }
        if !clipped {
            return None; // numerically unclippable; left for residual
        }
    }
    if idx.len() == 3 {
        out.push([idx[0], idx[1], idx[2]]);
    }
    Some(out)
}

#[cfg(test)]
#[path = "tests/repair.rs"]
mod tests;
