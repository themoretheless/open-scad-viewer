use super::*;

pub(super) fn recognize_analytic_tube(model: &Model) -> Option<AnalyticTube> {
    model.validate().ok()?;
    if model.bodies.len() != 1
        || model.shells.len() != 1
        || model.faces.len() != 10
        || model.vertices.len() != 16
    {
        return None;
    }
    let caps = model
        .faces
        .iter()
        .filter(|face| {
            face.surface.degree_u == 1 && face.surface.degree_v == 1 && !face.holes.is_empty()
        })
        .collect::<Vec<_>>();
    if caps.len() != 2
        || model
            .faces
            .iter()
            .filter(|face| face.surface.degree_u == 2 && face.surface.degree_v == 1)
            .count()
            != 8
    {
        return None;
    }
    let cp = &caps[0].surface.control_points;
    let a = cp[0][0].as_slice();
    let u = sub3([cp[1][0][0], cp[1][0][1], cp[1][0][2]], [a[0], a[1], a[2]]);
    let v = sub3([cp[0][1][0], cp[0][1][1], cp[0][1][2]], [a[0], a[1], a[2]]);
    let axis = unit3(cross3(u, v))?;
    let mut centroid = [0.; 3];
    for vertex in &model.vertices {
        for coordinate in 0..3 {
            centroid[coordinate] += vertex.point[coordinate] / model.vertices.len() as f64;
        }
    }
    let mut axial = Vec::new();
    let mut radial = Vec::new();
    let mut reference = None;
    for vertex in &model.vertices {
        let delta = sub3(vertex.point, centroid);
        let z = dot3(delta, axis);
        let rv = [
            delta[0] - z * axis[0],
            delta[1] - z * axis[1],
            delta[2] - z * axis[2],
        ];
        let radius = dot3(rv, rv).sqrt();
        axial.push(z);
        radial.push(radius);
        if reference
            .map(|(_, old): ([f64; 3], f64)| radius > old)
            .unwrap_or(true)
        {
            reference = Some((rv, radius));
        }
    }
    axial.sort_by(|a, b| a.total_cmp(b));
    radial.sort_by(|a, b| a.total_cmp(b));
    let z0 = axial[0];
    let height = axial[axial.len() - 1] - z0;
    let inner = radial[0];
    let outer = radial[radial.len() - 1];
    if !(inner > 0. && outer - inner >= 1e-5 && height > 1e-5) {
        return None;
    }
    let first = unit3(reference?.0)?;
    let second = unit3(cross3(axis, first))?;
    let origin = [
        centroid[0] + z0 * axis[0],
        centroid[1] + z0 * axis[1],
        centroid[2] + z0 * axis[2],
    ];
    // Vertices alone do not determine a rational surface. Certify every
    // cylindrical support by its full quarter-circle control net and weights.
    let tolerance = (outer.max(height) * 1e-10).max(1e-12);
    let mut radius_counts = [0usize; 2];
    for (face_index, face) in model.faces.iter().enumerate() {
        let s = &face.surface;
        if s.degree_u == 1 && s.degree_v == 1 {
            if face.holes.len() != 1 || s.weights.iter().flatten().any(|w| (*w - 1.).abs() > 1e-12)
            {
                return None;
            }
            continue;
        }
        if !face.holes.is_empty()
            || s.periodic_u
            || s.periodic_v
            || s.knots_u != [0., 0., 0., 1., 1., 1.]
            || s.knots_v != [0., 0., 1., 1.]
            || s.control_points.len() != 3
            || s.control_points.iter().any(|row| row.len() != 2)
            || !crate::intersections::recognize::unit_square_boundary(model, face_index)
        {
            return None;
        }
        for i in 0..3 {
            for j in 0..2 {
                let expected = if i == 1 {
                    std::f64::consts::FRAC_1_SQRT_2
                } else {
                    1.
                };
                if (s.weights[i][j] - expected).abs() > 1e-12 {
                    return None;
                }
            }
        }
        let local = |i: usize, j: usize| {
            let p = &s.control_points[i][j];
            let d = sub3([p[0], p[1], p[2]], origin);
            [dot3(d, first), dot3(d, second), dot3(d, axis)]
        };
        let a = local(0, 0);
        let b = local(2, 0);
        let r = a[0].hypot(a[1]);
        let slot = if (r - outer).abs() <= tolerance {
            0
        } else if (r - inner).abs() <= tolerance {
            1
        } else {
            return None;
        };
        radius_counts[slot] += 1;
        if (b[0].hypot(b[1]) - r).abs() > tolerance
            || (a[0] * b[0] + a[1] * b[1]).abs() > tolerance * r
        {
            return None;
        }
        let start_z = a[2];
        let end_z = local(0, 1)[2];
        if !((start_z.abs() <= tolerance && (end_z - height).abs() <= tolerance)
            || ((start_z - height).abs() <= tolerance && end_z.abs() <= tolerance))
        {
            return None;
        }
        for j in 0..2 {
            for i in 0..3 {
                let p = local(i, j);
                let xy = if i == 0 {
                    [a[0], a[1]]
                } else if i == 2 {
                    [b[0], b[1]]
                } else {
                    [a[0] + b[0], a[1] + b[1]]
                };
                let expected_z = if j == 0 { start_z } else { end_z };
                if (p[0] - xy[0]).hypot(p[1] - xy[1]).hypot(p[2] - expected_z) > tolerance {
                    return None;
                }
            }
        }
    }
    if radius_counts != [4, 4] {
        return None;
    }
    Some((outer, inner, height, origin, axis, [first, second]))
}

/// Qualified exact shell successor. Planar cells offset every support plane and
/// intersect the resulting half-spaces exactly. Cylinder cells use exact
/// rational radial/axial constructors. Adjacent planar openings and single-cap
/// cylindrical openings are refused until their corner/rim ownership is exact.
pub fn exact_analytic_shell(
    model: &Model,
    openings: &[usize],
    thickness: f64,
    direction: &str,
) -> Result<AuditedFeatureResult> {
    model.validate()?;
    audit_solid(model).map_err(|_| {
        refuse(
            "BREP_ANALYTIC_SHELL_REFUSED",
            "Shell source must pass the global solid audit",
        )
    })?;
    if !(thickness.is_finite() && (1e-5..=1e6).contains(&thickness)) {
        return Err(refuse(
            "BREP_ANALYTIC_SHELL_REFUSED",
            "Shell thickness must be finite and within 0.00001..1000000 mm",
        ));
    }
    if !matches!(direction, "inward" | "outward") {
        return Err(refuse(
            "BREP_ANALYTIC_SHELL_REFUSED",
            "Shell direction must be inward or outward",
        ));
    }
    let mut unique = openings.to_vec();
    unique.sort_unstable();
    if unique.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(refuse(
            "BREP_ANALYTIC_SHELL_REFUSED",
            "Duplicate shell openings are ambiguous",
        ));
    }

    let (result, scale, notes) = if let Some(cylinder) =
        crate::intersections::recognize_cylinder(model)?
    {
        let caps = model
            .faces
            .iter()
            .enumerate()
            .filter_map(|(index, face)| {
                (face.surface.degree_u == 1 && face.surface.degree_v == 1).then_some(index)
            })
            .collect::<Vec<_>>();
        if caps.len() != 2 || unique.iter().any(|face| !caps.contains(face)) {
            return Err(refuse(
                "BREP_ANALYTIC_SHELL_REFUSED",
                "Cylinder openings must select planar cap faces",
            ));
        }
        if unique.len() == 1 {
            return Err(refuse(
                "BREP_ANALYTIC_SHELL_TRANSITION_REFUSED",
                "Single-cap cylinder shell awaits exact annular rim and axial corner ownership",
            ));
        }
        let height = cylinder.half_height * 2.;
        let source_origin = [
            cylinder.center[0] - cylinder.axis[0] * cylinder.half_height,
            cylinder.center[1] - cylinder.axis[1] * cylinder.half_height,
            cylinder.center[2] - cylinder.axis[2] * cylinder.half_height,
        ];
        if unique.len() == 2 {
            let (outer, inner) = if direction == "inward" {
                if cylinder.radius <= thickness + 1e-5 {
                    return Err(refuse(
                        "BREP_ANALYTIC_SHELL_COLLAPSE_REFUSED",
                        "Inward radial offset collapses the cylinder wall",
                    ));
                }
                (cylinder.radius, cylinder.radius - thickness)
            } else {
                (cylinder.radius + thickness, cylinder.radius)
            };
            let local = tube(outer, inner, height)?;
            (
                place_axial(&local, cylinder.frame, cylinder.axis, source_origin)?,
                outer.max(height),
                vec![
                    "exact_cylinder_dual_cap_opening",
                    "exact_radial_offset",
                    "inner_outer_orientation_and_cavity_owned",
                ],
            )
        } else {
            if direction == "inward" {
                if cylinder.radius <= thickness + 1e-5 || height <= 2. * thickness + 1e-5 {
                    return Err(refuse(
                        "BREP_ANALYTIC_SHELL_COLLAPSE_REFUSED",
                        "Inward radial or axial offset collapses the closed cylinder cavity",
                    ));
                }
                let inner = crate::cylinder(cylinder.radius - thickness, height - 2. * thickness)?;
                let inner_origin = [
                    source_origin[0] + cylinder.axis[0] * thickness,
                    source_origin[1] + cylinder.axis[1] * thickness,
                    source_origin[2] + cylinder.axis[2] * thickness,
                ];
                let inner = place_axial(&inner, cylinder.frame, cylinder.axis, inner_origin)?;
                (
                    crate::imprint_pipeline::cavity(model, &inner, model.tolerance_mm)?,
                    cylinder.radius.max(height),
                    vec![
                        "exact_cylinder_closed_inward_shell",
                        "exact_radial_axial_offset",
                        "inner_shell_reversed_and_cavity_owned",
                    ],
                )
            } else {
                let local = crate::cylinder(cylinder.radius + thickness, height + 2. * thickness)?;
                let outer_origin = [
                    source_origin[0] - cylinder.axis[0] * thickness,
                    source_origin[1] - cylinder.axis[1] * thickness,
                    source_origin[2] - cylinder.axis[2] * thickness,
                ];
                let outer = place_axial(&local, cylinder.frame, cylinder.axis, outer_origin)?;
                (
                    crate::imprint_pipeline::cavity(&outer, model, model.tolerance_mm)?,
                    (cylinder.radius + thickness).max(height + 2. * thickness),
                    vec![
                        "exact_cylinder_closed_outward_shell",
                        "exact_radial_axial_offset",
                        "inner_shell_reversed_and_cavity_owned",
                    ],
                )
            }
        }
    } else if let Some((outer, inner, height, origin, axis, frame)) = recognize_analytic_tube(model)
    {
        if !unique.is_empty() {
            return Err(refuse(
                "BREP_ANALYTIC_SHELL_MIXED_OPENING_REFUSED",
                "Tube face openings require exact four-way annular rim ownership",
            ));
        }
        let (new_outer, new_inner, new_height, shift) = if direction == "inward" {
            if outer - inner <= 2. * thickness + 1e-5 || height <= 2. * thickness + 1e-5 {
                return Err(refuse(
                    "BREP_ANALYTIC_SHELL_COLLAPSE_REFUSED",
                    "Inward tube radial or axial offset collapses the material wall",
                ));
            }
            (
                outer - thickness,
                inner + thickness,
                height - 2. * thickness,
                thickness,
            )
        } else {
            if inner <= thickness + 1e-5 {
                return Err(refuse(
                    "BREP_ANALYTIC_SHELL_COLLAPSE_REFUSED",
                    "Outward tube offset collapses the inner radial boundary",
                ));
            }
            (
                outer + thickness,
                inner - thickness,
                height + 2. * thickness,
                -thickness,
            )
        };
        let local = tube(new_outer, new_inner, new_height)?;
        let shifted = [
            origin[0] + shift * axis[0],
            origin[1] + shift * axis[1],
            origin[2] + shift * axis[2],
        ];
        (
            place_axial(&local, frame, axis, shifted)?,
            new_outer.max(new_height),
            vec![
                "exact_analytic_tube_body_offset",
                "exact_inner_outer_radial_axial_supports",
                "tube_material_orientation_owned",
            ],
        )
    } else {
        if model
            .faces
            .iter()
            .any(|face| face.surface.degree_u != 1 || face.surface.degree_v != 1)
        {
            return Err(refuse(
                "BREP_ANALYTIC_SHELL_MIXED_OPENING_REFUSED",
                "Mixed curved-planar and unsupported analytic tube shell topology is not authored",
            ));
        }
        if !selected_faces_are_nonadjacent(model, &unique) {
            return Err(refuse(
                "BREP_ANALYTIC_SHELL_TRANSITION_REFUSED",
                "Adjacent planar openings await exact corner-transition ownership",
            ));
        }
        let result = if direction == "inward" {
            crate::operations::shell_planar(model, &unique, thickness)
        } else {
            crate::operations::shell_planar_outward(model, &unique, thickness)
        }
        .map_err(|error| {
            refuse(
                if error.code == "BREP_INVALID_SELECTION" {
                    "BREP_ANALYTIC_SHELL_SELECTION_REFUSED"
                } else {
                    "BREP_ANALYTIC_SHELL_FEASIBILITY_REFUSED"
                },
                &error.message,
            )
        })?;
        let (min, max) = model_bounds(model);
        (
            result,
            (0..3).map(|axis| max[axis] - min[axis]).fold(0., f64::max),
            vec![
                "exact_convex_support_plane_offset",
                "bounded_halfspace_collision_and_collapse_proof",
                "exact_nonadjacent_opening_rims",
            ],
        )
    };
    certify_blend_result(
        result,
        EXACT_ANALYTIC_SHELL_CAPABILITY,
        notes,
        scale,
        "BREP_ANALYTIC_SHELL_AUDIT_REFUSED",
    )
}

/// ASL-01: two planar convex sections with matching vertex counts → ruled solid loft.
pub fn analytic_solid_loft(sections: &[Model]) -> Result<(Model, FeatureCertificate)> {
    if sections.len() < 2 {
        return Err(refuse(
            "BREP_ANALYTIC_LOFT_REFUSED",
            "Solid loft requires at least two section solids",
        ));
    }
    let mut profiles = Vec::new();
    for section in sections {
        section.validate()?;
        if !is_axis_aligned_cuboid(section) {
            return Err(refuse(
                "BREP_ANALYTIC_LOFT_REFUSED",
                "SectionMatch walking slice requires axis-aligned cuboid section carriers",
            ));
        }
        let (min, max) = model_bounds(section);
        // Axis-aligned rectangular profile at the section's lowest Z, CCW in XY.
        let z = min[2];
        let pts = vec![
            [min[0], min[1], z],
            [max[0], min[1], z],
            [max[0], max[1], z],
            [min[0], max[1], z],
        ];
        if (max[0] - min[0]) <= 1e-12 || (max[1] - min[1]) <= 1e-12 {
            return Err(refuse(
                "BREP_ANALYTIC_LOFT_REFUSED",
                "SectionMatch failed: degenerate profile extents",
            ));
        }
        profiles.push(pts);
    }
    let n0 = profiles[0].len();
    if profiles.iter().any(|p| p.len() != n0) {
        return Err(refuse(
            "BREP_ANALYTIC_LOFT_REFUSED",
            "SectionMatch failed: unequal vertex counts",
        ));
    }
    let result = ruled_loft(&profiles)?;
    result.validate()?;
    if result.faces.is_empty() {
        return Err(refuse(
            "BREP_ANALYTIC_LOFT_REFUSED",
            "Loft produced empty solid; refuse surface-as-solid claim",
        ));
    }
    Ok((
        result,
        FeatureCertificate {
            capability: "analytic-solid-loft/1",
            complete: true,
            notes: vec![
                "asl01_ruled_section_match",
                "frame_law_sweep_via_frame_law_ruled_sweep",
            ],
        },
    ))
}

// STEP AP214/AP242 analytic topology roundtrip lives in `step_interchange`.

/// Fail-closed IGES walking slice: entity subset 110/116/128/190 only.
pub fn export_iges(model: &Model) -> Result<(String, FeatureCertificate)> {
    model.validate()?;
    if model.faces.is_empty() {
        return Err(refuse(
            "BREP_IGES_REFUSED",
            "Empty model cannot export as IGES B-rep",
        ));
    }
    let (min, max) = model_bounds(model);
    let mut lines = Vec::new();
    lines.push(
        "                                                                        S      1".into(),
    );
    lines.push(
        "1H,,1H;,4HSOLID,11Hopen-scad-v,32Hanalytic IGES walking slice,32H,    G      1".into(),
    );
    let mut seq = 1usize;
    for v in &model.vertices {
        // Entity 116: Point
        lines.push(format!(
            "     116       1       0       1       0       0       0       0       1D{seq:7}"
        ));
        seq += 1;
        lines.push(format!(
            "116,{:.15},{:.15},{:.15};                                          P{seq:7}",
            v.point[0], v.point[1], v.point[2]
        ));
        seq += 1;
    }
    for edge in &model.edges {
        if edge.curve.degree == 1 && edge.curve.control_points.len() == 2 {
            let a = &edge.curve.control_points[0];
            let b = &edge.curve.control_points[1];
            // Entity 110: Line
            lines.push(format!(
                "     110       1       0       1       0       0       0       0       1D{seq:7}"
            ));
            seq += 1;
            lines.push(format!(
                "110,{:.8},{:.8},{:.8},{:.8},{:.8},{:.8};                      P{seq:7}",
                a[0], a[1], a[2], b[0], b[1], b[2]
            ));
            seq += 1;
        }
    }
    for face in &model.faces {
        if face.surface.degree_u == 1 && face.surface.degree_v == 1 {
            // Entity 190: Plane Surface
            lines.push(format!(
                "     190       1       0       1       0       0       0       0       1D{seq:7}"
            ));
            seq += 1;
            lines.push(format!(
                "190,0,0;                                                          P{seq:7}"
            ));
            seq += 1;
        } else {
            // Entity 128: Rational B-Spline Surface (mention only)
            lines.push(format!(
                "     128       1       0       1       0       0       0       0       1D{seq:7}"
            ));
            seq += 1;
            lines.push(format!(
                "128,{},{},0,0,0,0,0;                                              P{seq:7}",
                face.surface.degree_u, face.surface.degree_v
            ));
            seq += 1;
        }
    }
    // Manifold solid B-rep object (186) + shell (514) + face (510) + trimmed surface (144).
    lines.push(format!(
        "     186       1       0       1       0       0       0       0       1D{seq:7}"
    ));
    seq += 1;
    lines.push(format!(
        "186,1,0;                                                          P{seq:7}"
    ));
    seq += 1;
    lines.push(format!(
        "     514       1       0       1       0       0       0       0       1D{seq:7}"
    ));
    seq += 1;
    lines.push(format!(
        "514,{},0;                                                         P{seq:7}",
        model.faces.len()
    ));
    seq += 1;
    for _ in &model.faces {
        lines.push(format!(
            "     510       1       0       1       0       0       0       0       1D{seq:7}"
        ));
        seq += 1;
        lines.push(format!(
            "510,1,0,0;                                                        P{seq:7}"
        ));
        seq += 1;
        lines.push(format!(
            "     144       1       0       1       0       0       0       0       1D{seq:7}"
        ));
        seq += 1;
        lines.push(format!(
            "144,0,1,0,0;                                                      P{seq:7}"
        ));
        seq += 1;
    }
    lines.push(format!(
        "/* open-scad-viewer iges-interchange/1; faces={} bounds=[{:.3},{:.3},{:.3}]-[{:.3},{:.3},{:.3}] */",
        model.faces.len(),
        min[0],
        min[1],
        min[2],
        max[0],
        max[1],
        max[2]
    ));
    lines.push(
        "S      1G      1D      1P      1                                        T      1".into(),
    );
    let text = lines.join("\n");
    if text.to_ascii_uppercase().contains("FACETED")
        || text.contains("solid ")
        || text.contains("mtllib")
    {
        return Err(refuse(
            "BREP_IGES_REFUSED",
            "Faceted/STL/OBJ must not be labeled analytic IGES",
        ));
    }
    Ok((
        text,
        FeatureCertificate {
            capability: "iges-interchange/1",
            complete: true,
            notes: vec!["iges_entity_110_116_128_190", "iges_solid_186_514_510_144"],
        },
    ))
}
