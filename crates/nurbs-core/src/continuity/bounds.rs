//! Continuous Euclidean jet error bounds from homogeneous coefficient hulls.
//! Both boundaries have the same denominator and exactly affine-compatible bases.
use value_codec::{Value,json};
use super::{Boundary, regularity::I};
use crate::{Result, check, intersection::next_up, surface::Surface};
type H = [I; 4];
fn homogeneous(s: &Surface, b: Boundary, i: usize, layer: usize) -> H {
    let (u, v) = b.index(s, i, layer);
    let w = I::point(s.weights[u][v]);
    std::array::from_fn(|a| {
        if a == 3 {
            w
        } else {
            I::point(s.control_points[u][v][a]).mul(w)
        }
    })
}
fn jets(s: &Surface, b: Boundary, i: usize, order: usize) -> Result<Vec<H>> {
    let (p, n, k, _) = b.cross(s);
    let a = k[p];
    let z = k[n];
    let span = I::point(z).sub(I::point(a));
    let delta = |layer| {
        if b.max {
            I::point(z).sub(I::point(k[n - layer]))
        } else {
            I::point(k[p + layer]).sub(I::point(a))
        }
    };
    let h0 = homogeneous(s, b, i, 0);
    let h1 = homogeneous(s, b, i, 1);
    let f = I::point(p as f64).mul(span).div_positive(delta(1))?;
    let d1 = std::array::from_fn(|a| h1[a].sub(h0[a]).mul(f));
    let mut result = vec![h0, d1];
    if order == 2 {
        // A linear homogeneous cross strip has zero second homogeneous jet.
        // Its Euclidean second derivative can still be nonzero when weights
        // vary; certify() retains the quotient terms below.
        if p == 1 {
            result.push([I::point(0.); 4]);
            return Ok(result);
        }
        let h2 = homogeneous(s, b, i, 2);
        let f = I::point((p * (p - 1)) as f64)
            .mul(span)
            .mul(span)
            .div_positive(delta(1))?;
        let mut d2 = [I::point(0.); 4];
        for a in 0..4 {
            d2[a] = h2[a]
                .sub(h1[a])
                .div_positive(delta(2))?
                .sub(h1[a].sub(h0[a]).div_positive(delta(1))?)
                .mul(f);
        }
        result.push(d2);
    }
    Ok(result)
}
fn magnitude(x: I) -> f64 {
    x.lo.abs().max(x.hi.abs())
}
fn norm(values: [I; 3]) -> f64 {
    let mut sum = I::point(0.);
    for v in values {
        let m = I::point(magnitude(v));
        sum = sum.add(m.mul(m));
    }
    next_up(sum.hi.sqrt())
}
pub(super) fn certify(
    reference: &Surface,
    edited: &Surface,
    r: Boundary,
    e: Boundary,
    order: usize,
    scale: f64,
) -> Result<Value> {
    let n = r.along(reference).1;
    let mut delta_first = Vec::new();
    let mut boundary = Vec::new();
    let mut difference = vec![[0_f64; 4]; order + 1];
    let mut target = vec![[0_f64; 4]; order + 1];
    let mut expected = vec![[0_f64; 4]; order + 1];
    let mut wmin = f64::INFINITY;
    for i in 0..n {
        let (ru, rv) = r.index(reference, i, 0);
        let (eu, ev) = e.index(edited, i, 0);
        check(
            reference.weights[ru][rv] == edited.weights[eu][ev],
            "Jet certificate requires the same boundary weights",
        )?;
        check(
            reference.control_points[ru][rv] == edited.control_points[eu][ev],
            "Jet certificate requires identical boundary control points",
        )?;
        wmin = wmin.min(reference.weights[ru][rv]);
        let a = jets(reference, r, i, order)?;
        let b = jets(edited, e, i, order)?;
        delta_first.push(std::array::from_fn::<_, 4, _>(|axis| {
            b[1][axis].add(a[1][axis].mul(I::point(scale)))
        }));
        boundary.push(a[0]);
        for q in 0..=order {
            let factor = match q {
                0 => I::point(1.),
                1 => I::point(-scale),
                _ => I::point(scale).mul(I::point(scale)),
            };
            for axis in 0..4 {
                let expected_value = a[q][axis].mul(factor);
                if q > 0 {
                    difference[q][axis] =
                        difference[q][axis].max(magnitude(b[q][axis].sub(expected_value)));
                }
                target[q][axis] = target[q][axis].max(magnitude(b[q][axis]));
                expected[q][axis] = expected[q][axis].max(magnitude(expected_value));
            }
        }
    }
    let d = |q: usize, a: usize| I::point(difference[q][a]);
    let t = |q: usize, a: usize| I::point(target[q][a]);
    let a = |q: usize, c: usize| I::point(expected[q][c]);
    let inv = I::point(1.).div_positive(I::point(wmin))?;
    let inv2 = inv.mul(inv);
    let inv3 = inv2.mul(inv);
    let two = I::point(2.);
    let position = 0_f64;
    let mut first = norm(std::array::from_fn(|axis| {
        d(1, axis).mul(inv).add(
            d(0, axis)
                .mul(a(1, 3))
                .add(t(0, axis).mul(d(1, 3)))
                .mul(inv2),
        )
    }));
    // Preserve rational cancellation before taking magnitudes. With common
    // boundary H and cross-jet difference D, the Euclidean numerator is
    // D_xyz*H_w-H_xyz*D_w. Products N_i*N_j of nonnegative seam basis
    // functions partition unity; symmetrization retains their correlation.
    // Bound extra work independently; larger seams retain the prior bound.
    if n <= 64 {
        let mut numerator = [0_f64; 3];
        for i in 0..n {
            for j in 0..n {
                for axis in 0..3 {
                    let term = delta_first[i][axis]
                        .mul(boundary[j][3])
                        .add(delta_first[j][axis].mul(boundary[i][3]))
                        .sub(boundary[i][axis].mul(delta_first[j][3]))
                        .sub(boundary[j][axis].mul(delta_first[i][3]))
                        .mul(I::point(0.5));
                    numerator[axis] = numerator[axis].max(magnitude(term));
                }
            }
        }
        let correlated = norm(std::array::from_fn(|axis| {
            I::point(numerator[axis]).mul(inv2)
        }));
        if correlated.is_finite() {
            first = first.min(correlated);
        }
    }
    let second = if order == 2 {
        Some(norm(std::array::from_fn(|axis| {
            d(2, axis)
                .mul(inv)
                .add(
                    d(0, axis)
                        .mul(a(2, 3))
                        .add(t(0, axis).mul(d(2, 3)))
                        .mul(inv2),
                )
                .add(
                    two.mul(d(1, axis).mul(a(1, 3)).add(t(1, axis).mul(d(1, 3))))
                        .mul(inv2),
                )
                .add(
                    two.mul(d(0, axis).mul(a(1, 3)).mul(a(1, 3)).add(
                        t(0, axis).mul(two.mul(a(1, 3)).mul(d(1, 3)).add(d(1, 3).mul(d(1, 3)))),
                    ))
                    .mul(inv3),
                )
        })))
    } else {
        None
    };
    let mixed = if order == 2 {
        let derivative_hull = |values: &[H]| -> Result<[f64; 4]> {
            let (p, n, k, _) = r.along(reference);
            let span = I::point(k[n]).sub(I::point(k[p]));
            let mut result = [0_f64; 4];
            for i in 0..n - 1 {
                let factor = I::point(p as f64)
                    .mul(span)
                    .div_positive(I::point(k[i + p + 1]).sub(I::point(k[i + 1])))?;
                for axis in 0..4 {
                    result[axis] = result[axis].max(magnitude(
                        values[i + 1][axis].sub(values[i][axis]).mul(factor),
                    ));
                }
            }
            Ok(result)
        };
        let dd = derivative_hull(&delta_first)?;
        let hd = derivative_hull(&boundary)?;
        Some(norm(std::array::from_fn(|axis| {
            I::point(dd[axis])
                .mul(inv)
                .add(
                    d(1, axis)
                        .mul(I::point(hd[3]))
                        .add(I::point(hd[axis]).mul(d(1, 3)))
                        .add(t(0, axis).mul(I::point(dd[3])))
                        .mul(inv2),
                )
                .add(
                    two.mul(t(0, axis))
                        .mul(d(1, 3))
                        .mul(I::point(hd[3]))
                        .mul(inv3),
                )
        })))
    } else {
        None
    };
    check(
        position.is_finite()
            && first.is_finite()
            && second.is_none_or(|x| x.is_finite())
            && mixed.is_none_or(|x| x.is_finite()),
        "Surface jet error bound overflowed",
    )?;
    Ok(json!({"wholeSeam":true,"normalizedParameters":true,"method":"outward-homogeneous-jet-difference-hull","positionUpper":position,"firstDerivativeUpper":first,"secondDerivativeUpper":second,"mixedDerivativeUpper":mixed}))
}
