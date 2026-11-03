//! Exact integration of polynomial original Cox patches. Positive equal weights
//! make the rational denominator constant; arbitrary degrees and knots remain.
use super::*;
use num_traits::ToPrimitive;
fn power(x: &Q, mut n: usize, w: &mut Work) -> Result<Q> {
    let mut out = Q::one();
    let mut base = x.clone();
    while n > 0 {
        if n % 2 == 1 {
            w.charge()?;
            out *= &base;
            w.coefficient(&out)?;
        }
        n /= 2;
        if n > 0 {
            w.charge()?;
            base = &base * &base;
            w.coefficient(&base)?;
        }
    }
    Ok(out)
}
fn integrate(p: &ExactSsPolynomial, d: [[Q; 2]; 2], w: &mut Work) -> Result<Q> {
    let mut total = Q::zero();
    for (e, c) in &p.terms {
        check(
            e[2] == 0 && e[3] == 0,
            "Original flux polynomial uses foreign span coordinates",
        )?;
        let mut term = c.clone();
        for k in 0..2 {
            let n = e[k]
                .checked_add(1)
                .ok_or_else(|| crate::resource("Flux power overflow"))?;
            let delta = power(&d[k][1], n, w)? - power(&d[k][0], n, w)?;
            w.charge()?;
            term = term * delta / Q::from_integer(n.into());
            w.coefficient(&term)?;
        }
        w.charge()?;
        total += term;
        w.coefficient(&total)?;
    }
    Ok(total)
}
pub(crate) fn bound(
    s: &Surface,
    rectangle: [[f64; 2]; 2],
    axis: usize,
    origin: f64,
    max_spans: usize,
    limits: ExactSsAlgebraLimits,
) -> Result<crate::surface_flux::PolynomialFluxReport> {
    s.validate()?;
    validate_limits(limits)?;
    check(
        max_spans <= 100000
            && axis < 3
            && origin.is_finite()
            && s.control_points.iter().flatten().all(|p| p.len() == 3),
        "Invalid original polynomial flux inputs",
    )?;
    let knots = [&s.knots_u, &s.knots_v];
    let p = [s.degree_u, s.degree_v];
    let n = [s.control_points.len(), s.control_points[0].len()];
    check(
        (0..2).all(|k| {
            rectangle[k].iter().all(|x| x.is_finite())
                && knots[k][p[k]] <= rectangle[k][0]
                && rectangle[k][0] < rectangle[k][1]
                && rectangle[k][1] <= knots[k][n[k]]
        }),
        "Polynomial flux leaves original active supports",
    )?;
    if !s.weights.iter().flatten().all(|v| *v == s.weights[0][0]) {
        return Ok(crate::surface_flux::PolynomialFluxReport {
            integral: None,
            work: 0,
            spans: 0,
        });
    }
    for (axis, periodic) in [s.periodic_u, s.periodic_v].into_iter().enumerate() {
        if periodic {
            check(
                crate::periodic_trim::verify_surface_seam_identity(s, axis)?.is_some(),
                "Polynomial periodic flux needs original seam identity",
            )?;
        }
    }
    let mut w = Work { used: 0, limits };
    let mut spans_used = 0;
    let result = (|| -> Result<[f64; 2]> {
        let mut total = Q::zero();
        for i in p[0]..n[0] {
            for j in p[1]..n[1] {
                let spans = [i, j];
                let d: [[f64; 2]; 2] = std::array::from_fn(|k| {
                    [
                        rectangle[k][0].max(knots[k][spans[k]]),
                        rectangle[k][1].min(knots[k][spans[k] + 1]),
                    ]
                });
                if d.iter().any(|v| v[0] >= v[1]) {
                    continue;
                }
                if spans_used == max_spans {
                    return Err(crate::resource("Exact original flux span work exhausted"));
                }
                spans_used += 1;
                let h = span::homogeneous(s, spans, 0, &mut w)?;
                let weight = h[3].terms.get(&[0; 4]).ok_or_else(|| {
                    crate::input("Original constant weight polynomial is unproved")
                })?;
                check(
                    h[3].terms.len() == 1 && weight > &Q::zero(),
                    "Original polynomial denominator is not a positive constant",
                )?;
                let du = (0..3)
                    .map(|k| h[k].derivative_bounded(0, &mut w))
                    .collect::<Result<Vec<_>>>()?;
                let dv = (0..3)
                    .map(|k| h[k].derivative_bounded(1, &mut w))
                    .collect::<Result<Vec<_>>>()?;
                let a = (axis + 1) % 3;
                let b = (axis + 2) % 3;
                let normal = du[a]
                    .product(&dv[b], &mut w)?
                    .subtract(&du[b].product(&dv[a], &mut w)?, &mut w)?;
                let position =
                    h[axis].subtract(&h[3].scale_monomial([0; 4], &q(origin)?, &mut w)?, &mut w)?;
                let density = position.product(&normal, &mut w)?;
                let mut clocks = std::array::from_fn(|_| [Q::zero(), Q::zero()]);
                for k in 0..2 {
                    let low = q(knots[k][spans[k]])?;
                    let width = q(knots[k][spans[k] + 1])? - &low;
                    clocks[k] = [(q(d[k][0])? - &low) / &width, (q(d[k][1])? - &low) / width];
                }
                let value = integrate(&density, clocks, &mut w)?;
                let cube = power(weight, 3, &mut w)?;
                w.charge()?;
                total += value / cube;
                w.coefficient(&total)?;
            }
        }
        let f = total
            .to_f64()
            .filter(|x| x.is_finite())
            .ok_or_else(|| crate::numeric_err("Original polynomial flux overflow"))?;
        let exact = q(f)?;
        Ok([
            if exact > total { f.next_down() } else { f },
            if exact < total { f.next_up() } else { f },
        ])
    })();
    Ok(crate::surface_flux::PolynomialFluxReport {
        integral: result.ok(),
        work: w.used,
        spans: spans_used,
    })
}
