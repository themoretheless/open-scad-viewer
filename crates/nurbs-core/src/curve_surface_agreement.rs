//! Bounded, conservative verification of C(t) = S(P(t)) over a full interval.
//! Positive rational hulls cover every knot span. Exhaustion is inconclusive.
use crate::distance_bounds::{Interval, box_distance};
use crate::{Result, check, curve::Curve, surface::Surface};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    WithinTolerance,
    Mismatch,
    Unresolved,
}
#[derive(Clone, Debug)]
pub struct Report {
    pub status: Status,
    pub cells: usize,
    /// Parameter in the normalized, forward pcurve traversal.
    pub witness: Option<f64>,
    /// Certified distance enclosure at the witness (only for Mismatch).
    pub witness_distance: Option<[f64; 2]>,
}

pub(crate) fn curve_bounds(c: &Curve, t: Interval) -> Result<Vec<Interval>> {
    let mut bounds = vec![[f64::INFINITY, f64::NEG_INFINITY]; c.control_points[0].len()];
    for span in c.degree..c.control_points.len() {
        if c.knots[span] >= c.knots[span + 1] {
            continue;
        }
        let lo = t.lo.max(c.knots[span]);
        let hi = t.hi.min(c.knots[span + 1]);
        if lo > hi {
            continue;
        }
        for (out, b) in bounds.iter_mut().zip(crate::curve_distance::enclosure(
            c,
            span,
            Interval::new(lo, hi)?,
        )?) {
            out[0] = out[0].min(b.lo);
            out[1] = out[1].max(b.hi);
        }
    }
    bounds
        .into_iter()
        .map(|[lo, hi]| Interval::new(lo, hi))
        .collect()
}

pub(crate) fn mapped(c: &Curve, t: Interval, reversed: bool) -> Result<Interval> {
    let a = c.knots[c.degree];
    let b = c.knots[c.control_points.len()];
    let t = if reversed {
        Interval::point(1.).sub(t)?.intersect(0., 1.)?
    } else {
        t
    };
    if a == 0. && b == 1. {
        return Ok(t);
    }
    if t.lo == 0. && t.hi == 0. {
        return Ok(Interval::point(a));
    }
    if t.lo == 1. && t.hi == 1. {
        return Ok(Interval::point(b));
    }
    Interval::point(a)
        .add(Interval::point(b).sub(Interval::point(a))?.mul(t)?)?
        .intersect(a, b)
}

fn distance(
    c: &Curve,
    p: &Curve,
    s: &Surface,
    t: Interval,
    reversed: bool,
) -> Result<Option<(f64, f64)>> {
    let edge = curve_bounds(c, mapped(c, t, reversed)?)?;
    let uv = curve_bounds(p, mapped(p, t, false)?)?;
    let domain = [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ];
    let Some(us) = crate::periodic_chart::charts(uv[0], domain[0], s.periodic_u)? else {
        return Ok(None);
    };
    let Some(vs) = crate::periodic_chart::charts(uv[1], domain[1], s.periodic_v)? else {
        return Ok(None);
    };
    let mut bounds = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
    for u in &us {
        for v in &vs {
            let patch = crate::surface_distance::rectangle_bounds(
                s,
                [[u.range.lo, u.range.hi], [v.range.lo, v.range.hi]],
            )?;
            for k in 0..3 {
                bounds[k][0] = bounds[k][0].min(patch[k][0]);
                bounds[k][1] = bounds[k][1].max(patch[k][1]);
            }
        }
    }
    let surface = bounds
        .into_iter()
        .map(|[lo, hi]| Interval::new(lo, hi))
        .collect::<Result<Vec<_>>>()?;
    Ok(Some(box_distance(&edge, &surface)?))
}

/// Covers the natural domains of both curves using a common normalized
/// parameter. Reversal applies to the 3D curve, as for a reversed coedge.
/// Periodic surface axes use their periodic extension for lifted UV values.
/// Both chart images remain covered when a parameter interval crosses a seam.
pub fn verify(
    c: &Curve,
    p: &Curve,
    s: &Surface,
    reversed: bool,
    tolerance: f64,
    max_cells: usize,
) -> Result<Report> {
    c.validate()?;
    p.validate()?;
    s.validate()?;
    check(
        c.control_points[0].len() == 3 && p.control_points[0].len() == 2,
        "Agreement needs a 3D curve and 2D pcurve",
    )?;
    check(
        tolerance.is_finite() && tolerance > 0.,
        "Agreement tolerance must be positive",
    )?;
    check(
        max_cells > 0 && max_cells <= 100_000,
        "Agreement cell budget must be in 1..100000",
    )?;
    let mut report = Report {
        status: Status::Unresolved,
        cells: 0,
        witness: None,
        witness_distance: None,
    };
    // Keep the common parameter correlated before trying Cartesian boxes.
    // An unavailable or numerically inconclusive optional bound falls back to
    // the full-span interval traversal; it never turns into a success.
    if let Ok(Some(upper)) = crate::curve_surface_composition::upper(c, p, s, reversed) {
        if upper <= tolerance {
            report.status = Status::WithinTolerance;
            report.cells = 1;
            return Ok(report);
        }
    }
    let mut stack = vec![[0., 1.]];
    while let Some([lo, hi]) = stack.pop() {
        if report.cells == max_cells {
            return Ok(report);
        }
        report.cells += 1;
        if let Ok(Some(upper)) =
            crate::curve_surface_composition::upper_cell(c, p, s, reversed, [lo, hi])
        {
            if upper <= tolerance {
                continue;
            }
        }
        if let Some((_, upper)) = distance(c, p, s, Interval::new(lo, hi)?, reversed)? {
            if upper <= tolerance {
                continue;
            }
        }
        let mid = lo + (hi - lo) * 0.5;
        if let Some((lower, upper)) = distance(c, p, s, Interval::point(mid), reversed)? {
            if lower > tolerance {
                report.status = Status::Mismatch;
                report.witness = Some(mid);
                report.witness_distance = Some([lower, upper]);
                return Ok(report);
            }
        }
        if mid == lo || mid == hi {
            return Ok(report);
        }
        stack.push([mid, hi]);
        stack.push([lo, mid]);
    }
    report.status = Status::WithinTolerance;
    Ok(report)
}

/// Exact identity on one Bezier chart, over normalized curve traversal.
/// None means this representation or its chart-domain inclusion is unproven.
/// A returned decision distinguishes exact equality, difference and work limits.
pub fn verify_exact(c: &Curve, p: &Curve, s: &Surface, reversed: bool, max_work: u64)
    -> Result<Option<cad_predicates::BezierIdentityDecision>> {
    use cad_predicates::{AuthoredScalar, Limits, PredicateContext, SourceArena, ToleranceContext};
    c.validate()?;p.validate()?;s.validate()?;
    check(c.control_points[0].len()==3 && p.control_points[0].len()==2,"Agreement needs a 3D curve and 2D pcurve")?;
    let bezier=|knots:&[f64],degree:usize,n:usize| n==degree+1
        && knots[..=degree].iter().all(|x|*x==knots[degree])
        && knots[n..].iter().all(|x|*x==knots[n]);
    if c.periodic || p.periodic || s.periodic_u || s.periodic_v
        || c.degree>32 || p.degree>8 || s.degree_u>8 || s.degree_v>8
        || !bezier(&c.knots,c.degree,c.control_points.len())
        || !bezier(&p.knots,p.degree,p.control_points.len())
        || !bezier(&s.knots_u,s.degree_u,s.control_points.len())
        || !bezier(&s.knots_v,s.degree_v,s.control_points[0].len()) {return Ok(None);}
    let domain=[[s.knots_u[s.degree_u],s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v],s.knots_v[s.control_points[0].len()]]];
    // Positive rational weights put the entire pcurve in its control hull.
    if p.control_points.iter().any(|v|(0..2).any(|k|v[k]<domain[k][0]||v[k]>domain[k][1])) {return Ok(None);}
    let mut values=Vec::new();
    for i in 0..c.control_points.len() {
        let i=if reversed {c.control_points.len()-1-i}else{i};
        values.extend(c.control_points[i].iter().copied());values.push(c.weights[i]);
    }
    for (v,w) in p.control_points.iter().zip(&p.weights){values.extend(v.iter().copied());values.push(*w);}
    for (row,weights) in s.control_points.iter().zip(&s.weights){for (v,w) in row.iter().zip(weights){values.extend(v.iter().copied());values.push(*w);}}
    values.extend(domain.into_iter().flatten());
    let source=SourceArena::authored("curve-surface-agreement",1,values.into_iter().map(|v|AuthoredScalar::Binary64Bits(v.to_bits())).collect())
        .map_err(|_|crate::Error::new("NURBS_INVALID_INPUT","Invalid authored composition source"))?;
    let mut index=0;let mut leaf=||{let r=source.leaf(index).unwrap();index+=1;r};
    let cc:Vec<_>=(0..c.control_points.len()).map(|_|std::array::from_fn(|_|leaf())).collect();
    let pp:Vec<_>=(0..p.control_points.len()).map(|_|std::array::from_fn(|_|leaf())).collect();
    let ss:Vec<Vec<_>>=s.control_points.iter().map(|row|row.iter().map(|_|std::array::from_fn(|_|leaf())).collect()).collect();
    let dd=std::array::from_fn(|_|std::array::from_fn(|_|leaf()));
    let tolerance=ToleranceContext::default_valid();
    let mut ctx=PredicateContext::new(&source,&tolerance,Limits{max_work,..Limits::default()},None);
    cad_predicates::rational_bezier_composition_identity(&mut ctx,&cc,&pp,&ss,dd)
        .map(Some).map_err(|_|crate::Error::new("NURBS_INVALID_INPUT","Invalid composition identity request"))
}
#[cfg(test)]
mod tests {
    use super::*;
    fn plane() -> Surface {
        Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 0.]],
                vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        }
    }
    fn line(points: Vec<Vec<f64>>) -> Curve {
        Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            weights: vec![1.; 2],
            control_points: points,
            periodic: false,
        }
    }
    #[test]
    fn exact_identity_leaves_multispan_definitions_unproven() {
        let c=Curve {degree:1,knots:vec![0.,0.,0.5,1.,1.],
            control_points:vec![vec![0.,0.,0.],vec![0.5,0.5,0.],vec![1.,1.,0.]],
            weights:vec![1.;3],periodic:false};
        let p=line(vec![vec![0.,0.],vec![1.,1.]]);
        assert!(verify_exact(&c,&p,&plane(),false,1000000).unwrap().is_none());
        assert_eq!(verify(&c,&p,&plane(),false,1e-6,10000).unwrap().status,Status::WithinTolerance);
    }
    #[test]
    fn exact_identity_is_distinct_from_tolerance_and_preserves_reversal() {
        use cad_predicates::BezierIdentity;
        let p=line(vec![vec![0.,0.],vec![1.,1.]]);
        let mut c=line(vec![vec![0.,0.,0.],vec![1.,1.,0.]]);
        assert_eq!(verify_exact(&c,&p,&plane(),false,1000000).unwrap().unwrap().outcome,BezierIdentity::Equal);
        c.control_points.reverse();
        assert_eq!(verify_exact(&c,&p,&plane(),true,1000000).unwrap().unwrap().outcome,BezierIdentity::Equal);
        c.control_points[0][2]=1e-12;
        assert_eq!(verify(&c,&p,&plane(),true,1e-6,1000).unwrap().status,Status::WithinTolerance);
        assert_eq!(verify_exact(&c,&p,&plane(),true,1000000).unwrap().unwrap().outcome,BezierIdentity::Different);
        assert!(matches!(verify_exact(&c,&p,&plane(),true,0).unwrap().unwrap().outcome,BezierIdentity::Indeterminate(_)));
        let outside=line(vec![vec![-1.,0.],vec![1.,1.]]);
        assert!(verify_exact(&c,&outside,&plane(),false,1000000).unwrap().is_none());
    }
    #[test]
    fn matching_reversed_and_exhausted() {
        let c = line(vec![vec![0., 0., 0.], vec![1., 0., 0.]]);
        let p = line(vec![vec![0., 0.], vec![1., 0.]]);
        assert_eq!(
            verify(&c, &p, &plane(), false, 0.01, 1024).unwrap().status,
            Status::WithinTolerance
        );
        assert_eq!(
            verify(&c, &p, &plane(), false, 1e-9, 1).unwrap().status,
            Status::WithinTolerance
        );
        let mut reversed = c.clone();
        reversed.control_points.reverse();
        assert_eq!(
            verify(&reversed, &p, &plane(), true, 0.01, 1024)
                .unwrap()
                .status,
            Status::WithinTolerance
        );
        assert_eq!(
            verify(&reversed, &p, &plane(), false, 0.01, 1024)
                .unwrap()
                .status,
            Status::Mismatch
        );
    }
    #[test]
    fn rational_multispan_and_nonbinary_domains() {
        let c = Curve {
            degree: 1,
            knots: vec![0.1, 0.1, 0.37, 0.9, 0.9],
            weights: vec![1., 2., 0.5],
            control_points: vec![vec![0.1, 0.2, 0.], vec![0.4, 0.7, 0.], vec![0.8, 0.3, 0.]],
            periodic: false,
        };
        let mut p = c.clone();
        for cp in &mut p.control_points {
            cp.pop();
        }
        assert_eq!(
            verify(&c, &p, &plane(), false, 1e-9, 1).unwrap().status,
            Status::Unresolved
        );
        assert_eq!(
            verify(&c, &p, &plane(), false, 1e-9, 4096).unwrap().status,
            Status::WithinTolerance
        );
    }
    #[test]
    fn pcurve_crosses_surface_knots_at_strict_tolerance() {
        let mut surface = plane();
        surface.knots_u = vec![0., 0., 0.3, 1., 1.];
        surface.control_points = vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![0.3, 0., 0.], vec![0.3, 1., 0.]],
            vec![vec![1., 0., 0.], vec![1., 1., 0.]],
        ];
        surface.weights = vec![vec![1.; 2]; 3];
        let c = Curve {
            degree: 1,
            knots: vec![0.1, 0.1, 0.37, 0.9, 0.9],
            weights: vec![1., 2., 0.5],
            control_points: vec![vec![0.1, 0.2, 0.], vec![0.4, 0.7, 0.], vec![0.8, 0.3, 0.]],
            periodic: false,
        };
        let mut p = c.clone();
        for cp in &mut p.control_points {
            cp.pop();
        }
        let report = verify(&c, &p, &surface, false, 1e-9, 4096).unwrap();
        assert_eq!(report.status, Status::WithinTolerance);
        assert!(report.cells < 4096);
    }

    #[test]
    fn rational_curved_support_is_verified_without_planar_assumption() {
        let weights = vec![1., 0.5_f64.sqrt(), 1.];
        let c = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![1., 0., 0.5], vec![1., 1., 0.5], vec![0., 1., 0.5]],
            weights: weights.clone(),
            periodic: false,
        };
        let s = Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: c.knots.clone(),
            knots_v: vec![0., 0., 1., 1.],
            control_points: c
                .control_points
                .iter()
                .map(|p| vec![vec![p[0], p[1], 0.], vec![p[0], p[1], 1.]])
                .collect(),
            weights: weights.into_iter().map(|w| vec![w, w]).collect(),
            periodic_u: false,
            periodic_v: false,
        };
        let p = line(vec![vec![0., 0.5], vec![1., 0.5]]);
        assert_eq!(
            verify(&c, &p, &s, false, 1e-9, 1).unwrap().status,
            Status::WithinTolerance
        );
    }

    #[test]
    fn rational_pcurve_composition_preserves_the_shared_parameter() {
        let mut s = plane();
        s.control_points[1][1][2] = 1.; // S(u,v)=(u,v,uv)
        let mut p = line(vec![vec![0.25, 0.25], vec![0.75, 0.75]]);
        p.weights = vec![1., 2.];
        // Homogeneous Bernstein product: U*W, U*W, U*U, W*W.
        let mut c = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            periodic: false,
            control_points: vec![
                vec![0.25, 0.25, 0.0625],
                vec![0.5, 0.5, 0.1875],
                vec![0.75, 0.75, 0.5625],
            ],
            weights: vec![1., 2., 4.],
        };
        let report = verify(&c, &p, &s, false, 1e-9, 1).unwrap();
        assert_eq!(report.status, Status::WithinTolerance);
        assert_eq!(report.cells, 1);
        c.control_points.reverse();
        c.weights.reverse();
        assert_eq!(
            verify(&c, &p, &s, true, 1e-9, 1).unwrap().status,
            Status::WithinTolerance
        );
        c.control_points[1][2] += 0.01;
        assert_eq!(
            verify(&c, &p, &s, true, 1e-9, 100).unwrap().status,
            Status::Mismatch
        );
    }

    #[test]
    fn periodic_lift_crosses_seam_and_negative_periods() {
        let surface = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![-1., 0., 1., 2., 3., 4.],
            knots_v: vec![0., 0., 1., 1.],
            weights: vec![vec![1.; 2]; 4],
            control_points: [[0., 0.], [1., 0.], [0., 1.], [0., 0.]]
                .into_iter()
                .map(|xy| vec![vec![xy[0], xy[1], 0.], vec![xy[0], xy[1], 1.]])
                .collect(),
            periodic_u: true,
            periodic_v: false,
        };
        let mut c = Curve {
            degree: 1,
            knots: vec![0., 0., 0.5, 1., 1.],
            weights: vec![1.; 3],
            control_points: vec![vec![0., 0.5, 0.5], vec![0., 0., 0.5], vec![0.5, 0., 0.5]],
            periodic: false,
        };
        for shift in [-6., 0., 6.] {
            let p = line(vec![vec![2.5 + shift, 0.5], vec![3.5 + shift, 0.5]]);
            let report = verify(&c, &p, &surface, false, 1e-9, 4096).unwrap();
            assert_eq!(
                report.status,
                Status::WithinTolerance,
                "shift={shift}, cells={}",
                report.cells
            );
        }
        let p = line(vec![vec![2.5, 0.5], vec![3.5, 0.5]]);
        c.control_points[1][2] += 0.01;
        let report = verify(&c, &p, &surface, false, 1e-9, 4096).unwrap();
        assert_eq!(report.status, Status::Mismatch);
        assert!(report.witness_distance.unwrap()[0] > 1e-9);
    }

    #[test]
    fn both_periodic_axes_cross_seams_together() {
        let f = [0., 1., 0.5, 0.];
        let surface = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![-1., 0., 1., 2., 3., 4.],
            knots_v: vec![-1., 0., 1., 2., 3., 4.],
            weights: vec![vec![1.; 4]; 4],
            control_points: f
                .iter()
                .map(|&x| f.iter().map(|&y| vec![x, y, x * y]).collect())
                .collect(),
            periodic_u: true,
            periodic_v: true,
        };
        let c = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 0.5, 0.5, 1., 1., 1.],
            weights: vec![1.; 5],
            periodic: false,
            control_points: vec![
                vec![0.25, 0.25, 0.0625],
                vec![0.125, 0.125, 0.],
                vec![0., 0., 0.],
                vec![0.25, 0.25, 0.],
                vec![0.5, 0.5, 0.25],
            ],
        };
        let p = line(vec![vec![-0.5, 5.5], vec![0.5, 6.5]]);
        assert_eq!(
            verify(&c, &p, &surface, false, 1e-9, 4096).unwrap().status,
            Status::WithinTolerance
        );
    }

    #[test]
    fn rational_quadratic_periodic_surface_seam() {
        let xy = [[1., 0.], [0., 1.], [-1., 0.], [0., -1.], [1., 0.], [0., 1.]];
        let weights = [1., 2., 1., 2., 1., 2.];
        let surface = Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![-2., -1., 0., 1., 2., 3., 4., 5., 6.],
            knots_v: vec![0., 0., 1., 1.],
            weights: weights.iter().map(|&w| vec![w, w]).collect(),
            control_points: xy
                .iter()
                .map(|p| vec![vec![p[0], p[1], 0.], vec![p[0], p[1], 1.]])
                .collect(),
            periodic_u: true,
            periodic_v: false,
        };
        fn average(a: [f64; 4], b: [f64; 4]) -> [f64; 4] {
            std::array::from_fn(|k| (a[k] + b[k]) * 0.5)
        }
        let h: Vec<[f64; 4]> = xy
            .iter()
            .zip(weights)
            .map(|(p, w)| [p[0] * w, p[1] * w, 0.5 * w, w])
            .collect();
        let left = [average(h[3], h[4]), h[4], average(h[4], h[5])];
        let right = [average(h[0], h[1]), h[1], average(h[1], h[2])];
        let controls = [
            average(average(left[0], left[1]), average(left[1], left[2])),
            average(left[1], left[2]),
            left[2],
            average(right[0], right[1]),
            average(average(right[0], right[1]), average(right[1], right[2])),
        ];
        let c = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 0.5, 0.5, 1., 1., 1.],
            weights: controls.iter().map(|p| p[3]).collect(),
            control_points: controls
                .iter()
                .map(|p| (0..3).map(|k| p[k] / p[3]).collect())
                .collect(),
            periodic: false,
        };
        let p = line(vec![vec![3.5, 0.5], vec![4.5, 0.5]]);
        assert_eq!(
            verify(&c, &p, &surface, false, 1e-9, 4096).unwrap().status,
            Status::WithinTolerance
        );
    }

    #[test]
    fn departure_has_enclosed_witness_and_invalid_uv_is_unresolved() {
        let c = line(vec![vec![0., 0., 0.1], vec![1., 0., 0.1]]);
        let p = line(vec![vec![0., 0.], vec![1., 0.]]);
        let r = verify(&c, &p, &plane(), false, 1e-6, 100).unwrap();
        assert_eq!(r.status, Status::Mismatch);
        let [lo, hi] = r.witness_distance.unwrap();
        assert!(lo <= 0.1 && hi >= 0.1 && lo > 1e-6);
        let outside = line(vec![vec![-1., 0.], vec![-0.5, 0.]]);
        assert_eq!(
            verify(&c, &outside, &plane(), false, 1e-6, 100)
                .unwrap()
                .status,
            Status::Unresolved
        );
        assert!(verify(&c, &p, &plane(), false, 0., 100).is_err());
    }
}
