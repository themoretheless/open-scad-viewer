use super::*;

fn bezier_surface(points: [[ [f64; 3]; 4 ]; 4]) -> Surface {
    Surface {
        degree_u: 3,
        degree_v: 3,
        knots_u: [vec![0.; 4], vec![2.; 4]].concat(),
        knots_v: [vec![0.; 4], vec![2.; 4]].concat(),
        control_points: points.iter().map(|r| r.iter().map(|p| p.to_vec()).collect()).collect(),
        weights: vec![vec![1.; 4]; 4],
        periodic_u: false,
        periodic_v: false,
    }
}

fn bump() -> Surface {
    let net = std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            let x = 2. * i as f64 / 3.;
            let y = 2. * j as f64 / 3.;
            [x, y, 0.4 * x * y * (2. - x) * (2. - y) + 0.1 * x]
        })
    });
    bezier_surface(net)
}

fn quadrant(s: &Surface, u: [f64; 2], v: [f64; 2]) -> Surface {
    let s = s
        .edit_axis(crate::surface::Axis::U, |c| c.trim(u[0], u[1]))
        .unwrap()
        .edit_axis(crate::surface::Axis::V, |c| c.trim(v[0], v[1]))
        .unwrap();
    normalized_axis(&normalized_axis(&s, true).unwrap(), false).unwrap()
}

/// Fan of four quadrants of a smooth bump around its center (1, 1).
fn fan() -> Vec<VertexPatch> {
    let s = bump();
    vec![
        VertexPatch {
            surface: quadrant(&s, [0., 1.], [0., 1.]),
            corner: Corner::MaxMax,
            outgoing: Boundary::UMax,
        },
        VertexPatch {
            surface: quadrant(&s, [1., 2.], [0., 1.]),
            corner: Corner::MinMax,
            outgoing: Boundary::VMax,
        },
        VertexPatch {
            surface: quadrant(&s, [1., 2.], [1., 2.]),
            corner: Corner::MinMin,
            outgoing: Boundary::UMin,
        },
        VertexPatch {
            surface: quadrant(&s, [0., 1.], [1., 2.]),
            corner: Corner::MaxMin,
            outgoing: Boundary::VMin,
        },
    ]
}

fn seam_normals_fan(a: &Surface, ea: Boundary, b: &Surface, eb: Boundary, samples: usize) -> f64 {
    let mut worst = 0_f64;
    for i in 0..=samples {
        let t = i as f64 / samples as f64;
        let pa = match ea {
            Boundary::UMin => a.evaluate(0., t).unwrap(),
            Boundary::UMax => a.evaluate(1., t).unwrap(),
            Boundary::VMin => a.evaluate(t, 0.).unwrap(),
            Boundary::VMax => a.evaluate(t, 1.).unwrap(),
        };
        let pb = match eb {
            Boundary::UMin => b.evaluate(0., t).unwrap(),
            Boundary::UMax => b.evaluate(1., t).unwrap(),
            Boundary::VMin => b.evaluate(t, 0.).unwrap(),
            Boundary::VMax => b.evaluate(t, 1.).unwrap(),
        };
        let na = pa.unit_normal().unwrap();
        let nb = pb.unit_normal().unwrap();
        worst = f64::max(worst, norm(cross(na, nb)));
    }
    worst
}

#[test]
fn smooth_fan_analyzes_compatible_and_join_is_near_identity() {
    let f = fan();
    let report = analyze(&f, 1e-9, 1e-9).unwrap();
    assert_eq!(report.valence, 4);
    assert_eq!(report.valence_class, ValenceClass::Even);
    assert!(
        report.conflicts.is_empty(),
        "unexpected conflicts: {:?}",
        report.conflicts
    );
    assert!(report.twist_compatible);
    let joined = join_fan(&f, 1e-9, 4).unwrap();
    assert!(joined.max_shift < 1e-6, "shift {}", joined.max_shift);
    assert!(joined.max_residual < 1e-6, "residual {}", joined.max_residual);
}

#[test]
fn perturbed_twist_is_diagnosed_and_join_restores_g1() {
    let mut f = fan();
    // Break the away-away twist at the shared corner of patch 0 (MaxMax):
    // the diagonally adjacent interior control point only affects duv.
    // The x shift keeps the defect visible: the seam osculating plane at
    // the vertex contains z, so a z twist jump is absorbable by beta(0).
    f[0].surface.control_points[2][2][0] += 0.3;
    let report = analyze(&f, 1e-9, 1e-6).unwrap();
    assert!(!report.twist_compatible);
    assert!(
        report
            .conflicts
            .iter()
            .any(|c| c.kind == ConflictKind::TwistMismatch),
        "expected a twist conflict: {:?}",
        report.conflicts
    );
    let joined = join_fan(&f, 1e-9, 4).unwrap();
    assert!(
        joined.max_residual < 1e-6,
        "residual {}",
        joined.max_residual
    );
    // The join may move interior poles, but both authored G0 boundaries
    // must remain identical after harmonization.
    let original = harmonize(&f).unwrap();
    for (index, patch) in f.iter().enumerate() {
        for boundary in [patch.outgoing, incoming(patch)] {
            assert_eq!(
                boundary_curve(&original[index], boundary).control_points,
                boundary_curve(&joined.surfaces[index], boundary).control_points,
            );
        }
    }
    // Verify normal agreement along every seam of the joined fan.
    for seam in 0..4 {
        let next = (seam + 1) % 4;
        let worst = seam_normals_fan(
            &joined.surfaces[seam],
            f[seam].outgoing,
            &joined.surfaces[next],
            incoming(&f[next]),
            16,
        );
        assert!(worst < 1e-4, "seam {seam} normal jump {worst}");
    }
}

#[test]
fn polynomial_join_accepts_uniform_weight_scales_and_refuses_near_uniform_weights() {
    let mut f = fan();
    for (index, patch) in f.iter_mut().enumerate() {
        for row in &mut patch.surface.weights {
            row.fill((index + 1) as f64);
        }
    }
    let joined = join_fan(&f, 1e-9, 4).unwrap();
    assert!(joined.max_shift < 1e-6);
    assert!(joined.max_residual < 1e-6);
    // Even a small positive weight defect invalidates the polynomial
    // derivative rows; a modeling tolerance cannot make them rational.
    f[0].surface.weights[2][2] += 1e-13;
    assert!(join_fan(&f, 1e-9, 4).is_err());
}

#[test]
fn cube_corner_reports_noncoplanar_tangents_and_odd_valence() {
    // Three unit-square faces of a cube around the origin.
    let face = |rows: [[f64; 3]; 4]| Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: rows
            .chunks(2)
            .map(|c| c.iter().map(|p| p.to_vec()).collect())
            .collect(),
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let xy = face([[0., 0., 0.], [0., 1., 0.], [1., 0., 0.], [1., 1., 0.]]);
    let yz = face([[0., 0., 0.], [0., 0., 1.], [0., 1., 0.], [0., 1., 1.]]);
    let zx = face([[0., 0., 0.], [1., 0., 0.], [0., 0., 1.], [1., 0., 1.]]);
    let fan = vec![
        VertexPatch {
            surface: xy,
            corner: Corner::MinMin,
            outgoing: Boundary::UMin,
        },
        VertexPatch {
            surface: yz,
            corner: Corner::MinMin,
            outgoing: Boundary::UMin,
        },
        VertexPatch {
            surface: zx,
            corner: Corner::MinMin,
            outgoing: Boundary::VMin,
        },
    ];
    let report = analyze(&fan, 1e-9, 1e-9).unwrap();
    assert_eq!(report.valence_class, ValenceClass::Odd);
    assert!(
        report
            .conflicts
            .iter()
            .any(|c| c.kind == ConflictKind::TangentMismatch),
        "cube corner must fail tangent coplanarity: {:?}",
        report.conflicts
    );
}

#[test]
fn position_gap_is_reported() {
    let mut f = fan();
    f[2].surface.control_points[0][0][2] += 0.05;
    let report = analyze(&f, 1e-6, 1e-6).unwrap();
    assert!(
        report
            .conflicts
            .iter()
            .any(|c| c.kind == ConflictKind::PositionGap)
    );
}
