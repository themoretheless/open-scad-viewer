use nurbs_core::{
    corner_blend::{
        Options,
        mapped::{self, Support},
    },
    curve::Curve,
    surface::Surface,
    surface_blend::Boundary,
};
fn plane() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![1., -1., 0.]],
            vec![vec![1., 1., 0.], vec![2., 0., 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
fn curve(a: [f64; 2], b: [f64; 2]) -> Curve {
    Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![a.to_vec(), b.to_vec()],
        weights: vec![1.; 2],
        periodic: false,
    }
}
fn options() -> Options {
    Options {
        boundaries: [Boundary::VStart; 4],
        reverse_along: [false; 4],
        gains: [2.; 4],
        compatibility_tolerance: 1e-9,
        max_controls: 4096,
    }
}
#[test]
fn original_noniso_traces_construct_an_admitted_patch_with_shared_work() {
    let s = plane();
    let curves = [
        curve([0.25, 0.5], [0.5, 0.25]),
        curve([0.5, 0.75], [0.75, 0.5]),
        curve([0.25, 0.5], [0.5, 0.75]),
        curve([0.5, 0.25], [0.75, 0.5]),
    ];
    let directions = [
        [-0.125, -0.125],
        [0.125, 0.125],
        [-0.125, 0.125],
        [0.125, -0.125],
    ];
    let input = std::array::from_fn(|i| Support {
        surface: &s,
        pcurve: &curves[i],
        direction: directions[i],
    });
    let saved = s.clone();
    let r = mapped::g2(
        input,
        options(),
        [1e-7, 1e-5, 1e-4],
        100000,
        2000000,
        1000000,
    )
    .unwrap();
    assert!(
        r.admitted,
        "cells={} work={} corner={} original={:?}",
        r.cells,
        r.coefficient_work,
        r.corner.admitted,
        r.original_seams
            .iter()
            .map(|x| x.decision)
            .collect::<Vec<_>>()
    );
    assert!(r.cells <= 100000 && r.coefficient_work <= 2000000);
    assert_eq!(r.original_seams.len(), 4);
    assert!(r.direct_admitted && r.direct_upper.is_some());
    assert_eq!(r.direct_original_seams.len(), 4);
    let just = mapped::g2(
        input,
        options(),
        [1e-7, 1e-5, 1e-4],
        r.cells,
        2000000,
        1000000,
    )
    .unwrap();
    assert!(just.admitted && just.direct_admitted);
    assert!(just.cells <= r.cells);
    let short = mapped::g2(
        input,
        options(),
        [1e-7, 1e-5, 1e-4],
        r.cells - 1,
        2000000,
        1000000,
    )
    .unwrap();
    assert!(!short.admitted && short.upper.is_none());
    for u in [0., 0.23, 0.71, 1.] {
        for v in [0., 0.37, 0.82, 1.] {
            let p = r.corner.surface.evaluate(u, v).unwrap().point;
            assert!((p[0] - (0.75 + 0.5 * u)).abs() < 1e-10);
            assert!((p[1] - (-0.25 + 0.5 * v)).abs() < 1e-10);
            assert!(p[2].abs() < 1e-12);
        }
    }
    assert_eq!(s, saved);
    assert!(mapped::g2(input, options(), [1e-7, 1e-5, 1e-4], 100000, 1, 1000000).is_err());
}

#[test]
fn curved_original_pcurves_feed_corner_construction_without_straightening_uv() {
    let sources: [Surface; 4] = std::array::from_fn(|side| {
        let mut s = plane();
        s.degree_u = 2;
        s.knots_u = vec![0., 0., 0., 1., 1., 1.];
        s.weights = vec![vec![1.; 2]; 3];
        s.control_points = (0..3)
            .map(|i| {
                (0..2)
                    .map(|j| {
                        let along = i as f64 / 2.;
                        let cross = j as f64 + if i == 2 { 0.25 } else { 0. } - 0.5;
                        match side {
                            0 => vec![0.75 - cross, along - 0.5, 0.],
                            1 => vec![1.25 + cross, along - 0.5, 0.],
                            2 => vec![along + 0.5, -0.25 - cross, 0.],
                            _ => vec![along + 0.5, 0.25 + cross, 0.],
                        }
                    })
                    .collect()
            })
            .collect();
        s
    });
    let pc = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![
            vec![0.25, 0.484375],
            vec![0.5, 0.453125],
            vec![0.75, 0.359375],
        ],
        weights: vec![1.; 3],
        periodic: false,
    };
    let saved = pc.clone();
    let input = std::array::from_fn(|i| Support {
        surface: &sources[i],
        pcurve: &pc,
        direction: [0., 0.125],
    });
    let mut o = options();
    o.gains = [4.; 4];
    let r = mapped::g2(input, o, [1e-7, 1e-5, 1e-4], 100000, 2000000, 1000000).unwrap();
    assert!(
        r.admitted,
        "cells={} work={} corner={} original={:?}",
        r.cells,
        r.coefficient_work,
        r.corner.admitted,
        r.original_seams
            .iter()
            .map(|x| x.decision)
            .collect::<Vec<_>>()
    );
    assert!(r.direct_admitted && r.direct_upper.is_some());
    assert_eq!(r.direct_original_seams.len(), 4);
    assert_eq!(pc, saved);
    let scarce_cells = mapped::g2(input, o, [1e-7, 1e-5, 1e-4], 8, 2000000, 1000000).unwrap();
    assert!(!scarce_cells.admitted && scarce_cells.upper.is_none());
    assert!(scarce_cells.cells <= 8);
    if let Ok(scarce_work) = mapped::g2(input, o, [1e-7, 1e-5, 1e-4], 100000, 100000, 1000000) {
        assert!(!scarce_work.admitted && scarce_work.upper.is_none());
        assert!(scarce_work.coefficient_work <= 100000);
    }
    println!(
        "curved mapped corner: cells={} coefficients={} exact={} upper={:?}",
        r.cells, r.coefficient_work, r.corner.exact_work, r.upper
    );
    for u in [0., 0.23, 0.71, 1.] {
        for v in [0., 0.37, 0.82, 1.] {
            let p = r.corner.surface.evaluate(u, v).unwrap().point;
            assert!((p[0] - (0.75 + 0.5 * u)).abs() < 1e-10);
            assert!((p[1] - (-0.25 + 0.5 * v)).abs() < 1e-10);
            assert!(p[2].abs() < 1e-12);
        }
    }
    assert!(
        r.upper
            .unwrap()
            .iter()
            .all(|b| b[0] <= 1e-7 && b[1] <= 1e-5 && b[2] <= 1e-4)
    );
    for t in [0., 0.2, 0.7, 1.] {
        let uv = pc.evaluate(t).unwrap().point;
        assert!((uv[1] - (0.5 - 0.25 * uv[0] * uv[0])).abs() < 1e-14);
    }
    let mut damaged = pc.clone();
    damaged.control_points[1][1] += 0.01;
    let incompatible = std::array::from_fn(|i| Support {
        surface: &sources[i],
        pcurve: if i == 0 { &damaged } else { &pc },
        direction: [0., 0.125],
    });
    assert!(
        mapped::g2(
            incompatible,
            o,
            [1e-7, 1e-5, 1e-4],
            100000,
            2000000,
            1000000
        )
        .map_or(true, |report| !report.admitted)
    );
}

#[test]
fn spatial_paraboloid_corner_is_proved_against_original_noniso_supports() {
    let mut source = plane();
    source.degree_u = 2;
    source.degree_v = 2;
    source.knots_u = vec![0., 0., 0., 1., 1., 1.];
    source.knots_v = source.knots_u.clone();
    source.weights = vec![vec![1.; 3]; 3];
    let square = [0., 0., 1.];
    source.control_points = (0..3)
        .map(|i| {
            (0..3)
                .map(|j| {
                    let u = i as f64 / 2.;
                    let v = j as f64 / 2.;
                    vec![
                        u + v,
                        u - v,
                        0.3 * square[i] - 0.2 * u * v + 0.3 * square[j],
                    ]
                })
                .collect()
        })
        .collect();
    let curves = [
        curve([0.25, 0.5], [0.5, 0.25]),
        curve([0.5, 0.75], [0.75, 0.5]),
        curve([0.25, 0.5], [0.5, 0.75]),
        curve([0.5, 0.25], [0.75, 0.5]),
    ];
    let directions = [
        [-0.125, -0.125],
        [0.125, 0.125],
        [-0.125, 0.125],
        [0.125, -0.125],
    ];
    let input = std::array::from_fn(|i| Support {
        surface: &source,
        pcurve: &curves[i],
        direction: directions[i],
    });
    let saved = source.clone();
    let r = mapped::g2(
        input,
        options(),
        [1e-7, 1e-5, 1e-4],
        100000,
        2000000,
        1000000,
    )
    .unwrap();
    println!(
        "spatial corner: admitted={} cells={} work={} original={:?}",
        r.admitted,
        r.cells,
        r.coefficient_work,
        r.original_seams
            .iter()
            .map(|s| s.decision)
            .collect::<Vec<_>>()
    );
    assert!(r.admitted && r.direct_admitted);
    let fresh = mapped::inspect_candidate(
        &r.corner.surface,
        input,
        options(),
        [1e-7, 1e-5, 1e-4],
        100000,
        2000000,
        1000000,
    )
    .unwrap();
    assert!(fresh.admitted && fresh.direct_admitted);
    let mut relabeled = r.corner.surface.clone();
    for k in &mut relabeled.knots_u {
        *k = 2. + 2. * *k;
    }
    for k in &mut relabeled.knots_v {
        *k = -3. + 4. * *k;
    }
    let nonunit = mapped::inspect_candidate(
        &relabeled,
        input,
        options(),
        [1e-7, 1e-5, 1e-4],
        100000,
        2000000,
        1000000,
    )
    .unwrap();
    assert!(nonunit.admitted && nonunit.direct_admitted);
    assert_eq!(nonunit.corner.surface, relabeled);
    let mut damaged = r.corner.surface.clone();
    let middle = damaged.control_points[0].len() / 2;
    damaged.control_points[2][middle][2] += 0.01;
    // Boundary positions and first transverse jets are unchanged at U=0;
    // admission must still inspect the altered second jet.
    let refused = mapped::inspect_candidate(
        &damaged,
        input,
        options(),
        [1e-7, 1e-5, 1e-4],
        100000,
        2000000,
        1000000,
    )
    .unwrap();
    assert!(!refused.admitted && refused.upper.is_none());
    assert!(refused.cells <= 100000 && refused.coefficient_work <= 2000000);
    for u in [0., 0.13, 0.44, 0.81, 1.] {
        for v in [0., 0.21, 0.63, 0.92, 1.] {
            let p = r.corner.surface.evaluate(u, v).unwrap().point;
            let x = 0.75 + 0.5 * u;
            let y = -0.25 + 0.5 * v;
            assert!((p[0] - x).abs() < 1e-10 && (p[1] - y).abs() < 1e-10);
            assert!((p[2] - (0.1 * x * x + 0.2 * y * y)).abs() < 1e-10);
        }
    }
    assert_eq!(source, saved);
}

#[test]
fn original_reversed_traces_and_nonunit_candidate_labels_keep_direct_side_frames() {
    let source = plane();
    let curves = [
        curve([0.25, 0.5], [0.5, 0.25]),
        curve([0.5, 0.75], [0.75, 0.5]),
        curve([0.25, 0.5], [0.5, 0.75]),
        curve([0.5, 0.25], [0.75, 0.5]),
    ];
    let directions = [
        [-0.125, -0.125],
        [0.125, 0.125],
        [-0.125, 0.125],
        [0.125, -0.125],
    ];
    let supports = std::array::from_fn(|i| Support {
        surface: &source,
        pcurve: &curves[i],
        direction: directions[i],
    });
    let baseline = mapped::g2(
        supports,
        options(),
        [1e-7, 1e-5, 1e-4],
        100000,
        2000000,
        1000000,
    )
    .unwrap();
    assert!(baseline.direct_admitted);
    for side in 0..4 {
        let mut reversed = curves.clone();
        reversed[side] = reversed[side].reverse().unwrap();
        let mut o = options();
        o.reverse_along[side] = true;
        let input = std::array::from_fn(|i| Support {
            surface: &source,
            pcurve: &reversed[i],
            direction: directions[i],
        });
        let r = mapped::g2(input, o, [1e-7, 1e-5, 1e-4], 100000, 2000000, 1000000).unwrap();
        assert!(r.admitted && r.direct_admitted);
        assert_eq!(r.orientations, baseline.orientations);
        for (u, v) in [(0., 0.), (0.13, 0.37), (0.69, 0.83), (1., 1.)] {
            let a = r.corner.surface.evaluate(u, v).unwrap().point;
            let b = baseline.corner.surface.evaluate(u, v).unwrap().point;
            assert!(a.iter().zip(b).all(|(a, b)| (a - b).abs() < 1e-10));
        }
    }
    let mut relabeled = baseline.corner.surface.clone();
    for k in &mut relabeled.knots_u {
        *k = 2. + 2. * *k;
    }
    for k in &mut relabeled.knots_v {
        *k = -3. + 4. * *k;
    }
    let checked = mapped::inspect_candidate(
        &relabeled,
        supports,
        options(),
        [1e-7, 1e-5, 1e-4],
        100000,
        2000000,
        1000000,
    )
    .unwrap();
    assert!(checked.admitted && checked.direct_admitted);
    assert_eq!(checked.corner.surface, relabeled);
}
