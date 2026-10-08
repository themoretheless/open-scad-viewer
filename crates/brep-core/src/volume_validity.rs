//! Sufficient geometric volume evidence; unsupported or exhausted stages remain
//! unproven. Uses authored boundary geometry, not a tessellated volume estimate.
use crate::{Error, Model, Result, boundary_embedding, shell_nesting, shell_orientation, solid_audit};
#[derive(Clone, Copy)]
pub struct Limits {
    pub boundary: boundary_embedding::Limits,
    pub nesting_pairs: usize,
    pub nesting_cells: usize,
    pub nesting_domain_cells: usize,
    pub orientation_cells: usize,
    pub orientation_domain_cells: usize,
    pub orientation_spans: usize,
}
pub struct ShellOrientation {
    pub shell: usize,
    pub expected_outward: bool,
    pub outward: Option<bool>,
    pub attempts: Vec<shell_orientation::Report>,
}
pub struct Report {
    pub proven: bool,
    pub boundary: boundary_embedding::Report,
    pub nesting: Option<shell_nesting::Report>,
    pub orientations: Vec<ShellOrientation>,
    pub orientation_cells: usize,
    pub orientation_domain_cells: usize,
}
pub fn inspect(model: &Model, tolerance_uv: f64, limits: Limits) -> Result<Report> {
    inspect_impl(model,tolerance_uv,limits,0,None,&[])
}
/// Joint sweep boundary evidence precedes nesting and material orientation.
/// Empty cap selection is the closed no-cap case; no artificial caps are inferred.
pub fn inspect_sweep(model:&Model,tolerance_uv:f64,limits:Limits,max_linear_cells:usize,
    caps:&[usize],cap_limits:crate::sweep_cap_contacts::Budgets)->Result<Report> {
    inspect_impl(model,tolerance_uv,limits,max_linear_cells,Some((caps,cap_limits)),&[])
}
pub fn inspect_sweep_with_projections(model:&Model,tolerance_uv:f64,limits:Limits,max_linear_cells:usize,
    caps:&[usize],cap_limits:crate::sweep_cap_contacts::Budgets,proposals:&[Option<[[i8;3];2]>])->Result<Report> {
    inspect_impl(model,tolerance_uv,limits,max_linear_cells,Some((caps,cap_limits)),proposals)
}
fn inspect_impl(model:&Model,tolerance_uv:f64,limits:Limits,max_linear_cells:usize,
    caps:Option<(&[usize],crate::sweep_cap_contacts::Budgets)>,proposals:&[Option<[[i8;3];2]>])->Result<Report> {
    if !(1..=1000000).contains(&limits.orientation_cells)
        || !(1..=8000000).contains(&limits.orientation_domain_cells)
        || !(1..=100000).contains(&limits.orientation_spans)
        || !(1..=100000).contains(&limits.nesting_pairs)
        || !(2..=1000000).contains(&limits.nesting_cells)
        || !(2..=8000000).contains(&limits.nesting_domain_cells) {
        return Err(Error::new("BREP_INVALID_INPUT", "Volume audit requires bounded positive stage budgets"));
    }
    let boundary = match caps {
        Some((caps,budget))=>boundary_embedding::inspect_sweep_with_projections(
            model,tolerance_uv,limits.boundary,max_linear_cells,caps,budget,proposals)?,
        _=>boundary_embedding::inspect_with_linear(model,tolerance_uv,limits.boundary,max_linear_cells)?,
    };
    let mut out = Report { proven: false, boundary, nesting: None,
        orientations: (0..model.shells.len()).map(|shell| ShellOrientation {
            shell, expected_outward: !model.bodies.iter().any(|b| b.inner_shells.contains(&shell)),
            outward: None, attempts: Vec::new() }).collect(),
        orientation_cells: 0, orientation_domain_cells: 0 };
    if !out.boundary.proven { return Ok(out); }
    let nesting = shell_nesting::inspect_with_boundary(model, model.tolerance_mm, tolerance_uv,
        limits.nesting_pairs, limits.nesting_cells, limits.nesting_domain_cells,Some(&out.boundary))?;
    let roles = nesting.roles_consistent == Some(true);
    out.nesting = Some(nesting);
    if !roles { return Ok(out); }
    for i in 0..model.shells.len() {
        // Keep the actual cavity orientation. The extraction helper may invert
        // it for recognition, but that would erase the fact under inspection.
        let shell = solid_audit::isolated_outward_shell(model, i, false)?;
        let mut bounds = [[f64::INFINITY, f64::NEG_INFINITY];3];
        for p in shell.faces.iter().flat_map(|f| &f.surface.control_points).flatten() {
            for k in 0..3 { bounds[k][0] = bounds[k][0].min(p[k]); bounds[k][1] = bounds[k][1].max(p[k]); }
        }
        let point = bounds.map(|b| b[0]*0.5+b[1]*0.5);
        let directions = [[1.,0.317,0.173],[0.239,1.,0.419],[0.137,0.283,1.]];
        let mut rays=directions.into_iter().map(|d|(point,d)).collect::<Vec<_>>();
        // A box centre may lie in the empty hole of a toroidal shell and all
        // rays can miss it. Propose additional rays aimed at a retained chart.
        // Evaluated points only propose origins; every crossing and its normal
        // still require the independent interval ray/orientation certificate.
        let s=&shell.faces[0].surface;
        // Avoid targeting the dyadic subdivision seam itself, where a strict
        // interior root certificate may remain unresolved in both children.
        let u=s.knots_u[s.degree_u]*0.629+s.knots_u[s.control_points.len()]*0.371;
        let v=s.knots_v[s.degree_v]*0.581+s.knots_v[s.control_points[0].len()]*0.419;
        if let Ok(target)=s.evaluate(u,v) {
            let reach=bounds.iter().map(|b|b[1]-b[0]).fold(0_f64,f64::max)*4.;
            for direction in directions {
                let origin=std::array::from_fn(|k|target.point[k]-direction[k]*reach);
                if reach>0. && reach.is_finite() && origin.iter().all(|x|x.is_finite()) {
                    rays.push((origin,direction));
                }
            }
        }
        let ray_count=rays.len();
        for (attempt, (origin,direction)) in rays.into_iter().enumerate() {
            let cells = limits.orientation_cells-out.orientation_cells;
            let domains = limits.orientation_domain_cells-out.orientation_domain_cells;
            if cells == 0 || domains == 0 { break; }
            let remaining = (model.shells.len()-i-1)*6+ray_count-attempt;
            let r = shell_orientation::inspect(&shell, origin, direction, tolerance_uv,
                (cells/remaining).max(1), (domains/remaining).max(1), limits.orientation_spans)?;
            out.orientation_cells += r.ray.cells;
            out.orientation_domain_cells += r.ray.domain_cells;
            out.orientations[i].outward = r.outward;
            out.orientations[i].attempts.push(r);
            if out.orientations[i].outward.is_some() { break; }
        }
    }
    out.proven = out.orientations.iter().all(|r| r.outward == Some(r.expected_outward));
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn orientation_rejects_invalid_limits_before_admitting_geometry() {
        let m=crate::cuboid([0.;3],[1.;3]).unwrap();
        let before=m.clone();
        for budget in [0,1000001] {
            assert!(inspect(&m,1e-8,Limits {orientation_cells:budget,..limits()}).is_err());
        }
        assert!(inspect(&m,1e-8,Limits {orientation_domain_cells:0,..limits()}).is_err());
        assert!(inspect(&m,1e-8,Limits {orientation_spans:0,..limits()}).is_err());
        assert_eq!(m,before);
    }
    fn limits() -> Limits {
        Limits { boundary: boundary_embedding::Limits { exact_work: 1000000,
            trim_pairs: 10000, trim_cells: 100000, trim_domain_cells: 1000000, spans: 1000,
            contacts: crate::face_contacts::Limits { pairs: 10000, cells: 100000,
                domain_cells: 1000000, cells_per_pair: 1000, domain_cells_per_pair: 10000 } },
            nesting_pairs: 100, nesting_cells: 100000, nesting_domain_cells: 1000000,
            orientation_cells: 100000, orientation_domain_cells: 1000000, orientation_spans: 100 }
    }
    #[test]
    fn corrected_inflection_hollow_arc_covers_every_pair_in_original_limits() {
        use nurbs_core::{curve::Curve,primitives::line,progressive_sweep::{Options,Orientation,Spacing,constant_vector_law}};
        let ring=|points:[[f64;3];4]|(0..4).map(|i|line(points[i],points[(i+1)%4]).unwrap()).collect();
        let loops=vec![ring([[0.03125,-0.03125,-0.03125],[-0.03125,0.03125,-0.03125],[-0.03125,0.03125,0.03125],[0.03125,-0.03125,0.03125]]),
            ring([[0.015625,-0.015625,-0.015625],[0.015625,-0.015625,0.015625],[-0.015625,0.015625,0.015625],[-0.015625,0.015625,-0.015625]])];
        let path=Curve {degree:3,knots:vec![0.,0.,0.,0.,1.,1.,1.,1.],
            control_points:vec![vec![0.,0.,0.],vec![1.,1.,0.],vec![2.,-1.,0.],vec![3.,0.,0.]],weights:vec![1.;4],periodic:false};
        let scale=constant_vector_law([1.,0.,0.]).unwrap();let twist=constant_vector_law([0.;3]).unwrap();
        let body=crate::analytic::progressive_profile_body_with_evidence_and_correction(&loops,&path,&scale,&twist,None,None,None,
            Options {normal:[0.,0.,1.],orientation:Orientation::CorrectedFrenet,
                spacing:Spacing::ArcLength {tolerance:0.001,max_cells:100000},initial_sections:3,max_sections:65,max_deviation:0.1},
            Some(crate::analytic::EndpointCapCorrection {quantum:2_f64.powi(-40),tolerance:1e-9,max_work:1000000})).unwrap();
        assert_eq!(body.boundary_error_within_budget,Some(true));
        assert!(body.boundary_error_upper.unwrap()<=0.1);
        assert_eq!(body.model.faces.len(),258);
        let before=body.model.clone();
        let report=inspect_sweep(&body.model,1e-8,limits(),20000,&[256,257],crate::sweep_cap_contacts::Budgets {
            max_walls:1024,max_exact_work:1000000,max_chart_cells:1000,max_trim_pairs:100000,
            max_trim_cells:100000,max_trim_domain_cells:1000000}).unwrap();
        let pairs=&report.boundary.intersections.pairs;
        assert!(report.proven,"boundary={} next={:?} individual={} grouped={} cells={} unresolved={:?}",
            report.boundary.proven,pairs.next_pair,pairs.pairs.len(),pairs.grouped_pairs,pairs.cells,
            pairs.pairs.iter().filter(|p|p.reason=="pair-unresolved").map(|p|p.faces).take(10).collect::<Vec<_>>());
        assert_eq!(pairs.total_pairs,33153);
        assert_eq!(pairs.pairs.len()+pairs.grouped_pairs,pairs.total_pairs);
        assert!(pairs.pairs.len()<=10000 && pairs.cells<=100000 && pairs.domain_cells<=1000000);
        assert!(pairs.grouped_pairs>0 && pairs.next_pair.is_none());
        assert_eq!(body.model,before);
    }
    #[test]
    fn corrected_affine_inflection_hollow_parameter_and_arc_preserve_original_limits() {
        use nurbs_core::{curve::Curve,primitives::line,progressive_sweep::{Options,Orientation,Spacing,constant_vector_law}};
        let ring=|points:[[f64;3];4]|(0..4).map(|i|line(points[i],points[(i+1)%4]).unwrap()).collect();
        let loops=vec![ring([[0.03125,-0.03125,-0.03125],[-0.03125,0.03125,-0.03125],[-0.03125,0.03125,0.03125],[0.03125,-0.03125,0.03125]]),
            ring([[0.015625,-0.015625,-0.015625],[0.015625,-0.015625,0.015625],[-0.015625,0.015625,0.015625],[-0.015625,0.015625,-0.015625]])];
        let path=Curve {degree:3,knots:vec![0.,0.,0.,0.,1.,1.,1.,1.],
            control_points:vec![vec![0.,0.,0.],vec![1.,1.,0.],vec![2.,-1.,0.],vec![3.,0.,0.]],weights:vec![1.;4],periodic:false};
        let scale=constant_vector_law([1.,0.,0.]).unwrap();let twist=constant_vector_law([0.;3]).unwrap();
        let axes=constant_vector_law([1.25,0.75,1.]).unwrap();
        let center=constant_vector_law([0.002,-0.003,0.]).unwrap();
        for spacing in [Spacing::Parameter,Spacing::ArcLength {tolerance:0.001,max_cells:100000}] {
        let body=crate::analytic::progressive_profile_body_with_evidence_and_correction(&loops,&path,&scale,&twist,Some((&axes,&center)),None,None,
            Options {normal:[0.,0.,1.],orientation:Orientation::CorrectedFrenet,
                spacing,initial_sections:3,max_sections:65,max_deviation:0.1},
            Some(crate::analytic::EndpointCapCorrection {quantum:2_f64.powi(-40),tolerance:1e-9,max_work:1000000})).unwrap();
        assert_eq!(body.boundary_error_within_budget,Some(true));
        assert!(body.boundary_error_upper.unwrap()<=0.1);
        let face_count=body.model.faces.len();
        let before=body.model.clone();
        let report=inspect_sweep(&body.model,1e-8,limits(),20000,&[face_count-2,face_count-1],crate::sweep_cap_contacts::Budgets {
            max_walls:1024,max_exact_work:1000000,max_chart_cells:1000,max_trim_pairs:100000,
            max_trim_cells:100000,max_trim_domain_cells:1000000}).unwrap();
        let pairs=&report.boundary.intersections.pairs;
        assert!(report.proven,"boundary={} next={:?} individual={} grouped={} cells={} unresolved={:?}",
            report.boundary.proven,pairs.next_pair,pairs.pairs.len(),pairs.grouped_pairs,pairs.cells,
            pairs.pairs.iter().filter(|p|p.reason=="pair-unresolved").map(|p|p.faces).take(10).collect::<Vec<_>>());
        assert_eq!(pairs.total_pairs,face_count*(face_count-1)/2);
        assert_eq!(pairs.pairs.len()+pairs.grouped_pairs,pairs.total_pairs);
        assert!(pairs.pairs.len()<=10000 && pairs.cells<=100000 && pairs.domain_cells<=1000000);
        assert!(pairs.grouped_pairs>0 && pairs.next_pair.is_none());
        assert_eq!(body.model,before);
        }
    }
    #[test]
    fn corrected_affine_zero_initial_curvature_hollow_parameter_and_arc_preserve_limits() {
        use nurbs_core::{curve::Curve,primitives::line,progressive_sweep::{Options,Orientation,Spacing,constant_vector_law}};
        let ring=|points:[[f64;3];4]|(0..4).map(|i|line(points[i],points[(i+1)%4]).unwrap()).collect();
        let loops=vec![ring([[0.,-0.03125,-0.03125],[0.,0.03125,-0.03125],[0.,0.03125,0.03125],[0.,-0.03125,0.03125]]),
            ring([[0.,-0.015625,-0.015625],[0.,-0.015625,0.015625],[0.,0.015625,0.015625],[0.,0.015625,-0.015625]])];
        let path=Curve {degree:3,knots:vec![0.,0.,0.,0.,1.,1.,1.,1.],
            control_points:vec![vec![0.,0.,0.],vec![1.,0.,0.],vec![2.,0.,0.],vec![3.,1.,0.]],weights:vec![1.;4],periodic:false};
        let scale=constant_vector_law([1.,0.,0.]).unwrap();let twist=constant_vector_law([0.;3]).unwrap();
        let axes=constant_vector_law([1.25,0.75,1.]).unwrap();
        let center=constant_vector_law([0.002,-0.003,0.]).unwrap();
        for spacing in [Spacing::Parameter,Spacing::ArcLength {tolerance:0.001,max_cells:100000}] {
        let body=crate::analytic::progressive_profile_body_with_evidence_and_correction(&loops,&path,&scale,&twist,Some((&axes,&center)),None,None,
            Options {normal:[0.,0.,1.],orientation:Orientation::CorrectedFrenet,
                spacing,initial_sections:3,max_sections:65,max_deviation:0.1},
            Some(crate::analytic::EndpointCapCorrection {quantum:2_f64.powi(-40),tolerance:1e-9,max_work:1000000})).unwrap();
        assert_eq!(body.boundary_error_within_budget,Some(true));
        assert!(body.boundary_error_upper.unwrap()<=0.1);
        let face_count=body.model.faces.len();
        let before=body.model.clone();
        let report=inspect_sweep(&body.model,1e-8,limits(),20000,&[face_count-2,face_count-1],crate::sweep_cap_contacts::Budgets {
            max_walls:1024,max_exact_work:1000000,max_chart_cells:1000,max_trim_pairs:100000,
            max_trim_cells:100000,max_trim_domain_cells:1000000}).unwrap();
        let pairs=&report.boundary.intersections.pairs;
        assert!(report.proven,"boundary={} next={:?} individual={} grouped={} cells={} unresolved={:?}",
            report.boundary.proven,pairs.next_pair,pairs.pairs.len(),pairs.grouped_pairs,pairs.cells,
            pairs.pairs.iter().filter(|p|p.reason=="pair-unresolved").map(|p|p.faces).take(10).collect::<Vec<_>>());
        assert_eq!(pairs.total_pairs,face_count*(face_count-1)/2);
        assert_eq!(pairs.pairs.len()+pairs.grouped_pairs,pairs.total_pairs);
        assert!(pairs.pairs.len()<=10000 && pairs.cells<=100000 && pairs.domain_cells<=1000000);
        assert!(pairs.grouped_pairs>0 && pairs.next_pair.is_none());
        assert_eq!(body.model,before);
        }
    }
    #[test]
    fn affine_closed_hollow_reuses_fresh_disjoint_face_proofs_for_exact_witnesses(){
        let profiles=[nurbs_core::primitives::circle([0.;3],[1.,0.,0.],0.5).unwrap(),
            nurbs_core::primitives::circle([0.;3],[1.,0.,0.],0.2).unwrap().reverse().unwrap()];
        let points=[[0.,0.,0.],[10.,0.,0.],[10.,10.,0.],[0.,10.,0.]];
        let scalar=|x:f64|nurbs_core::progressive_sweep::constant_vector_law([x,0.,0.]).unwrap();
        let scale=scalar(1.);let twist=scalar(0.);
        let axes=nurbs_core::progressive_sweep::constant_vector_law([2.,1.,1.]).unwrap();
        let center=nurbs_core::progressive_sweep::constant_vector_law([0.125,0.,0.]).unwrap();
        let sweep=nurbs_core::progressive_miter::Sweep::new(&profiles,&points,&scale,&twist,
            nurbs_core::progressive_miter::Options{normal:[0.,0.,1.],closed:true,miter_limit:2.,initial_steps:1,max_steps:1,max_deviation:0.001}).unwrap().with_affine_laws(&axes,&center).unwrap();
        let rows=sweep.sections_at(1).unwrap();
        let sections=rows.into_iter().map(|row|vec![vec![row[0].clone()],vec![row[1].clone()]]).collect::<Vec<_>>();
        let model=crate::periodic_section_loft(&sections).unwrap();
        let proof=inspect_with_full_test_limits(&model);
        assert!(proof.proven);
        let nesting=proof.nesting.unwrap();
        assert_eq!(nesting.parents,Some(vec![None,Some(0)]));
        assert!(nesting.pairs[0].result.boundary_separation_certified);
        assert_eq!(nesting.pairs[0].result.reason,"certified-boundaries-exact-witness-parities");
        assert_eq!(nesting.pairs[0].result.boundary.lower_bound_mm,0.);
        assert!(nesting.cells<=limits().nesting_cells && nesting.domain_cells<=limits().nesting_domain_cells);
        assert_eq!(proof.orientations.iter().map(|s|s.outward).collect::<Vec<_>>(),vec![Some(true),Some(false)]);
        assert!(!proof.boundary.intersections.pairs.disjoint_groups.is_empty());
        // An affine hollow miter keeps its embedded boundary and cavity
        // ownership when every inner face is flipped, but material orientation
        // must independently refuse the volume certificate.
        let mut inverted_cavity = model.clone();
        let inner = inverted_cavity.bodies[0].inner_shells[0];
        for face in &mut inverted_cavity.shells[inner].faces {
            face.reversed = !face.reversed;
        }
        inverted_cavity.rebuild_topology_ids();
        let before = inverted_cavity.clone();
        let refused_orientation = inspect_with_full_test_limits(&inverted_cavity);
        assert!(refused_orientation.boundary.proven);
        assert_eq!(refused_orientation.nesting.as_ref().unwrap().roles_consistent,Some(true));
        assert!(!refused_orientation.proven);
        assert_eq!(refused_orientation.orientations[inner].outward,Some(true));
        assert_eq!(inverted_cavity,before);
        let mut missing_group=inspect_with_full_test_limits(&model).boundary;
        let cross_group=missing_group.intersections.pairs.disjoint_groups.iter().position(|group|
            model.shells[0].faces.iter().any(|fa|fa.face==group.face)
                && model.shells[1].faces.iter().any(|fb|group.range[0]<=fb.face && fb.face<group.range[1])
            || model.shells[1].faces.iter().any(|fa|fa.face==group.face)
                && model.shells[0].faces.iter().any(|fb|group.range[0]<=fb.face && fb.face<group.range[1])).unwrap();
        missing_group.intersections.pairs.disjoint_groups.remove(cross_group);
        let refused_group=crate::shell_nesting::inspect_with_boundary(&model,model.tolerance_mm,1e-8,
            100,2,2,Some(&missing_group)).unwrap();
        assert!(refused_group.parents.is_none() && refused_group.roles_consistent.is_none());
        assert!(!refused_group.pairs[0].result.boundary_separation_certified);
        let mut missing=proof.boundary;
        let cross_pair=missing.intersections.pairs.pairs.iter().position(|p|
            model.shells[0].faces.iter().any(|fa|p.faces.contains(&fa.face))
                && model.shells[1].faces.iter().any(|fb|p.faces.contains(&fb.face))).unwrap();
        missing.intersections.pairs.pairs.remove(cross_pair);
        // A positive aggregate flag without every actual cross-shell pair is
        // insufficient; the exhausted distance fallback stays unresolved.
        assert!(missing.proven);
        let refused=crate::shell_nesting::inspect_with_boundary(&model,model.tolerance_mm,1e-8,
            100,100000,1000000,Some(&missing)).unwrap();
        assert!(refused.parents.is_none() && refused.roles_consistent.is_none());
        assert!(!refused.pairs[0].result.boundary_separation_certified);
        fn inspect_with_full_test_limits(model:&Model)->Report {
            inspect_sweep(model,1e-8,limits(),20000,&[],crate::sweep_cap_contacts::Budgets{
                max_walls:1024,max_exact_work:1000000,max_chart_cells:1000,max_trim_pairs:100000,max_trim_cells:100000,max_trim_domain_cells:1000000}).unwrap()
        }
    }
    #[test]
    fn periodic_hollow_miter_keeps_unproved_boundary_and_shell_roles_distinct() {
        let profiles=[
            nurbs_core::primitives::circle([0.;3],[1.,0.,0.],0.5).unwrap(),
            nurbs_core::primitives::circle([0.;3],[1.,0.,0.],0.2).unwrap().reverse().unwrap(),
        ];
        let rows=nurbs_core::paths::closed_miter_sections(&profiles,
            &[[0.,0.,0.],[10.,0.,0.],[10.,10.,0.],[0.,10.,0.]], [0.,0.,1.],4.).unwrap();
        let sections=rows.into_iter().map(|row|vec![vec![row[0].clone()],vec![row[1].clone()]]).collect::<Vec<_>>();
        let model=crate::periodic_section_loft(&sections).unwrap();
        let before=model.clone();
        assert_eq!(model.shells.len(),2);
        assert_eq!(model.bodies[0].inner_shells,vec![1]);
        let cap=crate::sweep_cap_contacts::Budgets {
            max_walls:1024,max_exact_work:1000000,max_chart_cells:1000,
            max_trim_pairs:100000,max_trim_cells:100000,max_trim_domain_cells:1000000,
        };
        let mut budget=limits();budget.boundary.contacts.pairs=1000;
        let r=inspect_sweep(&model,1e-8,budget,10000,&[],cap).unwrap();
        println!("periodic exact={} trim={} charts={} pairs={} boundary={} volume={}",
            r.boundary.agreement.all_equal && r.boundary.agreement.all_joins_exact,
            r.boundary.trim.all_valid,r.boundary.intersections.faces.all_faces_injective,
            r.boundary.intersections.pairs.all_pairs_classified,r.boundary.proven,r.proven);
        if let Some(n)=&r.nesting {
            println!("periodic nesting roles={:?} parents={:?} cells={} domains={}",n.roles_consistent,n.parents,n.cells,n.domain_cells);
            for pair in &n.pairs { println!("periodic shell pair {:?} reason={} lower={}",
                pair.shells,pair.result.reason,pair.result.boundary.lower_bound_mm); }
        }
        for s in &r.orientations {
            println!("periodic orientation shell={} expected={} outward={:?} attempts={}",s.shell,s.expected_outward,s.outward,s.attempts.len());
            for a in &s.attempts {println!("orientation ray parity={:?} crossings={} normal={:?} cells={} domains={} unresolved={:?}",
                a.ray.parity,a.ray.crossings.len(),a.normal_dot_direction,a.ray.cells,a.ray.domain_cells,
                a.ray.unresolved.iter().take(8).map(|r|(r.face,r.reason)).collect::<Vec<_>>());}
        }
        for face in &r.boundary.intersections.faces.faces {
            if !face.result.as_ref().is_some_and(|r|r.proven) && !face.linear.as_ref().is_some_and(|r|r.certified) {
                println!("unproved periodic face {}: {:?} {:?}",face.face,face.result,face.linear);
            }
        }
        assert!(r.boundary.cap_contacts.is_empty());
        assert!(r.boundary.intersections.faces.all_faces_injective);
        assert_eq!(r.boundary.intersections.faces.faces.len(),32);
        assert!(r.proven && r.boundary.proven);
        assert_eq!(r.nesting.as_ref().unwrap().parents,Some(vec![None,Some(0)]));
        assert_eq!(r.orientations.iter().map(|s|s.outward).collect::<Vec<_>>(),vec![Some(true),Some(false)]);
        assert!(r.orientation_cells<=budget.orientation_cells);
        assert!(r.orientation_domain_cells<=budget.orientation_domain_cells);
        if !r.boundary.proven {
            assert!(!r.proven && r.nesting.is_none());
            assert!(r.orientations.iter().all(|s|s.outward.is_none()));
        }
        assert_eq!(model,before);
    }
    #[test]
    fn sweep_boundary_proof_precedes_nesting_and_material_orientation() {
        let section=|z|vec![[
            [[0.5,0.],[0.5,0.5],[0.,0.5]],[[0.,0.5],[-0.5,0.5],[-0.5,0.]],
            [[-0.5,0.],[-0.5,-0.5],[0.,-0.5]],[[0.,-0.5],[0.5,-0.5],[0.5,0.]],
        ].into_iter().map(|p|nurbs_core::curve::Curve {
            degree:2,knots:vec![0.,0.,0.,1.,1.,1.],
            control_points:p.into_iter().map(|xy|vec![xy[0],xy[1],z]).collect(),
            weights:vec![1.,std::f64::consts::FRAC_1_SQRT_2,1.],periodic:false,
        }).collect()];
        let model=crate::rational_section_loft(&[section(0.),section(5.)]).unwrap();
        let before=model.clone();
        let cap=crate::sweep_cap_contacts::Budgets {
            max_walls:1024,max_exact_work:1000000,max_chart_cells:1000,
            max_trim_pairs:100000,max_trim_cells:100000,max_trim_domain_cells:1000000,
        };
        let r=inspect_sweep(&model,1e-8,limits(),10000,&[4,5],cap).unwrap();
        assert!(r.boundary.proven);
        assert_eq!(r.nesting.as_ref().unwrap().roles_consistent,Some(true));
        assert!(r.proven,"regular exact sweep requires material orientation evidence");
        let small=inspect_sweep(&model,1e-8,Limits{orientation_cells:1,orientation_domain_cells:1,..limits()},10000,&[4,5],cap).unwrap();
        assert!(small.boundary.proven && !small.proven);
        assert!(small.orientation_cells<=1 && small.orientation_domain_cells<=1);
        let mut inward=model.clone();
        for face in &mut inward.shells[0].faces {face.reversed=!face.reversed;}
        let wrong=inspect_sweep(&inward,1e-8,limits(),10000,&[4,5],cap).unwrap();
        assert!(wrong.boundary.proven && !wrong.proven);
        assert_eq!(wrong.orientations[0].outward,Some(false));
        let no_boundary=inspect_sweep(&model,1e-8,limits(),10000,&[4,5],
            crate::sweep_cap_contacts::Budgets{max_exact_work:0,..cap}).unwrap();
        assert!(!no_boundary.proven && !no_boundary.boundary.proven);
        assert!(no_boundary.nesting.is_none());
        assert!(no_boundary.orientations.iter().all(|s|s.attempts.is_empty() && s.outward.is_none()));
        // No-cap input uses the same prerequisites without inventing endpoints.
        let closed=crate::cuboid([0.;3],[1.;3]).unwrap();
        let no_caps=inspect_sweep(&closed,1e-8,limits(),10000,&[],cap).unwrap();
        assert!(no_caps.proven && no_caps.boundary.cap_contacts.is_empty());
        assert_eq!(model,before);
    }
    #[test]
    fn cube_orientation_and_exhaustion_are_independent_of_embedding() {
        let mut m = crate::cuboid([0.;3],[1.;3]).unwrap();
        assert!(inspect(&m,1e-8,limits()).unwrap().proven);
        let r = inspect(&m,1e-8,Limits { orientation_cells: 1, orientation_domain_cells: 1, ..limits() }).unwrap();
        assert!(r.boundary.proven); assert!(!r.proven);
        assert!(r.orientation_cells<=1 && r.orientation_domain_cells<=1);
        for u in &mut m.shells[0].faces { u.reversed = !u.reversed; }
        let r = inspect(&m,1e-8,limits()).unwrap();
        assert!(r.boundary.proven); assert!(!r.proven);
        assert_eq!(r.orientations[0].outward,Some(false));
    }
    #[test]
    fn cavity_requires_inward_inner_shell_and_preserves_document() {
        let mut m = crate::cuboid([0.;3],[10.;3]).unwrap();
        let mut inner_model = crate::cuboid([2.;3],[8.;3]).unwrap();
        let (v,e,l,f)=(m.vertices.len(),m.edges.len(),m.loops.len(),m.faces.len());
        for edge in &mut inner_model.edges { edge.vertices = edge.vertices.map(|x| x+v); }
        for wire in &mut inner_model.loops { for c in &mut wire.coedges { c.edge+=e; } }
        for face in &mut inner_model.faces { face.outer+=l; for h in &mut face.holes { *h+=l; } }
        for u in &mut inner_model.shells[0].faces { u.face+=f; u.reversed=!u.reversed; }
        m.vertices.extend(inner_model.vertices.clone()); m.edges.extend(inner_model.edges.clone());
        m.loops.extend(inner_model.loops.clone()); m.faces.extend(inner_model.faces.clone());
        m.shells.extend(inner_model.shells.clone()); m.bodies[0].inner_shells.push(1);
        m.rebuild_topology_ids();
        let before = format!("{m:?}");
        let r = inspect(&m,1e-8,limits()).unwrap();
        assert!(r.proven, "boundary={} exact={} trim={} intersections={} nesting={:?} orientations={:?}",r.boundary.proven,r.boundary.agreement.all_equal,r.boundary.trim.all_valid,r.boundary.intersections.absence_proven,r.nesting.as_ref().map(|n|n.roles_consistent),r.orientations.iter().map(|o|o.outward).collect::<Vec<_>>());
        assert_eq!(format!("{m:?}"),before);
        let inner = m.bodies[0].inner_shells[0];
        assert_eq!(r.orientations[inner].outward,Some(false));
        for u in &mut m.shells[inner].faces { u.reversed = !u.reversed; }
        let r = inspect(&m,1e-8,limits()).unwrap();
        assert!(r.boundary.proven); assert!(!r.proven);
        assert_eq!(r.orientations[inner].outward,Some(true));
    }
    #[test]
    fn partitioned_boolean_cavity_uses_exact_hull_boundary_contacts() {
        let m = crate::operations::boolean(&crate::cuboid([0.;3],[10.;3]).unwrap(),
            &crate::cuboid([2.;3],[8.;3]).unwrap(),"difference").unwrap();
        let r = inspect(&m,1e-8,limits()).unwrap();
        assert!(r.boundary.agreement.all_equal && r.boundary.trim.all_valid);
        assert!(r.boundary.intersections.absence_proven,"next={:?} unresolved={:?}",r.boundary.intersections.pairs.next_pair,r.boundary.intersections.pairs.pairs.iter().filter(|p|p.reason=="pair-unresolved").map(|p|p.faces).collect::<Vec<_>>());
        assert!(!r.boundary.hull_contacts.is_empty());
        assert!(r.proven, "embedding={} nesting={:?}",r.boundary.proven,r.nesting.as_ref().map(|n|n.roles_consistent));
    }

}
