//! Validate-once evaluation and immutable sampling. Public paths remain in `curve`.
use super::{
    CompensatedSum, Curve, Evaluation, basis_funs_ders, continuity_meta, find_span, rounding_bound,
};
use crate::{Result, check, numeric, numeric_err};

/// Borrowed validate-once curve sampler (`&self` queries, bit-exact with
/// [`Curve::evaluate`], zero heap allocations on fixed-dimension point queries).
#[derive(Clone, Copy)]
pub struct CurveSampler<'a> {
    pub(super) curve: &'a Curve,
    pub(super) domain: [f64; 2],
    pub(super) scale: f64,
    pub(super) dim: usize,
}
impl<'a> CurveSampler<'a> {
    pub fn new(curve: &'a Curve) -> Result<Self> {
        curve.validate()?;
        Ok(Self {
            curve,
            domain: curve.domain(),
            scale: curve.weights.iter().copied().fold(0., f64::max),
            dim: curve.control_points[0].len(),
        })
    }
    pub fn definition(&self) -> &'a Curve {
        self.curve
    }
    pub fn domain(&self) -> [f64; 2] {
        self.domain
    }
    pub fn dimension(&self) -> usize {
        self.dim
    }
    pub fn evaluate(&self, u: f64) -> Result<Evaluation> {
        self.curve.evaluate_validated_with(self.domain, self.scale, u)
    }
    pub fn point2(&self, u: f64) -> Result<[f64; 2]> {
        check(self.dim == 2, "CurveSampler::point2 requires a 2D curve")?;
        let eval = self.curve.evaluate_stack_validated(self.domain, self.scale, u)?;
        Ok([eval.point[0], eval.point[1]])
    }
    pub fn point3(&self, u: f64) -> Result<[f64; 3]> {
        check(self.dim == 3, "CurveSampler::point3 requires a 3D curve")?;
        Ok(self.curve.evaluate_stack_validated(self.domain, self.scale, u)?.point)
    }
    pub fn point_at_fraction2(&self, fraction: f64) -> Result<[f64; 2]> {
        self.point2(self.fraction_to_parameter(fraction))
    }
    pub fn point_at_fraction3(&self, fraction: f64) -> Result<[f64; 3]> {
        self.point3(self.fraction_to_parameter(fraction))
    }
    fn fraction_to_parameter(&self, fraction: f64) -> f64 {
        if fraction == 0. {
            self.domain[0]
        } else if fraction == 1. {
            self.domain[1]
        } else {
            self.domain[0] + fraction * (self.domain[1] - self.domain[0])
        }
    }
}

/// Validate-once curve evaluator: `new()` runs `Curve::validate()` a single
/// time, then `evaluate()` reuses a cached span hint (monotone sampling fast
/// path) and the O(p²) local basis with no full-width allocations.
pub struct CurveEvaluator<'a> {
    curve: &'a Curve,
    domain: [f64; 2],
    scale: f64,
    span: usize,
}
impl<'a> CurveEvaluator<'a> {
    pub fn new(curve: &'a Curve) -> Result<Self> {
        curve.validate()?;
        Ok(Self {
            curve,
            domain: curve.domain(),
            scale: curve.weights.iter().copied().fold(0., f64::max),
            span: curve.degree,
        })
    }
    fn locate(&mut self, u: f64) -> Result<usize> {
        let p = self.curve.degree;
        let n = self.curve.control_points.len();
        if u == self.domain[1] {
            self.span = n - 1;
            return Ok(n - 1);
        }
        let k = &self.curve.knots;
        let s = self.span;
        if s >= p && s + 1 < k.len() && u >= k[s] && u < k[s + 1] {
            return Ok(s);
        }
        let s = find_span(p, k, n, u)?;
        self.span = s;
        Ok(s)
    }
    pub fn evaluate(&mut self, u: f64) -> Result<Evaluation> {
        check(
            u.is_finite() && u >= self.domain[0] && u <= self.domain[1],
            "Parameter is outside the active knot domain",
        )?;
        let span = self.locate(u)?;
        let p = self.curve.degree;
        let local = basis_funs_ders(p, &self.curve.knots, span, u, 2)?;
        let dim = self.curve.control_points[0].len();
        let first = local
            .values
            .iter()
            .position(|v| *v > 0.)
            .ok_or_else(|| numeric_err("Empty basis support"))?;
        let origin = &self.curve.control_points[span - p + first];
        let mut weight = CompensatedSum::new();
        let mut w1 = CompensatedSum::new();
        let mut w2 = CompensatedSum::new();
        let mut point: [CompensatedSum; 3] = std::array::from_fn(|_| CompensatedSum::new());
        let mut d1: [CompensatedSum; 3] = std::array::from_fn(|_| CompensatedSum::new());
        let mut d2: [CompensatedSum; 3] = std::array::from_fn(|_| CompensatedSum::new());
        for j in 0..=p {
            let i = span - p + j;
            let w = self.curve.weights[i] / self.scale;
            weight.add(local.values[j] * w);
            w1.add(local.d1[j] * w);
            w2.add(local.d2[j] * w);
            for axis in 0..dim {
                let coordinate = self.curve.control_points[i][axis] - origin[axis];
                point[axis].add(local.values[j] * w * coordinate);
                d1[axis].add(local.d1[j] * w * coordinate);
                d2[axis].add(local.d2[j] * w * coordinate);
            }
        }
        let weight = weight.value();
        let w1 = w1.value();
        let w2 = w2.value();
        let rounding = rounding_bound(&point[..dim], weight);
        let mut point: Vec<f64> = point[..dim].iter().map(|a| a.value()).collect();
        let mut d1: Vec<f64> = d1[..dim].iter().map(|a| a.value()).collect();
        let mut d2: Vec<f64> = d2[..dim].iter().map(|a| a.value()).collect();
        numeric(
            weight > 0. && weight.is_finite(),
            "Rational denominator lost its positive finite value",
        )?;
        for axis in 0..dim {
            point[axis] /= weight;
            d1[axis] = (d1[axis] - w1 * point[axis]) / weight;
            d2[axis] = (d2[axis] - 2. * w1 * d1[axis] - w2 * point[axis]) / weight;
            point[axis] += origin[axis];
        }
        numeric(
            point.iter().chain(&d1).chain(&d2).all(|v| v.is_finite()),
            "Rational evaluation exhausted finite precision",
        )?;
        let (continuity, status, side) =
            continuity_meta(p, &self.curve.knots, self.domain, u, self.curve.periodic);
        Ok(Evaluation {
            point,
            d1: (continuity.unwrap_or(2) >= 1).then_some(d1),
            d2: (continuity.unwrap_or(2) >= 2).then_some(d2),
            domain: self.domain,
            continuity,
            derivative_status: status,
            derivative_side: side,
            rounding_bound: rounding,
        })
    }
}

#[cfg(test)]
mod hinted_span_tests {
    use super::super::{find_span, find_span_hinted};

    fn lcg(state: &mut u64) -> f64 {
        *state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (*state >> 11) as f64 / (1u64 << 53) as f64
    }

    #[test]
    fn hinted_matches_binary_search_on_random_inputs() {
        let degree = 3;
        let knots = [0., 0., 0., 0., 1., 2., 2., 3., 4., 4., 4., 4.];
        let n = 8;
        let mut state = 0xDEADBEEFCAFEF00D;
        for case in 0..2000 {
            let u = 4. * lcg(&mut state);
            let u = match case % 5 {
                0 => 0.,
                1 => 4.,
                2 => 2.,
                _ => u,
            };
            let mut hint = (lcg(&mut state) * 1000.) as usize;
            let expected = find_span(degree, &knots, n, u).unwrap();
            let got = find_span_hinted(degree, &knots, n, u, &mut hint).unwrap();
            assert_eq!(got, expected, "u={u} initial hint mismatch");
            assert_eq!(hint, expected, "hint must be updated to the span");
        }
    }

    #[test]
    fn hinted_monotone_sequence_is_stable() {
        let degree = 2;
        let knots = [0., 0., 0., 1., 2., 3., 3., 3.];
        let n = 5;
        let mut hint = usize::MAX;
        for i in 0..=64 {
            let u = 3. * i as f64 / 64.;
            let got = find_span_hinted(degree, &knots, n, u, &mut hint).unwrap();
            assert_eq!(got, find_span(degree, &knots, n, u).unwrap());
        }
    }

    #[test]
    fn hinted_rejects_invalid_arguments() {
        let knots = [0., 0., 0., 1., 1., 1.];
        let mut hint = 0;
        assert!(find_span_hinted(0, &knots, 3, 0.5, &mut hint).is_err());
        assert!(find_span_hinted(2, &knots, 2, 0.5, &mut hint).is_err());
        assert!(find_span_hinted(2, &knots[..5], 3, 0.5, &mut hint).is_err());
        assert!(find_span_hinted(2, &knots, 3, -0.5, &mut hint).is_err());
        assert!(find_span_hinted(2, &knots, 3, f64::NAN, &mut hint).is_err());
    }
}
