use super::*;

/// Options for [`offset_face`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OffsetOptions {
    /// Refuse (with a diagnosis naming the concentric partner) instead of
    /// auto-expanding a single-face offset to the whole concentric pair.
    pub strict: bool,
}

/// Outcome of one successful face offset.
#[derive(Clone, Debug)]
pub struct OffsetFaceReport {
    /// The staged, validated and audited result model.
    pub model: Model,
    /// Input-model index of the requested face.
    pub face: usize,
    /// "cylinder-radius" | "planar" (delegated to [`move_face`]).
    pub kind: &'static str,
    /// Every face whose radius changed: the requested face's coaxial sibling
    /// patches plus the concentric partner's, ascending.
    pub offset_faces: Vec<usize>,
    /// Distinct radii before the offset (ascending), one per coaxial group.
    pub old_radii: Vec<f64>,
    /// Matching radii after the offset.
    pub new_radii: Vec<f64>,
    /// The offset was auto-expanded to a concentric partner.
    pub auto_expanded: bool,
    /// Wall thickness (partner radius gap) before / after, when a pair was
    /// involved. Pair offsets preserve it exactly.
    pub wall_thickness_before_mm: Option<f64>,
    pub wall_thickness_after_mm: Option<f64>,
    /// Edges re-authored at the new radius / moved endpoints.
    pub recomputed_edges: usize,
    /// Cap pcurves re-mapped onto the unchanged planar surfaces.
    pub recomputed_pcurves: usize,
    pub volume_before_mm3: f64,
    pub volume_after_mm3: f64,
    /// Declared analytic estimate: π h Σ ±(r'² − r²) per coaxial group.
    pub delta_volume_estimate_mm3: f64,
    /// Measured delta from `mass_properties` (after − before).
    pub delta_volume_actual_mm3: f64,
    /// Post-operation solid audit certificate (acceptance gate).
    pub audit: SolidAuditCertificate,
}

/// Radial scale of `point` about `axis` from `old_r` to `new_r`: the axial
/// component is kept, the radial component scales by `new_r / old_r`.
pub(super) fn radial_scale(axis: &Axis, old_r: f64, new_r: f64, point: [f64; 3]) -> [f64; 3] {
    let d = sub(point, axis.point);
    let axial = dot(d, axis.direction);
    let radial = sub(d, mul(axis.direction, axial));
    add(
        axis.point,
        add(mul(axis.direction, axial), mul(radial, new_r / old_r)),
    )
}

/// Radial distance of `point` to `axis`.
pub(super) fn radial_distance(axis: &Axis, point: [f64; 3]) -> f64 {
    let d = sub(point, axis.point);
    let axial = dot(d, axis.direction);
    norm(sub(d, mul(axis.direction, axial)))
}

/// Two axis lines coincide within tolerances (direction parallel up to sign,
/// point of B on the line of A) — the 865 coaxiality criterion.
pub(super) fn coaxial_axes(a: &Axis, b: &Axis, linear_tol: f64) -> bool {
    let align = dot(a.direction, b.direction).abs();
    if 1. - align > 1e-6 {
        return false;
    }
    let delta = sub(b.point, a.point);
    let off = sub(delta, mul(a.direction, dot(delta, a.direction)));
    norm(off) <= linear_tol
}

/// Offset `face` radially (cylinders) or along its normal (planes, via the
/// move-face path) by `offset_mm`. See the module section for the supported
/// subset; anything else fails with [`OFFSET_UNSUPPORTED`] and the input is
/// unchanged.
pub fn offset_face(
    model: &Model,
    face: usize,
    offset_mm: f64,
    options: OffsetOptions,
    budget: &Budget,
) -> Result<OffsetFaceReport> {
    let snapshot = ModelSnapshot::new(model.clone())?;
    let source = snapshot.model();
    if face >= source.faces.len() {
        return Err(error(OFFSET_INVALID, format!("Face {face} is out of range")));
    }
    if !(offset_mm.is_finite() && offset_mm.abs() <= 1e6) {
        return Err(error(
            OFFSET_INVALID,
            "Offset must be finite and within ±1000000 mm",
        ));
    }
    let mut guard = budget.guard("offset-face");
    guard.check()?;

    // Classify through the attributed AAG (864/865 attributes).
    let mut aag = crate::aag::Aag::build(source, budget)?;
    aag.attach_face_attrs(source, budget)?;
    let attrs = aag.nodes[face].attrs.as_ref().ok_or_else(|| {
        error(OFFSET_INVALID, format!("Face {face} carries no fitted attributes"))
    })?;
    match attrs.class {
        SurfaceClass::Plane => {
            // Offset of a planar face = move along the outward normal.
            let raw = face_plane_fit(source, face)?;
            let sign = if face_use_reversed(source, face) { -1. } else { 1. };
            let outward = mul(raw.normal, sign);
            let moved = move_face(source, face, outward, offset_mm, budget)?;
            return Ok(OffsetFaceReport {
                model: moved.model,
                face,
                kind: "planar",
                offset_faces: vec![],
                old_radii: vec![],
                new_radii: vec![],
                auto_expanded: false,
                wall_thickness_before_mm: None,
                wall_thickness_after_mm: None,
                recomputed_edges: 0,
                recomputed_pcurves: 0,
                volume_before_mm3: moved.volume_before_mm3,
                volume_after_mm3: moved.volume_after_mm3,
                delta_volume_estimate_mm3: moved.delta_volume_estimate_mm3,
                delta_volume_actual_mm3: moved.delta_volume_actual_mm3,
                audit: moved.audit,
            });
        }
        SurfaceClass::Cylinder => {}
        other => {
            return Err(error(
                OFFSET_UNSUPPORTED,
                format!(
                    "Face {face} is {other:?}; offset-face/1 supports cylinders and planes only"
                ),
            ));
        }
    }
    offset_cylinder(source, face, offset_mm, options, &aag, &mut guard)
}

/// Radial offset of a cylindrical face, with concentric-pair handling.
pub(super) fn offset_cylinder(
    model: &Model,
    face: usize,
    offset_mm: f64,
    options: OffsetOptions,
    aag: &crate::aag::Aag,
    guard: &mut BudgetGuard,
) -> Result<OffsetFaceReport> {
    let linear = model.tolerance_mm * 8.;
    let attrs = aag.nodes[face].attrs.as_ref().expect("checked above");
    let axis = attrs
        .axis
        .ok_or_else(|| error(OFFSET_INVALID, format!("Face {face} has no fitted axis")))?;
    let radius = attrs
        .radius
        .ok_or_else(|| error(OFFSET_INVALID, format!("Face {face} has no fitted radius")))?;

    // 1. Concentric clusters, detected BEFORE any mutation: every
    //    cylindrical face coaxial with the target, clustered by radius.
    let mut groups: Vec<(f64, Vec<usize>)> = Vec::new(); // (radius, faces)
    for (i, node) in aag.nodes.iter().enumerate() {
        guard.tick()?;
        let Some(a) = &node.attrs else { continue };
        if a.class != SurfaceClass::Cylinder {
            continue;
        }
        let (Some(other_axis), Some(other_radius)) = (a.axis, a.radius) else {
            continue;
        };
        if !coaxial_axes(&axis, &other_axis, linear) {
            continue;
        }
        match groups
            .iter_mut()
            .find(|(r, _)| (*r - other_radius).abs() <= 1e-4 * other_radius)
        {
            Some((_, faces)) => faces.push(i),
            None => groups.push((other_radius, vec![i])),
        }
    }
    groups.sort_by(|a, b| a.0.total_cmp(&b.0));
    let Some(target) = groups
        .iter()
        .position(|(r, _)| (*r - radius).abs() <= 1e-4 * radius)
    else {
        return Err(error(
            OFFSET_INVALID,
            format!("Face {face} is missing from its own coaxial cluster"),
        ));
    };
    if groups.len() > 2 {
        return Err(error(
            OFFSET_UNSUPPORTED,
            "Stepped coaxial walls (more than two radii) are outside offset-face/1",
        ));
    }
    let partner = if groups.len() == 2 {
        Some(1 - target)
    } else {
        None
    };
    let auto_expanded = partner.is_some();
    if options.strict {
        if let Some(p) = partner {
            return Err(error(
                OFFSET_UNSUPPORTED,
                format!(
                    "Face {face} is one wall of a concentric pair; strict mode refuses single-member offsets. Partner faces: {:?} — drop strict to offset the pair",
                    groups[p].1
                ),
            ));
        }
    }

    // 2. Axial extents must match within a pair (partially overlapping walls
    //    would change thickness along z after a paired offset — conserva-
    //    tively refused).
    let axial_extent = |faces: &[usize]| -> [f64; 2] {
        let mut lo = f64::INFINITY;
        let mut hi = f64::NEG_INFINITY;
        for &f in faces {
            for p in model.faces[f].surface.control_points.iter().flatten() {
                let t = dot(sub([p[0], p[1], p[2]], axis.point), axis.direction);
                lo = lo.min(t);
                hi = hi.max(t);
            }
        }
        [lo, hi]
    };
    let target_extent = axial_extent(&groups[target].1);
    if let Some(p) = partner {
        let pe = axial_extent(&groups[p].1);
        if (pe[0] - target_extent[0]).abs() > linear || (pe[1] - target_extent[1]).abs() > linear {
            return Err(error(
                OFFSET_UNSUPPORTED,
                "Concentric walls span different axial ranges; partial pairing is refused",
            ));
        }
    }
    let height = target_extent[1] - target_extent[0];

    // 3. Blend guard: a tangent neighbor outside the groups is a
    //    fixed-radius blend whose contact would tear — refuse, naming it.
    let group_faces: BTreeSet<usize> = groups.iter().flat_map(|(_, f)| f.iter().copied()).collect();
    for aag_edge in &aag.edges {
        guard.tick()?;
        if !matches!(aag_edge.class, DihedralClass::Smooth | DihedralClass::Tangent) {
            continue;
        }
        let uses: Vec<usize> = aag_edge.uses.iter().map(|u| u.face).collect();
        let inside = uses.iter().any(|u| group_faces.contains(u));
        let outside = uses.iter().find(|u| !group_faces.contains(u));
        if inside {
            if let Some(&other) = outside {
                return Err(error(
                    OFFSET_UNSUPPORTED,
                    format!(
                        "Face {other} is tangent to the offset group (fixed-radius blend); refitting blends is out of scope"
                    ),
                ));
            }
        }
    }

    // 4. Radii sanity: a wall pushed onto/past the axis is a topology
    //    mutation — refused, not sewn.
    let mut plan: Vec<(f64, f64, Vec<usize>)> = Vec::new(); // (old, new, faces)
    for (old_r, faces) in &groups {
        let new_r = old_r + offset_mm;
        if !(new_r.is_finite() && new_r > linear) {
            return Err(error(
                OFFSET_UNSUPPORTED,
                format!(
                    "Offset consumes the wall at radius {old_r} (new radius {new_r}); refused"
                ),
            ));
        }
        plan.push((*old_r, new_r, faces.clone()));
    }

    // 5. Staged mutation: surfaces scale radially, rim circles re-author at
    //    the new radius, straight edges rebuild from moved endpoints, cap
    //    pcurves re-map. Topology and ids are untouched.
    let mut staged = model.clone();
    for (old_r, new_r, faces) in &plan {
        guard.tick()?;
        for &f in faces {
            for point in staged.faces[f].surface.control_points.iter_mut().flatten() {
                if point.len() != 3 {
                    return Err(error(
                        OFFSET_UNSUPPORTED,
                        format!("Face {f} has non-3D control points"),
                    ));
                }
                let scaled = radial_scale(&axis, *old_r, *new_r, [point[0], point[1], point[2]]);
                *point = scaled.to_vec();
            }
        }
        // Vertices of any group face loop sitting on the old radius move out.
        let mut moved_vertices: BTreeSet<usize> = BTreeSet::new();
        for &f in faces {
            for &ring in std::iter::once(&model.faces[f].outer).chain(&model.faces[f].holes) {
                for coedge in &model.loops[ring].coedges {
                    for &v in &model.edges[coedge.edge].vertices {
                        moved_vertices.insert(v);
                    }
                }
            }
        }
        // Rim vertices may also sit on cap seams not owned by the group;
        // the geometric radius test catches every point on the old circle.
        for (v, vertex) in model.vertices.iter().enumerate() {
            guard.tick()?;
            if (radial_distance(&axis, vertex.point) - old_r).abs() <= linear {
                moved_vertices.insert(v);
            }
        }
        let moved: BTreeMap<usize, [f64; 3]> = moved_vertices
            .iter()
            .map(|&v| (v, radial_scale(&axis, *old_r, *new_r, model.vertices[v].point)))
            .collect();
        for (&v, &point) in &moved {
            staged.vertices[v].point = point;
        }

        // Edges: rebuild every edge touched by moved vertices; a curved edge
        // must live entirely on the old radius (a rim circle), else refuse.
        for (e, edge) in model.edges.iter().enumerate() {
            guard.tick()?;
            let touched = edge.vertices.iter().any(|v| moved.contains_key(&v));
            if !touched {
                continue;
            }
            if edge.degenerate {
                continue; // collapsed pole edges carry no extent
            }
            if edge.curve.degree == 1 {
                let a = staged.vertices[edge.vertices[0]].point.to_vec();
                let b = staged.vertices[edge.vertices[1]].point.to_vec();
                staged.edges[e].curve = crate::line(a, b);
            } else {
                // A rim circle of the group: the *curve* lies on the old
                // radius (control points of a NURBS circle do not — they sit
                // on the control polygon outside the arc, so sample the
                // curve itself). Uniform radial scaling of the control net
                // then re-authors the circle at the new radius exactly.
                let domain = edge.curve.domain();
                let mut all_on_radius = true;
                // Interior samples only: periodic rim curves may exclude the
                // seam endpoint from the active domain.
                for i in 1..8 {
                    guard.tick()?;
                    let t = domain[0] + (domain[1] - domain[0]) * i as f64 / 8.;
                    let point = edge.curve.evaluate(t)?.point;
                    if point.len() != 3
                        || (radial_distance(&axis, [point[0], point[1], point[2]]) - old_r).abs()
                            > linear * 4.
                    {
                        all_on_radius = false;
                        break;
                    }
                }
                if !all_on_radius {
                    return Err(error(
                        OFFSET_UNSUPPORTED,
                        format!(
                            "Edge {e} is curved but not a rim circle of the offset group; re-fit is out of scope"
                        ),
                    ));
                }
                let mut curve = edge.curve.clone();
                for p in &mut curve.control_points {
                    let scaled =
                        radial_scale(&axis, *old_r, *new_r, [p[0], p[1], p[2]]);
                    *p = scaled.to_vec();
                }
                staged.edges[e].curve = curve;
            }
        }
    }

    // 6. Cap re-trim: pcurves of changed edges on faces outside the groups
    //    re-map onto the (unchanged) planar surfaces via the rigid plane
    //    frame. Group faces keep their pcurves — radial scaling preserves
    //    the angular parameterization.
    let mut recomputed_pcurves = 0usize;
    let mut changed_edges: BTreeSet<usize> = BTreeSet::new();
    for (e, edge) in staged.edges.iter().enumerate() {
        if edge.curve != model.edges[e].curve {
            changed_edges.insert(e);
        }
    }
    let recomputed_edges = changed_edges.len();
    for (f, staged_face) in staged.faces.clone().iter().enumerate() {
        guard.tick()?;
        if group_faces.contains(&f) {
            continue;
        }
        // Does the face use any changed edge?
        let uses_changed = std::iter::once(&staged_face.outer)
            .chain(&staged_face.holes)
            .any(|&ring| staged.loops[ring].coedges.iter().any(|c| changed_edges.contains(&c.edge)));
        if !uses_changed {
            continue;
        }
        let attrs = aag.nodes[f].attrs.as_ref();
        if attrs.map(|a| a.class) != Some(SurfaceClass::Plane) {
            return Err(error(
                OFFSET_UNSUPPORTED,
                format!(
                    "Face {f} is not planar but shares an edge with the offset group; re-trim is out of scope"
                ),
            ));
        }
        let s = &staged_face.surface;
        if s.degree_u != 1 || s.degree_v != 1 || s.control_points.len() != 2 || s.control_points[0].len() != 2
        {
            return Err(error(
                OFFSET_UNSUPPORTED,
                format!(
                    "Cap face {f} is not a bilinear patch; pcurve re-map needs the explicit UV frame"
                ),
            ));
        }
        // The cap patch must grow with the rim: a rim circle is inscribed in
        // the patch's UV square, so a larger radius would otherwise leave the
        // surface domain. Every control point scales radially by the factor
        // of the group whose rim it belongs to: points sitting on a group
        // radius match exactly; corner points of the bounding square (beyond
        // every rim) take the outermost rim's factor. Axial components are
        // kept, so the patch stays planar and bilinear. Anything in between
        // (a control net we cannot attribute) is refused.
        let face_groups: BTreeSet<usize> = std::iter::once(&staged_face.outer)
            .chain(&staged_face.holes)
            .flat_map(|&ring| staged.loops[ring].coedges.iter())
            .filter(|c| changed_edges.contains(&c.edge))
            .filter_map(|c| {
                let v = model.edges[c.edge].vertices[0];
                let d = radial_distance(&axis, model.vertices[v].point);
                plan.iter()
                    .position(|(old_r, _, _)| (d - old_r).abs() <= linear)
            })
            .collect();
        if face_groups.is_empty() {
            return Err(error(
                OFFSET_UNSUPPORTED,
                format!("Cap face {f} shares no rim edge with the offset group"),
            ));
        }
        let outer_group = *face_groups.iter().next_back().expect("nonempty");
        let factors: Vec<f64> = plan
            .iter()
            .map(|(old_r, new_r, _)| new_r / old_r)
            .collect();
        let max_old = plan[outer_group].0;
        for p in staged.faces[f].surface.control_points.iter_mut().flatten() {
            if p.len() != 3 {
                return Err(error(
                    OFFSET_UNSUPPORTED,
                    format!("Cap face {f} has non-3D control points"),
                ));
            }
            let d = radial_distance(&axis, [p[0], p[1], p[2]]);
            let on = face_groups
                .iter()
                .find(|&&g| (d - plan[g].0).abs() <= linear);
            let factor = match on {
                Some(&g) => factors[g],
                None if d >= max_old => factors[outer_group],
                None => {
                    return Err(error(
                        OFFSET_UNSUPPORTED,
                        format!(
                            "Cap face {f} has a control point at radius {d} that matches no offset rim; re-trim is out of scope"
                        ),
                    ));
                }
            };
            let scaled = radial_scale(&axis, 1., factor, [p[0], p[1], p[2]]);
            *p = scaled.to_vec();
        }
        let s = &staged.faces[f].surface;
        let origin: [f64; 3] = {
            let p = &s.control_points[0][0];
            [p[0], p[1], p[2]]
        };
        let du = {
            let p = &s.control_points[1][0];
            sub([p[0], p[1], p[2]], origin)
        };
        let dv = {
            let p = &s.control_points[0][1];
            sub([p[0], p[1], p[2]], origin)
        };
        let (len_u, len_v) = (norm(du), norm(dv));
        if len_u <= linear || len_v <= linear {
            return Err(error(
                OFFSET_UNSUPPORTED,
                format!("Cap face {f} has a degenerate UV frame"),
            ));
        }
        let u = mul(du, 1. / len_u);
        let v = mul(dv, 1. / len_v);
        for &ring in std::iter::once(&staged_face.outer).chain(&staged_face.holes) {
            for ci in 0..staged.loops[ring].coedges.len() {
                let coedge = &staged.loops[ring].coedges[ci];
                if !changed_edges.contains(&coedge.edge) {
                    continue;
                }
                let mut curve = staged.edges[coedge.edge].curve.clone();
                if coedge.reversed {
                    curve = curve.reverse()?;
                }
                let mapped = curve_to_uv(&curve, origin, u, v, len_u, len_v)?;
                staged.loops[ring].coedges[ci].pcurve = mapped;
                recomputed_pcurves += 1;
            }
        }
    }

    // 7. Contour: validate → audit → measure. Failure discards the clone.
    staged.validate()?;
    let audit = crate::solid_audit::audit_solid(&staged)?;
    let volume_before = volume_of(model)?;
    let volume_after = volume_of(&staged)?;
    // ΔV estimate: outer groups add, inner (void-enclosing) groups subtract.
    // The bore wall is the group whose radius is smaller within a pair.
    let mut estimate = 0.;
    for (old_r, new_r, _) in &plan {
        estimate += std::f64::consts::PI * height * (new_r * new_r - old_r * old_r);
    }
    if plan.len() == 2 {
        // The pair formula above counts the bore growth as material gain;
        // the inner wall encloses void, so its term flips sign.
        let (old_r, new_r, _) = plan[0]; // ascending: smallest radius first
        estimate -= 2. * std::f64::consts::PI * height * (new_r * new_r - old_r * old_r);
    }
    let thickness = if plan.len() == 2 {
        Some(plan[1].0 - plan[0].0)
    } else {
        None
    };
    let mut offset_faces: Vec<usize> = group_faces.iter().copied().collect();
    offset_faces.sort_unstable();
    guard.check()?;
    Ok(OffsetFaceReport {
        model: staged,
        face,
        kind: "cylinder-radius",
        offset_faces,
        old_radii: plan.iter().map(|(r, _, _)| *r).collect(),
        new_radii: plan.iter().map(|(_, r, _)| *r).collect(),
        auto_expanded,
        wall_thickness_before_mm: thickness,
        wall_thickness_after_mm: thickness.map(|_| plan[1].1 - plan[0].1),
        recomputed_edges,
        recomputed_pcurves,
        volume_before_mm3: volume_before,
        volume_after_mm3: volume_after,
        delta_volume_estimate_mm3: estimate,
        delta_volume_actual_mm3: volume_after - volume_before,
        audit,
    })
}
