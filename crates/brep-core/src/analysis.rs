//! Display-independent mass properties. Green's theorem integrates each authored
//! UV trim; the divergence theorem integrates the oriented closed boundary.
//! Convergence evidence is numerical, never a geometric solid certificate.
use super::*;

const GAUSS: [(f64, f64); 5] = [
    (-0.906179845938664, 0.236926885056189),
    (-0.538469310105683, 0.478628670499366),
    (0., 0.568888888888889),
    (0.538469310105683, 0.478628670499366),
    (0.906179845938664, 0.236926885056189),
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
impl value_codec::Serialize for MassProperties {
    fn to_value(&self) -> value_codec::Value {
        let mut out = value_codec::Map::new();
        for (name, value) in [
            ("surfaceAreaMm2", self.surface_area_mm2.to_value()),
            ("signedVolumeMm3", self.signed_volume_mm3.to_value()),
            ("centroid", self.centroid.to_value()),
            ("inertiaMm5", self.inertia_mm5.to_value()),
            ("conservativeBounds", self.conservative_bounds.to_value()),
            (
                "areaErrorEstimateMm2",
                self.area_error_estimate_mm2.to_value(),
            ),
            (
                "volumeErrorEstimateMm3",
                self.volume_error_estimate_mm3.to_value(),
            ),
            ("evaluations", self.evaluations.to_value()),
            ("status", "converged_estimate".to_value()),
            ("solidGeometryStatus", "not_certified".to_value()),
        ] {
            out.insert(name.into(), value);
        }
        value_codec::Value::Object(out)
    }
}
struct Budget {
    used: usize,
    limit: usize,
}
fn spans(knots: &[f64], domain: [f64; 2]) -> Vec<[f64; 2]> {
    knots
        .windows(2)
        .filter_map(|w| {
            let a = w[0].max(domain[0]);
            let b = w[1].min(domain[1]);
            (a < b).then_some([a, b])
        })
        .collect()
}
fn spend(budget: &mut Budget, amount: usize) -> Result<()> {
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
fn conditioned_curve_breaks(curve: &Curve, budget: &mut Budget) -> Result<Vec<f64>> {
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
fn conditioned_surface_grid(surface: &Surface, budget: &mut Budget) -> Result<[Vec<f64>; 2]> {
    let mut result: [Vec<f64>; 2] = [vec![], vec![]];
    for axis in 0..2 {
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
            result[axis].extend(conditioned_curve_breaks(&curve, budget)?);
        }
        result[axis].sort_by(f64::total_cmp);
        result[axis].dedup();
    }
    Ok(result)
}
/// The outer Green integral must break wherever a trim crosses either surface
/// knot family. Splitting only the inner U integral misses arbitrarily narrow
/// V spans and can falsely report quadrature convergence.
fn trim_intervals(
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
fn flux(
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
fn face_integral(
    model: &Model,
    use_: &FaceUse,
    divisions: usize,
    origin: [f64; 3],
    budget: &mut Budget,
    integration_spans: &BTreeMap<usize, Vec<Vec<[f64; 2]>>>,
    integration_grids: &[[Vec<f64>; 2]],
) -> Result<[f64; 11]> {
    let face = &model.faces[use_.face];
    let surface = &face.surface;
    let u0 = surface.knots_u[surface.degree_u];
    let mut result = [0.; 11];
    for &wire in std::iter::once(&face.outer).chain(&face.holes) {
        for (coedge_index, coedge) in model.loops[wire].coedges.iter().enumerate() {
            // Pcurves already follow their loop; coedge.reversed applies to the 3D carrier only.
            let curve = &coedge.pcurve;
            for &[start, end] in &integration_spans[&wire][coedge_index] {
                for part in 0..divisions {
                    let a = start + (end - start) * part as f64 / divisions as f64;
                    let b = start + (end - start) * (part + 1) as f64 / divisions as f64;
                    for (x, w) in GAUSS {
                        let evaluation = curve.evaluate((a + b) / 2. + x * (b - a) / 2.)?;
                        let dv = evaluation.d1.ok_or_else(|| {
                            Error::new("BREP_ANALYSIS_INDETERMINATE", "Undefined trim derivative")
                        })?[1];
                        if dv == 0. {
                            continue;
                        }
                        let [u, v] = [evaluation.point[0], evaluation.point[1]];
                        for [low, high] in spans(&integration_grids[use_.face][0], [u0, u]) {
                            for inner in 0..divisions {
                                let l = low + (high - low) * inner as f64 / divisions as f64;
                                let h = low + (high - low) * (inner + 1) as f64 / divisions as f64;
                                for (ix, iw) in GAUSS {
                                    let values = flux(
                                        surface,
                                        (l + h) / 2. + ix * (h - l) / 2.,
                                        v,
                                        origin,
                                        if use_.reversed { -1. } else { 1. },
                                        budget,
                                    )?;
                                    let weight = w * (b - a) / 2. * dv * iw * (h - l) / 2.;
                                    for i in 0..11 {
                                        result[i] += weight * values[i];
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(result)
}
/// Relative quadrature convergence target; error estimates do not bound all
/// floating-point or geometry errors and must not certify solid validity.
pub fn mass_properties(
    model: &Model,
    relative_tolerance: f64,
    max_evaluations: usize,
) -> Result<MassProperties> {
    model.validate()?;
    if !(1e-10..=1e-3).contains(&relative_tolerance)
        || !(100..=2_000_000).contains(&max_evaluations)
    {
        return Err(Error::new(
            "BREP_INVALID_ANALYSIS_OPTIONS",
            "Relative tolerance must be 1e-10..1e-3 and evaluation budget 100..2000000",
        ));
    }
    if model.bodies.is_empty() || model.shells.iter().any(|s| !s.closed) {
        return Err(Error::new(
            "BREP_UNSUPPORTED_OPERATION",
            "Mass properties require closed body shells",
        ));
    }
    let mut bounds = [[f64::INFINITY; 3], [f64::NEG_INFINITY; 3]];
    for p in model
        .faces
        .iter()
        .flat_map(|f| f.surface.control_points.iter().flatten())
    {
        for i in 0..3 {
            bounds[0][i] = bounds[0][i].min(p[i]);
            bounds[1][i] = bounds[1][i].max(p[i]);
        }
    }
    let origin = std::array::from_fn(|i| (bounds[0][i] + bounds[1][i]) / 2.);
    let scale = (0..3)
        .map(|i| bounds[1][i] - bounds[0][i])
        .fold(0., f64::max);
    let scales: [f64; 11] = std::array::from_fn(|i| {
        scale.powi(if i == 0 {
            2
        } else if i == 1 {
            3
        } else if i < 5 {
            4
        } else {
            5
        })
    });
    let uses: Vec<_> = model.shells.iter().flat_map(|s| s.faces.iter()).collect();
    let mut budget = Budget {
        used: 0,
        limit: max_evaluations,
    };
    let integration_grids = model
        .faces
        .iter()
        .map(|face| conditioned_surface_grid(&face.surface, &mut budget))
        .collect::<Result<Vec<_>>>()?;
    let mut integration_spans = BTreeMap::new();
    for (face_index, face) in model.faces.iter().enumerate() {
        for &wire in std::iter::once(&face.outer).chain(&face.holes) {
            let schedule = model.loops[wire]
                .coedges
                .iter()
                .map(|c| {
                    trim_intervals(
                        &c.pcurve,
                        &integration_grids[face_index],
                        relative_tolerance,
                        &mut budget,
                    )
                })
                .collect::<Result<_>>()?;
            integration_spans.insert(wire, schedule);
        }
    }
    let mut previous: Option<Vec<[f64; 11]>> = None;
    for divisions in [1, 2, 4, 8, 16] {
        let current: Vec<_> = uses
            .iter()
            .map(|u| {
                face_integral(
                    model,
                    u,
                    divisions,
                    origin,
                    &mut budget,
                    &integration_spans,
                    &integration_grids,
                )
            })
            .collect::<Result<_>>()?;
        if let Some(old) = &previous {
            let mut errors = [0.; 11];
            let mut total = [0.; 11];
            for (a, b) in old.iter().zip(&current) {
                for i in 0..11 {
                    errors[i] += (a[i] - b[i]).abs();
                    total[i] += b[i];
                }
            }
            if (0..11)
                .all(|i| errors[i] <= relative_tolerance * total[i].abs().max(scales[i] * 1e-3))
            {
                let volume = total[1];
                if volume <= scales[1] * 1e-14 {
                    return Err(Error::new(
                        "BREP_ANALYSIS_INDETERMINATE",
                        "Boundary has non-positive or numerically unresolved signed volume",
                    ));
                }
                let c: [f64; 3] = std::array::from_fn(|i| total[2 + i] / volume);
                let moments: [[f64; 3]; 3] = [
                    [total[5], total[8], total[9]],
                    [total[8], total[6], total[10]],
                    [total[9], total[10], total[7]],
                ];
                let central: [[f64; 3]; 3] = std::array::from_fn(|i| {
                    std::array::from_fn(|j| moments[i][j] - volume * c[i] * c[j])
                });
                let trace = (0..3).map(|i| central[i][i]).sum::<f64>();
                let inertia = std::array::from_fn(|i| {
                    std::array::from_fn(|j| {
                        if i == j {
                            trace - central[i][j]
                        } else {
                            -central[i][j]
                        }
                    })
                });
                if total
                    .iter()
                    .chain(inertia.iter().flatten())
                    .any(|x| !x.is_finite())
                {
                    return Err(Error::new(
                        "BREP_ANALYSIS_INDETERMINATE",
                        "Non-finite mass integral",
                    ));
                }
                return Ok(MassProperties {
                    surface_area_mm2: total[0],
                    signed_volume_mm3: volume,
                    centroid: std::array::from_fn(|i| origin[i] + c[i]),
                    inertia_mm5: inertia,
                    conservative_bounds: bounds,
                    area_error_estimate_mm2: errors[0],
                    volume_error_estimate_mm3: errors[1],
                    evaluations: budget.used,
                });
            }
        }
        previous = Some(current);
    }
    Err(Error::new(
        "BREP_ANALYSIS_INDETERMINATE",
        "Mass-property quadrature did not converge within 16 subdivisions",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn near(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-7 * b.abs().max(1.), "{a} != {b}");
    }
    #[test]
    fn unresolved_tangent_knot_crossing_cannot_report_convergence() {
        let curve = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![0., 0.], vec![0.5, 1.], vec![1., 0.]],
            weights: vec![1.; 3],
            periodic: false,
        };
        let mut surface = cuboid([0.; 3], [1.; 3]).unwrap().faces[0].surface.clone();
        surface = surface
            .edit_axis(nurbs_core::surface::Axis::V, |c| c.insert(0.5, 1))
            .unwrap();
        let mut budget = Budget {
            used: 0,
            limit: 200_000,
        };
        let grid = conditioned_surface_grid(&surface, &mut budget).unwrap();
        let error = trim_intervals(&curve, &grid, 1e-7, &mut budget).unwrap_err();
        assert_eq!(error.code, "BREP_ANALYSIS_INDETERMINATE");
        assert!(
            budget.used > curve.control_points.len(),
            "Root search work must count against the budget"
        );
    }
    #[test]
    fn planar_and_round_solids_match_analytic_mass_properties() {
        let b =
            mass_properties(&cuboid([1., 2., 3.], [3., 6., 9.]).unwrap(), 1e-7, 200_000).unwrap();
        near(b.signed_volume_mm3, 48.);
        near(b.surface_area_mm2, 88.);
        assert_eq!(b.centroid, [2., 4., 6.]);
        near(b.inertia_mm5[0][0], 48. * (16. + 36.) / 12.);
        let c = mass_properties(&cylinder(3., 8.).unwrap(), 1e-7, 200_000).unwrap();
        near(c.signed_volume_mm3, 72. * std::f64::consts::PI);
        near(c.surface_area_mm2, 66. * std::f64::consts::PI);
        near(c.centroid[2], 4.);
        near(c.inertia_mm5[2][2], c.signed_volume_mm3 * 9. / 2.);
        let t = mass_properties(&tube(3., 2., 8.).unwrap(), 1e-7, 200_000).unwrap();
        near(t.signed_volume_mm3, 40. * std::f64::consts::PI);
        near(t.surface_area_mm2, 90. * std::f64::consts::PI);
        near(t.inertia_mm5[2][2], t.signed_volume_mm3 * 13. / 2.);
    }
    #[test]
    fn holes_translations_and_resource_refusals_are_explicit() {
        let outline = [[0., 0.], [4., 0.], [4., 4.], [2., 4.], [2., 2.], [0., 2.]];
        let h = vec![[2.5, 0.5], [2.5, 1.5], [3.5, 1.5], [3.5, 0.5]];
        let mut m = extrude_polygon_with_holes(&outline, &[h], 0., 3.).unwrap();
        let before = mass_properties(&m, 1e-7, 200_000).unwrap();
        near(before.signed_volume_mm3, 33.);
        let delta = [10000., -50000., 70000.];
        for v in &mut m.vertices {
            for i in 0..3 {
                v.point[i] += delta[i];
            }
        }
        for e in &mut m.edges {
            for p in &mut e.curve.control_points {
                for i in 0..3 {
                    p[i] += delta[i];
                }
            }
        }
        for f in &mut m.faces {
            for p in f.surface.control_points.iter_mut().flatten() {
                for i in 0..3 {
                    p[i] += delta[i];
                }
            }
        }
        let after = mass_properties(&m, 1e-7, 200_000).unwrap();
        near(after.signed_volume_mm3, before.signed_volume_mm3);
        for i in 0..3 {
            near(after.centroid[i], before.centroid[i] + delta[i]);
            near(after.inertia_mm5[i][i], before.inertia_mm5[i][i]);
        }
        assert_eq!(
            mass_properties(&m, 1e-7, 100).unwrap_err().code,
            "BREP_RESOURCE_LIMIT"
        );
    }
}
