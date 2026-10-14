use super::*;

pub(super) fn exact_vertical_prism(model: &Model) -> Option<VerticalPrismEnvelope> {
    if model
        .faces
        .iter()
        .any(|face| !affine_surface(&face.surface, model.tolerance_mm))
        || model.edges.iter().any(|edge| edge.curve.degree != 1)
    {
        return None;
    }
    let cap = model.faces.iter().find(|face| {
        let points = face
            .surface
            .control_points
            .iter()
            .flatten()
            .collect::<Vec<_>>();
        points
            .iter()
            .all(|p| (p[2] - points[0][2]).abs() <= model.tolerance_mm)
            && face.holes.is_empty()
    })?;
    let wire = &model.loops[cap.outer];
    let profile = wire
        .coedges
        .iter()
        .map(|coedge| {
            let edge = &model.edges[coedge.edge];
            let vertex = if coedge.reversed {
                edge.vertices[1]
            } else {
                edge.vertices[0]
            };
            let p = model.vertices[vertex].point;
            [p[0], p[1]]
        })
        .collect::<Vec<_>>();
    if profile.len() < 3 {
        return None;
    }
    let mut cross_sum = 0.;
    let mut cx_sum = 0.;
    let mut cy_sum = 0.;
    let mut ix = 0.;
    let mut iy = 0.;
    let mut ixy = 0.;
    let mut perimeter = 0.;
    for i in 0..profile.len() {
        let a = profile[i];
        let b = profile[(i + 1) % profile.len()];
        let cross = a[0] * b[1] - b[0] * a[1];
        cross_sum += cross;
        cx_sum += (a[0] + b[0]) * cross;
        cy_sum += (a[1] + b[1]) * cross;
        ix += (a[1] * a[1] + a[1] * b[1] + b[1] * b[1]) * cross;
        iy += (a[0] * a[0] + a[0] * b[0] + b[0] * b[0]) * cross;
        ixy += (2. * a[0] * a[1] + a[0] * b[1] + b[0] * a[1] + 2. * b[0] * b[1]) * cross;
        perimeter += (b[0] - a[0]).hypot(b[1] - a[1]);
    }
    let sign = cross_sum.signum();
    let area = cross_sum.abs() / 2.;
    if area <= model.tolerance_mm.powi(2) {
        return None;
    }
    let cx = cx_sum / (3. * cross_sum);
    let cy = cy_sum / (3. * cross_sum);
    ix *= sign / 12.;
    iy *= sign / 12.;
    ixy *= sign / 24.;
    let mut zmin = f64::INFINITY;
    let mut zmax = f64::NEG_INFINITY;
    for v in &model.vertices {
        zmin = zmin.min(v.point[2]);
        zmax = zmax.max(v.point[2]);
    }
    let h = zmax - zmin;
    let volume = area * h;
    let ix_c = ix - area * cy * cy;
    let iy_c = iy - area * cx * cx;
    let ixy_c = ixy - area * cx * cy;
    let inertia = [
        [h * ix_c + area * h.powi(3) / 12., -h * ixy_c, 0.],
        [-h * ixy_c, h * iy_c + area * h.powi(3) / 12., 0.],
        [0., 0., h * (ix_c + iy_c)],
    ];
    Some((
        2. * area + perimeter * h,
        volume,
        [cx, cy, (zmin + zmax) / 2.],
        inertia,
        h.max(area.sqrt()),
    ))
}

/// Independent analytic enclosures for the finite canonical mass matrix.
/// General quadrature remains available through `mass_properties`, but never
/// acquires certified status.
pub fn certified_mass_properties(model: &Model) -> Result<CertifiedMassProperties> {
    model.validate()?;
    let audit = audit_solid(model)?;
    let context = model.tolerance_context()?;
    let mut parts = Vec::new();
    for body in &model.bodies {
        parts.push((
            exact_component_properties(&crate::solid_audit::isolated_outward_shell(
                model,
                body.outer_shell,
                false,
            )?)?,
            1.,
        ));
        for &shell in &body.inner_shells {
            parts.push((
                exact_component_properties(&crate::solid_audit::isolated_outward_shell(
                    model, shell, true,
                )?)?,
                -1.,
            ));
        }
    }
    if parts.is_empty() {
        return Err(Error::new(
            "BREP_CERTIFIED_MASS_REFUSED",
            "Certified mass requires at least one body",
        ));
    }
    let area = parts.iter().map(|(p, _)| p.area).sum::<f64>();
    let volume = parts.iter().map(|(p, sign)| sign * p.volume).sum::<f64>();
    if volume <= 0. {
        return Err(Error::new(
            "BREP_CERTIFIED_MASS_REFUSED",
            "Signed analytic shell composition has non-positive volume",
        ));
    }
    let centroid = std::array::from_fn(|i| {
        parts
            .iter()
            .map(|(p, sign)| sign * p.volume * p.centroid[i])
            .sum::<f64>()
            / volume
    });
    let mut inertia = [[0.; 3]; 3];
    for (part, sign) in &parts {
        let d = std::array::from_fn::<_, 3, _>(|i| part.centroid[i] - centroid[i]);
        let d2 = d.iter().map(|x| x * x).sum::<f64>();
        for i in 0..3 {
            for j in 0..3 {
                inertia[i][j] += sign
                    * (part.inertia[i][j]
                        + part.volume * ((if i == j { d2 } else { 0. }) - d[i] * d[j]));
            }
        }
    }
    let scale = parts.iter().map(|(p, _)| p.scale).fold(0., f64::max);
    let naming_complete = model.persistent_naming_complete()
        && model.1.faces.len() == model.faces.len()
        && model.1.edges.len() == model.edges.len();
    if !naming_complete {
        return Err(Error::new(
            "BREP_CERTIFIED_MASS_REFUSED",
            "Certified mass lacks complete ChangeSet/naming evidence",
        ));
    }
    let evidence = compose_predicate_evidence(
        &context,
        [
            PredicateEvidence::positional(&context, 0., scale)?,
            PredicateEvidence::topology_preservation(
                &context,
                "analytic mass cell matches audited closed boundary",
                audit.ok,
            )?,
        ],
    )?;
    Ok(CertifiedMassProperties {
        capability: CERTIFIED_MASS_PROPERTIES_CAPABILITY,
        surface_area_mm2: enclosure(area, scale * scale),
        volume_mm3: enclosure(volume, scale.powi(3)),
        centroid: centroid.map(|v| enclosure(v, scale)),
        inertia_mm5: certified_tensor(inertia, scale.powi(5)),
        context: context.spec_identity(),
        evidence,
        audit,
        change_set: model.1.change_set.clone(),
        naming_complete,
        component_count: parts.len(),
        cavity_count: model
            .bodies
            .iter()
            .map(|body| body.inner_shells.len())
            .sum(),
        proof: "closed_form_analytic_with_outward_rounded_binary64_enclosures",
    })
}

/// Append-only freeform mass finite cell for planar equal-weight Bezier AA
/// cuboids (elevated freeform_cuboid_solid). Bump/varying-weight quadrature
/// remains typed-refuse for a successor.
pub const FREEFORM_MASS_CAPABILITY: &str = "certified-generic-rational-freeform-mass-quadrature/1";

pub fn certified_freeform_mass_properties(model: &Model) -> Result<CertifiedMassProperties> {
    model.validate()?;
    let audit = audit_solid(model)?;
    let context = model.tolerance_context()?;
    let (min, max) = axis_aligned_freeform_box(model).ok_or_else(|| {
        Error::new(
            "BREP_CERTIFIED_MASS_REFUSED",
            "Freeform mass /1 admits planar equal-weight Bezier AA cuboids only; bump and varying-weight quadrature remain non-certified",
        )
    })?;
    let d = std::array::from_fn::<_, 3, _>(|i| max[i] - min[i]);
    if d.iter().any(|v| *v <= 0.) {
        return Err(Error::new(
            "BREP_CERTIFIED_MASS_REFUSED",
            "Freeform mass /1 requires a positive AA cuboid extent",
        ));
    }
    let volume = d[0] * d[1] * d[2];
    let area = 2. * (d[0] * d[1] + d[1] * d[2] + d[2] * d[0]);
    let centroid = std::array::from_fn(|i| (min[i] + max[i]) / 2.);
    let inertia = [
        [volume * (d[1] * d[1] + d[2] * d[2]) / 12., 0., 0.],
        [0., volume * (d[0] * d[0] + d[2] * d[2]) / 12., 0.],
        [0., 0., volume * (d[0] * d[0] + d[1] * d[1]) / 12.],
    ];
    let scale = d.into_iter().fold(0., f64::max);
    let naming_complete = model.persistent_naming_complete()
        && model.1.faces.len() == model.faces.len()
        && model.1.edges.len() == model.edges.len();
    if !naming_complete {
        return Err(Error::new(
            "BREP_CERTIFIED_MASS_REFUSED",
            "Freeform mass lacks complete ChangeSet/naming evidence",
        ));
    }
    let evidence = compose_predicate_evidence(
        &context,
        [
            PredicateEvidence::positional(&context, 0., scale)?,
            PredicateEvidence::topology_preservation(
                &context,
                "planar freeform Bezier cuboid matches audited closed boundary",
                audit.ok,
            )?,
        ],
    )?;
    Ok(CertifiedMassProperties {
        capability: FREEFORM_MASS_CAPABILITY,
        surface_area_mm2: enclosure(area, scale * scale),
        volume_mm3: enclosure(volume, scale.powi(3)),
        centroid: centroid.map(|v| enclosure(v, scale)),
        inertia_mm5: certified_tensor(inertia, scale.powi(5)),
        context: context.spec_identity(),
        evidence,
        audit,
        change_set: model.1.change_set.clone(),
        naming_complete,
        component_count: 1,
        cavity_count: 0,
        proof: "closed_form_planar_freeform_bezier_cuboid_enclosures",
    })
}

pub(super) const GAUSS_POLYNOMIAL: [(f64, f64); 5] = [
    (-0.906179845938664, 0.236926885056189),
    (-0.538469310105683, 0.478628670499366),
    (0., 0.568888888888889),
    (0.538469310105683, 0.478628670499366),
    (0.906179845938664, 0.236926885056189),
];
// Eight-point Gauss-Legendre integrates smooth rational patch cells with
// fewer subdivisions. Knot/weight conditioning and both directional error
// estimates still gate acceptance; every flux evaluation is budgeted.
pub(super) const GAUSS_RATIONAL: [(f64, f64); 8] = [
    (-0.9602898564975363, 0.1012285362903763),
    (-0.7966664774136267, 0.2223810344533745),
    (-0.5255324099163290, 0.3137066458778873),
    (-0.1834346424956498, 0.3626837833783620),
    (0.1834346424956498, 0.3626837833783620),
    (0.5255324099163290, 0.3137066458778873),
    (0.7966664774136267, 0.2223810344533745),
    (0.9602898564975363, 0.1012285362903763),
];
#[derive(Clone, Debug)]
pub struct MassProperties {
    pub surface_area_mm2: f64,
    pub signed_volume_mm3: f64,
    pub centroid: [f64; 3],
    /// Integral of (r² I - r rᵀ) about the centroid, for unit density.
    pub inertia_mm5: [[f64; 3]; 3],
    pub conservative_bounds: [[f64; 3]; 2],
    pub area_error_estimate_mm2: f64,
    pub volume_error_estimate_mm3: f64,
    pub evaluations: usize,
}

pub(super) struct Budget {
    pub(super) used: usize,
    pub(super) limit: usize,
}
pub(super) fn spans(knots: &[f64], domain: [f64; 2]) -> Vec<[f64; 2]> {
    knots
        .windows(2)
        .filter_map(|w| {
            let a = w[0].max(domain[0]);
            let b = w[1].min(domain[1]);
            (a < b).then_some([a, b])
        })
        .collect()
}
pub(super) fn spend(budget: &mut Budget, amount: usize) -> Result<()> {
    if amount > budget.limit.saturating_sub(budget.used) {
        return Err(Error::new(
            "BREP_RESOURCE_LIMIT",
            "Mass-property integration exhausted its evaluation budget",
        ));
    }
    budget.used += amount;
    Ok(())
}
/// Positive Bezier weights bound the rational denominator on a complete
/// parameter interval. Refine those bounds before quadrature so a narrow
/// rational feature cannot fall between every quadrature node.
pub(super) fn conditioned_curve_breaks(curve: &Curve, budget: &mut Budget) -> Result<Vec<f64>> {
    let mut scalar = curve.clone();
    scalar.control_points = vec![vec![0., 0.]; curve.control_points.len()];
    spend(budget, scalar.weights.len())?;
    let mut breaks = Vec::new();
    for domain in spans(&scalar.knots, scalar.domain()) {
        let bezier = scalar.trim(domain[0], domain[1])?;
        let mut stack = vec![(domain, bezier.weights, 0)];
        while let Some(([a, b], weights, depth)) = stack.pop() {
            spend(budget, weights.len() * weights.len())?;
            let min = weights.iter().copied().fold(f64::INFINITY, f64::min);
            let max = weights.iter().copied().fold(0., f64::max);
            if max <= min * 4. {
                breaks.extend([a, b]);
                continue;
            }
            let mid = a + (b - a) * 0.5;
            if depth >= 64 || mid == a || mid == b {
                return Err(Error::new(
                    "BREP_ANALYSIS_INDETERMINATE",
                    "Rational denominator cannot be resolved within floating-point parameter precision",
                ));
            }
            let n = weights.len();
            let mut work = weights;
            let mut left = vec![work[0]];
            let mut right = vec![work[n - 1]];
            for level in 1..n {
                for i in 0..n - level {
                    work[i] = (work[i] + work[i + 1]) * 0.5;
                }
                left.push(work[0]);
                right.push(work[n - level - 1]);
            }
            right.reverse();
            stack.push(([mid, b], right, depth + 1));
            stack.push(([a, mid], left, depth + 1));
        }
    }
    breaks.sort_by(f64::total_cmp);
    breaks.dedup();
    Ok(breaks)
}
pub(super) fn conditioned_surface_grid(surface: &Surface, budget: &mut Budget) -> Result<[Vec<f64>; 2]> {
    let mut result: [Vec<f64>; 2] = [vec![], vec![]];
    for (axis, output) in result.iter_mut().enumerate() {
        let (knots, degree, periodic) = if axis == 0 {
            (&surface.knots_u, surface.degree_u, surface.periodic_u)
        } else {
            (&surface.knots_v, surface.degree_v, surface.periodic_v)
        };
        let rows: Vec<Vec<f64>> = if axis == 0 {
            (0..surface.weights[0].len())
                .map(|j| surface.weights.iter().map(|r| r[j]).collect())
                .collect()
        } else {
            surface.weights.clone()
        };
        for weights in rows {
            let curve = Curve {
                degree,
                knots: knots.clone(),
                control_points: vec![vec![0., 0.]; weights.len()],
                weights,
                periodic,
            };
            output.extend(conditioned_curve_breaks(&curve, budget)?);
        }
        output.sort_by(f64::total_cmp);
        output.dedup();
    }
    Ok(result)
}
/// The outer Green integral must break wherever a trim crosses either surface
/// knot family. Splitting only the inner U integral misses arbitrarily narrow
/// V spans and can falsely report quadrature convergence.
pub(super) fn trim_intervals(
    curve: &Curve,
    surface_grid: &[Vec<f64>; 2],
    tolerance: f64,
    budget: &mut Budget,
) -> Result<Vec<[f64; 2]>> {
    use crate::intersections::{Coverage, CurvePlaneComponent, Options, Plane, curve_plane};
    let domain = curve.domain();
    let mut breaks = conditioned_curve_breaks(curve, budget)?;
    let base_spans = spans(&breaks, domain);
    // This boundary contributes Q dv = 0 identically.
    if curve
        .control_points
        .iter()
        .all(|p| p[1] == curve.control_points[0][1])
    {
        return Ok(base_spans);
    }
    for axis in 0..2 {
        let knots = &surface_grid[axis];
        let surface_domain = [knots[0], *knots.last().unwrap()];
        let surface_spans = spans(knots, surface_domain);
        let narrowest = surface_spans
            .iter()
            .map(|s| s[1] - s[0])
            .fold(f64::INFINITY, f64::min);
        let internal = knots
            .iter()
            .copied()
            .filter(|k| *k > surface_domain[0] && *k < surface_domain[1]);
        let low = curve
            .control_points
            .iter()
            .map(|p| p[axis])
            .fold(f64::INFINITY, f64::min);
        let high = curve
            .control_points
            .iter()
            .map(|p| p[axis])
            .fold(f64::NEG_INFINITY, f64::max);
        for knot in internal {
            if knot <= low || knot >= high {
                continue;
            }
            // Linear UV spans have an algebraic inverse. Keep their own knot
            // endpoints, including C0 derivative breaks, in the schedule.
            if curve.degree == 1 && curve.weights.iter().all(|w| *w == curve.weights[0]) {
                for [a, b] in &base_spans {
                    spend(budget, 2)?;
                    let x = curve.evaluate(*a)?.point[axis];
                    let y = curve.evaluate(*b)?.point[axis];
                    if (x < knot && knot < y) || (y < knot && knot < x) {
                        breaks.push(a + (b - a) * (knot - x) / (y - x));
                    }
                }
                continue;
            }
            // Positive rational weights give a conservative first derivative
            // bound from homogeneous control differences. Root uncertainty
            // must be small relative to the narrowest surface knot span.
            let min_weight = curve.weights.iter().copied().fold(f64::INFINITY, f64::min);
            let min_span = base_spans
                .iter()
                .map(|s| s[1] - s[0])
                .fold(f64::INFINITY, f64::min);
            let magnitude = curve
                .control_points
                .iter()
                .map(|p| (p[axis] - knot).abs())
                .fold(0., f64::max);
            let mut numerator_delta: f64 = 0.;
            let mut weight_delta: f64 = 0.;
            for i in 0..curve.weights.len() - 1 {
                numerator_delta = numerator_delta.max(
                    ((curve.control_points[i + 1][axis] - knot) * curve.weights[i + 1]
                        - (curve.control_points[i][axis] - knot) * curve.weights[i])
                        .abs(),
                );
                weight_delta = weight_delta.max((curve.weights[i + 1] - curve.weights[i]).abs());
            }
            let speed_bound = curve.degree as f64 * (numerator_delta + magnitude * weight_delta)
                / (min_span * min_weight);
            let residual_target = narrowest * tolerance * 0.01;
            let parameter_target = (domain[1] - domain[0]) * 1e-13;
            let parameter_target = parameter_target.min(residual_target / speed_bound);
            if !parameter_target.is_finite() || parameter_target <= 0. {
                return Err(Error::new(
                    "BREP_ANALYSIS_INDETERMINATE",
                    "Trim knot crossing cannot be isolated at the requested tolerance",
                ));
            }
            spend(budget, curve.control_points.len())?;
            let remaining = budget.limit.saturating_sub(budget.used).min(65536);
            if remaining == 0 {
                spend(budget, 1)?;
            }
            let mut lifted = curve.clone();
            for p in &mut lifted.control_points {
                p.push(0.);
            }
            let mut normal = [0.; 3];
            normal[axis] = 1.;
            let report = curve_plane(
                &lifted,
                Plane {
                    normal,
                    offset: knot,
                },
                Options {
                    distance_tolerance: residual_target,
                    parameter_tolerance: parameter_target,
                    max_depth: 64,
                    max_boxes: remaining,
                },
            )?;
            spend(budget, report.boxes_visited)?;
            if report.coverage != Coverage::NumericallyResolved || !report.unresolved.is_empty() {
                return Err(Error::new(
                    "BREP_ANALYSIS_INDETERMINATE",
                    "Unresolved trim/surface-knot crossing prevents mass quadrature",
                ));
            }
            for component in report.components {
                match component {
                    CurvePlaneComponent::Point(root) => {
                        if (root.parameter_interval[1] - root.parameter_interval[0]) * speed_bound
                            > residual_target
                        {
                            return Err(Error::new(
                                "BREP_ANALYSIS_INDETERMINATE",
                                "Trim knot root uncertainty exceeds mass quadrature tolerance",
                            ));
                        }
                        breaks.push(root.parameter);
                    }
                    CurvePlaneComponent::Overlap {
                        parameter_interval,
                        control_residual,
                    } => {
                        if control_residual != 0. {
                            return Err(Error::new(
                                "BREP_ANALYSIS_INDETERMINATE",
                                "Near-coincident trim knot span prevents mass quadrature",
                            ));
                        }
                        breaks.extend(parameter_interval);
                    }
                }
            }
        }
    }
    breaks.sort_by(f64::total_cmp);
    breaks.dedup();
    Ok(breaks
        .windows(2)
        .filter_map(|p| (p[0] < p[1]).then_some([p[0], p[1]]))
        .collect())
}
pub(super) fn flux(
    surface: &Surface,
    u: f64,
    v: f64,
    origin: [f64; 3],
    orientation: f64,
    budget: &mut Budget,
) -> Result<[f64; 11]> {
    if budget.used >= budget.limit {
        return Err(Error::new(
            "BREP_RESOURCE_LIMIT",
            "Mass-property integration exhausted its evaluation budget",
        ));
    }
    budget.used += 1;
    let e = surface.evaluate(u, v)?;
    let (du, dv) = e.first_derivatives().ok_or_else(|| {
        Error::new(
            "BREP_ANALYSIS_INDETERMINATE",
            "Undefined surface derivative in integration domain",
        )
    })?;
    let normal = [
        du[1] * dv[2] - du[2] * dv[1],
        du[2] * dv[0] - du[0] * dv[2],
        du[0] * dv[1] - du[1] * dv[0],
    ];
    let area = normal.iter().map(|x| x * x).sum::<f64>().sqrt();
    let n = normal.map(|x| x * orientation);
    let p: [f64; 3] = std::array::from_fn(|i| e.point[i] - origin[i]);
    let mut result = [0.; 11];
    result[0] = area;
    result[1] = (0..3).map(|i| p[i] * n[i] / 3.).sum();
    for i in 0..3 {
        result[2 + i] = p[i] * p[i] * n[i] / 2.;
        result[5 + i] = p[i].powi(3) * n[i] / 3.;
    }
    result[8] = p[0] * p[0] * p[1] * n[0] / 2.;
    result[9] = p[0] * p[0] * p[2] * n[0] / 2.;
    result[10] = p[1] * p[1] * p[2] * n[1] / 2.;
    Ok(result)
}
