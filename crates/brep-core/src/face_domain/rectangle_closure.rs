//! Literal rectangular boundary recognition, including shared-edge subdivisions.
use nurbs_core::curve::Curve;
pub(super) fn inspect(curves: &[Curve]) -> Option<[[f64; 2]; 2]> {
    if !(4..=256).contains(&curves.len())
        || curves.iter().any(|c| {
            c.validate().is_err()
                || c.degree != 1
                || c.control_points.len() != 2
                || c.control_points[0].len() != 2
                || c.periodic
                || c.knots[0] != c.knots[1]
                || c.knots[2] != c.knots[3]
        })
    {
        return None;
    }
    let segments = curves
        .iter()
        .map(|c| {
            [
                [c.control_points[0][0], c.control_points[0][1]],
                [c.control_points[1][0], c.control_points[1][1]],
            ]
        })
        .collect::<Vec<_>>();
    if segments
        .iter()
        .enumerate()
        .any(|(i, s)| s[1] != segments[(i + 1) % segments.len()][0])
    {
        return None;
    }
    let bounds = std::array::from_fn(|k| {
        [
            segments
                .iter()
                .map(|s| s[0][k])
                .fold(f64::INFINITY, f64::min),
            segments
                .iter()
                .map(|s| s[0][k])
                .fold(f64::NEG_INFINITY, f64::max),
        ]
    });
    if !(0..2).all(|k| bounds[k][0] < bounds[k][1]) {
        return None;
    }
    let mut sides: [Vec<[f64; 2]>; 4] = std::array::from_fn(|_| Vec::new());
    for s in &segments {
        let axis = if s[0][0] == s[1][0] && s[0][1] != s[1][1] {
            0
        } else if s[0][1] == s[1][1] && s[0][0] != s[1][0] {
            1
        } else {
            return None;
        };
        let value = s[0][axis];
        let side = if value == bounds[axis][0] {
            0
        } else if value == bounds[axis][1] {
            1
        } else {
            return None;
        };
        let free = 1 - axis;
        sides[axis * 2 + side].push([s[0][free].min(s[1][free]), s[0][free].max(s[1][free])]);
    }
    for (side, segments) in sides.iter_mut().enumerate() {
        segments.sort_by(|a, b| a[0].total_cmp(&b[0]));
        let free = 1 - side / 2;
        if segments.is_empty()
            || segments[0][0] != bounds[free][0]
            || segments.last().unwrap()[1] != bounds[free][1]
            || segments.windows(2).any(|w| w[0][1] != w[1][0])
        {
            return None;
        }
    }
    Some(bounds)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn polygon(points: &[[f64; 2]]) -> Vec<Curve> {
        (0..points.len())
            .map(|i| {
                Curve::from_polyline(vec![
                    points[i].to_vec(),
                    points[(i + 1) % points.len()].to_vec(),
                ])
                .unwrap()
            })
            .collect()
    }
    #[test]
    fn subdivided_sides_require_exact_closed_nonoverlapping_coverage() {
        let good = polygon(&[
            [0., 0.],
            [0.25, 0.],
            [1., 0.],
            [1., 1.],
            [0., 1.],
            [0., 0.5],
        ]);
        assert_eq!(inspect(&good), Some([[0., 1.], [0., 1.]]));
        let overlap = polygon(&[
            [0., 0.],
            [0.75, 0.],
            [0.25, 0.],
            [1., 0.],
            [1., 1.],
            [0., 1.],
        ]);
        assert!(inspect(&overlap).is_none());
        let interior = polygon(&[
            [0., 0.],
            [0.5, 0.],
            [0.5, 0.5],
            [1., 0.5],
            [1., 1.],
            [0., 1.],
        ]);
        assert!(inspect(&interior).is_none());
        let mut gap = good;
        gap[1].control_points[0][0] += 1e-12;
        assert!(inspect(&gap).is_none());
    }
}
