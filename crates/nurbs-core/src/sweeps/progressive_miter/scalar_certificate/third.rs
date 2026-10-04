//! Original rational third derivative, sharing work with the lower jet proof.
use super::*;
#[derive(Clone, Debug)]
pub struct ThirdReport {
    pub base: Report,
    pub third: Option<[f64; 2]>,
}
fn refuse(out: &mut ThirdReport, reason: &'static str) {
    out.base.status = Status::Unresolved;
    out.base.value = None;
    out.base.first = None;
    out.base.second = None;
    out.base.single_span = false;
    out.base.reason = Some(reason);
    out.third = None;
}
pub fn certify_third_traversal(
    c: &Curve,
    traversal: [f64; 2],
    max_cells: usize,
) -> Result<ThirdReport> {
    let base = certify_traversal(c, traversal, max_cells)?;
    let mut out = ThirdReport { base, third: None };
    if out.base.status != Status::Certified {
        return Ok(out);
    }
    let [a, b] = c.domain();
    let mapped = Interval::point(a)
        .add(
            Interval::point(b)
                .sub(Interval::point(a))?
                .mul(Interval::new(traversal[0], traversal[1])?)?,
        )?
        .intersect(a, b)?;
    let lo = mapped.lo.next_down().max(a);
    let hi = mapped.hi.next_up().min(b);
    let mut lower = f64::INFINITY;
    let mut upper = f64::NEG_INFINITY;
    for span in c.degree..c.control_points.len() {
        let l = lo.max(c.knots[span]);
        let h = hi.min(c.knots[span + 1]);
        if l >= h {
            continue;
        }
        if out.base.cells == max_cells {
            refuse(&mut out, "third-derivative-work-limit");
            return Ok(out);
        }
        out.base.cells += 1;
        let result = (|| -> Result<Interval> {
            let controls =
                crate::curve_distance::restricted_controls(c, span, Interval::new(l, h)?)?;
            let width = Interval::point(h).sub(Interval::point(l))?;
            let numerator = controls.iter().map(|p| p[0]).collect::<Vec<_>>();
            let weights = controls.iter().map(|p| p[3]).collect::<Vec<_>>();
            let weight = hull(&weights);
            let relative = hull(&numerator).div(weight)?;
            let n1 = derivative(&numerator, c.degree, width)?;
            let w1 = derivative(&weights, c.degree, width)?;
            let n2 = derivative(&n1, c.degree.saturating_sub(1), width)?;
            let w2 = derivative(&w1, c.degree.saturating_sub(1), width)?;
            let n3 = derivative(&n2, c.degree.saturating_sub(2), width)?;
            let w3 = derivative(&w2, c.degree.saturating_sub(2), width)?;
            let jets = span_jets(c, span, l, h)?;
            hull(&n3)
                .sub(jets[2].mul(hull(&w1))?.mul(Interval::point(3.))?)?
                .sub(jets[1].mul(hull(&w2))?.mul(Interval::point(3.))?)?
                .sub(relative.mul(hull(&w3))?)?
                .div(weight)
        })();
        let Ok(v) = result else {
            refuse(&mut out, "third-derivative-numeric-unresolved");
            return Ok(out);
        };
        lower = lower.min(v.lo);
        upper = upper.max(v.hi);
    }
    out.third = Some([lower, upper]);
    Ok(out)
}
#[test]
fn original_cubic_and_rational_third_derivatives_share_budget() {
    let cubic = Curve {
        degree: 3,
        knots: vec![2., 2., 2., 2., 5., 5., 5., 5.],
        control_points: vec![vec![0.; 3], vec![0.; 3], vec![0.; 3], vec![1., 0., 0.]],
        weights: vec![1.; 4],
        periodic: false,
    };
    let report = certify_third_traversal(&cubic, [0.25, 0.5], 100).unwrap();
    assert_eq!(report.base.status, Status::Certified);
    assert!(report.base.single_span);
    let bound = report.third.unwrap();
    assert!(bound[0] <= 2. / 9. && 2. / 9. <= bound[1]);
    let rational = Curve {
        degree: 1,
        knots: vec![2., 2., 5., 5.],
        control_points: vec![vec![0.; 3], vec![1., 0., 0.]],
        weights: vec![1., 2.],
        periodic: false,
    };
    let report = certify_third_traversal(&rational, [0.25, 0.5], 100).unwrap();
    let bound = report.third.unwrap();
    for t in [0.25_f64, 0.375, 0.5] {
        let expected = 12. / (27. * (1. + t).powi(4));
        assert!(bound[0] <= expected && expected <= bound[1]);
    }
    let refused = certify_third_traversal(&rational, [0.25, 0.5], report.base.cells - 1).unwrap();
    assert_eq!(refused.base.status, Status::Unresolved);
    assert!(refused.third.is_none() && refused.base.first.is_none());
}
