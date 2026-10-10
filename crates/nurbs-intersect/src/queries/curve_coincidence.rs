use super::*;
enum LineInversion {
    Root(f64),
    Ambiguous,
    BudgetExhausted,
}
/// Invert the rational line support of a collinear span at one line offset
/// through the shared-budget curve/plane isolator. Multiple or uncertain
/// roots keep the coincidence clip ambiguous rather than guessed.
fn invert_line_parameter(
    piece: &Curve,
    direction: [f64; 3],
    offset: f64,
    options: Options,
    report: &mut Report<CurveCurveComponent>,
) -> Result<LineInversion> {
    if report.boxes_visited >= options.max_boxes {
        return Ok(LineInversion::BudgetExhausted);
    }
    let roots = curve_plane(
        piece,
        Plane {
            normal: direction,
            offset,
        },
        Options {
            max_boxes: options.max_boxes - report.boxes_visited,
            ..options
        },
    )?;
    report.boxes_visited += roots.boxes_visited;
    report.bernstein_excluded += roots.bernstein_excluded;
    if !roots.unresolved.is_empty() {
        return Ok(LineInversion::Ambiguous);
    }
    let mut found = None;
    for component in roots.components {
        match component {
            CurvePlaneComponent::Point(point) => {
                if found.is_some() {
                    return Ok(LineInversion::Ambiguous);
                }
                found = Some(point.parameter);
            }
            CurvePlaneComponent::Overlap { .. } => return Ok(LineInversion::Ambiguous),
        }
    }
    Ok(found.map_or(LineInversion::Ambiguous, LineInversion::Root))
}
/// Coincidence admission for one source span pair. Collinear pairs clip the
/// shared line interval by inverting both rational parameterizations; curved
/// pairs coincide only under proportional homogeneous polygons after degree
/// elevation. Anything else returns false so subdivision can isolate points.
/// Returns true when the pair is fully handled (component, empty, or an
/// explicit unresolved region) and must not enter point subdivision.
pub(crate) fn curve_coincidence(
    piece_a: &Curve,
    piece_b: &Curve,
    ta: [f64; 2],
    tb: [f64; 2],
    options: Options,
    report: &mut Report<CurveCurveComponent>,
) -> Result<bool> {
    let box4 = || vec![ta[0], ta[1], tb[0], tb[1]];
    let a0 = point3(piece_a.control_points.first().unwrap());
    let a1 = point3(piece_a.control_points.last().unwrap());
    let d = sub(a1, a0);
    let length = d[0].hypot(d[1]).hypot(d[2]);
    if !length.is_finite() || length <= 0. {
        // Degenerate span: no support line; leave it to point isolation.
        return Ok(false);
    }
    let direction = d.map(|x| x / length);
    let off_line = |c: &[f64]| {
        let v = cross(direction, sub(point3(c), a0));
        v[0].hypot(v[1]).hypot(v[2])
    };
    let residual_a = piece_a
        .control_points
        .iter()
        .map(|c| off_line(c))
        .fold(0., f64::max);
    let residual_b = piece_b
        .control_points
        .iter()
        .map(|c| off_line(c))
        .fold(0., f64::max);
    let max_control_residual = residual_a.max(residual_b);
    if max_control_residual <= options.distance_tolerance {
        // Both spans numerically lie on one support line, so their
        // intersection is exactly the shared line interval (possibly empty).
        let projections = |piece: &Curve| {
            piece
                .control_points
                .iter()
                .map(|c| dot(direction, point3(c)))
                .collect::<Vec<_>>()
        };
        let sa = projections(piece_a);
        let sb = projections(piece_b);
        let monotone =
            |s: &[f64]| s.windows(2).all(|w| w[1] >= w[0]) || s.windows(2).all(|w| w[1] <= w[0]);
        if !monotone(&sa) || !monotone(&sb) {
            report.unresolved(box4(), UnresolvedReason::CoincidentTrim);
            return Ok(true);
        }
        let (sa0, sa1) = (*sa.first().unwrap(), *sa.last().unwrap());
        let (sb0, sb1) = (*sb.first().unwrap(), *sb.last().unwrap());
        let lo = sa0.max(sb0.min(sb1));
        let hi = sa1.min(sb0.max(sb1));
        if hi < lo {
            return Ok(true);
        }
        let mut parameters = [0.; 4];
        for (index, (piece, s)) in [(piece_a, lo), (piece_a, hi), (piece_b, lo), (piece_b, hi)]
            .into_iter()
            .enumerate()
        {
            match invert_line_parameter(piece, direction, s, options, report)? {
                LineInversion::Root(parameter) => parameters[index] = parameter,
                LineInversion::Ambiguous => {
                    report.unresolved(box4(), UnresolvedReason::CoincidentTrim);
                    return Ok(true);
                }
                LineInversion::BudgetExhausted => {
                    report.unresolved(box4(), UnresolvedReason::BudgetExceeded);
                    return Ok(true);
                }
            }
        }
        let [ta_lo, ta_hi, tb_lo, tb_hi] = parameters;
        if hi == lo {
            let point_a = point3(&piece_a.evaluate(ta_lo)?.point);
            let point_b = point3(&piece_b.evaluate(tb_lo)?.point);
            let residual = distance(point_a, point_b);
            if residual > options.distance_tolerance {
                report.unresolved(box4(), UnresolvedReason::NearCoincidence);
                return Ok(true);
            }
            admit_curve_corner(piece_a, piece_b, ta_lo, tb_lo, options, report)?;
            return Ok(true);
        }
        if ta_lo == ta_hi || tb_lo == tb_hi {
            report.unresolved(box4(), UnresolvedReason::NearCoincidence);
            return Ok(true);
        }
        let reversed = (ta_lo < ta_hi) != (tb_lo < tb_hi);
        push_curve_overlap(
            [ta_lo.min(ta_hi), ta_lo.max(ta_hi)],
            [tb_lo.min(tb_hi), tb_lo.max(tb_hi)],
            reversed,
            max_control_residual,
            report,
        );
        return Ok(true);
    }
    // Curved spans coincide only with proportional homogeneous polygons after
    // elevation; other partial curved coincidences stay with point isolation
    // and remain explicit unresolved bands where they cannot be separated.
    let degree = piece_a.degree.max(piece_b.degree);
    if degree > 25 {
        return Ok(false);
    }
    let ea = piece_a.elevate(degree)?;
    let eb = piece_b.elevate(degree)?;
    for reversed in [false, true] {
        let lambda = {
            let index = ea
                .weights
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.total_cmp(b.1))
                .map(|(i, _)| i)
                .unwrap();
            let j = if reversed { degree - index } else { index };
            eb.weights[j] / ea.weights[index]
        };
        if !lambda.is_finite() || lambda <= 0. {
            continue;
        }
        let weight_scale = eb.weights.iter().copied().fold(0., f64::max);
        let weight_residual = ea
            .weights
            .iter()
            .enumerate()
            .map(|(i, wa)| {
                let j = if reversed { degree - i } else { i };
                (eb.weights[j] - lambda * wa).abs()
            })
            .fold(0., f64::max)
            / weight_scale;
        let control_residual = ea
            .control_points
            .iter()
            .enumerate()
            .map(|(i, pa)| {
                let j = if reversed { degree - i } else { i };
                distance(point3(pa), point3(&eb.control_points[j]))
            })
            .fold(0., f64::max);
        if weight_residual <= 1e-9 && control_residual <= options.distance_tolerance {
            push_curve_overlap(ta, tb, reversed, control_residual, report);
            return Ok(true);
        }
    }
    Ok(false)
}
/// Push an overlap and drop corner events covered by both intervals. The
/// overlap owns its boundary contacts; containment is by parameter, never
/// by spatial proximity.
fn push_curve_overlap(
    first_interval: [f64; 2],
    second_interval: [f64; 2],
    reversed: bool,
    max_control_residual: f64,
    report: &mut Report<CurveCurveComponent>,
) {
    let covered = |c: &CurveCurveComponent| {
        matches!(c, CurveCurveComponent::Point { first, second, .. }
            if first_interval[0] <= *first && *first <= first_interval[1]
                && second_interval[0] <= *second && *second <= second_interval[1])
    };
    report.components.retain(|c| !covered(c));
    report.components.push(CurveCurveComponent::Overlap {
        first_interval,
        second_interval,
        reversed,
        max_control_residual,
    });
}
