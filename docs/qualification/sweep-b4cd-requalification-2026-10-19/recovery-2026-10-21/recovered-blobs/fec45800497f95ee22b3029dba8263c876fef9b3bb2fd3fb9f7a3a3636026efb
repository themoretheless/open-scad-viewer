use nurbs_core::thread::{self, Hand, Kind, Spec};
#[test]
fn complete_original_partitions_and_stored_uv_polygons_cover_the_axial_interior() {
    for (kind, hand, starts) in [
        (Kind::External, Hand::Right, 1),
        (Kind::External, Hand::Left, 1),
        (Kind::Internal, Hand::Right, 2),
    ] {
        let spec = Spec {
            kind,
            hand,
            starts,
            turns: 2,
            ..Spec::default()
        };
        let slab = [1.13, 1.87];
        let trims = thread::trim_candidates(spec, slab, 1e-3).unwrap();
        assert!(
            trims
                .candidates
                .iter()
                .any(|c| c.exact_uv_vertices.len() > 4)
        );
        if starts == 2 {
            assert!(
                trims
                    .candidates
                    .iter()
                    .any(|c| c.exact_uv_vertices.len() == 6)
            );
        }
        let proof = thread::certify_trim_candidates(spec, &trims, 1e-12, 1e-3, 2560).unwrap();
        assert!(proof.exact_affine_partition && proof.stored_uv_isotopy_certified);
        assert_eq!(proof.retained_patches + proof.excluded_patches, 40 * starts);
        assert!(proof.axial_error_band_upper < 1e-8);
        for c in &trims.candidates {
            let curve = c.pcurve(1e-12).unwrap().curve;
            for i in 0..=10 {
                for j in 0..=10 {
                    let u = i as f64 / 10.;
                    let v = j as f64 / 10.;
                    let z = c.patch.surface.evaluate(u, v).unwrap().point[2];
                    if z > slab[0] + proof.axial_error_band_upper + 1e-8
                        && z < slab[1] - proof.axial_error_band_upper - 1e-8
                    {
                        for p in curve.control_points.windows(2) {
                            let cross = (p[1][0] - p[0][0]) * (v - p[0][1])
                                - (p[1][1] - p[0][1]) * (u - p[0][0]);
                            assert!(cross >= -1e-12);
                        }
                    }
                }
            }
        }
    }
}
#[test]
fn omitted_changed_and_reversed_source_recipes_cannot_receive_coverage() {
    let spec = Spec {
        turns: 2,
        ..Spec::default()
    };
    let source = thread::trim_candidates(spec, [1.13, 1.87], 1e-3).unwrap();
    let check =
        |c: &thread::TrimCandidates| thread::certify_trim_candidates(spec, c, 1e-12, 1e-3, 2560);
    let mut omitted = source.clone();
    omitted.excluded.pop();
    assert!(check(&omitted).is_err());
    let mut changed = source.clone();
    changed.candidates[0].patch.surface.control_points[0][0][0] += 1e-5;
    assert!(check(&changed).is_err());
    let mut reversed = source.clone();
    reversed.candidates[0].exact_uv_vertices.reverse();
    assert!(check(&reversed).is_err());
    let mut excluded = source.clone();
    excluded.excluded[0].source_z_bounds = [-1e9, -1e9];
    assert!(check(&excluded).is_err());
    assert!(thread::certify_trim_candidates(spec, &source, 1e-12, 1e-3, 1).is_err());
    let mut observations = source.clone();
    observations.candidates[0].polygon_uv.fill([100., 100.]);
    assert!(check(&observations).is_ok());
}
