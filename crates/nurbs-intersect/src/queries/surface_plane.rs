use super::*;
#[derive(Clone, Debug)]
pub enum SurfacePlaneComponent {
    Curve {
        trace: SurfaceTrace,
        /// Parameter region supporting the branch, including root uncertainty.
        parameter_box: [f64; 4],
        samples: Vec<SurfacePoint>,
        max_sample_residual: f64,
    },
    Point(SurfacePoint),
    Overlap {
        parameter_box: [f64; 4],
        control_residual: f64,
    },
}

/// Enclose residual construction from retained trimmed Cartesian coefficients
/// and the normalized plane. Prior trim and plane normalization are not covered.
pub(crate) fn ruled_boundary_bounds(surface: &Surface, plane: Plane) -> Vec<[Interval; 2]> {
    let scale = surface.weights.iter().flatten().copied().fold(0., f64::max);
    surface
        .control_points
        .iter()
        .zip(&surface.weights)
        .map(|(points, weights)| {
            std::array::from_fn(|side| {
                let mut residual = Interval::exact(-plane.offset);
                for axis in 0..3 {
                    residual = residual.add(
                        Interval::exact(plane.normal[axis])
                            .mul(Interval::exact(points[side][axis])),
                    );
                }
                let weight = if weights[side] == scale {
                    Interval::exact(1.)
                } else {
                    let ratio = weights[side] / scale;
                    Interval {
                        lo: ratio.next_down().max(0.),
                        hi: ratio.next_up(),
                    }
                };
                residual.mul(weight)
            })
        })
        .collect()
}
#[cfg(test)]
pub(crate) fn admit_ruled_parameter_range(values: &[[f64; 2]]) -> Result<()> {
    if values.iter().flatten().any(|v| !v.is_finite()) {
        return Err(invalid("Nonfinite ruled boundary residual"));
    }
    admit_ruled_parameter_bounds(values.iter().map(|r| r.map(Interval::exact)).collect())
}
/// Bernstein bounds cover residual construction and subdivision rounding, but
/// not earlier knot edits or plane normalization. This is not a certificate.
pub(crate) fn admit_ruled_parameter_bounds(bounds: Vec<[Interval; 2]>) -> Result<()> {
    let mut pending = vec![(bounds, 0usize)];
    let mut visited = 0;
    while let Some((row, depth)) = pending.pop() {
        visited += 1;
        if visited > 4096 {
            return Err(Error::new(
                "BREP_INTERSECTION_UNRESOLVED",
                "Ruled parameter range admission exceeded its work budget",
            ));
        }
        let opposite = row.iter().all(|r| r[0].hi <= 0. && r[1].lo >= 0.)
            || row.iter().all(|r| r[0].lo >= 0. && r[1].hi <= 0.);
        if opposite {
            continue;
        }
        // Endpoint Bernstein coefficients enclose actual endpoint residuals.
        // Equal strict signs mean that the plane lies outside that ruling.
        for r in [row.first().unwrap(), row.last().unwrap()] {
            if r[0].sign() != 0 && r[0].sign() == r[1].sign() {
                return Err(invalid("Ruled trace leaves the source parameter domain"));
            }
        }
        if depth >= 32 {
            return Err(Error::new(
                "BREP_INTERSECTION_UNRESOLVED",
                "Ruled parameter range remains unresolved at the subdivision limit",
            ));
        }
        let mut work = row;
        let mut left = vec![work[0]];
        let mut right = vec![*work.last().unwrap()];
        while work.len() > 1 {
            work = work
                .windows(2)
                .map(|r| {
                    std::array::from_fn(|axis| r[0][axis].add(r[1][axis]).mul(Interval::exact(0.5)))
                })
                .collect();
            left.push(work[0]);
            right.push(*work.last().unwrap());
        }
        right.reverse();
        pending.push((right, depth + 1));
        pending.push((left, depth + 1));
    }
    Ok(())
}

/// Keep each ruled section span independently, avoiding any endpoint welding.
/// The source U interval is mapped to the same global fraction as evaluate().
pub(crate) fn ruled_segments(surface: &Surface, plane: Plane, interval: [f64; 2]) -> Result<Vec<Curve>> {
    let degree = surface.degree_u;
    if degree > 12 || interval[0] == interval[1] {
        return Err(invalid(
            "Trace conversion requires nonzero interval and result degree at most 25",
        ));
    }
    let lo = interval[0].min(interval[1]);
    let hi = interval[0].max(interval[1]);
    let domain = surface_domain(surface);
    let mut breaks = vec![lo, hi];
    breaks.extend(
        surface
            .knots_u
            .iter()
            .copied()
            .filter(|u| *u > lo && *u < hi),
    );
    breaks.sort_by(f64::total_cmp);
    breaks.dedup();
    if (breaks.len() - 1) * (2 * degree + 1) > 256 {
        return Err(invalid("Converted trace exceeds 256 control points"));
    }
    if interval[0] > interval[1] {
        breaks.reverse();
    }
    let mut result = Vec::new();
    for span in breaks.windows(2) {
        let a = span[0].min(span[1]);
        let b = span[0].max(span[1]);
        let piece = surface.trim([a, b, domain[2], domain[3]])?;
        if piece.control_points.len() != degree + 1 {
            return Err(invalid(
                "Ruled knot partition does not isolate one Bezier span",
            ));
        }
        let mut curve = SurfaceTrace::Ruled {
            surface: piece,
            plane,
            u_interval: [span[0], span[1]],
        }
        .to_curve()?;
        let t0 = if span[0] == interval[0] {
            0.
        } else {
            (span[0] - interval[0]) / (interval[1] - interval[0])
        };
        let t1 = if span[1] == interval[1] {
            1.
        } else {
            (span[1] - interval[0]) / (interval[1] - interval[0])
        };
        if !t0.is_finite() || !t1.is_finite() || t0 >= t1 {
            return Err(invalid("Ruled knot crossing interval is not representable"));
        }
        // A Bezier piece has only its two clamped endpoint knots. Reverse()
        // reverses coefficients while retaining this ascending source domain.
        for knot in &mut curve.knots {
            *knot = if *knot == a { t0 } else { t1 };
        }
        curve.validate()?;
        result.push(curve);
    }
    Ok(result)
}

/// Partition an affine UV trace at source knot crossings. Every piece remains
/// a homogeneous tensor restriction, and joins require identical endpoint
/// values; tolerance welding is deliberately not part of this conversion.
pub(crate) fn diagonal_segments(
    surface: &Surface,
    plane: Plane,
    start: [f64; 2],
    end: [f64; 2],
) -> Result<Vec<Curve>> {
    let degree = surface.degree_u + surface.degree_v;
    let mut crossings = vec![(0., None), (1., None)];
    for (axis, knots) in [(0, &surface.knots_u), (1, &surface.knots_v)] {
        for &knot in knots {
            if knot > start[axis].min(end[axis]) && knot < start[axis].max(end[axis]) {
                let t = (knot - start[axis]) / (end[axis] - start[axis]);
                if !t.is_finite() || t <= 0. || t >= 1. {
                    return Err(invalid("UV knot crossing is not representable"));
                }
                crossings.push((t, Some((axis, knot))));
            }
        }
    }
    crossings.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut breaks: Vec<(f64, [f64; 2], [bool; 2])> = Vec::new();
    for (t, crossing) in crossings {
        if breaks.last().is_none_or(|b| b.0 != t) {
            let uv = if t == 0. {
                start
            } else if t == 1. {
                end
            } else {
                std::array::from_fn(|axis| start[axis] + t * (end[axis] - start[axis]))
            };
            breaks.push((t, uv, [false; 2]));
        }
        if let Some((axis, knot)) = crossing {
            let b = breaks.last_mut().unwrap();
            if b.2[axis] && b.1[axis] != knot {
                return Err(invalid("Distinct UV knots collapse to one trace parameter"));
            }
            b.1[axis] = knot;
            b.2[axis] = true;
        }
    }
    if (breaks.len() - 1) * (degree + 1) > 256 {
        return Err(invalid("Converted trace exceeds 256 control points"));
    }
    let mut result = Vec::new();
    for pair in breaks.windows(2) {
        let (t0, a, _) = pair[0];
        let (t1, b, _) = pair[1];
        if a[0] == b[0] || a[1] == b[1] {
            return Err(invalid("UV knot crossing interval is not representable"));
        }
        let piece = surface.trim([
            a[0].min(b[0]),
            a[0].max(b[0]),
            a[1].min(b[1]),
            a[1].max(b[1]),
        ])?;
        if piece.control_points.len() != piece.degree_u + 1
            || piece.control_points[0].len() != piece.degree_v + 1
        {
            return Err(invalid(
                "UV knot partition does not isolate one tensor Bezier span",
            ));
        }
        let mut segment = SurfaceTrace::Line {
            surface: piece,
            plane,
            start: a,
            end: b,
        }
        .to_curve()?
        .elevate(degree)?;
        // Single-span conversion is parameterized on [0,1]. Preserve the
        // original trace fraction globally, including nonuniform crossings.
        for knot in &mut segment.knots {
            *knot = if *knot == 0. { t0 } else { t1 };
        }
        segment.validate()?;
        result.push(segment);
    }
    Ok(result)
}
pub(crate) fn diagonal_multispan(
    surface: &Surface,
    plane: Plane,
    start: [f64; 2],
    end: [f64; 2],
) -> Result<Curve> {
    let degree = surface.degree_u + surface.degree_v;
    let mut result: Option<Curve> = None;
    for segment in diagonal_segments(surface, plane, start, end)? {
        if let Some(joined) = &mut result {
            if joined.control_points.last() != segment.control_points.first() {
                return Err(invalid(
                    "Converted span endpoints do not coincide numerically",
                ));
            }
            let factor = joined.weights.last().unwrap() / segment.weights[0];
            joined.knots.pop();
            joined.knots.extend_from_slice(&segment.knots[degree + 1..]);
            joined
                .control_points
                .extend_from_slice(&segment.control_points[1..]);
            joined
                .weights
                .extend(segment.weights[1..].iter().map(|w| w * factor));
        } else {
            result = Some(segment);
        }
    }
    let mut curve = result.ok_or_else(|| invalid("Trace has no nonzero spans"))?;
    let scale = curve.weights.iter().copied().fold(0., f64::max);
    for weight in &mut curve.weights {
        *weight /= scale;
    }
    curve.validate()?;
    Ok(curve)
}

#[doc(hidden)]
pub fn transpose_surface(s: &Surface) -> Surface {
    Surface {
        degree_u: s.degree_v,
        degree_v: s.degree_u,
        knots_u: s.knots_v.clone(),
        knots_v: s.knots_u.clone(),
        control_points: (0..s.control_points[0].len())
            .map(|v| s.control_points.iter().map(|row| row[v].clone()).collect())
            .collect(),
        weights: (0..s.weights[0].len())
            .map(|v| s.weights.iter().map(|row| row[v]).collect())
            .collect(),
        periodic_u: s.periodic_v,
        periodic_v: s.periodic_u,
    }
}
#[doc(hidden)]
pub fn swap_trace(trace: SurfaceTrace) -> SurfaceTrace {
    match trace {
        SurfaceTrace::Line {
            surface,
            plane,
            start,
            end,
        } => SurfaceTrace::Line {
            surface: transpose_surface(&surface),
            plane,
            start: [start[1], start[0]],
            end: [end[1], end[0]],
        },
        SurfaceTrace::Ruled {
            surface,
            plane,
            u_interval,
        } => SurfaceTrace::RuledU {
            surface: transpose_surface(&surface),
            plane,
            v_interval: u_interval,
        },
        SurfaceTrace::RuledU {
            surface,
            plane,
            v_interval,
        } => SurfaceTrace::Ruled {
            surface: transpose_surface(&surface),
            plane,
            u_interval: v_interval,
        },
    }
}

#[doc(hidden)]
pub fn surface_domain(surface: &Surface) -> [f64; 4] {
    [
        surface.knots_u[surface.degree_u],
        surface.knots_u[surface.control_points.len()],
        surface.knots_v[surface.degree_v],
        surface.knots_v[surface.control_points[0].len()],
    ]
}

#[derive(Clone, Copy)]
pub(crate) struct AffineFrame {
    pub(crate) origin: [f64; 3],
    pub(crate) u: [f64; 3],
    pub(crate) v: [f64; 3],
    pub(crate) domain: [f64; 4],
}
pub(crate) fn affine_frame(surface: &Surface) -> Option<AffineFrame> {
    if surface.degree_u != 1
        || surface.degree_v != 1
        || surface.control_points.len() != 2
        || surface.control_points[0].len() != 2
        || surface.periodic_u
        || surface.periodic_v
        || !surface
            .weights
            .iter()
            .flatten()
            .all(|w| *w == surface.weights[0][0])
    {
        return None;
    }
    let p = &surface.control_points;
    let origin = point3(&p[0][0]);
    let u = sub(point3(&p[1][0]), origin);
    let v = sub(point3(&p[0][1]), origin);
    if (0..3).any(|i| p[1][1][i] != origin[i] + u[i] + v[i]) {
        return None;
    }
    let n = cross(u, v);
    if dot(n, n) <= f64::MIN_POSITIVE {
        return None;
    }
    Some(AffineFrame {
        origin,
        u,
        v,
        domain: surface_domain(surface),
    })
}

fn add_trace(
    report: &mut Report<SurfacePlaneComponent>,
    parameter_box: [f64; 4],
    trace: SurfaceTrace,
    tolerance: f64,
) -> Result<()> {
    let samples = (0..=8)
        .map(|i| trace.evaluate(i as f64 / 8.))
        .collect::<Result<Vec<_>>>()?;
    let max_sample_residual = samples.iter().map(|p| p.plane_residual).fold(0., f64::max);
    if max_sample_residual > tolerance {
        report.unresolved(parameter_box, UnresolvedReason::NearCoincidence);
    } else {
        report.components.push(SurfacePlaneComponent::Curve {
            trace,
            parameter_box,
            samples,
            max_sample_residual,
        });
    }
    Ok(())
}

/// Plane sections of untrimmed NURBS patches. Affine planes and rational ruled
/// patches linear in V retain procedural parameter traces. General patches may
/// be excluded by Bernstein control bounds; unresolved regions are explicit.
/// Face trims, branch sewing, tangency certification and arbitrary SS are later
/// stages: callers must not infer a trimmed B-rep section from this report alone.
pub fn surface_plane(
    surface: &Surface,
    plane: Plane,
    options: Options,
) -> Result<Report<SurfacePlaneComponent>> {
    surface.validate()?;
    let plane = plane.normalized()?;
    let options = options.validate()?;
    let linear_u = surface.degree_u == 1;
    let linear_v = surface.degree_v == 1;
    if linear_u && !linear_v {
        let mut report = surface_plane(&transpose_surface(surface), plane, options)?;
        let swap_box = |b: &mut [f64]| {
            b.swap(0, 2);
            b.swap(1, 3);
        };
        for pending in &mut report.unresolved {
            swap_box(&mut pending.parameter_box);
        }
        for component in &mut report.components {
            match component {
                SurfacePlaneComponent::Curve {
                    trace,
                    parameter_box,
                    samples,
                    ..
                } => {
                    *trace = swap_trace(trace.clone());
                    swap_box(parameter_box);
                    for sample in samples {
                        sample.uv.swap(0, 1);
                    }
                }
                SurfacePlaneComponent::Point(point) => point.uv.swap(0, 1),
                SurfacePlaneComponent::Overlap { parameter_box, .. } => swap_box(parameter_box),
            }
        }
        return Ok(report);
    }
    let mut report = Report::default();
    if let Some(frame) = affine_frame(surface) {
        report.boxes_visited = 1;
        let domain = frame.domain;
        let uv = [
            [domain[0], domain[2]],
            [domain[1], domain[2]],
            [domain[1], domain[3]],
            [domain[0], domain[3]],
        ];
        let points = uv
            .map(|p| surface.evaluate(p[0], p[1]).map(|j| j.point))
            .into_iter()
            .collect::<Result<Vec<_>>>()?;
        let distances: Vec<_> = points.iter().map(|p| plane.distance(*p)).collect();
        if distances.iter().all(|d| *d == 0.) {
            report.components.push(SurfacePlaneComponent::Overlap {
                parameter_box: domain,
                control_residual: 0.,
            });
            return Ok(report);
        }
        let mut hits = Vec::new();
        for i in 0..4 {
            let next = (i + 1) % 4;
            if distances[i] == 0. {
                hits.push(uv[i]);
            }
            if distances[i] * distances[next] < 0. {
                let t = distances[i] / (distances[i] - distances[next]);
                hits.push(std::array::from_fn(|d| {
                    uv[i][d] + t * (uv[next][d] - uv[i][d])
                }));
            }
        }
        hits.dedup();
        match hits.len() {
            0 => {
                report.bernstein_excluded = 1;
            }
            1 => {
                let point = surface.evaluate(hits[0][0], hits[0][1])?.point;
                report
                    .components
                    .push(SurfacePlaneComponent::Point(SurfacePoint {
                        uv: hits[0],
                        point,
                        plane_residual: plane.distance(point).abs(),
                    }));
            }
            2 => add_trace(
                &mut report,
                domain,
                SurfaceTrace::Line {
                    surface: surface.clone(),
                    plane,
                    start: hits[0],
                    end: hits[1],
                },
                options.distance_tolerance,
            )?,
            _ => report.unresolved(domain, UnresolvedReason::NearCoincidence),
        }
        return Ok(report);
    }
    let mut pending = std::collections::VecDeque::new();
    for u in spans(
        &surface.knots_u,
        surface.degree_u,
        surface.control_points.len(),
    ) {
        for v in spans(
            &surface.knots_v,
            surface.degree_v,
            surface.control_points[0].len(),
        ) {
            pending.push_back(([u[0], u[1], v[0], v[1]], 0usize));
        }
    }
    while let Some((domain, depth)) = pending.pop_front() {
        if report.boxes_visited >= options.max_boxes || report.components.len() >= 1024 {
            report.unresolved(domain, UnresolvedReason::BudgetExceeded);
            continue;
        }
        report.boxes_visited += 1;
        let piece = surface.trim(domain)?;
        let values: Vec<_> = piece
            .control_points
            .iter()
            .zip(&piece.weights)
            .flat_map(|(p, w)| {
                let c = Curve {
                    degree: 1,
                    knots: vec![],
                    control_points: p.clone(),
                    weights: w.clone(),
                    periodic: false,
                };
                coefficients(&c, plane)
            })
            .collect();
        if excluded(&values) {
            report.bernstein_excluded += 1;
            continue;
        }
        if values.iter().all(|c| c.value == 0.) {
            report.components.push(SurfacePlaneComponent::Overlap {
                parameter_box: domain,
                control_residual: 0.,
            });
            continue;
        }
        let ruled = piece.degree_v == 1 && piece.control_points[0].len() == 2;
        if !ruled {
            report.unresolved(domain, UnresolvedReason::UnsupportedSurface);
            continue;
        }
        let lower: Vec<_> = values.iter().step_by(2).copied().collect();
        let upper: Vec<_> = values.iter().skip(1).step_by(2).copied().collect();
        let boundary_scale = piece.weights[0][1] / piece.weights[0][0];
        let proportional_boundaries = boundary_scale.is_finite()
            && boundary_scale > 0.
            && piece.weights.iter().all(|w| w[1] / w[0] == boundary_scale)
            && piece
                .control_points
                .iter()
                .all(|row| plane.distance(point3(&row[0])) == plane.distance(point3(&row[1])));
        if lower.iter().zip(&upper).all(|(a, b)| a.value == b.value) || proportional_boundaries {
            // A plane parallel to every ruling reduces to a boundary-curve
            // root query. Each root carries its parameter interval through to
            // a whole generator line; no sampled 3D curve fitting is needed.
            if report.boxes_visited == options.max_boxes {
                report.unresolved(domain, UnresolvedReason::BudgetExceeded);
                continue;
            }
            let boundary = piece.iso(nurbs_core::surface::Axis::V, domain[2])?;
            let roots = curve_plane(
                &boundary,
                plane,
                Options {
                    max_boxes: options.max_boxes - report.boxes_visited,
                    ..options
                },
            )?;
            report.boxes_visited += roots.boxes_visited;
            report.bernstein_excluded += roots.bernstein_excluded;
            for pending in roots.unresolved {
                report.unresolved(
                    [
                        pending.parameter_box[0],
                        pending.parameter_box[1],
                        domain[2],
                        domain[3],
                    ],
                    pending.reason,
                );
            }
            for component in roots.components {
                match component {
                    CurvePlaneComponent::Point(root) => {
                        add_trace(
                            &mut report,
                            [
                                root.parameter_interval[0],
                                root.parameter_interval[1],
                                domain[2],
                                domain[3],
                            ],
                            SurfaceTrace::Line {
                                surface: piece.clone(),
                                plane,
                                start: [root.parameter, domain[2]],
                                end: [root.parameter, domain[3]],
                            },
                            options.distance_tolerance,
                        )?;
                    }
                    CurvePlaneComponent::Overlap {
                        parameter_interval,
                        control_residual,
                    } => {
                        report.components.push(SurfacePlaneComponent::Overlap {
                            parameter_box: [
                                parameter_interval[0],
                                parameter_interval[1],
                                domain[2],
                                domain[3],
                            ],
                            control_residual,
                        });
                    }
                }
            }
            continue;
        }
        let side = |v: &[Coefficient]| {
            if v.iter().all(|c| c.bound.sign() == 1) {
                1
            } else if v.iter().all(|c| c.bound.sign() == -1) {
                -1
            } else {
                0
            }
        };
        let lower_zero = lower.iter().all(|c| c.value == 0.);
        let upper_zero = upper.iter().all(|c| c.value == 0.);
        if (lower_zero && side(&upper) != 0) || (upper_zero && side(&lower) != 0) {
            let v = if lower_zero { domain[2] } else { domain[3] };
            // A continuous shared knot is owned by the preceding V span.
            // Full-multiplicity discontinuities retain both independent sides.
            let shared_lower = lower_zero
                && v > surface_domain(surface)[2]
                && surface.knots_v.iter().filter(|k| **k == v).count() <= surface.degree_v;
            if !shared_lower {
                add_trace(
                    &mut report,
                    [domain[0], domain[1], v, v],
                    SurfaceTrace::Line {
                        surface: piece,
                        plane,
                        start: [domain[0], v],
                        end: [domain[1], v],
                    },
                    options.distance_tolerance,
                )?;
            }
            continue;
        }
        if side(&lower) * side(&upper) == -1 {
            // Full cross-ruling branch: the denominator cannot vanish because
            // the two boundary curves are uniformly on opposite plane sides.
            let trace = SurfaceTrace::Ruled {
                surface: piece,
                plane,
                u_interval: [domain[0], domain[1]],
            };
            add_trace(&mut report, domain, trace, options.distance_tolerance)?;
            continue;
        }
        let width = domain[1] - domain[0];
        let midpoint = domain[0] + width * 0.5;
        if width <= options.parameter_tolerance
            || depth == options.max_depth
            || midpoint == domain[0]
            || midpoint == domain[1]
        {
            report.unresolved(domain, UnresolvedReason::BoundaryCrossing);
            continue;
        }
        pending.push_back(([midpoint, domain[1], domain[2], domain[3]], depth + 1));
        pending.push_back(([domain[0], midpoint, domain[2], domain[3]], depth + 1));
    }
    Ok(report)
}
