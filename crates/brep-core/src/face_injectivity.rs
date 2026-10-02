//! Sufficient absence-of-self-intersection checks within individual faces.
//! Pairwise contacts between different faces are deliberately not classified.
use crate::Model;
use nurbs_core::{
    Error, Result,
    surface_injectivity::{self, Report as FaceReport},
    surface_quotient_injectivity,
};
#[derive(Clone, Debug)]
pub struct Face {
    pub face: usize,
    pub result: Option<FaceReport>,
    pub quotient: Option<Quotient>,
}
#[derive(Clone, Debug)]
pub struct Quotient {
    pub collapsed_end: usize,
    pub pole_edge: usize,
    pub pole_vertex: usize,
    pub result: surface_quotient_injectivity::Report,
}
impl Face {
    pub fn absence_proven(&self)->bool {
        self.result.as_ref().is_some_and(|r|r.proven)||self.quotient.as_ref().is_some_and(|q|q.result.proven)
    }
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
        report.spans += result.as_ref().map_or(0, |r| r.spans);
        let quotient=if result.as_ref().is_some_and(|r|r.reason=="collapsed-boundary-requires-quotient-proof") && max_spans-report.spans>=256 {
            if let Some((end,edge,vertex))=declared_pole(model,face) {
                let proof=surface_quotient_injectivity::certify_source_frame(&f.surface,end,16,max_spans-report.spans)?;
                report.spans+=proof.cells;
                Some(Quotient{collapsed_end:end,pole_edge:edge,pole_vertex:vertex,result:proof})
            }else{None}
        }else{None};
        let face=Face {face,result,quotient};
        report.all_faces_injective &=face.absence_proven();
        report.faces.push(face);
    }
    Ok(report)
}
fn declared_pole(model:&Model,face:usize)->Option<(usize,usize,usize)> {
    let f=&model.faces[face];let s=&f.surface;
    let u=[s.knots_u[s.degree_u],s.knots_u[s.control_points.len()]];
    let v=[s.knots_v[s.degree_v],s.knots_v[s.control_points[0].len()]];
    for loop_index in std::iter::once(f.outer).chain(f.holes.iter().copied()) {
        for ce in &model.loops[loop_index].coedges {
            let edge=&model.edges[ce.edge];
            if edge.vertices[0]!=edge.vertices[1] {continue;}
            let pole=model.vertices[edge.vertices[0]].point;
            if !edge.curve.control_points.iter().all(|p|p.as_slice()==pole.as_slice()) || crate::validate_pole_boundary(s,&ce.pcurve,pole).is_err() {continue;}
            let a=&ce.pcurve.control_points[0];let b=&ce.pcurve.control_points[1];
            if a[0]==b[0] && ((a[1]==v[0]&&b[1]==v[1])||(a[1]==v[1]&&b[1]==v[0])) {
                if let Some(end)=u.iter().position(|value|*value==a[0]) {return Some((end,ce.edge,edge.vertices[0]));}
            }
        }
    }
    None
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn regular_and_quotient_proofs_survive_rigid_placement() {
        let base=crate::circular_blend::partial_annular_quarter(20.,5.,6.,1.25,1.,1e-7).unwrap();
        let (sa,ca)=0.37_f64.sin_cos();let (sb,cb)=(-0.61_f64).sin_cos();
        let placed=crate::transform::affine(&base,[[ca*cb,-sa,ca*sb,17.],[sa*cb,ca,sa*sb,-9.],[-sb,0.,cb,23.],[0.,0.,0.,1.]]).unwrap();
        let before=format!("{placed:?}");
        for (face,end) in [(0,0),(10,1)] {
            let proof=nurbs_core::surface_quotient_injectivity::certify_source_frame(&placed.faces[face].surface,end,16,256).unwrap();
            assert!(proof.proven,"placed pole: {proof:?}");
        }
        let r=inspect(&placed,4096).unwrap();
        assert!(r.all_faces_injective);
        assert_eq!(r.faces.iter().filter(|f|f.result.as_ref().is_some_and(|r|r.proven)).count(),25);
        assert_eq!(r.faces.iter().filter(|f|f.quotient.as_ref().is_some_and(|q|q.result.proven)).count(),2);
        let torus=r.faces[5].result.as_ref().unwrap();
        assert!(torus.proven);assert_eq!(torus.reason,"global-polar-projection-contraction");
        assert!(torus.polar_projection.is_some());assert!(torus.contraction_upper.unwrap()<1.);
        for i in [0,10] { assert_eq!(r.faces[i].result.as_ref().unwrap().reason,"collapsed-boundary-requires-quotient-proof"); }
        assert_eq!(format!("{placed:?}"),before);
    }
    #[test]
    fn quotient_budget_and_pole_ownership_are_required() {
        let base=crate::circular_blend::partial_annular_quarter(20.,5.,6.,1.25,1.,1e-7).unwrap();
        let before=format!("{base:?}");
        let short=inspect(&base,255).unwrap();
        assert!(short.faces[0].quotient.is_none());
        assert!(!short.all_faces_injective);assert!(short.spans<=255);
        let exact=inspect(&base,256).unwrap();
        assert!(exact.faces[0].quotient.as_ref().unwrap().result.proven);
        assert_eq!(exact.spans,256);assert!(!exact.all_faces_injective);
        assert!(exact.faces.iter().skip(1).all(|f|f.result.is_none()&&f.quotient.is_none()));
        let (_,edge,_)=declared_pole(&base,0).unwrap();
        let mut missing=base.clone();
        missing.loops[base.faces[0].outer].coedges.retain(|ce|ce.edge!=edge);
        assert!(declared_pole(&missing,0).is_none());
        let mut partial=base.clone();
        let ce=partial.loops[base.faces[0].outer].coedges.iter_mut().find(|ce|ce.edge==edge).unwrap();
        ce.pcurve.control_points[1][1]=(ce.pcurve.control_points[0][1]+ce.pcurve.control_points[1][1])*0.5;
        assert!(declared_pole(&partial,0).is_none());
        assert_eq!(format!("{base:?}"),before);
    }
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
