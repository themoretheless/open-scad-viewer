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
    let sampler = SurfaceSampler::new(&model.faces[face].surface)?;
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
    let sampler = SurfaceSampler::new(&model.faces[face].surface)?;
    let [u, v] = face_parameters(model, face, u, v);
let p = sampler.evaluate(u, v)?.point;
    if p.len() != 3 {
        return Err(pockets_error("pocket matching expects 3D surface points"));
    }
    Ok([p[0], p[1], p[2]])
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
        return Err(pockets_error(
            "AAG face attributes missing or stale; run attach_face_attrs first",
        ));
    }
    let mut guard: BudgetGuard = budget.guard("aag-find-pockets");
    guard.check()?;
    let n = model.faces.len();

    // Fillet-chain collapse: every chain becomes one supernode.
    let chains = crate::aag_fillets::find_fillet_chains(model, graph, budget)?;
    let mut parent: Vec<usize> = (0..n).collect();
    fn root(parent: &mut Vec<usize>, i: usize) -> usize {
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
    let mut fillet_roots: std::collections::BTreeSet<usize> = Default::default();
    for chain in &chains {
        guard.tick()?;
        for &f in chain.faces.iter().skip(1) {
            let (a, b) = (root(&mut parent, chain.faces[0]), root(&mut parent, f));
            if a != b {
                parent[b] = a;
            }
        }
        fillet_roots.insert(root(&mut parent, chain.faces[0]));
    }
    let roots: Vec<usize> = (0..n).map(|i| root(&mut parent, i)).collect();

    // Effective adjacency between supernodes (skip chain-internal edges).
    // (root_a, root_b, class, aag edge index)
    let mut adj: Vec<(usize, usize, DihedralClass, usize)> = Vec::new();
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
    let attrs_of = |face: usize| graph.nodes[face].attrs.as_ref().expect("stale checked");
    let wall_eligible = |face: usize| {
        matches!(
            attrs_of(face).class,
            SurfaceClass::Plane | SurfaceClass::Cone | SurfaceClass::Cylinder
        )
    };
    // Same-surface sibling test across a smooth edge.
    let linear_tol = (model.tolerance_mm * 100.).max(1e-7);
    let mut plane_key_cache: std::collections::BTreeMap<usize, ([f64; 3], f64)> = Default::default();
    let mut plane_key = |model: &Model, face: usize| -> Result<Option<([f64; 3], f64)>> {
        if let Some(k) = plane_key_cache.get(&face) {
            return Ok(Some(*k));
        }
        let Some(n) = outward_normal(model, face, 0.5, 0.5)? else {
            return Ok(None);
        };
        let p = face_point(model, face, 0.5, 0.5)?;
        let key = (n, dot(n, p));
        plane_key_cache.insert(face, key);
        Ok(Some(key))
    };
    let same_surface = |model: &Model,
                        a: usize,
                        b: usize,
                        plane_key: &mut dyn FnMut(&Model, usize) -> Result<Option<([f64; 3], f64)>>|
     -> Result<bool> {
        let (ca, cb) = (attrs_of(a).class, attrs_of(b).class);
        if ca != cb {
            return Ok(false);
        }
        match ca {
            SurfaceClass::Plane => {
                let (Some((na, oa)), Some((nb, ob))) = (plane_key(model, a)?, plane_key(model, b)?)
                else {
                    return Ok(false);
                };
                Ok(dot(na, nb) > 1. - AXIS_DOT_TOLERANCE && (oa - ob).abs() <= linear_tol)
            }
            SurfaceClass::Cone => Ok(match (attrs_of(a).axis, attrs_of(b).axis) {
                (Some(x), Some(y)) => coaxial(&x, &y, linear_tol),
                _ => false,
            }),
            SurfaceClass::Cylinder => {
                let (ra, rb_) = (attrs_of(a).radius, attrs_of(b).radius);
                Ok(match (attrs_of(a).axis, attrs_of(b).axis, ra, rb_) {
                    (Some(x), Some(y), Some(u), Some(v)) => {
                        coaxial(&x, &y, linear_tol)
                            && (u - v).abs() <= RADIUS_REL_TOLERANCE * u.max(v)
                    }
                    _ => false,
                })
            }
            _ => Ok(false),
        }
    };

    // BFS subgraph growth from every plane seed.
    let mut seen: std::collections::BTreeMap<Vec<usize>, usize> = Default::default();
    let mut pockets: Vec<PocketFeature> = Vec::new();
    for seed in 0..n {
        guard.tick()?;
        if fillet_roots.contains(&seed) || attrs_of(seed).class != SurfaceClass::Plane {
            continue;
        }
        let mut role: std::collections::BTreeMap<usize, PocketRole> = Default::default();
        role.insert(seed, PocketRole::Floor);
        let mut queue = std::collections::VecDeque::from([seed]);
        while let Some(r) = queue.pop_front() {
            guard.tick()?;
            let r_role = role[&r];
            for &(ra, rb, class, _) in adj.iter().filter(|(a, b, ..)| *a == r || *b == r) {
                let q = if ra == r { rb } else { ra };
                if role.contains_key(&q) {
                    continue;
                }
                let add = match class {
                    // Walls attach to the floor (or to a floor blend) through
                    // reentrant edges. Walls never expand further: a concave
                    // edge past a wall is a multi-level step, out of scope.
                    DihedralClass::Concave => match r_role {
                        PocketRole::Floor | PocketRole::Fillet if wall_eligible(q) => {
                            Some(PocketRole::Wall)
                        }
                        _ => None,
                    },
                    // Transparent fillet supernodes, and same-surface sibling
                    // patches (partitioned faces keep their role).
                    DihedralClass::Smooth => {
                        if fillet_roots.contains(&q) {
                            Some(PocketRole::Fillet)
                        } else if wall_eligible(q)
                            && matches!(r_role, PocketRole::Floor | PocketRole::Wall | PocketRole::Fillet)
                            && same_surface(model, r, q, &mut plane_key)?
                        {
                            Some(match r_role {
                                PocketRole::Floor => PocketRole::Floor,
                                _ => PocketRole::Wall,
                            })
                        } else if r_role == PocketRole::Fillet && wall_eligible(q) {
                            // Far support of a collapsed blend. It is usually
                            // a wall — but BFS order may reach a coplanar
                            // floor sibling through the blend before its own
                            // seam edges: a plane coplanar with the seed stays
                            // floor, never a 90°-leaning "wall".
                            if attrs_of(q).class == SurfaceClass::Plane
                                && same_surface(model, seed, q, &mut plane_key)?
                            {
                                Some(PocketRole::Floor)
                            } else {
                                Some(PocketRole::Wall)
                            }
                        } else {
                            None
                        }
                    }
                    _ => None,
                };
                if let Some(rl) = add {
                    role.insert(q, rl);
                    queue.push_back(q);
                }
            }
        }

        let floors: Vec<usize> = role
            .iter()
            .filter(|(_, rl)| **rl == PocketRole::Floor)
            .map(|(&r, _)| r)
            .collect();
        let walls: Vec<usize> = role
            .iter()
            .filter(|(_, rl)| **rl == PocketRole::Wall)
            .map(|(&r, _)| r)
            .collect();
        let fillets: Vec<usize> = role
            .iter()
            .filter(|(_, rl)| **rl == PocketRole::Fillet)
            .map(|(&r, _)| r)
            .collect();
        if walls.is_empty() {
            continue;
        }
        let in_subgraph = |r: usize| role.contains_key(&r);

        // Edge-pattern validation: internal edges concave/smooth, boundary
        // (rim) edges convex — with the collapse, floor blends read as
        // internal smooth edges.
        let mut valid = true;
        let mut rim_faces: Vec<usize> = Vec::new();
        let mut rim_points: Vec<[f64; 3]> = Vec::new();
        let mut floor_has_boundary = false;
        for &(ra, rb, class, ei) in &adj {
            guard.tick()?;
            match (in_subgraph(ra), in_subgraph(rb)) {
                (true, true) => {
                    if !matches!(class, DihedralClass::Concave | DihedralClass::Smooth) {
                        valid = false;
                        break;
                    }
                }
                (true, false) | (false, true) => {
                    if class != DihedralClass::Convex {
                        valid = false;
                        break;
                    }
                    let (inside, outside) = if in_subgraph(ra) { (ra, rb) } else { (rb, ra) };
                    if role[&inside] == PocketRole::Floor {
                        floor_has_boundary = true;
                    }
                    for u in &graph.edges[ei].uses {
                        if roots[u.face] == outside && !rim_faces.contains(&u.face) {
                            rim_faces.push(u.face);
                        }
                    }
                    let edge = &model.edges[graph.edges[ei].edge];
                    let domain = edge.curve.domain();
                    let mid = edge.curve.evaluate(0.5 * (domain[0] + domain[1]))?.point;
                    if mid.len() == 3 {
                        rim_points.push([mid[0], mid[1], mid[2]]);
                    }
                }
                _ => {}
            }
        }
        if !valid || rim_points.is_empty() {
            continue;
        }

        // Geometry and probes.
        let Some(floor_normal) = outward_normal(model, seed, 0.5, 0.5)? else {
            continue;
        };
        let floor_center = face_point(model, seed, 0.5, 0.5)?;
        let floor_offset = dot(floor_normal, floor_center);
        let depth = rim_points
            .iter()
            .map(|p| dot(floor_normal, *p) - floor_offset)
            .fold(0f64, f64::max);
        if !(depth.is_finite() && depth > 10. * model.tolerance_mm) {
            continue;
        }
        let eps = (1e-3 * depth).max(10. * model.tolerance_mm);
        // Wall leans are cheap (surface normals only) and gate the expensive
        // ray-parity probes below.
        let mut draft_max = 0f64;
        let mut wall_dirs: Vec<([f64; 3], [f64; 3])> = Vec::new(); // (center, normal)
        let mut walls_ok = true;
        for &w in &walls {
            guard.tick()?;
            let (Some(nw), Ok(wc)) = (outward_normal(model, w, 0.5, 0.5)?, face_point(model, w, 0.5, 0.5))
            else {
                walls_ok = false;
                break;
            };
            let lean = dot(nw, floor_normal).clamp(-1., 1.).asin().to_degrees();
            // A wall leaning more than 45° off perpendicular is parallel to
            // the floor normal — that "wall" faces the floor, so the seed
            // is a side wall, not the true floor. Reject this reading.
            if lean.abs() > 45. {
                walls_ok = false;
                break;
            }
            draft_max = draft_max.max(lean.abs());
            wall_dirs.push((wc, nw));
        }
        if !walls_ok {
            continue;
        }

        let expand = |roots_of: &[usize]| -> Vec<usize> {
            let mut out: Vec<usize> = (0..n).filter(|&f| roots_of.contains(&roots[f])).collect();
            out.sort_unstable();
            out
        };
        let floor_faces = expand(&floors);
        let wall_faces = expand(&walls);
        let fillet_faces = expand(&fillets);
        let floor_area = floor_faces.iter().map(|&f| attrs_of(f).area).sum();
        let mut faces = floor_faces.clone();
        faces.extend_from_slice(&wall_faces);
        faces.extend_from_slice(&fillet_faces);
        faces.sort_unstable();
        rim_faces.sort_unstable();

        // The same cavity can validate from many seeds (every coplanar floor
        // fragment, and a slot even reads as its own rotated self). Probes
        // are the expensive part: only run them for a new face set or for a
        // strictly deeper reading of one already found.
        let replace_idx = match seen.get(&faces) {
            Some(&idx) if depth <= pockets[idx].depth => None,
            Some(&idx) => Some(Some(idx)),
            None => Some(None),
        };
        let Some(replace_idx) = replace_idx else {
            continue;
        };

        // The cavity side of the floor must be void.
        guard.tick()?;
        let above_floor = [
            floor_center[0] + floor_normal[0] * eps,
            floor_center[1] + floor_normal[1] * eps,
            floor_center[2] + floor_normal[2] * eps,
        ];
        if probe_inside(model, above_floor)? != Some(false) {
            continue;
        }
        // Pocket/boss discriminator: just past every wall (outward normal
        // points into the cavity) must be void. A boss probe lands in
        // material. Unresolved rays reject conservatively.
        for (wc, nw) in &wall_dirs {
            guard.tick()?;
            let probe = [wc[0] + nw[0] * eps, wc[1] + nw[1] * eps, wc[2] + nw[2] * eps];
            if probe_inside(model, probe)? != Some(false) {
                walls_ok = false;
                break;
            }
        }
        if !walls_ok {
            continue;
        }

        let drafted = draft_max > 0.5;
        let candidate = PocketFeature {
            faces,
            floor_faces,
            wall_faces,
            fillet_faces,
            rim_faces,
            kind: if floor_has_boundary {
                PocketKind::Open
            } else {
                PocketKind::Closed
            },
            walls: if drafted {
                PocketWalls::Drafted
            } else {
                PocketWalls::Straight
            },
            draft_angle_deg: drafted.then_some(draft_max),
            depth,
            floor_area,
            floor_normal,
            parent: None,
            children: vec![],
        };
        match replace_idx {
            Some(idx) => pockets[idx] = candidate,
            None => {
                seen.insert(candidate.faces.clone(), pockets.len());
                pockets.push(candidate);
            }
        }
    }

    pockets.sort_by_key(|p| p.faces[0]);
    // Hierarchy: a pocket's rim landing inside another pocket's face set
    // makes it a child of that pocket.
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
    guard.check()?;
    Ok(pockets)
}
