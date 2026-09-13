use brep_core::{Model, analysis::mass_properties, boolean, prism_frame::localize};
use std::f64::consts::PI;

fn pose(model: &Model, angles: [f64; 3], delta: [f64; 3], mirror: bool) -> Model {
    let map = |mut p: [f64; 3]| {
        if mirror {
            p[0] = -p[0];
        }
        let (sx, cx) = angles[0].sin_cos();
        let (sy, cy) = angles[1].sin_cos();
        let (sz, cz) = angles[2].sin_cos();
        let p = [p[0], cx * p[1] - sx * p[2], sx * p[1] + cx * p[2]];
        let p = [cy * p[0] + sy * p[2], p[1], -sy * p[0] + cy * p[2]];
        [
            cz * p[0] - sz * p[1] + delta[0],
            sz * p[0] + cz * p[1] + delta[1],
            p[2] + delta[2],
        ]
    };
    let mut result = model.clone();
    for v in &mut result.vertices {
        v.point = map(v.point);
    }
    for e in &mut result.edges {
        for p in &mut e.curve.control_points {
            *p = map([p[0], p[1], p[2]]).to_vec();
        }
    }
    for f in &mut result.faces {
        for p in f.surface.control_points.iter_mut().flatten() {
            *p = map([p[0], p[1], p[2]]).to_vec();
        }
    }
    if mirror {
        for shell in &mut result.shells {
            for usage in &mut shell.faces {
                usage.reversed ^= true;
            }
        }
    }
    result.validate().unwrap();
    result
}

fn sources() -> (Model, Model) {
    (
        brep_core::cylinder(3., 5.).unwrap(),
        pose(
            &brep_core::cylinder(2., 5.).unwrap(),
            [0.; 3],
            [2., 0., 2.],
            false,
        ),
    )
}

fn roundtrip(model: &Model) -> Model {
    let encoded = value_codec::to_string(model).unwrap();
    let decoded: Model = value_codec::from_str(&encoded).unwrap();
    decoded.validate().unwrap();
    assert_eq!(value_codec::to_string(&decoded).unwrap(), encoded);
    decoded
}

fn check(model: &Model, expected: f64) {
    assert_eq!(model.validate().unwrap().boundary_edge_count, 0);
    assert_eq!(model.bodies.len(), 1);
    assert!(model.edges.iter().any(|e| e.curve.degree == 2));
    let mass = mass_properties(model, 1e-7, 600_000).unwrap();
    assert!(mass.signed_volume_mm3 > 0.);
    assert!(
        (mass.signed_volume_mm3 - expected).abs() < expected * 2e-7,
        "Expected {expected}, got {}",
        mass.signed_volume_mm3
    );
}

fn overlap() -> f64 {
    // rA=3, rB=2, distance=2.
    9. * (0.75_f64).acos() + 4. * (-0.125_f64).acos() - 0.5 * 63_f64.sqrt()
}

#[test]
fn rotated_and_reflected_stepped_results_support_roundtrip_and_further_booleans() {
    let (a, b) = sources();
    for (angles, mirror) in [([0.2, 0.7, 0.4], false), ([1.1, -0.4, 2.1], true)] {
        let a = pose(&a, angles, [7., -9., 11.], mirror);
        let b = pose(&b, angles, [7., -9., 11.], mirror);
        let joined = boolean(&a, &b, "union")
            .unwrap_or_else(|e| panic!("First rotated Boolean mirror={mirror}: {e:?}"));
        check(&joined, 65. * PI - 3. * overlap());
        let joined = roundtrip(&joined);
        let (local, _, frame) = localize(&joined, &b)
            .unwrap()
            .expect("Layered result needs a reusable frame");
        let mut levels: Vec<_> = local.vertices.iter().map(|v| v.point[2]).collect();
        levels.sort_by(f64::total_cmp);
        levels.dedup();
        assert_eq!(levels.len(), 4);
        assert!(frame.max_adjustment_mm <= frame.roundoff_bound_mm);
        let restored = frame.restore(&local).unwrap();
        assert_eq!(restored.1.faces, joined.1.faces);
        assert_eq!(restored.1.edges, joined.1.edges);
        assert_eq!(restored.1.lineage, joined.1.lineage);
        let cut = boolean(&joined, &b, "difference")
            .unwrap_or_else(|e| panic!("Layered reuse difference mirror={mirror}: {e:?}"));
        check(&cut, 45. * PI - 3. * overlap());
        let rejoined = boolean(&roundtrip(&cut), &b, "union")
            .unwrap_or_else(|e| panic!("Layered reuse union mirror={mirror}: {e:?}"));
        check(&rejoined, 65. * PI - 3. * overlap());
    }
}

#[test]
fn layered_localization_refuses_authored_control_net_distortion_above_roundoff() {
    let (a, b) = sources();
    let joined = boolean(&a, &b, "union").unwrap();
    let a = pose(&joined, [0.2, 0.7, 0.4], [7., -9., 11.], false);
    let b = pose(&b, [0.2, 0.7, 0.4], [7., -9., 11.], false);
    assert!(localize(&a, &b).unwrap().is_some());
    let mut distorted = a.clone();
    distorted.tolerance_mm = 1e-3;
    let side = distorted
        .faces
        .iter_mut()
        .find(|f| f.surface.degree_u == 2 && f.surface.degree_v == 1)
        .unwrap();
    side.surface.control_points[1][1][0] += 1e-6;
    distorted.validate().unwrap();
    assert!(localize(&distorted, &b).unwrap().is_none());
}
