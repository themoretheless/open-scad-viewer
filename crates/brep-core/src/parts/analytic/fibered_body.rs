//! Original open affine fibers -> immutable bound BRep, including filled caps.
//! Every source/cap/wall is owned here. No caller success flag or seam exclusion.
use super::*;
use crate::exact_affine_source::{exact_mul, image};
use nurbs_core::{affine_volume::RationalPiece, fibered_volume};
#[derive(Clone, Copy)]
pub struct FiberedBodyLimits {
    pub volume: fibered_volume::Limits,
    pub material_pairs: usize,
    pub material_cells: usize,
    pub material_domain_cells: usize,
    pub profile_cells: usize,
    pub source_trace_work: u64,
    pub binding_rows: usize,
    pub agreement_work: u64,
    pub regularity_cells: usize,
    pub normal_cells: usize,
    pub smoothness_work: u64,
    pub order: usize,
}
pub struct FiberedBodyReport {
    pub certified: bool,
    pub source_boundary_exact: bool,
    pub boundary_error_upper: Option<f64>,
    pub topology_preserved: bool,
    pub absence_of_self_intersections: bool,
    pub source_g1: bool,
    pub source_g2: bool,
    pub seam_continuity: &'static str,
    pub volume: Option<fibered_volume::Report>,
    pub material: Option<nurbs_core::trim_region_audit::Report>,
    pub regularity: Option<crate::boundary_embedding::regularity::Report>,
    pub agreement: Option<crate::boundary_agreement::ExactReport>,
    pub profile_cells: usize,
    pub source_trace_work: u64,
    pub binding_rows: usize,
    pub smoothness_work: u64,
    pub normal_cells: usize,
    pub inspected_profile_joins: usize,
    pub inspected_motion_joins: usize,
    pub vertex_links_complete: bool,
    pub reason: Option<&'static str>,
}
pub struct FiberedBody {
    model: Model,
    report: FiberedBodyReport,
}
impl FiberedBody {
    pub fn model(&self) -> &Model {
        &self.model
    }
    pub fn report(&self) -> &FiberedBodyReport {
        &self.report
    }
    pub fn into_model(self) -> Model {
        self.model
    }
}
pub enum FiberedBodyAttempt {
    Certified(Box<FiberedBody>),
    Refused(Box<FiberedBodyReport>),
}
fn refused(mut r: FiberedBodyReport, reason: &'static str) -> FiberedBodyAttempt {
    r.reason = Some(reason);
    FiberedBodyAttempt::Refused(Box::new(r))
}
fn empty() -> FiberedBodyReport {
    FiberedBodyReport {
        certified: false,
        source_boundary_exact: false,
        boundary_error_upper: None,
        topology_preserved: false,
        absence_of_self_intersections: false,
        source_g1: false,
        source_g2: false,
        seam_continuity: "C0",
        volume: None,
        material: None,
        regularity: None,
        agreement: None,
        profile_cells: 0,
        source_trace_work: 0,
        binding_rows: 0,
        smoothness_work: 0,
        normal_cells: 0,
        inspected_profile_joins: 0,
        inspected_motion_joins: 0,
        vertex_links_complete: false,
        reason: None,
    }
}
fn bezier(c: &Curve) -> bool {
    let n = c.control_points.len();
    n == c.degree + 1
        && !c.periodic
        && c.knots[..n].iter().all(|x| *x == c.knots[c.degree])
        && c.knots[n..].iter().all(|x| *x == c.knots[n])
}
fn unit_clock(c: &Curve) -> Curve {
    let mut c = c.clone();
    c.knots = [vec![0.; c.degree + 1], vec![1.; c.degree + 1]].concat();
    c
}
fn transformed(
    c: &Curve,
    pose: nurbs_core::affine_volume::Pose,
    origin: [f64; 3],
) -> Option<Curve> {
    let mut c = unit_clock(c);
    for p in &mut c.control_points {
        *p = image(pose, p, origin)?.to_vec();
    }
    Some(c)
}
fn domains(s: &Surface) -> [[f64; 2]; 2] {
    [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ]
}
fn normal_sign(
    s: &Surface,
    domain: [[f64; 2]; 2],
    boundary: Option<usize>,
    direction: [f64; 3],
    r: &mut FiberedBodyReport,
    max: usize,
) -> Result<Option<i8>> {
    let full = domains(s);
    let mut pending = vec![domain];
    let mut sign = None;
    while let Some(b) = pending.pop() {
        if r.normal_cells == max {
            return Ok(None);
        }
        let bound = if let Some(end) = boundary {
            r.normal_cells += 1;
            nurbs_core::surface_injectivity::boundary_normal_direction_bounds(
                s, 1, end, b[0], direction,
            )?
        } else {
            let q = nurbs_core::surface_injectivity::normal_direction_bounds_detailed(
                s,
                b,
                direction,
                (max - r.normal_cells).min(100000),
            )?;
            r.normal_cells += q.spans;
            q.bounds
        };
        let Some(v) = bound else { return Ok(None) };
        let current = if v[0] > 0. {
            Some(1)
        } else if v[1] < 0. {
            Some(-1)
        } else {
            None
        };
        if let Some(c) = current {
            if sign.is_some_and(|old| old != c) {
                return Ok(None);
            }
            sign = Some(c);
            continue;
        }
        let widths =
            std::array::from_fn::<_, 2, _>(|k| (b[k][1] - b[k][0]) / (full[k][1] - full[k][0]));
        let axis = if boundary.is_some() {
            0
        } else {
            usize::from(widths[1] > widths[0])
        };
        let mid = b[axis][0] * 0.5 + b[axis][1] * 0.5;
        if mid <= b[axis][0] || mid >= b[axis][1] {
            return Ok(None);
        }
        let (mut a, mut c) = (b, b);
        a[axis][1] = mid;
        c[axis][0] = mid;
        pending.extend([c, a]);
    }
    Ok(sign)
}
fn expected(
    profiles: &[Vec<Curve>],
    material: &LoftCap,
    origin: [f64; 3],
    motion: &[RationalPiece],
) -> Result<Option<Model>> {
    let poses = motion
        .iter()
        .map(|p| p.controls[0])
        .chain(std::iter::once(
            *motion.last().unwrap().controls.last().unwrap(),
        ))
        .collect::<Vec<_>>();
    let mut build = Builder::new();
    let mut sections = Vec::new();
    let mut ids = Vec::new();
    for pose in poses {
        let mut section = Vec::new();
        let mut layer = Vec::new();
        for ring in profiles {
            let mut curves = Vec::new();
            let mut vertices = Vec::new();
            for c in ring {
                let Some(c) = transformed(c, pose, origin) else {
                    return Ok(None);
                };
                vertices.push(build.model.vertices.len());
                build.model.vertices.push(Vertex {
                    point: c.control_points[0].clone().try_into().unwrap(),
                });
                curves.push(c);
            }
            section.push(curves);
            layer.push(vertices);
        }
        sections.push(section);
        ids.push(layer);
    }
    for (layer, piece) in motion.iter().enumerate() {
        for (ring, wire) in profiles.iter().enumerate() {
            for (index, source) in wire.iter().enumerate() {
                let mut cp = Vec::new();
                let mut weights = Vec::new();
                for (p, w) in source.control_points.iter().zip(&source.weights) {
                    let mut row = Vec::new();
                    let mut wr = Vec::new();
                    for (pose, v) in piece.controls.iter().zip(&piece.weights) {
                        let (Some(q), Some(w)) = (image(*pose, p, origin), exact_mul(*w, *v))
                        else {
                            return Ok(None);
                        };
                        row.push(q.to_vec());
                        wr.push(w);
                    }
                    cp.push(row);
                    weights.push(wr);
                }
                let surface = Surface {
                    degree_u: source.degree,
                    degree_v: piece.controls.len() - 1,
                    knots_u: [vec![0.; source.degree + 1], vec![1.; source.degree + 1]].concat(),
                    knots_v: [
                        vec![0.; piece.controls.len()],
                        vec![1.; piece.controls.len()],
                    ]
                    .concat(),
                    control_points: cp,
                    weights,
                    periodic_u: false,
                    periodic_v: false,
                };
                let vertical = |p: &[f64]| -> Option<Curve> {
                    Some(Curve {
                        degree: piece.controls.len() - 1,
                        knots: [
                            vec![0.; piece.controls.len()],
                            vec![1.; piece.controls.len()],
                        ]
                        .concat(),
                        control_points: piece
                            .controls
                            .iter()
                            .map(|pose| image(*pose, p, origin).map(|p| p.to_vec()))
                            .collect::<Option<Vec<_>>>()?,
                        weights: piece.weights.clone(),
                        periodic: false,
                    })
                };
                let (Some(left), Some(right)) = (
                    vertical(&source.control_points[0]),
                    vertical(source.control_points.last().unwrap()),
                ) else {
                    return Ok(None);
                };
                let next = (index + 1) % wire.len();
                build.rectangular_patch_parametric(
                    surface,
                    [
                        ids[layer][ring][index],
                        ids[layer][ring][next],
                        ids[layer + 1][ring][next],
                        ids[layer + 1][ring][index],
                    ],
                    [
                        sections[layer][ring][index].clone(),
                        right,
                        sections[layer + 1][ring][index].reverse()?,
                        left,
                    ],
                );
            }
        }
    }
    for (end, pose) in [
        (0, motion[0].controls[0]),
        (
            motion.len(),
            *motion.last().unwrap().controls.last().unwrap(),
        ),
    ] {
        let mut surface = material.surface.clone();
        for row in &mut surface.control_points {
            for p in row {
                let Some(q) = image(pose, p, origin) else {
                    return Ok(None);
                };
                *p = q.to_vec();
            }
        }
        let mut wires = Vec::new();
        for (ring, uv) in material.trims.iter().enumerate() {
            let mut coedges = Vec::new();
            for (index, pcurve) in uv.iter().enumerate() {
                coedges.push(build.coedge_parametric(
                    ids[end][ring][index],
                    ids[end][ring][(index + 1) % uv.len()],
                    sections[end][ring][index].clone(),
                    pcurve.clone(),
                    false,
                ));
            }
            wires.push(build.wire(coedges));
        }
        build.face(surface, wires[0], wires[1..].to_vec(), end == 0);
    }
    Ok(Some(build.finish()?))
}
mod binding;
fn profile_join(a: &Curve, b: &Curve, order: usize, max: u64) -> Result<(bool, u64)> {
    use cad_predicates::*;
    let values = a
        .control_points
        .iter()
        .zip(&a.weights)
        .rev()
        .chain(b.control_points.iter().zip(&b.weights))
        .flat_map(|(p, w)| [p[0], p[1], p[2], *w])
        .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
        .collect();
    let source = SourceArena::authored("original-open-fiber-profile-jets", 1, values)
        .map_err(|e| Error::new("BREP_FIBER_INVALID", e.to_string()))?;
    let n = a.control_points.len();
    let controls = (0..n + b.control_points.len())
        .map(|i| std::array::from_fn(|k| source.leaf(i * 4 + k).unwrap()))
        .collect::<Vec<_>>();
    let tolerance = ToleranceContext::default_valid();
    let mut context = PredicateContext::new(
        &source,
        &tolerance,
        Limits {
            max_work: max.min(MAX_WORK),
            ..Default::default()
        },
        None,
    );
    let p = rational_bezier_geometric_endpoint_identity(
        &mut context,
        &controls[..n],
        &controls[n..],
        order,
    )
    .map_err(|e| Error::new("BREP_FIBER_INVALID", e.to_string()))?;
    Ok((p.outcome == BezierIdentity::Equal, p.work_used))
}
fn motion_join(
    a: &RationalPiece,
    b: &RationalPiece,
    order: usize,
    max: u64,
) -> Result<(bool, u64)> {
    use cad_predicates::*;
    let values = a
        .controls
        .iter()
        .zip(&a.weights)
        .chain(b.controls.iter().zip(&b.weights))
        .flat_map(|(p, w)| p.iter().flatten().copied().chain(std::iter::once(*w)))
        .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
        .collect();
    let source = SourceArena::authored("original-open-fiber-motion-jets", 1, values)
        .map_err(|e| Error::new("BREP_FIBER_INVALID", e.to_string()))?;
    let controls = (0..a.controls.len() + b.controls.len())
        .map(|i| std::array::from_fn(|k| source.leaf(i * 13 + k).unwrap()))
        .collect::<Vec<_>>();
    let tolerance = ToleranceContext::default_valid();
    let mut context = PredicateContext::new(
        &source,
        &tolerance,
        Limits {
            max_work: max.min(MAX_WORK),
            ..Default::default()
        },
        None,
    );
    let p = binary_affine_pose_endpoint_jets(
        &mut context,
        &controls[..a.controls.len()],
        &controls[a.controls.len()..],
        order,
    )
    .map_err(|e| Error::new("BREP_FIBER_INVALID", e.to_string()))?;
    Ok((p.outcome == BezierIdentity::Equal, p.work_used))
}
/// Construct from ORIGINAL rational affine coefficients. The global ideal
/// volume, complete actual-source coefficient binding, filled material, every
/// boundary edge and regularity are mandatory. Sharp joins stay explicit C0.
pub fn construct_fibered_body(
    profiles: &[Vec<Curve>],
    material: &LoftCap,
    origin: [f64; 3],
    motion: &[RationalPiece],
    proposal: fibered_volume::Proposal,
    tolerance_uv: f64,
    l: FiberedBodyLimits,
) -> Result<FiberedBodyAttempt> {
    inspect(
        profiles,
        material,
        origin,
        motion,
        proposal,
        tolerance_uv,
        l,
        None,
    )
}
/// Supplied geometry is never repaired or replaced. Verification reproduces
/// expected original coefficient/incidence rows under its explicit row budget.
pub fn verify_fibered_body(
    profiles: &[Vec<Curve>],
    material: &LoftCap,
    origin: [f64; 3],
    motion: &[RationalPiece],
    proposal: fibered_volume::Proposal,
    tolerance_uv: f64,
    l: FiberedBodyLimits,
    model: &Model,
) -> Result<FiberedBodyAttempt> {
    inspect(
        profiles,
        material,
        origin,
        motion,
        proposal,
        tolerance_uv,
        l,
        Some(model),
    )
}
fn inspect(
    profiles: &[Vec<Curve>],
    material: &LoftCap,
    origin: [f64; 3],
    motion: &[RationalPiece],
    proposal: fibered_volume::Proposal,
    tolerance_uv: f64,
    l: FiberedBodyLimits,
    supplied: Option<&Model>,
) -> Result<FiberedBodyAttempt> {
    if let Some(m) = supplied {
        m.validate()?;
    }
    material.surface.validate()?;
    if !(1..=16).contains(&profiles.len())
        || material.trims.len() != profiles.len()
        || profiles
            .iter()
            .zip(&material.trims)
            .any(|(p, t)| !(3..=64).contains(&p.len()) || p.len() != t.len())
        || !tolerance_uv.is_finite()
        || tolerance_uv <= 0.
        || l.order > 2
        || l.volume.cells > 100000
        || l.volume.exact_work > 1000000
        || l.material_pairs > 100000
        || l.material_cells > 100000
        || l.material_domain_cells > 1000000
        || l.profile_cells > 100000
        || l.source_trace_work > 1000000
        || l.binding_rows > 1000000
        || l.agreement_work > 1000000
        || l.regularity_cells > 100000
        || l.normal_cells > 100000
        || l.smoothness_work > 1000000
    {
        return Err(Error::new(
            "BREP_FIBER_INVALID",
            "Invalid original fibered material or limits",
        ));
    }
    for (wire, trims) in profiles.iter().zip(&material.trims) {
        for (curve, uv) in wire.iter().zip(trims) {
            curve.validate()?;
            uv.validate()?;
            if curve.control_points.iter().any(|p| p.len() != 3)
                || uv.control_points.iter().any(|p| p.len() != 2)
            {
                return Err(Error::new(
                    "BREP_FIBER_INVALID",
                    "XYZ original profile and UV trim required",
                ));
            }
        }
    }
    // Validate every original frame/control even when another allowance is zero.
    let _ = fibered_volume::certify(
        &material.surface,
        origin,
        motion,
        proposal,
        fibered_volume::Limits {
            cells: 0,
            exact_work: 0,
        },
    )?;
    let mut r = empty();
    if profiles
        .iter()
        .flatten()
        .chain(material.trims.iter().flatten())
        .any(|c| !bezier(c))
    {
        return Ok(refused(r, "original-fiber-bezier-binding-unproved"));
    }
    let spans = profiles.iter().map(Vec::len).sum::<usize>();
    if spans * motion.len() + 2 > MAX_FACES {
        return Ok(refused(r, "original-fiber-face-limit"));
    }
    if l.material_pairs == 0 || l.material_cells == 0 || l.material_domain_cells == 0 {
        return Ok(refused(r, "original-fiber-material-work-limit"));
    }
    let domain = domains(&material.surface);
    if material
        .trims
        .iter()
        .flatten()
        .flat_map(|c| &c.control_points)
        .any(|p| (0..2).any(|k| p[k] < domain[k][0] || p[k] > domain[k][1]))
    {
        return Ok(refused(r, "original-material-natural-domain-unproved"));
    }
    let trim = nurbs_core::trim_region_audit::inspect(
        &material.trims,
        tolerance_uv,
        l.material_pairs,
        l.material_cells,
        l.material_domain_cells,
    )?;
    let valid = trim.valid == Some(true)
        && trim.winding.first() == Some(&Some(1))
        && trim.winding.iter().skip(1).all(|v| *v == Some(-1));
    r.material = Some(trim);
    if !valid {
        return Ok(refused(r, "original-fiber-material-unproved"));
    }
    for (wire, trims) in profiles.iter().zip(&material.trims) {
        for (curve, uv) in wire.iter().zip(trims) {
            if r.profile_cells == l.profile_cells {
                return Ok(refused(r, "original-profile-regularity-work-limit"));
            }
            let regular =
                nurbs_core::curve_regularity::inspect(curve, l.profile_cells - r.profile_cells)?;
            r.profile_cells += regular.cells;
            if !regular.spanwise_regular {
                return Ok(refused(r, "original-profile-regularity-unproved"));
            }
            if r.source_trace_work == l.source_trace_work {
                return Ok(refused(r, "original-profile-carrier-work-limit"));
            }
            let proof = nurbs_core::curve_surface_agreement::verify_exact(
                curve,
                uv,
                &material.surface,
                false,
                l.source_trace_work - r.source_trace_work,
            )?;
            let Some(p) = proof else {
                return Ok(refused(r, "original-profile-carrier-binding-unproved"));
            };
            r.source_trace_work += p.work_used;
            if p.outcome != cad_predicates::BezierIdentity::Equal {
                return Ok(refused(r, "original-profile-carrier-binding-unproved"));
            }
        }
    }
    let volume = fibered_volume::certify(&material.surface, origin, motion, proposal, l.volume)?;
    let proven = volume.embedding_certified
        && volume.parameter_topology_preserved
        && volume.spanwise_interior_regular;
    r.volume = Some(volume);
    if !proven {
        return Ok(refused(r, "original-fiber-global-volume-unproved"));
    }
    let rows = profiles
        .iter()
        .flatten()
        .map(|c| c.control_points.len())
        .sum::<usize>()
        * (motion.iter().map(|p| p.controls.len()).sum::<usize>() + motion.len() + 5)
        + 2 * material
            .surface
            .control_points
            .iter()
            .map(Vec::len)
            .sum::<usize>()
        + 2 * spans * motion.iter().map(|p| p.controls.len()).sum::<usize>()
        + 2 * material
            .trims
            .iter()
            .flatten()
            .map(|c| c.control_points.len())
            .sum::<usize>()
        + 128 * (spans * motion.len() + 2);
    if rows > l.binding_rows {
        return Ok(refused(r, "original-fiber-binding-row-limit"));
    }
    r.binding_rows = rows;
    let Some(mut wanted) = expected(profiles, material, origin, motion)? else {
        return Ok(refused(r, "original-fiber-exact-construction-unproved"));
    };
    let cap0 = wanted.faces.len() - 2;
    let cap1 = cap0 + 1;
    let direction = r.volume.as_ref().unwrap().height_direction.unwrap();
    let Some(cap_sign) = normal_sign(
        &wanted.faces[cap0].surface,
        domains(&wanted.faces[cap0].surface),
        None,
        proposal.world_rows[2],
        &mut r,
        l.normal_cells,
    )?
    else {
        return Ok(refused(r, "original-fiber-outward-cap-unproved"));
    };
    // Initial cap use is reversed; align the complete source-template shell.
    if -cap_sign != -direction {
        for use_ in &mut wanted.shells[0].faces {
            use_.reversed ^= true;
        }
    }
    wanted.validate()?;
    if supplied.is_some_and(|m| !binding::same(m, &wanted, &mut r, l.binding_rows)) {
        return Ok(refused(
            r,
            "actual-original-fiber-coefficient-binding-unproved",
        ));
    }
    let model = supplied.cloned().unwrap_or(wanted);
    if l.agreement_work == 0 {
        return Ok(refused(r, "actual-original-fiber-agreement-work-limit"));
    }
    let agreement = crate::boundary_agreement::verify_exact(&model, l.agreement_work)?;
    let valid = agreement.all_equal && agreement.all_joins_exact;
    r.agreement = Some(agreement);
    if !valid {
        return Ok(refused(r, "actual-original-fiber-edge-agreement-unproved"));
    }
    r.vertex_links_complete = crate::sweep_smoothness::complete::vertex_links(&model);
    if !r.vertex_links_complete {
        return Ok(refused(r, "actual-original-fiber-vertex-links-unproved"));
    }
    let regular = crate::boundary_embedding::regularity::inspect(&model, l.regularity_cells)?;
    let valid = regular.all_faces_regular;
    r.regularity = Some(regular);
    if !valid {
        return Ok(refused(
            r,
            "actual-original-fiber-boundary-regularity-unproved",
        ));
    }
    if l.order > 0 {
        for pair in motion.windows(2) {
            let (ok, work) = motion_join(
                &pair[0],
                &pair[1],
                l.order,
                l.smoothness_work - r.smoothness_work,
            )?;
            r.smoothness_work += work;
            r.inspected_motion_joins += 1;
            if !ok {
                return Ok(refused(r, "original-multipiece-motion-jets-unproved"));
            }
        }
        if r.volume
            .as_ref()
            .unwrap()
            .endpoint_height_flat_orders
            .unwrap()
            .iter()
            .any(|k| *k < l.order)
        {
            return Ok(refused(r, "original-cap-normal-jets-unproved"));
        }
        for wire in profiles {
            for i in 0..wire.len() {
                let (ok, work) = profile_join(
                    &wire[i],
                    &wire[(i + 1) % wire.len()],
                    l.order,
                    l.smoothness_work - r.smoothness_work,
                )?;
                r.smoothness_work += work;
                r.inspected_profile_joins += 1;
                if !ok {
                    return Ok(refused(r, "original-profile-geometric-jets-unproved"));
                }
            }
        }
        let face_reverse = |face| {
            model.shells[0]
                .faces
                .iter()
                .find(|u| u.face == face)
                .unwrap()
                .reversed
        };
        for (end, cap) in [(0, cap0), (1, cap1)] {
            let expected_sign = if end == 0 { -direction } else { direction };
            let Some(raw) = normal_sign(
                &model.faces[cap].surface,
                domains(&model.faces[cap].surface),
                None,
                proposal.world_rows[2],
                &mut r,
                l.normal_cells,
            )?
            else {
                return Ok(refused(r, "complete-cap-normal-unproved"));
            };
            if raw * (if face_reverse(cap) { -1 } else { 1 }) != expected_sign {
                return Ok(refused(r, "complete-cap-normal-orientation-unproved"));
            }
            let start = if end == 0 {
                0
            } else {
                spans * (motion.len() - 1)
            };
            for face in start..start + spans {
                let s = &model.faces[face].surface;
                let mut domain = domains(s);
                domain[1] = [end as f64; 2];
                let Some(raw) = normal_sign(
                    s,
                    domain,
                    Some(end),
                    proposal.world_rows[2],
                    &mut r,
                    l.normal_cells,
                )?
                else {
                    return Ok(refused(r, "complete-cap-wall-normal-unproved"));
                };
                if raw * (if face_reverse(face) { -1 } else { 1 }) != expected_sign {
                    return Ok(refused(r, "complete-cap-wall-oriented-regularity-unproved"));
                }
            }
        }
    }
    // Exact binding transfers the SAME original global embedding/material
    // boundary and full differential compatibility to the returned geometry.
    r.certified = true;
    r.source_boundary_exact = true;
    r.boundary_error_upper = Some(0.);
    r.topology_preserved = true;
    r.absence_of_self_intersections = true;
    r.source_g1 = l.order >= 1;
    r.source_g2 = l.order == 2;
    r.seam_continuity = match l.order {
        2 => "G2",
        1 => "G1",
        _ => "C0",
    };
    Ok(FiberedBodyAttempt::Certified(Box::new(FiberedBody {
        model,
        report: r,
    })))
}
#[cfg(test)]
mod tests;
