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
    inspect_with_linear(model, max_spans, 0)
}
/// Independent shared budgets for contraction spans and oblique refinement.
/// A successful alternative proves the same whole-chart injectivity property.
pub fn inspect_with_linear(model: &Model, max_spans: usize, max_linear_cells: usize) -> Result<Report> {
    inspect_with_projections(model,max_spans,max_linear_cells,&[])
}
pub fn inspect_with_projections(model: &Model, max_spans: usize, max_linear_cells: usize, proposals: &[Option<[[i8;3];2]>]) -> Result<Report> {
    let mut inspector = Inspector::new(std::sync::Arc::new(model.clone()), max_spans, max_linear_cells, proposals.to_vec())?;
    while !inspector.finished() { inspector.advance(64)?; }
    inspector.finish()
}

/// Native continuation over one immutable model. Progress never exposes a
/// positive partial certificate; original span/linear budgets span all steps.
pub(crate) struct Inspector {
    model: std::sync::Arc<Model>,
    max_spans: usize,
    max_linear_cells: usize,
    proposals: Vec<Option<[[i8;3];2]>>,
    report: Report,
}
impl Inspector {
    pub(crate) fn new(model: std::sync::Arc<Model>, max_spans: usize, max_linear_cells: usize,
        proposals: Vec<Option<[[i8;3];2]>>) -> Result<Self> {
        model.validate_boundary_diagnostic_inputs()?;
        if max_spans == 0 || max_spans > 100_000 || max_linear_cells > 100_000 {
            return Err(Error::new("BREP_INJECTIVITY_BUDGET", "Face injectivity budget must be in 1..100000"));
        }
        let face_count = model.faces.len();
        Ok(Self {model, max_spans, max_linear_cells, proposals,
            report: Report {all_faces_injective:true,spans:0,linear_cells:0,faces:Vec::with_capacity(face_count)}})
    }
    pub(crate) fn completed_faces(&self) -> usize { self.report.faces.len() }
    pub(crate) fn finished(&self) -> bool { self.completed_faces() == self.model.faces.len() }
    pub(crate) fn advance(&mut self, max_faces: usize) -> Result<usize> {
        if !(1..=100_000).contains(&max_faces) {
            return Err(Error::new("BREP_INJECTIVITY_STEP", "Injectivity step requires 1..100000 faces"));
        }
        let start=self.completed_faces();
        let stop=start.saturating_add(max_faces).min(self.model.faces.len());
        while self.completed_faces()<stop {
            let face=self.completed_faces();
            let f=&self.model.faces[face];
            let result=if self.report.spans<self.max_spans {
                Some(surface_injectivity::certify(&f.surface,self.max_spans-self.report.spans)?)
            }else{None};
            let linear=if !result.as_ref().is_some_and(|r|r.proven)
                && self.report.linear_cells<self.max_linear_cells {
                Some(nurbs_core::surface_linear_monotonicity::inspect_candidate_with_hint(
                    &f.surface,self.proposals.get(face).copied().flatten(),
                    self.max_linear_cells-self.report.linear_cells)?)
            }else{None};
            self.report.linear_cells+=linear.as_ref().map_or(0,|r|r.cells);
            self.report.all_faces_injective &= result.as_ref().is_some_and(|r|r.proven)
                || linear.as_ref().is_some_and(|r|r.certified);
            self.report.spans+=result.as_ref().map_or(0,|r|r.spans);
            self.report.faces.push(Face {face,result,linear});
        }
        Ok(self.completed_faces()-start)
    }
    pub(crate) fn finish(self) -> Result<Report> {
        if !self.finished() {
            return Err(Error::new("BREP_INJECTIVITY_PENDING", "Incomplete native injectivity audit has no final report"));
        }
        Ok(self.report)
    }
}

#[cfg(test)]
fn frozen_inspect_with_projections(model: &Model, max_spans: usize, max_linear_cells: usize, proposals: &[Option<[[i8;3];2]>]) -> Result<Report> {
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
            Some(surface_injectivity::certify(
                &f.surface,
                max_spans - report.spans,
            )?)
        } else {
            None
        };
        let linear = if !result.as_ref().is_some_and(|r| r.proven)
            && report.linear_cells < max_linear_cells {
            Some(nurbs_core::surface_linear_monotonicity::inspect_candidate_with_hint(
                &f.surface, proposals.get(face).copied().flatten(), max_linear_cells-report.linear_cells)?)
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
    fn resumable_injectivity_keeps_original_shared_budgets_and_report_order() {
        let section=|z|vec![
            vec![nurbs_core::primitives::circle([0.,0.,z],[0.,0.,1.],0.5).unwrap()],
            vec![nurbs_core::primitives::circle([0.,0.,z],[0.,0.,1.],0.2).unwrap().reverse().unwrap()],
        ];
        let models=[crate::cuboid([0.;3],[1.;3]).unwrap(),
            crate::rational_section_loft(&[section(0.),section(5.),section(10.)]).unwrap()];
        for model in models {
            for (spans,linear) in [(1,0),(1,1),(1000,0),(1000,1),(1000,10000)] {
                let expected=frozen_inspect_with_projections(&model,spans,linear,&[]).unwrap();
                for batch in [1,2,64] {
                    let mut source=std::sync::Arc::new(model.clone());
                    let mut state=Inspector::new(source.clone(),spans,linear,Vec::new()).unwrap();
                    assert!(std::sync::Arc::get_mut(&mut source).is_none());
                    while !state.finished() {
                        let before=state.completed_faces();
                        let done=state.advance(batch).unwrap();
                        assert!(done>0 && done<=batch);
                        assert_eq!(state.completed_faces(),before+done);
                    }
                    assert_eq!(state.advance(batch).unwrap(),0);
                    let actual=state.finish().unwrap();
                    assert_eq!(format!("{actual:?}"),format!("{expected:?}"));
                    assert!(actual.spans<=spans && actual.linear_cells<=linear);
                    assert_eq!(*source,model);
                }
            }
        }
    }
    #[test]
    fn resumable_injectivity_refuses_partial_report_and_invalid_step() {
        let model=std::sync::Arc::new(crate::cuboid([0.;3],[1.;3]).unwrap());
        let mut state=Inspector::new(model,6,0,Vec::new()).unwrap();
        assert_eq!(state.completed_faces(),0);
        assert!(state.advance(0).is_err());
        assert_eq!(state.completed_faces(),0);
        assert_eq!(state.advance(1).unwrap(),1);
        assert!(!state.finished());
        assert!(state.finish().is_err());
    }

    #[test]
    #[cfg(feature = "codec")]
    #[ignore = "requires an explicitly supplied retained-model diagnostic fixture"]
    fn diagnose_retained_model_shared_injectivity_work() {
        let path = std::env::var("OSV_INJECTIVITY_MODEL").expect("OSV_INJECTIVITY_MODEL");
        let model: Model = value_codec::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        let report = inspect_with_linear(&model, 100000, 100000).unwrap();
        for face in &report.faces {
            if let Some(linear) = &face.linear {
                if linear.cells > 100 || !linear.certified {
                    eprintln!("face={} contraction={:?} linear={linear:?}", face.face, face.result);
                }
            }
        }
        eprintln!("all={} spans={} linear_cells={}", report.all_faces_injective, report.spans, report.linear_cells);
        assert!(report.all_faces_injective);
    }
    #[test]
    fn retained_hollow_walls_use_shared_oblique_budget_without_overwriting_contraction() {
        let section=|z|vec![
            vec![nurbs_core::primitives::circle([0.,0.,z],[0.,0.,1.],0.5).unwrap()],
            vec![nurbs_core::primitives::circle([0.,0.,z],[0.,0.,1.],0.2).unwrap().reverse().unwrap()],
        ];
        let model=crate::rational_section_loft(&[section(0.),section(5.),section(10.)]).unwrap();
        let before=model.clone();
        let original=inspect(&model,1000).unwrap();
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
        let limited=inspect_with_linear(&model,1000,1).unwrap();
        assert!(!limited.all_faces_injective);
        assert!(limited.linear_cells<=1);
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
