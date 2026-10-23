use super::*;
#[derive(Clone, Debug)]
pub enum SurfaceSurfaceComponent {
    /// Implicitly closed convex boundary, ordered counterclockwise in first UV.
    Overlap {
        first_boundary: Vec<[f64; 2]>,
        second_boundary: Vec<[f64; 2]>,
        points: Vec<[f64; 3]>,
        max_sample_residual: f64,
    },
    Point {
        first: SurfacePoint,
        second: SurfacePoint,
        residual: f64,
    },
    /// Both retained traces use the same fraction in [0,1].
    Curve {
        first: Box<SurfaceTrace>,
        second: Box<SurfaceTrace>,
        max_sample_residual: f64,
    },
}

/// Finite affine-patch intersection with retained UV correspondence. General
/// curved pairs remain explicitly unresolved; coplanar affine overlap retains paired UV polygons.
pub fn surface_surface(
    first: &Surface,
    second: &Surface,
    options: Options,
) -> Result<Report<SurfaceSurfaceComponent>> {
    first.validate()?;
    second.validate()?;
    let options = options.validate()?;
    let mut report = Report::default();
    let domain = [surface_domain(first), surface_domain(second)].concat();
    let (Some(a), Some(b)) = (affine_frame(first), affine_frame(second)) else {
        // G6 narrow bicubic path (Complete-empty or Incomplete); never topology change.
        match crate::nurbs_ss::narrow_transverse_bicubic(first, second, options) {
            Ok(g6) => {
                let mut report = Report {
                    coverage: g6.coverage,
                    boxes_visited: g6.boxes_visited,
                    bernstein_excluded: g6.bernstein_excluded,
                    ..Report::default()
                };
                for pending in g6.unresolved {
                    report.unresolved(pending.parameter_box, pending.reason);
                }
                // Curve Complete stays inside nurbs_ss_g6; surface_surface only
                // forwards empty Complete / typed Incomplete without fake UV traces.
                return Ok(report);
            }
            Err(_) => {
                report.unresolved(domain, UnresolvedReason::UnsupportedSurface);
                return Ok(report);
            }
        }
    };
    let normal = cross(b.u, b.v);
    let plane_b = Plane {
        normal,
        offset: dot(normal, b.origin),
    }
    .normalized()?;
    let normal = cross(a.u, a.v);
    let plane_a = Plane {
        normal,
        offset: dot(normal, a.origin),
    }
    .normalized()?;
    let source = surface_plane(first, plane_b, options)?;
    report.boxes_visited = source.boxes_visited;
    report.bernstein_excluded = source.bernstein_excluded;
    for pending in source.unresolved {
        report.unresolved(
            [pending.parameter_box, surface_domain(second).to_vec()].concat(),
            pending.reason,
        );
    }
    let uu = dot(b.u, b.u);
    let uv = dot(b.u, b.v);
    let vv = dot(b.v, b.v);
    let determinant = uu * vv - uv * uv;
    if !determinant.is_finite() || determinant <= 1e-24 * uu * vv {
        report.unresolved(domain, UnresolvedReason::UnsupportedSurface);
        return Ok(report);
    }
    let project = |p| {
        let d = sub(p, b.origin);
        let x = dot(d, b.u);
        let y = dot(d, b.v);
        [
            b.domain[0] + (x * vv - y * uv) / determinant * (b.domain[1] - b.domain[0]),
            b.domain[2] + (y * uu - x * uv) / determinant * (b.domain[3] - b.domain[2]),
        ]
    };
    for component in source.components {
        if report.boxes_visited >= options.max_boxes {
            report.unresolved(domain.clone(), UnresolvedReason::BudgetExceeded);
            continue;
        }
        report.boxes_visited += 1;
        let (start, end) = match component {
            SurfacePlaneComponent::Curve {
                trace: SurfaceTrace::Line { start, end, .. },
                ..
            } => (start, end),
            SurfacePlaneComponent::Point(point) => (point.uv, point.uv),
            SurfacePlaneComponent::Overlap { .. } => {
                let corners = [
                    [a.domain[0], a.domain[2]],
                    [a.domain[1], a.domain[2]],
                    [a.domain[1], a.domain[3]],
                    [a.domain[0], a.domain[3]],
                ];
                let mut polygon = corners
                    .into_iter()
                    .map(|uv| first.evaluate(uv[0], uv[1]).map(|p| (uv, project(p.point))))
                    .collect::<Result<Vec<_>>>()?;
                let mut failed = false;
                for (axis, boundary, greater) in [
                    (0, b.domain[0], true),
                    (0, b.domain[1], false),
                    (1, b.domain[2], true),
                    (1, b.domain[3], false),
                ] {
                    if polygon.is_empty() {
                        break;
                    }
                    if report.boxes_visited >= options.max_boxes {
                        failed = true;
                        break;
                    }
                    report.boxes_visited += 1;
                    let inside = |p: ([f64; 2], [f64; 2])| {
                        if greater {
                            p.1[axis] >= boundary
                        } else {
                            p.1[axis] <= boundary
                        }
                    };
                    let mut clipped = Vec::new();
                    for i in 0..polygon.len() {
                        let previous = polygon[(i + polygon.len() - 1) % polygon.len()];
                        let current = polygon[i];
                        let before = inside(previous);
                        let after = inside(current);
                        if before != after {
                            let t = (boundary - previous.1[axis])
                                / (current.1[axis] - previous.1[axis]);
                            if !t.is_finite() || !(0. ..=1.).contains(&t) {
                                failed = true;
                                break;
                            }
                            let first_uv = std::array::from_fn(|j| {
                                previous.0[j] + t * (current.0[j] - previous.0[j])
                            });
                            let mut second_uv = std::array::from_fn(|j| {
                                previous.1[j] + t * (current.1[j] - previous.1[j])
                            });
                            second_uv[axis] = boundary;
                            clipped.push((first_uv, second_uv));
                        }
                        if after {
                            clipped.push(current);
                        }
                    }
                    if failed {
                        break;
                    }
                    clipped.dedup_by(|a, b| a.0 == b.0);
                    if clipped.len() > 1 && clipped.first().unwrap().0 == clipped.last().unwrap().0
                    {
                        clipped.pop();
                    }
                    polygon = clipped;
                }
                if failed {
                    report.unresolved(
                        domain.clone(),
                        if report.boxes_visited >= options.max_boxes {
                            UnresolvedReason::BudgetExceeded
                        } else {
                            UnresolvedReason::NearCoincidence
                        },
                    );
                    continue;
                }
                if polygon.is_empty() {
                    continue;
                }
                if polygon.len() >= 3 {
                    let origin = polygon[0].0;
                    let area = polygon
                        .windows(2)
                        .skip(1)
                        .map(|p| {
                            let x = [p[0].0[0] - origin[0], p[0].0[1] - origin[1]];
                            let y = [p[1].0[0] - origin[0], p[1].0[1] - origin[1]];
                            x[0] * y[1] - x[1] * y[0]
                        })
                        .sum::<f64>();
                    if !area.is_finite() || area <= 0. {
                        report.unresolved(domain.clone(), UnresolvedReason::NearCoincidence);
                        continue;
                    }
                    let mut points = Vec::new();
                    let mut residual = 0_f64;
                    for (u, v) in &polygon {
                        let valid = |p: [f64; 2], d: [f64; 4]| {
                            (0..2).all(|i| {
                                p[i].is_finite() && p[i] >= d[2 * i] && p[i] <= d[2 * i + 1]
                            })
                        };
                        if !valid(*u, a.domain) || !valid(*v, b.domain) {
                            residual = f64::INFINITY;
                            break;
                        }
                        let p = first.evaluate(u[0], u[1])?.point;
                        let q = second.evaluate(v[0], v[1])?.point;
                        let d = sub(p, q);
                        residual = residual.max(d[0].hypot(d[1]).hypot(d[2]));
                        points.push(p);
                    }
                    if residual > options.distance_tolerance {
                        report.unresolved(domain.clone(), UnresolvedReason::NearCoincidence);
                        continue;
                    }
                    report.components.push(SurfaceSurfaceComponent::Overlap {
                        first_boundary: polygon.iter().map(|p| p.0).collect(),
                        second_boundary: polygon.iter().map(|p| p.1).collect(),
                        points,
                        max_sample_residual: residual,
                    });
                    continue;
                }
                (polygon[0].0, polygon.last().unwrap().0)
            }
            _ => {
                report.unresolved(domain.clone(), UnresolvedReason::UnsupportedSurface);
                continue;
            }
        };
        let p = first.evaluate(start[0], start[1])?.point;
        let q = first.evaluate(end[0], end[1])?.point;
        let u = project(p);
        let v = project(q);
        let mut low = 0_f64;
        let mut high = 1_f64;
        let mut invalid_numeric = false;
        for axis in 0..2 {
            let delta = v[axis] - u[axis];
            let min = b.domain[2 * axis];
            let max = b.domain[2 * axis + 1];
            if !delta.is_finite() || !u[axis].is_finite() {
                invalid_numeric = true;
                break;
            }
            if delta == 0. {
                if u[axis] < min || u[axis] > max {
                    high = -1.;
                    break;
                }
            } else {
                let x = (min - u[axis]) / delta;
                let y = (max - u[axis]) / delta;
                if !x.is_finite() || !y.is_finite() {
                    invalid_numeric = true;
                    break;
                }
                low = low.max(x.min(y));
                high = high.min(x.max(y));
            }
        }
        if invalid_numeric {
            report.unresolved(domain.clone(), UnresolvedReason::NearCoincidence);
            continue;
        }
        if low > high {
            continue;
        }
        let interpolate =
            |x: [f64; 2], y: [f64; 2], t: f64| std::array::from_fn(|i| x[i] + t * (y[i] - x[i]));
        let first_trace = SurfaceTrace::Line {
            surface: first.clone(),
            plane: plane_b,
            start: interpolate(start, end, low),
            end: interpolate(start, end, high),
        };
        let second_trace = SurfaceTrace::Line {
            surface: second.clone(),
            plane: plane_a,
            start: interpolate(u, v, low),
            end: interpolate(u, v, high),
        };
        let mut max_residual = 0_f64;
        let mut sample = None;
        for i in 0..=8 {
            let (Ok(a), Ok(b)) = (
                first_trace.evaluate(i as f64 / 8.),
                second_trace.evaluate(i as f64 / 8.),
            ) else {
                max_residual = f64::INFINITY;
                break;
            };
            let d = sub(a.point, b.point);
            let residual = d[0].hypot(d[1]).hypot(d[2]);
            max_residual = max_residual
                .max(residual)
                .max(a.plane_residual)
                .max(b.plane_residual);
            sample = Some((a, b, residual));
        }
        if max_residual > options.distance_tolerance {
            report.unresolved(domain.clone(), UnresolvedReason::NearCoincidence);
            continue;
        }
        if low == high || start == end {
            let (first, second, residual) = sample.unwrap();
            report.components.push(SurfaceSurfaceComponent::Point {
                first,
                second,
                residual,
            });
        } else {
            report.components.push(SurfaceSurfaceComponent::Curve {
                first: Box::new(first_trace),
                second: Box::new(second_trace),
                max_sample_residual: max_residual,
            });
        }
    }
    if report.unresolved.is_empty() {
        // Affine×affine pairs above are algebraically partitioned; promote only
        // when every retained contact survived residual checks with no bands left.
        report.coverage = Coverage::Complete;
    }
    Ok(report)
}

pub(crate) fn invalid(message: &str) -> Error {
    Error::new("BREP_INTERSECTION_INVALID_INPUT", message)
}
