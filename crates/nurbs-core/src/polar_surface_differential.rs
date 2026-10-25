//! Original mean-differential columns for a verified quadratic collapsed rail.
//! S_v=t² B: a chord is A delta_u + B_mean delta_v integral(t²),
//! with independent averaged columns enclosed here, including t=0 limits.
use crate::{interval_eval::Interval as I, normal_alignment, surface::Surface, Result};
pub(crate) fn bounds(s: &Surface, end: bool, domain: [[f64; 2]; 2]) -> Result<Option<[[I; 2]; 3]>> {
    let p = s.degree_u;
    let q = s.degree_v;
    if p < 2
        || p > 8
        || q > 8
        || q < 1
        || s.periodic_u
        || s.periodic_v
        || s.control_points.len() != p + 1
        || s.control_points[0].len() != q + 1
    {
        return Ok(None);
    }
    let single = |d: usize, k: &[f64]| {
        k.len() == 2 * (d + 1)
            && k[..=d].iter().all(|x| *x == k[0])
            && k[d + 1..].iter().all(|x| *x == k[d + 1])
    };
    if !single(p, &s.knots_u) || !single(q, &s.knots_v) {
        return Ok(None);
    }
    let mut source = s.clone();
    let span = [
        [s.knots_u[p], s.knots_u[p + 1]],
        [s.knots_v[q], s.knots_v[q + 1]],
    ];
    let mut d = domain;
    if end {
        source.control_points.reverse();
        source.weights.reverse();
        let lo = I::point(span[0][0]).add(I::point(span[0][1]).sub(I::point(domain[0][1]))?)?;
        let hi = I::point(span[0][0]).add(I::point(span[0][1]).sub(I::point(domain[0][0]))?)?;
        d[0] = [lo.lo.max(span[0][0]), hi.hi.min(span[0][1])];
    }
    // Exact equality of authored binary64 definitions proves that both H_v
    // and W_v have two identically zero leading U rows before any arithmetic.
    if (0..2).any(|u| {
        (1..=q).any(|v| {
            source.weights[u][v] != source.weights[u][0]
                || source.control_points[u][v] != source.control_points[u][0]
        })
    }) {
        return Ok(None);
    }
    let origin = &source.control_points[0][0];
    let mut net = vec![vec![[I::point(0.); 4]; q + 1]; p + 1];
    for u in 0..=p {
        for v in 0..=q {
            let w = I::point(source.weights[u][v]);
            net[u][v][3] = w;
            for k in 0..3 {
                net[u][v][k] = I::point(source.control_points[u][v][k])
                    .sub(I::point(origin[k]))?
                    .mul(w)?;
            }
        }
    }
    let values = normal_alignment::restrict_hull(&net, [p, q], span, d)?;
    let denominator = values[3].mul(values[3])?;
    let mut dv = normal_alignment::differentiate_net(&net, [p, q], 1, span)?;
    for row in &mut dv[..2] {
        for h in row {
            *h = [I::point(0.); 4];
        }
    }
    let mut factored = Vec::new();
    for u in 2..=p {
        let scale = I::point((p * (p - 1)) as f64).div(I::point((u * (u - 1)) as f64))?;
        let mut row = Vec::new();
        for h in &dv[u] {
            let mut value = [I::point(0.); 4];
            for k in 0..4 {
                value[k] = h[k].mul(scale)?;
            }
            row.push(value);
        }
        factored.push(row);
    }
    let scaled = normal_alignment::restrict_hull(&factored, [p - 2, q - 1], span, d)?;
    let mut j = normal_alignment::jacobian_on(&source, [p, q], d)?;
    for k in 0..3 {
        j[k][1] = scaled[k]
            .mul(values[3])?
            .sub(values[k].mul(scaled[3])?)?
            .div(denominator)?;
    }
    Ok(Some(j))
}
#[cfg(test)]
mod tests {
    use super::*;
    fn s() -> Surface {
        Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.]; 2],
                vec![vec![0.5, 0., 0.]; 2],
                vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            ],
            weights: vec![vec![1.; 2]; 3],
            periodic_u: false,
            periodic_v: false,
        }
    }
    #[test]
    fn divided_columns_cover_independent_quadratic_formula() {
        for end in [false, true] {
            let mut s = s();
            if end {
                s.control_points.reverse();
            }
            for d in [
                [[0., 1.], [0., 1.]],
                [[0., 0.], [0.2, 0.7]],
                [[0.3, 0.4], [0.2, 0.7]],
            ] {
                let j = bounds(&s, end, d).unwrap().unwrap();
                assert!(j[0][0].contains(1.));
                assert!(j[0][1].contains(0.));
                assert!(j[1][1].contains(1.));
                for u in [d[0][0], d[0][1]] {
                    for v in [d[1][0], d[1][1]] {
                        let t = if end { 1. - u } else { u };
                        assert!(j[1][0].contains(2. * t * v));
                    }
                }
            }
        }
    }
    #[test]
    fn original_polar_chord_bound_covers_collapsed_diagonal_and_refuses_missing_work() {
        for end in [false, true] {
            let mut source = s();
            if end {
                source.control_points.reverse();
            }
            let d = [if end { [0.999, 1.] } else { [0., 0.001] }, [0., 1.]];
            assert!(
                crate::surface_self_chord::qualify_rectangle(&source, d, 1e-6, 3)
                    .unwrap()
                    .certificate
                    .is_none()
            );
            let r = crate::surface_self_chord::qualify_polar_rectangle(&source, d, end, 1e-6, 3)
                .unwrap();
            let c = r.certificate.unwrap();
            assert!(std::ptr::eq(c.surface(), &source));
            assert_eq!(c.domain(), d);
            assert_eq!(r.spans, 3);
            assert!(
                crate::surface_self_chord::qualify_polar_rectangle(&source, d, end, 1e-6, 2)
                    .unwrap()
                    .certificate
                    .is_none()
            );
        }
        let mut source = s();
        source.weights[0][1] = 0.5;
        assert!(crate::surface_self_chord::qualify_polar_rectangle(
            &source,
            [[0., 1.], [0., 1.]],
            false,
            1e-6,
            3
        )
        .unwrap()
        .certificate
        .is_none());
    }
    #[test]
    fn absent_exact_factor_refuses() {
        let mut s = s();
        s.control_points[0][1][1] = f64::MIN_POSITIVE;
        assert!(bounds(&s, false, [[0., 1.], [0., 1.]]).unwrap().is_none());
    }
}
