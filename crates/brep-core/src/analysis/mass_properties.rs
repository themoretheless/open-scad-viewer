use super::*;

/// The outer Green integral must break wherever a trim crosses either surface
/// knot family. Splitting only the inner U integral misses arbitrarily narrow
/// V spans and can falsely report quadrature convergence.
pub(super) fn trim_intervals(
    curve: &Curve,
    surface_grid: &[Vec<f64>; 2],
    tolerance: f64,
    budget: &mut Budget,
) -> Result<Vec<[f64; 2]>> {
    use nurbs_intersect::{Coverage, CurvePlaneComponent, Options, Plane, curve_plane};
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
    divisions: [usize; 2],
    origin: [f64; 3],
    budget: &mut Budget,
    integration_spans: &BTreeMap<usize, Vec<Vec<[f64; 2]>>>,
    integration_grids: &[[Vec<f64>; 2]],
) -> Result<[f64; 11]> {
    let face = &model.faces[use_.face];
    let surface = &face.surface;
    // Constant homogeneous weights cancel to a polynomial patch; retain the
    // cheaper rule for these faces. Varying positive weights need the higher
    // order rational rule. Both routes use the same directional convergence
    // estimates, conditioning schedules and shared evaluation budget.
    let quadrature: &[(f64, f64)] = if surface.weights.iter().flatten()
        .all(|weight| *weight == surface.weights[0][0]) {
        &GAUSS_POLYNOMIAL
    } else {
        &GAUSS_RATIONAL
    };
    let u0 = surface.knots_u[surface.degree_u];
    let mut result = [0.; 11];
    for &wire in std::iter::once(&face.outer).chain(&face.holes) {
        for (coedge_index, coedge) in model.loops[wire].coedges.iter().enumerate() {
            // Pcurves already follow their loop; coedge.reversed applies to the 3D carrier only.
            let curve = &coedge.pcurve;
            for &[start, end] in &integration_spans[&wire][coedge_index] {
                for part in 0..divisions[0] {
                    let a = start + (end - start) * part as f64 / divisions[0] as f64;
                    let b = start + (end - start) * (part + 1) as f64 / divisions[0] as f64;
                    for &(x, w) in quadrature {
                        let evaluation = curve.evaluate(((a + b) / 2. + x * (b - a) / 2.).clamp(a,b))?;
                        let dv = evaluation.d1.ok_or_else(|| {
                            Error::new("BREP_ANALYSIS_INDETERMINATE", "Undefined trim derivative")
                        })?[1];
                        if dv == 0. {
                            continue;
                        }
                        let mut uv = [evaluation.point[0], evaluation.point[1]];
                        // A positive rational trim stays in its control hull.
                        // World-coordinate charts can round an endpoint one ULP
                        // outside the surface. Clamp only when the complete
                        // original trim hull proves containment in that domain.
                        let domains=[[surface.knots_u[surface.degree_u],surface.knots_u[surface.knots_u.len()-surface.degree_u-1]],
                            [surface.knots_v[surface.degree_v],surface.knots_v[surface.knots_v.len()-surface.degree_v-1]]];
                        if curve.control_points.iter().all(|p|(0..2).all(|k|p[k]>=domains[k][0]&&p[k]<=domains[k][1])) {
                            for k in 0..2 {uv[k]=uv[k].clamp(domains[k][0],domains[k][1]);}
                        }
                        let [u,v]=uv;
                        for [low, high] in spans(&integration_grids[use_.face][0], [u0, u]) {
                            for inner in 0..divisions[1] {
                                let l = low + (high - low) * inner as f64 / divisions[1] as f64;
                                let h =
                                    low + (high - low) * (inner + 1) as f64 / divisions[1] as f64;
                                for &(ix, iw) in quadrature {
                                    let values = flux(
                                        surface,
                                        // Every quadrature node lies in its admitted interval.
                                        // Rounding on a narrow world-coordinate span may
                                        // otherwise place it one ULP before the lower knot.
                                        ((l + h) / 2. + ix * (h - l) / 2.).clamp(l,h),
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
    // Independently refine the Green boundary integral and its surface
    // antiderivative. Each face retains both directional error estimates;
    // accurate directions are not uniformly refined with harder neighbors.
    let integrate = |index: usize, divisions: [usize; 2], budget: &mut Budget| {
        face_integral(
            model,
            uses[index],
            divisions,
            origin,
            budget,
            &integration_spans,
            &integration_grids,
        )
        .map_err(|error| {
            if error.code == "BREP_RESOURCE_LIMIT" {
                Error::new(
                    "BREP_RESOURCE_LIMIT",
                    format!(
                        "{} (face {}, subdivisions {:?}, evaluations {}/{})",
                        error.message, uses[index].face, divisions, budget.used, budget.limit
                    ),
                )
            } else {
                error
            }
        })
    };
    let mut divisions = vec![[2, 2]; uses.len()];
    let mut current = (0..uses.len())
        .map(|i| integrate(i, [2, 2], &mut budget))
        .collect::<Result<Vec<_>>>()?;
    let mut face_errors = vec![[[0.; 11]; 2]; uses.len()];
    for index in 0..uses.len() {
        for axis in 0..2 {
            let mut coarse_divisions = divisions[index];
            coarse_divisions[axis] /= 2;
            let coarse = integrate(index, coarse_divisions, &mut budget)?;
            face_errors[index][axis] =
                std::array::from_fn(|i| (current[index][i] - coarse[i]).abs());
        }
    }
    loop {
        let mut errors = [0.; 11];
        let mut total = [0.; 11];
        for (index, values) in current.iter().enumerate() {
            for i in 0..11 {
                errors[i] += face_errors[index][0][i] + face_errors[index][1][i];
                total[i] += values[i];
            }
        }
        if (0..11).all(|i| errors[i] <= relative_tolerance * total[i].abs().max(scales[i] * 1e-3)) {
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
        let next = (0..uses.len())
            .flat_map(|index| (0..2).map(move |axis| (index, axis)))
            .filter(|&(index, axis)| divisions[index][axis] < 16)
            .max_by(|&(a, aa), &(b, bb)| {
                let score = |index: usize, axis: usize| {
                    (0..11)
                        .map(|i| face_errors[index][axis][i] / total[i].abs().max(scales[i] * 1e-3))
                        .fold(0_f64, f64::max)
                };
                score(a, aa).total_cmp(&score(b, bb))
            });
        let Some((index, axis)) = next else { break };
        let old = current[index];
        divisions[index][axis] *= 2;
        let refined = integrate(index, divisions[index], &mut budget)?;
        face_errors[index][axis] = std::array::from_fn(|i| (refined[i] - old[i]).abs());
        // Refresh the other direction at the new grid; mixed dependence must
        // not inherit a stale error estimate from the earlier face grid.
        let other = 1 - axis;
        let mut coarse_divisions = divisions[index];
        coarse_divisions[other] /= 2;
        let coarse = integrate(index, coarse_divisions, &mut budget)?;
        face_errors[index][other] = std::array::from_fn(|i| (refined[i] - coarse[i]).abs());
        current[index] = refined;
    }
    Err(Error::new(
        "BREP_ANALYSIS_INDETERMINATE",
        "Mass-property quadrature did not converge within 16 subdivisions",
    ))
}
