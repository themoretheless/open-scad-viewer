//! Certified local parallel-curve approximation in an authored XY plane.
//! Each segment bounds the Euclidean normal offset throughout its source cell.
//! This is a curve result, without region topology or healing claims.
use crate::{
    Result, check,
    curve::Curve,
    curve_jets,
    distance_bounds::{Interval, box_distance},
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
    check(
        distance.is_finite() && tolerance_mm.is_finite() && tolerance_mm > 0.,
        "Offset requires finite distance and positive tolerance",
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
    check(
        distance.is_finite() && tolerance_mm.is_finite() && tolerance_mm > 0.,
        "Offset requires finite distance and positive tolerance",
    )?;
    check(
        (1..=65536).contains(&max_cells),
        "Offset cell budget outside 1..65536",
    )?;
    check(
        curve.control_points.len() <= 65536,
        "Offset source exceeds control budget",
    )?;
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
}
