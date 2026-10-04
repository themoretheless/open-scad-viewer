//! Whole ruled-family correction bounds with native shared budgets and basis premises.
use crate::{Result, curve::Curve, section_projection, section_circle_repair};
pub struct Correction {pub section: usize, pub axis: usize, pub coefficients: [f64;2], pub offset: f64, pub quantum: f64, pub tolerance: f64}
pub struct Report {pub sections: Option<Vec<Vec<Curve>>>, pub upper: Option<f64>, pub planar: Vec<usize>, pub work: u64, pub reason: &'static str}
fn refuse(work: u64, reason: &'static str) -> Report {Report {sections: None,upper: None,planar: Vec::new(),work,reason}}
fn common_basis(sections: &[Vec<Curve>], validate: bool) -> bool {
    let Some(basis) = sections.first() else {return false;};
    !basis.is_empty() && sections.iter().all(|station| station.len() == basis.len() && station.iter().zip(basis).all(|(c,a)| {
        (!validate || (c.validate().is_ok() && !c.periodic && c.control_points.iter().all(|p| p.len()==3)))
        && c.degree==a.degree && c.control_points.len()==a.control_points.len() && c.knots==a.knots && c.weights==a.weights
        && c.weights.iter().all(|w| w.is_finite() && *w>0.)
    }))
}
pub fn project(sections: &[Vec<Curve>], corrections: &[Correction], max_work: u64) -> Result<Report> {
    if max_work==0 || max_work>1_000_000 {return Ok(refuse(0,"work-limit"));}
    let indices: std::collections::BTreeSet<_> = corrections.iter().map(|c| c.section).collect();
    if sections.len()<2 || sections.len()>1025 || sections.first().is_none_or(|s| s.len()>64)
        || corrections.is_empty() || corrections.len()>sections.len() || indices.len()!=corrections.len()
        || indices.iter().any(|&i| i>=sections.len()) || !common_basis(sections,true) {
        return Ok(refuse(0,"incompatible-section-basis"));
    }
    let mut corrected = sections.to_vec(); let mut planar=Vec::new(); let mut work=0; let mut upper=0_f64;
    for c in corrections {
        if work==max_work {return Ok(refuse(work,"work-limit"));}
        let r=section_projection::project(&sections[c.section],c.axis,c.coefficients,c.offset,c.quantum,c.tolerance,max_work-work)?;
        crate::check(r.work<=max_work-work,"Invalid native section correction work accounting")?;
        work+=r.work;
        let (Some(curves),Some(bound))=(r.curves,r.displacement_upper) else {return Ok(refuse(work,if r.reason=="work-limit" {"work-limit"} else {"projection-unproved"}));};
        if !r.exact_planar {return Ok(refuse(work,"projection-unproved"));}
        upper=upper.max(bound);corrected[c.section]=curves;planar.push(c.section);
    }
    Ok(Report {sections:Some(corrected),upper:Some(upper),planar,work,reason:"bounded-section-interpolation"})
}
pub fn repair_circle(sections: &[Vec<Curve>], quantum: f64, tolerance: f64, max_work: u64) -> Result<Report> {
    if max_work>1_000_000 || sections.is_empty() || sections.len()>1025 || sections.first().is_none_or(|s| s.is_empty()) {return Ok(refuse(0,"invalid-budget-or-sections"));}
    if !common_basis(sections,false) {return Ok(refuse(0,"incompatible-section-basis"));}
    let mut corrected=Vec::new();let mut work=0;let mut upper=0_f64;
    for station in sections {
        let r=section_circle_repair::repair(station,quantum,tolerance,max_work-work)?;
        crate::check(r.work<=max_work-work,"Invalid native circle correction work accounting")?;
        work+=r.work;
        let (Some(curves),Some(bound))=(r.curves,r.displacement_upper) else {return Ok(refuse(work,r.reason));};
        if !bound.is_finite() || bound<0. || bound>tolerance {return Ok(refuse(work,"invalid-native-displacement"));}
        upper=upper.max(bound);corrected.push(curves);
    }
    Ok(Report {sections:Some(corrected),upper:Some(upper),planar:Vec::new(),work,reason:"bounded-circle-section-interpolation"})
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sections() -> Vec<Vec<Curve>> {
        [0.,1.,2.].into_iter().map(|z| vec![crate::primitives::ellipse_arc([0.,0.,z],[1.,0.,0.],[0.,1.,0.],0.,360.).unwrap()]).collect()
    }
    #[test]
    fn correction_family_preserves_sources_and_refuses_incompatible_weights() {
        let sections=sections();let original=sections.clone();
        let report=repair_circle(&sections,2_f64.powi(-40),1e-8,1_000_000).unwrap();
        assert!(report.sections.is_some());
        assert!(report.upper.unwrap()<=1e-8);
        assert!(report.work>0);
        assert!(repair_circle(&sections,2_f64.powi(-40),1e-8,0).unwrap().sections.is_none());
        let mut unequal=sections.clone();unequal[1][0].weights[0]=1_f64.next_up();
        assert_eq!(repair_circle(&unequal,2_f64.powi(-40),1e-8,1_000_000).unwrap().reason,"incompatible-section-basis");
        assert_eq!(sections,original);
    }
    #[test]
    fn projection_family_shares_work_and_requires_unique_station_corrections() {
        let sections=sections();let original=sections.clone();
        let correction=Correction {section:0,axis:2,coefficients:[0.,0.],offset:0.,quantum:2_f64.powi(-40),tolerance:1e-8};
        let report=project(&sections,&[correction],1_000_000).unwrap();
        assert!(report.sections.is_some());assert_eq!(report.planar,vec![0]);assert!(report.work>0);
        assert_eq!(sections,original);
        let duplicate=|| Correction {section:0,axis:2,coefficients:[0.,0.],offset:0.,quantum:2_f64.powi(-40),tolerance:1e-8};
        assert_eq!(project(&sections,&[duplicate(),duplicate()],1_000_000).unwrap().reason,"incompatible-section-basis");
        assert!(project(&sections,&[duplicate()],0).unwrap().upper.is_none());
    }
}
