#[path = "surface/serialization.rs"]
#[cfg(feature = "codec")]
mod serialization;
use crate::curve::{Curve, basis, bounds, stack_basis_validated, validate_basis, validate_periodic_knots};
use crate::{Result, check, numeric};
use math_core::{cross, dot, norm};

#[derive(Clone, Debug, PartialEq)]
pub struct Surface {
    pub degree_u: usize,
    pub degree_v: usize,
    pub knots_u: Vec<f64>,
    pub knots_v: Vec<f64>,
    pub control_points: Vec<Vec<Vec<f64>>>,
    pub weights: Vec<Vec<f64>>,
    pub periodic_u: bool,
    pub periodic_v: bool,
}

#[derive(Clone, Copy)]
pub enum Axis {
    U,
    V,
}

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum DerivativeStatus {Available,InsufficientContinuity,Singular}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum DerivativeSide {TwoSided,Left,Right}

pub struct Evaluation {
    pub point: [f64; 3],
    du: Option<[f64; 3]>,
    dv: Option<[f64; 3]>,
    duu: Option<[f64; 3]>,
    duv: Option<[f64; 3]>,
    dvv: Option<[f64; 3]>,
    normal: Option<[f64; 3]>,
    gaussian_curvature: Option<f64>,
    mean_curvature: Option<f64>,
    derivative_status: &'static str,
    derivative_side_u: &'static str,
    derivative_side_v: &'static str,
    domain_u: [f64; 2],
    domain_v: [f64; 2],
    rounding_bound: Option<f64>,
}
impl Evaluation {
    /// Outward-rounded absolute error evidence for the largest point coordinate.
    /// Available only from the compensated [`SurfaceEvaluator`] path.
    pub fn rounding_bound(&self) -> Option<f64> {
        self.rounding_bound
    }
    /// Gaussian and signed mean curvature, when the surface jet is regular.
    pub fn curvatures(&self)->Option<(f64,f64)> {Some((self.gaussian_curvature?,self.mean_curvature?))}
    pub fn domains(&self)->([f64;2],[f64;2]) {(self.domain_u,self.domain_v)}
    pub fn derivative_status(&self)->DerivativeStatus {
        match self.derivative_status {"available"=>DerivativeStatus::Available,"insufficient_continuity"=>DerivativeStatus::InsufficientContinuity,"singular"=>DerivativeStatus::Singular,_=>unreachable!("internal surface derivative status")}
    }
    pub fn derivative_sides(&self)->(DerivativeSide,DerivativeSide) {
        fn side(s:&str)->DerivativeSide {match s {"two_sided"=>DerivativeSide::TwoSided,"left"=>DerivativeSide::Left,"right"=>DerivativeSide::Right,_=>unreachable!("internal derivative side")}}
        (side(self.derivative_side_u),side(self.derivative_side_v))
    }
    /// Parametric tangents when first derivatives are defined at the query.
    pub fn first_derivatives(&self) -> Option<([f64; 3], [f64; 3])> {
        Some((self.du?, self.dv?))
    }
    /// Second parametric derivatives (uu, uv, vv), when defined.
    pub fn second_derivatives(&self) -> Option<([f64; 3], [f64; 3], [f64; 3])> {
        Some((self.duu?, self.duv?, self.dvv?))
    }
    pub fn unit_normal(&self) -> Option<[f64; 3]> {
        self.normal
    }
}

impl Surface {
    fn axis_curves(&self, axis: Axis) -> Vec<Curve> {
        match axis {
            Axis::U => (0..self.control_points[0].len())
                .map(|v| Curve {
                    degree: self.degree_u,
                    knots: self.knots_u.clone(),
                    control_points: self.control_points.iter().map(|r| r[v].clone()).collect(),
                    weights: self.weights.iter().map(|r| r[v]).collect(),
                    periodic: self.periodic_u,
                })
                .collect(),
            Axis::V => self
                .control_points
                .iter()
                .enumerate()
                .map(|(u, r)| Curve {
                    degree: self.degree_v,
                    knots: self.knots_v.clone(),
                    control_points: r.clone(),
                    weights: self.weights[u].clone(),
                    periodic: self.periodic_v,
                })
                .collect(),
        }
    }
    pub fn validate(&self) -> Result<()> {
        check(
            (2..=256).contains(&self.control_points.len()),
            "NURBS surface requires 2..256 U rows of control points.",
        )?;
        let nv = self.control_points[0].len();
        check(
            (2..=256).contains(&nv) && self.control_points.iter().all(|r| r.len() == nv),
            "NURBS surface control net must be rectangular with 2..256 V points per U row.",
        )?;
        check(
            self.control_points.iter().flatten().all(|p| p.len() == 3),
            "NURBS surface control points must each have exactly three coordinates.",
        )?;
        check(
            self.weights.len() == self.control_points.len()
                && self.weights.iter().all(|r| r.len() == nv),
            "NURBS surface weights must match the rectangular U/V control net.",
        )?;
        for (i, p) in self.control_points.iter().flatten().enumerate() {
            if p.iter().any(|x| !x.is_finite()) {
                return Err(crate::input(format!(
                    "control_points[{i}] must have finite coordinates (no NaN or Inf)"
                )));
            }
        }
        for (i, &w) in self.weights.iter().flatten().enumerate() {
            if !w.is_finite() {
                return Err(crate::input(format!(
                    "weights[{i}] must be positive, finite (no NaN or Inf)"
                )));
            }
        }
        let max = self.weights.iter().flatten().copied().fold(0., f64::max);
        let min = self
            .weights
            .iter()
            .flatten()
            .copied()
            .fold(f64::INFINITY, f64::min);
        check(
            max / min <= 1e12,
            "NURBS surface weight conditioning must not exceed 1e12 across the complete net.",
        )?;
        self.validate_axis_lines(nv)
    }
    fn validate_axis_lines(&self, nv: usize) -> Result<()> {
        let nu = self.control_points.len();
        let domain_u = validate_basis(self.degree_u, &self.knots_u, nu)?;
        for v in 0..nv {
            check(
                (0..nu).all(|u| self.control_points[u][v].iter().all(|x| x.abs() <= 1e9)),
                "Control points must have consistent dimensions and finite coordinates bounded by 1e9",
            )?;
            check(
                (0..nu).all(|u| (1e-12..=1e12).contains(&self.weights[u][v])),
                "Weights must be positive, finite, and in [1e-12, 1e12]",
            )?;
            if self.periodic_u {
                let period_controls = nu - self.degree_u;
                check(
                    period_controls > self.degree_u,
                    "A periodic curve needs at least degree+1 unwrapped control points",
                )?;
                for i in 0..self.degree_u {
                    check(
                        self.weights[i][v] == self.weights[period_controls + i][v]
                            && self.control_points[i][v]
                                == self.control_points[period_controls + i][v],
                        "Periodic curves must explicitly repeat the first degree control points and weights at the end",
                    )?;
                }
                if v == 0 {
                    validate_periodic_knots(&self.knots_u, period_controls, domain_u)?;
                }
            }
        }
        let domain_v = validate_basis(self.degree_v, &self.knots_v, nv)?;
        if self.periodic_v {
            let period_controls = nv - self.degree_v;
            check(
                period_controls > self.degree_v,
                "A periodic curve needs at least degree+1 unwrapped control points",
            )?;
            for u in 0..nu {
                for i in 0..self.degree_v {
                    check(
                        self.weights[u][i] == self.weights[u][period_controls + i]
                            && self.control_points[u][i]
                                == self.control_points[u][period_controls + i],
                        "Periodic curves must explicitly repeat the first degree control points and weights at the end",
                    )?;
                }
                if u == 0 {
                    validate_periodic_knots(&self.knots_v, period_controls, domain_v)?;
                }
            }
        }
        Ok(())
    }
    pub fn evaluate(&self, u: f64, v: f64) -> Result<Evaluation> {
        self.validate()?;
        self.evaluate_validated(u, v)
    }
    pub fn evaluate_validated(&self, u: f64, v: f64) -> Result<Evaluation> {
        let domain_u = [
            self.knots_u[self.degree_u],
            self.knots_u[self.control_points.len()],
        ];
        let domain_v = [
            self.knots_v[self.degree_v],
            self.knots_v[self.control_points[0].len()],
        ];
        let scale = self.weights.iter().flatten().copied().fold(0., f64::max);
        self.evaluate_validated_with(domain_u, domain_v, scale, u, v)
    }
    pub fn evaluate_point_validated_with(
        &self,
        domain_u: [f64; 2],
        domain_v: [f64; 2],
        scale: f64,
        u: f64,
        v: f64,
    ) -> Result<[f64; 3]> {
        Ok(self
            .evaluate_jet_validated_with(domain_u, domain_v, scale, u, v)?
            .point)
    }
    pub fn evaluate_validated_with(
        &self,
        domain_u: [f64; 2],
        domain_v: [f64; 2],
        scale: f64,
        u: f64,
        v: f64,
    ) -> Result<Evaluation> {
        let jet = self.evaluate_jet_validated_with(domain_u, domain_v, scale, u, v)?;
        let su = jet.continuity_u.unwrap_or(2);
        let sv = jet.continuity_v.unwrap_or(2);
        let first = su >= 1 && sv >= 1;
        let second = su >= 2 && sv >= 2;
        let (mut normal, mut gaussian, mut mean) = (None, None, None);
        if first {
            let direction = cross(jet.du, jet.dv);
            let magnitude = norm(direction);
            let speed_u = norm(jet.du);
            let speed_v = norm(jet.dv);
            if magnitude > 1e-12 * speed_u * speed_v && speed_u > 0. && speed_v > 0. {
                let n = direction.map(|val| val / magnitude);
                normal = Some(n);
                if second {
                    let e = dot(jet.du, jet.du);
                    let f = dot(jet.du, jet.dv);
                    let g = dot(jet.dv, jet.dv);
                    let l = dot(n, jet.duu);
                    let m = dot(n, jet.duv);
                    let nn = dot(n, jet.dvv);
                    let determinant = magnitude * magnitude;
                    let k = (l * nn - m * m) / determinant;
                    let h = (e * nn - 2. * f * m + g * l) / (2. * determinant);
                    gaussian = k.is_finite().then_some(k);
                    mean = h.is_finite().then_some(h);
                }
            }
        }
        Ok(Evaluation {
            point: jet.point,
            du: (su >= 1).then_some(jet.du),
            dv: (sv >= 1).then_some(jet.dv),
            duu: (su >= 2).then_some(jet.duu),
            duv: first.then_some(jet.duv),
            dvv: (sv >= 2).then_some(jet.dvv),
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
            derivative_side_u: jet.side_u,
            derivative_side_v: jet.side_v,
            domain_u: jet.domain_u,
            domain_v: jet.domain_v,
            rounding_bound: None,
        })
    }
    fn evaluate_jet_validated_with(
        &self,
        domain_u: [f64; 2],
        domain_v: [f64; 2],
        scale: f64,
        u: f64,
        v: f64,
    ) -> Result<SurfaceJet> {
        let bu = stack_basis_validated(
            self.degree_u,
            &self.knots_u,
            self.control_points.len(),
            domain_u,
            u,
            self.periodic_u,
        )?;
        let bv = stack_basis_validated(
            self.degree_v,
            &self.knots_v,
            self.control_points[0].len(),
            domain_v,
            v,
            self.periodic_v,
        )?;
        let uj = [&bu.basis[..bu.len], &bu.d1[..bu.len], &bu.d2[..bu.len]];
        let vj = [&bv.basis[..bv.len], &bv.d1[..bv.len], &bv.d2[..bv.len]];
        // Use the same active-support origin as SurfaceEvaluator. An inactive
        // corner can introduce cancellation on an otherwise exact boundary.
        let first_u = bu.base
            + bu.basis[..bu.len]
                .iter()
                .position(|value| *value > 0.)
                .ok_or_else(|| crate::numeric_err("Empty basis support"))?;
        let first_v = bv.base
            + bv.basis[..bv.len]
                .iter()
                .position(|value| *value > 0.)
                .ok_or_else(|| crate::numeric_err("Empty basis support"))?;
        let origin = &self.control_points[first_u][first_v];
        let derivatives = [(0, 0), (1, 0), (0, 1), (2, 0), (1, 1), (0, 2)];
        let mut homogeneous = [[0.; 4]; 6];
        for iu_off in 0..bu.len {
            let i = bu.base + iu_off;
            let cp_row = &self.control_points[i];
            let w_row = &self.weights[i];
            for iv_off in 0..bv.len {
                let j = bv.base + iv_off;
                let w_norm = w_row[j] / scale;
                let cp = &cp_row[j];
                let rel = [cp[0] - origin[0], cp[1] - origin[1], cp[2] - origin[2]];
                for (out, (ou, ov)) in homogeneous.iter_mut().zip(derivatives) {
                    let coefficient = uj[ou][iu_off] * vj[ov][iv_off] * w_norm;
                    if coefficient == 0. {
                        continue;
                    }
                    out[3] += coefficient;
                    out[0] += coefficient * rel[0];
                    out[1] += coefficient * rel[1];
                    out[2] += coefficient * rel[2];
                }
            }
        }
        let [h, hu, hv, huu, huv, hvv] = homogeneous;
        numeric(
            h[3].is_finite() && h[3] > 0.,
            "NURBS surface has an invalid rational denominator.",
        )?;
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
                .all(|val| val.is_finite()),
            "NURBS surface evaluation exceeded finite numeric bounds.",
        )?;
        Ok(SurfaceJet {
            point,
            du,
            dv,
            duu,
            duv,
            dvv,
            continuity_u: bu.continuity,
            continuity_v: bv.continuity,
            side_u: bu.derivative_side,
            side_v: bv.derivative_side,
            domain_u: bu.domain,
            domain_v: bv.domain,
        })
    }
    pub fn edit_axis(&self, axis: Axis, edit: impl Fn(&Curve) -> Result<Curve>) -> Result<Self> {
        self.validate()?;
        let curves: Vec<Curve> = self
            .axis_curves(axis)
            .iter()
            .map(edit)
            .collect::<Result<_>>()?;
        let base = &curves[0];
        check(
            curves.iter().all(|c| {
                c.degree == base.degree
                    && c.control_points.len() == base.control_points.len()
                    && c.knots == base.knots
            }),
            "NURBS surface axis edit produced inconsistent parameter lines.",
        )?;
        let result = match axis {
            Axis::U => Self {
                degree_u: base.degree,
                knots_u: base.knots.clone(),
                control_points: (0..base.control_points.len())
                    .map(|u| curves.iter().map(|c| c.control_points[u].clone()).collect())
                    .collect(),
                weights: (0..base.weights.len())
                    .map(|u| curves.iter().map(|c| c.weights[u]).collect())
                    .collect(),
                periodic_u: base.periodic,
                ..self.clone()
            },
            Axis::V => Self {
                degree_v: base.degree,
                knots_v: base.knots.clone(),
                control_points: curves.iter().map(|c| c.control_points.clone()).collect(),
                weights: curves.iter().map(|c| c.weights.clone()).collect(),
                periodic_v: base.periodic,
                ..self.clone()
            },
        };
        result.validate()?;
        Ok(result)
    }
    pub fn trim(&self, b: [f64; 4]) -> Result<Self> {
        check(
            b.iter().all(|v| v.is_finite()) && b[0] < b[1] && b[2] < b[3],
            "NURBS surface trim requires increasing finite U and V intervals.",
        )?;
        self.edit_axis(Axis::U, |c| c.trim(b[0], b[1]))?
            .edit_axis(Axis::V, |c| c.trim(b[2], b[3]))
    }
    pub fn iso(&self, axis: Axis, parameter: f64) -> Result<Curve> {
        self.validate()?;
        let fixed_u = matches!(axis, Axis::U);
        let (fixed, varying, p, k, periodic) = if fixed_u {
            (
                self.control_points.len(),
                self.control_points[0].len(),
                self.degree_u,
                &self.knots_u,
                self.periodic_u,
            )
        } else {
            (
                self.control_points[0].len(),
                self.control_points.len(),
                self.degree_v,
                &self.knots_v,
                self.periodic_v,
            )
        };
        // At a nonperiodic clamped endpoint, the fixed basis selects exactly
        // one original row/column. Preserve definitions without recomputation.
        let a = k[p];
        let z = k[fixed];
        let endpoint = if !periodic && parameter == a && k[..=p].iter().all(|&t| t == a) {
            Some(0)
        } else if !periodic && parameter == z && k[k.len()-p-1..].iter().all(|&t| t == z) {
            Some(fixed-1)
        } else { None };
        if let Some(j) = endpoint {
            let curve = Curve {
                degree: if fixed_u { self.degree_v } else { self.degree_u },
                knots: if fixed_u { self.knots_v.clone() } else { self.knots_u.clone() },
                control_points: (0..varying).map(|i| if fixed_u {
                    self.control_points[j][i].clone()
                } else { self.control_points[i][j].clone() }).collect(),
                weights: (0..varying).map(|i| if fixed_u { self.weights[j][i] } else { self.weights[i][j] }).collect(),
                periodic: if fixed_u { self.periodic_v } else { self.periodic_u },
            };
            curve.validate()?;
            return Ok(curve);
        }
        let b = basis(p, k, fixed, parameter, periodic)?.basis;
        let mut points = Vec::new();
        let mut weights = Vec::new();
        for i in 0..varying {
            let origin = if fixed_u {
                &self.control_points[0][i]
            } else {
                &self.control_points[i][0]
            };
            let mut weighted = [0.; 3];
            let mut weight = 0.;
            for (j, bj) in b.iter().enumerate() {
                let (u, v) = if fixed_u { (j, i) } else { (i, j) };
                let coefficient = bj * self.weights[u][v];
                weight += coefficient;
                for d in 0..3 {
                    weighted[d] += coefficient * (self.control_points[u][v][d] - origin[d]);
                }
            }
            numeric(
                weight.is_finite() && weight > 0.,
                "NURBS iso-curve has an invalid rational denominator.",
            )?;
            points.push((0..3).map(|d| origin[d] + weighted[d] / weight).collect());
            weights.push(weight);
        }
        let c = Curve {
            degree: if fixed_u {
                self.degree_v
            } else {
                self.degree_u
            },
            knots: if fixed_u {
                self.knots_v.clone()
            } else {
                self.knots_u.clone()
            },
            control_points: points,
            weights,
            periodic: if fixed_u {
                self.periodic_v
            } else {
                self.periodic_u
            },
        };
        c.validate()?;
        Ok(c)
    }
    pub fn bounds(&self) -> Result<crate::bounds::Bounds> {
        self.validate()?;
        bounds(
            &self
                .control_points
                .iter()
                .flatten()
                .cloned()
                .collect::<Vec<_>>(),
        )
    }
}
struct SurfaceJet {
    point: [f64; 3],
    du: [f64; 3],
    dv: [f64; 3],
    duu: [f64; 3],
    duv: [f64; 3],
    dvv: [f64; 3],
    continuity_u: Option<i32>,
    continuity_v: Option<i32>,
    side_u: &'static str,
    side_v: &'static str,
    domain_u: [f64; 2],
    domain_v: [f64; 2],
}

#[path = "surface/construction.rs"]
mod construction;
pub use construction::{extrude, loft, loft_aligned, revolve, ruled_surface, sweep};

#[path = "surface/evaluator.rs"]
mod evaluator;
pub use evaluator::{SurfaceEvaluator, SurfaceRefSampler, SurfaceSampler};

#[cfg(test)]
#[path = "surface/construction_tests.rs"]
mod construction_tests;

#[cfg(test)]
#[path = "surface/numeric_tests.rs"]
mod numeric_tests;
