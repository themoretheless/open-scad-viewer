//! Validate-once evaluation and immutable sampling. Public paths remain in surface.
use super::*;
use crate::curve::{
    CompensatedSum, basis_funs_ders, continuity_meta, find_span_hinted, rounding_bound,
};

/// Validate-once surface evaluator with cached U/V span hints and compensated
/// local-basis accumulation. The borrowed definition is never copied.
pub struct SurfaceEvaluator<'a> {
    surface: &'a Surface,
    domain_u: [f64; 2],
    domain_v: [f64; 2],
    scale: f64,
    span_u: usize,
    span_v: usize,
}
impl<'a> SurfaceEvaluator<'a> {
    pub fn new(surface: &'a Surface) -> Result<Self> {
        surface.validate()?;
        Ok(Self {
            surface,
            domain_u: [
                surface.knots_u[surface.degree_u],
                surface.knots_u[surface.control_points.len()],
            ],
            domain_v: [
                surface.knots_v[surface.degree_v],
                surface.knots_v[surface.control_points[0].len()],
            ],
            scale: surface.weights.iter().flatten().copied().fold(0., f64::max),
            span_u: surface.degree_u,
            span_v: surface.degree_v,
        })
    }
    pub fn definition(&self) -> &Surface {
        self.surface
    }
    pub fn evaluate(&mut self, u: f64, v: f64) -> Result<Evaluation> {
        check(
            u.is_finite()
                && u >= self.domain_u[0]
                && u <= self.domain_u[1]
                && v.is_finite()
                && v >= self.domain_v[0]
                && v <= self.domain_v[1],
            "Parameter is outside the active knot domain",
        )?;
        let pu = self.surface.degree_u;
        let pv = self.surface.degree_v;
        let nu = self.surface.control_points.len();
        let nv = self.surface.control_points[0].len();
        let span_u = find_span_hinted(pu, &self.surface.knots_u, nu, u, &mut self.span_u)?;
        let span_v = find_span_hinted(pv, &self.surface.knots_v, nv, v, &mut self.span_v)?;
        let bu = basis_funs_ders(pu, &self.surface.knots_u, span_u, u, 2)?;
        let bv = basis_funs_ders(pv, &self.surface.knots_v, span_v, v, 2)?;
        let first_u = bu
            .values
            .iter()
            .position(|value| *value > 0.)
            .ok_or_else(|| crate::numeric_err("Empty basis support"))?;
        let first_v = bv
            .values
            .iter()
            .position(|value| *value > 0.)
            .ok_or_else(|| crate::numeric_err("Empty basis support"))?;
        let origin = &self.surface.control_points[span_u - pu + first_u][span_v - pv + first_v];
        let uj = [&bu.values, &bu.d1, &bu.d2];
        let vj = [&bv.values, &bv.d1, &bv.d2];
        let derivatives = [(0, 0), (1, 0), (0, 1), (2, 0), (1, 1), (0, 2)];
        let mut homogeneous: [[CompensatedSum; 4]; 6] =
            std::array::from_fn(|_| std::array::from_fn(|_| CompensatedSum::new()));
        for (out, (ou, ov)) in homogeneous.iter_mut().zip(derivatives) {
            for (j, basis_u) in uj[ou].iter().enumerate() {
                let iu = span_u - pu + j;
                for (k, basis_v) in vj[ov].iter().enumerate() {
                    let iv = span_v - pv + k;
                    let coefficient =
                        basis_u * basis_v * (self.surface.weights[iu][iv] / self.scale);
                    if coefficient == 0. {
                        continue;
                    }
                    out[3].add(coefficient);
                    for d in 0..3 {
                        out[d].add(
                            coefficient * (self.surface.control_points[iu][iv][d] - origin[d]),
                        );
                    }
                }
            }
        }
        let h: [[f64; 4]; 6] =
            std::array::from_fn(|i| std::array::from_fn(|j| homogeneous[i][j].value()));
        let [h, hu, hv, huu, huv, hvv] = h;
        numeric(
            h[3].is_finite() && h[3] > 0.,
            "NURBS surface has an invalid rational denominator.",
        )?;
        let rounding = rounding_bound(&homogeneous[0][..3], h[3]);
        let relative: [f64; 3] = std::array::from_fn(|i| h[i] / h[3]);
        let point = std::array::from_fn(|i| relative[i] + origin[i]);
        let du: [f64; 3] = std::array::from_fn(|i| (hu[i] - hu[3] * relative[i]) / h[3]);
        let dv: [f64; 3] = std::array::from_fn(|i| (hv[i] - hv[3] * relative[i]) / h[3]);
        let duu =
            std::array::from_fn(|i| (huu[i] - 2. * hu[3] * du[i] - huu[3] * relative[i]) / h[3]);
        let duv = std::array::from_fn(|i| {
            (huv[i] - hu[3] * dv[i] - hv[3] * du[i] - huv[3] * relative[i]) / h[3]
        });
        let dvv =
            std::array::from_fn(|i| (hvv[i] - 2. * hv[3] * dv[i] - hvv[3] * relative[i]) / h[3]);
        numeric(
            [point, du, dv, duu, duv, dvv]
                .iter()
                .flatten()
                .all(|value| value.is_finite()),
            "NURBS surface evaluation exceeded finite numeric bounds.",
        )?;
        let (su, _, side_u) = continuity_meta(
            pu,
            &self.surface.knots_u,
            self.domain_u,
            u,
            self.surface.periodic_u,
        );
        let (sv, _, side_v) = continuity_meta(
            pv,
            &self.surface.knots_v,
            self.domain_v,
            v,
            self.surface.periodic_v,
        );
        let su = su.unwrap_or(2);
        let sv = sv.unwrap_or(2);
        let first = su >= 1 && sv >= 1;
        let second = su >= 2 && sv >= 2;
        let (mut normal, mut gaussian, mut mean) = (None, None, None);
        if first {
            let direction = cross(du, dv);
            let magnitude = norm(direction);
            let speed_u = norm(du);
            let speed_v = norm(dv);
            if magnitude > 1e-12 * speed_u * speed_v && speed_u > 0. && speed_v > 0. {
                let n = direction.map(|value| value / magnitude);
                normal = Some(n);
                if second {
                    let e = dot(du, du);
                    let f = dot(du, dv);
                    let g = dot(dv, dv);
                    let l = dot(n, duu);
                    let m = dot(n, duv);
                    let nn = dot(n, dvv);
                    let determinant = magnitude * magnitude;
                    let k = (l * nn - m * m) / determinant;
                    let h = (e * nn - 2. * f * m + g * l) / (2. * determinant);
                    gaussian = k.is_finite().then_some(k);
                    mean = h.is_finite().then_some(h);
                }
            }
        }
        Ok(Evaluation {
            point,
            du: (su >= 1).then_some(du),
            dv: (sv >= 1).then_some(dv),
            duu: (su >= 2).then_some(duu),
            duv: first.then_some(duv),
            dvv: (sv >= 2).then_some(dvv),
            normal,
            gaussian_curvature: gaussian,
            mean_curvature: mean,
            derivative_status: if !first || !second {
                "insufficient_continuity"
            } else if normal.is_none() {
                "singular"
            } else {
                "available"
            },
            derivative_side_u: side_u,
            derivative_side_v: side_v,
            domain_u: self.domain_u,
            domain_v: self.domain_v,
            rounding_bound: Some(rounding),
        })
    }
}

/// Validated immutable snapshot for repeated sampling by any downstream adapter.
#[derive(Clone)]
pub struct SurfaceSampler {
    surface: Surface,
}
impl SurfaceSampler {
    pub fn new(surface: &Surface) -> Result<Self> {
        surface.validate()?;
        Ok(Self {
            surface: surface.clone(),
        })
    }
    pub fn evaluate(&self, u: f64, v: f64) -> Result<Evaluation> {
        self.surface.evaluate_validated(u, v)
    }
    pub fn definition(&self) -> &Surface {
        &self.surface
    }
}
