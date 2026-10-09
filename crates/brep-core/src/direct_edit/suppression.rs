use super::*;

/// Returns the rebuilt model, the number of removed edges and the length of
/// the new sharp edge.
pub(super) fn suppress_fillet_chain(
    model: &Model,
    chain: &FilletChain,
    guard: &mut BudgetGuard,
) -> Result<(Model, usize, f64)> {
    let tolerance = model.tolerance_mm;
    let chain_set: BTreeSet<usize> = chain.faces.iter().copied().collect();
    if chain_set.is_empty() {
        return Err(error(SUPPRESS_INVALID, "Fillet chain has no faces"));
    }
    for &face in &chain.faces {
        if face >= model.faces.len() {
            return Err(error(
                SUPPRESS_INVALID,
                format!("Fillet chain face {face} is out of range"),
            ));
        }
    }
    let [s0, s1] = chain.supports;
    if s0 == s1 || chain_set.contains(&s0) || chain_set.contains(&s1) {
        return Err(error(
            SUPPRESS_INVALID,
            format!("Fillet chain supports [{s0}, {s1}] are degenerate"),
        ));
    }

    // 1. Support extensions must meet. This is checked before any topology
    //    work so the diverging case fails fast with the culpable faces named.
    let plane0 = face_plane_fit(model, s0)?;
    let plane1 = face_plane_fit(model, s1)?;
    let Some((line_point, line_dir)) = intersect_planes(plane0, plane1, 1e-9) else {
        return Err(error(
            SUPPRESS_DIVERGENT,
            format!(
                "Support faces {s0} and {s1} do not intersect: parallel or diverging extensions cannot close the gap left by the fillet chain"
            ),
        ));
    };

    // 2. Chain boundary structure: contact edges (against a support) and cap
    //    edges (against a face closing a chain end).
    let adjacency = edge_face_adjacency(model, guard)?;
    let mut contact_edges: BTreeSet<usize> = BTreeSet::new();
    let mut cap_edges: BTreeMap<usize, Vec<usize>> = BTreeMap::new(); // cap face -> edges
    let mut removed_edges: BTreeSet<usize> = BTreeSet::new();
    for (edge, faces) in adjacency.iter().enumerate() {
        guard.tick()?;
        let inside = faces.iter().filter(|f| chain_set.contains(f)).count();
        if inside == 0 {
            continue;
        }
        removed_edges.insert(edge);
        if inside == faces.len() {
            continue; // internal edge between chain faces
        }
        if inside != 1 || faces.len() != 2 {
            return Err(unsupported(format!(
                "Edge {edge} borders the fillet chain non-manifoldly; chain boundary must be manifold"
            )));
        }
        let outside = *faces
            .iter()
            .find(|f| !chain_set.contains(f))
            .expect("one outside face");
        if outside == s0 || outside == s1 {
            contact_edges.insert(edge);
        } else {
            cap_edges.entry(outside).or_default().push(edge);
        }
    }
    if contact_edges.is_empty() {
        return Err(error(
            SUPPRESS_INVALID,
            "Fillet chain is not adjacent to its declared supports",
        ));
    }
    // Iteration 1: exactly two cap faces closing the two chain ends.
    if cap_edges.len() != 2 {
        return Err(unsupported(format!(
            "Fillet chain has {} cap faces; exactly two planar caps are supported (ring fillets and forked ends are refused)",
            cap_edges.len()
        )));
    }

    // 3. Corner points: sharp line ∩ each cap plane. Plane ∩ plane gave one
    //    line and line ∩ plane one point — the branch nearest to the original
    //    neighborhood is the only branch, so no ambiguity needs resolving.
    let mut corner_of_cap = BTreeMap::new();
    for (&cap, _) in &cap_edges {
        guard.tick()?;
        let plane = face_plane_fit(model, cap)?;
        let Some(corner) = line_plane(line_point, line_dir, plane, 1e-9) else {
            return Err(error(
                SUPPRESS_DIVERGENT,
                format!(
                    "Cap face {cap} is parallel to the support intersection line; extensions do not close"
                ),
            ));
        };
        corner_of_cap.insert(cap, corner);
    }
    let corners: Vec<[f64; 3]> = corner_of_cap.values().copied().collect();
    let sharp_length = dist(corners[0], corners[1]);
    if !(sharp_length.is_finite() && sharp_length > tolerance * 8.) {
        return Err(unsupported(
            "Support extensions meet the caps in coincident points; the sharp edge would be degenerate",
        ));
    }

    // 4. Vertex substitution: every vertex incident to a removed edge must
    //    sit on exactly one cap (its substitute is that cap's corner point).
    let mut substitute: BTreeMap<usize, [f64; 3]> = BTreeMap::new();
    for &edge in &removed_edges {
        guard.tick()?;
        for &vertex in &model.edges[edge].vertices {
            if substitute.contains_key(&vertex) {
                continue;
            }
            let mut caps_of_vertex: BTreeSet<usize> = BTreeSet::new();
            for (other, faces) in adjacency.iter().enumerate() {
                if !cap_edges.values().flatten().any(|e| *e == other) {
                    continue;
                }
                if model.edges[other].vertices.contains(&vertex) {
                    let cap = *faces
                        .iter()
                        .find(|f| !chain_set.contains(f))
                        .expect("cap edge has an outside face");
                    caps_of_vertex.insert(cap);
                }
            }
            if caps_of_vertex.len() != 1 {
                return Err(unsupported(format!(
                    "Vertex {vertex} of the fillet neighborhood touches {} caps; chain-split interior vertices are refused in this iteration",
                    caps_of_vertex.len()
                )));
            }
            substitute.insert(vertex, corner_of_cap[caps_of_vertex.iter().next().expect("one cap")]);
        }
    }

    // 5. Every surviving face of the model must be planar — the neighborhood
    //    is rebuilt from trimmed polygons, which re-authors the extended
    //    supports instead of extrapolating their B-splines off-domain.
    for (face, _) in model.faces.iter().enumerate() {
        guard.tick()?;
        if !chain_set.contains(&face) {
            face_plane_fit(model, face)?;
        }
    }

    // 6. Rebuild: polygons of all surviving faces with substitutions.
    let mut polygons = Vec::with_capacity(model.faces.len() - chain_set.len());
    for shell in &model.shells {
        guard.tick()?;
        for usage in &shell.faces {
            if chain_set.contains(&usage.face) {
                continue;
            }
            let face = &model.faces[usage.face];
            let rings = |ring_id: usize, is_hole: bool| -> Result<Vec<[f64; 3]>> {
                let ids = loop_vertices(model, ring_id)?;
                if is_hole && ids.iter().any(|v| substitute.contains_key(v)) {
                    return Err(unsupported(
                        "Fillet chain meets a hole loop; only outer-loop neighborhoods are supported",
                    ));
                }
                let mut points: Vec<[f64; 3]> = ids
                    .iter()
                    .map(|&v| substitute.get(&v).copied().unwrap_or(model.vertices[v].point))
                    .collect();
                // Collapse consecutive duplicates (both arc endpoints map to
                // the same sharp corner), including across the wrap.
                points.dedup_by(|a, b| dist(*a, *b) <= tolerance * 4.);
                while points.len() > 1
                    && dist(points[0], points[points.len() - 1]) <= tolerance * 4.
                {
                    points.pop();
                }
                if points.len() < 3 {
                    return Err(unsupported(
                        "Suppression consumed a neighboring face; topology mutation is out of scope",
                    ));
                }
                if usage.reversed {
                    points.reverse();
                }
                Ok(points)
            };
            let outer = rings(face.outer, false)?;
            let mut holes = Vec::with_capacity(face.holes.len());
            for &ring in &face.holes {
                holes.push(rings(ring, true)?);
            }
            polygons.push(PlanarBoundary { outer, holes });
        }
    }
    let mut staged = model_from_trimmed_polygons(polygons, tolerance)?;
    staged.refresh_change_set(&[model]);
    staged.validate()?;
    Ok((staged, removed_edges.len(), sharp_length))
}

// ---------------------------------------------------------------------------
// Hole / pocket suppression: pure subgraph deletion, no extension needed.
// ---------------------------------------------------------------------------

/// Remove `cluster` faces, their loops and edges, and drop the rim
/// hole-loops on neighboring faces. Returns the compacted model and the
/// number of removed edges.
pub(super) fn suppress_by_deletion(
    model: &Model,
    cluster: &BTreeSet<usize>,
    guard: &mut BudgetGuard,
) -> Result<(Model, usize)> {
    if cluster.is_empty() {
        return Err(error(SUPPRESS_INVALID, "Feature has no faces"));
    }
    for &face in cluster {
        if face >= model.faces.len() {
            return Err(error(
                SUPPRESS_INVALID,
                format!("Feature face {face} is out of range"),
            ));
        }
    }
    let adjacency = edge_face_adjacency(model, guard)?;

    // Rim edges: exactly one adjacent face inside the cluster, one outside.
    let mut removed_edges: BTreeSet<usize> = BTreeSet::new();
    let mut drop_loops: BTreeSet<usize> = BTreeSet::new();
    let mut outer_rim_faces: BTreeSet<usize> = BTreeSet::new();
    for (edge, faces) in adjacency.iter().enumerate() {
        guard.tick()?;
        let inside = faces.iter().filter(|f| cluster.contains(f)).count();
        if inside == 0 {
            continue;
        }
        removed_edges.insert(edge);
        if inside == faces.len() {
            continue;
        }
        if inside != 1 || faces.len() != 2 {
            return Err(unsupported(format!(
                "Edge {edge} borders the feature non-manifoldly; rim must be manifold"
            )));
        }
        let outside = *faces
            .iter()
            .find(|f| !cluster.contains(f))
            .expect("one outside face");
        // The outside face references this edge either through a hole loop
        // entirely owned by the feature (dropping it removes the opening) or
        // through its outer loop — the fragmented-mouth case, handled below
        // by merging the coplanar shard component into one face.
        let face = &model.faces[outside];
        let Some(ring) = std::iter::once(face.outer)
            .chain(face.holes.iter().copied())
            .find(|&ring| model.loops[ring].coedges.iter().any(|c| c.edge == edge))
        else {
            return Err(unsupported(format!(
                "Rim edge {edge} is not referenced by face {outside}"
            )));
        };
        if ring == face.outer {
            outer_rim_faces.insert(outside);
            continue;
        }
        if !model.loops[ring]
            .coedges
            .iter()
            .all(|c| {
                adjacency[c.edge]
                    .iter()
                    .any(|f| cluster.contains(f))
            })
        {
            return Err(unsupported(format!(
                "Rim loop of face {outside} is shared with geometry outside the feature; shared rims are refused"
            )));
        }
        drop_loops.insert(ring);
    }

    if !outer_rim_faces.is_empty() {
        return merge_coplanar_rims(
            model,
            cluster,
            &adjacency,
            &removed_edges,
            &outer_rim_faces,
            guard,
        );
    }

    // Removed loops: every loop of a cluster face plus the dropped rims.
    let mut removed_loops: BTreeSet<usize> = drop_loops;
    for &face in cluster {
        let face = &model.faces[face];
        removed_loops.insert(face.outer);
        removed_loops.extend(face.holes.iter().copied());
    }
    // Removed vertices: endpoints of removed edges not used by a kept edge.
    let mut used_vertices: BTreeSet<usize> = BTreeSet::new();
    for (edge, e) in model.edges.iter().enumerate() {
        if !removed_edges.contains(&edge) {
            used_vertices.extend(e.vertices);
        }
    }
    let removed_vertices: BTreeSet<usize> = removed_edges
        .iter()
        .flat_map(|&edge| model.edges[edge].vertices)
        .filter(|v| !used_vertices.contains(v))
        .collect();

    let staged = compact(
        model,
        cluster,
        &removed_loops,
        &removed_edges,
        &removed_vertices,
    )?;
    staged.validate()?;
    Ok((staged, removed_edges.len()))
}

/// Rebuild the model without the given entities, remapping every reference
/// and preserving the persistent ids of all kept entities.
pub(super) fn compact(
    model: &Model,
    remove_faces: &BTreeSet<usize>,
    remove_loops: &BTreeSet<usize>,
    remove_edges: &BTreeSet<usize>,
    remove_vertices: &BTreeSet<usize>,
) -> Result<Model> {
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
    let vertices = remap(model.vertices.len(), remove_vertices);
    let edges = remap(model.edges.len(), remove_edges);
    let loops = remap(model.loops.len(), remove_loops);
    let faces = remap(model.faces.len(), remove_faces);

    let mut out = Model(
        brep_topology::Model {
            vertices: model
                .vertices
                .iter()
                .enumerate()
                .filter(|(i, _)| !remove_vertices.contains(i))
                .map(|(_, v)| v.clone())
                .collect(),
            edges: model
                .edges
                .iter()
                .enumerate()
                .filter(|(i, _)| !remove_edges.contains(i))
                .map(|(_, e)| brep_topology::Edge {
                    degenerate: e.degenerate,
                    vertices: e.vertices.map(|v| vertices[&v]),
                    curve: e.curve.clone(),
                })
                .collect(),
            loops: model
                .loops
                .iter()
                .enumerate()
                .filter(|(i, _)| !remove_loops.contains(i))
                .map(|(_, l)| brep_topology::Loop {
                    coedges: l
                        .coedges
                        .iter()
                        .map(|c| brep_topology::Coedge {
                            edge: edges[&c.edge],
                            reversed: c.reversed,
                            pcurve: c.pcurve.clone(),
                        })
                        .collect(),
                })
                .collect(),
            faces: model
                .faces
                .iter()
                .enumerate()
                .filter(|(i, _)| !remove_faces.contains(i))
                .map(|(_, f)| brep_topology::Face {
                    surface: f.surface.clone(),
                    outer: loops[&f.outer],
                    holes: f
                        .holes
                        .iter()
                        .filter(|ring| !remove_loops.contains(ring))
                        .map(|ring| loops[ring])
                        .collect(),
                })
                .collect(),
            shells: Vec::new(),
            bodies: Vec::new(),
            tolerance_mm: model.tolerance_mm,
        },
        crate::TopologyIds {
            vertices: kept(&model.1.vertices, remove_vertices),
            edges: kept(&model.1.edges, remove_edges),
            loops: kept(&model.1.loops, remove_loops),
            faces: kept(&model.1.faces, remove_faces),
            shells: Vec::new(),
            bodies: Vec::new(),
            lineage: model.1.lineage.clone(),
            change_set: Default::default(),
        },
    );

    // Shells: drop removed faces; a shell that loses every face disappears.
    let mut shell_map = BTreeMap::new();
    for (old, shell) in model.shells.iter().enumerate() {
        let kept_uses: Vec<_> = shell
            .faces
            .iter()
            .filter(|u| !remove_faces.contains(&u.face))
            .map(|u| brep_topology::FaceUse {
                face: faces[&u.face],
                reversed: u.reversed,
            })
            .collect();
        if kept_uses.is_empty() {
            continue;
        }
        let new = out.shells.len();
        shell_map.insert(old, new);
        out.1.shells.push(model.1.shells[old]);
        out.shells.push(brep_topology::Shell {
            faces: kept_uses,
            closed: shell.closed,
        });
    }
    for (old, body) in model.bodies.iter().enumerate() {
        let Some(&outer_shell) = shell_map.get(&body.outer_shell) else {
            return Err(unsupported(format!(
                "Suppression consumed the outer shell of body {old}"
            )));
        };
        let inner_shells = body
            .inner_shells
            .iter()
            .filter_map(|s| shell_map.get(s).copied())
            .collect();
        out.1.bodies.push(model.1.bodies[old]);
        out.bodies.push(brep_topology::Body {
            outer_shell,
            inner_shells,
        });
    }
    out.refresh_change_set(&[model]);
    Ok(out)
}

pub(super) fn kept<T: Clone>(ids: &[T], removed: &BTreeSet<usize>) -> Vec<T> {
    ids.iter()
        .enumerate()
        .filter(|(i, _)| !removed.contains(i))
        .map(|(_, id)| id.clone())
        .collect()
}
