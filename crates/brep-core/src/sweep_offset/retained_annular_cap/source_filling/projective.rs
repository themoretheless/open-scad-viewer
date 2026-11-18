//! Positive tensor Möbius reparameterization of the source patch. This changes
//! its parameter correspondence, never its full image or authored coefficients.
use super::{I, Result, Surface, patch_error, patch_error_intervals};

pub(super) struct Bound {
    pub upper: f64,
    pub products: usize,
    pub reparameterized: bool,
}
pub(super) fn inspect(
    weights: &[f64],
    ideal: &[[[f64; 2]; 3]],
    target: &Surface,
    tolerance: f64,
    max_products: usize,
) -> Result<Option<Bound>> {
    let Some((upper, used)) = patch_error(weights, ideal, target, max_products)? else {
        return Ok(None);
    };
    let mut out = Bound {
        upper,
        products: used,
        reparameterized: false,
    };
    if upper <= tolerance {
        return Ok(Some(out));
    }
    let p = target.degree_u;
    if p == 0 || target.degree_v != 1 || weights.len() != 2 * (p + 1) {
        return Ok(Some(out));
    }
    // Float operations only propose arbitrary positive binary64 constants.
    // Every such mu, lambda, eta is a valid exact parameter homeomorphism;
    // their agreement with target ratios is not a certificate premise.
    let mu = target.weights[0][0] / weights[0];
    let lambda = ((target.weights[p][0] / weights[2 * p]) / mu).powf(1. / p as f64);
    let eta = (target.weights[0][1] / weights[1]) / mu;
    if ![mu, lambda, eta].iter().all(|x| x.is_finite() && *x > 0.)
        || [mu, lambda, eta] == [1., 1., 1.]
    {
        return Ok(Some(out));
    }
    let preparation = 5 * (p + 1) + p;
    if preparation + used > max_products - used {
        return Ok(Some(out));
    }
    out.products += preparation;
    let transformed = (|| -> Result<Vec<I>> {
        let mut power = I::point(1.);
        let mut result = Vec::with_capacity(weights.len());
        for i in 0..=p {
            for row in 0..2 {
                let mut w = I::point(weights[2 * i + row])
                    .mul(power)?
                    .mul(I::point(mu))?;
                if row == 1 {
                    w = w.mul(I::point(eta))?;
                }
                result.push(w);
            }
            if i < p {
                power = power.mul(I::point(lambda))?;
            }
        }
        Ok(result)
    })();
    let Ok(transformed) = transformed else {
        return Ok(Some(out));
    };
    // B_i^p(phi_lambda(u)) is proportional to lambda^i B_i^p(u),
    // phi_lambda(u)=lambda*u/(1-u+lambda*u). The analogous V map
    // multiplies the inner column by eta. mu is a common homogeneous scale.
    // Both positive maps are homeomorphisms of [0,1] onto itself. Thus this
    // source tensor patch has exactly the same complete image, including all
    // boundaries, as the source patch whose coverage was proved beforehand.
    out.products += used;
    if let Ok(Some((candidate, charged))) = patch_error_intervals(&transformed, ideal, target, used)
    {
        debug_assert_eq!(charged, used);
        if candidate < out.upper {
            out.upper = candidate;
            out.reparameterized = true;
        }
    }
    Ok(Some(out))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn patch() -> Surface {
        Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![1., 0., 0.], vec![0.5, 0., 0.]],
                vec![vec![1., 1., 0.], vec![0.5, 0.5, 0.]],
                vec![vec![0., 1., 0.], vec![0., 0.5, 0.]],
            ],
            weights: vec![vec![1.; 2]; 3],
            periodic_u: false,
            periodic_v: false,
        }
    }
    fn ideals(s: &Surface) -> Vec<[[f64; 2]; 3]> {
        s.control_points
            .iter()
            .flatten()
            .map(|p| std::array::from_fn(|k| [p[k]; 2]))
            .collect()
    }
    #[test]
    fn exact_dyadic_tensor_reparameterization_preserves_full_image_with_real_budget() {
        let mut s = patch();
        let ideal = ideals(&s);
        let weights = vec![1.; 6];
        s.weights = vec![vec![2., 8.], vec![4., 16.], vec![8., 32.]];
        let raw = patch_error(&weights, &ideal, &s, 36).unwrap().unwrap().0;
        assert!(raw > 0.1);
        let bound = inspect(&weights, &ideal, &s, 1e-10, 1000).unwrap().unwrap();
        assert!(bound.reparameterized && bound.upper < 1e-10);
        assert_eq!(bound.products, 89);
        let short = inspect(&weights, &ideal, &s, 1e-10, bound.products - 1)
            .unwrap()
            .unwrap();
        assert!(!short.reparameterized && short.upper == raw && short.products == 36);
        let mut damaged = s.clone();
        damaged.control_points[1][0][2] = 0.125;
        let moved = inspect(&weights, &ideal, &damaged, 1e-10, 1000)
            .unwrap()
            .unwrap();
        assert!(
            moved.upper > 0.01,
            "A new parameter map must not hide moved geometry"
        );
    }
    #[test]
    fn standard_to_dyadic_circle_weights_and_independent_ring_scale_are_certified() {
        let mut s = patch();
        let ideal = ideals(&s);
        let w = std::f64::consts::FRAC_1_SQRT_2;
        let source = vec![1., 1., w, w, 1., 1.];
        s.weights = vec![vec![1., 4.], vec![1., 4.], vec![2., 8.]];
        let bound = inspect(&source, &ideal, &s, 1e-10, 1000).unwrap().unwrap();
        assert!(bound.reparameterized && bound.upper < 1e-10);
        assert!(bound.products <= 1000);
        assert!(inspect(&source, &ideal, &s, 1e-10, 35).unwrap().is_none());
    }
}
