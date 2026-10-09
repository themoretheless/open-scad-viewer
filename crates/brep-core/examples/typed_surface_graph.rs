//! Build and inspect a retained intersection graph without a JSON round trip.
use brep_core::{
    branch_graph_from_surface_intersection, rational_traces_from_branch_graph,
    verify_general_ss_branch_graph,
};
use cad_predicates::ToleranceContext;
use nurbs_core::{Result, ss_intersection::intersect_surface_surface_report, surface::Surface};
fn plane(vertical: bool) -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: (0..2)
            .map(|u| {
                (0..2)
                    .map(|v| {
                        if vertical {
                            vec![u as f64, 0., v as f64]
                        } else {
                            vec![u as f64, v as f64, 0.]
                        }
                    })
                    .collect()
            })
            .collect(),
        weights: vec![vec![1., 1.], vec![1., 1.]],
        periodic_u: false,
        periodic_v: false,
    }
}
fn main() -> Result<()> {
    let report = intersect_surface_surface_report(&plane(false), &plane(true), None)?;
    let context = ToleranceContext::default_valid();
    let graph = branch_graph_from_surface_intersection(&report, &context)?;
    let audit = verify_general_ss_branch_graph(&graph, &context)?;
    let traces = rational_traces_from_branch_graph(&graph);
    let coverage =
        brep_core::coverage_verifier::verify_general_ss_native_coverage(&report, &context)?;
    assert_eq!(coverage.component_count, audit.component_count);
    assert!(coverage.complete);
    assert!(audit.complete);
    assert_eq!(audit.component_count, 1);
    assert_eq!(traces.len(), 2);
    assert!(!graph.boolean_mutation_authority);
    println!(
        "{} intersection branch; {} retained UV traces",
        audit.component_count,
        traces.len()
    );
    Ok(())
}
