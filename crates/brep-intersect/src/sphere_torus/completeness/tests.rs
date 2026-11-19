use super::super::super::test_utils::translated;
use super::*;

#[test]
fn a_missing_component_breaks_the_certificate() {
    let torus_model = brep_core::analytic::torus(3., 1.).unwrap();
    let torus = recognize_torus(&torus_model).unwrap().unwrap();
    let mut seen_multi = false;
    for (radius, at) in [
        (5.2_f64.sqrt(), [0.5, 0., 0.]),
        (1., [4.5, 0., 0.]),
        (1., [3.5, 0.4, 0.3]),
        (2.5, [2., 1., 0.]),
    ] {
        let sphere_model = translated(&brep_core::analytic::sphere(radius).unwrap(), at);
        let sphere = sphere_sphere::recognize_any(&sphere_model).unwrap().unwrap();
        let report = intersect_sphere_torus(&sphere_model, &torus_model, Options::default()).unwrap();
        let loops: Vec<Vec<Curve>> = report
            .components
            .iter()
            .filter_map(|c| match c {
                SphereTorusComponent::Traced { pieces, .. } => Some(pieces.clone()),
                _ => None,
            })
            .collect();
        assert!(!loops.is_empty(), "{radius} at {at:?}");
        assert!(certify_complete(&sphere, &torus, &sphere_model, &loops));
        // No traced loops: every root on the boxes' boundaries is unmatched.
        assert!(!certify_complete(&sphere, &torus, &sphere_model, &[]));
        // Dropping one of several loops is detected too.
        if loops.len() > 1 {
            seen_multi = true;
            assert!(!certify_complete(&sphere, &torus, &sphere_model, &loops[1..]));
        }
    }
    assert!(seen_multi, "no configuration with several loops was exercised");
}
