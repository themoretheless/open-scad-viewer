//! A scaled, retained loft with holes and rational sections survives STEP v9.
use nurbs_core::curve::Curve;

#[test]
fn scaled_retained_loft_round_trips_through_step_v9() {
    let mut hole = brep_core::sketch::circle_wire(1.).unwrap();
    hole = hole
        .iter()
        .rev()
        .map(Curve::reverse)
        .collect::<nurbs_core::Result<Vec<_>>>()
        .unwrap();
    let loops = vec![brep_core::sketch::circle_wire(3.).unwrap(), hole];
    let loft = brep_core::prism::loft_scaled(&loops, 0., 10., 2., [5., 7.]).unwrap();
    let (step, _, _) = cad_step::export_step_v9(&loft).unwrap();
    let (restored, _, _) = cad_step::import_step_v9(&step).unwrap();
    restored.validate().unwrap();
    assert_eq!(restored.bodies.len(), 1);
    assert_eq!(
        restored
            .faces
            .iter()
            .filter(|face| face.holes.len() == 1)
            .count(),
        2
    );
}
