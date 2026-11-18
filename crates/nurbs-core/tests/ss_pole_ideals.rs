use num_rational::BigRational as Q;
use nurbs_core::ss_intersection as ss;
#[allow(dead_code)]
#[path = "support/ss_original_ideal.rs"]
mod fixture;
use fixture::{cone, pole_plane as plane};
#[test]
fn exact_source_pole_and_general_elimination_distinguish_a_contact_from_an_off_pole_arc() {
    for vertical in [false, true] {
        let source = ss::FiberSources::new(&cone(), &plane(vertical)).unwrap();
        let ideal = ss::qualify_original_ss_ideal(&source, [1; 4], Default::default()).unwrap();
        let pole = ss::qualify_ss_pole_stratum(&ideal, 0, 1, 0, Default::default()).unwrap();
        assert_eq!(
            pole.point(),
            &[
                Q::from_integer(0.into()),
                Q::from_integer(0.into()),
                Q::from_integer(0.into())
            ]
        );
        pole.reverify().unwrap();
        assert!(ss::qualify_ss_pole_stratum(&ideal, 0, 1, 1, Default::default()).is_err());
        let basis = ss::qualify_ss_groebner_basis(&ideal, Default::default()).unwrap();
        let deflated = ss::qualify_ss_deflated_system(&basis, Default::default()).unwrap();
        let saturation =
            ss::qualify_ss_pole_saturation(&deflated, &pole, Default::default()).unwrap();
        saturation.reverify().unwrap();
        assert_eq!(saturation.proves_no_off_pole_roots(), !vertical);
        if vertical {
            assert_eq!(saturation.equations().len(), 3);
        }
    }
}

#[test]
fn original_pole_root_arc_keeps_the_collapsed_endpoint_and_the_full_radial_interval() {
    let source = ss::FiberSources::new(&cone(), &plane(true)).unwrap();
    let ideal = ss::qualify_original_ss_ideal(&source, [1; 4], Default::default()).unwrap();
    let pole = ss::qualify_ss_pole_stratum(&ideal, 0, 1, 0, Default::default()).unwrap();
    let basis = ss::qualify_ss_groebner_basis(&ideal, Default::default()).unwrap();
    let deflated = ss::qualify_ss_deflated_system(&basis, Default::default()).unwrap();
    let saturated = ss::qualify_ss_pole_saturation(&deflated, &pole, Default::default()).unwrap();
    let domain = [[0.4, 0.6], [0., 0.25], [0.4, 0.8], [0.2, 0.5]];
    let arc = ss::qualify_pole_ss_root_arc(&saturated, domain, Default::default()).unwrap();
    arc.reverify().unwrap();
    assert_eq!(arc.free_axis(), ss::FiberAxis::FirstV);
    assert!(arc.report().contraction_upper.unwrap() < 1.);
    assert_eq!(arc.pole_world_point(), pole.point());
    let mut away = domain;
    away[1][0] = 0.125;
    assert!(ss::qualify_pole_ss_root_arc(&saturated, away, Default::default()).is_err());
}
