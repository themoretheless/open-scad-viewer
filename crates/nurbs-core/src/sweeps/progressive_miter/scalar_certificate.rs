//! Outward-rounded jets of positive-weight scalar NURBS laws.
//!
//! Restriction uses interval homogeneous blossoms, never a rounded Curve::trim.
//! Bounds enclose values and one-sided derivatives with respect to the authored
//! knot parameter. They do not certify frames, stations, or a sweep surface.
use crate::{Result, check, curve::Curve, distance_bounds::Interval};
mod third;
mod first_point;
pub use first_point::{PointFirstReport,certify_first_point};
pub use third::{ThirdReport,certify_third_traversal};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Certified,
    Unresolved,
}
#[derive(Clone, Debug)]
pub struct Report {
    pub status: Status,
    pub cells: usize,
    pub value: Option<[f64; 2]>,
    pub first: Option<[f64; 2]>,
    pub second: Option<[f64; 2]>,
    /// A second-derivative interpolation remainder must not cross a law knot.
    pub single_span: bool,
    pub reason: Option<&'static str>,
}
fn hull(values: &[Interval]) -> Interval {
    Interval {
        lo: values.iter().map(|v| v.lo).fold(f64::INFINITY, f64::min),
        hi: values
            .iter()
            .map(|v| v.hi)
            .fold(f64::NEG_INFINITY, f64::max),
    }
}
fn derivative(values: &[Interval], degree: usize, width: Interval) -> Result<Vec<Interval>> {
    if degree == 0 {
        return Ok(vec![Interval::point(0.)]);
    }
    let factor = Interval::point(degree as f64).div(width)?;
    values
        .windows(2)
        .map(|v| v[1].sub(v[0])?.mul(factor))
        .collect()
}
fn span_jets(c: &Curve, span: usize, lo: f64, hi: f64) -> Result<[Interval; 3]> {
    let a=c.knots[span];
    let b=c.knots[span+1];
    // Outward traversal mapping can overlap an adjacent knot span by only
    // one ulp. Differentiate its original full-span polynomial before
    // restriction: dividing differences of restricted poles by a tiny width
    // needlessly destroys the enclosure (or underflows its denominator).
    if hi-lo <= (b-a)*1e-10 {
        let controls=crate::curve_distance::restricted_controls(c,span,Interval::new(a,b)?)?;
        let width=Interval::point(b).sub(Interval::point(a))?;
        let local=Interval::new(lo,hi)?.sub(Interval::point(a))?.div(width)?.intersect(0.,1.)?;
        let numerator=controls.iter().map(|p|p[0]).collect::<Vec<_>>();
        let weights=controls.iter().map(|p|p[3]).collect::<Vec<_>>();
        let n1=derivative(&numerator,c.degree,width)?;
        let w1=derivative(&weights,c.degree,width)?;
        let n2=derivative(&n1,c.degree.saturating_sub(1),width)?;
        let w2=derivative(&w1,c.degree.saturating_sub(1),width)?;
        let evaluate=|v|first_point::evaluate(v,local);
        let weight=evaluate(weights)?;
        let relative=evaluate(numerator)?.div(weight)?;
        let weight_first=evaluate(w1)?;
        let first=evaluate(n1)?.sub(relative.mul(weight_first)?)?.div(weight)?;
        let second=evaluate(n2)?.sub(first.mul(weight_first)?.mul(Interval::point(2.))?)?.sub(relative.mul(evaluate(w2)?)?)?.div(weight)?;
        return Ok([relative.add(Interval::point(c.control_points[span-c.degree][0]))?,first,second]);
    }
    let controls = crate::curve_distance::restricted_controls(c, span, Interval::new(lo, hi)?)?;
    let width = Interval::point(hi).sub(Interval::point(lo))?;
    let numerator: Vec<_> = controls.iter().map(|p| p[0]).collect();
    let weights: Vec<_> = controls.iter().map(|p| p[3]).collect();
    let weight = hull(&weights);
    let relative = hull(&numerator).div(weight)?;
    let n1 = derivative(&numerator, c.degree, width)?;
    let w1 = derivative(&weights, c.degree, width)?;
    let first = hull(&n1).sub(relative.mul(hull(&w1))?)?.div(weight)?;
    let n2 = derivative(&n1, c.degree.saturating_sub(1), width)?;
    let w2 = derivative(&w1, c.degree.saturating_sub(1), width)?;
    let second = hull(&n2)
        .sub(first.mul(hull(&w1))?.mul(Interval::point(2.))?)?
        .sub(relative.mul(hull(&w2))?)?
        .div(weight)?;
    let value = relative.add(Interval::point(c.control_points[span - c.degree][0]))?;
    Ok([value, first, second])
}
/// An exhausted work budget or nonrepresentable positive denominator is
/// inconclusive. Input errors remain errors. No partial hull is a certificate.
pub fn certify(c: &Curve, domain: [f64; 2], max_cells: usize) -> Result<Report> {
    c.validate()?;
    check(
        c.control_points
            .iter()
            .all(|p| p.len() == 3 && p[1] == 0. && p[2] == 0.),
        "Sweep scalar law requires [value,0,0]",
    )?;
    let [a, b] = c.domain();
    let [lo, hi] = domain;
    check(
        lo.is_finite() && hi.is_finite() && a <= lo && lo < hi && hi <= b,
        "Scalar certificate needs an ordered active subdomain",
    )?;
    check(
        max_cells <= 100000,
        "Scalar certificate work budget exceeds100000 cells",
    )?;
    for knot in c.knots.iter().copied().filter(|k| *k > lo && *k < hi) {
        check(
            c.knots.iter().filter(|k| **k == knot).count() <= c.degree,
            "Scalar certificate requires continuous laws",
        )?;
    }
    let mut cells = 0;
    let mut bounds = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
    let unresolved = |cells, reason| Report {
        status: Status::Unresolved,
        cells,
        value: None,
        first: None,
        second: None,
        single_span: false,
        reason: Some(reason),
    };
    for span in c.degree..c.control_points.len() {
        let l = lo.max(c.knots[span]);
        let h = hi.min(c.knots[span + 1]);
        if l >= h {
            continue;
        }
        if cells == max_cells {
            return Ok(unresolved(cells, "cell-budget-exhausted"));
        }
        cells += 1;
        let Ok(jets) = span_jets(c, span, l, h) else {
            return Ok(unresolved(cells, "numeric-enclosure-unresolved"));
        };
        for (out, j) in bounds.iter_mut().zip(jets) {
            out[0] = out[0].min(j.lo);
            out[1] = out[1].max(j.hi);
        }
    }
    Ok(Report {
        status: Status::Certified,
        cells,
        value: Some(bounds[0]),
        first: Some(bounds[1]),
        second: Some(bounds[2]),
        single_span: cells == 1,
        reason: None,
    })
}
/// Value-only enclosure, including exact endpoints; avoids forming jets on
/// vanishingly narrow intervals. Exhaustion discards the whole accumulated hull.
pub fn value_traversal(
    c: &Curve,
    traversal: [f64; 2],
    max_cells: usize,
) -> Result<Option<[f64; 2]>> {
    c.validate()?;
    check(
        max_cells <= 100000
            && traversal[0] >= 0.
            && traversal[1] <= 1.
            && traversal[0] <= traversal[1]
            && traversal.iter().all(|v| v.is_finite())
            && c.control_points.iter().all(|p| p.len() == 3),
        "Invalid scalar value certificate input",
    )?;
    let [a, b] = c.domain();
    let result = (|| -> Result<Option<[f64; 2]>> {
        let t = Interval::point(a)
            .add(
                Interval::point(b)
                    .sub(Interval::point(a))?
                    .mul(Interval::new(traversal[0], traversal[1])?)?,
            )?
            .intersect(a, b)?;
        let mut lo = f64::INFINITY;
        let mut hi = f64::NEG_INFINITY;
        let mut cells = 0;
        for span in c.degree..c.control_points.len() {
            if c.knots[span] >= c.knots[span + 1] {
                continue;
            }
            let l = t.lo.max(c.knots[span]);
            let h = t.hi.min(c.knots[span + 1]);
            if l > h {
                continue;
            }
            if cells == max_cells {
                return Ok(None);
            }
            cells += 1;
            let controls =
                crate::curve_distance::restricted_controls(c, span, Interval::new(l, h)?)?;
            let numerator: Vec<_> = controls.iter().map(|v| v[0]).collect();
            let weights: Vec<_> = controls.iter().map(|v| v[3]).collect();
            let v = hull(&numerator)
                .div(hull(&weights))?
                .add(Interval::point(c.control_points[span - c.degree][0]))?;
            lo = lo.min(v.lo);
            hi = hi.max(v.hi);
        }
        Ok(Some([lo, hi]))
    })();
    match result {
        Ok(value) => Ok(value),
        Err(_) => Ok(None),
    }
}
/// Certifies a law over normalized traversal without ordinary rounded
/// parameter mapping. Point ranges are widened inside the active domain so
/// the jet certificate also covers exact endpoints and interior point values.
pub fn certify_traversal(c: &Curve, traversal: [f64; 2], max_cells: usize) -> Result<Report> {
    c.validate()?;
    check(
        traversal[0].is_finite()
            && traversal[1].is_finite()
            && traversal[0] >= 0.
            && traversal[1] <= 1.
            && traversal[0] <= traversal[1],
        "Scalar traversal must be an ordered interval in [0,1]",
    )?;
    let [a, b] = c.domain();
    let mapped = (|| -> Result<Interval> {
        Interval::point(a)
            .add(
                Interval::point(b)
                    .sub(Interval::point(a))?
                    .mul(Interval::new(traversal[0], traversal[1])?)?,
            )?
            .intersect(a, b)
    })();
    let Ok(mapped) = mapped else {
        return Ok(Report {
            status: Status::Unresolved,
            cells: 0,
            value: None,
            first: None,
            second: None,
            single_span: false,
            reason: Some("numeric-enclosure-unresolved"),
        });
    };
    let lo = mapped.lo.next_down().max(a);
    let hi = mapped.hi.next_up().min(b);
    certify(c, [lo, hi], max_cells)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn contains(range: Option<[f64; 2]>, v: f64) {
        let [lo, hi] = range.unwrap();
        assert!(lo <= v && v <= hi, "{v} outside [{lo},{hi}]");
    }
    #[test]
    fn independent_linear_jet_on_a_nonunit_domain() {
        let c = Curve {
            degree: 1,
            knots: vec![3., 3., 7., 7.],
            control_points: vec![vec![1., 0., 0.], vec![9., 0., 0.]],
            weights: vec![1., 1.],
            periodic: false,
        };
        let r = certify(&c, [4., 6.], 1).unwrap();
        assert_eq!(r.status, Status::Certified);
        contains(r.value, 3.);
        contains(r.value, 7.);
        contains(r.first, 2.);
        contains(r.second, 0.);
        assert!(r.first.unwrap()[1] - r.first.unwrap()[0] < 1e-12);
    }
    #[test]
    fn rational_jets_cover_nondyadic_probes_without_trim() {
        let c = crate::paths::bezier(
            vec![vec![1., 0., 0.], vec![2.4, 0., 0.], vec![0.8, 0., 0.]],
            Some(vec![1., 0.7, 1.3]),
        )
        .unwrap();
        let r = certify(&c, [0.13, 0.81], 1).unwrap();
        assert_eq!(r.status, Status::Certified);
        for i in 0..127 {
            let e = c.evaluate(0.13 + 0.68 * i as f64 / 126.).unwrap();
            contains(r.value, e.point[0]);
            contains(r.first, e.d1.unwrap()[0]);
            contains(r.second, e.d2.unwrap()[0]);
        }
    }
    #[test]
    fn exhaustion_discards_partial_bounds_and_knots_disable_second_order_remainder() {
        let c = Curve {
            degree: 1,
            knots: vec![0., 0., 0.37, 1., 1.],
            control_points: vec![vec![1., 0., 0.], vec![2., 0., 0.], vec![1., 0., 0.]],
            weights: vec![1.; 3],
            periodic: false,
        };
        let r = certify(&c, [0., 1.], 1).unwrap();
        assert_eq!(r.status, Status::Unresolved);
        assert_eq!(r.cells, 1);
        assert!(r.value.is_none());
        let r = certify(&c, [0., 1.], 2).unwrap();
        assert_eq!(r.status, Status::Certified);
        assert!(!r.single_span);
        contains(r.first, 1. / 0.37);
        contains(r.first, -1. / 0.63);
        assert_eq!(certify(&c, [0., 1.], 0).unwrap().status, Status::Unresolved);
    }
    #[test]
    fn unrepresentable_jet_is_unresolved_instead_of_a_partial_certificate() {
        let c = Curve {
            degree: 1,
            knots: vec![0., 0., 1e-310, 1e-310],
            control_points: vec![vec![1., 0., 0.], vec![2., 0., 0.]],
            weights: vec![1., 1.],
            periodic: false,
        };
        let r = certify(&c, [0., 1e-310], 1).unwrap();
        assert_eq!(r.status, Status::Unresolved);
        assert_eq!(r.reason, Some("numeric-enclosure-unresolved"));
        assert!(r.value.is_none() && r.first.is_none() && r.second.is_none());
    }
    #[test]
    fn huge_constant_origins_do_not_pollute_derivative_bounds() {
        let c =
            crate::paths::bezier(vec![vec![1e8, 0., 0.], vec![1e8 + 1., 0., 0.]], None).unwrap();
        let r = certify(&c, [0.1, 0.9], 1).unwrap();
        contains(r.first, 1.);
        assert!(r.first.unwrap()[1] < 1.00001);
    }
    #[test]
    fn normalized_mapping_covers_nonunit_endpoints_and_interior_points() {
        let c = Curve {
            degree: 1,
            knots: vec![3., 3., 7., 7.],
            control_points: vec![vec![1., 0., 0.], vec![9., 0., 0.]],
            weights: vec![1., 1.],
            periodic: false,
        };
        for f in [0., 0.13, 0.5, 1.] {
            let r = certify_traversal(&c, [f, f], 1).unwrap();
            assert_eq!(r.status, Status::Certified);
            contains(r.value, 1. + 8. * f);
        }
        let u = certify_traversal(&c, [0.13, 0.81], 0).unwrap();
        assert_eq!(u.status, Status::Unresolved);
        assert!(u.value.is_none());
    }
}

/// One-sided endpoint derivative from the whole original active span.
/// Avoids dividing a vanishing point restriction; caller charges one span.
pub fn endpoint_first(c: &Curve, at_end: bool, max_cells: usize) -> Result<Option<[f64; 2]>> {
    c.validate()?;
    check(
        max_cells <= 100000
            && c.control_points
                .iter()
                .all(|p| p.len() == 3 && p[1] == 0. && p[2] == 0.),
        "Invalid scalar endpoint input",
    )?;
    if max_cells == 0 {
        return Ok(None);
    }
    if c.degree == 0 {
        return Ok(Some([0., 0.]));
    }
    let spans = (c.degree..c.control_points.len())
        .filter(|&i| c.knots[i] < c.knots[i + 1])
        .collect::<Vec<_>>();
    let span = if at_end {
        *spans.last().unwrap()
    } else {
        spans[0]
    };
    let result = (|| -> Result<Interval> {
        let a = c.knots[span];
        let b = c.knots[span + 1];
        let controls = crate::curve_distance::restricted_controls(c, span, Interval::new(a, b)?)?;
        let end = if at_end { c.degree } else { 0 };
        let (left, right) = if at_end {
            (c.degree - 1, c.degree)
        } else {
            (0, 1)
        };
        let relative = controls[end][0].div(controls[end][3])?;
        let n = controls[right][0].sub(controls[left][0])?;
        let w = controls[right][3].sub(controls[left][3])?;
        n.sub(relative.mul(w)?)?.div(controls[end][3])?.mul(
            Interval::point(c.degree as f64).div(Interval::point(b).sub(Interval::point(a))?)?,
        )
    })();
    Ok(result.ok().map(|v| [v.lo, v.hi]))
}

#[test]
fn rational_original_endpoint_first_avoids_point_width_division() {
    let c = Curve {
        degree: 1,
        knots: vec![7., 7., 9., 9.],
        control_points: vec![vec![1., 0., 0.], vec![2., 0., 0.]],
        weights: vec![1., 2.],
        periodic: false,
    };
    for (end, expected) in [(false, 1.), (true, 0.25)] {
        let v = endpoint_first(&c, end, 1).unwrap().unwrap();
        assert!(v[0] <= expected && expected <= v[1]);
        assert!(v[1] - v[0] < 1e-10);
    }
    assert!(endpoint_first(&c, false, 0).unwrap().is_none());
}

#[test]
fn tiny_original_span_restriction_retains_rational_jets() {
    let curve=Curve {degree:1,knots:vec![0.,0.,1.,1.],control_points:vec![vec![1.,0.,0.],vec![2.,0.,0.]],weights:vec![1.,2.],periodic:false};
    let lo=0.5_f64;
    let hi=lo.next_up();
    let report=certify(&curve,[lo,hi],1).unwrap();
    assert_eq!(report.status,Status::Certified);
    for t in [lo,hi] {
        for (bound,expected) in [(report.value.unwrap(),(1.+3.*t)/(1.+t)),(report.first.unwrap(),2./(1.+t).powi(2)),(report.second.unwrap(),-4./(1.+t).powi(3))] {
            assert!(bound[0]<=expected && expected<=bound[1],"{bound:?} excludes {expected}");
            assert!(bound[1]-bound[0]<1e-10);
        }
    }
    let short=certify(&curve,[lo,hi],0).unwrap();
    assert_eq!(short.status,Status::Unresolved);
    assert!(short.value.is_none() && short.first.is_none() && short.second.is_none());
}
