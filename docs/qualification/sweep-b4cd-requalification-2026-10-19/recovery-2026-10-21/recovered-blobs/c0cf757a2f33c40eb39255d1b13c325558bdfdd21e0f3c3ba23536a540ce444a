//! Whole-domain sufficient proof of nonzero one-sided curve tangents.
//! Failure to separate a derivative component is unresolved, not singularity.
use crate::{Result, check, curve::Curve, curve_jets, distance_bounds::Interval};
#[derive(Clone, Debug)]
pub struct Report {
    pub spanwise_regular: bool,
    pub cells: usize,
    pub proven_cells: usize,
    pub unresolved: Vec<[f64; 2]>,
    pub reason: Option<&'static str>,
}
/// Counts all created cells, including the original knot-span grid. Covers
/// each closed span, so authored corners include both one-sided derivatives.
/// Does not promise tangent continuity across a knot or periodic seam.
pub fn inspect(curve: &Curve, max_cells: usize) -> Result<Report> {
    Ok(inspect_speed(curve,max_cells)?.0)
}
/// Original parameter speed lower bound from the same complete span cover.
/// Any separated component bounds the Euclidean speed from below. No norm
/// midpoint, sampled derivative or unproved partial cover is returned.
pub(crate) fn inspect_speed(curve: &Curve, max_cells: usize) -> Result<(Report,Option<f64>)> {
    curve.validate()?;
    check(max_cells <= 100000, "Curve regularity budget exceeds100000")?;
    let mut pending: Vec<_> = crate::certificates::audit::curve_span_domains(curve);
    let mut report = Report {
        spanwise_regular: false,
        cells: 0,
        proven_cells: 0,
        unresolved: Vec::new(),
        reason: None,
    };
    if pending.len() > max_cells {
        report.unresolved = pending.into_iter().map(|(_, d)| d).collect();
        report.reason = Some("cell-budget-exhausted");
        return Ok((report,None));
    }
    let mut minimum_speed=f64::INFINITY;
    let mut speed_converted=true;
    report.cells = pending.len();
    while let Some((span, domain)) = pending.pop() {
        let jets = curve_jets::enclose(curve, span, domain);
        let Ok(jets) = jets else {
            report.unresolved.push(domain);
            report.reason = Some("numeric-enclosure-unresolved");
            continue;
        };
        let lower=jets[1].iter().map(|v|if v.lo>0.{v.lo}else if v.hi<0.{-v.hi}else{0.}).fold(0_f64,f64::max);
        if lower>0. {
            // Enclosed jets use the restricted cell's x in [0,1]. Divide
            // by its outward source-parameter width before using arc residuals.
            match Interval::point(domain[1]).sub(Interval::point(domain[0]))
                .and_then(|width|Interval::point(lower).div(width)) {
                Ok(speed) if speed.lo>0.&&speed.lo.is_finite()=>minimum_speed=minimum_speed.min(speed.lo),
                _=>speed_converted=false,
            }
            report.proven_cells += 1;
            continue;
        }
        let middle = domain[0] * 0.5 + domain[1] * 0.5;
        if report.cells + 2 > max_cells || !(domain[0] < middle && middle < domain[1]) {
            report.unresolved.push(domain);
            report.reason = Some("tangent-not-separated-within-budget");
            continue;
        }
        pending.push((span, [domain[0], middle]));
        pending.push((span, [middle, domain[1]]));
        report.cells += 2;
    }
    report.spanwise_regular = report.unresolved.is_empty();
    let speed=(report.spanwise_regular&&speed_converted&&minimum_speed.is_finite()&&minimum_speed>0.).then_some(minimum_speed);
    Ok((report,speed))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn whole_original_speed_lower_is_quantitative_and_never_partial(){
        let line=Curve{degree:1,knots:vec![2.,2.,6.,6.],
            control_points:vec![vec![0.;3],vec![12.,0.,0.]],weights:vec![1.;2],periodic:false};
        let (report,lower)=inspect_speed(&line,1).unwrap();
        assert!(report.spanwise_regular);assert!(lower.unwrap()>2.99&&lower.unwrap()<=3.);
        assert!(inspect_speed(&line,0).unwrap().1.is_none());
        let curved=crate::paths::bezier(vec![vec![0.,0.,0.],vec![1.,0.,0.],vec![1.,1.,0.]],None).unwrap();
        let (cover,lower)=inspect_speed(&curved,10000).unwrap();
        assert!(cover.spanwise_regular&&lower.unwrap()>0.);
        assert!(inspect_speed(&curved,cover.cells-1).unwrap().1.is_none());
        let singular=crate::paths::bezier(vec![vec![0.,0.,0.],vec![1.,0.,0.],vec![0.,0.,0.]],None).unwrap();
        assert!(inspect_speed(&singular,1000).unwrap().1.is_none());
    }
    #[test]
    fn line_and_c0_corner_cover_both_one_sided_tangents() {
        let c = Curve {
            degree: 1,
            knots: vec![3., 3., 5., 7., 7.],
            control_points: vec![vec![0., 0., 0.], vec![1., 0., 0.], vec![1., 1., 0.]],
            weights: vec![1.; 3],
            periodic: false,
        };
        assert!(inspect(&c, 2).unwrap().spanwise_regular);
        let u = inspect(&c, 1).unwrap();
        assert!(!u.spanwise_regular && u.cells == 0 && u.unresolved.len() == 2);
    }
    #[test]
    fn stationary_interior_and_exhaustion_never_prove_regularity() {
        let c = crate::paths::bezier(
            vec![vec![0., 0., 0.], vec![1., 0., 0.], vec![0., 0., 0.]],
            None,
        )
        .unwrap();
        for budget in [0, 1, 63] {
            let r = inspect(&c, budget).unwrap();
            assert!(!r.spanwise_regular && !r.unresolved.is_empty());
            assert!(r.cells <= budget);
        }
        let c = crate::paths::bezier(
            vec![vec![0., 0., 0.], vec![0., 1., 0.], vec![1., 1., 0.]],
            Some(vec![1., 0.7, 1.]),
        )
        .unwrap();
        assert!(inspect(&c, 255).unwrap().spanwise_regular);
    }
}
