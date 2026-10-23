//! Bounded construction of a proper intersection of two represented XY chords.
//! Endpoint contacts and overlapping/parallel chords require separate graph rules.
use crate::{
    Result, check,
    distance_bounds::{Interval, box_distance},
    numeric,
};
#[derive(Debug)]
pub struct Intersection {
    pub point: [f64; 2],
    pub parameter_a: [f64; 2],
    pub parameter_b: [f64; 2],
    pub error_upper_mm: f64,
}
fn cross(a: [Interval; 2], b: [Interval; 2]) -> Result<Interval> {
    a[0].mul(b[1])?.sub(a[1].mul(b[0])?)
}
pub fn proper(a: [[f64; 2]; 2], b: [[f64; 2]; 2], tolerance_mm: f64) -> Result<Intersection> {
    check(
        a.into_iter()
            .chain(b)
            .flatten()
            .all(|v| v.is_finite() && v.abs() <= 1e9)
            && tolerance_mm.is_finite()
            && tolerance_mm > 0.,
        "Intersection requires bounded finite XY points and positive tolerance.",
    )?;
    let a = a.map(|p| p.map(Interval::point));
    let b = b.map(|p| p.map(Interval::point));
    let u = [a[1][0].sub(a[0][0])?, a[1][1].sub(a[0][1])?];
    let v = [b[1][0].sub(b[0][0])?, b[1][1].sub(b[0][1])?];
    let delta = [b[0][0].sub(a[0][0])?, b[0][1].sub(a[0][1])?];
    let denominator = cross(u, v)?;
    numeric(
        denominator.lo > 0. || denominator.hi < 0.,
        "Chord intersection is parallel or unresolved.",
    )?;
    let t = cross(delta, v)?.div_signed(denominator)?;
    let s = cross(delta, u)?.div_signed(denominator)?;
    numeric(
        t.lo > 0. && t.hi < 1. && s.lo > 0. && s.hi < 1.,
        "Intersection is not proved strictly inside both chords.",
    )?;
    let mut enclosure = [Interval::point(0.); 2];
    for k in 0..2 {
        let left = a[0][k].add(u[k].mul(t)?)?;
        let right = b[0][k].add(v[k].mul(s)?)?;
        enclosure[k] = left.intersect(right.lo, right.hi)?;
    }
    let point = enclosure.map(|i| i.lo * 0.5 + i.hi * 0.5);
    let error_upper_mm = box_distance(&enclosure, &point.map(Interval::point))?.1;
    numeric(
        error_upper_mm <= tolerance_mm,
        "Intersection construction exceeds tolerance.",
    )?;
    Ok(Intersection {
        point,
        parameter_a: [t.lo, t.hi],
        parameter_b: [s.lo, s.hi],
        error_upper_mm,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn encloses_fractional_and_translated_intersections() {
        for shift in [0., 1000000.] {
            let a = [[shift, shift], [shift + 3., shift + 3.]];
            let b = [[shift, shift + 2.], [shift + 3., shift]];
            let r = proper(a, b, 1e-7).unwrap();
            let reversed = proper(a, [b[1], b[0]], 1e-7).unwrap();
            assert!(
                (reversed.point[0] - r.point[0]).abs()
                    <= r.error_upper_mm + reversed.error_upper_mm
            );
            assert!(r.parameter_a[0] <= 0.4 && r.parameter_a[1] >= 0.4);
            assert!(r.parameter_b[0] <= 0.4 && r.parameter_b[1] >= 0.4);
            assert!((r.point[0] - (shift + 1.2)).abs() <= r.error_upper_mm + 2e-10);
            assert!(r.error_upper_mm <= 1e-7);
        }
    }
    #[test]
    fn negative_miter_determinants_construct_the_same_join() {
        let forward =
            crate::curve_offset_join::miter([0., 2.], [1., 0.], [2., 0.], [0., 1.], 10., 1e-6)
                .unwrap();
        let reversed =
            crate::curve_offset_join::miter([0., 2.], [1., 0.], [2., 0.], [0., -1.], 10., 1e-6)
                .unwrap();
        assert!(
            (forward.points[1][0] - reversed.points[1][0]).abs()
                <= forward.error_upper_mm + reversed.error_upper_mm
        );
        assert!(
            (forward.points[1][1] - reversed.points[1][1]).abs()
                <= forward.error_upper_mm + reversed.error_upper_mm
        );
    }
    #[test]
    fn refuses_contacts_parallel_outside_and_unmet_tolerance() {
        let a = [[0., 0.], [2., 2.]];
        assert!(proper(a, [[2., 2.], [4., 0.]], 0.01).is_err());
        assert!(proper(a, [[0., 1.], [2., 3.]], 0.01).is_err());
        assert!(proper(a, [[3., 0.], [3., 4.]], 0.01).is_err());
        assert!(proper(a, [[0., 2.], [2., 0.]], 1e-30).is_err());
    }
}
