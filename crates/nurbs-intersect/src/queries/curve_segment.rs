use super::*;
/// Numerical curve/finite-segment correspondence. Overlaps retain the original
/// curve interval; degree-one coincidences are clipped in source parameters.
/// Higher-degree clipping isolates segment-boundary roots and retains unresolved bands.
#[derive(Clone, Debug)]
pub enum CurveSegmentComponent {
    Point {
        curve: CurvePoint,
        segment_parameter: f64,
        line_residual: f64,
    },
    Overlap {
        curve_interval: [f64; 2],
    },
}

/// Intersect a retained NURBS curve with a finite 3D segment using two supporting
/// planes. Both plane queries share max_boxes; results remain uncertified and
/// cannot authorize topology changes. No sampled polyline defines the curve.
pub fn curve_segment(
    curve: &Curve,
    start: [f64; 3],
    end: [f64; 3],
    options: Options,
) -> Result<Report<CurveSegmentComponent>> {
    curve.validate()?;
    let options = options.validate()?;
    if curve.control_points[0].len() != 3 || !start.iter().chain(&end).all(|x| x.is_finite()) {
        return Err(invalid("Curve/segment requires finite 3D inputs"));
    }
    let delta = sub(end, start);
    let length = delta[0].hypot(delta[1]).hypot(delta[2]);
    if !length.is_finite() || length == 0. {
        return Err(invalid("Segment must have finite nonzero length"));
    }
    let direction = delta.map(|x| x / length);
    let axis = (0..3)
        .min_by(|a, b| direction[*a].abs().total_cmp(&direction[*b].abs()))
        .unwrap();
    let mut basis = [0.; 3];
    basis[axis] = 1.;
    let n = cross(direction, basis);
    let first_plane = Plane {
        normal: n,
        offset: dot(n, start),
    }
    .normalized()?;
    let n = cross(direction, first_plane.normal);
    let second_plane = Plane {
        normal: n,
        offset: dot(n, start),
    }
    .normalized()?;
    let source = curve_plane(curve, first_plane, options)?;
    let mut report = Report {
        components: vec![],
        unresolved: source.unresolved,
        boxes_visited: source.boxes_visited,
        bernstein_excluded: source.bernstein_excluded,
        coverage: source.coverage,
    };
    let mut candidates = Vec::new();
    for component in source.components {
        match component {
            CurvePlaneComponent::Point(point) => candidates.push(CurvePlaneComponent::Point(point)),
            CurvePlaneComponent::Overlap {
                parameter_interval, ..
            } => {
                if report.boxes_visited >= options.max_boxes {
                    report.unresolved(parameter_interval, UnresolvedReason::BudgetExceeded);
                    continue;
                }
                let piece = curve.trim(parameter_interval[0], parameter_interval[1])?;
                let other = curve_plane(
                    &piece,
                    second_plane,
                    Options {
                        max_boxes: options.max_boxes - report.boxes_visited,
                        ..options
                    },
                )?;
                report.boxes_visited += other.boxes_visited;
                report.bernstein_excluded += other.bernstein_excluded;
                if other.coverage == Coverage::Incomplete {
                    report.coverage = Coverage::Incomplete;
                }
                report.unresolved.extend(other.unresolved);
                candidates.extend(other.components);
            }
        }
    }
    let project = |p| dot(sub(p, start), direction) / length;
    for candidate in candidates {
        let interval = match &candidate {
            CurvePlaneComponent::Point(p) => p.parameter_interval,
            CurvePlaneComponent::Overlap {
                parameter_interval, ..
            } => *parameter_interval,
        };
        let hull = if interval[0] < interval[1] {
            curve
                .trim(interval[0], interval[1])?
                .control_points
                .iter()
                .map(|p| point3(p))
                .collect::<Vec<_>>()
        } else {
            vec![point3(&curve.evaluate(interval[0])?.point)]
        };
        let parameters = hull.iter().map(|p| project(*p)).collect::<Vec<_>>();
        if parameters.iter().all(|t| *t < 0.) || parameters.iter().all(|t| *t > 1.) {
            continue;
        }
        let distances = hull
            .iter()
            .map(|p| second_plane.distance(*p))
            .collect::<Vec<_>>();
        if distances.iter().all(|d| *d > options.distance_tolerance)
            || distances.iter().all(|d| *d < -options.distance_tolerance)
        {
            continue;
        }
        if !parameters
            .iter()
            .all(|t| t.is_finite() && (0. ..=1.).contains(t))
        {
            if matches!(&candidate, CurvePlaneComponent::Overlap { .. })
                && curve.degree == 1
                && parameters.len() == 2
                && parameters.iter().all(|t| t.is_finite())
            {
                let piece = curve.trim(interval[0], interval[1])?;
                let [a, b] = [parameters[0], parameters[1]];
                let low = a.min(b).max(0.);
                let high = a.max(b).min(1.);
                let scale = piece.weights[0].max(piece.weights[1]);
                let [wa, wb] = [piece.weights[0] / scale, piece.weights[1] / scale];
                let parameter = |q: f64| {
                    let r = if q == a {
                        0.
                    } else if q == b {
                        1.
                    } else {
                        let numerator = wa * (q - a);
                        numerator / (wb * (b - q) + numerator)
                    };
                    interval[0] + r * (interval[1] - interval[0])
                };
                let [x, y] = [parameter(low), parameter(high)];
                if !x.is_finite()
                    || !y.is_finite()
                    || x < interval[0]
                    || x > interval[1]
                    || y < interval[0]
                    || y > interval[1]
                {
                    report.unresolved(interval, UnresolvedReason::NearCoincidence);
                    continue;
                }
                let clipped = [x.min(y), x.max(y)];
                if clipped[0] == clipped[1] && low != high {
                    report.unresolved(interval, UnresolvedReason::NearCoincidence);
                    continue;
                }
                if clipped[0] == clipped[1] {
                    let point = point3(&curve.evaluate(clipped[0])?.point);
                    let t = low;
                    let on_line = std::array::from_fn(|i| start[i] + t * delta[i]);
                    let d = sub(point, on_line);
                    let residual = d[0].hypot(d[1]).hypot(d[2]);
                    if residual > options.distance_tolerance {
                        report.unresolved(interval, UnresolvedReason::NearCoincidence);
                        continue;
                    }
                    if report.components.iter().any(|c| {
                        matches!(c,CurveSegmentComponent::Point {curve,..}
                        if curve.parameter==clipped[0])
                    }) {
                        continue;
                    }
                    report.components.push(CurveSegmentComponent::Point {
                        curve: CurvePoint {
                            parameter: clipped[0],
                            parameter_interval: clipped,
                            point,
                            plane_residual: first_plane.distance(point).abs(),
                            contact: Contact::Boundary,
                        },
                        segment_parameter: t,
                        line_residual: residual,
                    });
                } else {
                    report.components.push(CurveSegmentComponent::Overlap {
                        curve_interval: clipped,
                    });
                }
                continue;
            }
            if matches!(&candidate, CurvePlaneComponent::Overlap { .. }) {
                let piece = curve.trim(interval[0], interval[1])?;
                let mut cuts = vec![interval[0], interval[1]];
                let mut bands = Vec::new();
                let mut endpoints = Vec::new();
                for (position, segment_parameter) in [(start, 0.), (end, 1.)] {
                    if report.boxes_visited >= options.max_boxes {
                        bands.push(interval);
                        report.unresolved(interval, UnresolvedReason::BudgetExceeded);
                        break;
                    }
                    let boundary = curve_plane(
                        &piece,
                        Plane {
                            normal: direction,
                            offset: dot(direction, position),
                        },
                        Options {
                            max_boxes: options.max_boxes - report.boxes_visited,
                            ..options
                        },
                    )?;
                    report.boxes_visited += boundary.boxes_visited;
                    report.bernstein_excluded += boundary.bernstein_excluded;
                    for unresolved in boundary.unresolved {
                        let band = [unresolved.parameter_box[0], unresolved.parameter_box[1]];
                        cuts.extend(band);
                        bands.push(band);
                        report.unresolved(band, unresolved.reason);
                    }
                    for component in boundary.components {
                        match component {
                            CurvePlaneComponent::Point(point) => {
                                cuts.extend(point.parameter_interval);
                                if point.parameter_interval[0] == point.parameter_interval[1] {
                                    endpoints.push((point, segment_parameter));
                                } else {
                                    bands.push(point.parameter_interval);
                                    report.unresolved(
                                        point.parameter_interval,
                                        UnresolvedReason::BoundaryCrossing,
                                    );
                                }
                            }
                            CurvePlaneComponent::Overlap {
                                parameter_interval, ..
                            } => {
                                cuts.extend(parameter_interval);
                                bands.push(parameter_interval);
                                report.unresolved(
                                    parameter_interval,
                                    UnresolvedReason::CoincidentTrim,
                                );
                            }
                        }
                    }
                }
                cuts.sort_by(f64::total_cmp);
                cuts.dedup();
                for cell in cuts.windows(2) {
                    if cell[0] == cell[1]
                        || bands.iter().any(|b| cell[0] >= b[0] && cell[1] <= b[1])
                    {
                        continue;
                    }
                    if report.boxes_visited >= options.max_boxes {
                        report.unresolved(cell.to_vec(), UnresolvedReason::BudgetExceeded);
                        continue;
                    }
                    report.boxes_visited += 1;
                    let cell_curve = curve.trim(cell[0], cell[1])?;
                    let mut projection = cell_curve
                        .control_points
                        .iter()
                        .map(|p| project(point3(p)))
                        .collect::<Vec<_>>();
                    // Knot insertion can round the trim's endpoint controls.
                    // Query the authoritative source at the retained cut parameters.
                    projection[0] = project(point3(&curve.evaluate(cell[0])?.point));
                    let last = projection.len() - 1;
                    projection[last] = project(point3(&curve.evaluate(cell[1])?.point));

                    if projection.iter().all(|t| *t <= 0.) || projection.iter().all(|t| *t >= 1.) {
                        continue;
                    }
                    if projection
                        .iter()
                        .all(|t| t.is_finite() && (0. ..=1.).contains(t))
                    {
                        report.components.push(CurveSegmentComponent::Overlap {
                            curve_interval: [cell[0], cell[1]],
                        });
                    } else {
                        report.unresolved(cell.to_vec(), UnresolvedReason::CoincidentTrim);
                    }
                }
                for (point, t) in endpoints {
                    let covered = report.components.iter().any(|c| match c {
                        CurveSegmentComponent::Point { curve, .. } => {
                            curve.parameter == point.parameter
                        }
                        CurveSegmentComponent::Overlap { curve_interval } => {
                            point.parameter >= curve_interval[0]
                                && point.parameter <= curve_interval[1]
                        }
                    });
                    if covered {
                        continue;
                    }
                    let on_line = std::array::from_fn(|i| start[i] + t * delta[i]);
                    let d = sub(point.point, on_line);
                    let residual = d[0].hypot(d[1]).hypot(d[2]);
                    if residual <= options.distance_tolerance {
                        report.components.push(CurveSegmentComponent::Point {
                            curve: point,
                            segment_parameter: t,
                            line_residual: residual,
                        });
                    } else {
                        report.unresolved(
                            point.parameter_interval,
                            UnresolvedReason::NearCoincidence,
                        );
                    }
                }
                continue;
            }
            report.unresolved(
                interval,
                match candidate {
                    CurvePlaneComponent::Overlap { .. } => UnresolvedReason::CoincidentTrim,
                    _ => UnresolvedReason::BoundaryCrossing,
                },
            );
            continue;
        }
        match candidate {
            CurvePlaneComponent::Point(point) => {
                let t = project(point.point);
                let on_line = std::array::from_fn(|i| start[i] + t * delta[i]);
                let d = sub(point.point, on_line);
                let residual = d[0].hypot(d[1]).hypot(d[2]);
                if residual <= options.distance_tolerance {
                    if !report.components.iter().any(|c|matches!(c,CurveSegmentComponent::Point {curve,..} if curve.parameter==point.parameter)) {
                        report.components.push(CurveSegmentComponent::Point {curve:point,segment_parameter:t,line_residual:residual});
                    }
                } else {
                    report.unresolved(interval, UnresolvedReason::NearCoincidence);
                }
            }
            CurvePlaneComponent::Overlap { .. } => {
                report.components.push(CurveSegmentComponent::Overlap {
                    curve_interval: interval,
                })
            }
        }
    }
    report.components.sort_by(|a, b| {
        let parameter = |c: &CurveSegmentComponent| match c {
            CurveSegmentComponent::Point { curve, .. } => curve.parameter,
            CurveSegmentComponent::Overlap { curve_interval } => curve_interval[0],
        };
        parameter(a).total_cmp(&parameter(b))
    });
    Ok(report)
}
