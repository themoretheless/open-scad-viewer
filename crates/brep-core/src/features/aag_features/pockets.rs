use super::*;

pub(super) fn pockets_error(message: impl Into<String>) -> Error {
    Error::new("BREP_AAG_POCKETS_INPUT", message)
}

/// Open/closed classification of a pocket (by its mouth).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PocketKind {
    /// The floor is fully enclosed by walls: the mouth is a closed rim loop.
    Closed,
    /// At least one floor face reaches the exterior through a convex
    /// boundary edge: the pocket opens to the body boundary (slot).
    Open,
}

/// Wall slope classification of a pocket.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PocketWalls {
    /// Wall outward normals are perpendicular to the floor normal.
    Straight,
    /// At least one wall leans off perpendicular (draft angle reported).
    Drafted,
}

/// One recognized pocket: floor, walls, absorbed floor fillets, parameters
/// and nesting hierarchy.
#[derive(Clone, Debug, PartialEq)]
pub struct PocketFeature {
    /// All member faces (floor + wall + fillet), ascending.
    pub faces: Vec<usize>,
    pub floor_faces: Vec<usize>,
    pub wall_faces: Vec<usize>,
    /// Faces of collapsed fillet chains absorbed into the subgraph (floor
    /// blends), ascending.
    pub fillet_faces: Vec<usize>,
    /// Rim faces outside the pocket that its boundary edges land on,
    /// ascending. For a nested pocket these lie inside the parent.
    pub rim_faces: Vec<usize>,
    pub kind: PocketKind,
    pub walls: PocketWalls,
    /// Max wall lean-off from perpendicular, degrees; `Some` only for
    /// [`PocketWalls::Drafted`]. Positive means the mouth is wider than the
    /// floor (draft opens upward).
    pub draft_angle_deg: Option<f64>,
    /// Rim-to-floor distance along the floor normal.
    pub depth: f64,
    /// Sum of trimmed floor face areas.
    pub floor_area: f64,
    /// Outward normal of the floor (points into the cavity).
    pub floor_normal: [f64; 3],
    /// Index of the enclosing pocket in the returned vector, if nested.
    pub parent: Option<usize>,
    /// Indices of pockets nested directly inside this one.
    pub children: Vec<usize>,
}

/// Outward unit normal of one face at (u, v), honoring the shell's reversed
/// use; `None` at poles/singular charts.
pub(super) fn outward_normal(
    model: &Model,
    face: usize,
    u: f64,
    v: f64,
) -> Result<Option<[f64; 3]>> {
    let reversed = model
        .shells
        .iter()
        .flat_map(|s| s.faces.iter())
        .find(|use_| use_.face == face)
        .is_some_and(|use_| use_.reversed);
    let sampler = SurfaceRefSampler::new(&model.faces[face].surface)?;
    let [u, v] = face_parameters(model, face, u, v);
    let Some(mut n) = sampler.evaluate(u, v)?.unit_normal() else {
        return Ok(None);
    };
    if reversed {
        n = [-n[0], -n[1], -n[2]];
    }
    Ok(Some(n))
}

/// Surface point of one face at (u, v).
pub(super) fn face_point(model: &Model, face: usize, u: f64, v: f64) -> Result<[f64; 3]> {
    let sampler = SurfaceRefSampler::new(&model.faces[face].surface)?;
    let [u, v] = face_parameters(model, face, u, v);
    sampler.point(u, v)
}

/// Ray-parity probe: is `point` inside material? `None` = unresolved.
pub(super) fn probe_inside(model: &Model, point: [f64; 3]) -> Result<Option<bool>> {
    let dirs = [[1., 0.317, 0.173], [-0.219, 1., 0.413], [0.271, -0.193, 1.]];
    Ok(
        crate::ray_parity::classify_point(model, point, &dirs, model.tolerance_mm, 10_000, 200_000)?
            .parity,
    )
}

/// Role of a supernode inside a candidate pocket subgraph.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PocketRole {
    Floor,
    Wall,
    /// Collapsed fillet chain (transparent blend between floor and wall).
    Fillet,
}

/// Multiplier on `model.tolerance_mm` for the same-surface linear tolerance.
const SAME_SURFACE_TOLERANCE_FACTOR: f64 = 100.;
/// Lower bound of the same-surface linear tolerance.
const SAME_SURFACE_TOLERANCE_FLOOR: f64 = 1e-7;
/// Minimum pocket depth, in multiples of `model.tolerance_mm`.
const MIN_DEPTH_TOLERANCE_FACTOR: f64 = 10.;
/// Probe offset relative to the pocket depth.
const PROBE_OFFSET_DEPTH_FRACTION: f64 = 1e-3;
/// Lower bound of the probe offset, in multiples of `model.tolerance_mm`.
const PROBE_OFFSET_TOLERANCE_FACTOR: f64 = 10.;
/// A wall leaning further than this off perpendicular faces the floor.
const MAX_WALL_LEAN_DEG: f64 = 45.;
/// Wall leans above this angle report the pocket as drafted.
const DRAFT_THRESHOLD_DEG: f64 = 0.5;
/// Error text when face attributes are missing or stale.
const STALE_ATTRS_MESSAGE: &str =
    "AAG face attributes missing or stale; run attach_face_attrs first";

/// Plane key of a face: outward normal and signed offset along it.
type PlaneKey = ([f64; 3], f64);

/// One adjacency between supernodes: (root_a, root_b, class, aag edge index).
type SupernodeEdge = (usize, usize, DihedralClass, usize);

/// Union-find root with path compression.
fn union_root(parent: &mut [usize], i: usize) -> usize {
    let mut r = i;
    while parent[r] != r {
        r = parent[r];
    }
    let mut c = i;
    while parent[c] != c {
        let next = parent[c];
        parent[c] = r;
        c = next;
    }
    r
}

/// Fillet-chain collapse: every chain becomes one supernode. Returns the
/// supernode root of every face and the set of fillet supernode roots.
fn collapse_fillet_chains(
    n: usize,
    chains: &[crate::aag_fillets::FilletChain],
    guard: &mut BudgetGuard,
) -> Result<(Vec<usize>, std::collections::BTreeSet<usize>)> {
    let mut parent: Vec<usize> = (0..n).collect();
    let mut fillet_roots: std::collections::BTreeSet<usize> = Default::default();
    for chain in chains {
        guard.tick()?;
        for &f in chain.faces.iter().skip(1) {
            let (a, b) = (
                union_root(&mut parent, chain.faces[0]),
                union_root(&mut parent, f),
            );
            if a != b {
                parent[b] = a;
            }
        }
        fillet_roots.insert(union_root(&mut parent, chain.faces[0]));
    }
    let roots: Vec<usize> = (0..n).map(|i| union_root(&mut parent, i)).collect();
    Ok((roots, fillet_roots))
}

/// Effective adjacency between supernodes (skip chain-internal edges).
fn supernode_adjacency(
    graph: &Aag,
    roots: &[usize],
    guard: &mut BudgetGuard,
) -> Result<Vec<SupernodeEdge>> {
    let mut adj: Vec<SupernodeEdge> = Vec::new();
    for (ei, edge) in graph.edges.iter().enumerate() {
        guard.tick()?;
        if edge.uses.len() != 2 {
            continue;
        }
        let (ra, rb) = (roots[edge.uses[0].face], roots[edge.uses[1].face]);
        if ra != rb {
            adj.push((ra, rb, edge.class, ei));
        }
    }
    Ok(adj)
}

/// Boundary evidence of a validated candidate subgraph.
struct RimScan {
    rim_faces: Vec<usize>,
    rim_points: Vec<[f64; 3]>,
    floor_has_boundary: bool,
}

/// Wall geometry of a candidate: max lean and (center, normal) per wall.
struct WallLeans {
    draft_max: f64,
    wall_dirs: Vec<([f64; 3], [f64; 3])>,
}

/// Floor plane of a candidate seed.
struct FloorFrame {
    normal: [f64; 3],
    center: [f64; 3],
    depth: f64,
    eps: f64,
}

/// Pocket search state shared by every seed.
struct PocketSearch<'a> {
    model: &'a Model,
    graph: &'a Aag,
    attrs: Vec<&'a FaceAttrs>,
    roots: Vec<usize>,
    fillet_roots: std::collections::BTreeSet<usize>,
    adj: Vec<SupernodeEdge>,
    linear_tol: f64,
    plane_keys: std::collections::BTreeMap<usize, PlaneKey>,
    seen: std::collections::BTreeMap<Vec<usize>, usize>,
    pockets: Vec<PocketFeature>,
}

impl PocketSearch<'_> {
    fn wall_eligible(&self, face: usize) -> bool {
        matches!(
            self.attrs[face].class,
            SurfaceClass::Plane | SurfaceClass::Cone | SurfaceClass::Cylinder
        )
    }

    fn plane_key(&mut self, face: usize) -> Result<Option<PlaneKey>> {
        if let Some(k) = self.plane_keys.get(&face) {
            return Ok(Some(*k));
        }
        let Some(n) = outward_normal(self.model, face, 0.5, 0.5)? else {
            return Ok(None);
        };
        let p = face_point(self.model, face, 0.5, 0.5)?;
        let key = (n, dot(n, p));
        self.plane_keys.insert(face, key);
        Ok(Some(key))
    }

    /// Same-surface sibling test across a smooth edge.
    fn same_surface(&mut self, a: usize, b: usize) -> Result<bool> {
        let (fa, fb) = (self.attrs[a], self.attrs[b]);
        if fa.class != fb.class {
            return Ok(false);
        }
        let linear_tol = self.linear_tol;
        match fa.class {
            SurfaceClass::Plane => {
                let (Some((na, oa)), Some((nb, ob))) = (self.plane_key(a)?, self.plane_key(b)?)
                else {
                    return Ok(false);
                };
                Ok(dot(na, nb) > 1. - AXIS_DOT_TOLERANCE && (oa - ob).abs() <= linear_tol)
            }
            SurfaceClass::Cone => Ok(match (fa.axis, fb.axis) {
                (Some(x), Some(y)) => coaxial(&x, &y, linear_tol),
                _ => false,
            }),
            SurfaceClass::Cylinder => Ok(match (fa.axis, fb.axis, fa.radius, fb.radius) {
                (Some(x), Some(y), Some(u), Some(v)) => {
                    coaxial(&x, &y, linear_tol) && (u - v).abs() <= RADIUS_REL_TOLERANCE * u.max(v)
                }
                _ => false,
            }),
            _ => Ok(false),
        }
    }

    /// Role a neighbor `q` of supernode `r` takes across an edge of `class`.
    fn neighbor_role(
        &mut self,
        seed: usize,
        r: usize,
        r_role: PocketRole,
        q: usize,
        class: DihedralClass,
    ) -> Result<Option<PocketRole>> {
        match class {
            // Walls attach to the floor (or to a floor blend) through
            // reentrant edges. Walls never expand further: a concave
            // edge past a wall is a multi-level step, out of scope.
            DihedralClass::Concave => Ok(match r_role {
                PocketRole::Floor | PocketRole::Fillet if self.wall_eligible(q) => {
                    Some(PocketRole::Wall)
                }
                _ => None,
            }),
            // Transparent fillet supernodes, and same-surface sibling
            // patches (partitioned faces keep their role).
            DihedralClass::Smooth => {
                if self.fillet_roots.contains(&q) {
                    Ok(Some(PocketRole::Fillet))
                } else if self.wall_eligible(q) && self.same_surface(r, q)? {
                    Ok(Some(match r_role {
                        PocketRole::Floor => PocketRole::Floor,
                        _ => PocketRole::Wall,
                    }))
                } else if r_role == PocketRole::Fillet && self.wall_eligible(q) {
                    // Far support of a collapsed blend. It is usually
                    // a wall — but BFS order may reach a coplanar
                    // floor sibling through the blend before its own
                    // seam edges: a plane coplanar with the seed stays
                    // floor, never a 90°-leaning "wall".
                    if self.attrs[q].class == SurfaceClass::Plane && self.same_surface(seed, q)? {
                        Ok(Some(PocketRole::Floor))
                    } else {
                        Ok(Some(PocketRole::Wall))
                    }
                } else {
                    Ok(None)
                }
            }
            _ => Ok(None),
        }
    }

    /// BFS subgraph growth from one plane seed.
    fn grow_roles(
        &mut self,
        seed: usize,
        guard: &mut BudgetGuard,
    ) -> Result<std::collections::BTreeMap<usize, PocketRole>> {
        let mut role: std::collections::BTreeMap<usize, PocketRole> = Default::default();
        role.insert(seed, PocketRole::Floor);
        let mut queue = std::collections::VecDeque::from([seed]);
        while let Some(r) = queue.pop_front() {
            guard.tick()?;
            let r_role = role[&r];
            for i in 0..self.adj.len() {
                let (ra, rb, class, _) = self.adj[i];
                if ra != r && rb != r {
                    continue;
                }
                let q = if ra == r { rb } else { ra };
                if role.contains_key(&q) {
                    continue;
                }
                if let Some(rl) = self.neighbor_role(seed, r, r_role, q, class)? {
                    role.insert(q, rl);
                    queue.push_back(q);
                }
            }
        }
        Ok(role)
    }

    /// Edge-pattern validation: internal edges concave/smooth, boundary
    /// (rim) edges convex — with the collapse, floor blends read as
    /// internal smooth edges. `None` rejects the candidate.
    fn scan_rim(
        &self,
        role: &std::collections::BTreeMap<usize, PocketRole>,
        guard: &mut BudgetGuard,
    ) -> Result<Option<RimScan>> {
        let in_subgraph = |r: usize| role.contains_key(&r);
        let mut scan = RimScan {
            rim_faces: Vec::new(),
            rim_points: Vec::new(),
            floor_has_boundary: false,
        };
        for &(ra, rb, class, ei) in &self.adj {
            guard.tick()?;
            match (in_subgraph(ra), in_subgraph(rb)) {
                (true, true) => {
                    if !matches!(class, DihedralClass::Concave | DihedralClass::Smooth) {
                        return Ok(None);
                    }
                }
                (true, false) | (false, true) => {
                    if class != DihedralClass::Convex {
                        return Ok(None);
                    }
                    let (inside, outside) = if in_subgraph(ra) { (ra, rb) } else { (rb, ra) };
                    if role[&inside] == PocketRole::Floor {
                        scan.floor_has_boundary = true;
                    }
                    for u in &self.graph.edges[ei].uses {
                        if self.roots[u.face] == outside && !scan.rim_faces.contains(&u.face) {
                            scan.rim_faces.push(u.face);
                        }
                    }
                    let edge = &self.model.edges[self.graph.edges[ei].edge];
                    let domain = edge.curve.domain();
                    let mid = edge.curve.evaluate(0.5 * (domain[0] + domain[1]))?.point;
                    if mid.len() == 3 {
                        scan.rim_points.push([mid[0], mid[1], mid[2]]);
                    }
                }
                _ => {}
            }
        }
        Ok((!scan.rim_points.is_empty()).then_some(scan))
    }

    /// Floor plane and rim-to-floor depth; `None` rejects the candidate.
    fn floor_frame(&self, seed: usize, rim_points: &[[f64; 3]]) -> Result<Option<FloorFrame>> {
        let tolerance = self.model.tolerance_mm;
        let Some(normal) = outward_normal(self.model, seed, 0.5, 0.5)? else {
            return Ok(None);
        };
        let center = face_point(self.model, seed, 0.5, 0.5)?;
        let offset = dot(normal, center);
        let depth = rim_points
            .iter()
            .map(|p| dot(normal, *p) - offset)
            .fold(0f64, f64::max);
        if !(depth.is_finite() && depth > MIN_DEPTH_TOLERANCE_FACTOR * tolerance) {
            return Ok(None);
        }
        let eps =
            (PROBE_OFFSET_DEPTH_FRACTION * depth).max(PROBE_OFFSET_TOLERANCE_FACTOR * tolerance);
        Ok(Some(FloorFrame {
            normal,
            center,
            depth,
            eps,
        }))
    }

    /// Wall leans are cheap (surface normals only) and gate the expensive
    /// ray-parity probes. `None` rejects the candidate.
    fn wall_leans(
        &self,
        walls: &[usize],
        floor_normal: [f64; 3],
        guard: &mut BudgetGuard,
    ) -> Result<Option<WallLeans>> {
        let mut leans = WallLeans {
            draft_max: 0f64,
            wall_dirs: Vec::new(),
        };
        for &w in walls {
            guard.tick()?;
            let (Some(nw), Ok(wc)) = (
                outward_normal(self.model, w, 0.5, 0.5)?,
                face_point(self.model, w, 0.5, 0.5),
            ) else {
                return Ok(None);
            };
            let lean = dot(nw, floor_normal).clamp(-1., 1.).asin().to_degrees();
            // A wall leaning more than 45° off perpendicular is parallel to
            // the floor normal — that "wall" faces the floor, so the seed
            // is a side wall, not the true floor. Reject this reading.
            if lean.abs() > MAX_WALL_LEAN_DEG {
                return Ok(None);
            }
            leans.draft_max = leans.draft_max.max(lean.abs());
            leans.wall_dirs.push((wc, nw));
        }
        Ok(Some(leans))
    }

    /// Faces of every supernode in `roots_of`, ascending.
    fn expand(&self, roots_of: &[usize]) -> Vec<usize> {
        let mut out: Vec<usize> = (0..self.roots.len())
            .filter(|&f| roots_of.contains(&self.roots[f]))
            .collect();
        out.sort_unstable();
        out
    }

    /// Cavity probes: the void side of the floor and just past every wall
    /// (outward normal points into the cavity) must be void. A boss probe
    /// lands in material. Unresolved rays reject conservatively.
    fn probes_clear(
        &self,
        floor: &FloorFrame,
        leans: &WallLeans,
        guard: &mut BudgetGuard,
    ) -> Result<bool> {
        let eps = floor.eps;
        guard.tick()?;
        let above_floor = [
            floor.center[0] + floor.normal[0] * eps,
            floor.center[1] + floor.normal[1] * eps,
            floor.center[2] + floor.normal[2] * eps,
        ];
        if probe_inside(self.model, above_floor)? != Some(false) {
            return Ok(false);
        }
        for (wc, nw) in &leans.wall_dirs {
            guard.tick()?;
            let probe = [
                wc[0] + nw[0] * eps,
                wc[1] + nw[1] * eps,
                wc[2] + nw[2] * eps,
            ];
            if probe_inside(self.model, probe)? != Some(false) {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Grow, validate and probe one floor seed; records a new or deeper
    /// pocket reading.
    fn try_seed(&mut self, seed: usize, guard: &mut BudgetGuard) -> Result<()> {
        if self.fillet_roots.contains(&seed) || self.attrs[seed].class != SurfaceClass::Plane {
            return Ok(());
        }
        let role = self.grow_roles(seed, guard)?;
        let with_role = |want: PocketRole| -> Vec<usize> {
            role.iter()
                .filter(|(_, rl)| **rl == want)
                .map(|(&r, _)| r)
                .collect()
        };
        let floors = with_role(PocketRole::Floor);
        let walls = with_role(PocketRole::Wall);
        let fillets = with_role(PocketRole::Fillet);
        if walls.is_empty() {
            return Ok(());
        }
        let Some(mut rim) = self.scan_rim(&role, guard)? else {
            return Ok(());
        };
        let Some(floor) = self.floor_frame(seed, &rim.rim_points)? else {
            return Ok(());
        };
        let Some(leans) = self.wall_leans(&walls, floor.normal, guard)? else {
            return Ok(());
        };

        let floor_faces = self.expand(&floors);
        let wall_faces = self.expand(&walls);
        let fillet_faces = self.expand(&fillets);
        let floor_area = floor_faces.iter().map(|&f| self.attrs[f].area).sum();
        let mut faces = floor_faces.clone();
        faces.extend_from_slice(&wall_faces);
        faces.extend_from_slice(&fillet_faces);
        faces.sort_unstable();
        rim.rim_faces.sort_unstable();

        // The same cavity can validate from many seeds (every coplanar floor
        // fragment, and a slot even reads as its own rotated self). Probes
        // are the expensive part: only run them for a new face set or for a
        // strictly deeper reading of one already found.
        let replace_idx = match self.seen.get(&faces) {
            Some(&idx) if floor.depth <= self.pockets[idx].depth => return Ok(()),
            Some(&idx) => Some(idx),
            None => None,
        };
        if !self.probes_clear(&floor, &leans, guard)? {
            return Ok(());
        }

        let drafted = leans.draft_max > DRAFT_THRESHOLD_DEG;
        let candidate = PocketFeature {
            faces,
            floor_faces,
            wall_faces,
            fillet_faces,
            rim_faces: rim.rim_faces,
            kind: if rim.floor_has_boundary {
                PocketKind::Open
            } else {
                PocketKind::Closed
            },
            walls: if drafted {
                PocketWalls::Drafted
            } else {
                PocketWalls::Straight
            },
            draft_angle_deg: drafted.then_some(leans.draft_max),
            depth: floor.depth,
            floor_area,
            floor_normal: floor.normal,
            parent: None,
            children: vec![],
        };
        match replace_idx {
            Some(idx) => self.pockets[idx] = candidate,
            None => {
                self.seen
                    .insert(candidate.faces.clone(), self.pockets.len());
                self.pockets.push(candidate);
            }
        }
        Ok(())
    }
}

/// Hierarchy: a pocket's rim landing inside another pocket's face set makes
/// it a child of that pocket.
fn link_pocket_hierarchy(pockets: &mut [PocketFeature]) {
    let owner: std::collections::BTreeMap<usize, usize> = pockets
        .iter()
        .enumerate()
        .flat_map(|(i, p)| p.faces.iter().map(move |&f| (f, i)))
        .collect();
    let parents: Vec<Option<usize>> = pockets
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let mut votes: std::collections::BTreeMap<usize, usize> = Default::default();
            for f in &p.rim_faces {
                if let Some(&j) = owner.get(f) {
                    // The rim of a nested pocket lands on the parent's floor,
                    // so the parent's rim-to-floor span exceeds the child's.
                    // This also breaks the reverse vote (the parent's rim
                    // touches the child's walls at the pit mouth).
                    if j != i && pockets[j].depth > p.depth {
                        *votes.entry(j).or_default() += 1;
                    }
                }
            }
            votes.into_iter().max_by_key(|(_, c)| *c).map(|(j, _)| j)
        })
        .collect();
    for (i, p) in parents.iter().enumerate() {
        pockets[i].parent = *p;
        if let Some(j) = *p {
            pockets[j].children.push(i);
        }
    }
}

/// Find pockets in `model` by subgraph matching over `graph` (checklist 867).
///
/// Same contract as [`find_holes`]: `graph` must carry fresh face
/// attributes; work is charged to `budget`. Floor fillet chains from
/// [`crate::aag_fillets::find_fillet_chains`] are collapsed into transparent
/// supernodes before matching.
pub fn find_pockets(model: &Model, graph: &Aag, budget: &Budget) -> Result<Vec<PocketFeature>> {
    require_finite_f64(model.tolerance_mm, "model.tolerance_mm")?;
    if graph.nodes.len() != model.faces.len() {
        return Err(pockets_error("AAG node count does not match model faces"));
    }
    if !graph.stale_face_attrs(model).is_empty() {
        return Err(pockets_error(STALE_ATTRS_MESSAGE));
    }
    let mut guard: BudgetGuard = budget.guard("aag-find-pockets");
    guard.check()?;
    let n = model.faces.len();

    let chains = crate::aag_fillets::find_fillet_chains(model, graph, budget)?;
    let (roots, fillet_roots) = collapse_fillet_chains(n, &chains, &mut guard)?;
    let adj = supernode_adjacency(graph, &roots, &mut guard)?;
    let attrs = graph
        .nodes
        .iter()
        .map(|node| {
            node.attrs
                .as_ref()
                .ok_or_else(|| pockets_error(STALE_ATTRS_MESSAGE))
        })
        .collect::<Result<Vec<_>>>()?;
    let mut search = PocketSearch {
        model,
        graph,
        attrs,
        roots,
        fillet_roots,
        adj,
        linear_tol: (model.tolerance_mm * SAME_SURFACE_TOLERANCE_FACTOR)
            .max(SAME_SURFACE_TOLERANCE_FLOOR),
        plane_keys: Default::default(),
        seen: Default::default(),
        pockets: Vec::new(),
    };
    for seed in 0..n {
        guard.tick()?;
        search.try_seed(seed, &mut guard)?;
    }

    let mut pockets = search.pockets;
    pockets.sort_by_key(|p| p.faces[0]);
    link_pocket_hierarchy(&mut pockets);
    guard.check()?;
    Ok(pockets)
}
