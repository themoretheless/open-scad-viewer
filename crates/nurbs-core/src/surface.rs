use crate::curve::{Curve, basis, bounds};
use crate::{Result, check, numeric};
use math_core::{cross, dot, norm};

#[derive(Clone, Debug)]
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
impl value_codec::Serialize for Surface {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert(
            "degreeU".into(),
            value_codec::Serialize::to_value(&self.degree_u),
        );
        object.insert(
            "degreeV".into(),
            value_codec::Serialize::to_value(&self.degree_v),
        );
        object.insert(
            "knotsU".into(),
            value_codec::Serialize::to_value(&self.knots_u),
        );
        object.insert(
            "knotsV".into(),
            value_codec::Serialize::to_value(&self.knots_v),
        );
        object.insert(
            "controlPoints".into(),
            value_codec::Serialize::to_value(&self.control_points),
        );
        object.insert(
            "weights".into(),
            value_codec::Serialize::to_value(&self.weights),
        );
        object.insert(
            "periodicU".into(),
            value_codec::Serialize::to_value(&self.periodic_u),
        );
        object.insert(
            "periodicV".into(),
            value_codec::Serialize::to_value(&self.periodic_v),
        );
        value_codec::Value::Object(object)
    }
}
impl<'de> value_codec::Deserialize<'de> for Surface {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let mut object = value
            .as_object()
            .ok_or_else(|| value_codec::error("Expected object"))?
            .clone();
        let degree_u: usize = value_codec::Deserialize::from_value(
            object
                .remove("degreeU")
                .ok_or_else(|| value_codec::error("Missing field degreeU"))?,
        )?;
        let degree_v: usize = value_codec::Deserialize::from_value(
            object
                .remove("degreeV")
                .ok_or_else(|| value_codec::error("Missing field degreeV"))?,
        )?;
        let knots_u: Vec<f64> = value_codec::Deserialize::from_value(
            object
                .remove("knotsU")
                .ok_or_else(|| value_codec::error("Missing field knotsU"))?,
        )?;
        let knots_v: Vec<f64> = value_codec::Deserialize::from_value(
            object
                .remove("knotsV")
                .ok_or_else(|| value_codec::error("Missing field knotsV"))?,
        )?;
        let control_points: Vec<Vec<Vec<f64>>> = value_codec::Deserialize::from_value(
            object
                .remove("controlPoints")
                .ok_or_else(|| value_codec::error("Missing field controlPoints"))?,
        )?;
        let weights: Vec<Vec<f64>> = value_codec::Deserialize::from_value(
            object
                .remove("weights")
                .ok_or_else(|| value_codec::error("Missing field weights"))?,
        )?;
        let periodic_u: bool = if let Some(v) = object.remove("periodicU") {
            value_codec::Deserialize::from_value(v)?
        } else {
            Default::default()
        };
        let periodic_v: bool = if let Some(v) = object.remove("periodicV") {
            value_codec::Deserialize::from_value(v)?
        } else {
            Default::default()
        };
        Ok(Self {
            degree_u,
            degree_v,
            knots_u,
            knots_v,
            control_points,
            weights,
            periodic_u,
            periodic_v,
        })
    }
}
#[derive(Clone, Copy)]
pub enum Axis {
    U,
    V,
}
impl<'de> value_codec::Deserialize<'de> for Axis {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        match value.as_str().unwrap_or("") {
            "u" => Ok(Self::U),
            "v" => Ok(Self::V),
            _ => Err(value_codec::error("Unknown enum variant")),
        }
    }
}

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
impl value_codec::Serialize for Evaluation {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert(
            "point".into(),
            value_codec::Serialize::to_value(&self.point),
        );
        object.insert("du".into(), value_codec::Serialize::to_value(&self.du));
        object.insert("dv".into(), value_codec::Serialize::to_value(&self.dv));
        object.insert("duu".into(), value_codec::Serialize::to_value(&self.duu));
        object.insert("duv".into(), value_codec::Serialize::to_value(&self.duv));
        object.insert("dvv".into(), value_codec::Serialize::to_value(&self.dvv));
        object.insert(
            "normal".into(),
            value_codec::Serialize::to_value(&self.normal),
        );
        object.insert(
            "gaussianCurvature".into(),
            value_codec::Serialize::to_value(&self.gaussian_curvature),
        );
        object.insert(
            "meanCurvature".into(),
            value_codec::Serialize::to_value(&self.mean_curvature),
        );
        object.insert(
            "derivative_status".into(),
            value_codec::Serialize::to_value(&self.derivative_status),
        );
        object.insert(
            "derivative_side_u".into(),
            value_codec::Serialize::to_value(&self.derivative_side_u),
        );
        object.insert(
            "derivative_side_v".into(),
            value_codec::Serialize::to_value(&self.derivative_side_v),
        );
        object.insert(
            "domainU".into(),
            value_codec::Serialize::to_value(&self.domain_u),
        );
        object.insert(
            "domainV".into(),
            value_codec::Serialize::to_value(&self.domain_v),
        );
        value_codec::Value::Object(object)
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
    pub fn bounds(&self) -> Result<value_codec::Value> {
        self.validate()?;
        Ok(bounds(
            &self
                .control_points
                .iter()
                .flatten()
                .cloned()
                .collect::<Vec<_>>(),
        ))
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
                    .map(|q| (0..3).map(|i| p[i] + q[i] - start[i]).collect())
                    .collect()
            })
            .collect(),
        weights: profile
            .weights
            .iter()
            .map(|w| path.weights.iter().map(|v| w * v).collect())
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
    check(
        (2..=32).contains(&curves.len()),
        "Loft requires 2..32 sections",
    )?;
    for c in curves {
        c.validate()?;
        check(c.control_points[0].len() == 3, "Loft requires 3D sections")?;
    }
    let degree = curves.iter().map(|c| c.degree).max().unwrap();
    let mut sections = Vec::new();
    for c in curves {
        let [a, b] = c.domain();
        let mut c = c.trim(a, b)?;
        c.knots.iter_mut().for_each(|k| *k = (*k - a) / (b - a));
        if c.degree < degree {
            c = c.elevate(degree)?;
        }
        sections.push(c);
    }
    let mut unique: Vec<f64> = sections
        .iter()
        .flat_map(|c| c.knots.iter().copied())
        .filter(|k| *k > 0. && *k < 1.)
        .collect();
    unique.sort_by(f64::total_cmp);
    unique.dedup();
    for k in unique {
        let count = sections
            .iter()
            .map(|c| c.knots.iter().filter(|&&v| v == k).count())
            .max()
            .unwrap();
        for c in &mut sections {
            let existing = c.knots.iter().filter(|&&v| v == k).count();
            if existing < count {
                *c = c.insert(k, count - existing)?;
            }
            check(
                c.control_points.len() <= 32,
                "Aligned loft exceeds 32 section controls",
            )?;
        }
    }
    loft(&sections)
}
#[cfg(test)]
mod construction_tests {
    use super::*;
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
