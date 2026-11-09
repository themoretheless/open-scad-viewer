use super::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RepairBudget {
    pub tolerance: f64,
    pub max_extension_uv: f64,
    pub max_snap_uv: f64,
    pub max_projection_iterations: usize,
    pub agreement_cells: usize,
}

impl Default for RepairBudget {
    fn default() -> Self {
        Self {
            tolerance: 1e-7,
            max_extension_uv: 0.25,
            max_snap_uv: 1e-5,
            max_projection_iterations: 24,
            agreement_cells: 4096,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum RepairAction {
    PeriodicLift { axis: usize, periods: i32 },
    DomainClamp { axis: usize, amount: f64 },
    EndpointSnap { end: usize, amount: f64 },
    TangentExtension { end: usize, amount: f64 },
    RebuiltFromEdge,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RepairResult {
    pub pcurve: Curve,
    pub actions: Vec<RepairAction>,
    pub before: AgreementAudit,
    pub after: AgreementAudit,
}

fn invert_near(
    surface: &Surface,
    target: &[f64],
    seed: [f64; 2],
    iterations: usize,
) -> Result<([f64; 2], f64)> {
    let bounds = domain(surface);
    let mut uv = seed;
    for _ in 0..iterations {
        let evaluation = surface.evaluate(uv[0], uv[1])?;
        let residual = [
            target[0] - evaluation.point[0],
            target[1] - evaluation.point[1],
            target[2] - evaluation.point[2],
        ];
        let d = norm(&residual)?;
        if d <= 1e-12 {
            return Ok((uv, d));
        }
        let Some((du, dv)) = evaluation.first_derivatives() else {
            break;
        };
        let dot = |a: [f64; 3], b: [f64; 3]| a.into_iter().zip(b).map(|(x, y)| x * y).sum::<f64>();
        let aa = dot(du, du);
        let ab = dot(du, dv);
        let bb = dot(dv, dv);
        let det = aa * bb - ab * ab;
        if !det.is_finite() || det <= f64::MIN_POSITIVE {
            return Err(error(
                PCURVE_AMBIGUOUS,
                "Endpoint inversion is singular at a pole",
            ));
        }
        let step = [
            (bb * dot(du, residual) - ab * dot(dv, residual)) / det,
            (aa * dot(dv, residual) - ab * dot(du, residual)) / det,
        ];
        let next = [uv[0] + step[0], uv[1] + step[1]];
        if next.iter().any(|x| !x.is_finite()) {
            return Err(error(PCURVE_INVALID, "Endpoint inversion produced NaN/Inf"));
        }
        uv = [
            next[0].clamp(bounds[0][0], bounds[0][1]),
            next[1].clamp(bounds[1][0], bounds[1][1]),
        ];
    }
    let point = surface.evaluate(uv[0], uv[1])?.point;
    Ok((uv, distance(&point, target)?))
}

/// Bounded repair for periodic jumps, tiny domain excursions, endpoint gaps
/// and short tangent extensions. Every changed candidate must pass the same
/// 33-node audit; otherwise a fresh edge-driven pullback is attempted. No
/// ambiguous pole inversion, large snap or large extension is guessed.
pub fn repair_pcurve(
    edge: &Curve,
    imported: &Curve,
    surface: &Surface,
    periods: SurfacePeriods,
    same_sense: bool,
    budget: RepairBudget,
) -> Result<RepairResult> {
    if !budget.tolerance.is_finite()
        || budget.tolerance <= 0.
        || !budget.max_extension_uv.is_finite()
        || budget.max_extension_uv < 0.
        || !budget.max_snap_uv.is_finite()
        || budget.max_snap_uv < 0.
        || !(1..=64).contains(&budget.max_projection_iterations)
        || !(1..=100000).contains(&budget.agreement_cells)
    {
        return Err(error(PCURVE_BUDGET, "Invalid pcurve repair budget"));
    }
    let options = AuditOptions {
        tolerance: budget.tolerance,
        samples: AGREEMENT_SAMPLES,
    };
    let before = audit_same_parameter(edge, imported, surface, periods, same_sense, options)
        .unwrap_or_else(|_| AgreementAudit {
            consistent: false,
            same_parameter: true,
            max_distance: f64::INFINITY,
            worst_sample: 0,
            samples: AGREEMENT_SAMPLES,
        });
    if before.consistent {
        return Ok(RepairResult {
            pcurve: imported.clone(),
            actions: Vec::new(),
            after: before.clone(),
            before,
        });
    }
    let oriented;
    let edge = if same_sense {
        edge
    } else {
        oriented = edge.reverse()?;
        &oriented
    };
    let mut candidate = if imported.degree == 1 && !imported.periodic {
        let (points, _) = unwrap_points(&curve_points(imported)?, periods)?;
        polyline(points)?
    } else {
        imported.clone()
    };
    let mut actions = Vec::new();
    let bounds = domain(surface);
    for (axis, period) in [periods.u, periods.v].into_iter().enumerate() {
        if let Some(period) = period {
            for point in &mut candidate.control_points {
                let wrapped = period.wrap(point[axis])?;
                let shift = ((wrapped - point[axis]) / period.period()).round();
                if shift != 0. {
                    if shift < i32::MIN as f64 || shift > i32::MAX as f64 {
                        return Err(error(PCURVE_INVALID, "Repair lift exceeds i32 range"));
                    }
                    actions.push(RepairAction::PeriodicLift {
                        axis,
                        periods: shift as i32,
                    });
                    point[axis] = wrapped;
                }
            }
        } else {
            for point in &mut candidate.control_points {
                let clamped = point[axis].clamp(bounds[axis][0], bounds[axis][1]);
                let amount = (clamped - point[axis]).abs();
                if amount > 0. {
                    if amount > budget.max_snap_uv {
                        return Err(error(
                            PCURVE_AMBIGUOUS,
                            "Domain excursion exceeds the bounded snap",
                        ));
                    }
                    point[axis] = clamped;
                    actions.push(RepairAction::DomainClamp { axis, amount });
                }
            }
        }
    }
    candidate.validate()?;
    for end in 0..2 {
        let et = if end == 0 {
            edge.domain()[0]
        } else {
            edge.domain()[1]
        };
        let target = edge.evaluate(et)?.point;
        let index = if end == 0 {
            0
        } else {
            candidate.control_points.len() - 1
        };
        let seed = [
            candidate.control_points[index][0],
            candidate.control_points[index][1],
        ];
        let (uv, residual) = invert_near(surface, &target, seed, budget.max_projection_iterations)?;
        if residual > budget.tolerance {
            continue;
        }
        let amount = distance(&seed, &uv)?;
        if amount <= budget.max_snap_uv {
            candidate.control_points[index] = uv.to_vec();
            if amount > 0. {
                actions.push(RepairAction::EndpointSnap { end, amount });
            }
        } else if amount <= budget.max_extension_uv && imported.degree == 1 {
            let neighbor = if end == 0 {
                1
            } else {
                candidate.control_points.len() - 2
            };
            let tangent = if end == 0 {
                [
                    seed[0] - candidate.control_points[neighbor][0],
                    seed[1] - candidate.control_points[neighbor][1],
                ]
            } else {
                [
                    candidate.control_points[neighbor][0] - seed[0],
                    candidate.control_points[neighbor][1] - seed[1],
                ]
            };
            let extension = if end == 0 {
                [seed[0] - uv[0], seed[1] - uv[1]]
            } else {
                [uv[0] - seed[0], uv[1] - seed[1]]
            };
            let cross = (tangent[0] * extension[1] - tangent[1] * extension[0]).abs();
            if norm(&tangent)? == 0. || cross > budget.tolerance * norm(&tangent)? * amount.max(1.)
            {
                return Err(error(
                    PCURVE_AMBIGUOUS,
                    format!(
                        "Endpoint extension is not tangent to the authored pcurve (cross={cross})"
                    ),
                ));
            }
            if end == 0 {
                candidate.control_points.insert(0, uv.to_vec());
                candidate.weights.insert(0, 1.);
            } else {
                candidate.control_points.push(uv.to_vec());
                candidate.weights.push(1.);
            }
            let points = candidate
                .control_points
                .iter()
                .map(|p| Ok([p[0], p[1]]))
                .collect::<Result<Vec<_>>>()?;
            candidate = polyline(points)?;
            actions.push(RepairAction::TangentExtension { end, amount });
        } else if amount > budget.max_extension_uv {
            return Err(error(
                PCURVE_AMBIGUOUS,
                "Endpoint extension exceeds its bound",
            ));
        }
    }
    candidate.validate()?;
    let mut after = audit_same_parameter(edge, &candidate, surface, periods, true, options)?;
    if !after.consistent {
        let report = curve_pullback::project(
            edge,
            surface,
            curve_pullback::Limits {
                tolerance: budget.tolerance,
                max_segments: 64,
                max_iterations: budget.max_projection_iterations,
                agreement_cells: budget.agreement_cells,
            },
        )?;
        candidate = report.pcurve.ok_or_else(|| {
            error(
                PCURVE_UNRESOLVED,
                "No bounded repair or certified pullback passed",
            )
        })?;
        actions.push(RepairAction::RebuiltFromEdge);
        after = audit_same_parameter(edge, &candidate, surface, periods, true, options)?;
    }
    if !after.consistent {
        return Err(error(
            PCURVE_UNRESOLVED,
            "Repair candidate failed its final same-parameter audit",
        ));
    }
    Ok(RepairResult {
        pcurve: candidate,
        actions,
        before,
        after,
    })
}

#[derive(Clone, Debug, PartialEq)]
pub struct OrientedCoedge {
    pub pcurve: Curve,
    pub reversed: bool,
    pub same_sense: bool,
    pub changed: bool,
}

/// Align pcurve traversal with coedge reversal and STEP edge same_sense. Both
/// orientations are measured; a tie is ambiguous and never silently flipped.
pub fn reconcile_coedge_orientation(
    edge: &Curve,
    pcurve: &Curve,
    surface: &Surface,
    reversed: bool,
    same_sense: bool,
    options: AuditOptions,
) -> Result<OrientedCoedge> {
    let expected_reverse = reversed ^ !same_sense;
    let oriented;
    let edge = if expected_reverse {
        oriented = edge.reverse()?;
        &oriented
    } else {
        edge
    };
    let forward = audit_same_parameter(
        edge,
        pcurve,
        surface,
        SurfacePeriods::default(),
        true,
        options,
    )?;
    if forward.consistent {
        return Ok(OrientedCoedge {
            pcurve: pcurve.clone(),
            reversed,
            same_sense,
            changed: false,
        });
    }
    let reversed_pcurve = pcurve.reverse()?;
    let backward = audit_same_parameter(
        edge,
        &reversed_pcurve,
        surface,
        SurfacePeriods::default(),
        true,
        options,
    )?;
    if !backward.consistent {
        return Err(error(
            PCURVE_UNRESOLVED,
            "Neither pcurve orientation agrees with the oriented edge",
        ));
    }
    Ok(OrientedCoedge {
        pcurve: reversed_pcurve,
        reversed,
        same_sense,
        changed: true,
    })
}

/// Verify EDGE_CURVE.same_sense against authored vertices. A false result is a
/// defect report, not an implicit geometric reversal.
pub fn same_sense_matches_vertices(
    edge: &Curve,
    start: &[f64],
    end: &[f64],
    same_sense: bool,
    tolerance: f64,
) -> Result<bool> {
    edge.validate()?;
    if !tolerance.is_finite() || tolerance < 0. {
        return Err(error(PCURVE_INVALID, "Invalid vertex tolerance"));
    }
    let [a, b] = edge.domain();
    let direct = distance(&edge.evaluate(a)?.point, start)? <= tolerance
        && distance(&edge.evaluate(b)?.point, end)? <= tolerance;
    let opposite = distance(&edge.evaluate(a)?.point, end)? <= tolerance
        && distance(&edge.evaluate(b)?.point, start)? <= tolerance;
    if direct == opposite {
        return Err(error(
            PCURVE_AMBIGUOUS,
            "Closed or degenerate edge has ambiguous same_sense",
        ));
    }
    Ok(if same_sense { direct } else { opposite })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoopKind {
    Outer,
    Hole,
}

/// Normalize UV loop winding (outer CCW, holes CW) while retaining coedge
/// same_sense. Reversing a coedge changes only its traversal flag and pcurve.
pub fn normalize_loop_winding(coedges: &mut [OrientedCoedge], kind: LoopKind) -> Result<f64> {
    if coedges.len() < 2 {
        return Err(error(PCURVE_INVALID, "A loop needs at least two coedges"));
    }
    let mut twice_area = 0.;
    let mut previous_end: Option<[f64; 2]> = None;
    let mut first_start = None;
    for coedge in coedges.iter() {
        let points = curve_points(&coedge.pcurve)?;
        first_start.get_or_insert(points[0]);
        if previous_end
            .is_some_and(|end| distance(&end, &points[0]).map(|d| d > 1e-9).unwrap_or(true))
        {
            return Err(error(
                PCURVE_AMBIGUOUS,
                "Cannot orient a discontinuous UV loop",
            ));
        }
        previous_end = points.last().copied();
        for pair in points.windows(2) {
            twice_area += pair[0][0] * pair[1][1] - pair[1][0] * pair[0][1];
        }
    }
    if distance(
        &previous_end.expect("nonempty"),
        &first_start.expect("nonempty"),
    )? > 1e-9
    {
        return Err(error(PCURVE_AMBIGUOUS, "Cannot orient an open UV loop"));
    }
    if !twice_area.is_finite() || twice_area.abs() <= f64::EPSILON {
        return Err(error(
            PCURVE_AMBIGUOUS,
            "Loop signed area is zero or non-finite",
        ));
    }
    let wrong = match kind {
        LoopKind::Outer => twice_area < 0.,
        LoopKind::Hole => twice_area > 0.,
    };
    if wrong {
        coedges.reverse();
        for coedge in coedges {
            coedge.pcurve = coedge.pcurve.reverse()?;
            coedge.reversed = !coedge.reversed;
            coedge.changed = true;
        }
    }
    Ok(twice_area * 0.5)
}
