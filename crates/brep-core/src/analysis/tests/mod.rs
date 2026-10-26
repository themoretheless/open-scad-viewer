use super::*;
fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-7 * b.abs().max(1.), "{a} != {b}");
}
#[test]
fn unresolved_tangent_knot_crossing_cannot_report_convergence() {
    let curve = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![0., 0.], vec![0.5, 1.], vec![1., 0.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let mut surface = cuboid([0.; 3], [1.; 3]).unwrap().faces[0].surface.clone();
    surface = surface
        .edit_axis(nurbs_core::surface::Axis::V, |c| c.insert(0.5, 1))
        .unwrap();
    let mut budget = Budget {
        used: 0,
        limit: 200_000,
    };
    let grid = conditioned_surface_grid(&surface, &mut budget).unwrap();
    let error = trim_intervals(&curve, &grid, 1e-7, &mut budget).unwrap_err();
    assert_eq!(error.code, "BREP_ANALYSIS_INDETERMINATE");
    assert!(
        budget.used > curve.control_points.len(),
        "Root search work must count against the budget"
    );
}
#[test]
fn planar_and_round_solids_match_analytic_mass_properties() {
    let b =
        mass_properties(&cuboid([1., 2., 3.], [3., 6., 9.]).unwrap(), 1e-7, 200_000).unwrap();
    near(b.signed_volume_mm3, 48.);
    near(b.surface_area_mm2, 88.);
    assert_eq!(b.centroid, [2., 4., 6.]);
    near(b.inertia_mm5[0][0], 48. * (16. + 36.) / 12.);
    let c = mass_properties(&cylinder(3., 8.).unwrap(), 1e-7, 200_000).unwrap();
    near(c.signed_volume_mm3, 72. * std::f64::consts::PI);
    near(c.surface_area_mm2, 66. * std::f64::consts::PI);
    near(c.centroid[2], 4.);
    near(c.inertia_mm5[2][2], c.signed_volume_mm3 * 9. / 2.);
    let t = mass_properties(&tube(3., 2., 8.).unwrap(), 1e-7, 200_000).unwrap();
    near(t.signed_volume_mm3, 40. * std::f64::consts::PI);
    near(t.surface_area_mm2, 90. * std::f64::consts::PI);
    near(t.inertia_mm5[2][2], t.signed_volume_mm3 * 13. / 2.);
}
#[test]
fn world_coordinate_trim_endpoints_preserve_cap_mass() {
    let shell = crate::operations::boolean(
        &cuboid([0.; 3], [40., 30., 20.]).unwrap(),
        &cuboid([2., 2., 2.], [38., 28., 22.]).unwrap(),
        "difference",
    )
    .unwrap();
    let top = shell
        .faces
        .iter()
        .position(|f| {
            f.surface
                .control_points
                .iter()
                .flatten()
                .all(|p| p[2] == 20.)
        })
        .unwrap();
    let edited = crate::operations::push_planar_face(&shell, top, 0.5).unwrap();
    near(
        mass_properties(&edited, 1e-7, 200_000)
            .unwrap()
            .signed_volume_mm3,
        7152. + 264. * 0.5,
    );
}
#[test]
fn narrow_world_chart_after_repeated_cap_placement_preserves_mass() {
    let model:Model=value_codec::from_str(include_str!("../../../../../docs/qualification/cad-roadmap-2026-09-28/p1-development-2026-10-03/whole-wall/rotated-enclosure-mass.json")).unwrap();
    near(
        mass_properties(&model, 1e-7, 300_000)
            .unwrap()
            .signed_volume_mm3,
        7416.,
    );
}
#[test]
fn holes_translations_and_resource_refusals_are_explicit() {
    let outline = [[0., 0.], [4., 0.], [4., 4.], [2., 4.], [2., 2.], [0., 2.]];
    let h = vec![[2.5, 0.5], [2.5, 1.5], [3.5, 1.5], [3.5, 0.5]];
    let mut m = extrude_polygon_with_holes(&outline, &[h], 0., 3.).unwrap();
    let before = mass_properties(&m, 1e-7, 200_000).unwrap();
    near(before.signed_volume_mm3, 33.);
    let delta = [10000., -50000., 70000.];
    for v in &mut m.vertices {
        for i in 0..3 {
            v.point[i] += delta[i];
        }
    }
    for e in &mut m.edges {
        for p in &mut e.curve.control_points {
            for i in 0..3 {
                p[i] += delta[i];
            }
        }
    }
    for f in &mut m.faces {
        for p in f.surface.control_points.iter_mut().flatten() {
            for i in 0..3 {
                p[i] += delta[i];
            }
        }
    }
    let after = mass_properties(&m, 1e-7, 200_000).unwrap();
    near(after.signed_volume_mm3, before.signed_volume_mm3);
    for i in 0..3 {
        near(after.centroid[i], before.centroid[i] + delta[i]);
        near(after.inertia_mm5[i][i], before.inertia_mm5[i][i]);
    }
    assert_eq!(
        mass_properties(&m, 1e-7, 100).unwrap_err().code,
        "BREP_RESOURCE_LIMIT"
    );
}

fn contains(interval: CertifiedInterval, value: f64) {
    assert!(
        interval.lower <= value && value <= interval.upper,
        "{interval:?} excludes {value}"
    );
}

#[test]
fn certified_mass_encloses_box_cylinder_tube_and_exact_profile() {
    let box_mass =
        certified_mass_properties(&cuboid([1., 2., 3.], [3., 6., 9.]).unwrap()).unwrap();
    contains(box_mass.volume_mm3, 48.);
    contains(box_mass.surface_area_mm2, 88.);
    assert!(box_mass.audit.ok && box_mass.naming_complete);
    assert_eq!(box_mass.context, box_mass.evidence.context);

    let cylinder_mass = certified_mass_properties(&cylinder(3., 8.).unwrap()).unwrap();
    contains(cylinder_mass.volume_mm3, 72. * std::f64::consts::PI);
    contains(cylinder_mass.inertia_mm5[2][2], 324. * std::f64::consts::PI);

    let tube_mass = certified_mass_properties(&tube(3., 2., 8.).unwrap()).unwrap();
    contains(tube_mass.volume_mm3, 40. * std::f64::consts::PI);
    contains(tube_mass.surface_area_mm2, 90. * std::f64::consts::PI);

    let prism = extrude_polygon(&[[0., 0.], [4., 0.], [1., 3.]], -2., 5.).unwrap();
    let prism_mass = certified_mass_properties(&prism).unwrap();
    contains(prism_mass.volume_mm3, 42.);
}

#[test]
fn certified_mass_is_scale_and_rigid_transform_stable_and_refuses_mutation() {
    let large = certified_mass_properties(&cuboid([0.; 3], [1e4, 2e4, 3e4]).unwrap()).unwrap();
    contains(large.volume_mm3, 6e12);

    let placed = crate::transform::affine(
        &cylinder(2., 6.).unwrap(),
        [
            [0., 0., 1., 7.],
            [1., 0., 0., -3.],
            [0., 1., 0., 5.],
            [0., 0., 0., 1.],
        ],
    )
    .unwrap();
    let transformed = certified_mass_properties(&placed).unwrap();
    contains(transformed.volume_mm3, 24. * std::f64::consts::PI);
    contains(transformed.centroid[0], 10.);

    let mut mutated = cuboid([0.; 3], [1.; 3]).unwrap();
    mutated.1.change_set.changes.clear();
    assert_eq!(
        certified_mass_properties(&mutated).unwrap_err().code,
        "BREP_CERTIFIED_MASS_REFUSED"
    );
    let sphere = certified_mass_properties(&sphere(2.).unwrap()).unwrap();
    contains(sphere.volume_mm3, 32. * std::f64::consts::PI / 3.);
}

#[test]
fn certified_successor_encloses_cone_frustum_torus_and_spherical_cavity() {
    let cone = certified_mass_properties(&frustum(3., 0., 4.).unwrap()).expect("cone");
    contains(cone.volume_mm3, 12. * std::f64::consts::PI);
    contains(cone.surface_area_mm2, 24. * std::f64::consts::PI);
    contains(cone.centroid[2], 1.);
    contains(cone.inertia_mm5[2][2], 32.4 * std::f64::consts::PI);
    contains(cone.inertia_mm5[0][0], 23.4 * std::f64::consts::PI);

    let frustum = certified_mass_properties(&frustum(3., 1., 4.).unwrap()).expect("frustum");
    contains(frustum.volume_mm3, 52. * std::f64::consts::PI / 3.);

    let torus = certified_mass_properties(&torus(5., 2.).unwrap()).expect("torus");
    contains(torus.volume_mm3, 40. * std::f64::consts::PI.powi(2));
    contains(torus.surface_area_mm2, 40. * std::f64::consts::PI.powi(2));
    contains(
        torus.inertia_mm5[2][2],
        1120. * std::f64::consts::PI.powi(2),
    );
    contains(torus.inertia_mm5[0][0], 600. * std::f64::consts::PI.powi(2));

    let cavity =
        crate::imprint_pipeline::cavity(&sphere(3.).unwrap(), &sphere(1.).unwrap(), 1e-7)
            .unwrap();
    let hollow = certified_mass_properties(&cavity).expect("cavity");
    contains(hollow.volume_mm3, 104. * std::f64::consts::PI / 3.);
    assert_eq!(hollow.component_count, 2);
    assert_eq!(hollow.cavity_count, 1);
}

#[test]
fn freeform_tessellation_bounds_planar_cuboid_and_bump() {
    let planar = crate::freeform_cuboid_solid([0., 0., 0.], [2., 3., 4.]).unwrap();
    assert_eq!(
        certified_tessellation_deviation(&planar, 4)
            .unwrap_err()
            .code,
        "BREP_CERTIFIED_TESSELLATION_REFUSED"
    );
    let planar_dev = certified_freeform_tessellation_deviation(&planar, 1).unwrap();
    assert!(
        planar_dev <= 1e-12,
        "planar elevated cuboid should be zero-bound"
    );

    let bump = crate::freeform_cuboid_with_bump_face([0., 0., 0.], [2., 2., 2.]).unwrap();
    let coarse = certified_freeform_tessellation_deviation(&bump, 1).unwrap();
    let fine = certified_freeform_tessellation_deviation(&bump, 4).unwrap();
    assert!(coarse > 0., "bump needs a positive Bernstein bound");
    assert!(fine < coarse, "bound must tighten with subdivisions");
    assert!(fine <= coarse / 15., "1/n² scaling should dominate");

    let mut unequal = bump.clone();
    unequal.faces[1].surface.weights[1][1] = 2.;
    assert_eq!(
        certified_freeform_tessellation_deviation(&unequal, 4)
            .unwrap_err()
            .code,
        "BREP_CERTIFIED_TESSELLATION_REFUSED"
    );
}

#[test]
fn freeform_mass_encloses_planar_cuboid_and_refuses_bump() {
    let planar = crate::freeform_cuboid_solid([1., 2., 3.], [3., 6., 9.]).unwrap();
    assert_eq!(
        certified_mass_properties(&planar).unwrap_err().code,
        "BREP_CERTIFIED_MASS_REFUSED"
    );
    let mass = certified_freeform_mass_properties(&planar).unwrap();
    assert_eq!(mass.capability, FREEFORM_MASS_CAPABILITY);
    contains(mass.volume_mm3, 48.);
    contains(mass.surface_area_mm2, 88.);
    assert!(mass.audit.ok && mass.naming_complete);

    let bump = crate::freeform_cuboid_with_bump_face([0., 0., 0.], [2., 2., 2.]).unwrap();
    assert_eq!(
        certified_freeform_mass_properties(&bump).unwrap_err().code,
        "BREP_CERTIFIED_MASS_REFUSED"
    );
}
