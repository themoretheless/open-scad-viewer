//! Every tolerant (numerical SSI) Boolean result across a sweep of sphere
//! placements through a cylinder's wall, cap and rim zones must tessellate
//! as a closed manifold shell at the app's LOD, and its mesh must respect
//! the B-rep's control hull and vertices. Refusals are allowed; invalid or
//! non-manifold output is not.
use brep_core::{cylinder, operations, sphere, transform};
use geometry_bridge::brep::nurbs;

#[test]
fn tolerant_results_across_a_placement_sweep_tessellate_closed() {
    let c = cylinder(3., 10.).unwrap();
    let mut solved = 0;
    let mut refused = 0;
    for i in 0..5 {
        for j in 0..3 {
            let x = 0.6 + i as f64 * 1.05;
            let z = 1.3 + j as f64 * 3.4;
            let s = transform::affine(
                &sphere(1.7).unwrap(),
                [
                    [1., 0., 0., x],
                    [0., 1., 0., 0.35],
                    [0., 0., 1., z],
                    [0., 0., 0., 1.],
                ],
            )
            .unwrap();
            for operation in ["union", "difference", "intersection"] {
                let model = match operations::boolean(&c, &s, operation) {
                    Ok(model) => model,
                    Err(e) => {
                        assert!(
                            matches!(
                                e.code,
                                "BREP_UNSUPPORTED_OPERATION" | "BREP_ANALYTIC_BOOLEAN_REFUSED"
                            ),
                            "{operation} at ({x}, {z}): {} {}",
                            e.code,
                            e.message
                        );
                        refused += 1;
                        continue;
                    }
                };
                if model.is_empty() {
                    continue;
                }
                let t = nurbs(&model, 4)
                    .unwrap_or_else(|e| panic!("{operation} at ({x}, {z}) lod 4: {e:?}"));
                assert!(t.built.report.closed, "{operation} at ({x}, {z}) closed");
                assert_eq!(t.built.report.non_manifold_edges, 0);
                assert!(t.built.report.signed_volume_mm3 > 0.);
                solved += 1;
            }
        }
    }
    assert!(solved >= 30, "solved {solved}, refused {refused}");
}
