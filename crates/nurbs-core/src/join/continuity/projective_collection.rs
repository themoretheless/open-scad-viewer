//! Complete explicitly declared seam sets with one native shared exact budget.
//! Adjacency, trim ownership and omitted joins are separate obligations.
use crate::{Result, check, surface::Surface};
use super::{ExactStripJetReport, inspect_surface_projective_strip_jets, inspect_surface_exact_strip_jets};

pub struct ProjectiveSeam {
    pub patches: [usize; 2],
    pub boundaries: [String; 2],
    pub order: usize,
    pub normal_scale: f64,
}
pub struct ProjectiveSeamCollectionReport {
    pub certified: bool,
    pub certified_order: Option<usize>,
    pub exact_work: u64,
    pub unresolved_seams: Vec<usize>,
    pub seams: Vec<ExactStripJetReport>,
}
pub fn inspect_projective_seam_collection(
    patches: &[Surface], seams: &[ProjectiveSeam], max_work: u64,
) -> Result<ProjectiveSeamCollectionReport> {
    inspect_collection(patches, seams, max_work, true, false, false)
}
/// Treat declared normal scales as proposals, then independently prove exact
/// rational projective identities and regularity. Adjacency/ownership remain
/// separate caller obligations. Explicit-scale APIs retain their semantics.
pub fn inspect_projective_seam_collection_with_rational_scale_proposals(
    patches:&[Surface],seams:&[ProjectiveSeam],max_work:u64,
)->Result<ProjectiveSeamCollectionReport>{
    inspect_collection(patches,seams,max_work,true,true,false)
}
/// Preserve remaining owner work after the complete-set claim is already
/// refused. Unvisited seams remain explicitly unresolved, never certified.
pub fn inspect_projective_seam_collection_until_refused(
    patches:&[Surface],seams:&[ProjectiveSeam],max_work:u64,
)->Result<ProjectiveSeamCollectionReport>{
    inspect_collection(patches,seams,max_work,true,true,true)
}
pub fn inspect_exact_seam_collection(
    patches: &[Surface], seams: &[ProjectiveSeam], max_work: u64,
) -> Result<ProjectiveSeamCollectionReport> {
    check(max_work <= 1000000, "Invalid exact seam work budget")?;
    inspect_collection(patches, seams, max_work, false, false, false)
}
fn inspect_collection(
    patches: &[Surface], seams: &[ProjectiveSeam], max_work: u64, projective: bool, rational_proposal:bool, stop_after_refusal:bool,
) -> Result<ProjectiveSeamCollectionReport> {
    check(max_work <= 2000000 && patches.len() <= 4096 && seams.len() <= 4096,
        "Invalid exact seam collection limits")?;
    // Validate the entire declaration before consuming any proof work.
    for seam in seams {
        check(seam.patches.iter().all(|&p| p < patches.len()), "Invalid exact seam patch index")?;
        check(matches!(seam.order, 1 | 2), "Invalid exact seam order")?;
        check(seam.normal_scale.is_finite() && seam.normal_scale > 0., "Invalid exact seam normal scale")?;
        check(seam.boundaries.iter().all(|b| matches!(b.as_str(), "uMin" | "uMax" | "vMin" | "vMax")),
            "Invalid exact seam boundary")?;
    }
    if stop_after_refusal {
        // A failed early identity must not hide malformed later geometry.
        let mut validated=vec![false;patches.len()];
        for seam in seams {
            for &index in &seam.patches {
                if !validated[index] {
                    patches[index].validate()?;
                    check(patches[index].control_points.iter().flatten().all(|p|p.len()==3),
                        "Exact strip jets require XYZ surfaces")?;
                    validated[index]=true;
                }
            }
        }
    }
    let mut out = ProjectiveSeamCollectionReport {
        certified: false, certified_order: None, exact_work: 0,
        unresolved_seams: Vec::new(), seams: Vec::new(),
    };
    for (index, seam) in seams.iter().enumerate() {
        let remaining = (max_work - out.exact_work).min(1000000);
        let inspect = if projective { inspect_surface_projective_strip_jets }
            else { inspect_surface_exact_strip_jets };
        let mut proof = if rational_proposal {
            super::exact_strip::inspect_station_projective_strip_jets(
                &patches[seam.patches[0]],&patches[seam.patches[1]],
                &seam.boundaries[0],&seam.boundaries[1],seam.order,seam.normal_scale,
                super::station_scale::propose_station_scale_scalar(seam.normal_scale),remaining,
            )?
        }else{inspect(
            &patches[seam.patches[0]], &patches[seam.patches[1]],
            &seam.boundaries[0], &seam.boundaries[1], seam.order, seam.normal_scale, remaining,
        )?};
        check(proof.work <= remaining, "Invalid native exact seam work accounting")?;
        // Keep the caller's exact nominal unit relation first. Sampling is
        // only a second proposal and cannot remove an already proved seam.
        if rational_proposal && seam.normal_scale==1. && !proof.certified
            && !proof.exact_identity && proof.work<remaining {
            let ratio=super::station_scale::propose_boundary_normal_scale(
                &patches[seam.patches[0]],&patches[seam.patches[1]],
                &seam.boundaries[0],&seam.boundaries[1],
            )?;
            let scalar=super::station_scale::propose_station_scale_scalar(ratio);
            if scalar!=cad_predicates::AuthoredScalar::Binary64Bits(1f64.to_bits())
                && scalar!= (cad_predicates::AuthoredScalar::RationalConstant{numerator:1,denominator:1}) {
                let prior_work=proof.work;
                proof=super::exact_strip::inspect_station_projective_strip_jets(
                    &patches[seam.patches[0]],&patches[seam.patches[1]],
                    &seam.boundaries[0],&seam.boundaries[1],seam.order,ratio,scalar,remaining-prior_work,
                )?;
                proof.work+=prior_work;
            }
        }
        if rational_proposal && !proof.certified && !proof.exact_identity && proof.work<remaining {
            if let Some(scalar)=super::station_scale::propose_boundary_binary_ratio_scalar(
                &patches[seam.patches[0]],&patches[seam.patches[1]],
                &seam.boundaries[0],&seam.boundaries[1],
            )? {
                let prior_work=proof.work;
                proof=super::exact_strip::inspect_station_projective_strip_jets(
                    &patches[seam.patches[0]],&patches[seam.patches[1]],
                    &seam.boundaries[0],&seam.boundaries[1],seam.order,seam.normal_scale,scalar,remaining-prior_work,
                )?;
                proof.work+=prior_work;
            }
        }
        check(proof.work<=remaining,"Invalid native candidate work accounting")?;
        out.exact_work += proof.work;
        if !proof.certified { out.unresolved_seams.push(index); }
        out.seams.push(proof);
        if stop_after_refusal && !out.unresolved_seams.is_empty() {
            for skipped in index+1..seams.len() {
                out.unresolved_seams.push(skipped);
                out.seams.push(ExactStripJetReport {
                    certified:false,exact_identity:false,regularity_certified:false,work:0,
                    reason:"seam-audit-skipped-after-refusal",
                });
            }
            break;
        }
    }
    out.certified = !seams.is_empty() && out.unresolved_seams.is_empty();
    if out.certified { out.certified_order = seams.iter().map(|s| s.order).min(); }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nominal_first_keeps_explicit_semantics_and_charges_rational_fallback() {
        let mut a=plane(0.);let mut b=plane(3.);
        for row in &mut a.control_points {row[1][1]=3.;}
        for row in &mut b.control_points {row[1][1]=10.;}
        let patches=vec![a,b];
        let seams=[ProjectiveSeam {patches:[0,1],boundaries:["vMax".into(),"vMin".into()],order:2,normal_scale:1.}];
        assert!(!inspect_projective_seam_collection(&patches,&seams,1000000).unwrap().certified);
        let proof=inspect_projective_seam_collection_with_rational_scale_proposals(&patches,&seams,1000000).unwrap();
        assert!(proof.certified);
        assert!(!inspect_projective_seam_collection_with_rational_scale_proposals(&patches,&seams,proof.exact_work-1).unwrap().certified);
        assert_eq!(proof.exact_work,proof.seams[0].work);
        let mut kink=patches.clone();kink[1].control_points[1][1][2]=0.125;
        assert!(!inspect_projective_seam_collection_with_rational_scale_proposals(&kink,&seams,1000000).unwrap().certified);
    }
    fn plane(y: f64) -> Surface {
        Surface { degree_u: 1, degree_v: 1, knots_u: vec![0.,0.,1.,1.], knots_v: vec![0.,0.,1.,1.],
            control_points: (0..2).map(|x| vec![vec![x as f64,y,0.],vec![x as f64,y+1.,0.]]).collect(),
            weights: vec![vec![1.;2];2], periodic_u: false, periodic_v: false }
    }
    #[test]
    fn represented_nonuniform_station_speeds_keep_exact_g2_without_snapping() {
        let mut a=plane(27.);let mut b=plane(28.);
        let shared=f64::from_bits(28f64.to_bits()+1);
        for row in &mut a.control_points {row[1][1]=shared;}
        for row in &mut b.control_points {row[0][1]=shared;}
        let patches=[a,b];
        let scale=super::super::station_scale::propose_boundary_normal_scale(&patches[0],&patches[1],"vMax","vMin").unwrap();
        let seams=[ProjectiveSeam{patches:[0,1],boundaries:["vMax".into(),"vMin".into()],order:2,normal_scale:scale}];
        let saved=patches[0].control_points.clone();
        let proof=inspect_projective_seam_collection_until_refused(&patches,&seams,2000000).unwrap();
        assert!(proof.certified && proof.certified_order==Some(2));
        assert!(!inspect_projective_seam_collection_until_refused(&patches,&seams,proof.exact_work-1).unwrap().certified);
        assert_eq!(patches[0].control_points,saved);
        let mut kink=patches.clone();kink[1].control_points[1][1][2]=f64::EPSILON;
        assert!(!inspect_projective_seam_collection_until_refused(&kink,&seams,2000000).unwrap().certified);
        // Explicit binary-scale API still requires its originally represented scale.
        assert!(!inspect_projective_seam_collection(&patches,&seams,2000000).unwrap().certified);
    }
    #[test]
    fn refused_complete_sets_leave_other_axes_work_without_hiding_invalid_tail() {
        let mut patches=[plane(0.),plane(1.),plane(2.)];
        let seams=(0..2).map(|i|ProjectiveSeam {patches:[i,i+1],
            boundaries:["vMax".into(),"vMin".into()],order:2,normal_scale:1.}).collect::<Vec<_>>();
        let positive=inspect_projective_seam_collection_until_refused(&patches,&seams,2000000).unwrap();
        assert!(positive.certified && positive.seams.iter().all(|p|p.certified));
        let short=inspect_projective_seam_collection_until_refused(&patches,&seams,positive.exact_work-1).unwrap();
        assert!(!short.certified && short.exact_work<=positive.exact_work-1);
        patches[1].control_points[1][1][2]=0.125;
        let all=inspect_projective_seam_collection_with_rational_scale_proposals(&patches,&seams,2000000).unwrap();
        let stopped=inspect_projective_seam_collection_until_refused(&patches,&seams,2000000).unwrap();
        assert!(!stopped.certified && stopped.certified_order.is_none());
        assert_eq!(stopped.seams.len(),seams.len());
        assert_eq!(stopped.unresolved_seams,vec![0,1]);
        assert!(stopped.exact_work<all.exact_work);
        assert_eq!(stopped.exact_work,stopped.seams[0].work);
        assert_eq!(stopped.seams[1].reason,"seam-audit-skipped-after-refusal");
        assert!(!stopped.seams[1].certified && !stopped.seams[1].exact_identity && !stopped.seams[1].regularity_certified);
        assert_eq!(stopped.seams[1].work,0);
        patches[2].knots_v[0]=2.;
        assert!(inspect_projective_seam_collection_until_refused(&patches,&seams,2000000).is_err());
    }
    #[test]
    #[cfg(feature = "transport")]
    fn transport_preserves_native_collection_work_and_order() {
        let request=value_codec::json!({"op":"sweep_projective_seams_audit",
            "patches":[plane(0.),plane(1.)],"seams":[{"patches":[0,1],
                "boundaries":["vMax","vMin"],"order":2,"normalScale":1.,"jetTolerance":0.}],
            "maxWork":2000000});
        let r=crate::transport::dispatch(request.clone()).unwrap();
        assert_eq!(r["exactG1G2Certified"],true);
        assert_eq!(r["certifiedOrder"],2);
        assert_eq!(r["exactWork"],r["seams"][0]["work"]);
        let mut exact=request.clone();
        exact["op"]=value_codec::json!("sweep_exact_seams_audit");
        exact["maxWork"]=value_codec::json!(1000000);
        let r=crate::transport::dispatch(exact).unwrap();
        assert_eq!(r["exactG1G2Certified"],true);
        assert_eq!(r["certifiedOrder"],2);
        assert_eq!(r["exactWork"],r["seams"][0]["work"]);
        assert!(r.get("method").is_none());
        let mut zero=request;
        zero["maxWork"]=value_codec::json!(0);
        let r=crate::transport::dispatch(zero).unwrap();
        assert_eq!(r["exactG1G2Certified"],false);
        assert_eq!(r["unresolvedSeams"],value_codec::json!([0]));
    }
    #[test]
    fn collection_owns_complete_order_and_shared_work_without_vacuous_success() {
        let patches = [plane(0.), plane(1.), plane(2.)];
        let mut seams = (0..2).map(|i| ProjectiveSeam { patches: [i,i+1],
            boundaries: ["vMax".into(),"vMin".into()], order: 2, normal_scale: 1. }).collect::<Vec<_>>();
        let positive = inspect_projective_seam_collection(&patches,&seams,2000000).unwrap();
        assert!(positive.certified && positive.certified_order == Some(2));
        let exact = inspect_exact_seam_collection(&patches,&seams,1000000).unwrap();
        assert!(exact.certified && exact.certified_order == Some(2));
        assert!(!inspect_exact_seam_collection(&patches,&seams,exact.exact_work-1).unwrap().certified);
        assert!(inspect_exact_seam_collection(&patches,&seams,1000001).is_err());
        assert_eq!(positive.exact_work, positive.seams.iter().map(|r|r.work).sum());
        let short = inspect_projective_seam_collection(&patches,&seams,positive.exact_work-1).unwrap();
        assert!(!short.certified && short.certified_order.is_none());
        assert_eq!(short.unresolved_seams, vec![1]);
        assert!(short.exact_work < positive.exact_work);
        seams[0].order = 1;
        assert_eq!(inspect_projective_seam_collection(&patches,&seams,2000000).unwrap().certified_order,Some(1));
        assert!(!inspect_projective_seam_collection(&patches,&[],0).unwrap().certified);
        seams[1].patches[1] = 3;
        assert!(inspect_projective_seam_collection(&patches,&seams,2000000).is_err());
    }
}
