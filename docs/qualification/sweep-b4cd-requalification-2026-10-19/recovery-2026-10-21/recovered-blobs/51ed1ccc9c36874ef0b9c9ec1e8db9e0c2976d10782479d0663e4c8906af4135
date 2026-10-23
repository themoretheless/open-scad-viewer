use brep_core::{GearSpec, analytic, gear};
use nurbs_core::surface::SurfaceSampler;

#[test]
fn validated_face_sampling_matches_checked_evaluation_at_every_coedge_sample() {
    let model = gear(&GearSpec {
        teeth: 12,
        herringbone: true,
        helix_angle_deg: 20.,
        bore: 3.,
        ..GearSpec::default()
    })
    .unwrap();
    for face in &model.faces {
        let sampler = SurfaceSampler::new(&face.surface).unwrap();
        for &index in std::iter::once(&face.outer).chain(&face.holes) {
            for coedge in &model.loops[index].coedges {
                let curve = &coedge.pcurve;
                let domain = curve.domain();
                for step in 0..=8 {
                    let uv = curve
                        .evaluate(domain[0] + step as f64 / 8. * (domain[1] - domain[0]))
                        .unwrap()
                        .point;
                    let checked = face.surface.evaluate(uv[0], uv[1]).unwrap();
                    let sampled = sampler.evaluate(uv[0], uv[1]).unwrap();
                    assert_eq!(sampled.point, checked.point);
                }
            }
        }
    }
    assert_eq!(model.validate().unwrap().boundary_edge_count, 0);
}

#[test]
fn validation_still_rejects_bad_surface_weights_and_changed_geometry() {
    let model = analytic::cylinder(3., 4.).unwrap();
    model.validate().unwrap();
    let mut invalid = model.clone();
    invalid.faces[0].surface.weights[0][0] = 0.;
    let expected = invalid.faces[0].surface.validate().unwrap_err();
    let actual = invalid.validate().unwrap_err();
    assert_eq!(actual.code, expected.code);
    assert_eq!(actual.message, expected.message);
    let mut changed = model;
    changed.faces[0].surface.control_points[0][0][2] += 1.;
    let error = changed.validate().unwrap_err();
    assert!(
        error
            .message
            .contains("pcurve/surface and 3D edge disagree"),
        "{error:?}"
    );
}

#[test]
fn pole_and_closed_analytic_faces_remain_valid() {
    for model in [
        analytic::sphere(3.).unwrap(),
        analytic::cylinder(3., 4.).unwrap(),
    ] {
        let report = model.validate().unwrap();
        assert!(report.topology_valid);
        assert_eq!(report.boundary_edge_count, 0);
    }
}
