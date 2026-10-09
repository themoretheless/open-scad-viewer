use super::*;

/// Outcome of one successful face move.
#[derive(Clone, Debug)]
pub struct MoveFaceReport {
    /// The staged, validated and audited result model.
    pub model: Model,
    /// Input-model index of the moved face (owning-body local when the model
    /// holds several bodies — the edit is isolated via `body_edit`).
    pub moved_face: usize,
    /// Coplanar shard faces that moved together with the selected face.
    pub driven_faces: Vec<usize>,
    /// Perpendicular neighbors that bounded the move (AAG classification).
    pub absorbing_faces: Vec<usize>,
    /// Faces that collapsed to zero area and were absorbed by the re-sew.
    pub absorbed_faces: Vec<usize>,
    /// Faces cancelled in annihilating pairs (moved shard + far side of a
    /// consumed thin wall), both input indices per pair flattened.
    pub annihilated_faces: Vec<usize>,
    /// Signed displacement along the outward normal (mm).
    pub signed_distance_mm: f64,
    pub volume_before_mm3: f64,
    pub volume_after_mm3: f64,
    /// Declared analytic estimate: signed distance × driven region area.
    pub delta_volume_estimate_mm3: f64,
    /// Measured delta from `mass_properties` (after − before).
    pub delta_volume_actual_mm3: f64,
    /// Post-operation solid audit certificate (acceptance gate).
    pub audit: SolidAuditCertificate,
}

/// Move `face` by `direction * distance`. Iteration 1: the face must be
/// planar and `direction` must be parallel to its outward normal; the body
/// must be fully planar with straight edges. On any failure the model is
/// unchanged (the staged rebuild is discarded).
pub fn move_face(
    model: &Model,
    face: usize,
    direction: [f64; 3],
    distance: f64,
    budget: &Budget,
) -> Result<MoveFaceReport> {
    let snapshot = ModelSnapshot::new(model.clone())?;
    let source = snapshot.model();
    if face >= source.faces.len() {
        return Err(error(
            MOVE_INVALID,
            format!("Face {face} is out of range"),
        ));
    }
    for (i, v) in direction.iter().enumerate() {
        if !v.is_finite() {
            return Err(error(MOVE_INVALID, format!("direction[{i}] is not finite")));
        }
    }
    if !(distance.is_finite() && distance.abs() <= 1e6) {
        return Err(error(
            MOVE_INVALID,
            "Distance must be finite and within ±1000000 mm",
        ));
    }
    let length = norm(direction);
    if length <= 0. {
        return Err(error(MOVE_INVALID, "Direction must be nonzero"));
    }
    let unit_dir = mul(direction, 1. / length);

    let mut guard = budget.guard("move-face");
    guard.check()?;

    // Multi-body models: isolate the owning body, edit, reassemble without
    // touching other bodies' identities (body_edit contour).
    if source.bodies.len() > 1 {
        let mut staged_report = None;
        let edited = crate::body_edit::edit_face(source, face, |part, local| {
            let report = move_face_local(part, local, unit_dir, distance, budget, &mut guard)?;
            staged_report = Some(report);
            Ok(staged_report.as_ref().expect("report").model.clone())
        })?;
        let mut report = staged_report.expect("closure ran");
        // Re-audit and re-measure on the reassembled model; face indices in
        // the report are owning-body local (documented on the struct).
        edited.validate()?;
        let audit = crate::solid_audit::audit_solid(&edited)?;
        let volume_before = volume_of(source)?;
        let volume_after = volume_of(&edited)?;
        report.model = edited;
        report.audit = audit;
        report.volume_before_mm3 = volume_before;
        report.volume_after_mm3 = volume_after;
        report.delta_volume_actual_mm3 = volume_after - volume_before;
        return Ok(report);
    }
    move_face_local(source, face, unit_dir, distance, budget, &mut guard)
}

/// Core of [`move_face`] on one connected body.
pub(super) fn move_face_local(
    model: &Model,
    face: usize,
    unit_dir: [f64; 3],
    distance: f64,
    budget: &Budget,
    guard: &mut BudgetGuard,
) -> Result<MoveFaceReport> {
    let tolerance = model.tolerance_mm;
    let linear = tolerance * 8.;
    let delta = mul(unit_dir, distance);

    // Iteration 1 admits planar polygonal bodies: every face planar, every
    // edge straight. Curved surroundings fail closed with the face named.
    for (i, f) in model.faces.iter().enumerate() {
        guard.tick()?;
        face_plane_fit(model, i).map_err(|_| {
            error(
                MOVE_UNSUPPORTED,
                format!("Face {i} is not planar; move-face/1 admits planar bodies only"),
            )
        })?;
        for &ring in std::iter::once(&f.outer).chain(&f.holes) {
            if model.loops[ring]
                .coedges
                .iter()
                .any(|c| model.edges[c.edge].curve.degree != 1)
            {
                return Err(error(
                    MOVE_UNSUPPORTED,
                    format!(
                        "Face {i} has curved boundary edges; move-face/1 admits straight edges only"
                    ),
                ));
            }
        }
    }

    // Canonical (outward-oriented) polygon per face.
    let polygons = canonical_polygons(model, &BTreeMap::new(), guard)?;
    let plane = polygons[face].plane;
    let align = dot(unit_dir, plane.normal);
    if align.abs() < 1. - 1e-9 {
        return Err(error(
            MOVE_UNSUPPORTED,
            "Off-normal moves of a planar face are outside move-face/1",
        ));
    }

    // Neighbor classification over the AAG (864/865 attributes): coplanar
    // shards are driven with the face; perpendicular neighbors bound the
    // move (absorbing); anything else is reported as absorbing too, since
    // oblique planar neighbors are still stretched by the rebuild.
    let mut aag = crate::aag::Aag::build(model, budget)?;
    aag.attach_face_attrs(model, budget)?;
    let adjacency = edge_face_adjacency(model, guard)?;
    let driven = coplanar_component(model, face, plane, &BTreeSet::new(), &adjacency, guard)?;
    let mut absorbing = Vec::new();
    for aag_edge in &aag.edges {
        guard.tick()?;
        let uses: Vec<usize> = aag_edge.uses.iter().map(|u| u.face).collect();
        if uses.contains(&face) {
            for &other in &uses {
                if other != face
                    && !driven.contains(&other)
                    && !absorbing.contains(&other)
                {
                    absorbing.push(other);
                }
            }
        }
    }

    // Moved vertices: every vertex touched by a driven face.
    let mut moved_vertices: BTreeSet<usize> = BTreeSet::new();
    for &d in &driven {
        for &ring in std::iter::once(&model.faces[d].outer).chain(&model.faces[d].holes) {
            guard.tick()?;
            for v in loop_vertices(model, ring)? {
                moved_vertices.insert(v);
            }
        }
    }
    let substitute: BTreeMap<usize, [f64; 3]> = moved_vertices
        .iter()
        .map(|&v| (v, add(model.vertices[v].point, delta)))
        .collect();

    // Declared ΔV estimate: signed distance along the outward normal times
    // the driven region's area (translation of a rigid planar region).
    let moved_area: f64 = driven
        .iter()
        .map(|&d| 0.5 * norm(polygons[d].area_vec))
        .sum();
    let signed_distance = dot(delta, plane.normal);

    let moved = canonical_polygons(model, &substitute, guard)?;

    // Absorption: a non-driven face whose polygon collapsed. Flipping: a
    // face whose orientation inverted (the move crossed its geometry) —
    // refused, partial overruns are out of scope.
    let mut absorbed: BTreeSet<usize> = BTreeSet::new();
    for (i, poly) in moved.iter().enumerate() {
        guard.tick()?;
        if driven.contains(&i) {
            continue;
        }
        let area_after = norm(poly.area_vec);
        let area_before = norm(polygons[i].area_vec);
        if area_after <= linear * linear {
            absorbed.insert(i);
            continue;
        }
        if area_before > linear * linear && dot(poly.area_vec, polygons[i].area_vec) < 0. {
            return Err(error(
                MOVE_UNSUPPORTED,
                format!(
                    "Move inverts face {i}; crossing geometry partially is refused (suppression handles full removal)"
                ),
            ));
        }
    }

    // Parallel-face interactions: annihilation when the moved plane lands on
    // an opposite face with an identical boundary; crossing when it passes
    // through. Both are detected on the canonical planes.
    let mut annihilated: BTreeSet<usize> = BTreeSet::new();
    for &d in &driven {
        guard.tick()?;
        if absorbed.contains(&d) {
            continue;
        }
        for (i, poly) in moved.iter().enumerate() {
            guard.tick()?;
            if driven.contains(&i) || absorbed.contains(&i) || annihilated.contains(&i) {
                continue;
            }
            let n2 = polygons[i].plane.normal;
            let parallel = dot(plane.normal, n2).abs();
            if parallel < 1. - 1e-9 {
                continue;
            }
            let o1 = dot(plane.normal, polygons[d].plane_point());
            let o2 = dot(plane.normal, poly.plane_point());
            let gap_before = o2 - o1;
            let gap_after = gap_before - dot(delta, plane.normal);
            if gap_before.abs() <= linear {
                continue; // already coplanar (driven shards): not an interaction
            }
            if gap_after.abs() <= linear {
                // Coincident after the move: exact annihilation or refusal.
                let opposite = dot(plane.normal, n2) < 0.;
                if opposite
                    && same_point_set(&moved[d].boundary.outer, &poly.boundary.outer, linear)
                {
                    annihilated.insert(d);
                    annihilated.insert(i);
                } else {
                    return Err(error(
                        MOVE_UNSUPPORTED,
                        format!(
                            "Move lands on face {i} with a different boundary; partial annihilation is refused"
                        ),
                    ));
                }
            } else if gap_before.signum() != gap_after.signum() {
                return Err(error(
                    MOVE_UNSUPPORTED,
                    format!(
                        "Move crosses face {i}; the limit is the absorbing neighbor, shorten the distance"
                    ),
                ));
            }
        }
    }

    let mut dropped: BTreeSet<usize> = absorbed.clone();
    dropped.extend(annihilated.iter().copied());
    if dropped.contains(&face) && annihilated.contains(&face) {
        // The selected face annihilated with its counterpart: legal (thin
        // wall consumed end to end).
    }
    let kept: Vec<PlanarBoundary> = moved
        .iter()
        .enumerate()
        .filter(|(i, _)| !dropped.contains(i))
        .map(|(_, p)| p.boundary.clone())
        .collect();
    if kept.len() < 4 {
        return Err(error(
            MOVE_UNSUPPORTED,
            "Move consumes the whole body; refused instead of returning an empty solid",
        ));
    }
    let mut staged = model_from_trimmed_polygons(kept, tolerance)
        .map_err(|e| Error::new(MOVE_UNSUPPORTED, format!("Move rebuild failed: {}", e.message)))?;
    restore_unchanged_ids(&mut staged, model, guard)?;
    staged.refresh_change_set(&[model]);
    staged.validate()?;
    let audit = crate::solid_audit::audit_solid(&staged)?;
    let volume_before = volume_of(model)?;
    let volume_after = volume_of(&staged)?;
    guard.check()?;
    Ok(MoveFaceReport {
        model: staged,
        moved_face: face,
        driven_faces: driven.iter().copied().filter(|&d| d != face).collect(),
        absorbing_faces: absorbing,
        absorbed_faces: absorbed.iter().copied().collect(),
        annihilated_faces: annihilated.iter().copied().collect(),
        signed_distance_mm: signed_distance,
        volume_before_mm3: volume_before,
        volume_after_mm3: volume_after,
        delta_volume_estimate_mm3: signed_distance * moved_area,
        delta_volume_actual_mm3: volume_after - volume_before,
        audit,
    })
}

/// One face's canonical (outward-oriented) polygonal boundary.
struct CanonicalPolygon {
    boundary: PlanarBoundary,
    /// Signed area vector of the outer ring (canonical orientation).
    area_vec: [f64; 3],
    /// Canonical outward plane of the face.
    plane: PlaneEq,
}

impl CanonicalPolygon {
    fn plane_point(&self) -> [f64; 3] {
        // Any boundary point lies on the plane; the first outer vertex is
        // guaranteed to exist (validated rings have ≥ 3 points).
        self.boundary.outer[0]
    }
}

/// Boundary of every face in canonical orientation (reversed face uses
/// flipped), with optional vertex substitution. Faces are indexed as in
/// `model.faces`; shell order is irrelevant.
pub(super) fn canonical_polygons(
    model: &Model,
    substitute: &BTreeMap<usize, [f64; 3]>,
    guard: &mut BudgetGuard,
) -> Result<Vec<CanonicalPolygon>> {
    let mut reversed_of = vec![false; model.faces.len()];
    for shell in &model.shells {
        for usage in &shell.faces {
            reversed_of[usage.face] = usage.reversed;
        }
    }
    let mut out = Vec::with_capacity(model.faces.len());
    for (face_id, face) in model.faces.iter().enumerate() {
        guard.tick()?;
        let rings = |ring_id: usize| -> Result<Vec<[f64; 3]>> {
            let mut points: Vec<[f64; 3]> = loop_vertices(model, ring_id)?
                .iter()
                .map(|&v| substitute.get(&v).copied().unwrap_or(model.vertices[v].point))
                .collect();
            // Substitution can collapse a ring segment (a shard shrinking to
            // a triangle): weld consecutive duplicates, including the wrap.
            let tol = model.tolerance_mm * 4.;
            points.dedup_by(|a, b| dist(*a, *b) <= tol);
            while points.len() > 1
                && dist(points[0], points[points.len() - 1]) <= tol
            {
                points.pop();
            }
            if reversed_of[face_id] {
                points.reverse();
            }
            Ok(points)
        };
        let outer = rings(face.outer)?;
        let mut holes = Vec::with_capacity(face.holes.len());
        for &ring in &face.holes {
            holes.push(rings(ring)?);
        }
        // Rings collapsed below 3 points by substitution are legal: they are
        // zero-area and get absorbed by the caller, never rebuilt.
        let n = outer.len();
        let area_vec = (0..n).fold([0.; 3], |sum, i| {
            add(sum, cross(outer[i], outer[(i + 1) % n]))
        });
        let length = norm(area_vec);
        // Degenerate (zero-area) rings are legal here: move_face detects and
        // absorbs them. They carry a zero normal placeholder, never used for
        // geometry decisions (absorbed faces are skipped before that).
        let normal = if length.is_finite() && length > 0. {
            mul(area_vec, 1. / length)
        } else {
            [0., 0., 0.]
        };
        let plane_point = outer[0];
        out.push(CanonicalPolygon {
            boundary: PlanarBoundary { outer, holes },
            area_vec,
            plane: PlaneEq {
                normal,
                offset: dot(normal, plane_point),
            },
        });
    }
    Ok(out)
}

/// Do two rings cover the same point set (order-free, within `tolerance`)?
pub(super) fn same_point_set(a: &[[f64; 3]], b: &[[f64; 3]], tolerance: f64) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut used = vec![false; b.len()];
    a.iter().all(|p| {
        b.iter()
            .enumerate()
            .position(|(i, q)| !used[i] && dist(*p, *q) <= tolerance)
            .map(|i| used[i] = true)
            .is_some()
    })
}

/// Restore persistent ids of geometrically untouched entities after a full
/// polygon rebuild: vertices match by exact point, edges by endpoint pair +
/// identical curve, loops by identical sorted edge sets, faces by identical
/// (outer, holes) loop sets, shells/bodies by identical face sets. Ambiguous
/// or changed entities keep their fresh rebuild ids. This is the 871 naming
/// criterion: ids of faces the move never touched survive the rebuild.
pub(super) fn restore_unchanged_ids(staged: &mut Model, source: &Model, guard: &mut BudgetGuard) -> Result<()> {
    // Vertices: exact point equality, unique in both directions.
    let mut source_by_point: BTreeMap<[u64; 3], Vec<usize>> = BTreeMap::new();
    for (i, v) in source.vertices.iter().enumerate() {
        source_by_point.entry(v.point.map(|x| x.to_bits())).or_default().push(i);
    }
    let mut staged_by_point: BTreeMap<[u64; 3], Vec<usize>> = BTreeMap::new();
    for (i, v) in staged.vertices.iter().enumerate() {
        staged_by_point.entry(v.point.map(|x| x.to_bits())).or_default().push(i);
    }
    let mut vertex_map: Vec<Option<usize>> = vec![None; staged.vertices.len()];
    for (key, staged_ids) in &staged_by_point {
        if staged_ids.len() != 1 {
            continue;
        }
        if let Some(source_ids) = source_by_point.get(key) {
            if source_ids.len() == 1 {
                vertex_map[staged_ids[0]] = Some(source_ids[0]);
            }
        }
    }

    // Edges: mapped endpoint pair plus bit-identical curve (rebuilt straight
    // edges are deterministic given the same endpoints).
    let mut edge_map: Vec<Option<usize>> = vec![None; staged.edges.len()];
    for (i, edge) in staged.edges.iter().enumerate() {
        guard.tick()?;
        let (Some(a), Some(b)) = (
            vertex_map[edge.vertices[0]],
            vertex_map[edge.vertices[1]],
        ) else {
            continue;
        };
        let key = [a.min(b), a.max(b)];
        let matches: Vec<usize> = source
            .edges
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                let k = [e.vertices[0].min(e.vertices[1]), e.vertices[0].max(e.vertices[1])];
                k == key && e.curve == edge.curve && e.degenerate == edge.degenerate
            })
            .map(|(j, _)| j)
            .collect();
        if matches.len() == 1 {
            edge_map[i] = Some(matches[0]);
        }
    }

    // Loops: identical sorted mapped edge sets.
    let loop_key = |wire: &brep_topology::Loop<nurbs_core::curve::Curve>,
                    map: &Vec<Option<usize>>|
     -> Option<Vec<usize>> {
        let mut edges: Vec<usize> = wire
            .coedges
            .iter()
            .map(|c| map[c.edge])
            .collect::<Option<Vec<_>>>()?;
        edges.sort_unstable();
        Some(edges)
    };
    let source_loop_keys: BTreeMap<Vec<usize>, Vec<usize>> = {
        let mut map: BTreeMap<Vec<usize>, Vec<usize>> = BTreeMap::new();
        let identity: Vec<Option<usize>> = (0..source.edges.len()).map(Some).collect();
        for (j, wire) in source.loops.iter().enumerate() {
            if let Some(key) = loop_key(wire, &identity) {
                map.entry(key).or_default().push(j);
            }
        }
        map
    };
    let mut loop_map: Vec<Option<usize>> = vec![None; staged.loops.len()];
    for (i, wire) in staged.loops.iter().enumerate() {
        guard.tick()?;
        if let Some(key) = loop_key(wire, &edge_map) {
            if let Some(matches) = source_loop_keys.get(&key) {
                if matches.len() == 1 {
                    loop_map[i] = Some(matches[0]);
                }
            }
        }
    }

    // Faces: identical outer + hole loops (all matched).
    let face_key = |face: &brep_topology::Face<nurbs_core::surface::Surface>,
                    map: &Vec<Option<usize>>|
     -> Option<(usize, Vec<usize>)> {
        let outer = map[face.outer]?;
        let mut holes: Vec<usize> = face.holes.iter().map(|&l| map[l]).collect::<Option<_>>()?;
        holes.sort_unstable();
        Some((outer, holes))
    };
    let identity_loops: Vec<Option<usize>> = (0..source.loops.len()).map(Some).collect();
    let mut source_face_keys: BTreeMap<(usize, Vec<usize>), Vec<usize>> = BTreeMap::new();
    for (j, f) in source.faces.iter().enumerate() {
        if let Some(key) = face_key(f, &identity_loops) {
            source_face_keys.entry(key).or_default().push(j);
        }
    }
    let mut face_map: Vec<Option<usize>> = vec![None; staged.faces.len()];
    for (i, f) in staged.faces.iter().enumerate() {
        guard.tick()?;
        if let Some(key) = face_key(f, &loop_map) {
            if let Some(matches) = source_face_keys.get(&key) {
                if matches.len() == 1 {
                    face_map[i] = Some(matches[0]);
                }
            }
        }
    }

    // Shells and bodies: identical mapped face / shell sets.
    let mut shell_map: Vec<Option<usize>> = vec![None; staged.shells.len()];
    for (i, shell) in staged.shells.iter().enumerate() {
        let Some(mut faces) = shell
            .faces
            .iter()
            .map(|u| face_map[u.face])
            .collect::<Option<Vec<_>>>()
        else {
            continue;
        };
        faces.sort_unstable();
        let matches: Vec<usize> = source
            .shells
            .iter()
            .enumerate()
            .filter(|(_, s)| {
                let mut f: Vec<usize> = s.faces.iter().map(|u| u.face).collect();
                f.sort_unstable();
                f == faces
            })
            .map(|(j, _)| j)
            .collect();
        if matches.len() == 1 {
            shell_map[i] = Some(matches[0]);
        }
    }

    for (i, mapped) in vertex_map.iter().enumerate() {
        if let Some(j) = mapped {
            staged.1.vertices[i] = source.1.vertices[*j];
        }
    }
    for (i, mapped) in edge_map.iter().enumerate() {
        if let Some(j) = mapped {
            staged.1.edges[i] = source.1.edges[*j];
        }
    }
    for (i, mapped) in loop_map.iter().enumerate() {
        if let Some(j) = mapped {
            staged.1.loops[i] = source.1.loops[*j];
        }
    }
    for (i, mapped) in face_map.iter().enumerate() {
        if let Some(j) = mapped {
            staged.1.faces[i] = source.1.faces[*j];
        }
    }
    for (i, mapped) in shell_map.iter().enumerate() {
        if let Some(j) = mapped {
            staged.1.shells[i] = source.1.shells[*j];
        }
    }
    let mut body_ids: Vec<Option<usize>> = vec![None; staged.bodies.len()];
    for (i, body) in staged.bodies.iter().enumerate() {
        let Some(outer) = shell_map.get(body.outer_shell).copied().flatten() else {
            continue;
        };
        let Some(mut inners) = body
            .inner_shells
            .iter()
            .map(|&s| shell_map[s])
            .collect::<Option<Vec<_>>>()
        else {
            continue;
        };
        inners.sort_unstable();
        let matches: Vec<usize> = source
            .bodies
            .iter()
            .enumerate()
            .filter(|(_, b)| {
                let mut s: Vec<usize> = b.inner_shells.clone();
                s.sort_unstable();
                b.outer_shell == outer && s == inners
            })
            .map(|(j, _)| j)
            .collect();
        if matches.len() == 1 {
            body_ids[i] = Some(matches[0]);
        }
    }
    for (i, mapped) in body_ids.iter().enumerate() {
        if let Some(j) = mapped {
            staged.1.bodies[i] = source.1.bodies[*j];
        }
    }
    Ok(())
}
