//! Original world-space Lipschitz bound for T(x,y,z)=(x,y,z-H(x,y)).
//! Positive rational hulls of BOTH original supports bound the entire segment
//! joining S1(P1) to S2(P2), including images outside the first UV rectangle.
use super::*;
use num_traits::ToPrimitive;
fn derivative_world(height: &P, inverse: &[[Q; 3]; 2], axis: usize) -> P {
    let mut gradient = P::new();
    for (e, c) in height {
        for a in 0..2 {
            if e[a] > 0 {
                let mut power = *e;
                power[a] -= 1;
                add(
                    &mut gradient,
                    power,
                    c * Q::from_integer(e[a].into()) * &inverse[a][axis + 1],
                );
            }
        }
    }
    gradient
}
/// The checked family has at most 81 poles per support and degree <=8/axis.
/// Exact finite polynomial arithmetic supplies the bound; sampled Jacobians,
/// extracted rounded poles and inverse-transformed error guesses do not.
pub(in crate::intersection::ss_intersection) fn graph_shear_world_gain(
    first: &Surface,
    second: &Surface,
) -> Result<f64> {
    admit(first)?;
    admit(second)?;
    let inverse = inverse_xy(first)?;
    let height = bernstein_power(first, 2)?;
    let mut radius = [Q::one(), Q::one()];
    for s in [first, second] {
        for pole in s.control_points.iter().flatten() {
            let x = q(pole[0])?;
            let y = q(pole[1])?;
            for a in 0..2 {
                let value = (&inverse[a][0] + &inverse[a][1] * &x + &inverse[a][2] * &y).abs();
                radius[a] = radius[a].clone().max(value);
            }
        }
    }
    // In infinity norm the first two rows of DT have norm one; the third
    // row is (-Hx,-Hy,1). Exact coefficient cancellation precedes abs bounds.
    let mut bound = Q::one();
    for axis in 0..2 {
        for (e, c) in derivative_world(&height, &inverse, axis) {
            bound += c.abs() * radius[0].pow(e[0] as i32) * radius[1].pow(e[1] as i32);
        }
    }
    let rounded = bound
        .to_f64()
        .filter(|x| x.is_finite())
        .ok_or_else(|| crate::input("No finite original shear error gain"))?;
    let outward = if q(rounded)? < bound {
        rounded.next_up()
    } else {
        rounded
    };
    check(
        outward.is_finite() && outward >= 1.,
        "Original shear error gain overflowed",
    )?;
    Ok(outward)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn surface() -> Surface {
        Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![2., 2., 2., 6., 6., 6.],
            knots_v: vec![-2., -2., 2., 2.],
            control_points: (0..3)
                .map(|i| {
                    (0..2)
                        .map(|j| vec![i as f64 / 2., j as f64, if i == 2 { 1. } else { 0. }])
                        .collect()
                })
                .collect(),
            weights: vec![vec![3.; 2]; 3],
            periodic_u: false,
            periodic_v: false,
        }
    }
    #[test]
    fn both_original_footprints_and_world_inverse_control_the_gain() {
        let a = surface();
        assert_eq!(graph_shear_world_gain(&a, &a).unwrap(), 3.);
        let mut b = a.clone();
        for p in b.control_points.iter_mut().flatten() {
            p[0] *= 4.;
        }
        assert_eq!(graph_shear_world_gain(&a, &b).unwrap(), 9.);
        let mut a = a;
        for p in a.control_points.iter_mut().flatten() {
            p[0] *= 4.;
            p[1] *= 0.25;
        }
        assert_eq!(graph_shear_world_gain(&a, &a).unwrap(), 1.5);
        let mut damaged = a.clone();
        damaged.control_points[1][0][0] = damaged.control_points[1][0][0].next_up();
        assert!(graph_shear_world_gain(&damaged, &a).is_err());
        let mut huge = surface();
        for p in huge.control_points.iter_mut().flatten() {
            p[2] *= f64::MAX / 2.;
        }
        assert!(graph_shear_world_gain(&huge, &huge).is_err());
    }
    #[test]
    fn coupled_polynomials_and_nonbinary_upper_rounding_are_exact() {
        let mut s = surface();
        s.degree_v = 2;
        s.knots_v = vec![0., 0., 0., 1., 1., 1.];
        s.control_points = (0..3)
            .map(|i| {
                (0..3)
                    .map(|j| {
                        vec![
                            i as f64 / 2.,
                            j as f64 / 2.,
                            if i == 2 && j == 2 { 1. } else { 0. },
                        ]
                    })
                    .collect()
            })
            .collect();
        s.weights = vec![vec![7.; 3]; 3];
        assert_eq!(graph_shear_world_gain(&s, &s).unwrap(), 5.);
        // x=3u makes Hx=2u/3 and the exact norm bound is 5/3.
        let mut s = surface();
        for p in s.control_points.iter_mut().flatten() {
            p[0] *= 3.;
        }
        let upper = graph_shear_world_gain(&s, &s).unwrap();
        let exact = Q::new(5.into(), 3.into());
        assert!(q(upper).unwrap() >= exact);
        assert!(q(upper.next_down()).unwrap() < exact);
    }
}
