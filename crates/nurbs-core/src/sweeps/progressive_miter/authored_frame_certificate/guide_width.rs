//! Original-law rail distance jets for contact fitting. Normal-plane contact,
//! initial anchor and positive transformed anchor remain separate premises.
use super::*;

#[derive(Clone, Debug)]
pub struct GuideWidthReport {
    pub status: Status,
    pub cells: usize,
    pub single_span: bool,
    pub value: Option<[f64; 2]>,
    pub first: Option<[f64; 2]>,
    pub second: Option<[f64; 2]>,
}

pub fn certify_guide_width(
    path: &Curve,
    guide: &Curve,
    traversal: [f64; 2],
    max_cells: usize,
) -> Result<GuideWidthReport> {
    check(max_cells <= 100000, "Guide width work exceeds100000 cells")?;
    let mut out = GuideWidthReport {
        status: Status::Unresolved,
        cells: 0,
        single_span: false,
        value: None,
        first: None,
        second: None,
    };
    let p = vector_certificate::certify_traversal(path, traversal, max_cells, false)?;
    out.cells += p.cells;
    if p.status != Status::Certified {
        return Ok(out);
    }
    let g = vector_certificate::certify_traversal(guide, traversal, max_cells - out.cells, false)?;
    out.cells += g.cells;
    if g.status != Status::Certified {
        return Ok(out);
    }
    let pjet = jet(path, &p)?;
    let gjet = jet(guide, &g)?;
    let v = sub(gjet.v, pjet.v)?;
    let d = sub(gjet.d, pjet.d)?;
    let dd = sub(gjet.dd, pjet.dd)?;
    let squared = square(v[0])?.add(square(v[1])?)?.add(square(v[2])?)?;
    let width = I::new(
        squared.lo.max(0.).sqrt().next_down().max(0.),
        squared.hi.sqrt().next_up(),
    )?;
    if width.lo <= 0. {
        return Ok(out);
    }
    let first = dot(v, d)?.div(width)?;
    let second = dot(d, d)?
        .add(dot(v, dd)?)?
        .sub(square(first)?)?
        .div(width)?;
    out.status = Status::Certified;
    out.single_span = p.single_span && g.single_span;
    out.value = Some([width.lo, width.hi]);
    out.first = Some([first.lo, first.hi]);
    out.second = Some([second.lo, second.hi]);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_independent_domain_width_jets_and_work_refusal() {
        let mut path = crate::primitives::line([0.; 3], [0., 0., 10.]).unwrap();
        path.knots = vec![2., 2., 5., 5.];
        let mut guide = crate::primitives::line([1., 0., 0.], [1., 1., 10.]).unwrap();
        guide.knots = vec![17., 17., 19., 19.];
        let report = certify_guide_width(&path, &guide, [0.25, 0.5], 100).unwrap();
        assert_eq!(report.status, Status::Certified);
        assert!(report.single_span);
        for t in [0.25_f64, 0.375, 0.5] {
            let w = (1. + t * t).sqrt();
            for (bound, expected) in [
                (report.value.unwrap(), w),
                (report.first.unwrap(), t / w),
                (report.second.unwrap(), 1. / w.powi(3)),
            ] {
                assert!(
                    bound[0] <= expected && expected <= bound[1],
                    "{bound:?} excludes {expected}"
                );
            }
        }
        let short = certify_guide_width(&path, &guide, [0.25, 0.5], report.cells - 1).unwrap();
        assert_eq!(short.status, Status::Unresolved);
        assert!(short.value.is_none() && short.first.is_none() && short.second.is_none());
        assert!(short.cells <= report.cells - 1);
        let zero = certify_guide_width(&path, &path, [0.25, 0.5], 100).unwrap();
        assert_eq!(zero.status, Status::Unresolved);
        assert!(zero.value.is_none());
    }
}
