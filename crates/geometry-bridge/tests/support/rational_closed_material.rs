use brep_core::LoftCap;
use nurbs_core::{affine_volume::RationalPiece, curve::Curve, surface::Surface};
use value_codec::{Value, json};
fn fixture(nonplanar: bool) -> (Vec<Vec<Curve>>, LoftCap, Vec<RationalPiece>) {
    let corners = [
        vec![
            [5., -1.],
            [5., -0.75],
            [5., -0.5],
            [5., 0.],
            [5., 0.5],
            [5., 0.75],
            [5., 1.],
        ],
        vec![
            [5., 1.],
            [5., 1.25],
            [5., 1.5],
            [4.75, 1.75],
            [4.5, 2.],
            [4.25, 2.],
            [4., 2.],
        ],
        vec![
            [4., 2.],
            [3.75, 2.],
            [3.5, 2.],
            [3., if nonplanar { 2.0625 } else { 2. }],
            [2.5, 2.],
            [2.25, 2.],
            [2., 2.],
        ],
        vec![
            [2., 2.],
            [1.75, 2.],
            [1.5, 2.],
            [1.25, 1.75],
            [1., 1.5],
            [1., 1.25],
            [1., 1.],
        ],
        vec![
            [1., 1.],
            [1., 0.75],
            [1., 0.5],
            [1., 0.],
            [1., -0.5],
            [1., -0.75],
            [1., -1.],
        ],
        vec![
            [1., -1.],
            [1., -1.25],
            [1., -1.5],
            [1.25, -1.75],
            [1.5, -2.],
            [1.75, -2.],
            [2., -2.],
        ],
        vec![
            [2., -2.],
            [2.25, -2.],
            [2.5, -2.],
            [3., -2.],
            [3.5, -2.],
            [3.75, -2.],
            [4., -2.],
        ],
        vec![
            [4., -2.],
            [4.25, -2.],
            [4.5, -2.],
            [4.75, -1.75],
            [5., -1.5],
            [5., -1.25],
            [5., -1.],
        ],
    ];
    let ring = corners
        .iter()
        .map(|p| Curve {
            degree: 6,
            knots: [vec![0.; 7], vec![1.; 7]].concat(),
            control_points: p.iter().map(|p| vec![p[0], p[1], 0.]).collect(),
            weights: vec![1.; 7],
            periodic: false,
        })
        .collect::<Vec<_>>();
    let trims = ring
        .iter()
        .map(|c| {
            let mut c = c.clone();
            for p in &mut c.control_points {
                p.pop();
            }
            c
        })
        .collect();
    let u = [0.75, 5.25];
    let v = [-2.5, 2.5];
    let surface = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![u[0], u[0], u[1], u[1]],
        knots_v: vec![v[0], v[0], v[1], v[1]],
        control_points: u
            .iter()
            .map(|&u| v.iter().map(|&v| vec![u, v, 0.]).collect())
            .collect(),
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let az = [
        [1., 0.],
        [1., 1.],
        [0., 1.],
        [-1., 1.],
        [-1., 0.],
        [-1., -1.],
        [0., -1.],
        [1., -1.],
        [1., 0.],
    ];
    let motion = (0..4)
        .map(|i| RationalPiece {
            weights: vec![1., 0.5, 1.],
            controls: az[2 * i..=2 * i + 2]
                .iter()
                .map(|p| [[p[0], p[1], 0.], [0., 0., 1.], [0.; 3], [0.; 3]])
                .collect(),
        })
        .collect();
    (
        vec![ring],
        LoftCap {
            surface,
            trims: vec![trims],
        },
        motion,
    )
}

pub fn request(nonplanar: bool, affine: bool) -> Value {
    let (profiles, material, mut motion) = fixture(nonplanar);
    if affine {
        let basis = [[1., 0.125, 0.25], [0., 1., 0.125], [0.125, 0., 1.]];
        for piece in &mut motion {
            for p in &mut piece.controls {
                p[1] = [p[0][0] / 8., p[0][1] / 8., 1.];
                for col in 0..3 {
                    let q = p[col];
                    p[col] = std::array::from_fn(|k| {
                        basis[0][k] * q[0] + basis[1][k] * q[1] + basis[2][k] * q[2]
                    });
                }
                p[3] = [1., 2., 3.];
            }
        }
        for w in &mut motion[1].weights {
            *w *= 2.;
        }
    }
    json!({"op":"brep_rational_closed_material_construct",
        "source":{"profiles":profiles,"origin":[0.,0.,0.],"motion":motion.iter().map(|p|json!({"controls":p.controls,"weights":p.weights})).collect::<Vec<_>>()},
        "material":{"surface":material.surface,"trims":material.trims},"capProfiles":[[0,2],[0,6]],"toleranceUV":1e-8,
        "limits":{"sourceCells":100000,"sourceExactWork":1000000,"constructionRows":1000000,"maxFaces":1024,"maxBindingWork":1000000,"maxOrientationCells":100000,"maxCapCells":100000,
            "material":{"maxPairs":100000,"maxCells":100000,"maxDomainCells":1000000},
            "volume":{"projection":[[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]],"maxCells":1000000,"maxExactWork":1000000},
            "smoothness":{"order":2,"maxExactWork":16000000,"maxAgreementWork":1000000,"maxRegularityCells":100000}}})
}

pub fn full(r: &Value, stage: &str) {
    for key in [
        "certified",
        "solidGeometryCertified",
        "continuousBound",
        "topologyPreserved",
        "shellRolesCertified",
        "fullBoundaryG1Certified",
        "fullBoundaryG2Certified",
        "idealSourceBoundaryG1Certified",
        "idealSourceBoundaryG2Certified",
        "physicalCapsCertified",
    ] {
        assert_eq!(
            r[key], true,
            "{stage}/{key}: reason={} binding={} smoothness={}",
            r["reason"], r["retainedBinding"], r["smoothness"]
        );
    }
    assert_eq!(r["physicalCapCount"], 2);
    assert_eq!(r["boundaryErrorUpper"], 0.);
    assert_eq!(r["filledCapErrorUpper"], 0.);
    assert_eq!(r["seamContinuity"], "G2");
    for key in [
        "embeddingCertified",
        "volumeRegularityCertified",
        "parameterTopologyPreserved",
    ] {
        assert_eq!(r["originalVolume"][key], true, "{stage}/{key}");
    }
    for cap in r["caps"].as_array().unwrap() {
        assert_eq!(cap["faces"].as_array().unwrap().len(), 4);
        assert_eq!(cap["parameterMonotonicityCertified"], true);
    }
}
