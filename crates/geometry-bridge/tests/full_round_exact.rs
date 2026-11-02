use brep_core::Model;
use nurbs_core::curve::Curve;
use value_codec::{json, Value};
fn source(weight: f64) -> (Model, [usize; 3]) {
    let curve = |points: [[f64; 2]; 3]| Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: points.map(|p| p.to_vec()).to_vec(),
        weights: vec![1., weight, 1.],
        periodic: false,
    };
    let line =
        |a: [f64; 2], b: [f64; 2]| Curve::from_polyline(vec![a.to_vec(), b.to_vec()]).unwrap();
    let profile = vec![
        line([0., 0.], [4., 0.]),
        curve([[4., 0.], [5., 4.], [4., 8.]]),
        line([4., 8.], [0., 8.]),
        curve([[0., 8.], [-1., 4.], [0., 0.]]),
    ];
    let m = brep_core::prism::extrude(&[profile], 0., 6.).unwrap();
    let side = |left: bool| {
        m.faces
            .iter()
            .position(|f| {
                f.surface.degree_u == 2
                    && f.surface.control_points.iter().flatten().all(|p| {
                        if left {
                            p[0] <= 0.
                        } else {
                            p[0] >= 4.
                        }
                    })
            })
            .unwrap()
    };
    let middle = m
        .faces
        .iter()
        .position(|f| {
            f.surface
                .control_points
                .iter()
                .flatten()
                .all(|p| p[1] == 8.)
        })
        .unwrap();
    let faces = [side(true), middle, side(false)];
    (m, faces)
}

#[test]
fn router_constructs_and_restores_actual_exact_topology_without_float_coefficients() {
    let (m, faces) = source(0.7);
    let artifact =
        geometry_bridge::dispatch(json!({"op":"cad_full_round_exact","source":m,"faces":faces}))
            .unwrap();
    assert_eq!(artifact["format"].as_str(), Some("admitted-full-round/1"));
    assert!(artifact["geometry"]["vertices"][0]["point"][0][0].is_string());
    let bytes = value_codec::encode_binary(&artifact).unwrap();
    let artifact: Value = value_codec::decode_binary(&bytes).unwrap();
    let restored =
        geometry_bridge::dispatch(json!({"op":"cad_full_round_exact_restore","artifact":artifact}))
            .unwrap();
    assert_eq!(restored["geometry"], artifact["geometry"]);
    let mut bad = artifact.clone();
    bad["geometry"]["edges"][0]["curve"]["homogeneousControls"][0][0][0] = json!("999");
    assert!(
        geometry_bridge::dispatch(json!({"op":"cad_full_round_exact_restore","artifact":bad}))
            .is_err()
    );
}
#[test]
fn router_refuses_caller_success_fields_and_insufficient_material_budget() {
    let (m, faces) = source(1.);
    assert!(geometry_bridge::dispatch(
        json!({"op":"cad_full_round_exact","source":m,"faces":faces,"noAddedMaterial":true})
    )
    .is_err());
    assert!(geometry_bridge::dispatch(json!({"op":"cad_full_round_exact","source":m,"faces":faces,"limits":{"maxPairProjections":0}})).is_err());
}
