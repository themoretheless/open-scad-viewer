//! Exact candidate parameters of an immutable certified UV root.
//! Candidates never replace the original root expression or its enclosure.
use crate::source_contact_point::SourcePoint;
use nurbs_core::{Error, Result};
pub struct Report {
    pub parameters: Option<[f64; 2]>,
    pub work_used: u64,
    pub reason: &'static str,
}
pub fn verify(point: &SourcePoint, candidate: [f64; 2], max_work: u64) -> Result<Report> {
    if candidate.iter().any(|t| !t.is_finite())
        || !(1..=cad_predicates::MAX_WORK).contains(&max_work)
    {
        return Err(Error::new(
            "BREP_SOURCE_ROOT_PARAMETER_INPUT",
            "Choose finite original parameters and bounded exact work",
        ));
    }
    let mut out = Report {
        parameters: None,
        work_used: 0,
        reason: "source-root-candidate-outside-selector",
    };
    let selector = point.selector();
    if (0..2).any(|i| candidate[i] <= selector[i][0] || candidate[i] >= selector[i][1]) {
        return Ok(out);
    }
    // SourcePoint privately owns a unique certified root in this original box.
    // Exact equality inside that box therefore identifies that root.
    out.reason = "source-root-candidate-identity-unproven";
    if let Some(r) = nurbs_core::curve_point_identity::verify(
        [point.boundary(), point.contact()],
        candidate,
        max_work,
    )? {
        out.work_used = r.work_used;
        if r.outcome == cad_predicates::BezierIdentity::Equal {
            out.parameters = Some(candidate);
            out.reason = "source-root-candidate-qualified";
        }
    }
    Ok(out)
}

/// Exact rational candidates identify the retained original root; no cached
/// enclosure or Cartesian point can replace its equations and selector.
pub struct FractionReport {
    pub parameters:Option<[[f64;2];2]>,
    pub work_used:u64,
    pub reason:&'static str,
}
pub fn verify_fraction(point:&SourcePoint,candidate:[[f64;2];2],max_work:u64)->Result<FractionReport> {
    use nurbs_core::interval_eval::Interval as I;
    if !(1..=cad_predicates::MAX_WORK).contains(&max_work)
        || candidate.iter().any(|p|p.iter().any(|t|!t.is_finite()) || p[1]<=0.) {
        return Err(Error::new("BREP_SOURCE_ROOT_PARAMETER_INPUT","Finite rational candidates and bounded exact work required"));
    }
    let mut out=FractionReport{parameters:None,work_used:0,reason:"source-root-fraction-outside-selector"};
    for i in 0..2 {
        let enclosure=I::point(candidate[i][0]).div(I::point(candidate[i][1]))?;
        let selector=point.selector()[i];
        if enclosure.lo<=selector[0] || enclosure.hi>=selector[1] {return Ok(out);}
    }
    out.reason="source-root-fraction-identity-unproven";
    if let Some(proof)=nurbs_core::curve_point_identity::verify_fraction([point.boundary(),point.contact()],candidate,max_work)? {
        out.work_used=proof.work_used;
        if proof.outcome==cad_predicates::BezierIdentity::Equal {
            out.parameters=Some(candidate);out.reason="source-root-fraction-qualified";
        }
    }
    Ok(out)
}
/// Bounded rational proposals; only fresh original-curve equality admits a root.
/// Midpoint rounding proposes numerators and never supplies geometric evidence.
pub struct FractionDiscovery {
    pub parameters: Option<[[f64; 2]; 2]>,
    pub work_used: u64,
    pub candidate_checks: usize,
    pub reason: &'static str,
}
pub fn discover_fraction(point: &SourcePoint, max_denominator: usize,
    max_candidates_per_axis: usize, max_work: u64) -> Result<FractionDiscovery> {
    use nurbs_core::interval_eval::Interval as I;
    if !(1..=4096).contains(&max_denominator)
        || !(1..=64).contains(&max_candidates_per_axis)
        || !(1..=cad_predicates::MAX_WORK).contains(&max_work) {
        return Err(Error::new("BREP_SOURCE_ROOT_PARAMETER_INPUT", "Bounded rational discovery required"));
    }
    let mut out = FractionDiscovery { parameters: None, work_used: 0,
        candidate_checks: 0, reason: "source-root-fraction-search-unproven" };
    let mut proposals: [Vec<[f64; 2]>; 2] = [Vec::new(), Vec::new()];
    for axis in 0..2 {
        let [lo, hi] = point.selector()[axis];
        let middle = lo * 0.5 + hi * 0.5;
        let mut seen = std::collections::BTreeSet::new();
        for denominator in 1..=max_denominator {
            let numerator = (middle * denominator as f64).round();
            // Keep authored integer candidates exactly representable; no casts
            // from huge or nonfinite estimates may manufacture a proposal.
            if !numerator.is_finite() || numerator.abs() > (1_u64 << 40) as f64 { continue; }
            let mut n = numerator as i64;
            let mut d = denominator as i64;
            let (mut a, mut b) = (n.unsigned_abs(), d as u64);
            while b != 0 { let r = a % b; a = b; b = r; }
            n /= a as i64; d /= a as i64;
            if !seen.insert((n, d)) { continue; }
            let fraction = [n as f64, d as f64];
            let bounds = I::point(fraction[0]).div(I::point(fraction[1]))?;
            if bounds.lo > lo && bounds.hi < hi {
                proposals[axis].push(fraction);
                if proposals[axis].len() == max_candidates_per_axis { break; }
            }
        }
    }
    for a in &proposals[0] { for b in &proposals[1] {
        if out.work_used == max_work {
            out.reason = "source-root-fraction-search-work-exhausted";
            return Ok(out);
        }
        out.candidate_checks += 1;
        let proof = verify_fraction(point, [*a, *b], max_work - out.work_used)?;
        out.work_used += proof.work_used;
        if let Some(parameters) = proof.parameters {
            out.parameters = Some(parameters);
            out.reason = "source-root-fraction-search-qualified";
            return Ok(out);
        }
    }}
    Ok(out)
}
#[cfg(test)]
mod fraction_tests {
    use super::*;
    use nurbs_core::{curve::Curve,surface::Surface};
    #[test]
    fn nonlinear_one_third_root_retains_original_equations() {
        let surface=Surface{degree_u:1,degree_v:1,knots_u:vec![0.,0.,1.,1.],knots_v:vec![0.,0.,1.,1.],
            control_points:vec![vec![vec![0.,0.,0.],vec![0.,1.,0.]],vec![vec![1.,0.,0.],vec![1.,1.,0.]]],
            weights:vec![vec![1.;2];2],periodic_u:false,periodic_v:false};
        let boundary=Curve{degree:2,knots:vec![0.,0.,0.,1.,1.,1.],control_points:vec![vec![0.,0.],vec![0.5,0.],vec![1.,1.]],weights:vec![1.;3],periodic:false};
        let cutter=Curve::from_polyline(vec![vec![0.,0.],vec![3.,1.]]).unwrap();
        let root=crate::source_contact_point::qualify(&surface,&boundary,&cutter,[[0.3,0.36],[0.10,0.12]],10000).unwrap();
        let root=root.point.expect(root.reason);let definition=root.definition();
        let candidate=[[1.,3.],[1.,9.]];
        let proof=verify_fraction(&root,candidate,cad_predicates::MAX_WORK).unwrap();
        assert_eq!(proof.parameters,Some(candidate),"{} work {} selector {:?}",proof.reason,proof.work_used,root.selector());
        assert!(verify(&root,[1./3.,1./9.],cad_predicates::MAX_WORK).unwrap().parameters.is_none());
        assert!(verify_fraction(&root,[[1.,3.],[10.,89.]],cad_predicates::MAX_WORK).unwrap().parameters.is_none());
        assert!(verify_fraction(&root,[[1.,2.],[1.,9.]],cad_predicates::MAX_WORK).unwrap().parameters.is_none());
        assert!(verify_fraction(&root,[[1.,0.],[1.,9.]],cad_predicates::MAX_WORK).is_err());
        assert!(verify_fraction(&root,candidate,proof.work_used-1).unwrap().parameters.is_none());
        let bytes=value_codec::encode_binary(&definition).unwrap();
        let replay=crate::source_contact_point::restore(value_codec::decode_binary(&bytes).unwrap(),10000).unwrap();
        let replay=replay.point.expect(replay.reason);
        assert_eq!(replay.definition(),definition);
        assert_eq!(verify_fraction(&replay,candidate,cad_predicates::MAX_WORK).unwrap().parameters,Some(candidate));
        let discovered=discover_fraction(&root,128,16,cad_predicates::MAX_WORK).unwrap();
        assert_eq!(discovered.parameters,Some(candidate),"{}",discovered.reason);
        assert!(discovered.candidate_checks>0 && discovered.candidate_checks<=256);
        assert_eq!(discover_fraction(&replay,128,16,cad_predicates::MAX_WORK).unwrap().parameters,Some(candidate));
        assert!(discover_fraction(&root,8,16,cad_predicates::MAX_WORK).unwrap().parameters.is_none());
        assert!(discover_fraction(&root,128,16,1).unwrap().parameters.is_none());
        assert!(discover_fraction(&root,0,16,cad_predicates::MAX_WORK).is_err());
        assert!(discover_fraction(&root,128,65,cad_predicates::MAX_WORK).is_err());
        let horizontal=Curve::from_polyline(vec![vec![0.,0.5],vec![1.,0.5]]).unwrap();
        let irrational=crate::source_contact_point::qualify(&surface,&boundary,&horizontal,[[0.70,0.72],[0.70,0.72]],10000).unwrap();
        let irrational=irrational.point.expect(irrational.reason);
        let original=irrational.definition();
        assert!(discover_fraction(&irrational,128,8,cad_predicates::MAX_WORK).unwrap().parameters.is_none());
        assert_eq!(irrational.definition(),original);
        assert_eq!(root.definition(),definition);
    }
}
