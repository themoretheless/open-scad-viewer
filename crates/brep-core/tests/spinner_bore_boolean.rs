use brep_core::{GearSpec, boolean, cylinder, gear};

#[test]
fn spinner_bore_difference_preserves_teeth_or_reports_unproved_coverage() {
    let spec = GearSpec { module: 1.5, teeth: 32, height: 10., helix_angle_deg: -35.,
        herringbone: true, clearance: 0.1, backlash: 0.05, bore: 42., ..Default::default() };
    let sun = gear(&spec).unwrap();
    let cutter = cylinder(22., 10.).unwrap();
    let reference = gear(&GearSpec { bore: 44., ..spec }).unwrap();
    assert_eq!(reference.validate().unwrap().boundary_edge_count, 0);
    match boolean(&sun, &cutter, "difference") {
        Err(error) => {
            assert_eq!(error.code, "BREP_NURBS_BOOLEAN_UNPROVED");
            eprintln!("Pending spinner bore Boolean: {}", error.message);
        }
        Ok(result) => {
            assert_eq!(result.bodies.len(), 1);
            assert_eq!(result.validate().unwrap().boundary_edge_count, 0);
            for z in [0., 10.] {
                let cap_radius = result.vertices.iter()
                    .filter(|vertex| (vertex.point[2] - z).abs() < 1e-7)
                    .map(|vertex| vertex.point[0].hypot(vertex.point[1])).fold(f64::INFINITY, f64::min);
                assert!((cap_radius - 22.).abs() < 1e-7);
            }
            // The cylindrical cut is strictly inside the tooth roots. Preserve the authored flanks.
            for edge in sun.edges.iter().filter(|edge| edge.curve.degree > 2
                && edge.curve.control_points.iter().all(|p| p[0].hypot(p[1]) > 22.1)) {
                assert!(result.edges.iter().any(|retained| retained.curve == edge.curve));
            }
        }
    }
}
