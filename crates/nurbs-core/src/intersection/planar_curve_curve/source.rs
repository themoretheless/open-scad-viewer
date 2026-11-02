//! Bounds are always evaluated on the original curve or surface definition.
use super::*;
use crate::surface::Surface;
use num_rational::BigRational as Q;
use num_traits::{FromPrimitive, ToPrimitive};
pub(super) trait Bounds {
    fn domain(&self) -> [f64; 2];
    fn spans(&self) -> Vec<(usize, [f64; 2])>;
    fn hull(&self, span: usize, d: [f64; 2]) -> Result<Vec<I>>;
    fn derivative(&self, span: usize, d: [f64; 2]) -> Result<Vec<I>>;
}
impl Bounds for Curve {
    fn domain(&self) -> [f64; 2] {
        self.domain()
    }
    fn spans(&self) -> Vec<(usize, [f64; 2])> {
        (self.degree..self.control_points.len())
            .filter(|&i| self.knots[i] < self.knots[i + 1])
            .map(|i| (i, [self.knots[i], self.knots[i + 1]]))
            .collect()
    }
    fn hull(&self, span: usize, d: [f64; 2]) -> Result<Vec<I>> {
        curve_distance::enclosure(self, span, I::new(d[0], d[1])?)
    }
    fn derivative(&self, span: usize, d: [f64; 2]) -> Result<Vec<I>> {
        let j = curve_jets::enclose(self, span, d)?;
        let width = I::point(d[1]).sub(I::point(d[0]))?;
        j[1].iter().map(|x| x.div(width)).collect()
    }
}
pub(super) struct Iso<'a> {
    surface: &'a Surface,
    axis: usize,
    fixed_span: usize,
    fixed: [f64; 2],
}
impl<'a> Iso<'a> {
    pub fn new(surface: &'a Surface, axis: usize) -> Result<Self> {
        let (k, p, n) = if axis == 0 {
            (
                &surface.knots_v,
                surface.degree_v,
                surface.control_points[0].len(),
            )
        } else {
            (
                &surface.knots_u,
                surface.degree_u,
                surface.control_points.len(),
            )
        };
        let q = |x| Q::from_f64(x).unwrap();
        let fixed = (q(k[p]) + q(k[n])) / q(2.);
        let fixed_span = (p..n)
            .find(|&i| q(k[i]) <= fixed && fixed < q(k[i + 1]))
            .ok_or_else(|| crate::input("No original iso span"))?;
        let f = fixed
            .to_f64()
            .ok_or_else(|| crate::numeric_err("No finite iso enclosure"))?;
        let lo = if q(f) > fixed { f.next_down() } else { f };
        let hi = if q(f) < fixed { f.next_up() } else { f };
        check(
            q(lo) <= fixed && fixed <= q(hi) && lo >= k[fixed_span] && hi <= k[fixed_span + 1],
            "Iso enclosure leaves original knot span",
        )?;
        Ok(Self {
            surface,
            axis,
            fixed_span,
            fixed: [lo, hi],
        })
    }
    fn jets(&self, span: usize, d: [f64; 2]) -> Result<([[I; 3]; 3], [usize; 2])> {
        let spans = if self.axis == 0 {
            [span, self.fixed_span]
        } else {
            [self.fixed_span, span]
        };
        let domain = if self.axis == 0 {
            [d, self.fixed]
        } else {
            [self.fixed, d]
        };
        Ok((
            crate::surface_measure::first_jets::calculate(self.surface, spans, domain)?,
            spans,
        ))
    }
}
impl Bounds for Iso<'_> {
    fn domain(&self) -> [f64; 2] {
        let s = self.surface;
        if self.axis == 0 {
            [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]]
        } else {
            [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]]
        }
    }
    fn spans(&self) -> Vec<(usize, [f64; 2])> {
        let s = self.surface;
        let (k, p, n) = if self.axis == 0 {
            (&s.knots_u, s.degree_u, s.control_points.len())
        } else {
            (&s.knots_v, s.degree_v, s.control_points[0].len())
        };
        (p..n)
            .filter(|&i| k[i] < k[i + 1])
            .map(|i| (i, [k[i], k[i + 1]]))
            .collect()
    }
    fn hull(&self, span: usize, d: [f64; 2]) -> Result<Vec<I>> {
        let (j, spans) = self.jets(span, d)?;
        let s = self.surface;
        let origin = &s.control_points[spans[0] - s.degree_u][spans[1] - s.degree_v];
        (0..3).map(|k| j[0][k].add(I::point(origin[k]))).collect()
    }
    fn derivative(&self, span: usize, d: [f64; 2]) -> Result<Vec<I>> {
        let (j, _) = self.jets(span, d)?;
        let k = if self.axis == 0 {
            &self.surface.knots_u
        } else {
            &self.surface.knots_v
        };
        let width = I::point(k[span + 1]).sub(I::point(k[span]))?;
        j[self.axis + 1].iter().map(|x| x.div(width)).collect()
    }
}

/// One fixed projection is proved strictly increasing on both sides of a C0
/// source join. Its coefficients are only a proposal; interval dot products
/// admit it. The shared endpoint is then the only possible common point.
pub(super) fn ordered_join(a: &[I], b: &[I]) -> Result<bool> {
    let direction = |j: &[I]| -> Option<[f64; 3]> {
        let c: [f64; 3] = std::array::from_fn(|i| j[i].lo * 0.5 + j[i].hi * 0.5);
        let scale = c.iter().map(|x| x.abs()).fold(0., f64::max);
        if !scale.is_finite() || scale == 0. {
            return None;
        }
        let c = c.map(|x| x / scale);
        let norm = c.iter().map(|x| x * x).sum::<f64>().sqrt();
        Some(c.map(|x| x / norm))
    };
    let (Some(x), Some(y)) = (direction(a), direction(b)) else {
        return Ok(false);
    };
    let g: [f64; 3] = std::array::from_fn(|i| x[i] + y[i]);
    if !g.iter().all(|x| x.is_finite()) {
        return Ok(false);
    }
    for j in [a, b] {
        let mut dot = I::point(0.);
        for i in 0..3 {
            dot = dot.add(j[i].mul(I::point(g[i]))?)?;
        }
        if dot.lo <= 0. {
            return Ok(false);
        }
    }
    Ok(true)
}
