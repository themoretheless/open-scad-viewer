//! Whole filled-trim face enclosures, not nominal-mesh boxes.
use super::*;
use nurbs_core::interval_eval::{self, Interval as I};
pub(super) fn model(m: &Model) -> Result<Vec<[[f64; 2]; 3]>> {
    m.faces
        .iter()
        .map(|f| face(&f.surface, &m.loops[f.outer].coedges))
        .collect()
}
fn face(s: &crate::Surface, ring: &[crate::Coedge]) -> Result<[[f64; 2]; 3]> {
    if ring.is_empty()
        || ring
            .iter()
            .any(|c| c.pcurve.weights.iter().any(|w| *w <= 0.))
        || s.weights.iter().flatten().any(|w| *w <= 0.)
    {
        return Err(fail(
            "trim face enclosure requires positive original weights",
        ));
    }
    let carrier: [[f64; 2]; 3] = std::array::from_fn(|k| {
        [
            s.control_points
                .iter()
                .flatten()
                .map(|p| p[k])
                .min_by(f64::total_cmp)
                .unwrap(),
            s.control_points
                .iter()
                .flatten()
                .map(|p| p[k])
                .max_by(f64::total_cmp)
                .unwrap(),
        ]
    });
    // A periodic trim can carry UV lifts OUTSIDE the natural cell.
    // Clipping that lifted UV box to the cell could erase real face
    // points. The full positive carrier hull is valid for every lift.
    if s.periodic_u || s.periodic_v {
        return Ok(carrier);
    }
    let uv: [[f64; 2]; 2] = std::array::from_fn(|k| {
        [
            ring.iter()
                .flat_map(|c| &c.pcurve.control_points)
                .map(|p| p[k])
                .min_by(f64::total_cmp)
                .unwrap(),
            ring.iter()
                .flat_map(|c| &c.pcurve.control_points)
                .map(|p| p[k])
                .max_by(f64::total_cmp)
                .unwrap(),
        ]
    });
    let natural = [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ];
    let d: [I; 2] = std::array::from_fn(|k| I {
        lo: uv[k][0].max(natural[k][0]),
        hi: uv[k][1].min(natural[k][1]),
    });
    let b = interval_eval::evaluate_surface_interval(
        s,
        I::new(d[0].lo, d[0].hi)?,
        I::new(d[1].lo, d[1].hi)?,
    )?;
    // Intersect two independently valid enclosures; constant coordinates
    // remain exact even when interval arithmetic widens by a few ulps.
    Ok(std::array::from_fn(|k| {
        [
            b[k].lo.max(
                s.control_points
                    .iter()
                    .flatten()
                    .map(|p| p[k])
                    .min_by(f64::total_cmp)
                    .unwrap(),
            ),
            b[k].hi.min(
                s.control_points
                    .iter()
                    .flatten()
                    .map(|p| p[k])
                    .max_by(f64::total_cmp)
                    .unwrap(),
            ),
        ]
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn periodic_uv_lifts_cannot_shrink_the_original_carrier_enclosure() {
        let poles = [
            [1., 0.],
            [0.5, 1.],
            [-0.5, 1.],
            [-1., 0.],
            [-0.5, -1.],
            [0.5, -1.],
            [1., 0.],
            [0.5, 1.], // Explicit repetition of the first degree poles.
        ];
        let s = crate::Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: (0..11).map(|i| i as f64).collect(),
            knots_v: vec![0., 0., 1., 1.],
            control_points: poles
                .iter()
                .map(|p| vec![vec![p[0], p[1], 0.], vec![p[0], p[1], 1.]])
                .collect(),
            weights: vec![vec![1.; 2]; 8],
            periodic_u: true,
            periodic_v: false,
        };
        s.validate().unwrap();
        let uv = [[8., 0.], [14., 0.], [14., 1.], [8., 1.]];
        let ring = (0..4)
            .map(|i| {
                crate::Coedge::new(
                    0,
                    false,
                    crate::Curve::from_polyline(vec![uv[i].to_vec(), uv[(i + 1) % 4].to_vec()])
                        .unwrap(),
                )
            })
            .collect::<Vec<_>>();
        // This is an enclosure-helper test, not an admission of a B-rep body.
        // The lifted outer domain covers a full period. Clipping it to [2,8]
        // would retain only u=8 and wrongly discard three quarters of the face.
        let b = face(&s, &ring).unwrap();
        for u in [2., 3.5, 5., 6.5, 8.] {
            let p = s.evaluate(u, 0.5).unwrap().point;
            assert!((0..3).all(|k| b[k][0] <= p[k] && p[k] <= b[k][1]));
        }
        assert!(b[0][0] <= -1. && b[0][1] >= 1. && b[1][0] <= -1. && b[1][1] >= 1.);
    }
}
