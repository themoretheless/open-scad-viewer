use super::*;

/// One directed edge use while chaining a ring: traversal orientation and
/// start vertex, already normalized for the face use's `reversed` flag.
#[derive(Clone, Copy, Debug)]
struct RingUse {
    edge: usize,
    reversed: bool,
    start: usize,
}

/// Is `face` used with `reversed = true` by any shell? (A face belongs to
/// exactly one shell use in every model this kernel authors.)
pub(super) fn face_use_reversed(model: &Model, face: usize) -> bool {
    model
        .shells
        .iter()
        .flat_map(|s| s.faces.iter())
        .find(|u| u.face == face)
        .is_some_and(|u| u.reversed)
}

/// Two planes are the same geometric plane within `tolerance`, sign-aligned.
pub(super) fn same_plane(a: PlaneEq, b: PlaneEq, tolerance: f64) -> bool {
    let align = dot(a.normal, b.normal);
    if align.abs() < 1. - 1e-9 {
        return false;
    }
    let signed_offset = if align > 0. { b.offset } else { -b.offset };
    (a.offset - signed_offset).abs() <= tolerance
}

/// Connected component of surviving faces coplanar with the seed face,
/// linked through shared edges.
pub(crate) fn coplanar_component(
    model: &Model,
    seed: usize,
    plane: PlaneEq,
    cluster: &BTreeSet<usize>,
    adjacency: &[Vec<usize>],
    guard: &mut BudgetGuard,
) -> Result<BTreeSet<usize>> {
    let tolerance = model.tolerance_mm * 8.;
    let mut component = BTreeSet::from([seed]);
    let mut stack = vec![seed];
    while let Some(face) = stack.pop() {
        guard.tick()?;
        for &ring in std::iter::once(&model.faces[face].outer).chain(&model.faces[face].holes) {
            for coedge in &model.loops[ring].coedges {
                for &neighbor in &adjacency[coedge.edge] {
                    if neighbor == face
                        || cluster.contains(&neighbor)
                        || component.contains(&neighbor)
                    {
                        continue;
                    }
                    // Non-planar neighbors are simply not coplanar shards.
                    let Ok(neighbor_plane) = face_plane_fit(model, neighbor) else {
                        continue;
                    };
                    if same_plane(plane, neighbor_plane, tolerance) {
                        component.insert(neighbor);
                        stack.push(neighbor);
                    }
                }
            }
        }
    }
    Ok(component)
}

/// Orthonormal in-plane basis for a unit normal (same construction as
/// `operations::plane_basis`).
pub(super) fn plane_basis(normal: [f64; 3]) -> ([f64; 3], [f64; 3]) {
    let axis = if normal[0].abs() <= normal[1].abs() && normal[0].abs() <= normal[2].abs() {
        [1., 0., 0.]
    } else if normal[1].abs() <= normal[2].abs() {
        [0., 1., 0.]
    } else {
        [0., 0., 1.]
    };
    let u = {
        let c = cross(normal, axis);
        let n = norm(c);
        mul(c, 1. / n)
    };
    (u, cross(normal, u))
}

/// Rigid-map a 3D curve into the 2D UV frame of a planar patch:
/// `uv(p) = (dot(p − origin, u) / len_u, dot(p − origin, v) / len_v)`.
/// Exact for any NURBS: the map is affine, so weights and knots carry over.
pub(super) fn curve_to_uv(
    curve: &nurbs_core::curve::Curve,
    origin: [f64; 3],
    u: [f64; 3],
    v: [f64; 3],
    len_u: f64,
    len_v: f64,
) -> Result<nurbs_core::curve::Curve> {
    let mut mapped = curve.clone();
    for point in &mut mapped.control_points {
        if point.len() != 3 {
            return Err(unsupported("Edge curves must be 3D for pcurve mapping"));
        }
        let d = sub([point[0], point[1], point[2]], origin);
        *point = vec![dot(d, u) / len_u, dot(d, v) / len_v];
    }
    mapped.validate()?;
    Ok(mapped)
}

/// Merge path of [`suppress_by_deletion`]: every rim edge of the cluster
/// that sits on an *outer* loop belongs to a coplanar shard component; each
/// component is merged into one re-authored planar face with the mouth
/// filled. Returns the rebuilt model and the number of removed edges.
pub(super) fn merge_coplanar_rims(
    model: &Model,
    cluster: &BTreeSet<usize>,
    adjacency: &[Vec<usize>],
    rim_edges: &BTreeSet<usize>,
    outer_rim_faces: &BTreeSet<usize>,
    guard: &mut BudgetGuard,
) -> Result<(Model, usize)> {
    let components = shard_components(model, cluster, adjacency, outer_rim_faces, guard)?;
    let mut merged = Vec::with_capacity(components.len());
    for component in &components {
        guard.tick()?;
        merged.push(merge_component(model, component, rim_edges, guard)?);
    }
    assemble_merged(model, cluster, rim_edges, &merged, guard)
}

/// One merged planar face replacing a coplanar shard component.
struct MergedFace {
    surface: nurbs_core::surface::Surface,
    /// (coedges in traversal order) — outer first, holes after.
    rings: Vec<Vec<RingUse>>,
    /// Shell that owned the shards; the merged face joins it.
    shell: usize,
    /// Faces replaced by this merge.
    replaced: BTreeSet<usize>,
    /// Seam edges internal to the component (removed).
    seams: BTreeSet<usize>,
}

/// Stage 1: one component per connected coplanar shard region carrying rim
/// edges on outer loops.
fn shard_components(
    model: &Model,
    cluster: &BTreeSet<usize>,
    adjacency: &[Vec<usize>],
    outer_rim_faces: &BTreeSet<usize>,
    guard: &mut BudgetGuard,
) -> Result<Vec<BTreeSet<usize>>> {
    let mut components: Vec<BTreeSet<usize>> = Vec::new();
    let mut assigned: BTreeSet<usize> = BTreeSet::new();
    for &seed in outer_rim_faces {
        guard.tick()?;
        if assigned.contains(&seed) {
            continue;
        }
        let plane = face_plane_fit(model, seed)?;
        let component = coplanar_component(model, seed, plane, cluster, adjacency, guard)?;
        // Every shard of the component must be planar (checked inside) and
        // the component must swallow every outer-rim face it touches.
        assigned.extend(component.iter().copied());
        components.push(component);
    }
    Ok(components)
}

/// Stage 2: cancel internal seams, drop rim edges, re-chain the remaining
/// boundary into rings, build the merged face.
fn merge_component(
    model: &Model,
    component: &BTreeSet<usize>,
    rim_edges: &BTreeSet<usize>,
    guard: &mut BudgetGuard,
) -> Result<MergedFace> {
    let tolerance = model.tolerance_mm;
    let (uses, shell) = component_outer_uses(model, component, rim_edges, guard)?;
    let (boundary, seams) = cancel_seams(uses)?;
    let rings = chain_rings(model, &boundary, guard)?;
    let seed = *component
        .first()
        .ok_or_else(|| error(SUPPRESS_INVALID, "Empty shard component"))?;
    let seed_plane = face_plane_fit(model, seed)?;
    let ordered_rings = order_rings(model, rings, seed_plane.normal, tolerance)?;
    let surface = merged_surface(model, &ordered_rings, seed_plane.normal, tolerance)?;
    Ok(MergedFace {
        surface,
        rings: ordered_rings,
        shell,
        replaced: component.clone(),
        seams,
    })
}

/// Index of the shell that uses `face`.
fn shell_of_face(model: &Model, face: usize) -> Result<usize> {
    model
        .shells
        .iter()
        .position(|sh| sh.faces.iter().any(|u| u.face == face))
        .ok_or_else(|| {
            error(
                SUPPRESS_INVALID,
                format!("Face {face} is not used by any shell"),
            )
        })
}

/// Directed uses of a loop's non-rim coedges, normalized for the face use.
fn ring_uses(
    model: &Model,
    ring: usize,
    flipped: bool,
    rim_edges: &BTreeSet<usize>,
    out: &mut Vec<RingUse>,
) {
    out.extend(
        model.loops[ring]
            .coedges
            .iter()
            .filter(|c| !rim_edges.contains(&c.edge))
            .map(|c| {
                let reversed = c.reversed ^ flipped;
                RingUse {
                    edge: c.edge,
                    reversed,
                    start: model.edges[c.edge].vertices[usize::from(reversed)],
                }
            }),
    );
}

/// Hole loops: fully rim → dropped with the feature; partially rim → shared
/// mouth, refused; untouched → left out of the merged boundary.
fn check_hole_rims(model: &Model, face: usize, rim_edges: &BTreeSet<usize>) -> Result<()> {
    for &hole in &model.faces[face].holes {
        let coedges = &model.loops[hole].coedges;
        let rim_count = coedges
            .iter()
            .filter(|c| rim_edges.contains(&c.edge))
            .count();
        if rim_count > 0 && rim_count < coedges.len() {
            return Err(unsupported(
                "A hole loop of a shard face is partly feature rim; shared rims are refused",
            ));
        }
    }
    Ok(())
}

/// Directed outer-loop uses of every shard (rim edges skipped) and the one
/// shell owning the component.
fn component_outer_uses(
    model: &Model,
    component: &BTreeSet<usize>,
    rim_edges: &BTreeSet<usize>,
    guard: &mut BudgetGuard,
) -> Result<(Vec<RingUse>, usize)> {
    let mut uses: Vec<RingUse> = Vec::new();
    let mut shell_of: Option<usize> = None;
    for &face in component {
        guard.tick()?;
        let flipped = face_use_reversed(model, face);
        let shell = shell_of_face(model, face)?;
        match shell_of {
            None => shell_of = Some(shell),
            Some(previous) if previous == shell => {}
            Some(_) => {
                return Err(unsupported(
                    "Coplanar shard component spans several shells; merge is refused",
                ));
            }
        }
        check_hole_rims(model, face, rim_edges)?;
        ring_uses(
            model,
            model.faces[face].outer,
            flipped,
            rim_edges,
            &mut uses,
        );
    }
    let Some(shell) = shell_of else {
        return Err(error(SUPPRESS_INVALID, "Empty shard component"));
    };
    Ok((uses, shell))
}

/// Seam cancellation: an edge used twice inside the component is internal
/// (once per adjacent shard, opposite directions).
fn cancel_seams(uses: Vec<RingUse>) -> Result<(Vec<RingUse>, BTreeSet<usize>)> {
    let mut counts: BTreeMap<usize, usize> = BTreeMap::new();
    for use_ in &uses {
        *counts.entry(use_.edge).or_insert(0) += 1;
    }
    let mut seams = BTreeSet::new();
    let mut boundary: Vec<RingUse> = Vec::new();
    for use_ in uses {
        match counts.get(&use_.edge).copied().unwrap_or(0) {
            1 => boundary.push(use_),
            2 => {
                seams.insert(use_.edge);
            }
            _ => {
                return Err(unsupported(format!(
                    "Edge {} is used more than twice inside a shard component",
                    use_.edge
                )));
            }
        }
    }
    Ok((boundary, seams))
}

/// Chain boundary uses into closed rings by vertex connectivity.
fn chain_rings(
    model: &Model,
    boundary: &[RingUse],
    guard: &mut BudgetGuard,
) -> Result<Vec<Vec<RingUse>>> {
    let mut outgoing: BTreeMap<usize, usize> = BTreeMap::new();
    for (i, use_) in boundary.iter().enumerate() {
        if outgoing.insert(use_.start, i).is_some() {
            return Err(unsupported(
                "Shard boundary branches at a vertex; region is not simply mergeable",
            ));
        }
    }
    let end_of = |use_: RingUse| model.edges[use_.edge].vertices[usize::from(!use_.reversed)];
    let mut rings: Vec<Vec<RingUse>> = Vec::new();
    let mut done = vec![false; boundary.len()];
    for (start_i, first) in boundary.iter().enumerate() {
        guard.tick()?;
        if done[start_i] {
            continue;
        }
        let mut ring = Vec::new();
        let mut i = start_i;
        loop {
            done[i] = true;
            let use_ = boundary[i];
            ring.push(use_);
            let next = end_of(use_);
            if next == first.start {
                break;
            }
            i = *outgoing
                .get(&next)
                .ok_or_else(|| unsupported("Shard boundary does not close after rim removal"))?;
            if done[i] {
                return Err(unsupported("Shard boundary self-intersects"));
            }
            if ring.len() > boundary.len() {
                return Err(unsupported("Shard boundary chaining diverged"));
            }
        }
        rings.push(ring);
    }
    Ok(rings)
}

/// Twice the area vector of a closed polygon given by its ring's start
/// vertices (sum of consecutive cross products).
fn ring_area_vector(model: &Model, ring: &[RingUse]) -> [f64; 3] {
    let n = ring.len();
    (0..n).fold([0.; 3], |sum, i| {
        let p = model.vertices[ring[i].start].point;
        let q = model.vertices[ring[(i + 1) % n].start].point;
        add(sum, cross(p, q))
    })
}

/// Classify rings by signed area along the component normal: the one
/// largest positive ring is the outer boundary; negative ones are holes.
/// Two positive rings would mean a disconnected region.
fn order_rings(
    model: &Model,
    rings: Vec<Vec<RingUse>>,
    normal: [f64; 3],
    tolerance: f64,
) -> Result<Vec<Vec<RingUse>>> {
    let mut outers: Vec<Vec<RingUse>> = Vec::new();
    let mut holes: Vec<Vec<RingUse>> = Vec::new();
    for ring in rings {
        let area = 0.5 * dot(ring_area_vector(model, &ring), normal);
        if area > tolerance * tolerance {
            outers.push(ring);
        } else if area < -tolerance * tolerance {
            holes.push(ring);
        } else {
            return Err(unsupported("Shard merge produced a zero-area ring"));
        }
    }
    if outers.len() != 1 {
        return Err(unsupported(
            "Shard component merges into several disconnected regions; refused",
        ));
    }
    outers.extend(holes);
    Ok(outers)
}

/// Re-author the merged face: bilinear patch over the UV bbox of all ring
/// points.
fn merged_surface(
    model: &Model,
    rings: &[Vec<RingUse>],
    plane_normal: [f64; 3],
    tolerance: f64,
) -> Result<nurbs_core::surface::Surface> {
    let outer = rings
        .first()
        .ok_or_else(|| unsupported("Merged shard face is degenerate"))?;
    let area_vec = ring_area_vector(model, outer);
    let length = norm(area_vec);
    if !(length.is_finite() && length > 0.) {
        return Err(unsupported("Merged shard face is degenerate"));
    }
    let normal = mul(area_vec, 1. / length);
    if dot(normal, plane_normal) < 0. {
        return Err(unsupported(
            "Merged shard ring wound against the shard plane normal",
        ));
    }
    let (u, v) = plane_basis(normal);
    let origin = model.vertices[outer[0].start].point;
    let (mut min_u, mut max_u) = (f64::INFINITY, f64::NEG_INFINITY);
    let (mut min_v, mut max_v) = (f64::INFINITY, f64::NEG_INFINITY);
    for use_ in rings.iter().flatten() {
        let d = sub(model.vertices[use_.start].point, origin);
        let (pu, pv) = (dot(d, u), dot(d, v));
        min_u = min_u.min(pu);
        max_u = max_u.max(pu);
        min_v = min_v.min(pv);
        max_v = max_v.max(pv);
    }
    if max_u - min_u <= tolerance || max_v - min_v <= tolerance {
        return Err(unsupported("Merged shard face collapsed"));
    }
    let surface_origin = add(origin, add(mul(u, min_u), mul(v, min_v)));
    let du = mul(u, max_u - min_u);
    let dv = mul(v, max_v - min_v);
    Ok(nurbs_core::surface::Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![surface_origin.to_vec(), add(surface_origin, dv).to_vec()],
            vec![
                add(surface_origin, du).to_vec(),
                add(add(surface_origin, du), dv).to_vec(),
            ],
        ],
        weights: vec![vec![1., 1.], vec![1., 1.]],
        periodic_u: false,
        periodic_v: false,
    })
}

/// Faces, loops, edges and vertices removed by the merge.
struct MergeRemovals {
    faces: BTreeSet<usize>,
    loops: BTreeSet<usize>,
    edges: BTreeSet<usize>,
    vertices: BTreeSet<usize>,
}

/// Cluster + shards removed, rim and seam edges removed, plus every loop of
/// a removed face and every vertex left without a surviving edge.
fn merge_removals(
    model: &Model,
    cluster: &BTreeSet<usize>,
    rim_edges: &BTreeSet<usize>,
    merged: &[MergedFace],
) -> MergeRemovals {
    let mut faces: BTreeSet<usize> = cluster.clone();
    let mut edges: BTreeSet<usize> = rim_edges.clone();
    for merged_face in merged {
        faces.extend(merged_face.replaced.iter().copied());
        edges.extend(merged_face.seams.iter().copied());
    }
    let mut loops: BTreeSet<usize> = BTreeSet::new();
    for &face in &faces {
        let face = &model.faces[face];
        loops.insert(face.outer);
        loops.extend(face.holes.iter().copied());
    }
    let used_vertices: BTreeSet<usize> = model
        .edges
        .iter()
        .enumerate()
        .filter(|(edge, _)| !edges.contains(edge))
        .flat_map(|(_, e)| e.vertices)
        .collect();
    let vertices = edges
        .iter()
        .flat_map(|&edge| model.edges[edge].vertices)
        .filter(|v| !used_vertices.contains(v))
        .collect();
    MergeRemovals {
        faces,
        loops,
        edges,
        vertices,
    }
}

/// Old index → new index over the kept entries of `0..count`.
fn surviving_remap(count: usize, keep: impl Fn(usize) -> bool) -> BTreeMap<usize, usize> {
    (0..count)
        .filter(|&old| keep(old))
        .enumerate()
        .map(|(new, old)| (old, new))
        .collect()
}

/// Stage 3: assemble the staged model: cluster + shards removed, seams
/// removed, merged faces appended, their loops appended, everything remapped.
fn assemble_merged(
    model: &Model,
    cluster: &BTreeSet<usize>,
    rim_edges: &BTreeSet<usize>,
    merged: &[MergedFace],
    guard: &mut BudgetGuard,
) -> Result<(Model, usize)> {
    let removals = merge_removals(model, cluster, rim_edges, merged);
    let edges = surviving_remap(model.edges.len(), |e| !removals.edges.contains(&e));
    // Old shell index → new shell index (compact drops emptied shells).
    let shell_remap = surviving_remap(model.shells.len(), |s| {
        model.shells[s]
            .faces
            .iter()
            .any(|u| !removals.faces.contains(&u.face))
    });
    let mut out = compact(
        model,
        &removals.faces,
        &removals.loops,
        &removals.edges,
        &removals.vertices,
    )?;
    for merged_face in merged {
        guard.tick()?;
        let shell = *shell_remap
            .get(&merged_face.shell)
            .ok_or_else(|| unsupported("Shard shell vanished during suppression"))?;
        append_merged_face(model, &mut out, merged_face, shell, &edges)?;
    }
    out.rebuild_topology_ids();
    out.refresh_change_set(&[model]);
    out.validate()?;
    Ok((out, removals.edges.len()))
}

fn control_point(surface: &nurbs_core::surface::Surface, i: usize, j: usize) -> [f64; 3] {
    let p = &surface.control_points[i][j];
    [p[0], p[1], p[2]]
}

/// Append one merged face's loops and face; hook the face into its shell.
fn append_merged_face(
    model: &Model,
    out: &mut Model,
    merged_face: &MergedFace,
    shell: usize,
    edges: &BTreeMap<usize, usize>,
) -> Result<()> {
    // UV frame data for pcurve mapping, recomputed from the surface.
    let origin_corner = control_point(&merged_face.surface, 0, 0);
    let u_vec = sub(control_point(&merged_face.surface, 1, 0), origin_corner);
    let v_vec = sub(control_point(&merged_face.surface, 0, 1), origin_corner);
    let len_u = norm(u_vec);
    let len_v = norm(v_vec);
    let u = mul(u_vec, 1. / len_u);
    let v = mul(v_vec, 1. / len_v);
    let mut loop_ids = Vec::new();
    for ring in &merged_face.rings {
        let mut coedges = Vec::with_capacity(ring.len());
        for use_ in ring {
            let mut curve = model.edges[use_.edge].curve.clone();
            if use_.reversed {
                curve = curve.reverse()?;
            }
            let mapped = curve_to_uv(&curve, origin_corner, u, v, len_u, len_v)?;
            let edge = *edges
                .get(&use_.edge)
                .ok_or_else(|| error(SUPPRESS_INVALID, "Merged ring edge was removed"))?;
            coedges.push(brep_topology::Coedge::new(edge, use_.reversed, mapped));
        }
        loop_ids.push(out.loops.len());
        out.loops.push(brep_topology::Loop { coedges });
    }
    let face_id = out.faces.len();
    out.faces.push(brep_topology::Face {
        surface: merged_face.surface.clone(),
        outer: loop_ids[0],
        holes: loop_ids[1..].to_vec(),
    });
    // Shard uses were dropped by `compact` (shard faces are in
    // `remove_faces`); the shell gains the merged face instead.
    out.shells[shell].faces.push(brep_topology::FaceUse {
        face: face_id,
        reversed: false,
    });
    Ok(())
}
