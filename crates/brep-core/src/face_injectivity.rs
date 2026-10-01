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
}
#[derive(Clone, Debug)]
pub struct Report {
    pub all_faces_injective: bool,
    pub spans: usize,
    pub faces: Vec<Face>,
}
pub fn inspect(model: &Model, max_spans: usize) -> Result<Report> {
    model.validate_boundary_diagnostic_inputs()?;
    if max_spans == 0 || max_spans > 100_000 {
        return Err(Error::new(
            "BREP_INJECTIVITY_BUDGET",
            "Face injectivity budget must be in 1..100000",
        ));
    }
    let mut report = Report {
        all_faces_injective: true,
        spans: 0,
        faces: Vec::new(),
    };
    for (face, f) in model.faces.iter().enumerate() {
        let result = if report.spans < max_spans {
            Some(surface_injectivity::certify(
                &f.surface,
                max_spans - report.spans,
            )?)
        } else {
            None
        };
        report.all_faces_injective &= result.as_ref().is_some_and(|r| r.proven);
        report.spans += result.as_ref().map_or(0, |r| r.spans);
        report.faces.push(Face { face, result });
    }
    Ok(report)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn perspective_projection_certifies_every_sphere_chart_with_shared_budget(){
        let model=crate::analytic::sphere(3.).unwrap();let before=format!("{model:?}");
        let r=inspect(&model,1000).unwrap();assert!(r.all_faces_injective);assert_eq!(r.spans,968);
        for f in &r.faces{let x=f.result.as_ref().unwrap();assert_eq!(x.reason,"global-projective-projection-contraction");assert!(x.projective_projection.is_some());}
        let r=inspect(&model,967).unwrap();assert!(!r.all_faces_injective);assert_eq!(r.spans,967);
        assert_eq!(r.faces.last().unwrap().result.as_ref().unwrap().reason,"work-limit");
        assert_eq!(format!("{model:?}"),before);
    }
    #[test]
    fn linear_projection_certifies_every_authored_cylinder_side(){
        let model=crate::analytic::cylinder(2.,4.).unwrap();
        assert!(inspect(&model,1000).unwrap().all_faces_injective);
        let partial=inspect(&model,16).unwrap();assert!(!partial.all_faces_injective);assert!(partial.spans<=16);
        for (i,direction) in [[-1.,1.,0.],[-1.,-1.,0.],[1.,-1.,0.],[1.,1.,0.]].into_iter().enumerate(){
            let q=surface_injectivity::certify_linear_projection(&model.faces[i].surface,[direction,[0.,0.,1.]],16).unwrap();
            assert!(q.is_some(),"face {i}: {q:?}");
        }
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
        let r = inspect(&model, 1000).unwrap();
        assert!(!r.all_faces_injective);
        assert!(!r.faces[0].result.as_ref().unwrap().proven);
        assert!(
            r.faces[1..]
                .iter()
                .all(|f| f.result.as_ref().unwrap().proven)
        );
    }
}
