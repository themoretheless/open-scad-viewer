//! Degree-7 Cartesian V proposal with two free interior rows. Original
//! boundary two-jets are retained as targets; every actual result needs fresh
//! G2, absolute curvature, source agreement and body admission.
use super::*;
fn mix(a: f64, b: f64, t: f64) -> f64 {
    if a == b { a } else { (1. - t) * a + t * b }
}
pub(super) fn propose(
    source: &[Surface],
    relaxation: f64,
    stock: [[f64; 2]; 3],
    work: &mut Work,
) -> Result<Vec<Surface>> {
    require(
        relaxation.is_finite() && (0. ..=1.).contains(&relaxation),
        "Bound interior fairing relaxation",
    )?;
    let mut out = Vec::new();
    for s in source {
        s.validate()?;
        require(
            s.degree_v == 5
                && s.control_points[0].len() == 6
                && s.knots_v == [vec![0.; 6], vec![1.; 6]].concat()
                && !s.periodic_v
                && s.control_points[0][0].len() == 3
                && s.weights.iter().all(|w| w.iter().all(|&v| v == w[0])),
            "Interior fairing requires original Cartesian V cells",
        )?;
        let mut points = Vec::new();
        for row in &s.control_points {
            let mut p = row.clone();
            // Positive polynomial degree elevation. Constant components are
            // copied literally, including all collapsed pole columns.
            for q in [6, 7] {
                let mut elevated = vec![p[0].clone()];
                for i in 1..q {
                    elevated.push(
                        (0..3)
                            .map(|k| mix(p[i][k], p[i - 1][k], i as f64 / q as f64))
                            .collect(),
                    );
                }
                elevated.push(p.last().unwrap().clone());
                p = elevated;
            }
            // Minimize the sum of squared second control differences in V
            // with the first/last THREE rows fixed. This is a shape proposal,
            // not an optimization/cap certificate.
            for k in 0..3 {
                if p.iter().all(|v| v[k] == p[0][k]) {
                    continue;
                }
                let r3 = -p[1][k] + 4. * p[2][k] - p[5][k];
                let r4 = -p[2][k] + 4. * p[5][k] - p[6][k];
                let a = (6. * r3 + 4. * r4) / 20.;
                let b = (4. * r3 + 6. * r4) / 20.;
                p[3][k] = mix(p[3][k], a, relaxation);
                p[4][k] = mix(p[4][k], b, relaxation);
            }
            for point in &p {
                for k in 0..3 {
                    require(
                        point[k].is_finite() && stock[k][0] <= point[k] && point[k] <= stock[k][1],
                        "Interior fairing control hull leaves original stock",
                    )?;
                }
            }
            work.charge(180)?;
            points.push(p);
        }
        let patch = Surface {
            degree_u: s.degree_u,
            degree_v: 7,
            knots_u: s.knots_u.clone(),
            knots_v: [vec![0.; 8], vec![1.; 8]].concat(),
            control_points: points,
            weights: s.weights.iter().map(|w| vec![w[0]; 8]).collect(),
            periodic_u: s.periodic_u,
            periodic_v: false,
        };
        patch.validate()?;
        out.push(patch);
    }
    Ok(out)
}
/// Positive Cartesian V elevation supplies extra free interior rows. Floating
/// boundary derivatives must be independently checked after construction.
pub(super) fn elevate_v(
    source: &[Surface],
    degree: usize,
    work: &mut Work,
) -> Result<Vec<Surface>> {
    let mut out = Vec::new();
    for s in source {
        s.validate()?;
        let q = s.degree_v;
        require(
            degree >= q
                && s.control_points.iter().flatten().all(|p| p.len() == 3)
                && degree <= 16
                && !s.periodic_v
                && s.control_points[0].len() == q + 1
                && s.knots_v == [vec![s.knots_v[q]; q + 1], vec![s.knots_v[q + 1]; q + 1]].concat()
                && s.weights.iter().all(|w| w.iter().all(|&x| x == w[0])),
            "Bound Cartesian V elevation",
        )?;
        let mut t = s.clone();
        for target in q + 1..=degree {
            let mut points = Vec::new();
            for row in &t.control_points {
                work.charge(target * 12)?;
                let mut r = vec![row[0].clone()];
                for j in 1..target {
                    r.push(
                        (0..3)
                            .map(|k| mix(row[j][k], row[j - 1][k], j as f64 / target as f64))
                            .collect(),
                    );
                }
                r.push(row.last().unwrap().clone());
                points.push(r);
            }
            t.control_points = points;
            t.weights = t.weights.iter().map(|w| vec![w[0]; target + 1]).collect();
            t.degree_v = target;
            t.knots_v = [
                vec![s.knots_v[q]; target + 1],
                vec![s.knots_v[q + 1]; target + 1],
            ]
            .concat();
        }
        t.validate()?;
        out.push(t);
    }
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn whole_original_boundary_g2_is_preserved_while_interior_changes() {
        let s = Surface {
            degree_u: 1,
            degree_v: 5,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: [vec![0.; 6], vec![1.; 6]].concat(),
            control_points: [0., 1.]
                .map(|x| {
                    (0..6)
                        .map(|j| vec![x, j as f64 / 5., if j < 3 { 0. } else { 0.5 }])
                        .collect()
                })
                .to_vec(),
            weights: vec![vec![1.; 6]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let before = s.clone();
        let q = propose(
            std::slice::from_ref(&s),
            1.,
            [[-2., 2.]; 3],
            &mut Work {
                used: 0,
                max: 10000,
            },
        )
        .unwrap();
        let q = &q[0];
        assert_eq!(q.degree_v, 7);
        assert!(
            (q.evaluate(0.5, 0.25).unwrap().point[2] - s.evaluate(0.5, 0.25).unwrap().point[2])
                .abs()
                > 1e-5
        );
        for (v, z) in [(0., 0.), (1., 0.5)] {
            let c = Curve::from_polyline(vec![vec![0., v, z], vec![1., v, z]]).unwrap();
            let p = Curve::from_polyline(vec![vec![0., v], vec![1., v]]).unwrap();
            let r = nurbs_core::surface_g2_pcurves::inspect(
                nurbs_core::surface_g2_pcurves::PcurveSeam { curve: &c, pcurves: [&p, &p], surfaces: [&s, q], reversed: [false, false] },
                nurbs_core::surface_g2_pcurves::G2PcurveTolerance { position: 1e-7, normal: 1e-5, curvature: 1e-4 },
                1000,
                100000,
            )
            .unwrap();
            assert!(r.positions.proven);
            assert_eq!(
                r.geometric.decision,
                nurbs_core::surface_join::Decision::WithinTolerance
            );
            assert!(r.geometric.regularity_proven);
        }
        assert_eq!(s, before);
        assert!(
            propose(
                std::slice::from_ref(&s),
                1.,
                [[-2., 2.]; 3],
                &mut Work { used: 0, max: 0 }
            )
            .is_err()
        );
    }
    #[test]
    fn constant_pole_columns_and_stock_guards_are_literal() {
        let s = Surface {
            degree_u: 1,
            degree_v: 5,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: [vec![0.; 6], vec![1.; 6]].concat(),
            control_points: vec![vec![vec![0.1, 0.2, 0.3]; 6]; 2],
            weights: vec![vec![0.25; 6]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let q = propose(
            std::slice::from_ref(&s),
            1.,
            [[0., 1.]; 3],
            &mut Work { used: 0, max: 1000 },
        )
        .unwrap();
        assert!(
            q[0].control_points
                .iter()
                .flatten()
                .all(|p| p == &s.control_points[0][0])
        );
        assert!(
            propose(
                std::slice::from_ref(&s),
                1.,
                [[0., 0.01]; 3],
                &mut Work { used: 0, max: 1000 }
            )
            .is_err()
        );
        assert!(
            propose(
                std::slice::from_ref(&s),
                f64::NAN,
                [[0., 1.]; 3],
                &mut Work { used: 0, max: 1000 }
            )
            .is_err()
        );
    }
}
