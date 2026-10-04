//! Sufficient absence-of-self-intersection checks within individual faces.
//! Pairwise contacts between different faces are deliberately not classified.
use crate::Model;
use nurbs_core::{
    Error, Result,
    surface_injectivity::{self, Report as FaceReport},
};
#[derive(Clone, Debug)]
pub struct Face {
    pub face: usize,
    pub result: Option<FaceReport>,
    pub linear: Option<nurbs_core::surface_linear_monotonicity::Report>,
}
#[derive(Clone, Debug)]
pub struct Report {
    pub all_faces_injective: bool,
    pub spans: usize,
    pub linear_cells: usize,
    pub faces: Vec<Face>,
}
pub fn inspect(model: &Model, max_spans: usize) -> Result<Report> {
    inspect_impl(model, max_spans, 0, true)
}
/// Independent shared budgets for contraction spans and oblique refinement.
/// A successful alternative proves the same whole-chart injectivity property.
pub fn inspect_with_linear(model: &Model, max_spans: usize, max_linear_cells: usize) -> Result<Report> {
    inspect_impl(model,max_spans,max_linear_cells,false)
}
fn inspect_impl(model:&Model,max_spans:usize,max_linear_cells:usize,legacy:bool)->Result<Report> {
    model.validate_boundary_diagnostic_inputs()?;
    if max_spans == 0 || max_spans > 100_000 || max_linear_cells > 100_000 {
        return Err(Error::new(
            "BREP_INJECTIVITY_BUDGET",
            "Face injectivity budget must be in 1..100000",
        ));
    }
    let mut report = Report {
        all_faces_injective: true,
        spans: 0,
        linear_cells: 0,
        faces: Vec::new(),
    };
    for (face, f) in model.faces.iter().enumerate() {
        let result = if report.spans < max_spans {
            Some(if legacy {surface_injectivity::certify(&f.surface,max_spans-report.spans)?} else {surface_injectivity::certify_contraction(&f.surface,max_spans-report.spans)?})
        } else {
            None
        };
        let linear = if !result.as_ref().is_some_and(|r| r.proven)
            && report.linear_cells < max_linear_cells {
            Some(nurbs_core::surface_linear_monotonicity::inspect_candidate(
                &f.surface, max_linear_cells-report.linear_cells)?)
        } else { None };
        report.linear_cells += linear.as_ref().map_or(0, |r| r.cells);
        report.all_faces_injective &= result.as_ref().is_some_and(|r| r.proven)
            || linear.as_ref().is_some_and(|r|r.certified);
        report.spans += result.as_ref().map_or(0, |r| r.spans);
        report.faces.push(Face { face, result, linear });
    }
    Ok(report)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retained_hollow_walls_use_shared_oblique_budget_without_overwriting_contraction() {
        let section=|z|vec![
            vec![nurbs_core::primitives::circle([0.,0.,z],[0.,0.,1.],0.5).unwrap()],
            vec![nurbs_core::primitives::circle([0.,0.,z],[0.,0.,1.],0.2).unwrap().reverse().unwrap()],
        ];
        let model=crate::rational_section_loft(&[section(0.),section(5.),section(10.)]).unwrap();
        let before=model.clone();
        let original=inspect_with_linear(&model,1000,0).unwrap();
        assert!(!original.all_faces_injective);
        let proven=inspect_with_linear(&model,1000,10000).unwrap();
        assert!(proven.all_faces_injective,"{proven:?}");
        assert!(proven.linear_cells>0 && proven.linear_cells<=10000);
        assert_eq!(proven.faces.len(),model.faces.len());
        for (a,b) in original.faces.iter().zip(&proven.faces) {
            assert_eq!(format!("{:?}",a.result),format!("{:?}",b.result));
        }
        let partial=inspect_with_linear(&model,1000,1).unwrap();
        assert!(!partial.all_faces_injective);
        assert!(partial.linear_cells<=1);
        assert_eq!(partial.faces.len(),model.faces.len());
        assert!(partial.faces.iter().any(|f|f.linear.is_none() && !f.result.as_ref().is_some_and(|r|r.proven)));
        assert!(inspect_with_linear(&model,1000,100001).is_err());
        assert_eq!(model,before);
    }
    #[test]
    fn actual_spatial_miter_walls_have_bounded_injectivity_proofs() {
        let profiles=vec![
            nurbs_core::primitives::circle([0.;3],[0.,0.,1.],0.5).unwrap(),
            nurbs_core::primitives::circle([0.;3],[0.,0.,1.],0.2).unwrap().reverse().unwrap(),
        ];
        let points=[[0.,0.,0.],[0.,0.,10.],[10.,0.,10.],[10.,10.,15.]];
        let mut sections=nurbs_core::paths::miter_sections(&profiles,&points,[1.,0.,0.],2.).unwrap();
        let last=sections.len()-1;
        let correction=nurbs_core::section_projection::project(&sections[last],1,[0.,-0.5],17.5,2_f64.powi(-40),1e-9,1000000).unwrap();
        sections[last]=correction.curves.unwrap();
        let sections=sections.into_iter().map(|row|row.into_iter().map(|c|vec![c]).collect()).collect::<Vec<_>>();
        let model=crate::rational_section_loft(&sections).unwrap();
        let limited=inspect_with_linear(&model,1000,10000).unwrap();
        assert!(!limited.all_faces_injective);
        assert!(limited.linear_cells<=10000);
        let report=inspect_with_linear(&model,1000,20000).unwrap();
        println!("spatial injectivity shared cells: {}",report.linear_cells);
        assert!(report.all_faces_injective,"{report:?}");
        assert!(report.linear_cells<=20000);
    }
    #[test]
    fn cube_faces_pass_and_budget_keeps_unvisited_faces_explicit() {
        let model = crate::cuboid([0.; 3], [1.; 3]).unwrap();
        let r = inspect(&model, 6).unwrap();
        assert!(r.all_faces_injective);
        assert_eq!(r.spans, 6);
        assert_eq!(r.faces.len(), 6);
        let r = inspect(&model, 1).unwrap();
        assert!(!r.all_faces_injective);
        assert_eq!(r.faces.len(), 6);
        assert_eq!(r.faces.iter().filter(|f| f.result.is_none()).count(), 5);
    }
    #[test]
    fn collapsing_one_face_does_not_hide_behind_topological_validity() {
        let mut model = crate::cuboid([0.; 3], [1.; 3]).unwrap();
        for row in &mut model.faces[0].surface.control_points {
            for p in row {
                *p = vec![0.; 3];
            }
        }
        let r = inspect(&model, 6).unwrap();
        assert!(!r.all_faces_injective);
        assert!(!r.faces[0].result.as_ref().unwrap().proven);
        assert!(
            r.faces[1..]
                .iter()
                .all(|f| f.result.as_ref().unwrap().proven)
        );
    }
}
