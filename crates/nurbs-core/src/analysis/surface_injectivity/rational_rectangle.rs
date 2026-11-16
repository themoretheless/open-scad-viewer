//! Injectivity of a projected rational patch with a rectangular boundary and
//! a strictly positive same-point Jacobian determinant.
use super::*;
use crate::numerics::bernstein_tensor::{Net, derivative, minus, product};

pub(crate) fn certify(s: &Surface, work: &mut usize, limit: usize) -> Option<[usize; 2]> {
    let degree = [s.degree_u, s.degree_v];
    if s.periodic_u
        || s.periodic_v
        || degree.iter().any(|d| !(2..=5).contains(d))
        || s.control_points.len() != degree[0] + 1
        || s.control_points[0].len() != degree[1] + 1
    {
        return None;
    }
    let mut spend = |cost: usize| {
        if cost > limit - *work {
            return None;
        }
        *work += cost;
        Some(())
    };
    for (knots, d) in [(&s.knots_u, degree[0]), (&s.knots_v, degree[1])] {
        spend(1)?;
        if knots
            .iter()
            .enumerate()
            .any(|(i, k)| *k != knots[if i <= d { 0 } else { d + 1 }])
        {
            return None;
        }
    }
    let [m, n] = degree;
    let origin = &s.control_points[0][0];
    let end = [&s.control_points[m][0], &s.control_points[0][n]];
    // A coordinate-aligned rectangle has exactly one changing projected
    // coordinate along each boundary direction. Try the longest chords.
    let axes = end.map(|p| {
        (0..3)
            .max_by(|&a, &b| {
                (p[a] - origin[a])
                    .abs()
                    .total_cmp(&(p[b] - origin[b]).abs())
            })
            .unwrap()
    });
    if axes[0] == axes[1] {
        return None;
    }
    let start = axes.map(|k| origin[k]);
    let end = std::array::from_fn::<_, 2, _>(|i| end[i][axes[i]]);
    if (0..2).any(|i| start[i] == end[i]) {
        return None;
    }
    let sign = std::array::from_fn::<_, 2, _>(|i| if start[i] < end[i] { 1. } else { -1. });
    let ordered = |a: f64, b: f64, axis: usize| if sign[axis] > 0. { a < b } else { a > b };
    let mut weights = Vec::with_capacity(m + 1);
    let mut x = Vec::with_capacity(m + 1);
    let mut y = Vec::with_capacity(m + 1);
    for i in 0..=m {
        spend(1)?;
        let mut wrow = Vec::with_capacity(n + 1);
        let mut xrow = Vec::with_capacity(n + 1);
        let mut yrow = Vec::with_capacity(n + 1);
        for j in 0..=n {
            let a = s.control_points[i][j][axes[0]];
            let b = s.control_points[i][j][axes[1]];
            if ordered(a, start[0], 0)
                || ordered(end[0], a, 0)
                || ordered(b, start[1], 1)
                || ordered(end[1], b, 1)
                || (i == 0 && a != start[0])
                || (i == m && a != end[0])
                || (j == 0 && b != start[1])
                || (j == n && b != end[1])
                || ((j == 0 || j == n)
                    && i > 0
                    && !ordered(s.control_points[i - 1][j][axes[0]], a, 0))
                || ((i == 0 || i == m)
                    && j > 0
                    && !ordered(s.control_points[i][j - 1][axes[1]], b, 1))
            {
                return None;
            }
            let w = I::point(s.weights[i][j]);
            wrow.push(w);
            xrow.push(
                I::point(a)
                    .sub(I::point(start[0]))
                    .ok()?
                    .mul(I::point(sign[0]))
                    .ok()?
                    .mul(w)
                    .ok()?,
            );
            yrow.push(
                I::point(b)
                    .sub(I::point(start[1]))
                    .ok()?
                    .mul(I::point(sign[1]))
                    .ok()?
                    .mul(w)
                    .ok()?,
            );
        }
        weights.push(wrow);
        x.push(xrow);
        y.push(yrow);
    }

    positive_jacobian(&x, &y, &weights, &mut spend)?;
    // Strict rail pole ordering makes every boundary side a homeomorphism.
    // The positive weights and in-rectangle poles send the open parameter
    // rectangle to the open image rectangle. The nonzero Jacobian makes
    // this a proper local diffeomorphism, hence a covering. The target
    // interior is simply connected and the source interior connected, so
    // the covering has one sheet. Boundary and interior images are disjoint.
    Some(axes)
}

/// Sign of the projected rational Jacobian from the original homogeneous nets.
pub(super) fn positive_jacobian(
    x: &Net,
    y: &Net,
    weights: &Net,
    mut spend: impl FnMut(usize) -> Option<()>,
) -> Option<()> {
    // Form det((X,Y,W), (Xu,Yu,Wu), (Xv,Yv,Wv)). Its sign is the
    // projected Jacobian sign because W³ is strictly positive.
    let [xu, xv] = [derivative(x, 0).ok()?, derivative(x, 1).ok()?];
    let [yu, yv] = [derivative(y, 0).ok()?, derivative(y, 1).ok()?];
    let [wu, wv] = [derivative(weights, 0).ok()?, derivative(weights, 1).ok()?];
    let mut mul = |a: &Net, b: &Net| {
        let pairs = a.len() * a[0].len() * b.len() * b[0].len();
        spend(pairs.div_ceil(32))?;
        product(a, b).ok()
    };
    let mut term = |w, a, b, c, d| {
        let left = mul(a, b)?;
        let right = mul(c, d)?;
        mul(w, &minus(left, right).ok()?)
    };
    let base = term(weights, &xu, &yv, &xv, &yu)?;
    let v = term(&wv, &xu, y, x, &yu)?;
    let u = term(&wu, x, &yv, &xv, y)?;
    let determinant = minus(minus(base, v).ok()?, u).ok()?;
    for row in determinant {
        spend(1)?;
        if row.iter().any(|value| value.lo <= 0.) {
            return None;
        }
    }
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn correlated_determinant_proves_a_map_that_independent_columns_refuse() {
        let mut surface = Surface {
            degree_u: 2,
            degree_v: 4,
            knots_u: [vec![0.; 3], vec![1.; 3]].concat(),
            knots_v: [vec![0.; 5], vec![1.; 5]].concat(),
            control_points: (0..=2)
                .map(|u| (0..=4).map(|v| vec![2. * u as f64, 0., v as f64]).collect())
                .collect(),
            weights: vec![vec![1.; 5]; 3],
            periodic_u: false,
            periodic_v: false,
        };
        surface.weights[1][2] = 4.;
        surface.control_points[1][2][2] = 3.5;
        assert!(super::super::rational_rails::certify(&surface, &mut 0, 1000).is_none());
        assert!(super::super::rational_cones::certify(&surface, &mut 0, 1000).is_none());
        assert_eq!(certify(&surface, &mut 0, 1000), Some([0, 2]));
        // A boundary that doubles back cannot supply the homeomorphism.
        surface.control_points[0][1][2] = 20.;
        assert!(surface.evaluate(0., 0.25).unwrap().point[2] > 4.);
        assert!(certify(&surface, &mut 0, 1000).is_none());
    }
}
