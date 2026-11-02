#![cfg(feature = "codec")]
use brep_core::{
    feature_ops::{
        full_round_curved::admitted::{recipe, transport},
        full_round_exact,
    },
    Model,
};
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
fn frozen_recipe_and_binary_json_transport_reconstruct_identical_rational_geometry() {
    let (m, faces) = source(0.7);
    let initial = full_round_exact(&m, faces, Default::default()).unwrap();
    let rebuilt = recipe::restore(&m, faces, initial.recipe(), Default::default()).unwrap();
    assert_eq!(rebuilt.model(), initial.model());
    let artifact = transport::export(&initial, 4 << 20).unwrap();
    if let Ok(directory) = std::env::var("FULL_ROUND_FIXTURE_DIR") {
        let directory = std::path::Path::new(&directory);
        std::fs::create_dir_all(directory).unwrap();
        let temporary = directory.join("artifact.tmp");
        std::fs::write(&temporary, value_codec::to_string(&artifact).unwrap()).unwrap();
        std::fs::rename(temporary, directory.join("artifact.json")).unwrap();
    }
    assert_eq!(
        artifact["geometry"]["format"].as_str(),
        Some("exact-rational-nurbs-brep/1")
    );
    assert!(artifact["geometry"]["vertices"][0]["point"][0][0].is_string());
    let binary = value_codec::encode_binary(&artifact).unwrap();
    let binary: Value = value_codec::decode_binary(&binary).unwrap();
    let body = transport::restore(&binary, 4 << 20).unwrap();
    assert_eq!(body.model(), initial.model());
    let text = value_codec::to_string(&artifact).unwrap();
    let json: Value = value_codec::from_str(&text).unwrap();
    let body = transport::restore(&json, 4 << 20).unwrap();
    assert_eq!(body.model(), initial.model());
    assert_eq!(body.source(), &m);
    body.reverify().unwrap();
}
#[test]
fn supplied_geometry_success_bits_and_invalid_recipe_cannot_authorize_admission() {
    let (m, faces) = source(1.);
    let initial = full_round_exact(&m, faces, Default::default()).unwrap();
    let artifact = transport::export(&initial, 4 << 20).unwrap();
    let mut changed = artifact.clone();
    changed["geometry"]["vertices"][0]["point"][0][0] = json!("999");
    assert!(transport::restore(&changed, 4 << 20).is_err());
    let mut changed = artifact.clone();
    changed["evidence"]["exactG1"] = json!(false);
    assert!(transport::restore(&changed, 4 << 20).is_err());
    let mut changed = artifact.clone();
    changed["recipe"]["radius"] = json!(["1", "100"]);
    assert!(transport::restore(&changed, 4 << 20).is_err());
    let mut changed = artifact.clone();
    changed["trusted"] = json!(true);
    assert!(transport::restore(&changed, 4 << 20).is_err());
    assert!(transport::restore(&artifact, 1).is_err());
    let mut changed = initial.recipe().clone();
    changed.contact_ranges[0][0] = ["1".into(), "4".into()];
    assert!(recipe::restore(&m, faces, &changed, Default::default()).is_err());
}
