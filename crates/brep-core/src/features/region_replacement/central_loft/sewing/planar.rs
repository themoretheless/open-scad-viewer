//! Proposals on the actual axis-aligned affine support; fresh G0 is mandatory.
use super::*;
pub(super) fn frame(s: &Surface) -> Result<[(usize, f64, f64); 2]> {
    require(
        s.degree_u == 1
            && s.degree_v == 1
            && s.control_points.len() == 2
            && s.control_points[0].len() == 2
            && s.knots_u[0] == s.knots_u[1]
            && s.knots_u[2] == s.knots_u[3]
            && s.knots_v[0] == s.knots_v[1]
            && s.knots_v[2] == s.knots_v[3]
            && s.weights.iter().flatten().all(|&w| w == s.weights[0][0]),
        "Sewing requires an actual affine rectangular planar support",
    )?;
    let o = &s.control_points[0][0];
    let mut frame = [(0, 0., 0.); 2];
    for k in 0..2 {
        let p = if k == 0 {
            &s.control_points[1][0]
        } else {
            &s.control_points[0][1]
        };
        let axes: Vec<_> = (0..3).filter(|&j| o[j] != p[j]).collect();
        require(
            axes.len() == 1,
            "Planar support must have literal independent coordinate axes",
        )?;
        let j = axes[0];
        frame[k] = (j, o[j], p[j] - o[j]);
    }
    require(
        frame[0].0 != frame[1].0
            && (0..3).all(|k| {
                s.control_points[1][1][k]
                    == if k == frame[0].0 {
                        s.control_points[1][0][k]
                    } else if k == frame[1].0 {
                        s.control_points[0][1][k]
                    } else {
                        o[k]
                    }
            }),
        "Planar support has a non-affine fourth pole",
    )?;
    Ok(frame)
}
pub(super) fn project(c: &Curve, s: &Surface) -> Result<Curve> {
    let f = frame(s)?;
    let mut pc = c.clone();
    pc.control_points = c
        .control_points
        .iter()
        .map(|p| {
            (0..2)
                .map(|k| {
                    let (axis, o, d) = f[k];
                    let knots = if k == 0 { &s.knots_u } else { &s.knots_v };
                    let sign = d.signum();
                    let end = if k == 0 {
                        s.control_points[1][0][axis]
                    } else {
                        s.control_points[0][1][axis]
                    };
                    if knots[1] == sign * o && knots[2] == sign * end {
                        sign * p[axis]
                    } else {
                        knots[1] + (p[axis] - o) / d * (knots[2] - knots[1])
                    }
                })
                .collect()
        })
        .collect();
    pc.validate()?;
    Ok(pc)
}
pub(super) fn cap(bounds: [[f64; 2]; 3], axis: usize, fixed: f64) -> Surface {
    let axes: Vec<_> = (0..3).filter(|&k| k != axis).collect();
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![
            bounds[axes[0]][0],
            bounds[axes[0]][0],
            bounds[axes[0]][1],
            bounds[axes[0]][1],
        ],
        knots_v: vec![
            bounds[axes[1]][0],
            bounds[axes[1]][0],
            bounds[axes[1]][1],
            bounds[axes[1]][1],
        ],
        control_points: (0..2)
            .map(|u| {
                (0..2)
                    .map(|v| {
                        let mut p = vec![0.; 3];
                        p[axis] = fixed;
                        p[axes[0]] = bounds[axes[0]][u];
                        p[axes[1]] = bounds[axes[1]][v];
                        p
                    })
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
