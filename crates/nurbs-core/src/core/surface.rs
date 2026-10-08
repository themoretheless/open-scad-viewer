#[path = "surface/serialization.rs"]
#[cfg(feature = "codec")]
mod serialization;
use crate::curve::{Curve, basis, bounds};
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
}
impl Evaluation {
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
            (2..=32).contains(&self.control_points.len()),
            "NURBS surface requires 2..32 U rows of control points.",
        )?;
        let nv = self.control_points[0].len();
        check(
            (2..=32).contains(&nv) && self.control_points.iter().all(|r| r.len() == nv),
            "NURBS surface control net must be rectangular with 2..32 V points per U row.",
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
        for axis in [Axis::U, Axis::V] {
            for c in self.axis_curves(axis) {
                c.validate()?;
            }
        }
        Ok(())
    }
    pub fn evaluate(&self, u: f64, v: f64) -> Result<Evaluation> {
        self.validate()?;
        self.evaluate_validated(u, v)
    }
    pub(crate) fn evaluate_validated(&self, u: f64, v: f64) -> Result<Evaluation> {
        let bu = basis(
            self.degree_u,
            &self.knots_u,
            self.control_points.len(),
            u,
            self.periodic_u,
        )?;
        let bv = basis(
            self.degree_v,
            &self.knots_v,
            self.control_points[0].len(),
            v,
            self.periodic_v,
        )?;
        let uj = [&bu.basis, &bu.d1, &bu.d2];
        let vj = [&bv.basis, &bv.d1, &bv.d2];
        let scale = self.weights.iter().flatten().copied().fold(0., f64::max);
        let origin = &self.control_points[0][0];
        let derivatives = [(0, 0), (1, 0), (0, 1), (2, 0), (1, 1), (0, 2)];
        let mut homogeneous = [[0.; 4]; 6];
        for (out, (ou, ov)) in homogeneous.iter_mut().zip(derivatives) {
            for (i, basis_u) in uj[ou].iter().enumerate() {
                for (j, basis_v) in vj[ov].iter().enumerate() {
                    let coefficient = basis_u * basis_v * (self.weights[i][j] / scale);
                    if coefficient == 0. {
                        continue;
                    }
                    out[3] += coefficient;
                    for d in 0..3 {
                        out[d] += coefficient * (self.control_points[i][j][d] - origin[d]);
                    }
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
                .all(|v| v.is_finite()),
            "NURBS surface evaluation exceeded finite numeric bounds.",
        )?;
        let su = bu.continuity.unwrap_or(2);
        let sv = bv.continuity.unwrap_or(2);
        let first = su >= 1 && sv >= 1;
        let second = su >= 2 && sv >= 2;
        let (mut normal, mut gaussian, mut mean) = (None, None, None);
        if first {
            let direction = cross(du, dv);
            let magnitude = norm(direction);
            let speed_u = norm(du);
            let speed_v = norm(dv);
            if magnitude > 1e-12 * speed_u * speed_v && speed_u > 0. && speed_v > 0. {
                let n = direction.map(|v| v / magnitude);
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
            derivative_side_u: bu.derivative_side,
            derivative_side_v: bv.derivative_side,
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
pub fn loft(curves: &[Curve]) -> Result<Surface> {
    check(
        (2..=32).contains(&curves.len()),
        "Loft requires 2..32 compatible curves.",
    )?;
    for c in curves {
        c.validate()?;
    }
    let b = &curves[0];
    check(
        b.control_points[0].len() == 3
            && curves.iter().all(|c| {
                c.degree == b.degree
                    && c.knots == b.knots
                    && c.control_points.len() == b.control_points.len()
                    && c.control_points[0].len() == 3
            }),
        "Loft curves must have identical degree, knots, dimension and control count; refine them first.",
    )?;
    let mut kv = vec![0.];
    kv.extend((0..curves.len()).map(|i| i as f64));
    kv.push((curves.len() - 1) as f64);
    let result = Surface {
        degree_u: b.degree,
        degree_v: 1,
        knots_u: b.knots.clone(),
        knots_v: kv,
        control_points: (0..b.control_points.len())
            .map(|i| curves.iter().map(|c| c.control_points[i].clone()).collect())
            .collect(),
        weights: (0..b.weights.len())
            .map(|i| curves.iter().map(|c| c.weights[i]).collect())
            .collect(),
        periodic_u: b.periodic,
        periodic_v: false,
    };
    result.validate()?;
    Ok(result)
}
pub fn extrude(c: &Curve, vector: [f64; 3]) -> Result<Surface> {
    check(
        vector.iter().all(|v| v.is_finite()) && norm(vector) > 0.,
        "Extrusion vector must be finite and nonzero.",
    )?;
    c.validate()?;
    let mut second = c.clone();
    second.control_points = c
        .control_points
        .iter()
        .map(|p| p.iter().enumerate().map(|(i, x)| x + vector[i]).collect())
        .collect();
    loft(&[c.clone(), second])
}
pub fn revolve(c: &Curve, origin: [f64; 3], axis: [f64; 3], angle: f64) -> Result<Surface> {
    c.validate()?;
    check(
        c.control_points[0].len() == 3
            && origin
                .iter()
                .chain(&axis)
                .chain([angle].iter())
                .all(|v| v.is_finite())
            && norm(axis) > 0.
            && angle != 0.
            && angle.abs() <= 360.,
        "Revolution requires 3D data, a nonzero axis and angle within +/-360 degrees.",
    )?;
    let unit = axis.map(|v| v / norm(axis));
    let arcs = (angle.abs() / 90.).ceil() as usize;
    let delta = angle * std::f64::consts::PI / 180. / arcs as f64;
    let mut kv = vec![0.; 3];
    for i in 1..arcs {
        kv.extend([i as f64, i as f64]);
    }
    kv.extend([arcs as f64; 3]);
    let aw: Vec<f64> = (0..=2 * arcs)
        .map(|j| if j % 2 == 1 { (delta / 2.).cos() } else { 1. })
        .collect();
    let points = c
        .control_points
        .iter()
        .map(|p| {
            let relative: [f64; 3] = std::array::from_fn(|i| p[i] - origin[i]);
            let projection = dot(relative, unit);
            let center: [f64; 3] = std::array::from_fn(|i| origin[i] + projection * unit[i]);
            let radial: [f64; 3] = std::array::from_fn(|i| p[i] - center[i]);
            let perpendicular = cross(unit, radial);
            aw.iter()
                .enumerate()
                .map(|(j, w)| {
                    if j == 2 * arcs && angle.abs() == 360. {
                        return p.clone();
                    }
                    (0..3)
                        .map(|i| {
                            center[i]
                                + (((j as f64 * delta / 2.).cos()) * radial[i]
                                    + ((j as f64 * delta / 2.).sin()) * perpendicular[i])
                                    / w
                        })
                        .collect()
                })
                .collect()
        })
        .collect();
    let result = Surface {
        degree_u: c.degree,
        degree_v: 2,
        knots_u: c.knots.clone(),
        knots_v: kv,
        control_points: points,
        weights: c
            .weights
            .iter()
            .map(|w| aw.iter().map(|v| w * v).collect())
            .collect(),
        periodic_u: c.periodic,
        periodic_v: false,
    };
    result.validate()?;
    Ok(result)
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

/// Exact translational sweep P(u)+Q(v)-Q(v_start). Profile orientation is fixed;
/// this is not a Frenet-frame sweep. Rational data and both parameterizations survive.
pub fn sweep(profile: &Curve, path: &Curve) -> Result<Surface> {
    profile.validate()?;
    path.validate()?;
    check(
        profile.control_points[0].len() == 3 && path.control_points[0].len() == 3,
        "Sweep requires two 3D curves",
    )?;
    let start = path.evaluate(path.domain()[0])?.point;
    let profile_max = profile.weights.iter().copied().fold(0., f64::max);
    let path_max = path.weights.iter().copied().fold(0., f64::max);
    let profile_min = profile
        .weights
        .iter()
        .copied()
        .fold(f64::INFINITY, f64::min);
    let path_min = path.weights.iter().copied().fold(f64::INFINITY, f64::min);
    // Common homogeneous scale does not affect either rational parameterization.
    // Preserve admitted products; otherwise remove only the common source scales.
    let normalize = profile_max * path_max > 1e12 || profile_min * path_min < 1e-12;
    let (profile_scale, path_scale) = if normalize {
        (profile_max, path_max)
    } else {
        (1., 1.)
    };
    let s = Surface {
        degree_u: profile.degree,
        degree_v: path.degree,
        knots_u: profile.knots.clone(),
        knots_v: path.knots.clone(),
        control_points: profile
            .control_points
            .iter()
            .map(|p| {
                path.control_points
                    .iter()
                    .map(|q| (0..3).map(|i| p[i] + (q[i] - start[i])).collect())
                    .collect()
            })
            .collect(),
        weights: profile
            .weights
            .iter()
            .map(|w| {
                path.weights
                    .iter()
                    .map(|v| (w / profile_scale) * (v / path_scale))
                    .collect()
            })
            .collect(),
        periodic_u: profile.periodic,
        periodic_v: path.periodic,
    };
    s.validate()?;
    Ok(s)
}
/// Compatible section construction: normalize domains, elevate degrees and unify
/// knot multiplicities without changing section geometry. V remains piecewise linear.
pub fn loft_aligned(curves: &[Curve]) -> Result<Surface> {
    loft(&crate::sections::compatible(curves)?)
}

#[cfg(test)]
mod construction_tests {
    use super::*;
    #[test]
    fn translational_sweep_retains_small_profile_at_distant_path_origin() {
        let profile = Curve::from_polyline(vec![vec![1e-9, 0., 0.], vec![2e-9, 0., 0.]]).unwrap();
        let path = Curve::from_polyline(vec![vec![1e9, 0., 0.], vec![1e9, 0., 5.]]).unwrap();
        let surface = sweep(&profile, &path).unwrap();
        for (i, p) in profile.control_points.iter().enumerate() {
            assert_eq!(&surface.control_points[i][0], p);
        }
        for u in [0., 0.17, 0.5, 0.83, 1.] {
            for v in [0., 0.23, 0.7, 1.] {
                let p = surface.evaluate(u, v).unwrap().point;
                assert!((p[0] - (1. + u) * 1e-9).abs() < 1e-23);
                assert!((p[2] - 5. * v).abs() < 1e-12);
            }
        }
    }
    #[test]
    fn translational_sweep_admits_extreme_common_weight_scales() {
        let mut profile = Curve::from_polyline(vec![vec![1., 0., 0.], vec![2., 0., 0.]]).unwrap();
        let mut path = Curve::from_polyline(vec![vec![0., 0., 0.], vec![0., 0., 5.]]).unwrap();
        for scale in [1e-12, 1e12] {
            profile.weights.fill(scale);
            path.weights.fill(scale);
            let surface = sweep(&profile, &path).unwrap();
            assert!(surface.weights.iter().flatten().all(|w| *w == 1.));
            let p = surface.evaluate(0.37, 0.61).unwrap().point;
            assert!((p[0] - 1.37).abs() < 1e-12 && (p[2] - 3.05).abs() < 1e-12);
        }
        profile.weights = vec![1e-12, 1.];
        path.weights = vec![1e-12, 1.];
        assert!(sweep(&profile, &path).is_err());
    }
    #[test]
    fn rational_sweep_matches_sum_of_curves() {
        let p = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![1., 0., 0.], vec![1., 1., 0.], vec![0., 1., 0.]],
            weights: vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.],
            periodic: false,
        };
        let q =
            Curve::from_polyline(vec![vec![0.; 3], vec![0., 0., 2.], vec![3., 0., 4.]]).unwrap();
        let s = sweep(&p, &q).unwrap();
        for (u, v) in [(0.2, 0.3), (0.8, 1.5)] {
            let actual = s.evaluate(u, v).unwrap().point;
            let a = p.evaluate(u).unwrap().point;
            let b = q.evaluate(v).unwrap().point;
            for k in 0..3 {
                assert!((actual[k] - a[k] - b[k]).abs() < 1e-10);
            }
        }
    }
    #[test]
    fn loft_aligns_degrees_and_domains() {
        let a = Curve::from_polyline(vec![vec![0.; 3], vec![1., 0., 0.]]).unwrap();
        let mut b = a.elevate(2).unwrap();
        for p in &mut b.control_points {
            p[2] = 3.;
        }
        for k in &mut b.knots {
            *k = 2. + *k * 4.;
        }
        let s = loft_aligned(&[a.clone(), b.clone()]).unwrap();
        for u in [0., 0.2, 0.7, 1.] {
            let x = s.evaluate(u, 0.).unwrap().point;
            let y = s.evaluate(u, 1.).unwrap().point;
            let expected = b.evaluate(2. + u * 4.).unwrap().point;
            for k in 0..3 {
                assert!((x[k] - a.evaluate(u).unwrap().point[k]).abs() < 1e-10);
                assert!((y[k] - expected[k]).abs() < 1e-10);
            }
        }
    }
}
