use super::*;
use crate::cylinder;

#[test]
#[cfg(feature = "codec")]
fn annular_outer_round_revolves_to_exact_toroidal_faces() {
    let source = crate::tube(20., 5., 6.).unwrap();
    let before = value_codec::to_string(&source).unwrap();
    let profile = [[5., 0.], [20., 0.], [20., 6.], [5., 6.]];
    for radius in [0.25_f64, 1., 2.5] {
        let mut curves = rounded_profile_curves(&source, &profile, &[false, false, true, false], radius, 0.).unwrap();
        for curve in &mut curves {
            for point in &mut curve.control_points { point.truncate(2); }
        }
        let result = crate::revolve_wire(&curves, source.tolerance_mm).unwrap();
        crate::solid_audit::audit_solid(&result).unwrap();
        let volume = crate::analysis::mass_properties(&result, 1e-9, 300_000).unwrap().signed_volume_mm3;
        // Pappus: rotate the removed square-minus-quarter-disc area.
        // Its first moment relative to the arc center is r^3 / 6.
        let removed_moment = (20.-radius)*(1.-std::f64::consts::PI/4.)*radius*radius + radius.powi(3)/6.;
        let expected = 2250.*std::f64::consts::PI - 2.*std::f64::consts::PI*removed_moment;
        assert!((volume-expected).abs()<2e-5, "{volume} != {expected}");
        assert_eq!(result.faces.iter().filter(|f|f.surface.degree_u==2 && f.surface.degree_v==2).count(),4);
        for face in result.faces.iter().filter(|f|f.surface.degree_u==2 && f.surface.degree_v==2) {
            let s=&face.surface;
            let domain_u=[s.knots_u[s.degree_u],s.knots_u[s.knots_u.len()-s.degree_u-1]];
            let domain_v=[s.knots_v[s.degree_v],s.knots_v[s.knots_v.len()-s.degree_v-1]];
            for a in [0.,0.25,0.5,0.75,1.] {
                for b in [0.,0.25,0.5,0.75,1.] {
                    let p=s.evaluate(domain_u[0]+a*(domain_u[1]-domain_u[0]),domain_v[0]+b*(domain_v[1]-domain_v[0])).unwrap().point;
                    let residual=(p[0].hypot(p[1])-(20.-radius)).hypot(p[2]-(6.-radius))-radius;
                    assert!(residual.abs()<1e-9,"torus radius residual {residual}");
                }
            }
        }
    }
    assert!(rounded_profile_curves(&source,&profile,&[false,false,true,false],6.,0.).is_err());
    assert_eq!(value_codec::to_string(&source).unwrap(),before);
}

#[test]
#[cfg(feature = "codec")]
fn concave_prism_selected_outer_round_has_analytic_volume() {
    let profile = [[0., 0.], [40., 0.], [40., 5.], [5., 5.], [5., 30.], [0., 30.]];
    let source = crate::extrude_polygon(&profile, 0., 20.).unwrap();
    let before = value_codec::to_string(&source).unwrap();
    for rounded in [
        [true, false, false, false, false, false],
        [true, true, true, false, true, true],
    ] {
        let result = rounded_convex_prism_edges(&source, &profile, &rounded, 1., 0., 20.).unwrap();
        crate::solid_audit::audit_solid(&result).unwrap();
        let volume = crate::analysis::mass_properties(&result, 1e-9, 300_000).unwrap().signed_volume_mm3;
        let count = rounded.iter().filter(|v| **v).count();
        let expected = 6500. - count as f64 * (1. - std::f64::consts::PI / 4.) * 20.;
        assert!((volume - expected).abs() < 2e-5, "{volume} != {expected}");
        assert_eq!(result.faces.iter().filter(|f| f.surface.degree_u == 2 || f.surface.degree_v == 2).count(), count);
    }
    let error = rounded_convex_prism_edges(&source, &profile, &[false, false, false, true, false, false], 1., 0., 20.).unwrap_err();
    assert!(error.message.contains("must be convex"));
    let selected = [true, false, false, false, false, false];
    let near = rounded_convex_prism_edges(&source, &profile, &selected, 16., 0., 20.).unwrap();
    crate::solid_audit::audit_solid(&near).unwrap();
    let expected = 6500. - 16_f64.powi(2) * (1. - std::f64::consts::PI / 4.) * 20.;
    let volume = crate::analysis::mass_properties(&near, 1e-9, 300_000).unwrap().signed_volume_mm3;
    assert!((volume - expected).abs() < 2e-5, "{volume} != {expected}");
    let collision = rounded_convex_prism_edges(&source, &profile, &selected, 20., 0., 20.).unwrap_err();
    assert!(collision.message.contains("nonadjacent span"));
    assert_eq!(value_codec::to_string(&source).unwrap(), before);
}

#[test]
fn disjoint_union_is_separated_without_prism() {
    let a = cylinder(2., 4.).unwrap();
    let b0 = cylinder(2., 4.).unwrap();
    let b = crate::transform::affine(
        &b0,
        [
            [1., 0., 0., 12.],
            [0., 1., 0., 0.],
            [0., 0., 1., 0.],
            [0., 0., 0., 1.],
        ],
    )
    .unwrap();
    let out = regularized_empty_algebra(&a, &b, "union", SpatialRelation::Disjoint).unwrap();
    assert!(out.bodies.len() >= 2 || out.shells.len() >= 2);
    out.validate().unwrap();
}

#[test]
fn imprint_plan_refuses_duplicate_face_parameter() {
    let events = [
        ImprintEvent {
            face: 0,
            edge: None,
            uv: [0.5, 0.5],
            point: [0., 0., 0.],
            parameter: 0.1,
        },
        ImprintEvent {
            face: 0,
            edge: None,
            uv: [0.6, 0.5],
            point: [0., 0., 1.],
            parameter: 0.1,
        },
    ];
    assert_eq!(
        build_imprint_plan(&events, 1.).unwrap_err().code,
        "BREP_IMPRINT_PIPELINE_REFUSED"
    );
}

#[test]
fn contained_difference_builds_cavity() {
    let outer = cylinder(4., 6.).unwrap();
    let inner = cylinder(1.5, 6.).unwrap();
    let out =
        regularized_empty_algebra(&outer, &inner, "difference", SpatialRelation::AContainsB)
            .unwrap();
    assert!(!out.bodies[0].inner_shells.is_empty());
    out.validate().unwrap();
}
