use super::*;
#[test]
fn actual_dense_quadratic_profile_proves_g1_without_promoting_curvature_jumps() {
    // Sixteen exact quadratic pieces with equal endpoint first jets and
    // alternating curvature. Decomposition must preserve these distinctions.
    let mut controls = vec![vec![0.; 3]];
    for i in 0..16 {
        let slope = if i % 2 == 0 { 0. } else { 0.25 };
        controls.push(vec![i as f64 + 0.5, i as f64 * 0.125 + slope * 0.5, 0.]);
        controls.push(vec![(i + 1) as f64, (i + 1) as f64 * 0.125, 0.]);
    }
    let profile = Curve {
        degree: 2,
        knots: [
            vec![0.; 3],
            (1..16).flat_map(|i| [i as f64 / 16.; 2]).collect(),
            vec![1.; 3],
        ]
        .concat(),
        control_points: controls,
        weights: vec![1.; 33],
        periodic: false,
    };
    let path = crate::primitives::line([0.; 3], [0., 0., 4.]).unwrap();
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let profiles = [profile];
    let options = Options {
        normal: [1., 0., 0.],
        orientation: Orientation::Fixed,
        spacing: Spacing::Parameter,
        initial_sections: 2,
        max_sections: 2,
        max_deviation: 0.01,
    };
    let level = MultiSweep::new(&profiles, &path, &scale, &twist, options)
        .unwrap()
        .preview_at(2)
        .unwrap();
    let proof = level
        .certify_retained_decomposition_smoothness(1., 1000000)
        .unwrap();
    assert_eq!(proof.g2.expected_joins, 15);
    assert!(proof.g2.coverage_complete && !proof.g2.all_joins_certified);
    let g1 = proof.g1.as_ref().unwrap();
    assert!(
        g1.coverage_complete && g1.all_joins_certified && proof.g1_certified,
        "{proof:?}"
    );
    assert_eq!(proof.exact_work, proof.g2.exact_work + g1.exact_work);
    assert!(proof.exact_work <= proof.max_work);
    #[cfg(feature = "transport")]
    {
        let r = crate::transport::dispatch(value_codec::json!({
            "op":"surface_progressive_sweep_decomposition_smoothness",
            "profiles":profiles,"path":path,"scale":scale,"twist":twist,
            "normal":[1.,0.,0.],"orientation":"fixed","spacing":"parameter",
            "initial_sections":2,"max_sections":2,"max_deviation":0.01,
            "preview_sections":2,"transverseScale":1.,"maxExactWork":1000000}))
        .unwrap();
        assert_eq!(r["decompositionG1Certified"], true);
        assert_eq!(r["g2"]["decompositionJoinsCertified"], false);
        assert_eq!(r["g1"]["decompositionJoinsCertified"], true);
        assert_eq!(r["g1"]["requestedOrder"], 1);
        assert_eq!(r["g2"]["requestedOrder"], 2);
        assert_eq!(r["scope"], "within-source-profile-decomposition-only");
        for key in [
            "allProfileJoinsCertified",
            "closedProfileSeamsCertified",
            "sourceFrameSmoothnessCertified",
            "capJoinsCertified",
            "continuousBound",
            "solidCertified",
        ] {
            assert_eq!(r[key], false);
        }
    }
    let zero = level
        .certify_retained_decomposition_smoothness(1., 0)
        .unwrap();
    assert!(!zero.g1_certified && !zero.g2.all_joins_certified);
    // A middle control change creates a true tangent corner, not just G2 loss.
    let mut corner = level;
    corner.patches[1].control_points[1][0][1] += 0.125;
    corner.patches[1].control_points[1][1][1] += 0.125;
    assert!(
        !corner
            .certify_retained_decomposition_smoothness(1., 1000000)
            .unwrap()
            .g1_certified
    );
}
#[test]
fn actual_dense_profile_decomposition_joins_cover_all_station_chunks() {
    // 33 original poles force native profile decomposition; 65 stations
    // force three literal station chunks per profile part.
    let profile = Curve {
        degree: 1,
        knots: [
            vec![0.],
            (0..=32).map(|i| i as f64 / 32.).collect(),
            vec![1.],
        ]
        .concat(),
        control_points: (0..=32).map(|i| vec![i as f64 / 32., 0., 0.]).collect(),
        weights: vec![1.; 33],
        periodic: false,
    };
    let path = crate::primitives::line([0.; 3], [0., 0., 4.]).unwrap();
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let options = Options {
        normal: [1., 0., 0.],
        orientation: Orientation::Fixed,
        spacing: Spacing::Parameter,
        initial_sections: 5,
        max_sections: 65,
        max_deviation: 0.01,
    };
    let level = Sweep::new(&profile, &path, &scale, &twist, options)
        .unwrap()
        .preview_at(65)
        .unwrap();
    assert_eq!(level.patches.len(), 32 * 3);
    let profiles = [profile.clone(), profile.clone()];
    let multi = MultiSweep::new(&profiles, &path, &scale, &twist, options)
        .unwrap()
        .preview_at(65)
        .unwrap();
    let pairs = multi.retained_decomposition_join_pairs().unwrap();
    assert_eq!(pairs.len(), 2 * 31 * 3);
    let aggregate = multi
        .certify_retained_decomposition_joins(2, 1., 1000000)
        .unwrap();
    assert_eq!(aggregate.expected_joins, 186);
    assert!(!aggregate.all_joins_certified);
    assert_eq!(aggregate.coverage_complete, aggregate.joins.len() == 186);
    assert!(aggregate.exact_work <= 1000000);
    if !aggregate.coverage_complete {
        assert_eq!(
            aggregate.reason,
            Some("retained-decomposition-work-unproved")
        );
    }
    let endpoints = MultiSweep::new(
        &profiles,
        &path,
        &scale,
        &twist,
        Options {
            initial_sections: 2,
            ..options
        },
    )
    .unwrap()
    .preview_at(2)
    .unwrap();
    let positive = endpoints
        .certify_retained_decomposition_joins(2, 1., 1000000)
        .unwrap();
    assert!(
        positive.coverage_complete && positive.all_joins_certified,
        "{positive:?}"
    );
    assert_eq!(positive.expected_joins, 62);
    #[cfg(feature = "transport")]
    {
        let request = value_codec::json!({"op":"surface_progressive_sweep_decomposition_joins",
            "profiles":profiles,"path":path,"scale":scale,"twist":twist,
            "normal":[1.,0.,0.],"orientation":"fixed","spacing":"parameter",
            "initial_sections":2,"max_sections":65,"max_deviation":0.01,
            "preview_sections":2,"order":2,"transverseScale":1.,"maxExactWork":1000000});
        let r = crate::transport::dispatch(request.clone()).unwrap();
        assert_eq!(r["decompositionJoinsCertified"], true);
        assert_eq!(r["coverageComplete"], true);
        assert_eq!(r["expectedJoins"], 62);
        assert_eq!(r["scope"], "within-source-profile-decomposition-only");
        for key in [
            "allProfileJoinsCertified",
            "closedProfileSeamsCertified",
            "sourceFrameSmoothnessCertified",
            "capJoinsCertified",
            "continuousBound",
            "solidCertified",
        ] {
            assert_eq!(r[key], false);
        }
        let mut zero = request;
        zero["maxExactWork"] = value_codec::json!(0);
        let r = crate::transport::dispatch(zero).unwrap();
        assert_eq!(r["coverageComplete"], false);
        assert_eq!(r["decompositionJoinsCertified"], false);
    }
    let short = endpoints
        .certify_retained_decomposition_joins(2, 1., positive.exact_work - 1)
        .unwrap();
    assert!(!short.all_joins_certified && short.exact_work < positive.exact_work);
    let zero = endpoints
        .certify_retained_decomposition_joins(2, 1., 0)
        .unwrap();
    assert!(!zero.coverage_complete && !zero.all_joins_certified && zero.joins.is_empty());
    assert!(!pairs.iter().any(|p| p[0] < 96 && p[1] >= 96));
    for part in 0..31 {
        for chunk in 0..3 {
            assert!(pairs.contains(&[part * 3 + chunk, (part + 1) * 3 + chunk]));
        }
    }
    let mut orphan = multi.clone();
    orphan.profile_patch_ranges.pop();
    assert!(orphan.retained_decomposition_join_pairs().is_err());
    let mut missing = multi;
    missing.patches.remove(1);
    assert!(missing.retained_decomposition_join_pairs().is_err());
    let mut work = 0;
    let mut certified = 0;
    let mut unresolved = 0;
    for part in 0..31 {
        for chunk in 0..3 {
            let proof = level
                .certify_retained_profile_join(
                    part * 3 + chunk,
                    (part + 1) * 3 + chunk,
                    2,
                    1.,
                    1000000 - work,
                )
                .unwrap();
            if proof.certified {
                certified += 1;
            } else {
                // Binary64 station generation can leave tiny exact jet
                // differences in a nominally planar construction. Preserve
                // refusal rather than infer retained G2 from the source.
                assert_eq!(
                    proof.reason, "linear-along-jet-smoothness-unproved",
                    "part={part}, chunk={chunk}: {proof:?}"
                );
                unresolved += 1;
            }
            work += proof.work;
        }
    }
    assert!(work > 0 && work <= 1000000);
    assert!(certified > 0 && unresolved > 0);
    // Adjacent flattened indices may belong to disjoint station chunks.
    assert!(
        level
            .certify_retained_profile_join(0, 1, 2, 1., 1000000)
            .is_err()
    );
    let mut corner = profile;
    corner.control_points[16][1] = 0.25;
    let corner = Sweep::new(&corner, &path, &scale, &twist, options)
        .unwrap()
        .preview_at(65)
        .unwrap();
    assert!(
        !corner
            .certify_retained_profile_join(15 * 3, 16 * 3, 1, 1., 1000000)
            .unwrap()
            .certified
    );
}
#[test]
fn retained_profile_join_requires_station_ownership_and_exact_jets() {
    let profile = crate::primitives::line([0.; 3], [1., 0., 0.]).unwrap();
    let path = crate::primitives::line([0.; 3], [0., 0., 4.]).unwrap();
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let options = Options {
        normal: [1., 0., 0.],
        orientation: Orientation::Fixed,
        spacing: Spacing::Parameter,
        initial_sections: 3,
        max_sections: 5,
        max_deviation: 0.01,
    };
    let mut level = Sweep::new(&profile, &path, &scale, &twist, options)
        .unwrap()
        .preview_at(3)
        .unwrap();
    let plane = |x: f64| Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: (0..2)
            .map(|u| {
                (0..2)
                    .map(|v| vec![x + u as f64, 0., 4. * v as f64])
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    level.patches = vec![plane(0.), plane(1.)];
    let positive = level
        .certify_retained_profile_join(0, 1, 2, 1., 1000000)
        .unwrap();
    assert!(
        positive.certified && positive.regularity_certified,
        "{positive:?}"
    );
    assert!(
        !level
            .certify_retained_profile_join(0, 1, 2, 1., positive.work - 1)
            .unwrap()
            .certified
    );
    let mut wrong_interval = level.clone();
    wrong_interval.patches[1].knots_v = vec![0.5, 0.5, 1., 1.];
    assert!(
        wrong_interval
            .certify_retained_profile_join(0, 1, 2, 1., 1000000)
            .is_err()
    );
    for p in &mut level.patches[1].control_points[1] {
        p[1] = 0.25;
    }
    let corner = level
        .certify_retained_profile_join(0, 1, 1, 1., 1000000)
        .unwrap();
    assert!(!corner.certified && !corner.exact_identity);
    assert!(
        level
            .certify_retained_profile_join(0, 0, 1, 1., 1000000)
            .is_err()
    );
}
#[test]
#[cfg(feature = "transport")]
fn retained_profile_join_transport_reconstructs_geometry_and_preserves_scope() {
    let profile = crate::primitives::line([0.; 3], [1., 0., 0.]).unwrap();
    let next = crate::primitives::line([1., 0., 0.], [2., 0., 0.]).unwrap();
    let path = crate::primitives::line([0.; 3], [0., 0., 4.]).unwrap();
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let request = value_codec::json!({"op":"surface_progressive_sweep_profile_join",
        "profiles":[profile,next],"path":path,"scale":scale,"twist":twist,
        "normal":[1.,0.,0.],"orientation":"fixed","spacing":"parameter",
        "initial_sections":3,"max_sections":5,"max_deviation":0.01,
        "preview_sections":3,"leftPatch":0,"rightPatch":1,"order":2,
        "transverseScale":1.,"maxExactWork":1000000});
    let proof = crate::transport::dispatch(request.clone()).unwrap();
    assert_eq!(proof["certified"], true);
    assert_eq!(proof["regularityCertified"], true);
    assert_eq!(proof["scope"], "explicit-retained-profile-join-only");
    for key in [
        "allProfileJoinsCertified",
        "sourceFrameSmoothnessCertified",
        "capJoinsCertified",
        "continuousBound",
        "solidCertified",
    ] {
        assert_eq!(proof[key], false);
    }
    let mut short = request;
    short["maxExactWork"] = value_codec::json!(0);
    assert_eq!(
        crate::transport::dispatch(short).unwrap()["certified"],
        false
    );
}
#[test]
#[cfg(feature = "transport")]
fn retained_station_jet_transport_keeps_source_and_solid_scope_separate() {
    let profile = crate::primitives::line([1., 0., 0.], [2., 0., 0.]).unwrap();
    let path = crate::primitives::line([0.; 3], [0., 0., 4.]).unwrap();
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let request = value_codec::json!({"op":"surface_progressive_sweep_station_seams","profiles":[profile],"path":path,"scale":scale,"twist":twist,
        "normal":[1.,0.,0.],"orientation":"rmf","spacing":"parameter","initial_sections":3,"max_sections":5,"max_deviation":0.01,"preview_sections":5,"order":2,"maxExactWork":1000000});
    let proof = crate::transport::dispatch(request.clone()).unwrap();
    assert_eq!(proof["allStationSeamsCertified"], true);
    assert_eq!(proof["requestedOrder"], 2);
    assert_eq!(proof["sourceFrameSmoothnessCertified"], false);
    assert_eq!(proof["solidCertified"], false);
    assert_eq!(proof["scope"], "retained-station-seams-only");
    let mut short = request;
    short["maxExactWork"] = value_codec::json!(0);
    let unresolved = crate::transport::dispatch(short).unwrap();
    assert_eq!(unresolved["allStationSeamsCertified"], false);
}
#[test]
fn rational_multi_span_profile_station_jets_keep_group_and_budget_scope() {
    let profile = Curve {
        degree: 3,
        knots: vec![0., 0., 0., 0., 0.5, 1., 1., 1., 1.],
        control_points: vec![
            vec![1., 0., 0.],
            vec![2., 0.125, 0.],
            vec![3., 0.25, 0.],
            vec![4., 0.125, 0.],
            vec![5., 0., 0.],
        ],
        weights: vec![1., 0.75, 1.25, 0.875, 1.],
        periodic: false,
    };
    let path = crate::primitives::line([0.; 3], [0., 0., 4.]).unwrap();
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let sweep = Sweep::new(
        &profile,
        &path,
        &scale,
        &twist,
        Options {
            normal: [1., 0., 0.],
            orientation: Orientation::RotationMinimizing,
            spacing: Spacing::Parameter,
            initial_sections: 3,
            max_sections: 65,
            max_deviation: 0.01,
        },
    )
    .unwrap();
    let level = sweep.preview_at(5).unwrap();
    assert_eq!(level.patches.len(), 1);
    assert!(
        level
            .patches
            .iter()
            .any(|p| p.weights.iter().flatten().any(|w| *w != 1.))
    );
    let proof = level.certify_retained_station_seams(2, 1000000).unwrap();
    assert!(proof.all_station_seams_certified, "{proof:?}");
    assert_eq!(proof.seams.len(), 3);
    assert!(
        proof
            .seams
            .iter()
            .all(|r| r.c0_identity && r.regularity_certified && !r.closure)
    );
    let short = level
        .certify_retained_station_seams(2, proof.exact_work - 1)
        .unwrap();
    assert!(!short.all_station_seams_certified && short.exact_work <= proof.exact_work - 1);
    // A missing station chunk must not turn a partial surface group into
    // a vacuous positive audit, even when other seam predicates pass.
    let mut partial = sweep.preview_at(65).unwrap();
    partial.patches.remove(1);
    let missing = partial.certify_retained_station_seams(1, 1000000).unwrap();
    assert!(!missing.all_station_seams_certified);
    assert_eq!(
        missing.reason,
        Some("retained-station-chunk-correspondence-unproved")
    );
}
#[test]
fn rational_circle_station_g2_uses_exact_along_chain_and_closure_jets() {
    let profile = crate::primitives::circle([0.; 3], [0., 0., 1.], 1.).unwrap();
    let path = crate::primitives::line([0.; 3], [0., 0., 4.]).unwrap();
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let options = Options {
        normal: [1., 0., 0.],
        orientation: Orientation::RotationMinimizing,
        spacing: Spacing::Parameter,
        initial_sections: 3,
        max_sections: 5,
        max_deviation: 0.01,
    };
    let level = Sweep::new(&profile, &path, &scale, &twist, options)
        .unwrap()
        .preview_at(5)
        .unwrap();
    let proof = level.certify_retained_station_seams(2, 1000000).unwrap();
    assert!(proof.all_station_seams_certified, "{proof:?}");
    assert_eq!(proof.seams.len(), 3);
    assert!(
        proof
            .seams
            .iter()
            .all(|s| s.regularity_certified && s.c0_identity)
    );
    assert!(
        !level
            .certify_retained_station_seams(2, proof.exact_work - 1)
            .unwrap()
            .all_station_seams_certified
    );
    let mut wrong = profile.clone();
    wrong.control_points[3][0] = wrong.control_points[3][0].next_up();
    let level = Sweep::new(&wrong, &path, &scale, &twist, options)
        .unwrap()
        .preview_at(5)
        .unwrap();
    let refused = level.certify_retained_station_seams(2, 1000000).unwrap();
    assert!(!refused.all_station_seams_certified);
    assert!(refused.seams.iter().all(|s| s.c0_identity));
}
#[test]
fn actual_closed_rmf_station_seam_keeps_c0_distinct_from_g1() {
    let profile = crate::primitives::line([1., 0., 1.], [1., 0., 2.]).unwrap();
    let path = crate::primitives::circle([0.; 3], [0., 0., 1.], 1.).unwrap();
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let sweep = Sweep::new(
        &profile,
        &path,
        &scale,
        &twist,
        Options {
            normal: [0., 0., 1.],
            orientation: Orientation::RotationMinimizing,
            spacing: Spacing::Parameter,
            initial_sections: 5,
            max_sections: 33,
            max_deviation: 1.,
        },
    )
    .unwrap();
    let level = sweep.preview_at(33).unwrap();
    assert!(level.report.closed_path);
    let proof = level.certify_retained_station_seams(1, 1000000).unwrap();
    assert!(!proof.all_station_seams_certified);
    assert!(proof.seams.iter().all(|r| r.c0_identity), "{proof:?}");
    let closure = proof.seams.iter().filter(|r| r.closure).collect::<Vec<_>>();
    assert_eq!(closure.len(), 1);
    assert_eq!(proof.seams.len(), 32);
    assert!(closure.iter().all(|r| !r.certified));
}
#[test]
fn actual_retained_straight_sweep_seams_prove_g2_and_charge_shared_work() {
    let profile = crate::primitives::line([1., 0., 0.], [2., 0., 0.]).unwrap();
    let path = crate::primitives::line([0.; 3], [0., 0., 4.]).unwrap();
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let sweep = Sweep::new(
        &profile,
        &path,
        &scale,
        &twist,
        Options {
            normal: [1., 0., 0.],
            orientation: Orientation::RotationMinimizing,
            spacing: Spacing::Parameter,
            initial_sections: 3,
            max_sections: 65,
            max_deviation: 0.01,
        },
    )
    .unwrap();
    let across = sweep
        .preview_at(65)
        .unwrap()
        .certify_retained_station_seams(2, 1000000)
        .unwrap();
    assert!(across.all_station_seams_certified, "{across:?}");
    assert_eq!(across.seams.len(), 63);
    let level = sweep.preview_at(5).unwrap();
    let proof = level.certify_retained_station_seams(2, 1000000).unwrap();
    assert!(proof.all_station_seams_certified, "{proof:?}");
    assert_eq!(proof.seams.len(), 3);
    assert!(
        proof
            .seams
            .iter()
            .all(|r| r.c0_identity && r.regularity_certified)
    );
    assert!(proof.exact_work > 0);
    assert!(
        !level
            .certify_retained_station_seams(2, proof.exact_work - 1)
            .unwrap()
            .all_station_seams_certified
    );
    assert!(
        !level
            .certify_retained_station_seams(2, 0)
            .unwrap()
            .all_station_seams_certified
    );
    let mut kink = level.clone();
    kink.patches[0].control_points[1][2][0] += 0.125;
    let refused = kink.certify_retained_station_seams(1, 1000000).unwrap();
    assert!(
        !refused.all_station_seams_certified && refused.seams.iter().all(|r| r.c0_identity)
    );
    let mut singular = level;
    singular.patches[0].control_points[1] = singular.patches[0].control_points[0].clone();
    assert!(
        !singular
            .certify_retained_station_seams(2, 1000000)
            .unwrap()
            .all_station_seams_certified
    );
}
