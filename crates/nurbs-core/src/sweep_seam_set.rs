//! Native ownership of complete declared seam sets and shared exact-work budgets.
use crate::{Result, check, continuity::{self, ExactStripJetReport}, surface::Surface};

pub struct Seam {
    pub patches: [usize; 2],
    pub boundaries: [String; 2],
    pub order: usize,
    pub normal_scale: f64,
}
pub struct Report {
    pub certified: bool,
    pub order: Option<usize>,
    pub work: u64,
    pub unresolved: Vec<usize>,
    pub seams: Vec<ExactStripJetReport>,
}
pub fn inspect(patches: &[Surface], seams: &[Seam], max_work: u64, projective: bool) -> Result<Report> {
    check(max_work <= if projective {2_000_000} else {1_000_000}, "Invalid exact seam work budget")?;
    for seam in seams {
        check(seam.patches.iter().all(|&i| i < patches.len()), "Invalid exact seam patch index")?;
        check(matches!(seam.order, 1 | 2), "Invalid exact seam order")?;
        check(seam.normal_scale.is_finite() && seam.normal_scale > 0., "Invalid exact seam normal scale")?;
        check(seam.boundaries.iter().all(|s| matches!(s.as_str(), "uMin" | "uMax" | "vMin" | "vMax")), "Invalid exact seam boundary")?;
    }
    let mut work = 0;
    let mut reports = Vec::with_capacity(seams.len());
    for seam in seams {
        let remaining = max_work - work;
        let predicate = if projective {continuity::inspect_surface_projective_strip_jets} else {continuity::inspect_surface_exact_strip_jets};
        let report = predicate(&patches[seam.patches[0]], &patches[seam.patches[1]], &seam.boundaries[0], &seam.boundaries[1], seam.order, seam.normal_scale, remaining.min(1_000_000))?;
        check(report.work <= remaining, "Invalid native exact seam work accounting")?;
        work += report.work;
        reports.push(report);
    }
    let unresolved: Vec<_> = reports.iter().enumerate().filter_map(|(i, r)| (!r.certified).then_some(i)).collect();
    let certified = !reports.is_empty() && unresolved.is_empty();
    let order = certified.then(|| if seams.iter().any(|s| s.order == 1) {1} else {2});
    Ok(Report {certified, order, work, unresolved, seams: reports})
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn empty_set_has_no_claim_and_limits_are_native() {
        for projective in [false, true] {
            let report = inspect(&[], &[], 0, projective).unwrap();
            assert!(!report.certified);
            assert_eq!(report.order, None);
            assert_eq!(report.work, 0);
            assert!(report.unresolved.is_empty());
            assert!(inspect(&[], &[], 2_000_001, projective).is_err());
        }
    }
    #[test]
    fn repeated_seams_charge_one_shared_budget_and_refuse_exhaustion() {
        let plane=|y:f64| Surface {degree_u:1,degree_v:1,knots_u:vec![0.,0.,1.,1.],knots_v:vec![0.,0.,1.,1.],
            control_points:vec![vec![vec![0.,y,0.],vec![0.,y+1.,0.]],vec![vec![1.,y,0.],vec![1.,y+1.,0.]]],
            weights:vec![vec![1.,1.],vec![1.,1.]],periodic_u:false,periodic_v:false};
        let patches=vec![plane(0.),plane(1.)];
        let declaration=|| Seam {patches:[0,1],boundaries:["vMax".into(),"vMin".into()],order:2,normal_scale:1.};
        for projective in [false,true] {
            let one=inspect(&patches,&[declaration()],1_000_000,projective).unwrap();
            assert!(one.certified && one.work>0);
            let both=inspect(&patches,&[declaration(),declaration()],1_000_000,projective).unwrap();
            assert!(both.certified);assert_eq!(both.work,2*one.work);
            let limited=inspect(&patches,&[declaration(),declaration()],one.work,projective).unwrap();
            assert!(!limited.certified);assert_eq!(limited.order,None);
            assert_eq!(limited.unresolved,vec![1]);assert!(limited.work<=one.work);
        }
    }
    #[test]
    fn missing_patch_is_rejected_before_predicate_execution() {
        let seam = Seam {patches: [0, 1], boundaries: ["vMax".into(), "vMin".into()], order: 2, normal_scale: 1.};
        assert!(inspect(&[], &[seam], 1_000_000, true).is_err());
    }
}
