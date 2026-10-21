use nurbs_core::{
    intersection::ss_intersection::{self as ss, CoverageProof, SurfaceSurfaceComponent as C},
    surface::Surface,
};
fn rect(fixed: usize, height: f64, axes: [usize; 2], range: [[f64; 2]; 2]) -> Surface {
    let p = |i: usize, j: usize| {
        let mut p = vec![0.; 3];
        p[fixed] = height;
        p[axes[0]] = range[0][i];
        p[axes[1]] = range[1][j];
        p
    };
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![vec![p(0, 0), p(0, 1)], vec![p(1, 0), p(1, 1)]],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
fn xy(z: f64, x: [f64; 2], y: [f64; 2]) -> Surface {
    rect(2, z, [0, 1], [x, y])
}
#[test]
fn near_parallel_gap_and_disjoint_coplanar_extents_are_proved_empty() {
    let a = xy(0., [0., 1.], [0., 1.]);
    for gap in [f64::from_bits(1), 2_f64.powi(-40)] {
        let r =
            ss::intersect_surface_surface_report(&a, &xy(gap, [0., 1.], [0., 1.]), None).unwrap();
        assert!(r.complete() && r.components.is_empty());
        assert_eq!(r.coverage_proof, CoverageProof::ControlHullDisjoint);
    }
    let r = ss::intersect_surface_surface_report(&a, &xy(0., [2., 3.], [0., 1.]), None).unwrap();
    assert!(r.complete() && r.components.is_empty());
}
#[test]
fn bounded_planar_overlap_and_crossing_use_clipped_source_domains() {
    let a = xy(0., [0., 2.], [0., 2.]);
    let b = xy(0., [1., 3.], [0.5, 1.5]);
    let r = ss::intersect_surface_surface_report(&a, &b, None).unwrap();
    assert!(r.complete());
    assert!(!r.summary().permits_topology_authorship);
    let C::Overlap(o) = &r.components[0] else {
        panic!("overlap expected")
    };
    assert_eq!(o.geometry_enclosure, [[1., 2.], [0.5, 1.5], [0., 0.]]);
    assert_eq!(o.first_uv_box, [0.5, 1., 0.25, 0.75]);
    assert_eq!(o.second_uv_box, [0., 0.5, 0., 1.]);
    let b = rect(1, 1., [0, 2], [[0.5, 1.5], [-1., 1.]]);
    let r = ss::intersect_surface_surface_report(&a, &b, None).unwrap();
    assert!(r.complete());
    let C::Exact(line) = &r.components[0] else {
        panic!("line expected")
    };
    assert_eq!(line.geometry_enclosure, [[0.5, 1.5], [1., 1.], [0., 0.]]);
    for sample in &line.samples {
        let pa = a
            .evaluate(sample.uv_first[0], sample.uv_first[1])
            .unwrap()
            .point;
        let pb = b
            .evaluate(sample.uv_second[0], sample.uv_second[1])
            .unwrap()
            .point;
        for k in 0..3 {
            assert!(
                (pa[k] - sample.point[k]).abs() < 1e-15 && (pb[k] - sample.point[k]).abs() < 1e-15
            );
        }
    }
    let outside = rect(1, 3., [0, 2], [[0., 2.], [-1., 1.]]);
    let r = ss::intersect_surface_surface_report(&a, &outside, None).unwrap();
    assert!(r.complete() && r.components.is_empty());
}
#[test]
fn sub_tolerance_transverse_branch_is_retained_and_reversed_charts_map_exactly() {
    let a = xy(0., [2., 0.], [0., 2.]);
    let length = 2_f64.powi(-35);
    let b = rect(1, 1., [0, 2], [[0., length], [-1., 1.]]);
    let report = ss::intersect_surface_surface_report(&a, &b, None).unwrap();
    assert!(report.complete());
    let C::Exact(line) = &report.components[0] else {
        panic!("short line must not disappear")
    };
    assert_eq!(line.geometry_enclosure, [[0., length], [1., 1.], [0., 0.]]);
    assert_eq!(line.samples.len(), 2);
    for sample in &line.samples {
        let pa = a
            .evaluate(sample.uv_first[0], sample.uv_first[1])
            .unwrap()
            .point;
        let pb = b
            .evaluate(sample.uv_second[0], sample.uv_second[1])
            .unwrap()
            .point;
        assert_eq!(pa, sample.point);
        assert_eq!(pb, sample.point);
    }
    assert!(!report.summary().permits_topology_authorship);
}

#[test]
fn identity_encloses_entire_surface_and_does_not_grant_topology() {
    let mut a = xy(0., [0., 1.], [0., 1.]);
    a.control_points[1][1][2] = 0.2;
    a.weights = vec![vec![1., 0.8], vec![1.2, 1.5]];
    let r = ss::intersect_surface_surface_report(&a, &a, None).unwrap();
    assert!(r.complete());
    assert_eq!(r.coverage_proof, CoverageProof::SameParameterIdentity);
    assert!(!r.summary().permits_topology_authorship);
    let C::Overlap(o) = &r.components[0] else {
        panic!("identity overlap expected")
    };
    assert_eq!(o.geometry_enclosure, [[0., 1.], [0., 1.], [0., 0.2]]);
}
#[test]
fn nonbinary_maps_and_nearly_parallel_crossings_never_gain_false_authority() {
    let a = xy(0., [0., 3.], [0., 1.]);
    let b = rect(0, 1., [1, 2], [[0., 1.], [-1., 1.]]);
    let r = ss::intersect_surface_surface_report(&a, &b, None).unwrap();
    assert!(!r.complete());
    assert_eq!(r.coverage_proof, CoverageProof::Unproven);
    assert!(!r.unresolved.is_empty());
    assert!(!r.summary().permits_topology_authorship);
    let a = xy(0., [0., 1.], [0., 1.]);
    let mut b = a.clone();
    for row in &mut b.control_points {
        for p in row {
            p[2] = (p[0] - 0.5) * 1e-12;
        }
    }
    let r = ss::intersect_surface_surface_report(&a, &b, None).unwrap();
    assert!(!r.complete());
    assert!(!r.components.iter().any(|c| matches!(c, C::Overlap(_))));
    assert!(!r.summary().permits_topology_authorship);
}
#[cfg(feature = "codec")]
#[test]
fn serialized_proof_scope_and_structural_verifier_revoke_unproved_authority() {
    use value_codec::{Serialize, json};
    let a = xy(0., [0., 2.], [0., 2.]);
    let b = xy(0., [1., 3.], [0.5, 1.5]);
    let r = ss::intersect_surface_surface_report(&a, &b, None).unwrap();
    let mut v = r.to_value();
    assert_eq!(v["coverage"]["proofScope"], "rectilinear_charts");
    assert_eq!(v["branchGraph"]["permitsTopologyAuthorship"], false);
    assert_eq!(v["uvArrangement"]["permitsTrimClassification"], false);
    assert_eq!(
        ss::verify_ss_coverage(&v).unwrap()["semanticProofReplayed"],
        false
    );
    v["coverage"]["proofScope"] = json!("unproven");
    assert!(ss::verify_ss_coverage(&v).is_err());
    let a = xy(0., [0., 1.], [0., 1.]);
    let mut b = a.clone();
    for row in &mut b.control_points {
        for p in row {
            p[2] = p[0] - 0.5;
        }
    }
    let r = ss::intersect_surface_surface_report(&a, &b, None).unwrap();
    let v = r.to_value();
    assert!(!r.unresolved.is_empty());
    assert_eq!(v["coverage"]["complete"], false);
    for component in v["components"].as_array().unwrap() {
        assert_eq!(component["evidenceScope"], "candidate_only");
        if component.get("pcurveFirst").is_some() {
            assert_eq!(component["pcurveFirst"]["correspondence"], "candidate_only");
        }
    }
}

#[cfg(feature = "codec")]
use nurbs_core::{ss_intersection, surface};

#[cfg(feature = "codec")]
#[test]
fn existing_bounded_and_general_surface_surface_codec_regression() {
    let xy = surface::Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![1., 0., 0.], vec![1., 1., 0.]],
        ],
        weights: vec![vec![1., 1.], vec![1., 1.]],
        periodic_u: false,
        periodic_v: false,
    };
    let xz = surface::Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 0., 1.]],
            vec![vec![1., 0., 0.], vec![1., 0., 1.]],
        ],
        weights: vec![vec![1., 1.], vec![1., 1.]],
        periodic_u: false,
        periodic_v: false,
    };
    let report = ss_intersection::intersect_surface_surface(&xy, &xz, None).unwrap();
    assert_eq!(report["version"], "nurbs-ss/1");
    assert_eq!(report["kind"], "surface_surface");
    assert_eq!(report["coverage"]["complete"], true);
    assert!(report["unresolved"].as_array().unwrap().is_empty());
    assert!(
        report["components"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["kind"] == "curve")
    );
    assert_eq!(report["booleanMutationAuthority"], false);
    assert_eq!(report["topologyAuthority"]["granted"], false);
    assert!(report["branchGraph"]["components"].is_array());
    assert!(report["uvArrangement"]["traces"].is_array());
    let audit = ss_intersection::verify_ss_coverage(&report).unwrap();
    assert_eq!(audit["complete"], true);
    assert!(
        audit["notes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|n| n == "no_graph_patch_iso_fixture"),
    );

    let coincident = ss_intersection::intersect_surface_surface(&xy, &xy, None).unwrap();
    assert!(
        coincident["components"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["kind"] == "overlap")
    );
    assert!(
        coincident["components"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c.get("coedgeTrim").is_some())
    );

    let above = surface::Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 2.], vec![0., 1., 2.]],
            vec![vec![1., 0., 2.], vec![1., 1., 2.]],
        ],
        weights: vec![vec![1., 1.], vec![1., 1.]],
        periodic_u: false,
        periodic_v: false,
    };
    let empty = ss_intersection::intersect_surface_surface(&xy, &above, None).unwrap();
    assert_eq!(empty["coverage"]["complete"], true);
    assert!(empty["components"].as_array().unwrap().is_empty());

    // Rational positive-weight bicubic transverse seed (no graph-patch iso path).
    let patch_a = surface::Surface {
        degree_u: 3,
        degree_v: 3,
        knots_u: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        control_points: vec![
            vec![
                vec![0., 0., 0.],
                vec![0., 1. / 3., 0.],
                vec![0., 2. / 3., 0.],
                vec![0., 1., 0.],
            ],
            vec![
                vec![1. / 3., 0., 0.],
                vec![1. / 3., 1. / 3., 0.1],
                vec![1. / 3., 2. / 3., 0.1],
                vec![1. / 3., 1., 0.],
            ],
            vec![
                vec![2. / 3., 0., 0.],
                vec![2. / 3., 1. / 3., 0.1],
                vec![2. / 3., 2. / 3., 0.1],
                vec![2. / 3., 1., 0.],
            ],
            vec![
                vec![1., 0., 0.],
                vec![1., 1. / 3., 0.],
                vec![1., 2. / 3., 0.],
                vec![1., 1., 0.],
            ],
        ],
        weights: vec![
            vec![1., 1., 1., 1.],
            vec![1., 1.2, 0.8, 1.],
            vec![1., 0.9, 1.1, 1.],
            vec![1., 1., 1., 1.],
        ],
        periodic_u: false,
        periodic_v: false,
    };
    let cutter = surface::Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0.5, -1.], vec![0., 0.5, 1.]],
            vec![vec![1., 0.5, -1.], vec![1., 0.5, 1.]],
        ],
        weights: vec![vec![1., 1.], vec![1., 1.]],
        periodic_u: false,
        periodic_v: false,
    };
    let general = ss_intersection::intersect_surface_surface(&patch_a, &cutter, None).unwrap();
    assert_eq!(general["version"], "nurbs-ss/1");
    assert!(general["coverage"]["boxesVisited"].as_u64().unwrap() > 0);
    // Either complete components or typed unresolved — never silent false Complete.
    if general["coverage"]["complete"] == true {
        assert!(general["unresolved"].as_array().unwrap().is_empty());
    } else {
        assert!(!general["unresolved"].as_array().unwrap().is_empty());
        assert_eq!(general["topologyAuthority"]["revoked"], true);
    }
    assert_eq!(general["booleanMutationAuthority"], false);
    assert!(ss_intersection::ss_resource_probe(0, 1, 4).is_err());
    assert!(ss_intersection::ss_resource_probe(26, 1, 4).is_err());
    assert!(ss_intersection::ss_resource_probe(3, 3, 257).is_err());
}
