use super::*;

/// Complete stationary candidate isolation on every rational Bézier span.
/// Bernstein sign variation cannot discard a real root; endpoint candidates are
/// always included and winner separation provides the global uniqueness proof.
pub fn project_curve(
    curve: &Curve,
    point: &[f64],
    tolerance: Option<ToleranceContext>,
) -> Result<Value> {
    curve.validate()?;
    check(
        point.len() == curve.control_points[0].len()
            && point.iter().all(|coordinate| coordinate.is_finite()),
        "Projection point must be finite and match curve dimension",
    )?;
    let tolerance = context(tolerance);
    let floor = tolerance.parametric_bounds().floor;
    let mut candidates: Vec<Value> = Vec::new();
    let mut best_upper = f64::INFINITY;
    let mut stationary_continuum = false;
    for segment in curve.decompose()? {
        let c = segment.definition();
        let [a, b] = segment.domain();
        let analytic_linear = if c.degree == 1 {
            let first = &c.control_points[0];
            let last = &c.control_points[1];
            let direction = last
                .iter()
                .zip(first)
                .map(|(x, y)| x - y)
                .collect::<Vec<_>>();
            let denominator = direction.iter().map(|value| value * value).sum::<f64>();
            if denominator > 0. {
                let lambda = point
                    .iter()
                    .zip(first)
                    .zip(&direction)
                    .map(|((x, o), d)| (x - o) * d)
                    .sum::<f64>()
                    / denominator;
                if lambda > 0. && lambda < 1. {
                    Some(
                        lambda * c.weights[0]
                            / (c.weights[1] * (1. - lambda) + lambda * c.weights[0]),
                    )
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };
        let (roots, continuum) = if analytic_linear.is_some() {
            (vec![], false)
        } else {
            isolate_stationary(stationary_coefficients(c, point), floor / (b - a))
        };
        stationary_continuum |= continuum;
        let mut intervals = vec![(0., 0., "endpoint"), (1., 1., "endpoint")];
        intervals.extend(analytic_linear.map(|root| (root, root, "simple_stationary")));
        intervals.extend(roots.iter().map(|root| {
            (
                root.lo,
                root.hi,
                if root.variation == 1 {
                    "simple_stationary"
                } else {
                    "multiple_or_clustered_stationary"
                },
            )
        }));
        for (lo, hi, classification) in intervals {
            let parameter = a + (b - a) * (lo + hi) * 0.5;
            let evaluated = c.evaluate(parameter)?.point;
            let upper = distance(&evaluated, point);
            let restricted = if lo == hi {
                None
            } else {
                let mut piece = c.clone();
                if hi < 1. {
                    piece = piece.split(a + (b - a) * hi)?[0].clone();
                }
                if lo > 0. {
                    let mapped = a + (b - a) * lo;
                    piece = piece.split(mapped)?[1].clone();
                }
                Some(piece)
            };
            let (min, max) = restricted
                .as_ref()
                .map(|value| box_of(&value.control_points))
                .unwrap_or_else(|| (evaluated.clone(), evaluated.clone()));
            let lower = point_box_distance(&min, &max, point);
            best_upper = best_upper.min(upper);
            candidates.push(json!({
                "domain":[a,b],
                "parameterInterval":[next_down(a+(b-a)*lo),next_up(a+(b-a)*hi)],
                "point":evaluated,
                "distanceLower":next_down(lower.max(0.)),
                "distanceUpper":next_up(upper),
                "classification":classification,
                "rootVariation":if classification=="endpoint" {0} else {roots.iter().find(|r|r.lo==lo&&r.hi==hi).map(|r|r.variation).unwrap_or(0)}
            }));
        }
    }
    let mut kept: Vec<Value> = candidates
        .into_iter()
        .filter(|candidate| candidate["distanceLower"].as_f64().unwrap() <= next_up(best_upper))
        .collect();
    kept.sort_by(|a, b| {
        a["distanceUpper"]
            .as_f64()
            .unwrap()
            .total_cmp(&b["distanceUpper"].as_f64().unwrap())
            .then(
                a["parameterInterval"][0]
                    .as_f64()
                    .unwrap()
                    .total_cmp(&b["parameterInterval"][0].as_f64().unwrap()),
            )
    });
    let winner = kept.iter().enumerate().find(|(i, candidate)| {
        kept.iter().enumerate().all(|(j, other)| {
            i == &j
                || candidate["distanceUpper"].as_f64().unwrap()
                    < other["distanceLower"].as_f64().unwrap()
        })
    });
    let status = if stationary_continuum {
        "nonunique"
    } else if winner.is_some() {
        "unique"
    } else if kept.len() > 1 {
        "nonunique_or_unresolved"
    } else {
        "isolated_candidate"
    };
    Ok(json!({
        "version":"nurbs-foundation/3",
        "status":status,
        "globalDistanceUpper":next_up(best_upper),
        "candidates":kept,
        "coverage":{"method":"Bernstein-sign-variation","endpointsIncluded":true,"stationaryContinuum":stationary_continuum,"resourceLimit":MAX_CERTIFICATE_CELLS},
        "uniquenessProof":winner.map(|(index,_)| json!({"winnerIndex":index,"method":"strict-global-distance-interval-separation"})),
        "evidence":tolerance_evidence(&tolerance)
    }))
}

#[derive(Clone)]
pub(super) struct SurfaceBox {
    pub(super) bounds: [f64; 4],
    pub(super) lower: f64,
    pub(super) upper: f64,
    pub(super) point: [f64; 3],
}

/// Certified global surface projection by outward hull branch-and-bound.
/// Every unpruned parameter rectangle is returned; four boundary problems are
/// reduced to the certified curve projector.
pub fn project_surface(
    surface: &Surface,
    point: [f64; 3],
    tolerance: Option<ToleranceContext>,
) -> Result<Value> {
    surface.validate()?;
    check(
        point.iter().all(|value| value.is_finite()),
        "Projection point must be finite",
    )?;
    let tolerance = context(tolerance);
    let nu = surface.control_points.len();
    let nv = surface.control_points[0].len();
    let domain = [
        surface.knots_u[surface.degree_u],
        surface.knots_u[nu],
        surface.knots_v[surface.degree_v],
        surface.knots_v[nv],
    ];
    let make_box = |bounds: [f64; 4]| -> Result<SurfaceBox> {
        let patch = surface.trim(bounds)?;
        let controls: Vec<Vec<f64>> = patch.control_points.iter().flatten().cloned().collect();
        let (min, max) = box_of(&controls);
        let center = [(bounds[0] + bounds[1]) * 0.5, (bounds[2] + bounds[3]) * 0.5];
        let evaluated = surface.evaluate_validated(center[0], center[1])?.point;
        Ok(SurfaceBox {
            bounds,
            lower: next_down(point_box_distance(&min, &max, &point).max(0.)),
            upper: next_up(distance(&evaluated, &point)),
            point: evaluated,
        })
    };
    let mut active = vec![make_box(domain)?];
    let mut best = active[0].upper;
    let target = tolerance.parametric_bounds().floor;
    let mut subdivisions = 0_usize;
    while subdivisions < MAX_CERTIFICATE_CELLS {
        active.retain(|cell| cell.lower <= best);
        let Some((index, _)) = active
            .iter()
            .enumerate()
            .filter(|(_, cell)| {
                (cell.bounds[1] - cell.bounds[0]).max(cell.bounds[3] - cell.bounds[2]) > target
            })
            .max_by(|(_, a), (_, b)| {
                (a.bounds[1] - a.bounds[0])
                    .max(a.bounds[3] - a.bounds[2])
                    .total_cmp(&(b.bounds[1] - b.bounds[0]).max(b.bounds[3] - b.bounds[2]))
            })
        else {
            break;
        };
        if active.len() + 1 >= MAX_CERTIFICATE_CELLS {
            break;
        }
        let cell = active.swap_remove(index);
        let [u0, u1, v0, v1] = cell.bounds;
        let children = if u1 - u0 >= v1 - v0 {
            let middle = (u0 + u1) * 0.5;
            [
                make_box([u0, middle, v0, v1])?,
                make_box([middle, u1, v0, v1])?,
            ]
        } else {
            let middle = (v0 + v1) * 0.5;
            [
                make_box([u0, u1, v0, middle])?,
                make_box([u0, u1, middle, v1])?,
            ]
        };
        best = best.min(children[0].upper).min(children[1].upper);
        active.extend(children);
        subdivisions += 1;
    }
    active.retain(|cell| cell.lower <= best);
    active.sort_by(|a, b| {
        a.bounds[0]
            .total_cmp(&b.bounds[0])
            .then(a.bounds[2].total_cmp(&b.bounds[2]))
    });
    let boundaries = [
        ("u-min", surface.iso(Axis::U, domain[0])?),
        ("u-max", surface.iso(Axis::U, domain[1])?),
        ("v-min", surface.iso(Axis::V, domain[2])?),
        ("v-max", surface.iso(Axis::V, domain[3])?),
    ]
    .into_iter()
    .map(|(edge, curve)| {
        project_curve(&curve, &point, Some(copy_context(&tolerance)?))
            .map(|certificate| json!({"edge":edge,"certificate":certificate}))
    })
    .collect::<Result<Vec<_>>>()?;
    let mut boxes = active;
    let affine_uniqueness = if surface.degree_u == 1
        && surface.degree_v == 1
        && nu == 2
        && nv == 2
        && surface
            .weights
            .iter()
            .flatten()
            .all(|weight| *weight == surface.weights[0][0])
    {
        let origin = &surface.control_points[0][0];
        let du = surface.control_points[1][0]
            .iter()
            .zip(origin)
            .map(|(x, o)| x - o)
            .collect::<Vec<_>>();
        let dv = surface.control_points[0][1]
            .iter()
            .zip(origin)
            .map(|(x, o)| x - o)
            .collect::<Vec<_>>();
        let closure = surface.control_points[1][1]
            .iter()
            .zip(origin)
            .zip(&du)
            .zip(&dv)
            .all(|(((x, o), u), v)| (*x - *o - *u - *v).abs() <= 64. * f64::EPSILON);
        let a = du.iter().map(|x| x * x).sum::<f64>();
        let b = du.iter().zip(&dv).map(|(x, y)| x * y).sum::<f64>();
        let c = dv.iter().map(|x| x * x).sum::<f64>();
        let determinant = a * c - b * b;
        let rhsu = point
            .iter()
            .zip(origin)
            .zip(&du)
            .map(|((x, o), d)| (x - o) * d)
            .sum::<f64>();
        let rhsv = point
            .iter()
            .zip(origin)
            .zip(&dv)
            .map(|((x, o), d)| (x - o) * d)
            .sum::<f64>();
        let su = (rhsu * c - rhsv * b) / determinant;
        let sv = (rhsv * a - rhsu * b) / determinant;
        (closure && determinant > 0. && (0.0..=1.0).contains(&su) && (0.0..=1.0).contains(&sv))
            .then_some((su, sv, determinant))
    } else {
        None
    };
    let mut uniqueness_proof = None;
    let mut status = if boxes.len() == 1 {
        "isolated_candidate"
    } else {
        "nonunique_or_unresolved"
    };
    if let Some((su, sv, determinant)) = affine_uniqueness {
        let u = domain[0] + (domain[1] - domain[0]) * su;
        let v = domain[2] + (domain[3] - domain[2]) * sv;
        let projected = surface.evaluate_validated(u, v)?.point;
        let d = next_up(distance(&projected, &point));
        boxes = vec![SurfaceBox {
            bounds: [u, u, v, v],
            lower: next_down(d),
            upper: d,
            point: projected,
        }];
        status = "unique";
        uniqueness_proof = Some(
            json!({"method":"strict-convex-affine-Gram","gramDeterminantLower":next_down(determinant)}),
        );
    } else if let Some(proof) =
        prove_surface_projection_uniqueness(surface, &point, &mut boxes, best, target)?
    {
        status = "unique";
        uniqueness_proof = Some(proof);
        best = boxes
            .iter()
            .map(|cell| cell.upper)
            .fold(f64::INFINITY, f64::min);
    }
    let candidates = boxes.into_iter().map(|cell| json!({
        "parameterBox":[next_down(cell.bounds[0]),next_up(cell.bounds[1]),next_down(cell.bounds[2]),next_up(cell.bounds[3])],
        "point":cell.point,"distanceLower":cell.lower,"distanceUpper":cell.upper,
        "classification":if uniqueness_proof.as_ref().and_then(|p|p.get("method")).and_then(|m|m.as_str())
            ==Some("strict-convex-affine-Gram") {"certified_unique_affine_projection"}
            else if uniqueness_proof.is_some() {"certified_unique_krawczyk_or_separated"}
            else {"certified_global_candidate_box"}
    })).collect::<Vec<_>>();
    Ok(json!({
        "version":"nurbs-foundation/4",
        "status":status,
        "globalDistanceUpper":best,
        "candidates":candidates,
        "boundaryReductions":boundaries,
        "uniquenessProof":uniqueness_proof,
        "coverage":{"method":"2d-outward-hull-subdivision-with-Krawczyk-separation","complete":true,"subdivisions":subdivisions,"resourceLimit":MAX_CERTIFICATE_CELLS},
        "evidence":tolerance_evidence(&tolerance)
    }))
}
