use brep_core::{Model, face_cut, minkowski3d::curved, operations};
use nurbs_core::curve::Curve;
#[path = "support/seam_graph.rs"]
mod fixture;
fn split() -> (Model, Vec<usize>) {
    let a = fixture::source();
    let path = face_cut::Segment {
        pcurve: Curve::from_polyline(vec![vec![0., 0.], vec![1., 1.]]).unwrap(),
        curve: Curve {
            degree: 4,
            knots: [vec![0.; 5], vec![1.; 5]].concat(),
            control_points: (0..5)
                .map(|i| {
                    vec![
                        i as f64 / 4.,
                        i as f64 / 4.,
                        1. + if i == 2 { 0.25 } else { 0. },
                    ]
                })
                .collect(),
            weights: vec![1.; 5],
            periodic: false,
        },
    };
    let p = face_cut::partition(&a, 1, [4, 6], &[path], 1e-8, 100000).unwrap();
    (p.model, p.faces.to_vec())
}
fn tool() -> Model {
    brep_core::cuboid([0.1875, 0.21875, 0.75], [0.3125, 0.34375, 2.]).unwrap()
}
#[test]
fn source_exact_uv_cancellation_removes_the_complete_original_seam() {
    let (m, faces) = split();
    let before = m.clone();
    let r = curved::certify_same_carrier_merge(&m, &faces, 0).unwrap();
    curved::verify_same_carrier_merge(&m, r.model(), r.certificate()).unwrap();
    assert_eq!(r.model().faces.len(), 6);
    assert_eq!(r.certificate().removed_edges.len(), 1);
    assert_eq!(r.certificate().merged_profile.orientation, vec![1]);
    r.model().validate_continuous(32768).unwrap();
    let mut reversed = faces.clone();
    reversed.reverse();
    assert_eq!(
        curved::certify_same_carrier_merge(&m, &reversed, 0)
            .unwrap()
            .certificate(),
        r.certificate()
    );
    let mut forged = r.certificate().clone();
    forged.removed_edges.clear();
    assert!(curved::verify_same_carrier_merge(&m, r.model(), &forged).is_err());
    let mut forged = r.certificate().clone();
    forged.merged_profile.orientation[0] = -1;
    assert!(curved::verify_same_carrier_merge(&m, r.model(), &forged).is_err());
    let mut forged = r.model().clone();
    forged.vertices[0].point[0] += 0.01;
    assert!(curved::verify_same_carrier_merge(&m, &forged, r.certificate()).is_err());
    assert_eq!(m, before);
}
#[test]
fn standard_boolean_cuts_across_the_saved_original_nonplanar_seam() {
    let (original, faces) = split();
    let a: Model = value_codec::from_str(&value_codec::to_string(&original).unwrap()).unwrap();
    assert_eq!(a, original);
    let b = tool();
    for face in faces {
        assert!(curved::certify_graph_box_boolean(&a, &b, "difference", face, 0).is_err());
    }
    for op in ["difference", "intersection"] {
        let r = operations::certify_seam_normalized_boolean(&a, &b, op, 0).unwrap();
        assert_eq!(r.certificate().merges[0].len(), 1);
        assert!(r.certificate().merges[1].is_empty());
        r.model().validate_continuous(32768).unwrap();
        operations::verify_seam_normalized_boolean(&a, &b, r.model(), r.certificate()).unwrap();
        assert_eq!(&operations::boolean(&a, &b, op).unwrap(), r.model());
        let mut forged = r.certificate().clone();
        forged.merges[0].clear();
        assert!(operations::verify_seam_normalized_boolean(&a, &b, r.model(), &forged).is_err());
    }
    let reversed = operations::certify_seam_normalized_boolean(&b, &a, "intersection", 0).unwrap();
    assert_eq!(
        &operations::boolean(&b, &a, "intersection").unwrap(),
        reversed.model()
    );
}
#[test]
fn different_carriers_and_open_or_relabelled_regions_cannot_be_coalesced() {
    let (a, faces) = split();
    assert!(curved::certify_same_carrier_merge(&a, &[faces[0], faces[0]], 0).is_err());
    assert!(curved::certify_same_carrier_merge(&a, &[faces[0], 0], 0).is_err());
    assert!(curved::certify_same_carrier_merge(&a, &faces, 4).is_err());
    let mut changed = a.clone();
    changed.faces[faces[1]].surface.control_points[1][1][2] += 1. / 1024.;
    assert!(curved::certify_same_carrier_merge(&changed, &faces, 0).is_err());
    let mut changed = a.clone();
    changed.shells[0].closed = false;
    assert!(curved::certify_same_carrier_merge(&changed, &faces, 0).is_err());
}

#[test]
fn closed_original_seams_can_be_crossed_without_filling_a_previous_pocket() {
    use nurbs_core::surface::Axis;
    let original = fixture::source();
    let first = operations::boolean(&original, &tool(), "difference").unwrap();
    let roof = first
        .faces
        .iter()
        .position(|f| f.surface == original.faces[1].surface)
        .unwrap();
    let s = &first.faces[roof].surface;
    let uv = [[0.375, 0.125], [0.5, 0.125], [0.5, 0.25], [0.375, 0.25]];
    let curves = [
        s.iso(Axis::V, 0.125).unwrap().trim(0.375, 0.5).unwrap(),
        s.iso(Axis::U, 0.5).unwrap().trim(0.125, 0.25).unwrap(),
        s.iso(Axis::V, 0.25)
            .unwrap()
            .trim(0.375, 0.5)
            .unwrap()
            .reverse()
            .unwrap(),
        s.iso(Axis::U, 0.375)
            .unwrap()
            .trim(0.125, 0.25)
            .unwrap()
            .reverse()
            .unwrap(),
    ];
    let path = curves
        .into_iter()
        .enumerate()
        .map(|(i, mut curve)| {
            // One literal shared endpoint per corner; these proposals must
            // still pass partition_closed's fresh whole-curve lift checks.
            curve.control_points[0] = s.evaluate(uv[i][0], uv[i][1]).unwrap().point.to_vec();
            let last = curve.control_points.len() - 1;
            curve.control_points[last] = s
                .evaluate(uv[(i + 1) % 4][0], uv[(i + 1) % 4][1])
                .unwrap()
                .point
                .to_vec();
            face_cut::Segment {
                curve,
                pcurve: Curve::from_polyline(vec![uv[i].to_vec(), uv[(i + 1) % 4].to_vec()])
                    .unwrap(),
            }
        })
        .collect::<Vec<_>>();
    let divided = face_cut::partition_closed(&first, roof, &path, 1e-8, 100000).unwrap();
    let merge = curved::certify_same_carrier_merge(&divided.model, &divided.faces, 0).unwrap();
    assert_eq!(merge.certificate().removed_edges.len(), 4);
    assert_eq!(
        merge.model().faces[merge.certificate().result_face]
            .holes
            .len(),
        1
    );
    let second_tool = brep_core::cuboid([0.34375, 0.15625, 0.75], [0.40625, 0.21875, 2.]).unwrap();
    let cut = operations::boolean(&divided.model, &second_tool, "difference").unwrap();
    cut.validate_continuous(65536).unwrap();
    assert_eq!(
        cut.faces
            .iter()
            .find(|f| f.surface == original.faces[1].surface)
            .unwrap()
            .holes
            .len(),
        2
    );
}

#[test]
fn a_curved_original_uv_seam_preserves_standard_operation_authority() {
    let cube = brep_core::cuboid([0.; 3], [1.; 3]).unwrap();
    let pc = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![0., 0.], vec![0.5, 0.25], vec![1., 1.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let c = Curve {
        control_points: pc
            .control_points
            .iter()
            .map(|p| vec![p[0], p[1], 1.])
            .collect(),
        ..pc.clone()
    };
    let split = face_cut::partition(
        &cube,
        1,
        [4, 6],
        &[face_cut::Segment {
            curve: c,
            pcurve: pc,
        }],
        1e-8,
        100000,
    )
    .unwrap();
    let b = brep_core::cuboid([0.28125, 0.171875, 0.75], [0.40625, 0.265625, 2.]).unwrap();
    for op in ["union", "intersection", "difference"] {
        let r = operations::certify_seam_normalized_boolean(&split.model, &b, op, 0)
            .unwrap_or_else(|e| panic!("{op}: {e:?}"));
        assert_eq!(r.certificate().merges[0][0].removed_edges.len(), 1);
        let expected = operations::boolean(&cube, &b, op).unwrap();
        assert_eq!(r.model().0, expected.0);
        assert_eq!(
            operations::boolean(&split.model, &b, op).unwrap().0,
            expected.0
        );
        operations::verify_seam_normalized_boolean(&split.model, &b, r.model(), r.certificate())
            .unwrap();
    }
    // The existing orthogonal author refuses this touching-component XOR;
    // exact seam removal must not turn that refusal into a proof of authorship.
    let expected = operations::boolean(&cube, &b, "xor").unwrap_err();
    let refused =
        operations::certify_seam_normalized_boolean(&split.model, &b, "xor", 0).unwrap_err();
    assert_eq!(refused.code, expected.code);
}
