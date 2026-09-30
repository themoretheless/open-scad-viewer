//! Explicit joins of represented planar offset endpoints and tangent lines.
//! Represented-line joins and source-knot joins have explicit positional bounds.
//! Callers must still connect/trim curve spans and resolve region topology.
use crate::{
    Result, check,
    distance_bounds::{Interval, box_distance},
    numeric,
};
#[derive(Debug)]
pub struct Join {
    pub points: Vec<[f64; 2]>,
    pub error_upper_mm: f64,
}
fn cross(a: [Interval; 2], b: [Interval; 2]) -> Result<Interval> {
    a[0].mul(b[1])?.sub(a[1].mul(b[0])?)
}
/// A bevel retains the represented endpoints exactly.
pub fn bevel(a: [f64; 2], b: [f64; 2]) -> Result<Join> {
    check(
        a.into_iter().chain(b).all(f64::is_finite),
        "Join requires finite endpoints",
    )?;
    Ok(Join {
        points: vec![a, b],
        error_upper_mm: 0.,
    })
}
/// Intersect the two represented tangent lines with a bounded extension.
/// Parallel, poorly conditioned or excessive extensions return an error.
pub fn miter(
    a: [f64; 2],
    u: [f64; 2],
    b: [f64; 2],
    v: [f64; 2],
    max_extension: f64,
    tolerance: f64,
) -> Result<Join> {
    check(
        a.into_iter().chain(u).chain(b).chain(v).all(f64::is_finite)
            && max_extension.is_finite()
            && max_extension > 0.
            && tolerance.is_finite()
            && tolerance > 0.,
        "Join requires finite coordinates and positive limits",
    )?;
    miter_enclosed(
        a.map(Interval::point),
        u.map(Interval::point),
        b.map(Interval::point),
        v.map(Interval::point),
        max_extension,
        tolerance,
    )
}
fn miter_enclosed(
    a: [Interval; 2],
    u: [Interval; 2],
    b: [Interval; 2],
    v: [Interval; 2],
    max_extension: f64,
    tolerance: f64,
) -> Result<Join> {
    let delta = [b[0].sub(a[0])?, b[1].sub(a[1])?];
    let denominator = cross(u, v)?;
    numeric(
        denominator.lo > 0. || denominator.hi < 0.,
        "Miter tangent lines are parallel or unresolved; use bevel",
    )?;
    let t = cross(delta, v)?.div(denominator)?;
    let enclosure = [a[0].add(u[0].mul(t)?)?, a[1].add(u[1].mul(t)?)?];
    for origin in [a, b] {
        numeric(
            box_distance(&enclosure, &origin)?.1 <= max_extension,
            "Miter exceeds extension limit; use bevel or increase limit",
        )?;
    }
    let mut points = Vec::new();
    let mut error_upper_mm: f64 = 0.;
    for vertex in [a, enclosure, b] {
        let point = vertex.map(|i| i.lo * 0.5 + i.hi * 0.5);
        error_upper_mm = error_upper_mm.max(box_distance(&vertex, &point.map(Interval::point))?.1);
        points.push(point);
    }
    numeric(
        error_upper_mm <= tolerance,
        "Miter construction exceeds tolerance; use bevel",
    )?;
    Ok(Join {
        points,
        error_upper_mm,
    })
}
/// Bound a miter joining the two exact one-sided normal-offset tangent lines
/// at a continuous interior source knot. This does not trim adjacent curves
/// or certify the offset region's topology.
pub fn miter_at_knot(
    curve: &crate::curve::Curve,
    knot: f64,
    distance: f64,
    max_extension: f64,
    tolerance: f64,
) -> Result<Join> {
    curve.validate()?;
    let [start, end] = curve.domain();
    check(
        curve.control_points[0].len() == 2
            && !curve.periodic
            && knot.is_finite()
            && knot > start
            && knot < end
            && distance.is_finite()
            && max_extension.is_finite()
            && max_extension > 0.
            && tolerance.is_finite()
            && tolerance > 0.,
        "Miter requires a planar nonperiodic curve, interior knot and positive limits",
    )?;
    let multiplicity = curve.knots.iter().filter(|&&t| t == knot).count();
    check(
        multiplicity > 0 && multiplicity <= curve.degree,
        "Miter requires a continuous existing source knot",
    )?;
    let left = (curve.degree..curve.control_points.len())
        .find(|&i| curve.knots[i] < knot && curve.knots[i + 1] == knot)
        .ok_or_else(|| crate::input("Missing left join span"))?;
    let right = (curve.degree..curve.control_points.len())
        .find(|&i| curve.knots[i] == knot && curve.knots[i + 1] > knot)
        .ok_or_else(|| crate::input("Missing right join span"))?;
    let jets = |span, end| {
        crate::curve_jets::endpoint(curve, span, [curve.knots[span], curve.knots[span + 1]], end)
    };
    let l = jets(left, true)?;
    let r = jets(right, false)?;
    let a = crate::curve_offset::offset_point(&l, distance)?;
    let b = crate::curve_offset::offset_point(&r, distance)?;
    miter_enclosed(
        [a[0], a[1]],
        [l[1][0], l[1][1]],
        [b[0], b[1]],
        [r[1][0], r[1][1]],
        max_extension,
        tolerance,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn perpendicular_lines_and_bevel() {
        let a = [0., 2.];
        let b = [2., 0.];
        let j = miter(a, [1., 0.], b, [0., 1.], 3., 1e-10).unwrap();
        assert_eq!(j.points[0], a);
        assert_eq!(j.points[2], b);
        assert!((j.points[1][0] - 2.).hypot(j.points[1][1] - 2.) <= j.error_upper_mm);
        assert!(j.error_upper_mm < 1e-10);
        let j = bevel(a, b).unwrap();
        assert_eq!(j.points, vec![a, b]);
        assert_eq!(j.error_upper_mm, 0.);
    }
    #[test]
    fn finite_limits_parallel_and_long_extensions() {
        assert!(miter([0., 0.], [1., 0.], [0., 1.], [2., 0.], 10., 1e-6).is_err());
        assert!(miter([0., 2.], [1., 0.], [2., 0.], [0., 1.], 1., 1e-6).is_err());
        assert!(bevel([f64::NAN, 0.], [1., 1.]).is_err());
        assert!(miter([0., 0.], [0., 0.], [1., 1.], [1., 0.], 10., 1e-6).is_err());
    }
    #[test]
    fn translated_fractional_intersection() {
        for origin in [0., 1e6] {
            let j = miter(
                [origin, origin],
                [0.5, 0.25],
                [origin + 1., origin],
                [0., 0.5],
                10.,
                1e-6,
            )
            .unwrap();
            let error = (j.points[1][0] - (origin + 1.)).hypot(j.points[1][1] - (origin + 0.5));
            assert!(error <= j.error_upper_mm);
            assert!(j.error_upper_mm < 1e-6);
        }
    }
    #[test]
    fn source_knot_preserves_original_and_bounds_weighted_corner() {
        for distance in [-2., 2.] {
            let c = crate::curve::Curve {
                degree: 1,
                knots: vec![0., 0., 0.5, 1., 1.],
                weights: vec![0.7, 1.3, 0.9],
                control_points: vec![vec![0., 0.], vec![10., 0.], vec![10., 10.]],
                periodic: false,
            };
            let original = c.clone();
            let j = miter_at_knot(&c, 0.5, distance, 3., 1e-8).unwrap();
            let ideal = [
                [10., distance],
                [10. - distance, distance],
                [10. - distance, 0.],
            ];
            for (p, q) in j.points.iter().zip(ideal) {
                assert!((p[0] - q[0]).hypot(p[1] - q[1]) <= j.error_upper_mm);
            }
            assert_eq!(c.control_points, original.control_points);
            assert_eq!(c.knots, original.knots);
            assert_eq!(c.weights, original.weights);
            assert!(miter_at_knot(&c, 0.25, distance, 3., 1e-8).is_err());
            assert!(miter_at_knot(&c, 0.5, distance, 1., 1e-8).is_err());
        }
    }
    #[test]
    fn rational_curved_sides_use_one_sided_source_jets() {
        let c = crate::curve::Curve {
            degree: 2,
            knots: vec![0., 0., 0., 0.5, 0.5, 1., 1., 1.],
            weights: vec![0.7, 0.8, 1.3, 0.9, 1.1],
            control_points: vec![
                vec![0., 3.],
                vec![5., 0.],
                vec![10., 0.],
                vec![10., 5.],
                vec![13., 10.],
            ],
            periodic: false,
        };
        for distance in [-2., 2.] {
            let j = miter_at_knot(&c, 0.5, distance, 3., 1e-8).unwrap();
            for (p, q) in j.points.iter().zip([
                [10., distance],
                [10. - distance, distance],
                [10. - distance, 0.],
            ]) {
                assert!((p[0] - q[0]).hypot(p[1] - q[1]) <= j.error_upper_mm);
            }
        }
    }
}
