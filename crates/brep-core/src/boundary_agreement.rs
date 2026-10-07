//! Full-interval edge/pcurve lift checks, distinct from sampled validation.
use crate::Model;
use nurbs_core::{
    Error, Result,
    curve_surface_agreement::{self, Status},
};

#[derive(Clone, Debug)]
pub struct BoundaryUse {
    pub face: usize,
    pub wire: usize,
    pub coedge: usize,
    pub edge: usize,
    pub status: Status,
    pub witness: Option<f64>,
    pub witness_distance: Option<[f64; 2]>,
}
#[derive(Clone, Debug)]
pub struct Report {
    /// Only edge/lift agreement, not self-intersection or volume validity.
    pub complete: bool,
    pub cells: usize,
    pub uses: Vec<BoundaryUse>,
}

/// A shared budget includes both uses of each edge. Unprocessed uses are
/// explicitly unresolved. The model is immutable. Structural validation remains
/// mandatory, while full-interval checks replace sampled edge/lift admission.
pub fn verify(model: &Model, max_cells: usize) -> Result<Report> {
    model.validate_boundary_diagnostic_inputs()?;
    if max_cells == 0 || max_cells > 1_000_000 {
        return Err(Error::new(
            "BREP_AGREEMENT_BUDGET",
            "Boundary agreement budget must be in 1..1000000",
        ));
    }
    let mut report = Report {
        complete: true,
        cells: 0,
        uses: Vec::new(),
    };
    for (face_id, face) in model.faces.iter().enumerate() {
        for &wire in std::iter::once(&face.outer).chain(&face.holes) {
            for (coedge_id, coedge) in model.loops[wire].coedges.iter().enumerate() {
                let mut entry = BoundaryUse {
                    face: face_id,
                    wire,
                    coedge: coedge_id,
                    edge: coedge.edge,
                    status: Status::Unresolved,
                    witness: None,
                    witness_distance: None,
                };
                if report.cells < max_cells {
                    let checked = curve_surface_agreement::verify(
                        &model.edges[coedge.edge].curve,
                        &coedge.pcurve,
                        &face.surface,
                        coedge.reversed,
                        model.tolerance_mm,
                        (max_cells - report.cells).min(100_000),
                    )?;
                    report.cells += checked.cells;
                    entry.status = checked.status;
                    entry.witness = checked.witness;
                    entry.witness_distance = checked.witness_distance;
                }
                report.complete &= entry.status == Status::WithinTolerance;
                report.uses.push(entry);
            }
        }
    }
    Ok(report)
}
#[derive(Clone, Debug)]
pub struct ExactUse {
    pub face: usize,
    pub wire: usize,
    pub coedge: usize,
    pub edge: usize,
    /// None for an unsupported representation or an unvisited use.
    pub decision: Option<cad_predicates::BezierIdentityDecision>,
}
#[derive(Clone, Debug)]
pub struct ExactReport {
    pub all_equal: bool,
    /// Endpoint closure alone does not establish simple trim regions.
    pub joins: Vec<(usize, usize, Option<bool>)>,
    pub all_joins_exact: bool,
    pub work: u64,
    pub uses: Vec<ExactUse>,
}
/// Exact edge/lift identity, distinct from tolerance agreement. This alone is
/// not a certificate of valid trim regions, embedding or solid volume.
pub fn verify_exact(model: &Model, max_work: u64) -> Result<ExactReport> {
    model.validate_boundary_diagnostic_inputs()?;
    if max_work == 0 || max_work > cad_predicates::MAX_WORK {
        return Err(Error::new("BREP_AGREEMENT_BUDGET","Exact boundary work must be in 1..1000000"));
    }
    let mut report=ExactReport{all_equal:true,joins:Vec::new(),all_joins_exact:true,work:0,uses:Vec::new()};
    for (face_id,face) in model.faces.iter().enumerate() {
        for &wire in std::iter::once(&face.outer).chain(&face.holes) {
            let curves=model.loops[wire].coedges.iter().map(|c|c.pcurve.clone()).collect::<Vec<_>>();
            let closed=nurbs_core::trim_domain::exact_loop_joins(&curves)?;
            report.all_joins_exact &= closed==Some(true);
            report.joins.push((face_id,wire,closed));
            for (coedge_id,coedge) in model.loops[wire].coedges.iter().enumerate() {
                let decision=if report.work<max_work {
                    curve_surface_agreement::verify_exact(&model.edges[coedge.edge].curve,&coedge.pcurve,&face.surface,coedge.reversed,max_work-report.work)?
                }else{None};
                if let Some(d)=&decision {report.work+=d.work_used;}
                report.all_equal &= decision.as_ref().is_some_and(|d|d.outcome==cad_predicates::BezierIdentity::Equal);
                report.uses.push(ExactUse{face:face_id,wire,coedge:coedge_id,edge:coedge.edge,decision});
            }
        }
    }
    Ok(report)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn moving_hollow_tensor_sides_fit_shared_exact_work_budget() {
        let ring=|radius:f64,z:f64,f:f64| {
            let h=f.hypot(1.);
            [[ [radius,0.],[radius,radius],[0.,radius]],
             [[0.,radius],[-radius,radius],[-radius,0.]],
             [[-radius,0.],[-radius,-radius],[0.,-radius]],
             [[0.,-radius],[radius,-radius],[radius,0.]]].into_iter().map(|points|nurbs_core::curve::Curve {
                degree:2,knots:vec![0.,0.,0.,1.,1.,1.],
                control_points:points.into_iter().map(|xy|vec![2.*xy[0],xy[1]/h,z-xy[1]*f/h]).collect(),
                weights:vec![1.,std::f64::consts::FRAC_1_SQRT_2,1.],periodic:false,
            }).collect::<Vec<_>>()
        };
        let mut sections=(0..=8).map(|i| {
            let f=i as f64/8.;let outer=ring(0.5,10.*f,f);
            let hole=ring(0.25,10.*f,f).into_iter().rev().map(|c|c.reverse().unwrap()).collect();
            vec![outer,hole]
        }).collect::<Vec<_>>();
        for (index,z) in [(0,0.),(8,10.)] {
            let flat=sections[index].iter().flatten().cloned().collect::<Vec<_>>();
            let projected=nurbs_core::section_projection::project(&flat,2,[0.,0.],z,2_f64.powi(-40),0.6,1000000).unwrap().curves.unwrap();
            sections[index]=vec![projected[..4].to_vec(),projected[4..].to_vec()];
        }
        let model=crate::rational_section_loft(&sections).unwrap();
        assert_eq!(model.faces.len(),66);
        let report=verify_exact(&model,1000000).unwrap();
        assert!(report.all_equal && report.all_joins_exact,"work={} unresolved={:?}",report.work,
            report.uses.iter().filter(|u|!u.decision.as_ref().is_some_and(|d|d.outcome==cad_predicates::BezierIdentity::Equal)).collect::<Vec<_>>());
        assert!(report.work<1000000);
        let partial=verify_exact(&model,report.work-1).unwrap();
        assert!(!partial.all_equal && partial.work<=report.work-1);
        assert_eq!(partial.uses.len(),report.uses.len());
    }
    #[test]
    fn exact_closure_survives_a_nonplanar_bilinear_warp_and_weight_scaling() {
        let mut m=crate::cuboid([0.;3],[1.;3]).unwrap();
        // z -> z + xy/4: top/bottom become bilinear graphs. Every box edge
        // has constant x or y, so its exact image remains a straight segment.
        for v in &mut m.vertices {v.point[2]+=v.point[0]*v.point[1]/4.;}
        for e in &mut m.edges {
            for p in &mut e.curve.control_points {p[2]+=p[0]*p[1]/4.;}
            for w in &mut e.curve.weights {*w*=3.;}
        }
        for f in &mut m.faces {
            for row in &mut f.surface.control_points {for p in row {p[2]+=p[0]*p[1]/4.;}}
            for row in &mut f.surface.weights {for w in row {*w*=2.;}}
        }
        m.validate().unwrap();
        let r=verify_exact(&m,1000000).unwrap();
        assert!(r.all_equal);assert!(r.all_joins_exact);assert_eq!(r.joins.len(),6);assert_eq!(r.uses.len(),24);
        let needed=r.work;
        assert!(needed>0 && needed<1000000);
        assert!(verify_exact(&m,needed).unwrap().all_equal);
        let partial=verify_exact(&m,needed-1).unwrap();
        assert!(!partial.all_equal);assert!(partial.work<=needed-1);
        assert_eq!(partial.uses.len(),24);
    }
    #[test]
    fn exact_closure_distinguishes_a_subtolerance_gap_and_keeps_every_use() {
        let mut m=crate::cuboid([0.;3],[1.;3]).unwrap();
        let r=verify_exact(&m,1000000).unwrap();
        assert!(r.all_equal);assert!(r.all_joins_exact);assert_eq!(r.joins.len(),6);assert_eq!(r.uses.len(),24);assert!(r.work<=1000000);
        let partial=verify_exact(&m,1).unwrap();
        assert!(!partial.all_equal);assert_eq!(partial.uses.len(),24);assert!(partial.work<=1);
        m.edges[0].curve.control_points[0][2]+=1e-12;
        assert!(verify(&m,10000).unwrap().complete);
        let before=format!("{m:?}");
        let r=verify_exact(&m,1000000).unwrap();
        assert!(!r.all_equal);
        assert!(r.uses.iter().any(|u|u.decision.as_ref().is_some_and(|d|d.outcome==cad_predicates::BezierIdentity::Different)));
        assert_eq!(format!("{m:?}"),before);
    }
    #[test]
    fn exhaustion_preserves_all_boundary_uses_without_mutation() {
        let model = crate::cuboid([0.; 3], [1.; 3]).unwrap();
        let before = format!("{model:?}");
        let result = verify(&model, 1).unwrap();
        assert!(!result.complete);
        assert_eq!(result.cells, 1);
        assert_eq!(result.uses.len(), 24);
        assert_eq!(result.uses[0].status, Status::WithinTolerance);
        assert!(
            result.uses[1..]
                .iter()
                .all(|u| u.status == Status::Unresolved)
        );
        assert_eq!(format!("{model:?}"), before);
    }
    #[test]
    fn cylinder_and_sphere_boundary_charts_at_model_tolerance() {
        for model in [crate::cylinder(2., 3.).unwrap(), crate::sphere(2.).unwrap()] {
            let report = verify(&model, 4096).unwrap();
            assert!(report.complete);
            assert_eq!(report.uses.len(), 24);
            assert!(
                report
                    .uses
                    .iter()
                    .all(|u| u.status == Status::WithinTolerance)
            );
        }
    }
    #[test]
    fn lifted_periodic_uv_is_diagnosable_without_admitting_regular_operations() {
        let mut model = crate::cuboid([0.; 3], [1.; 3]).unwrap();
        let face = &mut model.faces[0];
        face.surface
            .control_points
            .push(face.surface.control_points[0].clone());
        face.surface.weights.push(face.surface.weights[0].clone());
        face.surface.knots_u = vec![-1., 0., 1., 2., 3.];
        face.surface.periodic_u = true;
        let wire = face.outer;
        for coedge in &mut model.loops[wire].coedges {
            for uv in &mut coedge.pcurve.control_points {
                uv[0] += 2.;
            }
        }
        let before = format!("{model:?}");
        assert!(model.validate().is_err());
        let result = verify(&model, 10_000).unwrap();
        assert!(result.complete);
        assert_eq!(result.uses.len(), 24);
        assert!(model.validate().is_err());
        assert_eq!(format!("{model:?}"), before);
    }
    #[test]
    fn sampled_mismatch_gets_a_local_diagnostic_and_bad_indices_still_refuse() {
        let mut model = crate::cuboid([0.; 3], [1.; 3]).unwrap();
        for row in &mut model.faces[0].surface.control_points {
            for cp in row {
                cp[2] += 0.125;
            }
        }
        assert!(model.validate().is_err());
        let result = verify(&model, 10_000).unwrap();
        assert!(!result.complete);
        let bad: Vec<_> = result
            .uses
            .iter()
            .filter(|u| u.status == Status::Mismatch)
            .collect();
        assert_eq!(bad.len(), 4);
        assert!(
            bad.iter()
                .all(|u| u.face == 0 && u.witness_distance.unwrap()[0] > model.tolerance_mm)
        );
        model.loops[0].coedges[0].edge = usize::MAX;
        assert!(verify(&model, 10_000).is_err());
    }
    #[test]
    fn every_cube_boundary_use_can_be_proven() {
        let model = crate::cuboid([0.; 3], [1.; 3]).unwrap();
        let result = verify(&model, 24).unwrap();
        assert!(result.complete);
        assert_eq!(result.uses.len(), 24);
        assert_eq!(result.cells, 24);
        assert!(
            result
                .uses
                .iter()
                .all(|u| u.status == Status::WithinTolerance)
        );
    }
}
