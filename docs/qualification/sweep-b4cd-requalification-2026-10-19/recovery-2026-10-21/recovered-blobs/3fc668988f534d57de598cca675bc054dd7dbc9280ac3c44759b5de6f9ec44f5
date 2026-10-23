use brep_core::face_cut;
use nurbs_core::{curve_surface_agreement::Status, surface::Surface};
#[test]
fn nonlinear_original_uv_section_qualifies_without_sampled_polyline() {
    let source = Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: (0..3)
            .map(|i| {
                [0., 1.]
                    .iter()
                    .map(|&v| vec![i as f64 / 2., v, if i == 2 { 0.25 + v } else { v }])
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 2]; 3],
        periodic_u: false,
        periodic_v: false,
    };
    let plane = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![-1., -1., 0.5], vec![-1., 2., 0.5]],
            vec![vec![2., -1., 0.5], vec![2., 2., 0.5]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let before = (source.clone(), plane.clone());
    let out = face_cut::generate_ruled_plane_cut([&source, &plane], None, 1e-7, 100000).unwrap();
    assert_eq!(out.pcurves[0].degree, 2);
    assert!(
        out.pcurves[0]
            .control_points
            .windows(2)
            .any(|p| p[0][1] != p[1][1])
    );
    assert!(
        out.agreements
            .iter()
            .all(|r| r.status == Status::WithinTolerance)
    );
    assert!(out.cells <= 100000);
    let mut rational = source.clone();
    rational.weights[1] = vec![0.5; 2];
    let rational_before = rational.clone();
    let r = face_cut::generate_ruled_plane_cut([&rational, &plane], None, 1e-7, 100000).unwrap();
    assert!(
        r.pcurves[0]
            .weights
            .iter()
            .any(|w| *w != r.pcurves[0].weights[0])
    );
    assert!(
        r.agreements
            .iter()
            .all(|p| p.status == Status::WithinTolerance)
    );
    assert!(r.cells <= 100000);
    for t in [0., 0.13, 0.5, 0.81, 1.] {
        let p = r.curve.evaluate(t).unwrap().point;
        let w = 1. - t + t * t;
        let x = (0.5 * t + 0.5 * t * t) / w;
        let y = 0.5 - 0.25 * t * t / w;
        for (actual, expected) in p.iter().zip([x, y, 0.5]) {
            assert!((actual - expected).abs() < 1e-10);
        }
    }
    let mut unequal_ruling = rational.clone();
    unequal_ruling.weights[1][1] = 0.75;
    let unequal =
        face_cut::generate_ruled_plane_cut([&unequal_ruling, &plane], None, 1e-7, 100000).unwrap();
    assert!(
        unequal
            .agreements
            .iter()
            .all(|p| p.status == Status::WithinTolerance)
    );
    for t in [0., 0.13, 0.5, 0.81, 1.] {
        let basis = [(1. - t) * (1. - t), 2. * t * (1. - t), t * t];
        let mut signed = [0.; 2];
        for i in 0..3 {
            for j in 0..2 {
                signed[j] += basis[i]
                    * unequal_ruling.weights[i][j]
                    * (unequal_ruling.control_points[i][j][2] - 0.5);
            }
        }
        let v = signed[0] / (signed[0] - signed[1]);
        let mut denominator = 0.;
        let mut numerator = [0.; 2];
        for i in 0..3 {
            for j in 0..2 {
                let w = basis[i] * unequal_ruling.weights[i][j] * if j == 0 { 1. - v } else { v };
                denominator += w;
                for k in 0..2 {
                    numerator[k] += w * unequal_ruling.control_points[i][j][k];
                }
            }
        }
        let p = unequal.curve.evaluate(t).unwrap().point;
        for k in 0..2 {
            assert!((p[k] - numerator[k] / denominator).abs() < 1e-10);
        }
        assert!((p[2] - 0.5).abs() < 1e-10);
    }
    assert_eq!(rational, rational_before);
    let angle = 0.37_f64;
    let transform = |p: &[f64]| {
        vec![
            angle.cos() * p[0] + angle.sin() * p[2] + 2.,
            p[1] - 3.,
            -angle.sin() * p[0] + angle.cos() * p[2] + 1.,
        ]
    };
    let pose = |s: &Surface| {
        let mut out = s.clone();
        for row in &mut out.control_points {
            for p in row {
                *p = transform(p);
            }
        }
        out
    };
    let rotated_source = pose(&unequal_ruling);
    let rotated_plane = pose(&plane);
    let rotated =
        face_cut::generate_ruled_plane_cut([&rotated_source, &rotated_plane], None, 1e-7, 100000)
            .unwrap();
    assert!(
        rotated
            .agreements
            .iter()
            .all(|p| p.status == Status::WithinTolerance)
    );
    for t in [0., 0.13, 0.5, 0.81, 1.] {
        let expected = transform(&unequal.curve.evaluate(t).unwrap().point);
        let actual = rotated.curve.evaluate(t).unwrap().point;
        assert!(
            actual
                .iter()
                .zip(expected)
                .all(|(a, b)| (a - b).abs() < 1e-10)
        );
    }
    let mut reparameterized = rational.clone();
    for t in &mut reparameterized.knots_u {
        *t = 2. + 2. * *t;
    }
    for t in &mut reparameterized.knots_v {
        *t = -1. + 4. * *t;
    }
    let reparam =
        face_cut::generate_ruled_plane_cut([&reparameterized, &plane], None, 1e-7, 100000).unwrap();
    assert!(
        reparam
            .agreements
            .iter()
            .all(|p| p.status == Status::WithinTolerance)
    );
    // The original saved chart may be ruled in U rather than V.
    let transpose = |s: &Surface| Surface {
        degree_u: s.degree_v, degree_v: s.degree_u,
        knots_u: s.knots_v.clone(), knots_v: s.knots_u.clone(),
        control_points: (0..s.control_points[0].len()).map(|j|
            s.control_points.iter().map(|row| row[j].clone()).collect()).collect(),
        weights: (0..s.weights[0].len()).map(|j|
            s.weights.iter().map(|row| row[j]).collect()).collect(),
        periodic_u: false, periodic_v: false,
    };
    let transposed = transpose(&reparameterized);
    let original = transposed.clone();
    let both = face_cut::generate_ruled_plane_cut(
        [&transposed, &plane], None, 1e-7, 100000).unwrap();
    assert!(both.agreements.iter().all(|p| p.status == Status::WithinTolerance));
    for t in [2., 2.26, 3., 3.62, 4.] {
        let at = |c: &nurbs_core::curve::Curve| { let d = c.domain(); d[0]+(d[1]-d[0])*(t-2.)/2. };
        let a = reparam.curve.evaluate(at(&reparam.curve)).unwrap().point;
        let b = both.curve.evaluate(at(&both.curve)).unwrap().point;
        assert!(a.iter().zip(&b).all(|(a,b)| (a-b).abs()<1e-10));
        let uv = both.pcurves[0].evaluate(t).unwrap().point;
        let actual = transposed.evaluate(uv[0], uv[1]).unwrap().point;
        assert!(actual.iter().zip(&b).all(|(a,b)| (a-b).abs()<1e-10));
    }
    // Clip in the ORIGINAL transposed chart and bind actual world endpoints.
    let clipped_ends = [2.5, 3.5].map(|t| {
        let uv = both.pcurves[0].evaluate(t).unwrap().point;
        [uv[0], uv[1]]
    });
    let world_ends = [2.5, 3.5].map(|t| {
        let d = both.curve.domain();
        let p = both.curve.evaluate(d[0]+(d[1]-d[0])*(t-2.)/2.).unwrap().point; [p[0], p[1], p[2]]
    });
    let clip = face_cut::generate_ruled_plane_cut_between(
        [&transposed, &plane], clipped_ends, world_ends, 1e-7, 100000).unwrap();
    assert!(clip.agreements.iter().all(|p| p.status == Status::WithinTolerance));
    assert_eq!(clip.pcurves[0].domain(), [2.5, 3.5]);
    assert_eq!(transposed, original);
    let cubic = Surface {
        degree_u: 3,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        knots_v: source.knots_v.clone(),
        control_points: (0..4)
            .map(|i| {
                [0., 1.]
                    .iter()
                    .map(|&v| vec![i as f64 / 3., v, if i == 3 { 0.25 + v } else { v }])
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 2], vec![0.5; 2], vec![0.75; 2], vec![1.; 2]],
        periodic_u: false,
        periodic_v: false,
    };
    let c = face_cut::generate_ruled_plane_cut([&cubic, &plane], None, 1e-7, 100000).unwrap();
    assert_eq!(c.pcurves[0].degree, 4);
    assert!(
        c.agreements
            .iter()
            .all(|p| p.status == Status::WithinTolerance)
    );
    // Independent closed-form section of z=u²/4+v at z=1/2.
    for t in [0., 0.13, 0.5, 0.81, 1.] {
        let p = out.curve.evaluate(t).unwrap().point;
        for (actual, expected) in p.iter().zip([t, 0.5 - t * t / 4., 0.5]) {
            assert!((actual - expected).abs() < 1e-10);
        }
    }
    assert!(
        face_cut::generate_ruled_plane_cut(
            [&source, &plane],
            Some([[0., 0.5, 0.51], [1., 0.25, 0.5]]),
            1e-7,
            100000
        )
        .is_err()
    );
    let mut varying = source.clone();
    varying.control_points[2][1][2] += 0.25;
    let variable_slope =
        face_cut::generate_ruled_plane_cut([&varying, &plane], None, 1e-7, 100000).unwrap();
    assert!(
        variable_slope
            .agreements
            .iter()
            .all(|p| p.status == Status::WithinTolerance)
    );
    let mut singular = source.clone();
    singular.control_points[2][1][2] = -0.25;
    assert!(face_cut::generate_ruled_plane_cut([&singular, &plane], None, 1e-7, 100000).is_err());
    assert!(face_cut::generate_ruled_plane_cut([&source, &plane], None, 1e-7, 3).is_err());
    assert_eq!((source, plane), before);
}
