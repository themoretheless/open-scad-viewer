use super::*;
#[test]
fn whole_original_guided_affine_spatial_carrier_uses_actual_uv_charts() {
    let path = source::path();
    let mut guide = path.clone();
    for p in &mut guide.control_points {
        p[2] += 1.;
    }
    let (_, _, twist) = source::laws();
    let (_, mut cap, _) = source::material();
    for (i, row) in cap.control_points.iter_mut().enumerate() {
        for (j, p) in row.iter_mut().enumerate() {
            p[2] += (if i == j { 1. } else { -1. }) / 32768.;
        }
    }
    let mut profiles = polygon(1.);
    profiles.extend(polygon(0.25));
    for p in profiles.iter_mut().flat_map(|c| &mut c.control_points) {
        p[2] += 16. * p[0] * p[1];
    }
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let axes = constant_vector_law([1., 1.125, 0.875]).unwrap();
    let center = constant_vector_law([1. / 1024., 0., 0.]).unwrap();
    let sweep = MultiSweep::new(
        &profiles,
        &path,
        &scale,
        &twist,
        Options {
            normal: [0., 0., 1.],
            orientation: Orientation::RotationMinimizing,
            spacing: Spacing::ArcLength {
                tolerance: 0.001,
                max_cells: 100000,
            },
            initial_sections: 33,
            max_sections: 129,
            max_deviation: 0.05,
        },
    )
    .unwrap()
    .with_orientation_guide(&guide)
    .unwrap()
    .with_affine_laws(&axes, &center)
    .unwrap();
    let r = sweep
        .certify_ideal_volume_atlas([&cap, &cap], projection(), 999970, 1000000)
        .unwrap();
    eprintln!("native guided spatial carrier: {r:?}");
    assert!(
        r.embedding_certified && r.regularity_certified && r.parameter_topology_preserved,
        "{r:?}"
    );
}
