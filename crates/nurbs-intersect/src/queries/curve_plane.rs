use super::*;
#[derive(Clone, Debug)]
pub enum CurvePlaneComponent {
    Point(CurvePoint),
    /// All retained Bezier control points lie numerically in the plane.
    Overlap {
        parameter_interval: [f64; 2],
        control_residual: f64,
    },
}

#[derive(Clone, Copy)]
pub(crate) struct Coefficient {
    pub(crate) value: f64,
    pub(crate) bound: Interval,
}
pub(crate) fn coefficients(curve: &Curve, plane: Plane) -> Vec<Coefficient> {
    curve
        .control_points
        .iter()
        .zip(&curve.weights)
        .map(|(point, weight)| {
            let mut bound = Interval::exact(-plane.offset);
            for (a, b) in plane.normal.into_iter().zip(point) {
                bound = bound.add(Interval::exact(a).mul(Interval::exact(*b)));
            }
            Coefficient {
                value: plane.distance(point3(point)) * weight,
                bound: bound.mul(Interval::exact(*weight)),
            }
        })
        .collect()
}
fn split(coefficients: &[Coefficient]) -> (Vec<Coefficient>, Vec<Coefficient>) {
    let mut row = coefficients.to_vec();
    let mut left = vec![row[0]];
    let mut right = vec![*row.last().unwrap()];
    while row.len() > 1 {
        row = row
            .windows(2)
            .map(|p| Coefficient {
                value: (p[0].value + p[1].value) * 0.5,
                bound: p[0].bound.add(p[1].bound).mul(Interval::exact(0.5)),
            })
            .collect();
        left.push(row[0]);
        right.push(*row.last().unwrap());
    }
    right.reverse();
    (left, right)
}
pub(crate) fn excluded(coefficients: &[Coefficient]) -> bool {
    coefficients.iter().all(|c| c.bound.sign() == 1)
        || coefficients.iter().all(|c| c.bound.sign() == -1)
}
fn sign_variations(coefficients: &[Coefficient]) -> usize {
    let signs: Vec<_> = coefficients
        .iter()
        .filter_map(|c| {
            if c.value == 0. {
                None
            } else {
                Some(c.value > 0.)
            }
        })
        .collect();
    signs.windows(2).filter(|s| s[0] != s[1]).count()
}

/// Isolate intersections of any validated positive-weight 3D NURBS curve with
/// a plane, over the curve's entire active knot domain. Multiple/tangent roots,
/// precision bands and work exhaustion stay explicit instead of becoming empty.
pub fn curve_plane(
    curve: &Curve,
    plane: Plane,
    options: Options,
) -> Result<Report<CurvePlaneComponent>> {
    curve.validate()?;
    if curve.control_points[0].len() != 3 {
        return Err(invalid("Curve/plane requires a 3D curve"));
    }
    let plane = plane.normalized()?;
    let options = options.validate()?;
    let mut report = Report::default();
    let mut pending: std::collections::VecDeque<([f64; 2], Option<Vec<Coefficient>>, usize)> =
        spans(&curve.knots, curve.degree, curve.control_points.len())
            .into_iter()
            .map(|domain| (domain, None, 0))
            .collect();
    while let Some((interval, values, depth)) = pending.pop_front() {
        if report.boxes_visited >= options.max_boxes {
            report.unresolved(interval, UnresolvedReason::BudgetExceeded);
            continue;
        }
        report.boxes_visited += 1;
        let initial_span = values.is_none();
        let values = match values {
            Some(values) => values,
            None => coefficients(&curve.trim(interval[0], interval[1])?, plane),
        };
        if initial_span && values.iter().all(|c| c.value == 0.) {
            report.components.push(CurvePlaneComponent::Overlap {
                parameter_interval: interval,
                control_residual: 0.,
            });
            continue;
        }
        if excluded(&values) {
            report.bernstein_excluded += 1;
            continue;
        }
        // Endpoints belong to both neighboring spans; only exact numerical
        // zeros are merged, never distinct roots merely closer than a tol.
        for (parameter, coefficient) in [
            (interval[0], values[0]),
            (interval[1], *values.last().unwrap()),
        ] {
            if coefficient.value == 0. {
                let jet = curve.evaluate(parameter)?;
                let point = point3(&jet.point);
                if plane.distance(point).abs() <= options.distance_tolerance {
                    // At a C0 knot the two-sided jet does not exist; screen
                    // transversality with the one-sided span jets and accept
                    // either side that confirms a transverse crossing.
                    let transverse = curve_tangents(curve, parameter)?
                        .iter()
                        .any(|d| dot(plane.normal, *d).abs() > options.distance_tolerance);
                    if parameter != curve.domain()[0]
                        && parameter != curve.domain()[1]
                        && !transverse
                    {
                        report.unresolved(interval, UnresolvedReason::TangencyOrMultipleRoot);
                    } else if !report.components.iter().any(
                        |c| matches!(c, CurvePlaneComponent::Point(p) if p.parameter == parameter),
                    ) {
                        report
                            .components
                            .push(CurvePlaneComponent::Point(CurvePoint {
                                parameter,
                                parameter_interval: [parameter, parameter],
                                point,
                                plane_residual: plane.distance(point).abs(),
                                contact: Contact::Boundary,
                            }));
                    }
                } else {
                    report.unresolved(interval, UnresolvedReason::NearCoincidence);
                }
            }
        }
        let variations = sign_variations(&values);
        if variations == 0 {
            // A root bitwise on a shared box face leaves no sign variation:
            // decide both faces by exact evaluation, but only where the
            // face's own coefficient bound cannot exclude zero — a strict
            // bound proves the face value is no rounding-scale contact.
            // Half-open ownership makes exactly one box report the face;
            // the halo box stays silent.
            let domain_hi = curve.domain()[1];
            let mut handled = false;
            for (face, coefficient) in [
                (interval[0], values[0]),
                (interval[1], *values.last().unwrap()),
            ] {
                if coefficient.bound.sign() != 0 {
                    continue;
                }
                match plane_face_root(curve, plane, interval, face, options)? {
                    FaceRoot::Root(point) => {
                        if owns_parameter(interval[0], interval[1], domain_hi, face)
                            && !report.components.iter().any(
                                |c| matches!(c, CurvePlaneComponent::Point(p) if p.parameter == point.parameter),
                            )
                        {
                            report.components.push(CurvePlaneComponent::Point(point));
                        }
                        handled = true;
                    }
                    FaceRoot::Ambiguous => {
                        report.unresolved(interval, UnresolvedReason::TangencyOrMultipleRoot);
                        handled = true;
                    }
                    FaceRoot::Miss => {}
                }
            }
            if handled {
                // Interior coefficients with zero-containing bounds keep
                // their explicit band; a confirmed face does not clear them.
                if values[1..values.len() - 1]
                    .iter()
                    .any(|c| c.value != 0. && c.bound.sign() == 0)
                {
                    report.unresolved(interval, UnresolvedReason::NearCoincidence);
                }
                continue;
            }
            // Numerically one-sided Bernstein coefficients. This is not an
            // interval exclusion if zero-containing coefficient intervals
            // remain; coverage is deliberately only numerical throughout.
            if values.iter().any(|c| c.value != 0. && c.bound.sign() == 0) {
                report.unresolved(interval, UnresolvedReason::NearCoincidence);
            }
            continue;
        }
        let width = interval[1] - interval[0];
        let midpoint = interval[0] + width * 0.5;
        if width <= options.parameter_tolerance
            || depth == options.max_depth
            || midpoint == interval[0]
            || midpoint == interval[1]
        {
            // Terminal boxes compare the midpoint candidate against exact
            // face evaluations: a face whose residual is not beaten by the
            // midpoint carries the root — report it at the exact face
            // parameter with this box as the isolating interval. Exact
            // parameters dedup against the neighbor's own face report, and
            // the final touching-interval merge absorbs any remaining
            // midpoint/face pair beside a shared face.
            let point = point3(&curve.evaluate(midpoint)?.point);
            let residual = plane.distance(point).abs();
            let midpoint_confirms = variations == 1 && residual <= options.distance_tolerance;
            let mut best_face: Option<(f64, f64)> = None;
            for &face in &[interval[0], interval[1]] {
                let face_residual = plane.distance(point3(&curve.evaluate(face)?.point)).abs();
                if face_residual <= options.distance_tolerance {
                    best_face = Some(match best_face {
                        Some((f, r)) if r <= face_residual => (f, r),
                        _ => (face, face_residual),
                    });
                }
            }
            if let Some((face, face_residual)) = best_face
                && (!midpoint_confirms || face_residual <= residual)
            {
                match plane_face_root(curve, plane, interval, face, options)? {
                    FaceRoot::Root(mut point) => {
                        point.parameter_interval = interval;
                        if !report.components.iter().any(
                                |c| matches!(c, CurvePlaneComponent::Point(p) if p.parameter == point.parameter),
                            ) {
                                report.components.push(CurvePlaneComponent::Point(point));
                            }
                    }
                    FaceRoot::Ambiguous => {
                        report.unresolved(interval, UnresolvedReason::TangencyOrMultipleRoot)
                    }
                    FaceRoot::Miss => {}
                }
                if variations > 1 {
                    report.unresolved(interval, UnresolvedReason::TangencyOrMultipleRoot);
                }
                continue;
            }
            if midpoint_confirms {
                report
                    .components
                    .push(CurvePlaneComponent::Point(CurvePoint {
                        parameter: midpoint,
                        parameter_interval: interval,
                        point,
                        plane_residual: residual,
                        contact: Contact::Transverse,
                    }));
            } else {
                report.unresolved(interval, UnresolvedReason::TangencyOrMultipleRoot);
            }
            continue;
        }
        let (left, right) = split(&values);
        pending.push_back(([interval[0], midpoint], Some(left), depth + 1));
        pending.push_back(([midpoint, interval[1]], Some(right), depth + 1));
    }
    merge_plane_points(&mut report);
    report.components.sort_by(|a, b| {
        let parameter = |c: &CurvePlaneComponent| match c {
            CurvePlaneComponent::Point(p) => p.parameter,
            CurvePlaneComponent::Overlap {
                parameter_interval, ..
            } => parameter_interval[0],
        };
        parameter(a).total_cmp(&parameter(b))
    });
    Ok(report)
}
/// Merge curve/plane point events whose isolating intervals touch in
/// parameter space — the same subdivision-tiling adjacency rule as
/// merge_curve_points, never spatial proximity. The merged event keeps the
/// lowest-residual parameter and the union of the isolating intervals, so a
/// root sitting on a shared box face is reported exactly once.
fn merge_plane_points(report: &mut Report<CurvePlaneComponent>) {
    let interval = |c: &CurvePlaneComponent| match c {
        CurvePlaneComponent::Point(p) => Some(p.parameter_interval),
        _ => None,
    };
    let n = report.components.len();
    let mut parent: Vec<usize> = (0..n).collect();
    fn find(parent: &mut [usize], mut i: usize) -> usize {
        while parent[i] != i {
            parent[i] = parent[parent[i]];
            i = parent[i];
        }
        i
    }
    for i in 0..n {
        for j in i + 1..n {
            let (Some(a), Some(b)) = (
                interval(&report.components[i]),
                interval(&report.components[j]),
            ) else {
                continue;
            };
            if a[0] <= b[1] && b[0] <= a[1] {
                let (ri, rj) = (find(&mut parent, i), find(&mut parent, j));
                if ri != rj {
                    parent[ri] = rj;
                }
            }
        }
    }
    let mut merged: Vec<Option<CurvePlaneComponent>> = (0..n).map(|_| None).collect();
    let mut order: Vec<usize> = Vec::new();
    for (i, component) in report.components.iter().take(n).enumerate() {
        let Some(iv) = interval(component) else {
            continue;
        };
        let root = find(&mut parent, i);
        if merged[root].is_none() {
            order.push(root);
        }
        let CurvePlaneComponent::Point(member) = component else {
            continue;
        };
        match &mut merged[root] {
            None => merged[root] = Some(CurvePlaneComponent::Point(member.clone())),
            Some(CurvePlaneComponent::Point(point)) => {
                if member.plane_residual < point.plane_residual {
                    point.parameter = member.parameter;
                    point.point = member.point;
                    point.plane_residual = member.plane_residual;
                }
                if member.contact == Contact::Boundary {
                    point.contact = Contact::Boundary;
                }
                point.parameter_interval[0] = point.parameter_interval[0].min(iv[0]);
                point.parameter_interval[1] = point.parameter_interval[1].max(iv[1]);
            }
            _ => unreachable!(),
        }
    }
    let mut components = Vec::new();
    let mut index = 0;
    for root in order {
        while index < root {
            if interval(&report.components[index]).is_none() {
                components.push(report.components[index].clone());
            }
            index += 1;
        }
        if let Some(c) = merged[root].take() {
            components.push(c);
        }
        index = root + 1;
    }
    while index < n {
        if interval(&report.components[index]).is_none() {
            components.push(report.components[index].clone());
        }
        index += 1;
    }
    report.components = components;
}
