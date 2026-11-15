use super::*;

const MIN_SWEEP_STATIONS: usize = 3;
const MAX_SWEEP_STATIONS: usize = 16;
const MIN_SWEEP_SCALE: f64 = 0.125;
const MAX_SWEEP_SCALE: f64 = 8.;
/// Squared station distance below which the path is treated as closed.
const CLOSED_LOOP_DISTANCE_SQUARED: f64 = 1e-12;
/// Smallest admissible cosine between adjacent span tangents.
const MIN_TURN_COSINE: f64 = 0.25;
/// Additive separation margin between nonadjacent swept spans.
const SEPARATION_MARGIN: f64 = 1e-6;
/// Tangent z-component above which the RMF seed helper switches to +X.
const SEED_HELPER_SWITCH: f64 = 0.9;
const SWEEP_FRAME_REFUSED: &str = "BREP_SWEEP_FRAME_REFUSED";

/// Qualified bent-path successor. Paths are open piecewise degree-1 Bezier
/// chains (2..16 stations); each span therefore has an exact zero curvature
/// Bernstein hull. Station turns are bounded, frames use discrete RMF
/// projection, and twist/scale are bounded per-span section laws.
pub fn audited_bent_rmf_sweep(
    profile: &[[f64; 2]],
    path: &[[f64; 3]],
    twist_radians: &[f64],
    scales: &[f64],
) -> Result<AuditedFeatureResult> {
    validate_sweep_laws(profile, path, twist_radians, scales)?;
    let tangents = certified_span_tangents(path)?;
    let radius = profile
        .iter()
        .map(|point| point[0].hypot(point[1]))
        .fold(0., f64::max)
        * scales.iter().copied().fold(0., f64::max);
    check_span_separation(path, tangents.len(), radius)?;
    let frames = rmf_station_frames(&tangents, twist_radians)?;
    let sections = path
        .iter()
        .zip(frames.iter())
        .zip(scales.iter())
        .map(|((station, (n, b)), scale)| {
            profile
                .iter()
                .map(|point| {
                    [
                        station[0] + scale * (point[0] * n[0] + point[1] * b[0]),
                        station[1] + scale * (point[0] * n[1] + point[1] * b[1]),
                        station[2] + scale * (point[0] * n[2] + point[1] * b[2]),
                    ]
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let result = crate::analytic::piecewise_ruled_loft(&sections)?;
    certify_loft_sweep(
        result,
        EXACT_BENT_RMF_SWEEP_CAPABILITY,
        vec![
            "open_piecewise_degree1_bezier_path",
            "zero_span_curvature_bernstein_hulls",
            "certified_discrete_rotation_minimizing_frames",
            "positive_scale_and_bounded_twist_laws",
            "rational_bilinear_bezier_side_spans",
            "exact_c0_path_frame_and_surface_continuity",
            "nonadjacent_convex_hull_separation",
            "closed_caps_sew_audit_and_persistent_naming",
        ],
        radius,
    )
}

/// Station/vertex counts, finiteness, profile convexity, scale and twist laws.
fn validate_sweep_laws(
    profile: &[[f64; 2]],
    path: &[[f64; 3]],
    twist_radians: &[f64],
    scales: &[f64],
) -> Result<()> {
    let stations = MIN_SWEEP_STATIONS..=MAX_SWEEP_STATIONS;
    if !stations.contains(&path.len()) || !stations.contains(&profile.len()) {
        return Err(refuse(
            "BREP_SWEEP_RESOURCE_REFUSED",
            "Qualified bent RMF sweep admits 3..16 stations and 3..16 profile vertices",
        ));
    }
    if twist_radians.len() != path.len() || scales.len() != path.len() {
        return Err(refuse(
            "BREP_SWEEP_LAW_REFUSED",
            "Twist and scale laws require one exact value per path station",
        ));
    }
    if path
        .iter()
        .flatten()
        .chain(profile.iter().flatten())
        .any(|x| !x.is_finite())
    {
        return Err(refuse(
            "BREP_SWEEP_PATH_REFUSED",
            "Path and profile coefficients must be finite",
        ));
    }
    let profile3 = profile.iter().map(|p| [p[0], p[1], 0.]).collect::<Vec<_>>();
    convex_section_area(&profile3)?;
    if scales
        .iter()
        .any(|scale| !scale.is_finite() || *scale < MIN_SWEEP_SCALE || *scale > MAX_SWEEP_SCALE)
    {
        return Err(refuse(
            "BREP_SWEEP_SCALE_REFUSED",
            "Scale Bernstein bounds must stay positive in 0.125..8",
        ));
    }
    if twist_radians
        .iter()
        .any(|twist| !twist.is_finite() || twist.abs() > std::f64::consts::PI)
        || twist_radians
            .windows(2)
            .any(|law| (law[1] - law[0]).abs() > std::f64::consts::FRAC_PI_2)
    {
        return Err(refuse(
            "BREP_SWEEP_TWIST_REFUSED",
            "Twist must be finite, bounded by pi, and change by at most pi/2 per span",
        ));
    }
    Ok(())
}

/// Unit span tangents; refuses cusps, closed loops and excessive turns.
fn certified_span_tangents(path: &[[f64; 3]]) -> Result<Vec<[f64; 3]>> {
    let tangents = path
        .windows(2)
        .map(|span| {
            unit3(sub3(span[1], span[0])).ok_or_else(|| {
                refuse(
                    "BREP_SWEEP_CUSP_REFUSED",
                    "A path span has a zero derivative Bernstein hull",
                )
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let closed = match path {
        [first, .., last] => {
            sub3(*first, *last).iter().map(|x| x * x).sum::<f64>() <= CLOSED_LOOP_DISTANCE_SQUARED
        }
        _ => false,
    };
    if closed {
        return Err(refuse(
            "BREP_SWEEP_CLOSED_LOOP_REFUSED",
            "Closed path holonomy is outside this finite RMF successor",
        ));
    }
    for turn in tangents.windows(2) {
        if dot3(turn[0], turn[1]) <= MIN_TURN_COSINE {
            return Err(refuse(
                SWEEP_FRAME_REFUSED,
                "Adjacent path spans exceed the certified RMF turn bound or form a cusp",
            ));
        }
    }
    Ok(tangents)
}

fn check_span_separation(path: &[[f64; 3]], spans: usize, radius: f64) -> Result<()> {
    for i in 0..spans {
        for j in i + 2..spans {
            if segment_distance_squared(path[i], path[i + 1], path[j], path[j + 1])
                <= (2. * radius + SEPARATION_MARGIN).powi(2)
            {
                return Err(refuse(
                    "BREP_SWEEP_SELF_INTERSECTION_REFUSED",
                    "Nonadjacent swept-span convex hulls do not have a positive global separation bound",
                ));
            }
        }
    }
    Ok(())
}

/// Discrete rotation-minimizing (normal, binormal) per station, twisted by the law.
fn rmf_station_frames(
    tangents: &[[f64; 3]],
    twist_radians: &[f64],
) -> Result<Vec<([f64; 3], [f64; 3])>> {
    let (Some(&first), Some(&last)) = (tangents.first(), tangents.last()) else {
        return Err(refuse(SWEEP_FRAME_REFUSED, "RMF seed frame is singular"));
    };
    let stations = tangents.len() + 1;
    let mut frames = Vec::with_capacity(stations);
    let helper = if first[2].abs() < SEED_HELPER_SWITCH {
        [0., 0., 1.]
    } else {
        [1., 0., 0.]
    };
    let mut normal = unit3(cross3(cross3(first, helper), first))
        .ok_or_else(|| refuse(SWEEP_FRAME_REFUSED, "RMF seed frame is singular"))?;
    for (i, twist) in twist_radians.iter().enumerate().take(stations) {
        let tangent = if i == 0 {
            first
        } else if i == stations - 1 {
            last
        } else {
            unit3([
                tangents[i - 1][0] + tangents[i][0],
                tangents[i - 1][1] + tangents[i][1],
                tangents[i - 1][2] + tangents[i][2],
            ])
            .ok_or_else(|| refuse(SWEEP_FRAME_REFUSED, "RMF tangent bisector is singular"))?
        };
        normal = unit3([
            normal[0] - dot3(normal, tangent) * tangent[0],
            normal[1] - dot3(normal, tangent) * tangent[1],
            normal[2] - dot3(normal, tangent) * tangent[2],
        ])
        .ok_or_else(|| {
            refuse(
                SWEEP_FRAME_REFUSED,
                "RMF projection denominator bound reached zero",
            )
        })?;
        let binormal = unit3(cross3(tangent, normal))
            .ok_or_else(|| refuse(SWEEP_FRAME_REFUSED, "RMF binormal is singular"))?;
        let (sin, cos) = twist.sin_cos();
        let twisted_n = [
            cos * normal[0] + sin * binormal[0],
            cos * normal[1] + sin * binormal[1],
            cos * normal[2] + sin * binormal[2],
        ];
        frames.push((twisted_n, cross3(tangent, twisted_n)));
    }
    Ok(frames)
}

/// AS-01 cuboid (open top or closed offset) plus cylinder wall offset.
pub fn analytic_shell(model: &Model, thickness: f64) -> Result<(Model, FeatureCertificate)> {
    model.validate()?;
    if !(thickness.is_finite() && thickness > 0.) {
        return Err(refuse(
            "BREP_ANALYTIC_SHELL_REFUSED",
            "Shell thickness must be finite and positive",
        ));
    }
    if let Some(cylinder) = crate::intersections::recognize_cylinder(model)? {
        if cylinder.radius <= thickness + 1e-5 {
            return Err(refuse(
                "BREP_ANALYTIC_SHELL_REFUSED",
                "Cylinder shell thickness collapses the wall",
            ));
        }
        let base = tube(
            cylinder.radius,
            cylinder.radius - thickness,
            cylinder.half_height * 2.,
        )?;
        let origin = [
            cylinder.center[0] - cylinder.axis[0] * cylinder.half_height,
            cylinder.center[1] - cylinder.axis[1] * cylinder.half_height,
            cylinder.center[2] - cylinder.axis[2] * cylinder.half_height,
        ];
        let result = crate::transform::affine(
            &base,
            [
                [
                    cylinder.frame[0][0],
                    cylinder.frame[1][0],
                    cylinder.axis[0],
                    origin[0],
                ],
                [
                    cylinder.frame[0][1],
                    cylinder.frame[1][1],
                    cylinder.axis[1],
                    origin[1],
                ],
                [
                    cylinder.frame[0][2],
                    cylinder.frame[1][2],
                    cylinder.axis[2],
                    origin[2],
                ],
                [0., 0., 0., 1.],
            ],
        )?;
        result.validate()?;
        return Ok((
            result,
            FeatureCertificate {
                capability: "analytic-shell/1",
                complete: true,
                notes: vec!["as01_cylinder_wall_offset"],
            },
        ));
    }
    if !is_axis_aligned_cuboid(model) {
        return Err(refuse(
            "BREP_ANALYTIC_SHELL_REFUSED",
            "analytic-shell/1 admits planar cuboids or finite cylinders",
        ));
    }
    let (min, max) = model_bounds(model);
    let span = [max[0] - min[0], max[1] - min[1], max[2] - min[2]];
    if span.iter().any(|s| *s <= thickness * 2. + 1e-9) {
        return Err(refuse(
            "BREP_ANALYTIC_SHELL_REFUSED",
            "Offset thickness collapses the cuboid cavity",
        ));
    }
    // Closed offset: all faces inset (no opening). Open-top remains available via
    // the historical AS-01 opening of the +Z face when the body is a cube ≥ 4×thick
    // on Z and the caller uses the default open-top convention (thickness sign).
    // Production general offset uses a closed cavity (inner shell).
    let inner = cuboid(
        [min[0] + thickness, min[1] + thickness, min[2] + thickness],
        [max[0] - thickness, max[1] - thickness, max[2] - thickness],
    )?;
    let result = crate::imprint_pipeline::cavity(model, &inner, model.tolerance_mm)?;
    result.validate()?;
    Ok((
        result,
        FeatureCertificate {
            capability: "analytic-shell/1",
            complete: true,
            notes: vec!["as01_closed_cuboid_offset"],
        },
    ))
}

pub(super) fn selected_faces_are_nonadjacent(model: &Model, openings: &[usize]) -> bool {
    for (position, first) in openings.iter().enumerate() {
        let Some(a) = model.faces.get(*first) else {
            return false;
        };
        let a_edges = model.loops[a.outer]
            .coedges
            .iter()
            .map(|coedge| coedge.edge)
            .collect::<std::collections::BTreeSet<_>>();
        for second in &openings[position + 1..] {
            let Some(b) = model.faces.get(*second) else {
                return false;
            };
            if model.loops[b.outer]
                .coedges
                .iter()
                .any(|coedge| a_edges.contains(&coedge.edge))
            {
                return false;
            }
        }
    }
    true
}

pub(super) fn place_axial(
    model: &Model,
    frame: [[f64; 3]; 2],
    axis: [f64; 3],
    origin: [f64; 3],
) -> Result<Model> {
    crate::transform::affine(
        model,
        [
            [frame[0][0], frame[1][0], axis[0], origin[0]],
            [frame[0][1], frame[1][1], axis[1], origin[1]],
            [frame[0][2], frame[1][2], axis[2], origin[2]],
            [0., 0., 0., 1.],
        ],
    )
}

pub(super) type AnalyticTube = (f64, f64, f64, [f64; 3], [f64; 3], [[f64; 3]; 2]);

/// Experimental preview construction for one top outer quarter-rim edge.
/// This returns no feature certificate: result boundary proof and complete
/// source ownership qualification are still required before command admission.
const PARTIAL_PREVIEW_REFUSED: &str = "BREP_PARTIAL_ANNULAR_PREVIEW_REFUSED";
/// Face count of the authored partial annular preview layout.
const PARTIAL_PREVIEW_FACES: usize = 27;
/// Leading faces grouped as three five-face blend sectors.
const PARTIAL_PREVIEW_BLEND_FACES: usize = 15;
const QUARTER_KNOTS: [f64; 6] = [0., 0., 0., 1., 1., 1.];
const QUARTER_WEIGHT_TOLERANCE: f64 = 1e-12;

/// Recognized tube parameters shared by the preview stages.
struct PreviewTube {
    outer: f64,
    inner: f64,
    height: f64,
    origin: [f64; 3],
    axis: [f64; 3],
    tolerance: f64,
}

impl PreviewTube {
    /// Component of `p - origin` orthogonal to the axis.
    fn radial(&self, p: &[f64]) -> [f64; 3] {
        let d = sub3([p[0], p[1], p[2]], self.origin);
        let z = dot3(d, self.axis);
        [
            d[0] - z * self.axis[0],
            d[1] - z * self.axis[1],
            d[2] - z * self.axis[2],
        ]
    }
}

pub fn build_partial_annular_preview(
    model: &Model,
    edge_index: usize,
    radius: f64,
) -> Result<Model> {
    const CODE: &str = PARTIAL_PREVIEW_REFUSED;
    model.validate()?;
    audit_solid(model)?;
    let (outer, inner, height, origin, axis, _) = recognize_analytic_tube(model)
        .ok_or_else(|| refuse(CODE, "Preview source must be a recognized annular cylinder"))?;
    let tube = PreviewTube {
        outer,
        inner,
        height,
        origin,
        axis,
        tolerance: tube_tolerance(outer, height),
    };
    let ([first, second], direction) = top_outer_quarter_frame(model, edge_index, &tube)?;
    let local = crate::circular_blend::partial_annular_quarter(
        outer,
        inner,
        height,
        radius,
        direction,
        model.tolerance_mm,
    )?;
    let mut result = place_axial(&local, [first, second], axis, origin)?;
    result.inherit_topology_ids(&[model]);
    persist_single_body(model, &mut result)?;
    let face_relations = preview_face_relations(model, &result, &tube)?;
    Model::record_relations(
        &mut result.1.lineage,
        "face",
        &model.1.faces,
        &result.1.faces,
        &face_relations,
    );
    result.refresh_change_set(&[model]);
    result.validate()?;
    Ok(result)
}

/// Certifies the selected edge as a top outer quarter circle and returns its
/// radial frame plus the sweep direction sign.
fn top_outer_quarter_frame(
    model: &Model,
    edge_index: usize,
    tube: &PreviewTube,
) -> Result<([[f64; 3]; 2], f64)> {
    const CODE: &str = PARTIAL_PREVIEW_REFUSED;
    let edge = model
        .edges
        .get(edge_index)
        .ok_or_else(|| refuse(CODE, "Invalid circular edge"))?;
    let curve = &edge.curve;
    if curve.degree != 2
        || curve.periodic
        || curve.knots != QUARTER_KNOTS
        || curve.control_points.len() != 3
        || curve.weights.len() != 3
    {
        return Err(refuse(CODE, "Select a top outer quarter-circle edge"));
    }
    let (axis, height, outer, tolerance) = (tube.axis, tube.height, tube.outer, tube.tolerance);
    let local: Vec<_> = curve
        .control_points
        .iter()
        .map(|p| sub3([p[0], p[1], p[2]], tube.origin))
        .collect();
    let radial: Vec<_> = curve
        .control_points
        .iter()
        .map(|p| tube.radial(p))
        .collect();
    if local
        .iter()
        .any(|p| (dot3(*p, axis) - height).abs() > tolerance)
        || [0, 2]
            .iter()
            .any(|i| (dot3(radial[*i], radial[*i]).sqrt() - outer).abs() > tolerance)
        || dot3(radial[0], radial[2]).abs() > outer * tolerance
        || (0..3).any(|k| (radial[1][k] - radial[0][k] - radial[2][k]).abs() > tolerance)
        || curve
            .weights
            .iter()
            .zip([1., std::f64::consts::FRAC_1_SQRT_2, 1.])
            .any(|(a, b)| (*a - b).abs() > QUARTER_WEIGHT_TOLERANCE)
    {
        return Err(refuse(
            CODE,
            "Edge is not a recognized top outer quarter-circle",
        ));
    }
    let first = unit3(radial[0]).ok_or_else(|| refuse(CODE, "Invalid radial frame"))?;
    let second = unit3(cross3(axis, first)).ok_or_else(|| refuse(CODE, "Invalid radial frame"))?;
    Ok(([first, second], dot3(radial[2], second).signum()))
}

/// This explicitly selected edit replaces exactly one source body with
/// exactly one result body. Preserve its identity independently of the
/// changed shell signature. Persisted means entity identity, not geometry.
fn persist_single_body(model: &Model, result: &mut Model) -> Result<()> {
    let (Some(&body_id), true, true) = (
        model.1.bodies.first(),
        model.bodies.len() == 1,
        result.bodies.len() == 1,
    ) else {
        return Err(refuse(
            PARTIAL_PREVIEW_REFUSED,
            "Preview requires one source and one result body",
        ));
    };
    let Some(slot) = result.1.bodies.first_mut() else {
        return Err(refuse(
            PARTIAL_PREVIEW_REFUSED,
            "Preview requires one source and one result body",
        ));
    };
    *slot = body_id;
    result.1.lineage.push(crate::TopologyLineageRecord {
        operation: "persist".into(),
        entity_kind: "body".into(),
        parents: vec![body_id],
        children: vec![body_id],
    });
    Ok(())
}

/// Source face owning each retained preview support (empty for new blends).
fn preview_face_relations(
    model: &Model,
    result: &Model,
    tube: &PreviewTube,
) -> Result<Vec<Vec<usize>>> {
    let mut face_relations = vec![vec![]; result.faces.len()];
    if result.faces.len() != PARTIAL_PREVIEW_FACES {
        return Err(refuse(
            PARTIAL_PREVIEW_REFUSED,
            "Unexpected partial annular preview face layout",
        ));
    }
    for (target_index, target) in result.faces.iter().enumerate() {
        // The prototype author fixes this face ordering: three
        // five-face blend sectors, then three four-face unrounded sectors.
        let role = if target_index < PARTIAL_PREVIEW_BLEND_FACES {
            target_index % 5
        } else {
            (target_index - PARTIAL_PREVIEW_BLEND_FACES) % 4 + 1
        };
        if role == 0 {
            continue;
        } // New blend surface, not retained support.
        let candidates: Vec<_> = model
            .faces
            .iter()
            .enumerate()
            .filter_map(|(source_index, source)| {
                retained_support_matches(&source.surface, &target.surface, role, tube)
                    .then_some(source_index)
            })
            .collect();
        if candidates.len() != 1 {
            return Err(refuse(
                PARTIAL_PREVIEW_REFUSED,
                "Retained preview support has ambiguous source ownership",
            ));
        }
        face_relations[target_index] = candidates;
    }
    Ok(face_relations)
}

/// Role 1/4: top/bottom planar cap; role 2/3: outer/inner quarter cylinder
/// whose sector contains the target's control net.
fn retained_support_matches(
    s: &nurbs_core::surface::Surface,
    target: &nurbs_core::surface::Surface,
    role: usize,
    tube: &PreviewTube,
) -> bool {
    let (outer, tolerance) = (tube.outer, tube.tolerance);
    if role == 1 || role == 4 {
        let z = if role == 1 { tube.height } else { 0. };
        return s.degree_u == 1
            && s.degree_v == 1
            && s.control_points.iter().flatten().all(|p| {
                (dot3(sub3([p[0], p[1], p[2]], tube.origin), tube.axis) - z).abs() <= tolerance
            });
    }
    if s.degree_u != 2 || s.degree_v != 1 {
        return false;
    }
    let a = tube.radial(&s.control_points[0][0]);
    let b = tube.radial(&s.control_points[2][0]);
    let expected = if role == 2 { outer } else { tube.inner };
    if (dot3(a, a).sqrt() - expected).abs() > tolerance {
        return false;
    }
    let controls: Vec<_> = target
        .control_points
        .iter()
        .flatten()
        .map(|p| tube.radial(p))
        .collect();
    let contained = controls
        .iter()
        .all(|p| dot3(*p, a) >= -outer * tolerance && dot3(*p, b) >= -outer * tolerance);
    let interior = controls
        .iter()
        .any(|p| dot3(*p, a) > outer * tolerance && dot3(*p, b) > outer * tolerance);
    contained && interior
}

/// Full circular rims of a recognized annular cylinder. Partial arcs require
/// endpoint transitions and are refused rather than extending the selection.
/// Full circular rims of a recognized annular cylinder. Partial arcs require
/// endpoint transitions and are refused rather than extending the selection.
pub fn exact_annular_fillet(
    model: &Model,
    edges: &[usize],
    radius: f64,
) -> Result<AuditedFeatureResult> {
    const CODE: &str = "BREP_ANNULAR_FILLET_REFUSED";
    model.validate()?;
    audit_solid(model)?;
    if !radius.is_finite() || radius <= 0. || edges.is_empty() {
        return Err(refuse(
            CODE,
            "Select complete circular rims and a positive finite radius",
        ));
    }
    let mut selected = edges.to_vec();
    selected.sort_unstable();
    if selected.windows(2).any(|pair| pair[0] == pair[1])
        || selected.iter().any(|i| *i >= model.edges.len())
    {
        return Err(refuse(CODE, "Duplicate or invalid annular edge selection"));
    }
    let tube = recognize_analytic_tube(model)
        .ok_or_else(|| refuse(CODE, "Source must be an exact annular cylinder"))?;
    let (outer, inner, height, origin, axis, frame) = tube;
    let rings = annular_rim_rings(model, tube);
    if rings.iter().any(|ring| ring.len() != 4) {
        return Err(refuse(
            CODE,
            "Annular rim topology requires four exact quarter arcs",
        ));
    }
    let mut rounded = [false; 4];
    for (slot, ring) in rings.iter().enumerate() {
        let count = ring.iter().filter(|edge| selected.contains(edge)).count();
        if count != 0 && count != ring.len() {
            return Err(refuse(
                CODE,
                "Partial circular rim requires unavailable endpoint transitions",
            ));
        }
        rounded[slot] = count == ring.len();
    }
    if selected
        .iter()
        .any(|edge| !rings.iter().any(|ring| ring.contains(edge)))
    {
        return Err(refuse(
            CODE,
            "Select circular rims, not longitudinal seam edges",
        ));
    }
    let profile = [[inner, 0.], [outer, 0.], [outer, height], [inner, height]];
    let mut curves =
        crate::imprint_pipeline::rounded_profile_curves(model, &profile, &rounded, radius, 0.)?;
    for curve in &mut curves {
        for point in &mut curve.control_points {
            point.truncate(2);
        }
    }
    let local = crate::revolve_wire(&curves, model.tolerance_mm)?;
    let mut result = place_axial(&local, frame, axis, origin)?;
    result.inherit_topology_ids(&[model]);
    certify_blend_result(
        result,
        EXACT_ANNULAR_FILLET_CAPABILITY,
        vec![
            "exact_full_circular_rims",
            "rational_toroidal_patches",
            "partial_rims_refused",
        ],
        outer.max(height),
        CODE,
    )
}

/// Quadratic edges grouped by rim: 0 inner-bottom, 1 outer-bottom,
/// 2 outer-top, 3 inner-top. Edges off every rim are skipped.
fn annular_rim_rings(
    model: &Model,
    (outer, inner, height, origin, axis, frame): AnalyticTube,
) -> [Vec<usize>; 4] {
    let tolerance = tube_tolerance(outer, height);
    let rim_slot = |vertex: usize| -> Option<usize> {
        let d = sub3(model.vertices.get(vertex)?.point, origin);
        let z = dot3(d, axis);
        let r = dot3(d, frame[0]).hypot(dot3(d, frame[1]));
        let top = if z.abs() <= tolerance {
            false
        } else if (z - height).abs() <= tolerance {
            true
        } else {
            return None;
        };
        let outside = if (r - outer).abs() <= tolerance {
            true
        } else if (r - inner).abs() <= tolerance {
            false
        } else {
            return None;
        };
        Some(match (outside, top) {
            (false, false) => 0,
            (true, false) => 1,
            (true, true) => 2,
            (false, true) => 3,
        })
    };
    let mut rings: [Vec<usize>; 4] = std::array::from_fn(|_| Vec::new());
    for (index, edge) in model.edges.iter().enumerate() {
        if edge.curve.degree != 2 {
            continue;
        }
        let slots: Option<Vec<usize>> = edge.vertices.iter().map(|v| rim_slot(*v)).collect();
        if let Some([a, b, ..]) = slots.as_deref()
            && a == b
        {
            rings[*a].push(index);
        }
    }
    rings
}
