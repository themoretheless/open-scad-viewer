//! Correlated rational secants from an original natural chart boundary.
use crate::{Result, check, interval_eval::Interval as I, normal_alignment, surface::Surface};
pub struct OriginalBoundarySecant<'a> {
    surface: &'a Surface,
    endpoints: [[f64; 2]; 2],
    axis: usize,
    boundary: f64,
    components: [[f64; 2]; 3],
}
impl OriginalBoundarySecant<'_> {
    pub fn surface(&self) -> &Surface {
        self.surface
    }
    pub fn endpoints(&self) -> [[f64; 2]; 2] {
        self.endpoints
    }
    pub fn axis(&self) -> usize {
        self.axis
    }
    pub fn boundary(&self) -> f64 {
        self.boundary
    }
    pub fn components(&self) -> [[f64; 2]; 3] {
        self.components
    }
}
fn choose(n: usize, k: usize) -> f64 {
    let mut x = 1u64;
    for i in 0..k.min(n - k) {
        x = x * (n - i) as u64 / (i + 1) as u64;
    }
    x as f64
}
/// Enclose (S(u,v)-S(u,b))/(v-b), including its continuous limit at b.
/// The integrated coordinate must have one original clamped Bezier span;
/// the other coordinate may contain multiple original B-spline spans.
/// Every complete original free span is charged. Unsupported layouts refuse.
pub fn boundary_secant_bounds<'a>(
    s: &'a Surface,
    endpoints: [[f64; 2]; 2],
    axis: usize,
    boundary: f64,
    max_spans: usize,
) -> Result<(Option<OriginalBoundarySecant<'a>>, usize)> {
    check(axis < 2, "Choose an original boundary secant coordinate")?;
    normal_alignment::validate_rectangle(s, endpoints, max_spans)?;
    let degrees = [s.degree_u, s.degree_v];
    let counts = [s.control_points.len(), s.control_points[0].len()];
    let knots = [&s.knots_u, &s.knots_v];
    let free = 1 - axis;
    let p = degrees[free];
    let q = degrees[axis];
    let low = knots[axis][q];
    let high = knots[axis][counts[axis]];
    check(
        boundary == low || boundary == high,
        "Use an original natural secant boundary",
    )?;
    if q == 0
        || q > 8
        || p > 8
        || counts[axis] != q + 1
        || s.periodic_u
        || s.periodic_v
        || !knots[axis][..=q].iter().all(|x| *x == low)
        || !knots[axis][q + 1..].iter().all(|x| *x == high)
    {
        return Ok((None, 0));
    }
    let end = boundary == high;
    let boundary_index = if end { q } else { 0 };
    let mut hull = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
    let mut spent = 0;
    for index in p..counts[free] {
        let interval = [knots[free][index], knots[free][index + 1]];
        if interval[0] == interval[1] {
            continue;
        }
        let domain_free = [
            endpoints[free][0].max(interval[0]),
            endpoints[free][1].min(interval[1]),
        ];
        if domain_free[0] > domain_free[1] {
            continue;
        }
        if spent == max_spans {
            return Ok((None, spent));
        }
        let mut indices = [0; 2];
        indices[free] = index;
        indices[axis] = q;
        let mut span = [[0.; 2]; 2];
        span[free] = interval;
        span[axis] = [low, high];
        let extracted = crate::curve_surface_composition::surface_net_on(s, indices, span)?;
        let mut net = vec![vec![[I::point(0.); 4]; q + 1]; p + 1];
        let origin = &s.control_points[0][0];
        for i in 0..=p {
            for j in 0..=q {
                net[i][j] = if axis == 1 {
                    extracted[i][j]
                } else {
                    extracted[j][i]
                };
                for k in 0..3 {
                    net[i][j][k] = net[i][j][k].sub(I::point(origin[k]).mul(net[i][j][3])?)?;
                }
            }
        }
        let basis_span = [interval, [low, high]];
        let domain = [domain_free, endpoints[axis]];
        let values = normal_alignment::restrict_hull(&net, [p, q], basis_span, domain)?;
        let boundary_values = normal_alignment::restrict_hull(
            &net,
            [p, q],
            basis_span,
            [domain_free, [boundary; 2]],
        )?;
        let denominator = values[3].mul(boundary_values[3])?;
        // Q=H(u,v)W(u,b)-H(u,b)W(u,v). Its boundary coefficient is
        // symbolically zero by antisymmetry of the common polynomial product.
        // Omit that coefficient before intervals can lose this dependency.
        let mut divided = vec![vec![[I::point(0.); 4]; q]; 2 * p + 1];
        for i in 0..=p {
            for l in 0..=p {
                let factor = I::point(choose(p, i))
                    .mul(I::point(choose(p, l)))?
                    .div(I::point(choose(2 * p, i + l)))?;
                for j in 0..=q {
                    if j == boundary_index {
                        continue;
                    }
                    let column = if end { j } else { j - 1 };
                    let division =
                        I::point(q as f64).div(I::point(if end { q - j } else { j } as f64))?;
                    for k in 0..3 {
                        let coefficient = net[i][j][k]
                            .mul(net[l][boundary_index][3])?
                            .sub(net[i][boundary_index][k].mul(net[l][j][3])?)?;
                        divided[i + l][column][k] = divided[i + l][column][k]
                            .add(coefficient.mul(factor)?.mul(division)?)?;
                    }
                }
            }
        }
        let numerator =
            normal_alignment::restrict_hull(&divided, [2 * p, q - 1], basis_span, domain)?;
        let length = I::point(high).sub(I::point(low))?;
        let scale = I::point(if end { -1. } else { 1. }).div(length)?;
        for k in 0..3 {
            let value = numerator[k].mul(scale)?.div(denominator)?;
            hull[k][0] = hull[k][0].min(value.lo);
            hull[k][1] = hull[k][1].max(value.hi);
        }
        spent += 1;
    }
    if spent == 0 {
        return Ok((None, 0));
    }
    Ok((
        Some(OriginalBoundarySecant {
            surface: s,
            endpoints,
            axis,
            boundary,
            components: hull,
        }),
        spent,
    ))
}
#[cfg(test)]
mod tests {
    use super::*;
    fn surface() -> Surface {
        Surface {
            degree_u: 1,
            degree_v: 2,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 0., 0.], vec![0., 1., 0.]],
                vec![vec![1., 0., 0.], vec![1., 0., 0.], vec![1., 1., 0.]],
            ],
            weights: vec![vec![1.; 3]; 2],
            periodic_u: false,
            periodic_v: false,
        }
    }
    #[test]
    fn polynomial_and_rational_secants_include_limits_and_analytic_oracle() {
        for rational in [false, true] {
            let mut s = surface();
            if rational {
                s.weights = vec![vec![1., 0.7, 1.]; 2];
            }
            for boundary in [0., 1.] {
                let endpoints = [[0.25, 0.25], [0., 0.2]];
                let (proof, spent) = boundary_secant_bounds(&s, endpoints, 1, boundary, 1).unwrap();
                let proof = proof.unwrap();
                assert_eq!(spent, 1);
                assert!(std::ptr::eq(proof.surface(), &s));
                assert_eq!(proof.endpoints(), endpoints);
                let r = proof.components()[1];
                for i in 0..=20 {
                    let v = i as f64 * 0.01;
                    let w = if rational { 0.7 } else { 1. };
                    let d = (1. - v) * (1. - v) + 2. * w * v * (1. - v) + v * v;
                    let value = v * v / d;
                    let expected = if boundary == 1. {
                        (value - 1.) / (v - 1.)
                    } else if v == 0. {
                        0.
                    } else {
                        value / v
                    };
                    assert!(
                        r[0] <= expected && expected <= r[1],
                        "{r:?} misses {expected}"
                    );
                }
                if !rational && boundary == 1. {
                    assert!(r[1] - r[0] < 0.201);
                }
            }
        }
    }
    #[test]
    fn transpose_budget_and_invalid_source_guards() {
        let mut s = surface();
        let original = boundary_secant_bounds(&s, [[0.2, 0.2], [0.3, 0.3]], 1, 1., 1)
            .unwrap()
            .0
            .unwrap()
            .components();
        s.control_points = (0..3)
            .map(|v| (0..2).map(|u| s.control_points[u][v].clone()).collect())
            .collect();
        s.weights = vec![vec![1.; 2]; 3];
        std::mem::swap(&mut s.degree_u, &mut s.degree_v);
        std::mem::swap(&mut s.knots_u, &mut s.knots_v);
        let transposed = boundary_secant_bounds(&s, [[0.3, 0.3], [0.2, 0.2]], 0, 1., 1)
            .unwrap()
            .0
            .unwrap()
            .components();
        assert_eq!(original, transposed);
        assert!(boundary_secant_bounds(&s, [[0.3, 0.3], [0.2, 0.2]], 0, 0.5, 1).is_err());
        assert!(boundary_secant_bounds(&s, [[0.3, 0.3], [0.2, 0.2]], 0, 1., 0).is_err());
    }
    #[test]
    fn every_free_knot_span_must_be_covered() {
        let mut s = surface();
        s.knots_u = vec![0., 0., 0.5, 1., 1.];
        s.control_points.insert(
            1,
            vec![vec![0.5, 0., 0.], vec![0.5, 0., 0.], vec![0.5, 1., 0.]],
        );
        s.weights = vec![vec![1.; 3]; 3];
        let endpoints = [[0.2, 0.8], [0.1, 0.2]];
        let (missing, spent) = boundary_secant_bounds(&s, endpoints, 1, 1., 1).unwrap();
        assert!(missing.is_none());
        assert_eq!(spent, 1);
        let (proof, spent) = boundary_secant_bounds(&s, endpoints, 1, 1., 2).unwrap();
        let r = proof.unwrap().components()[1];
        assert_eq!(spent, 2);
        assert!(r[0] <= 1.1 && r[1] >= 1.2);
        assert!(
            boundary_secant_bounds(&s, endpoints, 0, 0., 2)
                .unwrap()
                .0
                .is_none()
        );
    }
}
