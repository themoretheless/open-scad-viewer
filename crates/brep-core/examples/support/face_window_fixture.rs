use brep_core::{Model, face_window::Boundary};
use nurbs_core::{Result, curve::Curve, surface::Surface};
pub fn build() -> Result<(Model, usize, Surface, Vec<Boundary>)> {
    let source = brep_core::cuboid([0.; 3], [8.; 3])?;
    let face = source
        .faces
        .iter()
        .position(|f| {
            f.surface
                .control_points
                .iter()
                .flatten()
                .all(|p| p[2] == 8.)
        })
        .unwrap();
    let original = &source.faces[face].surface;
    let mut grid = Vec::new();
    for i in 0..7 {
        let mut row = Vec::new();
        for j in 0..7 {
            let mut p = original
                .evaluate(0.25 + 0.5 * i as f64 / 6., 0.25 + 0.5 * j as f64 / 6.)?
                .point;
            if i == 3 && j == 3 {
                p[2] -= 1.024;
            }
            row.push(p.to_vec());
        }
        grid.push(row);
    }
    let knots = [vec![0.; 7], vec![1.; 7]].concat();
    let patch = Surface {
        degree_u: 6,
        degree_v: 6,
        knots_u: knots.clone(),
        knots_v: knots.clone(),
        control_points: grid,
        weights: vec![vec![1.; 7]; 7],
        periodic_u: false,
        periodic_v: false,
    };
    let origin = &original.control_points[0][0];
    let axis_u = (0..3)
        .find(|&k| original.control_points[1][0][k] != origin[k])
        .unwrap();
    let axis_v = (0..3)
        .find(|&k| original.control_points[0][1][k] != origin[k])
        .unwrap();
    let du = original.control_points[1][0][axis_u] - origin[axis_u];
    let dv = original.control_points[0][1][axis_v] - origin[axis_v];
    let uv = [
        [[0., 0.], [1., 0.]],
        [[1., 0.], [1., 1.]],
        [[1., 1.], [0., 1.]],
        [[0., 1.], [0., 0.]],
    ];
    let mut boundary = Vec::new();
    for side in 0..4 {
        let mut points: Vec<_> = if side == 0 || side == 2 {
            (0..7)
                .map(|i| patch.control_points[i][if side == 0 { 0 } else { 6 }].clone())
                .collect()
        } else {
            (0..7)
                .map(|j| patch.control_points[if side == 1 { 6 } else { 0 }][j].clone())
                .collect()
        };
        if side >= 2 {
            points.reverse();
        }
        let source_points = points
            .iter()
            .map(|p| {
                vec![
                    (p[axis_u] - origin[axis_u]) / du,
                    (p[axis_v] - origin[axis_v]) / dv,
                ]
            })
            .collect();
        let curve = Curve {
            degree: 6,
            knots: knots.clone(),
            control_points: points,
            weights: vec![1.; 7],
            periodic: false,
        };
        let source_pcurve = Curve {
            degree: 6,
            knots: knots.clone(),
            control_points: source_points,
            weights: vec![1.; 7],
            periodic: false,
        };
        let replacement_pcurve =
            Curve::from_polyline(vec![uv[side][0].to_vec(), uv[side][1].to_vec()])?;
        boundary.push(Boundary {
            curve,
            source_pcurve,
            replacement_pcurve,
        });
    }
    Ok((source, face, patch, boundary))
}
