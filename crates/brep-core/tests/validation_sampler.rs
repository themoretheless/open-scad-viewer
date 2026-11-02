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

#[test]
fn independent_uv_trim_clocks_preserve_loop_and_keep_unproved_continuous_checks_closed() {
    let mut model = brep_core::cuboid([0.;3],[1.;3]).unwrap();
    let domains = model.edges.iter().map(|e|e.curve.domain()).collect::<Vec<_>>();
    for wire in &mut model.loops {
        for (i,c) in wire.coedges.iter_mut().enumerate() {
            let a = c.pcurve.control_points[0].clone();
            let b = c.pcurve.control_points[1].clone();
            let mut p = c.pcurve.clone();
            p.control_points = vec![
                a.iter().zip(&b).map(|(a,b)|2.*a-b).collect(),
                a.iter().zip(&b).map(|(a,b)|3.*b-2.*a).collect(),
            ];
            p.knots = vec![2.,2.,6.,6.];
            let uv = if i%2==0 { [3.,4.] } else {
                p.control_points.reverse(); [5.,4.]
            };
            *c = brep_core::Coedge::new(c.edge,c.reversed,p).with_trim(brep_topology::CoedgeTrim {
                curve_parameter:domains[c.edge],pcurve_parameter:uv,periodic_lift:[[0;2];2],
            }).unwrap();
        }
    }
    let before = model.clone();
    assert_eq!(model.validate().unwrap().boundary_edge_count,0);
    // The generic interval verifier cannot certify this independently clocked
    // representation yet. Sampled success must not turn that into admission.
    assert_eq!(model.validate_continuous(10000).unwrap_err().code,"BREP_GEOMETRY_UNRESOLVED");
    assert_eq!(model,before);
    let c = &mut model.loops[0].coedges[0];
    let mut trim = c.trim().unwrap(); trim.pcurve_parameter[0] = 1.;
    *c = c.clone().with_trim(trim).unwrap();
    assert!(model.validate().is_err());
}
