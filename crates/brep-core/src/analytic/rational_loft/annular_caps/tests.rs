use super::*;
use nurbs_core::{
    primitives::{circle, line},
    progressive_sweep::{Options, Orientation, Spacing, constant_vector_law},
};

fn limits() -> Limits {
    Limits {
        exact_work: 1_000_000,
        products: 100_000,
        trim_pairs: 10_000,
        trim_cells: 100_000,
        trim_domain_cells: 1_000_000,
        spans: 1024,
        linear_cells: 100_000,
        contacts: crate::face_contacts::Limits {
            pairs: 10_000,
            cells: 100_000,
            domain_cells: 1_000_000,
            cells_per_pair: 1000,
            domain_cells_per_pair: 10_000,
        },
    }
}
fn options(orientation: Orientation, initial: usize, max: usize) -> Options {
    Options {
        normal: [0., 0., 1.],
        orientation,
        spacing: Spacing::Parameter,
        initial_sections: initial,
        max_sections: max,
        max_deviation: 0.01,
    }
}
fn make(kind: usize) -> Boundary {
    let axis = if kind == 0 {
        [3., 4., 0.]
    } else {
        [1., 0., 0.]
    };
    let loops = vec![
        vec![circle([0.; 3], axis, 0.1).unwrap()],
        vec![circle([0.; 3], axis, 0.05).unwrap().reverse().unwrap()],
    ];
    let path = if kind == 0 {
        line([0.; 3], [3., 4., 0.]).unwrap()
    } else {
        Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![0.; 3], vec![0.5, 0., 0.], vec![1., 1., 0.]],
            weights: vec![1.; 3],
            periodic: false,
        }
    };
    let scale = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![1., 0., 0.],
            vec![if kind == 0 { 2. } else { 1. }, 0., 0.],
        ],
        weights: vec![1.; 2],
        periodic: false,
    };
    let twist = constant_vector_law([0.; 3]).unwrap();
    let orientation = if kind == 1 {
        Orientation::Frenet
    } else {
        Orientation::RotationMinimizing
    };
    let body = super::super::progressive_profile_body_with_annular_caps(
        &loops,
        &path,
        &scale,
        &twist,
        None,
        None,
        None,
        options(orientation, if kind == 0 { 2 } else { 3 }, 129),
        None,
        None,
        None,
        limits(),
    )
    .unwrap();
    let boundary = body
        .complete_boundary()
        .expect("Source-owned complete boundary must be proved");
    assert!(boundary.error_upper.unwrap() <= 0.01 && boundary.wall_error_upper.is_some());
    assert!(
        boundary
            .filled_cap_error_upper
            .unwrap()
            .iter()
            .all(|&e| e <= 1e-8)
    );
    let caps = body.caps().unwrap();
    assert_eq!(caps.face_groups().each_ref().map(|g| g.len()), [4, 4]);
    assert!(caps.exact_work() <= limits().exact_work);
    assert_eq!(
        caps.exact_work(),
        caps.construction_work() + caps.filling().exact_work
    );
    assert!(body.evidence().retained_caps.is_none() && body.evidence().cap_projection.is_none());
    assert!(body.evidence().retained_walls.certified);
    assert!(body.model().faces.len() <= crate::MAX_FACES);
    body
}

#[test]
fn owned_oblique_rmf_full_boundary_and_cap_work_refusals() {
    let body = make(0);
    let axis = [3., 4., 0.];
    let profiles = [
        circle([0.; 3], axis, 0.1).unwrap(),
        circle([0.; 3], axis, 0.05).unwrap().reverse().unwrap(),
    ];
    let path = line([0.; 3], axis).unwrap();
    let scale = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![1., 0., 0.], vec![2., 0., 0.]],
        weights: vec![1.; 2],
        periodic: false,
    };
    let twist = constant_vector_law([0.; 3]).unwrap();
    let raw = super::super::rational_section_loft(&body.body.station_sections).unwrap();
    let source = MultiSweep::new(
        &profiles,
        &path,
        &scale,
        &twist,
        options(Orientation::RotationMinimizing, 2, 129),
    )
    .unwrap();
    let short = Limits {
        exact_work: body.caps().unwrap().exact_work() - 1,
        ..limits()
    };
    match assemble(&source, &raw, &[1, 1], 0.01, 1e-9, short) {
        Ok(denied) => assert!(denied.evidence.filling().filled_cap_error_upper.is_none()),
        Err(_) => {}
    }
    let denied = assemble(
        &source,
        &raw,
        &[1, 1],
        0.01,
        1e-9,
        Limits {
            products: 0,
            ..limits()
        },
    )
    .unwrap();
    assert!(!denied.evidence.filling().within_budget);
    assert!(denied.evidence.filling().filled_cap_error_upper.is_none());
}

#[test]
fn owned_quadratic_frenet_full_boundary() {
    make(1);
}
#[test]
fn owned_planar_quadratic_rmf_full_boundary() {
    make(2);
}

#[test]
fn annular_station_reconstruction_preserves_all_caps_and_actual_g2() {
    let loops = vec![
        vec![circle([0.; 3], [0., 0., 1.], 0.5).unwrap()],
        vec![
            circle([0.; 3], [0., 0., 1.], 0.25)
                .unwrap()
                .reverse()
                .unwrap(),
        ],
    ];
    let path = line([0.; 3], [0., 0., 10.]).unwrap();
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let mut opt = options(Orientation::Fixed, 3, 3);
    opt.normal = [1., 0., 0.];
    let mut body = super::super::progressive_profile_body_with_annular_caps(
        &loops,
        &path,
        &scale,
        &twist,
        None,
        None,
        None,
        opt,
        None,
        None,
        None,
        limits(),
    )
    .unwrap();
    assert!(body.complete_boundary().is_some());
    let full = body.reconstruct_stations(0.125, 1e-9, 1_000_000).unwrap();
    let reconstructed = full.reconstruction.as_ref().unwrap();
    let model = reconstructed
        .model
        .as_ref()
        .expect("Complete owned annular reconstruction");
    assert!(reconstructed.candidate.work <= 1_000_000);
    assert!(
        full.boundary.as_ref().unwrap().continuous_bound
            && full.boundary.as_ref().unwrap().within_budget == Some(true)
    );
    preserve_groups(body.model(), model, body.caps().unwrap().face_groups()).unwrap();
    let jets =
        crate::sweep_smoothness::inspect_profile(model, &body.cap_faces(), 2_000_000).unwrap();
    assert!(
        jets.station.g2_certified,
        "Actual reconstructed station jets must be G2"
    );
    assert!(body.reconstruct_stations(0.125, 1e-9, 0).is_err());
    let face = body.cap_faces()[0];
    let pole = &mut body.body.model.faces[face].surface.control_points[0][0][0];
    *pole = pole.next_up();
    assert!(body.reconstruct_stations(0.125, 1e-9, 1_000_000).is_err());
}
