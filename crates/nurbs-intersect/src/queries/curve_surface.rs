use super::*;
#[derive(Clone, Debug)]
pub enum CurveSurfaceComponent {
    Point {
        curve: CurvePoint,
        uv: [f64; 2],
        surface_residual: f64,
    },
    Overlap {
        curve_interval: [f64; 2],
    },
}

/// Curve/surface entry point currently admits affine rectangular planar patches.
/// Curved support surfaces return an explicit unsupported parameter region.
/// Coplanar curves are admitted only if their entire control hull lies inside
/// the rectangle. Partial coincident clipping is explicitly unresolved.
pub fn curve_surface(
    curve: &Curve,
    surface: &Surface,
    options: Options,
) -> Result<Report<CurveSurfaceComponent>> {
    curve.validate()?;
    if curve.control_points[0].len() != 3 {
        return Err(invalid("Curve/surface requires a 3D curve"));
    }
    surface.validate()?;
    let options = options.validate()?;
    let Some(frame) = affine_frame(surface) else {
        let mut report = Report::default();
        report.unresolved(curve.domain(), UnresolvedReason::UnsupportedSurface);
        return Ok(report);
    };
    let normal = cross(frame.u, frame.v);
    let plane = Plane {
        normal,
        offset: dot(normal, frame.origin),
    };
    let source = curve_plane(curve, plane, options)?;
    let mut report = Report {
        components: vec![],
        unresolved: source.unresolved,
        boxes_visited: source.boxes_visited,
        bernstein_excluded: source.bernstein_excluded,
        coverage: source.coverage,
    };
    let a = dot(frame.u, frame.u);
    let b = dot(frame.u, frame.v);
    let c = dot(frame.v, frame.v);
    let determinant = a * c - b * b;
    if determinant <= 1e-24 * a * c {
        report.unresolved(curve.domain(), UnresolvedReason::UnsupportedSurface);
        return Ok(report);
    }
    let project = |point: [f64; 3]| {
        let d = sub(point, frame.origin);
        let x = dot(d, frame.u);
        let y = dot(d, frame.v);
        let u = (x * c - y * b) / determinant;
        let v = (y * a - x * b) / determinant;
        [
            frame.domain[0] + u * (frame.domain[1] - frame.domain[0]),
            frame.domain[2] + v * (frame.domain[3] - frame.domain[2]),
        ]
    };
    let inside = |uv: [f64; 2]| {
        uv[0] >= frame.domain[0]
            && uv[0] <= frame.domain[1]
            && uv[1] >= frame.domain[2]
            && uv[1] <= frame.domain[3]
    };
    for component in source.components {
        match component {
            CurvePlaneComponent::Point(point) => {
                let uv = project(point.point);
                if point.parameter_interval[0] < point.parameter_interval[1] {
                    let interval_curve =
                        curve.trim(point.parameter_interval[0], point.parameter_interval[1])?;
                    let hull: Vec<_> = interval_curve
                        .control_points
                        .iter()
                        .map(|p| project(point3(p)))
                        .collect();
                    let outside = (0..2).any(|axis| {
                        hull.iter().all(|p| p[axis] < frame.domain[2 * axis])
                            || hull.iter().all(|p| p[axis] > frame.domain[2 * axis + 1])
                    });
                    if outside {
                        continue;
                    }
                    if !hull.iter().all(|p| inside(*p)) {
                        report.unresolved(
                            point.parameter_interval,
                            UnresolvedReason::BoundaryCrossing,
                        );
                        continue;
                    }
                }
                if inside(uv) {
                    let on_surface = surface.evaluate(uv[0], uv[1])?.point;
                    let delta = sub(on_surface, point.point);
                    let residual = dot(delta, delta).sqrt();
                    if residual <= options.distance_tolerance {
                        report.components.push(CurveSurfaceComponent::Point {
                            curve: point,
                            uv,
                            surface_residual: residual,
                        });
                    } else {
                        report.unresolved(
                            point.parameter_interval,
                            UnresolvedReason::NearCoincidence,
                        );
                    }
                }
            }
            CurvePlaneComponent::Overlap {
                parameter_interval, ..
            } => {
                let piece = curve.trim(parameter_interval[0], parameter_interval[1])?;
                if piece
                    .control_points
                    .iter()
                    .all(|p| inside(project(point3(p))))
                {
                    report.components.push(CurveSurfaceComponent::Overlap {
                        curve_interval: parameter_interval,
                    });
                } else {
                    let dual_u =
                        std::array::from_fn(|i| (c * frame.u[i] - b * frame.v[i]) / determinant);
                    let dual_v =
                        std::array::from_fn(|i| (a * frame.v[i] - b * frame.u[i]) / determinant);
                    let mut cuts = vec![parameter_interval[0], parameter_interval[1]];
                    let mut bands = Vec::new();
                    let mut endpoints = Vec::new();
                    for normal in [dual_u, dual_v] {
                        for boundary in [0., 1.] {
                            if report.boxes_visited >= options.max_boxes {
                                bands.push(parameter_interval);
                                report.unresolved(
                                    parameter_interval,
                                    UnresolvedReason::BudgetExceeded,
                                );
                                continue;
                            }
                            let roots = curve_plane(
                                &piece,
                                Plane {
                                    normal,
                                    offset: dot(normal, frame.origin) + boundary,
                                },
                                Options {
                                    max_boxes: options.max_boxes - report.boxes_visited,
                                    ..options
                                },
                            )?;
                            report.boxes_visited += roots.boxes_visited;
                            report.bernstein_excluded += roots.bernstein_excluded;
                            for unresolved in roots.unresolved {
                                let band =
                                    [unresolved.parameter_box[0], unresolved.parameter_box[1]];
                                cuts.extend(band);
                                bands.push(band);
                                report.unresolved(band, unresolved.reason);
                            }
                            for root in roots.components {
                                match root {
                                    CurvePlaneComponent::Point(point) => {
                                        cuts.extend(point.parameter_interval);
                                        if point.parameter_interval[0]
                                            == point.parameter_interval[1]
                                        {
                                            endpoints.push(point);
                                        } else {
                                            bands.push(point.parameter_interval);
                                            report.unresolved(
                                                point.parameter_interval,
                                                UnresolvedReason::BoundaryCrossing,
                                            );
                                        }
                                    }
                                    // A curve along a rectangle edge is admissible;
                                    // the other three boundary constraints still apply.
                                    CurvePlaneComponent::Overlap {
                                        parameter_interval, ..
                                    } => cuts.extend(parameter_interval),
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
                        let mut hull = cell_curve
                            .control_points
                            .iter()
                            .map(|p| project(point3(p)))
                            .collect::<Vec<_>>();
                        hull[0] = project(point3(&curve.evaluate(cell[0])?.point));
                        let last = hull.len() - 1;
                        hull[last] = project(point3(&curve.evaluate(cell[1])?.point));
                        if hull.iter().all(|p| inside(*p)) {
                            report.components.push(CurveSurfaceComponent::Overlap {
                                curve_interval: [cell[0], cell[1]],
                            });
                        } else if !(0..2).any(|axis| {
                            (hull.iter().all(|p| p[axis] <= frame.domain[2 * axis])
                                && hull.iter().any(|p| p[axis] < frame.domain[2 * axis]))
                                || (hull.iter().all(|p| p[axis] >= frame.domain[2 * axis + 1])
                                    && hull.iter().any(|p| p[axis] > frame.domain[2 * axis + 1]))
                        }) {
                            report.unresolved(cell.to_vec(), UnresolvedReason::CoincidentTrim);
                        }
                    }
                    for point in endpoints {
                        if report.components.iter().any(|c| match c {
                            CurveSurfaceComponent::Point { curve, .. } => {
                                curve.parameter == point.parameter
                            }
                            CurveSurfaceComponent::Overlap { curve_interval } => {
                                point.parameter >= curve_interval[0]
                                    && point.parameter <= curve_interval[1]
                            }
                        }) {
                            continue;
                        }
                        let uv = project(point.point);
                        if !inside(uv) {
                            continue;
                        }
                        let actual = surface.evaluate(uv[0], uv[1])?.point;
                        let d = sub(actual, point.point);
                        let residual = d[0].hypot(d[1]).hypot(d[2]);
                        if residual <= options.distance_tolerance {
                            report.components.push(CurveSurfaceComponent::Point {
                                curve: point,
                                uv,
                                surface_residual: residual,
                            });
                        } else {
                            report.unresolved(
                                point.parameter_interval,
                                UnresolvedReason::NearCoincidence,
                            );
                        }
                    }
                }
            }
        }
    }
    Ok(report)
}
