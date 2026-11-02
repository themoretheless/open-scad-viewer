use geometry_bridge::dispatch;
use nurbs_core::curve::Curve;
use value_codec::{Value, json};

fn artifact(weight: f64, hole: bool) -> Value {
    let line =
        |a: [f64; 2], b: [f64; 2]| Curve::from_polyline(vec![a.to_vec(), b.to_vec()]).unwrap();
    let curved = |p: [[f64; 2]; 3]| Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: p.map(|p| p.to_vec()).to_vec(),
        weights: vec![1., weight, 1.],
        periodic: false,
    };
    let mut loops = vec![vec![
        line([0., 0.], [4., 0.]),
        curved([[4., 0.], [5., 4.], [4., 8.]]),
        line([4., 8.], [0., 8.]),
        curved([[0., 8.], [-1., 4.], [0., 0.]]),
    ]];
    if hole {
        let p = [[1., 1.], [1., 2.], [2., 2.], [2., 1.], [1., 1.]];
        loops.push(p.windows(2).map(|p| line(p[0], p[1])).collect());
    }
    let source = brep_core::prism::extrude(&loops, 0., 6.).unwrap();
    dispatch(json!({"op":"cad_full_round_exact","source":source,"faces":[3,2,1]})).unwrap()
}
fn display(artifact: &Value, segments: usize) -> Value {
    dispatch(json!({"op":"cad_full_round_exact_display","artifact":artifact,"segments":segments}))
        .unwrap()
}
#[test]
fn rational_display_is_closed_oriented_and_keeps_actual_face_identity() {
    for weight in [1., 0.7] {
        let a = artifact(weight, false);
        for segments in [2, 8, 16] {
            let mesh = display(&a, segments);
            assert_eq!(mesh["report"]["closed"].as_bool(), Some(true));
            assert_eq!(mesh["report"]["orientationConflicts"].as_u64(), Some(0));
            assert_eq!(mesh["report"]["degenerateTriangles"].as_u64(), Some(0));
            assert!(mesh["report"]["signedVolumeMm3"].as_f64().unwrap() > 0.);
            assert_eq!(mesh["geometricErrorCertified"].as_bool(), Some(false));
            assert_eq!(mesh["report"]["errorBoundCertified"].as_bool(), Some(false));
            assert_eq!(
                mesh["topologyFaceIds"],
                a["geometry"]["topologyIds"]["faces"]
            );
            let ids = mesh["faceIds"].as_array().unwrap();
            assert_eq!(
                ids.len(),
                mesh["displayIndices"].as_array().unwrap().len() / 3
            );
            assert!(ids.iter().all(|id| id.as_u64().unwrap()
                < mesh["topologyFaceIds"].as_array().unwrap().len() as u64));
        }
    }
}
#[test]
fn display_caps_preserve_the_through_hole() {
    let a = artifact(0.7, true);
    let mesh = display(&a, 8);
    assert_eq!(mesh["report"]["closed"].as_bool(), Some(true));
    let positions: Vec<f64> = value_codec::from_value(mesh["displayVertices"].clone()).unwrap();
    let indices: Vec<usize> = value_codec::from_value(mesh["displayIndices"].clone()).unwrap();
    let face_ids: Vec<usize> = value_codec::from_value(mesh["faceIds"].clone()).unwrap();
    let caps = mesh["topologyFaceIds"].as_array().unwrap().len() - 2;
    for (triangle, face) in indices.chunks_exact(3).zip(face_ids) {
        if face < caps {
            continue;
        }
        let p = triangle
            .iter()
            .map(|i| [positions[6 * i], positions[6 * i + 1]])
            .collect::<Vec<_>>();
        let cross = |a: [f64; 2], b: [f64; 2], c: [f64; 2]| {
            (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
        };
        for q in [[1.5, 1.5], [1.1, 1.1], [1.9, 1.9]] {
            let signs = [
                cross(p[0], p[1], q),
                cross(p[1], p[2], q),
                cross(p[2], p[0], q),
            ];
            assert!(
                !(signs.iter().all(|v| *v >= 0.) || signs.iter().all(|v| *v <= 0.)),
                "cap filled hole"
            );
        }
    }
}
#[test]
fn display_refuses_bad_budget_and_forged_geometry() {
    let a = artifact(0.7, false);
    for (segments, budget) in [(0, 20000), (33, 20000), (8, 2), (8, 0)] {
        assert!(dispatch(json!({"op":"cad_full_round_exact_display","artifact":a,"segments":segments,"maxTriangles":budget})).is_err());
    }
    let mut bad = a.clone();
    bad["geometry"]["vertices"][0]["point"][0] = json!(["999", "1"]);
    assert!(
        dispatch(json!({"op":"cad_full_round_exact_display","artifact":bad,"segments":8})).is_err()
    );
}

#[test]
fn box_display_volume_converges_to_an_independent_round_profile_reference() {
    let source = brep_core::cuboid([0.; 3], [4., 8., 6.]).unwrap();
    let a = dispatch(json!({"op":"cad_full_round_exact","source":source,"faces":[5,4,3]})).unwrap();
    // 4 x 6 rectangle plus a radius-2 semicircle, extruded by 6.
    // This checks display convergence, not an exact mass certificate.
    let reference = 6. * (24. + 2. * std::f64::consts::PI);
    let mut errors = Vec::new();
    for segments in [2, 8, 16] {
        let mesh = display(&a, segments);
        let volume = mesh["report"]["signedVolumeMm3"].as_f64().unwrap();
        assert!(volume < reference);
        errors.push(reference - volume);
    }
    assert!(errors.windows(2).all(|e| e[1] < e[0]));
    assert!(errors[2] < 0.1 && errors[2] < errors[0] / 20.);
}
