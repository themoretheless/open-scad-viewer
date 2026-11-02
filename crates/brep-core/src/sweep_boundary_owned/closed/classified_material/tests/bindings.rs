use super::*;

#[test]
fn original_multispan_addresses_survive_material_outer_and_hole_decomposition() {
    let path = source::path();
    let (a, n, twist) = source::laws();
    let (_, cap, _) = source::material();
    let join = |curves: Vec<Curve>| {
        let mut points = vec![curves[0].control_points[0].clone()];
        points.extend(curves.iter().map(|c| c.control_points[1].clone()));
        Curve::from_polyline(points).unwrap()
    };
    let mut profiles = Vec::new();
    let mut trims = Vec::new();
    for (scale, reverse) in [(1., false), (0.25, true)] {
        let (world, uv) = polygon(scale, reverse, false);
        profiles.push(join(world));
        trims.push(vec![join(uv)]);
    }
    let material = LoftCap {
        surface: cap,
        trims,
    };
    let before = (profiles.clone(), material.clone());
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let sweep = MultiSweep::new(&profiles, &path, &scale, &twist, options())
        .unwrap()
        .with_frame_laws(&a, &n)
        .unwrap();
    let classes = [
        OriginalProfileClass::G2,
        OriginalProfileClass::C0,
        OriginalProfileClass::C0,
        OriginalProfileClass::C0,
        OriginalProfileClass::C0,
    ]
    .repeat(2);
    let body = match certify_original_classified_closed_material(
        &sweep,
        &[1, 1],
        &material,
        &classes,
        projection(),
        1e-8,
        limits(),
    )
    .unwrap()
    {
        OriginalClassifiedClosedMaterialAttempt::Certified(body) => body,
        OriginalClassifiedClosedMaterialAttempt::Refused(r) => {
            panic!("multispan ownership: {:?}, {:?}", r.reason, r.volume)
        }
    };
    assert_eq!(body.volume().loop_sizes(), &[1, 1]);
    assert_eq!(body.bindings().len(), 10);
    for (i, binding) in body.bindings().iter().enumerate() {
        assert_eq!(binding.ring(), i / 5);
        assert_eq!(binding.profile(), (i / 5, i % 5));
        assert_eq!(binding.trim(), (0, i % 5));
        assert_eq!(binding.kind(), MaterialSpanBindingKind::NormalizedMap);
    }
    assert_eq!(body.boundary_classes(), classes);
    assert_eq!(
        (&profiles, &material.surface, &material.trims),
        (&before.0, &before.1.surface, &before.1.trims)
    );
}
