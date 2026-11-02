//! Literal exact rational pose composition; no sampled transform is authority.
use nurbs_core::{
    affine_volume::{Pose, RationalPiece},
    curve::Curve,
};
fn rotate(p: [f64; 2], i: usize) -> [f64; 2] {
    match i % 4 {
        0 => p,
        1 => [-p[1], p[0]],
        2 => [-p[0], -p[1]],
        _ => [p[1], -p[0]],
    }
}
fn pose(p: [f64; 2]) -> Pose {
    [
        [p[0] * 2f64.powi(-22), p[1] * 2f64.powi(-22), 0.],
        [0., 0., 1.],
        [0.; 3],
        [0.; 3],
    ]
}
pub fn motion() -> (Vec<RationalPiece>, Vec<RationalPiece>, Curve) {
    let scale = 2819201.;
    let xy = [[scale, 0.], [scale, scale], [0., scale]];
    let mapped = [
        [2819201., 0.],
        [2819201., 402743.],
        [2681679., 1168937.],
        [2151495.5, 2003116.5],
        [1402856., 2374064.],
        [961091.25, 2626982.75],
        [659813., 2699235.],
        [402743., 2819201.],
        [0., 2819201.],
    ];
    let reference = (0..4)
        .map(|j| RationalPiece {
            controls: xy.iter().map(|&p| pose(rotate(p, j))).collect(),
            weights: vec![280., 140., 280.],
        })
        .collect();
    let target = (0..4)
        .map(|j| RationalPiece {
            controls: mapped.iter().map(|&p| pose(rotate(p, j))).collect(),
            weights: vec![280., 245., 205., 190., 209., 220., 235., 245., 280.],
        })
        .collect();
    // phi=t+1.5*t^2*(1-t)^2 on each quarter. Refine at 1/2 so
    // every original scalar derivative Bernstein coefficient is strictly positive.
    let mut rows = vec![0., 0.25, 0.75, 0.75, 1.];
    let mut left = vec![rows[0]];
    let mut right = vec![rows[4]];
    while rows.len() > 1 {
        rows = rows.windows(2).map(|v| (v[0] + v[1]) * 0.5).collect();
        left.push(rows[0]);
        right.push(*rows.last().unwrap());
    }
    right.reverse();
    let mut values = Vec::new();
    for j in 0..4 {
        for half in [&left, &right] {
            for (i, &v) in half.iter().enumerate() {
                if !values.is_empty() && i == 0 {
                    continue;
                }
                values.push(vec![(j as f64 + v) * 0.25, 0.]);
            }
        }
    }
    let mut knots = vec![0.; 5];
    for j in 1..8 {
        knots.extend(vec![j as f64 * 0.125; 4]);
    }
    knots.extend(vec![1.; 5]);
    let map = Curve {
        degree: 4,
        knots,
        control_points: values,
        weights: vec![1.; 33],
        periodic: false,
    };
    (reference, target, map)
}
