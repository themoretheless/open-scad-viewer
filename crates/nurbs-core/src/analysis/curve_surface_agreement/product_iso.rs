//! Correlated original-product iso lifts with the ORIGINAL varying weights.
//! Rounded station coordinates and owned endpoint edits still need fresh G0.
use super::*;
struct Product {
    axis: usize,
    station: Interval,
}
fn descriptor(p: &Curve, s: &Surface, backwards: bool) -> Result<Option<Product>> {
    if p.degree != 1
        || p.control_points.len() != 2
        || p.weights[0] != p.weights[1]
        || p.periodic
        || s.periodic_u
        || s.periodic_v
        || s.degree_v != 1
        || s.control_points[0].len() != 2
        || s.control_points[0][0].len() != 3
        || s.control_points.len() > 256
    {
        return Ok(None);
    }
    let n = s.control_points.len();
    let u = [s.knots_u[s.degree_u], s.knots_u[n]];
    let v = [s.knots_v[1], s.knots_v[2]];
    let first = usize::from(backwards);
    if p.control_points[first][0] != u[0]
        || p.control_points[1 - first][0] != u[1]
        || p.control_points[0][1] != p.control_points[1][1]
        || !s.knots_u[..=s.degree_u].iter().all(|&t| t == u[0])
        || !s.knots_u[n..].iter().all(|&t| t == u[1])
    {
        return Ok(None);
    }
    let parameter = p.control_points[0][1];
    if parameter < v[0] || parameter > v[1] {
        return Ok(None);
    }
    for axis in 0..3 {
        let a = s.control_points[0][0][axis];
        let b = s.control_points[0][1][axis];
        if a == b {
            continue;
        }
        if !s.control_points.iter().zip(&s.weights).all(|(row, w)| {
            w[0] == w[1]
                && (0..3).all(|k| {
                    if k == axis {
                        row[0][k] == a && row[1][k] == b
                    } else {
                        row[0][k] == row[1][k]
                    }
                })
        }) {
            continue;
        }
        let fraction = Interval::point(parameter)
            .sub(Interval::point(v[0]))?
            .div(Interval::point(v[1]).sub(Interval::point(v[0]))?)?;
        let station =
            Interval::point(a).add(fraction.mul(Interval::point(b).sub(Interval::point(a))?)?)?;
        return Ok(Some(Product { axis, station }));
    }
    Ok(None)
}
pub(super) fn candidate(p: &Curve, s: &Surface) -> Result<Option<Curve>> {
    let Some(product) = descriptor(p, s, false)? else {
        return Ok(None);
    };
    let a = s.control_points[0][0][product.axis];
    let b = s.control_points[0][1][product.axis];
    let value = (product.station.lo * 0.5 + product.station.hi * 0.5).clamp(a.min(b), a.max(b));
    let c = Curve {
        degree: s.degree_u,
        knots: s.knots_u.clone(),
        periodic: false,
        control_points: s
            .control_points
            .iter()
            .map(|row| {
                let mut p = row[0].clone();
                p[product.axis] = value;
                p
            })
            .collect(),
        weights: s.weights.iter().map(|w| w[0]).collect(),
    };
    c.validate()?;
    Ok(Some(c))
}
pub(super) fn author_owned_station(
    c: &mut Curve,
    p: &Curve,
    s: &Surface,
    endpoints: [[f64; 3]; 2],
) -> Result<()> {
    let Some(product) = descriptor(p, s, false)? else {
        return Ok(());
    };
    if endpoints[0][product.axis] == endpoints[1][product.axis] {
        // The SAME owned station constrains the whole end section. Editing
        // only its end poles can introduce enormous actual curvature with
        // small rational weights. This authors new coefficients; the complete
        // original-source agreement below is recomputed after this change.
        for point in &mut c.control_points {
            point[product.axis] = endpoints[0][product.axis];
        }
    }
    Ok(())
}
pub(super) fn upper(c: &Curve, p: &Curve, s: &Surface, reversed: bool) -> Result<Option<f64>> {
    // Reversing BOTH traversal clocks preserves the full-interval identity.
    // Only the two poles of a literal affine pcurve are interchanged here;
    // the world curve and all original source coefficients stay unchanged.
    let Some(product) = descriptor(p, s, reversed)? else {
        return Ok(None);
    };
    if c.periodic
        || c.degree != s.degree_u
        || c.knots != s.knots_u
        || c.control_points.len() != s.control_points.len()
        || c.weights.iter().zip(&s.weights).any(|(a, b)| *a != b[0])
    {
        return Ok(None);
    }
    let mut maximum = 0f64;
    for (actual, source) in c.control_points.iter().zip(&s.control_points) {
        let expected: Vec<_> = (0..3)
            .map(|k| {
                if k == product.axis {
                    product.station
                } else {
                    Interval::point(source[0][k])
                }
            })
            .collect();
        let actual: Vec<_> = actual.iter().map(|&v| Interval::point(v)).collect();
        maximum = maximum.max(box_distance(&actual, &expected)?.1);
    }
    // Identical original weights and clocks give identical positive rational
    // basis functions. Their convex sum of coefficient differences has norm
    // <= the largest bound above, on the WHOLE original parameter interval.
    Ok(Some(maximum))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (Surface, Curve) {
        let c = crate::construct::curves::bounded_corner::propose(0.1, 10., 2.5e-8)
            .unwrap()
            .quartic_g2()
            .unwrap();
        let s = Surface {
            degree_u: c.degree,
            degree_v: 1,
            knots_u: c.knots,
            knots_v: vec![0., 0., 1., 1.],
            control_points: c
                .control_points
                .iter()
                .map(|p| vec![vec![p[0], p[1], 0.2], vec![p[0], p[1], 2.2]])
                .collect(),
            weights: c.weights.iter().map(|w| vec![*w; 2]).collect(),
            periodic_u: false,
            periodic_v: false,
        };
        let p = Curve::from_polyline(vec![vec![0., 0.3], vec![1., 0.3]]).unwrap();
        (s, p)
    }
    #[test]
    fn original_weights_cover_owned_end_edits_without_dividing_by_minimum_weight() {
        let (s, p) = fixture();
        let before = s.clone();
        let mut c = candidate(&p, &s).unwrap().unwrap();
        assert_eq!(
            c.weights,
            s.weights.iter().map(|w| w[0]).collect::<Vec<_>>()
        );
        c.control_points[0][2] += 1e-9;
        c.control_points[4][2] -= 1e-9;
        let report = verify(&c, &p, &s, false, 1e-7, 1).unwrap();
        assert_eq!(report.status, Status::WithinTolerance);
        assert_eq!(report.cells, 1);
        c.control_points[2][0] += 0.01;
        assert_ne!(
            verify(&c, &p, &s, false, 1e-7, 1).unwrap().status,
            Status::WithinTolerance
        );
        assert_eq!(s, before);
    }
    #[test]
    fn literal_weights_station_and_product_structure_are_required() {
        let (mut s, mut p) = fixture();
        let mut c = candidate(&p, &s).unwrap().unwrap();
        c.weights[0] = c.weights[0].next_up();
        assert!(upper(&c, &p, &s, false).unwrap().is_none());
        p.control_points[1][1] = p.control_points[1][1].next_up();
        assert!(candidate(&p, &s).unwrap().is_none());
        p.control_points[1][1] = p.control_points[0][1];
        s.control_points[1][1][0] = f64::from_bits(1);
        assert!(candidate(&p, &s).unwrap().is_none());
    }
    #[test]
    fn actual_returned_lift_is_freshly_checked_after_endpoint_authorship() {
        let (s, p) = fixture();
        let c = candidate(&p, &s).unwrap().unwrap();
        let mut endpoints = [[0.; 3]; 2];
        endpoints[0].copy_from_slice(&c.control_points[0]);
        endpoints[1].copy_from_slice(&c.control_points[4]);
        endpoints[0][2] += 1e-9;
        let report = lift(&p, &s, Some(endpoints), 1e-7, 1).unwrap();
        let actual = report.curve.unwrap();
        assert_eq!(actual.control_points[0], endpoints[0]);
        assert_eq!(
            verify(&actual, &p, &s, false, 1e-7, 1).unwrap().status,
            Status::WithinTolerance
        );
        assert!(
            lift(&p, &s, Some(endpoints), 1e-7, 0)
                .unwrap()
                .curve
                .is_none()
        );
    }
    #[test]
    fn reversed_coedges_keep_original_clocks_and_partial_intervals_do_not_inherit_full_proof() {
        let (s, p) = fixture();
        let c = candidate(&p, &s).unwrap().unwrap();
        let reversed = p.reverse().unwrap();
        assert_eq!(
            verify(&c, &reversed, &s, true, 1e-7, 1).unwrap().status,
            Status::WithinTolerance
        );
        assert!(upper(&c, &reversed, &s, false).unwrap().is_none());
        assert_ne!(
            verify_on(&c, &p, &s, false, [0., 0.5], 1e-7, 1)
                .unwrap()
                .status,
            Status::WithinTolerance
        );
    }
    #[test]
    fn equal_owned_root_stations_constrain_the_whole_extracted_section() {
        let (s, p) = fixture();
        let c = candidate(&p, &s).unwrap().unwrap();
        let station = c.control_points[0][2].next_down();
        let mut endpoints = [[0.; 3]; 2];
        endpoints[0].copy_from_slice(&c.control_points[0]);
        endpoints[1].copy_from_slice(&c.control_points[4]);
        endpoints[0][2] = station;
        endpoints[1][2] = station;
        let lifted = lift(&p, &s, Some(endpoints), 1e-7, 1).unwrap();
        let actual = lifted.curve.unwrap();
        assert!(actual.control_points.iter().all(|p| p[2] == station));
        assert_eq!(actual.control_points[0], endpoints[0]);
        assert_eq!(actual.control_points[4], endpoints[1]);
        assert_eq!(
            verify(&actual, &p, &s, false, 1e-7, 1).unwrap().status,
            Status::WithinTolerance
        );
    }
}
