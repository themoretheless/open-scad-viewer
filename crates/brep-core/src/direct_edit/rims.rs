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
pub(super) fn coplanar_component(
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
    let tolerance = model.tolerance_mm;

    // 1. Components: one per connected coplanar shard region carrying rim
    //    edges on outer loops.
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
        for &face in &component {
            assigned.insert(face);
        }
        components.push(component);
    }

    // 2. Per component: cancel internal seams, drop rim edges, re-chain the
    //    remaining boundary into rings, build the merged face.
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
    let mut merged = Vec::new();
    for component in &components {
        guard.tick()?;
        // Collect directed outer-loop uses, rim edges skipped.
        let mut uses: Vec<RingUse> = Vec::new();
        let mut kept_hole_rings: Vec<Vec<RingUse>> = Vec::new();
        let mut shell_of: Option<usize> = None;
        for &face in component {
            guard.tick()?;
            let flipped = face_use_reversed(model, face);
            let (shell, _) = model
                .shells
                .iter()
                .enumerate()
                .find_map(|(s, sh)| {
                    sh.faces
                        .iter()
                        .any(|u| u.face == face)
                        .then_some((s, ()))
                })
                .ok_or_else(|| {
                    error(SUPPRESS_INVALID, format!("Face {face} is not used by any shell"))
                })?;
            match shell_of {
                None => shell_of = Some(shell),
                Some(previous) if previous == shell => {}
                Some(_) => {
                    return Err(unsupported(
                        "Coplanar shard component spans several shells; merge is refused",
                    ));
                }
            }
            let record = |ring: usize, out: &mut Vec<RingUse>| {
                let wire = &model.loops[ring];
                let iter: Vec<RingUse> = wire
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
                    })
                    .collect();
                out.extend(iter);
            };
            // Hole loops: fully rim → dropped with the feature; partially
            // rim → shared mouth, refused; untouched → kept as a hole ring.
            for &hole in &model.faces[face].holes {
                let rim_count = model.loops[hole]
                    .coedges
                    .iter()
                    .filter(|c| rim_edges.contains(&c.edge))
                    .count();
                if rim_count == model.loops[hole].coedges.len() {
                    continue;
                }
                if rim_count > 0 {
                    return Err(unsupported(
                        "A hole loop of a shard face is partly feature rim; shared rims are refused",
                    ));
                }
                let mut ring_uses = Vec::new();
                record(hole, &mut ring_uses);
                kept_hole_rings.push(ring_uses);
            }
            record(model.faces[face].outer, &mut uses);
        }
        let Some(shell) = shell_of else {
            return Err(error(SUPPRESS_INVALID, "Empty shard component"));
        };

        // Seam cancellation: an edge used twice inside the component is
        // internal (once per adjacent shard, opposite directions).
        let mut counts: BTreeMap<usize, usize> = BTreeMap::new();
        for use_ in &uses {
            *counts.entry(use_.edge).or_insert(0) += 1;
        }
        let mut seams = BTreeSet::new();
        let mut boundary: Vec<RingUse> = Vec::new();
        for use_ in uses {
            match counts[&use_.edge] {
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

        // Chain boundary uses into closed rings by vertex connectivity.
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
        for start_i in 0..boundary.len() {
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
                if next == boundary[start_i].start {
                    break;
                }
                i = *outgoing.get(&next).ok_or_else(|| {
                    unsupported("Shard boundary does not close after rim removal")
                })?;
                if done[i] {
                    return Err(unsupported("Shard boundary self-intersects"));
                }
                if ring.len() > boundary.len() {
                    return Err(unsupported("Shard boundary chaining diverged"));
                }
            }
            rings.push(ring);
        }

        // Classify rings by signed area along the component normal: the one
        // largest positive ring is the outer boundary; negative ones are
        // holes. Two positive rings would mean a disconnected region.
        let seed_plane = face_plane_fit(model, *component.iter().next().expect("non-empty"))?;
        let area_of = |ring: &[RingUse]| -> f64 {
            let points: Vec<[f64; 3]> = ring
                .iter()
                .map(|use_| model.vertices[use_.start].point)
                .collect();
            let n = points.len();
            let area = (0..n).fold([0.; 3], |sum, i| {
                add(sum, cross(points[i], points[(i + 1) % n]))
            });
            0.5 * dot(area, seed_plane.normal)
        };
        let mut outers: Vec<usize> = Vec::new();
        let mut holes: Vec<usize> = Vec::new();
        for (i, ring) in rings.iter().enumerate() {
            let area = area_of(ring);
            if area > tolerance * tolerance {
                outers.push(i);
            } else if area < -tolerance * tolerance {
                holes.push(i);
            } else {
                return Err(unsupported("Shard merge produced a zero-area ring"));
            }
        }
        if outers.len() != 1 {
            return Err(unsupported(
                "Shard component merges into several disconnected regions; refused",
            ));
        }
        let ordered_rings: Vec<Vec<RingUse>> = std::iter::once(outers[0])
            .chain(holes)
            .map(|i| rings[i].clone())
            .collect();

        // Re-author the merged face: bilinear patch over the UV bbox of all
        // ring points; pcurves re-mapped from the 3D edge curves.
        let outer_points: Vec<[f64; 3]> = ordered_rings[0]
            .iter()
            .map(|use_| model.vertices[use_.start].point)
            .collect();
        let n = outer_points.len();
        let area_vec = (0..n).fold([0.; 3], |sum, i| {
            add(sum, cross(outer_points[i], outer_points[(i + 1) % n]))
        });
        let length = norm(area_vec);
        if !(length.is_finite() && length > 0.) {
            return Err(unsupported("Merged shard face is degenerate"));
        }
        let normal = mul(area_vec, 1. / length);
        if dot(normal, seed_plane.normal) < 0. {
            return Err(unsupported(
                "Merged shard ring wound against the shard plane normal",
            ));
        }
        let (u, v) = plane_basis(normal);
        let origin = outer_points[0];
        let all_points: Vec<[f64; 3]> = ordered_rings
            .iter()
            .flat_map(|ring| ring.iter().map(|use_| model.vertices[use_.start].point))
            .collect();
        let uv_of = |p: [f64; 3]| {
            let d = sub(p, origin);
            [dot(d, u), dot(d, v)]
        };
        let uvs: Vec<[f64; 2]> = all_points.iter().map(|&p| uv_of(p)).collect();
        let min_u = uvs.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min);
        let max_u = uvs.iter().map(|p| p[0]).fold(f64::NEG_INFINITY, f64::max);
        let min_v = uvs.iter().map(|p| p[1]).fold(f64::INFINITY, f64::min);
        let max_v = uvs.iter().map(|p| p[1]).fold(f64::NEG_INFINITY, f64::max);
        if max_u - min_u <= tolerance || max_v - min_v <= tolerance {
            return Err(unsupported("Merged shard face collapsed"));
        }
        let surface_origin = add(origin, add(mul(u, min_u), mul(v, min_v)));
        let du = mul(u, max_u - min_u);
        let dv = mul(v, max_v - min_v);
        let surface = nurbs_core::surface::Surface {
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
        };
        merged.push(MergedFace {
            surface,
            rings: ordered_rings,
            shell,
            replaced: component.clone(),
            seams,
        });
    }

    // 3. Assemble the staged model: cluster + shards removed, seams removed,
    //    merged faces appended, their loops appended, everything remapped.
    let mut remove_faces: BTreeSet<usize> = cluster.clone();
    let mut remove_edges: BTreeSet<usize> = rim_edges.clone();
    let mut remove_loops: BTreeSet<usize> = BTreeSet::new();
    for &face in cluster {
        let face = &model.faces[face];
        remove_loops.insert(face.outer);
        remove_loops.extend(face.holes.iter().copied());
    }
    for merged_face in &merged {
        for &face in &merged_face.replaced {
            remove_faces.insert(face);
            let face = &model.faces[face];
            remove_loops.insert(face.outer);
            remove_loops.extend(face.holes.iter().copied());
        }
        remove_edges.extend(merged_face.seams.iter().copied());
    }
    let mut used_vertices: BTreeSet<usize> = BTreeSet::new();
    for (edge, e) in model.edges.iter().enumerate() {
        if !remove_edges.contains(&edge) {
            used_vertices.extend(e.vertices);
        }
    }
    let remove_vertices: BTreeSet<usize> = remove_edges
        .iter()
        .flat_map(|&edge| model.edges[edge].vertices)
        .filter(|v| !used_vertices.contains(v))
        .collect();

    fn remap(count: usize, removed: &BTreeSet<usize>) -> BTreeMap<usize, usize> {
        let mut map = BTreeMap::new();
        let mut next = 0;
        for old in 0..count {
            if !removed.contains(&old) {
                map.insert(old, next);
                next += 1;
            }
        }
        map
    }
    let edges = remap(model.edges.len(), &remove_edges);

    // Old shell index → new shell index (compact drops emptied shells).
    let mut shell_remap: BTreeMap<usize, usize> = BTreeMap::new();
    {
        let mut next = 0;
        for (old, shell) in model.shells.iter().enumerate() {
            if shell.faces.iter().any(|u| !remove_faces.contains(&u.face)) {
                shell_remap.insert(old, next);
                next += 1;
            }
        }
    }

    let mut out = compact(
        model,
        &remove_faces,
        &remove_loops,
        &remove_edges,
        &remove_vertices,
    )?;

    // Append merged loops and faces; hook faces into their shells.
    for merged_face in &merged {
        guard.tick()?;
        let shell = *shell_remap.get(&merged_face.shell).ok_or_else(|| {
            unsupported("Shard shell vanished during suppression")
        })?;
        let mut loop_ids = Vec::new();
        // UV frame data for pcurve mapping, recomputed from the surface.
        let origin_corner: [f64; 3] = {
            let p = &merged_face.surface.control_points[0][0];
            [p[0], p[1], p[2]]
        };
        let du_cp = &merged_face.surface.control_points[1][0];
        let dv_cp = &merged_face.surface.control_points[0][1];
        let u_vec = sub(
            [du_cp[0], du_cp[1], du_cp[2]],
            origin_corner,
        );
        let v_vec = sub(
            [dv_cp[0], dv_cp[1], dv_cp[2]],
            origin_corner,
        );
        let len_u = norm(u_vec);
        let len_v = norm(v_vec);
        let u = mul(u_vec, 1. / len_u);
        let v = mul(v_vec, 1. / len_v);
        for ring in &merged_face.rings {
            let mut coedges = Vec::with_capacity(ring.len());
            for use_ in ring {
                let mapped = {
                    let mut curve = model.edges[use_.edge].curve.clone();
                    if use_.reversed {
                        curve = curve.reverse()?;
                    }
                    curve_to_uv(&curve, origin_corner, u, v, len_u, len_v)?
                };
                coedges.push(brep_topology::Coedge {
                    edge: edges[&use_.edge],
                    reversed: use_.reversed,
                    pcurve: mapped,
                });
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
    }
    out.rebuild_topology_ids();
    out.refresh_change_set(&[model]);
    out.validate()?;
    Ok((out, remove_edges.len()))
}
