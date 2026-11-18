//! Whole original coordinate-plane two-jets at a clamped extrusion boundary.
//! Positions remain a separate original-source obligation.
use super::*;
fn step(a: &[f64], b: &[f64]) -> Option<(usize, i8)> {
    let axes = (0..3).filter(|&k| a[k] != b[k]).collect::<Vec<_>>();
    let [axis] = axes.as_slice() else {
        return None;
    };
    Some((*axis, if b[*axis] > a[*axis] { 1 } else { -1 }))
}
fn cross(a: usize, b: usize, sa: i8, sb: i8) -> (usize, i8) {
    (3 - a - b, sa * sb * if (a + 1) % 3 == b { 1 } else { -1 })
}
fn normal(s: &Surface, p: &Curve) -> Option<(usize, i8)> {
    if s.periodic_u
        || s.periodic_v
        || p.periodic
        || p.control_points.len() > 256
        || s.control_points[0][0].len() != 3
    {
        return None;
    }
    let n = s.control_points.len();
    let m = s.control_points[0].len();
    let u = [s.knots_u[s.degree_u], s.knots_u[n]];
    let v = [s.knots_v[s.degree_v], s.knots_v[m]];
    if !s.knots_u[..=s.degree_u].iter().all(|&t| t == u[0])
        || !s.knots_u[n..].iter().all(|&t| t == u[1])
        || !s.knots_v[..=s.degree_v].iter().all(|&t| t == v[0])
        || !s.knots_v[m..].iter().all(|&t| t == v[1])
        || !p
            .control_points
            .iter()
            .all(|p| p[0] >= u[0] && p[0] <= u[1] && p[1] >= v[0] && p[1] <= v[1])
    {
        return None;
    }
    if s.degree_u == 1
        && s.degree_v == 1
        && n == 2
        && m == 2
        && s.weights.iter().flatten().all(|w| *w == s.weights[0][0])
    {
        if let (Some((a, sa)), Some((b, sb))) = (
            step(&s.control_points[0][0], &s.control_points[1][0]),
            step(&s.control_points[0][0], &s.control_points[0][1]),
        ) {
            if a != b
                && (0..3).all(|k| {
                    s.control_points[1][1][k]
                        == if k == a {
                            s.control_points[1][0][k]
                        } else {
                            s.control_points[0][1][k]
                        }
                })
            {
                return Some(cross(a, b, sa, sb));
            }
        }
    }
    if s.degree_u < 1 || s.degree_v != 1 || m != 2 || n > 256 {
        return None;
    }
    let at_end = if p.control_points.iter().all(|p| p[0] == u[0]) {
        false
    } else if p.control_points.iter().all(|p| p[0] == u[1]) {
        true
    } else {
        return None;
    };
    for free in 0..3 {
        let a = s.control_points[0][0][free];
        let b = s.control_points[0][1][free];
        if a == b
            || !s.control_points.iter().zip(&s.weights).all(|(row, w)| {
                w[0] == w[1]
                    && (0..3).all(|k| {
                        if k == free {
                            row[0][k] == a && row[1][k] == b
                        } else {
                            row[0][k] == row[1][k]
                        }
                    })
            })
        {
            continue;
        }
        let (i, j) = if at_end { (n - 2, n - 1) } else { (0, 1) };
        let Some((tangent, sign)) = step(&s.control_points[i][0], &s.control_points[j][0]) else {
            continue;
        };
        if tangent == free {
            continue;
        }
        let axis = 3 - free - tangent;
        let endpoint = if at_end { n - 1 } else { 0 };
        if !(0..(s.degree_u + 1).min(3)).all(|k| {
            let row = if at_end { n - 1 - k } else { k };
            s.control_points[row][0][axis] == s.control_points[endpoint][0][axis]
        }) {
            continue;
        }
        // Clamped U differentiation uses only the first/last three poles.
        // Their normal component is constant; product structure gives an
        // independent nonzero V tangent. Positive weights and noncoincident
        // adjacent poles give the exact nonzero U tangent sign everywhere.
        return Some(cross(tangent, free, sign, if b > a { 1 } else { -1 }));
    }
    None
}
pub(super) fn orientation(s: [&Surface; 2], p: [&Curve; 2]) -> Option<Orientation> {
    let a = normal(s[0], p[0])?;
    let b = normal(s[1], p[1])?;
    if a.0 != b.0 {
        return None;
    }
    Some(if a.1 == b.1 {
        Orientation::Same
    } else {
        Orientation::Opposite
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (Surface, Surface, Curve, Curve, Curve) {
        let c = crate::construct::curves::bounded_corner::propose(0.1, 10., 2.5e-8)
            .unwrap()
            .quartic_g2()
            .unwrap();
        let y = c.control_points[0][1];
        let skin = Surface {
            degree_u: c.degree,
            degree_v: 1,
            knots_u: c.knots,
            knots_v: vec![0., 0., 1., 1.],
            control_points: c
                .control_points
                .iter()
                .map(|p| vec![p.clone(), vec![p[0], p[1], 2.]])
                .collect(),
            weights: c.weights.iter().map(|w| vec![*w; 2]).collect(),
            periodic_u: false,
            periodic_v: false,
        };
        let plane = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0.; 3], vec![0., 0., 2.]],
                vec![vec![0., 1., 0.], vec![0., 1., 2.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let curve = Curve::from_polyline(vec![vec![0., y, 0.], vec![0., y, 2.]]).unwrap();
        let pa = Curve::from_polyline(vec![vec![y, 0.], vec![y, 1.]]).unwrap();
        let pb = Curve::from_polyline(vec![vec![0., 0.], vec![0., 1.]]).unwrap();
        (plane, skin, curve, pa, pb)
    }
    #[test]
    fn original_quartic_endpoint_has_regular_flat_two_jets_on_the_whole_contact() {
        let (plane, skin, c, pa, pb) = fixture();
        assert_eq!(
            orientation([&plane, &skin], [&pa, &pb]),
            Some(Orientation::Opposite)
        );
        let r = inspect(
            PcurveSeam { curve: &c, pcurves: [&pa, &pb], surfaces: [&plane, &skin], reversed: [false; 2] },
            G2PcurveTolerance { position: 1e-7, normal: 1e-5, curvature: 1e-4 },
            1000,
            100000,
        )
        .unwrap();
        assert!(r.positions.proven);
        assert_eq!(r.geometric.decision, Decision::WithinTolerance);
        assert!(r.geometric.regularity_proven);
    }
    #[test]
    fn exact_product_endpoint_and_nonstationary_tangent_are_required() {
        let (_, mut s, _, _, mut p) = fixture();
        assert!(normal(&s, &p).is_some());
        s.control_points[2][0][0] = f64::from_bits(1);
        assert!(normal(&s, &p).is_none());
        s.control_points[2][0][0] = 0.;
        p.control_points[0][0] = f64::from_bits(1);
        assert!(normal(&s, &p).is_none());
        p.control_points[0][0] = 0.;
        s.control_points[1][0] = s.control_points[0][0].clone();
        s.control_points[1][1] = s.control_points[0][1].clone();
        assert!(normal(&s, &p).is_none());
    }
}
