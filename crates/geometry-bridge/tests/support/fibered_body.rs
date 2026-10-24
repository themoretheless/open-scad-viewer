#![allow(dead_code)]
use brep_core::{FiberedBodyLimits, LoftCap};
use nurbs_core::{affine_volume::RationalPiece, curve::Curve, fibered_volume, surface::Surface};
use value_codec::{Value, json};
fn ring(radius: f64) -> Vec<Curve> {
    (0..4)
        .map(|quadrant| {
            let xy = [[radius, 0.], [radius, radius], [0., radius]];
            Curve {
                degree: 2,
                knots: vec![0., 0., 0., 1., 1., 1.],
                control_points: xy
                    .into_iter()
                    .map(|[x, y]| {
                        let (x, y) = match quadrant {
                            0 => (x, y),
                            1 => (-y, x),
                            2 => (-x, -y),
                            _ => (y, -x),
                        };
                        vec![x, y, 0.]
                    })
                    .collect(),
                weights: vec![1., 1., 2.],
                periodic: false,
            }
        })
        .collect()
}
fn material(hole: bool) -> (Vec<Vec<Curve>>, LoftCap) {
    let mut profiles = vec![ring(2.)];
    if hole {
        profiles.push(
            ring(1.)
                .into_iter()
                .rev()
                .map(|c| c.reverse().unwrap())
                .collect(),
        );
    }
    let trims = profiles
        .iter()
        .map(|r| {
            r.iter()
                .map(|c| {
                    let mut c = c.clone();
                    for p in &mut c.control_points {
                        p.pop();
                    }
                    c
                })
                .collect()
        })
        .collect();
    let q = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![-2., -2., 2., 2.],
        knots_v: vec![-2., -2., 2., 2.],
        control_points: vec![
            vec![vec![-2., -2., 0.], vec![-2., 2., 0.]],
            vec![vec![2., -2., 0.], vec![2., 2., 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    (profiles, LoftCap { surface: q, trims })
}
fn motion() -> Vec<RationalPiece> {
    vec![RationalPiece {
        controls: [1., 1.25, 1.5, 1.5, 1.25, 1.]
            .into_iter()
            .zip([0., 0., 0., 1., 1., 1.])
            .map(|(a, z)| [[a, 0., 0.], [0., a, 0.], [0.; 3], [0., 0., z]])
            .collect(),
        weights: vec![1.; 6],
    }]
}
fn proposal() -> fibered_volume::Proposal {
    fibered_volume::Proposal {
        source_normal: [0., 0., 1.],
        world_rows: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
    }
}
fn limits() -> FiberedBodyLimits {
    FiberedBodyLimits {
        volume: fibered_volume::Limits {
            cells: 100000,
            exact_work: 1000000,
        },
        material_pairs: 100000,
        material_cells: 100000,
        material_domain_cells: 1000000,
        profile_cells: 100000,
        source_trace_work: 1000000,
        binding_rows: 1000000,
        agreement_work: 1000000,
        regularity_cells: 100000,
        normal_cells: 100000,
        smoothness_work: 1000000,
        order: 2,
    }
}

pub fn request() -> Value {
    let (profiles, m) = material(false);
    let motion = motion();
    let q = proposal();
    let l = limits();
    json!({"op":"brep_fibered_source_construct",
        "source":{"profiles":profiles,"origin":[0.,0.,0.],"motion":motion.iter().map(|p|json!({"controls":p.controls,"weights":p.weights})).collect::<Vec<_>>()},
        "material":{"surface":m.surface,"trims":m.trims},
        "proposal":{"sourceNormal":q.source_normal,"worldRows":q.world_rows},"toleranceUV":1e-8,
        "limits":{"volume":{"cells":l.volume.cells,"exactWork":l.volume.exact_work},
            "materialPairs":l.material_pairs,"materialCells":l.material_cells,"materialDomainCells":l.material_domain_cells,
            "profileCells":l.profile_cells,"sourceTraceWork":l.source_trace_work,"bindingRows":l.binding_rows,
            "agreementWork":l.agreement_work,"regularityCells":l.regularity_cells,"normalCells":l.normal_cells,
            "smoothnessWork":l.smoothness_work,"order":l.order}})
}
pub fn full(r: &Value) {
    for key in [
        "certified",
        "solidGeometryCertified",
        "sourceBoundaryExact",
        "continuousBound",
        "topologyPreserved",
        "absenceOfSelfIntersections",
        "fullBoundaryG1Certified",
        "fullBoundaryG2Certified",
        "idealSourceBoundaryG1Certified",
        "idealSourceBoundaryG2Certified",
        "actualBoundaryRegular",
        "exactBoundaryAgreement",
        "vertexLinksComplete",
    ] {
        assert_eq!(r[key], true, "{key}: {r}");
    }
    assert_eq!(r["boundaryErrorUpper"], 0.);
    assert_eq!(r["seamContinuity"], "G2");
    assert_eq!(
        r["originalVolume"]["knownSingularParameterEnds"],
        json!([true, true])
    );
    assert_eq!(r["originalVolume"]["spanwiseInteriorRegular"], true);
    assert!(!r["model"].is_null());
}

fn three_pieces() -> Vec<RationalPiece> {
    let scales = [
        [1., 1.25, 1.5, 1.5, 1.5, 1.5],
        [1.5; 6],
        [1.5, 1.5, 1.5, 1.5, 1.25, 1.],
    ];
    let heights = [
        [0., 0., 0., 0.5, 0.75, 1.],
        [1., 1.25, 1.5, 1.5, 1.75, 2.],
        [2., 2.25, 2.5, 3., 3., 3.],
    ];
    scales
        .into_iter()
        .zip(heights)
        .map(|(a, h)| RationalPiece {
            controls: a
                .into_iter()
                .zip(h)
                .map(|(a, z)| [[a, 0., 0.], [0., a, 0.], [0.; 3], [0., 0., z]])
                .collect(),
            weights: vec![1.; 6],
        })
        .collect()
}

pub fn three_piece_request() -> Value {
    let mut v = request();
    v["source"]["motion"] = json!(
        three_pieces()
            .iter()
            .map(|p| json!({"controls":p.controls,"weights":p.weights}))
            .collect::<Vec<_>>()
    );
    v
}
pub fn oblique_request(descending: bool) -> Value {
    let mut v = request();
    let mut m = motion();
    m[0].weights = vec![1., 0.3, 2., 0.7, 3., 2.];
    for (pose, b) in m[0].controls.iter_mut().zip([0., 0.25, 0.5, 0.5, 0.25, 0.]) {
        pose[0][2] = pose[0][0];
        pose[3][0] = b;
        pose[3][2] += b;
        if descending {
            pose[3][0] = -pose[3][0];
            pose[3][2] = -pose[3][2];
        }
    }
    v["source"]["motion"] = json!(
        m.iter()
            .map(|p| json!({"controls":p.controls,"weights":p.weights}))
            .collect::<Vec<_>>()
    );
    v["proposal"] =
        json!({"sourceNormal":[0.,0.,0.7],"worldRows":[[1.,0.,0.],[0.,1.,0.],[-0.3,0.,0.3]]});
    v
}

pub fn hole_request(order: usize) -> Value {
    let mut v = request();
    let (p, m) = material(true);
    v["source"]["profiles"] = json!(p);
    v["material"] = json!({"surface":m.surface,"trims":m.trims});
    v["limits"]["order"] = json!(order);
    v
}
