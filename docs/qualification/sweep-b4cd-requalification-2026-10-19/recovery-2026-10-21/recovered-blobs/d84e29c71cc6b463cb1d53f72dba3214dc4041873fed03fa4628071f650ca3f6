//! Point-safe original rational derivative using full-span homogeneous controls.
use super::*;
#[derive(Clone, Debug)]
pub struct PointFirstReport {
    pub cells: usize,
    pub first: Option<[f64; 2]>,
}
pub(super) fn evaluate(mut p: Vec<Interval>, t: Interval) -> Result<Interval> {
    let beta = Interval::point(1.).sub(t)?.intersect(0., 1.)?;
    for n in (1..p.len()).rev() {
        for i in 0..n {
            p[i] = p[i].mul(beta)?.add(p[i + 1].mul(t)?)?;
        }
    }
    Ok(p[0])
}
pub fn certify_first_point(c: &Curve, t: f64, max_cells: usize) -> Result<PointFirstReport> {
    c.validate()?;
    check(
        max_cells <= 100000
            && t.is_finite()
            && (0. ..=1.).contains(&t)
            && c.control_points
                .iter()
                .all(|p| p.len() == 3 && p[1] == 0. && p[2] == 0.),
        "Invalid scalar point derivative input",
    )?;
    let mut out = PointFirstReport {
        cells: 0,
        first: None,
    };
    let [a, b] = c.domain();
    let mapped = Interval::point(a)
        .add(
            Interval::point(b)
                .sub(Interval::point(a))?
                .mul(Interval::point(t))?,
        )?
        .intersect(a, b)?;
    let mut lower = f64::INFINITY;
    let mut upper = f64::NEG_INFINITY;
    for span in c.degree..c.control_points.len() {
        let a = c.knots[span];
        let b = c.knots[span + 1];
        if a >= b || mapped.hi < a || mapped.lo > b {
            continue;
        }
        if out.cells == max_cells {
            return Ok(out);
        }
        out.cells += 1;
        let result = (|| -> Result<Interval> {
            let controls =
                crate::curve_distance::restricted_controls(c, span, Interval::new(a, b)?)?;
            let width = Interval::point(b).sub(Interval::point(a))?;
            let local = mapped
                .intersect(a, b)?
                .sub(Interval::point(a))?
                .div(width)?
                .intersect(0., 1.)?;
            let numerator = controls.iter().map(|p| p[0]).collect::<Vec<_>>();
            let weights = controls.iter().map(|p| p[3]).collect::<Vec<_>>();
            let n1 = derivative(&numerator, c.degree, width)?;
            let w1 = derivative(&weights, c.degree, width)?;
            let w = evaluate(weights, local)?;
            let relative = evaluate(numerator, local)?.div(w)?;
            evaluate(n1, local)?
                .sub(relative.mul(evaluate(w1, local)?)?)?
                .div(w)
        })();
        let Ok(v) = result else {
            return Ok(out);
        };
        lower = lower.min(v.lo);
        upper = upper.max(v.hi);
    }
    out.first = Some([lower, upper]);
    Ok(out)
}
#[test]
fn original_point_derivative_contains_rational_stations_and_budget_refusal() {
    let c = Curve {
        degree: 1,
        knots: vec![2., 2., 5., 5.],
        control_points: vec![vec![0.; 3], vec![1., 0., 0.]],
        weights: vec![1., 2.],
        periodic: false,
    };
    for t in [0.0_f64, 0.13, 0.375, 0.5, 0.87, 1.] {
        let report = certify_first_point(&c, t, 1).unwrap();
        let range = report.first.unwrap();
        let expected = 2. / (3. * (1. + t).powi(2));
        assert!(range[0] <= expected && expected <= range[1]);
        assert!(range[1] - range[0] < 1e-10);
        assert!(certify_first_point(&c, t, 0).unwrap().first.is_none());
    }
}
