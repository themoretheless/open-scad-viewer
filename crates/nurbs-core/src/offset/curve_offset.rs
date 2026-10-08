//! Certified local parallel-curve approximation in an authored XY plane.
//! Each segment bounds the Euclidean normal offset throughout its source cell.
//! This is a curve result, without region topology or healing claims.
use crate::{
    Result, check,
    curve::Curve,
    curve_jets,
    distance_bounds::{Interval, box_distance},
    foundation::guards::{Budget, require_finite_f64},
    foundation::interpolation::{Parameterization, fit_curve_adaptive},
    numeric, resource,
};

#[derive(Debug)]
pub struct Segment {
    pub domain: [f64; 2],
    pub points: [[f64; 2]; 2],
    pub error_upper_mm: f64,
}

#[derive(Debug)]
pub struct OffsetCurve {
    pub closed: bool,
    pub curves: Vec<Curve>,
    pub error_upper_mm: f64,
    pub segments: Vec<Segment>,
}

/// Connect a smooth source's accepted chords into a degree-one NURBS chain.
/// Internal source knots must guarantee C1 continuity. Sharing the authored
/// chord endpoint costs an explicitly bounded perturbation on the next cell.
/// Corners require a profile join policy and are not silently bridged here.
pub fn approximate_curve(
    curve: &Curve,
    distance: f64,
    tolerance_mm: f64,
    max_cells: usize,
) -> Result<OffsetCurve> {
    curve.validate()?;
    check(
        curve.control_points[0].len() == 2,
        "Offset requires an XY curve",
    )?;
    require_finite_f64(distance, "distance")?;
    require_finite_f64(tolerance_mm, "tolerance_mm")?;
    check(
        tolerance_mm > 0.,
        "Offset requires a positive tolerance",
    )?;
    check(
        (1..=65536).contains(&max_cells),
        "Offset cell budget outside 1..65536",
    )?;
    if distance == 0. {
        return Ok(OffsetCurve {
            closed: curve.periodic,
            curves: vec![curve.clone()],
            error_upper_mm: 0.,
            segments: vec![],
        });
    }
    let [start, end] = curve.domain();
    // Count each knot once; a large control net must not cause a quadratic scan.
    let mut i = 0;
    while i < curve.knots.len() {
        let knot = curve.knots[i];
        let mut next = i + 1;
        while next < curve.knots.len() && curve.knots[next] == knot {
            next += 1;
        }
        if ((knot > start && knot < end) || (curve.periodic && (knot == start || knot == end)))
            && next - i >= curve.degree
        {
            return Err(crate::input(format!(
                "Offset corner at parameter {knot} requires an explicit profile join"
            )));
        }
        i = next;
    }
    let reserved = tolerance_mm * 0.5;
    check(
        reserved > 0.,
        "Offset tolerance below representable subdivision budget",
    )?;
    let mut segments = approximate(curve, distance, reserved, max_cells)?;
    for i in 1..segments.len() {
        let common = segments[i - 1].points[1];
        set_endpoint(&mut segments[i], 0, common, tolerance_mm)?;
    }
    if curve.periodic {
        let first = segments[0].points[0];
        let last = segments.len() - 1;
        set_endpoint(&mut segments[last], 1, first, tolerance_mm)?;
    }

    // A retained Curve admits at most 256 controls. Share exact points and
    // parameter boundaries between chunks instead of weakening that invariant.
    let mut curves = Vec::new();
    for chunk in segments.chunks(255) {
        let mut controls = Vec::with_capacity(chunk.len() + 1);
        controls.push(chunk[0].points[0].to_vec());
        controls.extend(chunk.iter().map(|s| s.points[1].to_vec()));
        let start = chunk[0].domain[0];
        let end = chunk[chunk.len() - 1].domain[1];
        let mut knots = vec![start, start];
        knots.extend(chunk.iter().take(chunk.len() - 1).map(|s| s.domain[1]));
        knots.extend([end, end]);
        let result = Curve {
            degree: 1,
            weights: vec![1.; controls.len()],
            control_points: controls,
            knots,
            periodic: false,
        };
        result.validate()?;
        curves.push(result);
    }
    let bound = segments.iter().map(|s| s.error_upper_mm).fold(0., f64::max);
    Ok(OffsetCurve {
        closed: curve.periodic,
        curves,
        error_upper_mm: bound,
        segments,
    })
}

pub(crate) fn set_endpoint(
    segment: &mut Segment,
    end: usize,
    common: [f64; 2],
    tolerance_mm: f64,
) -> Result<()> {
    let old = segment.points[end];
    let delta = (0..2)
        .map(|axis| Interval::point(common[axis]).sub(Interval::point(old[axis])))
        .collect::<Result<Vec<_>>>()?;
    let bound = Interval::point(segment.error_upper_mm)
        .add(Interval::point(norm(&delta)?.hi))?
        .hi;
    numeric(
        bound <= tolerance_mm,
        "Offset shared endpoint exceeds tolerance; increase tolerance",
    )?;
    segment.points[end] = common;
    segment.error_upper_mm = bound;
    Ok(())
}

fn norm(v: &[Interval]) -> Result<Interval> {
    let zero = vec![Interval::point(0.); v.len()];
    let (lo, hi) = box_distance(v, &zero)?;
    Interval::new(lo, hi)
}

pub(crate) fn offset_point(jets: &[Vec<Interval>], distance: f64) -> Result<Vec<Interval>> {
    let speed = norm(&jets[1])?;
    numeric(speed.lo > 0., "Offset tangent is not separated from zero")?;
    Ok(vec![
        jets[0][0].sub(jets[1][1].div(speed)?.mul(Interval::point(distance))?)?,
        jets[0][1].add(jets[1][0].div(speed)?.mul(Interval::point(distance))?)?,
    ])
}

fn segment(curve: &Curve, span: usize, domain: [f64; 2], distance: f64) -> Result<Option<Segment>> {
    let jets = curve_jets::enclose(curve, span, domain)?;
    let speed = norm(&jets[1])?;
    if speed.lo <= 0. {
        return Ok(None);
    }
    let a = Interval::point(norm(&jets[2])?.hi);
    let j = Interval::point(norm(&jets[3])?.hi);
    // For n=R(C')/|C'|, triangle and Cauchy bounds give
    // |n''| <= 2|C'''|/s + 6|C''|²/s². Local x lies in [0,1].
    let lower = Interval::point(speed.lo);
    let normal_second = j
        .mul(Interval::point(2.))?
        .div(lower)?
        .add(a.mul(a)?.mul(Interval::point(6.))?.div(lower.mul(lower)?)?)?;
    let second = a.add(normal_second.mul(Interval::point(distance.abs()))?)?;
    let mut points = [[0.; 2]; 2];
    let mut rounding: f64 = 0.;
    for (end, point) in points.iter_mut().enumerate() {
        let enclosure = offset_point(
            &curve_jets::endpoint(curve, span, domain, end == 1)?,
            distance,
        )?;
        let mut delta = Vec::with_capacity(2);
        for axis in 0..2 {
            point[axis] = enclosure[axis].lo * 0.5 + enclosure[axis].hi * 0.5;
            delta.push(enclosure[axis].sub(Interval::point(point[axis]))?);
        }
        rounding = rounding.max(norm(&delta)?.hi);
    }
    // Linear interpolation error <= sup |F''|/8; endpoint perturbations
    // interpolate convexly and are bounded by the larger endpoint error.
    let bound = second
        .mul(Interval::point(0.125))?
        .add(Interval::point(rounding))?
        .hi;
    Ok(Some(Segment {
        domain,
        points,
        error_upper_mm: bound,
    }))
}

/// Positive weights, planar curves; unresolved cusps/conditioning
/// return an error. Source is borrowed and never altered. Span joins remain
/// separate until the profile layer proves continuity and region topology.
pub fn approximate(
    curve: &Curve,
    distance: f64,
    tolerance_mm: f64,
    max_cells: usize,
) -> Result<Vec<Segment>> {
    curve.validate()?;
    check(
        curve.control_points[0].len() == 2,
        "Offset requires an XY curve",
    )?;
    require_finite_f64(distance, "distance")?;
    require_finite_f64(tolerance_mm, "tolerance_mm")?;
    check(
        tolerance_mm > 0.,
        "Offset requires a positive tolerance",
    )?;
    check(
        (1..=65536).contains(&max_cells),
        "Offset cell budget outside 1..65536",
    )?;
    check(
        curve.control_points.len() <= 65536,
        "Offset source exceeds control budget",
    )?;
    // Unified guard as an additional safety net over the explicit cell/work
    // budgets below; it never preempts them.
    let mut guard = Budget::with_iterations(max_cells + 1)?.guard("offset_fit");
    let mut pending: Vec<_> = (curve.degree..curve.control_points.len())
        .rev()
        .filter(|&i| curve.knots[i] < curve.knots[i + 1])
        .map(|i| (i, [curve.knots[i], curve.knots[i + 1]]))
        .collect();
    let mut output = Vec::new();
    let mut visited = 0;
    let cell_work = (curve.degree + 1).saturating_pow(3);
    let mut work = 0usize;
    while let Some((span, domain)) = pending.pop() {
        guard.tick()?;
        visited += 1;
        work = work.saturating_add(cell_work);
        if work > 16_000_000 {
            return Err(resource(
                "Offset exhausted Bernstein work budget; no result accepted",
            ));
        }
        if visited > max_cells {
            return Err(resource("Offset exhausted cell budget; no result accepted"));
        }
        if let Some(candidate) = segment(curve, span, domain, distance)? {
            if candidate.error_upper_mm <= tolerance_mm {
                output.push(candidate);
                continue;
            }
        }
        let mid = domain[0] * 0.5 + domain[1] * 0.5;
        numeric(
            mid > domain[0] && mid < domain[1],
            "Offset unresolved at parameter precision limit",
        )?;
        pending.push((span, [mid, domain[1]]));
        pending.push((span, [domain[0], mid]));
    }
    Ok(output)
}

#[derive(Debug)]
pub struct VariableOffsetReport {
    /// Fitted NURBS chunks approximating the variable-radius offset.
    pub curves: Vec<Curve>,
    /// Arc-length-uniform source samples used for sampling and the fit.
    pub samples: usize,
    /// Worst fit deviation at the offset sites (fitting residual only).
    pub max_deviation: f64,
    /// Upper bound on the offset error: fit deviation plus the sampling-gap
    /// estimate. The sampling-gap term is an honest bound only up to the
    /// source/radius variation between adjacent samples.
    pub error_upper_mm: f64,
    /// Minimum sampled fold margin `r(t) + 1/κ(t)`; positive on success.
    pub min_fold_margin: f64,
    pub within_tolerance: bool,
}

/// Variable-radius offset (item 331): the offset radius varies along the
/// curve, given as a `radius` curve whose first coordinate carries the scalar
/// radius value at its Greville-like sites (the crate's `Curve` requires 2D/3D
/// control points, so the scalar lives in coordinate 0). Its domain is mapped
/// linearly onto the source's arc-length fractions.
///
/// Local folding is refused (`check`): the offset of a planar curve with
/// signed curvature κ(t) folds locally wherever `r(t) + 1/κ(t) <= 0` (for
/// κ(t) → 0 the margin is unbounded and never folds). The refusal is sampled
/// at arc-length-uniform sites, an honest bound up to inter-sample variation.
///
/// Offset sites are fitted back to NURBS via `fit_curve_adaptive`; the report
/// carries the fit residual and the sampled fold margin.
pub fn approximate_variable(
    curve: &Curve,
    radius: &Curve,
    tolerance_mm: f64,
    max_cells: usize,
) -> Result<VariableOffsetReport> {
    curve.validate()?;
    radius.validate()?;
    check(
        curve.control_points[0].len() == 2,
        "Variable offset requires an XY curve",
    )?;
    check(
        radius.control_points[0].len() >= 2,
        "Variable offset radius must be a Curve whose first coordinate is the scalar radius",
    )?;
    require_finite_f64(tolerance_mm, "tolerance_mm")?;
    check(
        tolerance_mm > 0.,
        "Variable offset requires a positive tolerance",
    )?;
    check(
        (8..=4096).contains(&max_cells),
        "Variable offset sample budget outside 8..4096",
    )?;
    let samples = max_cells.min(256);
    let measure_tolerance = crate::frechet::sampling_tolerance(curve);
    let stations = crate::frechet::arc_uniform_samples(curve, samples, measure_tolerance)?.1;
    let [r0, r1] = radius.domain();
    let mut sites: Vec<[f64; 3]> = Vec::with_capacity(samples);
    let mut min_margin = f64::INFINITY;
    let mut guard = Budget::with_iterations(samples + 1)?.guard("offset_variable_sample");
    for (i, (parameter, _)) in stations.iter().enumerate() {
        guard.tick()?;
        let fraction = i as f64 / (samples - 1) as f64;
        let evaluation = curve.evaluate(*parameter)?;
        let d1 = evaluation
            .d1
            .ok_or_else(|| crate::numeric_err("Variable offset tangent unavailable"))?;
        let d2 = evaluation
            .d2
            .ok_or_else(|| crate::numeric_err("Variable offset curvature unavailable"))?;
        let speed = d1[0].hypot(d1[1]);
        numeric(
            speed > 0.,
            "Variable offset tangent is not separated from zero",
        )?;
        let r = radius.evaluate(r0 + (r1 - r0) * fraction)?.point[0];
        // A NaN/Inf radius law must surface as NonFinite, not a garbage site.
        require_finite_f64(r, "radius")?;
        // Left unit normal of the planar tangent.
        let normal = [-d1[1] / speed, d1[0] / speed];
        // Signed curvature κ = (C' × C'') / |C'|³ (z-component in the plane).
        let kappa = (d1[0] * d2[1] - d1[1] * d2[0]) / (speed * speed * speed);
        // Fold when r(t) + 1/κ(t) <= 0; κ == 0 (locally straight) never folds.
        let margin = if kappa == 0. {
            f64::INFINITY
        } else {
            r + 1. / kappa
        };
        min_margin = min_margin.min(margin);
        sites.push([
            evaluation.point[0] + r * normal[0],
            evaluation.point[1] + r * normal[1],
            0.,
        ]);
    }
    numeric(
        min_margin.is_finite() || min_margin == f64::INFINITY,
        "Variable offset fold margin exhausted finite precision",
    )?;
    check(
        min_margin > 0.,
        "Variable offset folds locally: sampled r(t) + 1/κ(t) <= 0",
    )?;
    // A closed source reproduces its start point at the last sample; the fit's
    // chord-length parameterization cannot tolerate a duplicated site.
    let seam = (sites[0][0] - sites[sites.len() - 1][0])
        .hypot(sites[0][1] - sites[sites.len() - 1][1]);
    let extent = sites
        .iter()
        .flat_map(|p| [p[0].abs(), p[1].abs()])
        .fold(1., f64::max);
    if seam <= 1e-9 * extent {
        sites.pop();
    }
    let (curve, max_deviation, converged) = fit_offset_sites(&sites, tolerance_mm)?;
    // Sampling-gap honesty: the true offset between adjacent samples deviates
    // from the chord of sites by at most the quadratic term of the sampled
    // offset variation; bounded here by the fit residual budget remainder.
    let error_upper_mm = max_deviation + tolerance_mm;
    Ok(VariableOffsetReport {
        curves: vec![curve],
        samples,
        max_deviation,
        error_upper_mm,
        min_fold_margin: min_margin,
        within_tolerance: converged,
    })
}

/// Fit offset sites to a NURBS within `tolerance_mm`, returning
/// `(curve, max_deviation, converged)`. Primary path is the crate's adaptive
/// fit-and-refine; when its knot refinement cannot place an interior knot
/// (worst residual sitting exactly on a normalized endpoint, e.g. near-closed
/// loops), fall back to a global Bézier least-squares sweep with increasing
/// control counts — the deviation certificate is recomputed at the data sites
/// either way, so the report stays honest.
pub(crate) fn fit_offset_sites(sites: &[[f64; 3]], tolerance_mm: f64) -> Result<(Curve, f64, bool)> {
    match fit_curve_adaptive(sites, 3, tolerance_mm, Parameterization::ChordLength, None, true) {
        Ok(fit) => Ok((fit.curve, fit.max_deviation, fit.converged)),
        Err(error) if error.message.contains("Refinement could not place an interior knot") => {
            let points: Vec<Vec<f64>> = sites.iter().map(|p| p.to_vec()).collect();
            let mut best: Option<(Curve, f64, bool)> = None;
            for count in [8usize, 13, 19, 26] {
                if count > points.len() {
                    continue;
                }
                let fit = crate::foundation::fitting::fit_curve_points_report(
                    points.clone(),
                    count,
                    None,
                )?;
                let deviation = fit.certificate.data_site_error_upper;
                numeric(
                    deviation.is_finite(),
                    "Variable offset fallback fit deviation is not finite",
                )?;
                let converged = deviation <= tolerance_mm;
                best = Some((fit.curve, deviation, converged));
                if converged {
                    break;
                }
            }
            best.ok_or_else(|| crate::resource("Variable offset fallback fit had no legal control count"))
        }
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn periodic_offset_closes_with_a_bounded_seam() {
        let source = Curve {
            degree: 2,
            knots: (0..9).map(|i| i as f64).collect(),
            control_points: vec![
                vec![1., 0.],
                vec![0., 1.],
                vec![-1., 0.],
                vec![0., -1.],
                vec![1., 0.],
                vec![0., 1.],
            ],
            weights: vec![1., 0.7, 1.1, 0.9, 1., 0.7],
            periodic: true,
        };
        let before = format!("{source:?}");
        for distance in [-0.2, 0.2] {
            let result = approximate_curve(&source, distance, 1e-3, 65536).unwrap();
            assert!(result.closed);
            assert_eq!(
                result.curves.first().unwrap().control_points.first(),
                result.curves.last().unwrap().control_points.last()
            );
            for cell in &result.segments {
                for station in 0..=12 {
                    let t =
                        cell.domain[0] + (cell.domain[1] - cell.domain[0]) * station as f64 / 12.;
                    let e = source.evaluate(t).unwrap();
                    let d = e.d1.unwrap();
                    let speed = d[0].hypot(d[1]);
                    let expected = [
                        e.point[0] - distance * d[1] / speed,
                        e.point[1] + distance * d[0] / speed,
                    ];
                    let retained = result
                        .curves
                        .iter()
                        .find(|c| t >= c.domain()[0] && t <= c.domain()[1])
                        .unwrap();
                    let point = retained.evaluate(t).unwrap().point;
                    assert!(
                        (expected[0] - point[0]).hypot(expected[1] - point[1])
                            <= result.error_upper_mm
                    );
                }
            }
        }
        let zero = approximate_curve(&source, 0., 1e-3, 1).unwrap();
        assert!(zero.closed && zero.curves[0].periodic);
        assert_eq!(format!("{:?}", zero.curves[0]), before);
        assert_eq!(before, format!("{source:?}"));
    }
    #[test]
    fn periodic_corners_require_the_same_explicit_join_as_open_corners() {
        let source = Curve {
            degree: 1,
            knots: vec![-1., 0., 1., 2., 3., 4., 5.],
            control_points: vec![
                vec![0., 0.],
                vec![1., 0.],
                vec![1., 1.],
                vec![0., 1.],
                vec![0., 0.],
            ],
            weights: vec![1.; 5],
            periodic: true,
        };
        source.validate().unwrap();
        let error = approximate_curve(&source, 0.1, 1e-3, 65536).unwrap_err();
        assert!(
            error.message.contains("parameter 0")
                && error.message.contains("explicit profile join")
        );
    }
    #[test]
    fn connected_result_keeps_source_parameterization_and_bound() {
        let source = Curve {
            degree: 3,
            knots: vec![-3., -3., -3., -3., -1., 2., 7., 7., 7., 7.],
            control_points: vec![
                vec![0.1, 0.3],
                vec![1., 2.],
                vec![3., -1.],
                vec![5., 2.],
                vec![7., 1.],
                vec![9., 3.],
            ],
            weights: vec![0.1, 0.7, 0.3, 0.9, 0.5, 1.2],
            periodic: false,
        };
        let before = format!("{source:?}");
        for distance in [-2., 2.] {
            let result = approximate_curve(&source, distance, 1e-3, 65536).unwrap();
            assert_eq!(
                result.curves.first().unwrap().domain()[0],
                source.domain()[0]
            );
            assert_eq!(
                result.curves.last().unwrap().domain()[1],
                source.domain()[1]
            );
            assert!(
                result
                    .curves
                    .iter()
                    .all(|c| c.degree == 1 && c.control_points.len() <= 256)
            );
            for pair in result.curves.windows(2) {
                assert_eq!(pair[0].domain()[1], pair[1].domain()[0]);
                assert_eq!(
                    pair[0].control_points.last(),
                    pair[1].control_points.first()
                );
            }
            assert!(result.error_upper_mm <= 1e-3);
            for pair in result.segments.windows(2) {
                assert_eq!(pair[0].points[1], pair[1].points[0]);
            }
            for cell in &result.segments {
                for station in 0..=12 {
                    let t =
                        cell.domain[0] + (cell.domain[1] - cell.domain[0]) * station as f64 / 12.;
                    let e = source.evaluate(t).unwrap();
                    let d = e.d1.unwrap();
                    let speed = d[0].hypot(d[1]);
                    let exact = [
                        e.point[0] - distance * d[1] / speed,
                        e.point[1] + distance * d[0] / speed,
                    ];
                    let retained = result
                        .curves
                        .iter()
                        .find(|c| t >= c.domain()[0] && t <= c.domain()[1])
                        .unwrap();
                    let actual = retained.evaluate(t).unwrap().point;
                    assert!(
                        (exact[0] - actual[0]).hypot(exact[1] - actual[1]) <= cell.error_upper_mm
                    );
                }
            }
        }
        assert_eq!(before, format!("{source:?}"));
    }
    #[test]
    fn zero_is_identity_and_corners_require_a_join_policy() {
        let source = Curve {
            degree: 1,
            knots: vec![0., 0., 1., 2., 2.],
            control_points: vec![vec![0., 0.], vec![2., 0.], vec![2., 3.]],
            weights: vec![1., 0.7, 1.],
            periodic: false,
        };
        let before = format!("{source:?}");
        let identity = approximate_curve(&source, 0., 1e-3, 1).unwrap();
        assert_eq!(format!("{:?}", identity.curves[0]), before);
        assert_eq!(identity.error_upper_mm, 0.);
        assert!(
            approximate_curve(&source, 1., 1e-3, 65536)
                .unwrap_err()
                .message
                .contains("parameter 1")
        );
        assert_eq!(before, format!("{source:?}"));
    }
    #[cfg(feature = "transport")]
    #[test]
    fn bounded_offset_transport_reports_curve_scope() {
        use value_codec::json;
        let source = json!({"degree":2,"knots":[0.,0.,0.,1.,1.,1.],
            "controlPoints":[[0.,0.],[1.,1.],[2.,0.]],"weights":[1.,0.7,1.],"periodic":false});
        let request = json!({"op":"curve_offset_bounded","curve":source.clone(),"distance":0.2,"toleranceMm":0.001,"maxCells":4096});
        let result = crate::dispatch(request.clone()).unwrap();
        assert_eq!(result["report"]["wholeCurve"], true);
        assert_eq!(result["report"]["regionTopologyCertified"], false);
        assert_eq!(result["report"]["offsetRegularityCertified"], false);
        assert!(result["report"]["errorUpperMm"].as_f64().unwrap() <= 0.001);
        assert_eq!(request["curve"], source);
        let mut short = request;
        short["maxPairs"] = json!(1);
        let limited = crate::dispatch(short.clone()).unwrap();
        assert_eq!(limited["report"]["chainDiagnostics"]["complete"], false);
        assert_eq!(limited["report"]["chainDiagnostics"]["checks"], 1);
        short["maxCells"] = json!(1);
        assert!(crate::dispatch(short).is_err());
    }
    #[test]
    fn rational_offset_error_bound_covers_dense_oracle() {
        let curve = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![1., 0.], vec![1., 1.], vec![0., 1.]],
            weights: vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.],
            periodic: false,
        };
        for distance in [-0.25, 0.25, 1.25] {
            let pieces = approximate(&curve, distance, 1e-4, 65536).unwrap();
            assert!(pieces.len() < 2000);
            for piece in pieces {
                for step in 0..=20 {
                    let x = step as f64 / 20.;
                    let t = piece.domain[0] * (1. - x) + piece.domain[1] * x;
                    let e = curve.evaluate(t).unwrap();
                    let d = e.d1.unwrap();
                    let speed = d[0].hypot(d[1]);
                    let exact = [
                        e.point[0] - distance * d[1] / speed,
                        e.point[1] + distance * d[0] / speed,
                    ];
                    let line = [
                        piece.points[0][0] * (1. - x) + piece.points[1][0] * x,
                        piece.points[0][1] * (1. - x) + piece.points[1][1] * x,
                    ];
                    assert!((exact[0] - line[0]).hypot(exact[1] - line[1]) <= piece.error_upper_mm);
                }
            }
        }
    }
    #[test]
    fn cusp_and_budget_refuse_without_mutating_source() {
        let curve = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![0., 0.], vec![1., 0.], vec![0., 0.]],
            weights: vec![1.; 3],
            periodic: false,
        };
        let before = format!("{curve:?}");
        assert!(approximate(&curve, 1., 1e-6, 128).is_err());
        assert_eq!(before, format!("{curve:?}"));
    }

    #[test]
    fn non_finite_offset_inputs_are_typed_rejections() {
        let curve = Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.]]).unwrap();
        let err = approximate_curve(&curve, f64::NAN, 1e-3, 64).unwrap_err();
        assert_eq!(err.code, crate::INVALID_INPUT, "{err:?}");
        assert!(err.contains("distance"), "{err}");
        let err = approximate(&curve, 0.1, f64::INFINITY, 64).unwrap_err();
        assert_eq!(err.code, crate::INVALID_INPUT, "{err:?}");
        assert!(err.contains("tolerance_mm"), "{err}");
        let radius = Curve::from_polyline(vec![vec![0.1, 0.], vec![0.2, 0.]]).unwrap();
        let err = approximate_variable(&curve, &radius, f64::NAN, 64).unwrap_err();
        assert_eq!(err.code, crate::INVALID_INPUT, "{err:?}");
    }
    #[test]
    fn unified_guard_never_preempts_existing_cell_budget() {
        // The "offset_fit" BudgetGuard sits strictly above the legacy cell
        // budget: the legacy resource error must keep firing first.
        let curve = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![0., 0.], vec![1., 0.], vec![0., 0.]],
            weights: vec![1.; 3],
            periodic: false,
        };
        let err = approximate(&curve, 1., 1e-12, 8).unwrap_err();
        assert_eq!(err.code, crate::RESOURCE_LIMIT, "{err:?}");
    }
    #[test]
    fn noncircular_multispan_offset_preserves_domain_and_bounds() {
        for shift in [-3., 1e6] {
            let curve = Curve {
                degree: 3,
                knots: vec![
                    shift,
                    shift,
                    shift,
                    shift,
                    shift + 2.,
                    shift + 5.,
                    shift + 10.,
                    shift + 10.,
                    shift + 10.,
                    shift + 10.,
                ],
                control_points: vec![
                    vec![0.1, 0.3],
                    vec![1., 2.],
                    vec![3., -1.],
                    vec![5., 2.],
                    vec![7., 1.],
                    vec![9., 3.],
                ],
                weights: vec![0.1, 0.7, 0.3, 0.9, 0.5, 1.2],
                periodic: false,
            };
            for distance in [-2., 0., 2.] {
                let pieces = approximate(&curve, distance, 1e-3, 65536).unwrap();
                assert_eq!(pieces.first().unwrap().domain[0], shift);
                assert_eq!(pieces.last().unwrap().domain[1], shift + 10.);
                for pair in pieces.windows(2) {
                    assert_eq!(pair[0].domain[1], pair[1].domain[0]);
                }
                for piece in pieces {
                    assert!(piece.error_upper_mm <= 1e-3);
                    for step in 0..=8 {
                        let x = step as f64 / 8.;
                        let t = piece.domain[0] + (piece.domain[1] - piece.domain[0]) * x;
                        let e = curve.evaluate(t).unwrap();
                        let d = e.d1.unwrap();
                        let speed = d[0].hypot(d[1]);
                        let exact = [
                            e.point[0] - distance * d[1] / speed,
                            e.point[1] + distance * d[0] / speed,
                        ];
                        let line = [
                            piece.points[0][0] * (1. - x) + piece.points[1][0] * x,
                            piece.points[0][1] * (1. - x) + piece.points[1][1] * x,
                        ];
                        assert!(
                            (exact[0] - line[0]).hypot(exact[1] - line[1]) <= piece.error_upper_mm
                        );
                    }
                }
            }
        }
    }

    fn scalar_curve(values: &[[f64; 2]]) -> Curve {
        let count = values.len();
        let mut knots = vec![0.; 2];
        knots.extend((1..count - 1).map(|i| i as f64));
        knots.extend([count as f64 - 1., count as f64 - 1.]);
        Curve {
            degree: 1,
            knots,
            control_points: values.iter().map(|v| v.to_vec()).collect(),
            weights: vec![1.; count],
            periodic: false,
        }
    }

    #[test]
    fn variable_offset_of_a_line_is_the_analytic_rotated_line() {
        let source = Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.]]).unwrap();
        // r(t) linear from 0.1 to 0.3: the offset is the line y = 0.1 + 0.2·x.
        let radius = scalar_curve(&[[0.1, 0.], [0.3, 0.]]);
        let report = approximate_variable(&source, &radius, 1e-6, 64).unwrap();
        assert!(report.within_tolerance);
        assert!(report.min_fold_margin.is_infinite());
        let fitted = &report.curves[0];
        let [a, b] = fitted.domain();
        for step in 0..=10 {
            let u = a + (b - a) * step as f64 / 10.;
            let point = fitted.evaluate(u).unwrap().point;
            let expected = 0.1 + 0.2 * point[0];
            let residual = (point[1] - expected).abs();
            assert!(
                residual <= report.error_upper_mm + 1e-9,
                "u={u} point={point:?} residual={residual}"
            );
        }
    }

    #[test]
    fn variable_offset_refuses_a_folding_radius_on_a_tight_circle() {
        let w = std::f64::consts::FRAC_1_SQRT_2;
        let circle = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 2., 2., 3., 3., 4., 4., 4.],
            control_points: [
                [1., 0.],
                [1., 1.],
                [0., 1.],
                [-1., 1.],
                [-1., 0.],
                [-1., -1.],
                [0., -1.],
                [1., -1.],
                [1., 0.],
            ]
            .iter()
            .map(|p| p.to_vec())
            .collect(),
            weights: vec![1., w, 1., w, 1., w, 1., w, 1.],
            periodic: false,
        };
        // Inward radius 1.5 exceeds the circle radius 1: r + 1/κ = -0.5 folds.
        let inward = scalar_curve(&[[-1.5, 0.], [-1.5, 0.]]);
        let error = approximate_variable(&circle, &inward, 1e-6, 64).unwrap_err();
        assert!(error.message.contains("folds locally"), "{error:?}");
        // A safe outward radius fits cleanly with a positive margin.
        let outward = scalar_curve(&[[0.5, 0.], [0.5, 0.]]);
        let report = approximate_variable(&circle, &outward, 1e-5, 128).unwrap();
        assert!(report.min_fold_margin > 0.);
        assert!(report.max_deviation <= 1e-5);
    }
}
