//! Homogeneous point/G1 equalities and first/second-jet Newton rows.
use super::*;
use crate::curve::basis;
pub(super) struct Rows {
    pub a: Vec<Vec<f64>>,
    pub b: Vec<f64>,
}
impl Rows {
    pub fn new() -> Self {
        Self {
            a: Vec::new(),
            b: Vec::new(),
        }
    }
    pub fn add(&mut self, mut a: Vec<f64>, mut b: f64) -> Result<()> {
        if self.a.len() == 2048 {
            return Err(error("Homogeneous equation allowance exhausted"));
        }
        let length = super::super::norm(&a);
        if !length.is_finite() || !b.is_finite() {
            return Err(error("Nonfinite homogeneous equation"));
        }
        if length == 0. {
            return if b.abs() < 1e-12 {
                Ok(())
            } else {
                Err(error("Inconsistent zero homogeneous equation"))
            };
        }
        for x in &mut a {
            *x /= length;
        }
        b /= length;
        self.a.push(a);
        self.b.push(b);
        Ok(())
    }
}
fn polynomial(s: &Surface, uv: [f64; 2]) -> Result<Vec<[f64; 3]>> {
    let u = basis(s.degree_u, &s.knots_u, s.control_points.len(), uv[0], false)?;
    let v = basis(
        s.degree_v,
        &s.knots_v,
        s.control_points[0].len(),
        uv[1],
        false,
    )?;
    Ok((0..u.basis.len())
        .flat_map(|i| {
            let v = &v;
            let u = &u;
            (0..v.basis.len()).map(move |j| {
                [
                    u.basis[i] * v.basis[j],
                    u.d1[i] * v.basis[j],
                    u.basis[i] * v.d1[j],
                ]
            })
        })
        .collect())
}
fn point_row(
    s: &Surface,
    uv: [f64; 2],
    point: [f64; 3],
    origin: [f64; 3],
    rows: &mut Rows,
) -> Result<()> {
    let b = polynomial(s, uv)?;
    let n = b.len();
    for k in 0..3 {
        let mut a = vec![0.; 4 * n];
        for i in 0..n {
            a[k * n + i] = b[i][0];
            a[3 * n + i] = -(point[k] - origin[k]) * b[i][0];
        }
        rows.add(a, 0.)?;
    }
    Ok(())
}
fn tangent_rows(
    s: &Surface,
    uv: [f64; 2],
    point: [f64; 3],
    normal: [f64; 3],
    origin: [f64; 3],
    rows: &mut Rows,
) -> Result<()> {
    let b = polynomial(s, uv)?;
    let n = b.len();
    let length = super::super::norm(&normal);
    if length <= 0. || !length.is_finite() {
        return Err(error("Homogeneous tangent plane needs a resolved normal"));
    }
    let normal = normal.map(|x| x / length);
    let h = (0..3)
        .map(|k| normal[k] * (point[k] - origin[k]))
        .sum::<f64>();
    for axis in 1..=2 {
        let mut a = vec![0.; 4 * n];
        for i in 0..n {
            for k in 0..3 {
                a[k * n + i] = normal[k] * b[i][axis];
            }
            a[3 * n + i] = -h * b[i][axis];
        }
        rows.add(a, 0.)?;
    }
    Ok(())
}
fn jet_row(
    s: &Surface,
    uv: [f64; 2],
    e: &JetEquation,
    origin: [f64; 3],
    current: &[f64],
    rows: &mut Rows,
) -> Result<()> {
    let needs_second = e.second.iter().flatten().any(|x| *x != 0.);
    let basis = super::super::jet::coefficient_jets(s, uv, needs_second)?;
    let at = s.evaluate(uv[0], uv[1])?;
    let (u, v) = at
        .first_derivatives()
        .ok_or_else(|| error("Homogeneous jet tangent unresolved"))?;
    let second = if needs_second {
        let (uu, uv, vv) = at
            .second_derivatives()
            .ok_or_else(|| error("Homogeneous second jet unresolved"))?;
        [uu, uv, vv]
    } else {
        [[0.; 3]; 3]
    };
    let p = std::array::from_fn::<_, 3, _>(|k| at.point[k] - origin[k]);
    let first = [u, v];
    let n = basis.len();
    let mut a = vec![0.; 4 * n];
    for (i, r) in basis.iter().enumerate() {
        // dS/dH_i = B_i/W; dS/dW_i = -S*(B_i/W).
        // Differentiate the latter product before forming the scalar jet.
        let w = s.weights[i / s.weights[0].len()][i % s.weights[0].len()];
        let r = r.map(|x| x / w);
        for k in 0..3 {
            a[k * n + i] = (0..2).map(|j| e.first[j][k] * r[j + 1]).sum::<f64>()
                + (0..3).map(|j| e.second[j][k] * r[j + 3]).sum::<f64>();
        }
        let mut value = 0.;
        for k in 0..3 {
            for j in 0..2 {
                value += e.first[j][k] * (first[j][k] * r[0] + p[k] * r[j + 1]);
            }
            for (j, [x, y]) in [[0, 0], [0, 1], [1, 1]].into_iter().enumerate() {
                value += e.second[j][k]
                    * (second[j][k] * r[0]
                        + first[x][k] * r[y + 1]
                        + first[y][k] * r[x + 1]
                        + p[k] * r[j + 3]);
            }
        }
        a[3 * n + i] = -value;
    }
    let value = super::super::jet::value(s, uv, e)?;
    let rhs = e.value - value + super::super::dot(&a, current);
    rows.add(a, rhs)
}
fn parameter(c: &Curve, t: f64) -> f64 {
    let d = c.domain();
    (d[0] + (d[1] - d[0]) * t).clamp(d[0], d[1])
}
pub(super) fn build(
    s: &Surface,
    origin: [f64; 3],
    guides: &[network::CurveConstraint<'_>],
    curvatures: &[network::CurvatureConstraint<'_>],
    sites: &[Vec<f64>],
    hard: &[Constraint],
    newton: bool,
) -> Result<Rows> {
    let n = s.control_points.len() * s.control_points[0].len();
    let current = fit::encode(s, origin);
    let mut rows = Rows::new();
    for (index, g) in guides
        .iter()
        .copied()
        .chain(curvatures.iter().map(|g| network::CurveConstraint {
            spatial: g.spatial,
            parameters: g.parameters,
            reverse: g.reverse,
            tangent: Some(g.support),
        }))
        .enumerate()
    {
        for &t in &sites[index] {
            let q = if g.reverse { 1. - t } else { t };
            let point = g.spatial.evaluate(parameter(g.spatial, q))?.point;
            let point = [point[0], point[1], point[2]];
            let uv = g.parameters.evaluate(parameter(g.parameters, t))?.point;
            let uv = [uv[0], uv[1]];
            point_row(s, uv, point, origin, &mut rows)?;
            if let Some(support) = g.tangent {
                let r = if support.reverse { 1. - q } else { q };
                let pc = support
                    .parameters
                    .evaluate(parameter(support.parameters, r))?
                    .point;
                let source = support.surface.evaluate(pc[0], pc[1])?;
                let normal = source
                    .unit_normal()
                    .ok_or_else(|| error("Original homogeneous support normal unresolved"))?;
                tangent_rows(s, uv, point, normal, origin, &mut rows)?;
                if newton && index >= guides.len() {
                    let mut jets = Vec::new();
                    network::curvature_equations(
                        s,
                        uv,
                        support.surface,
                        [pc[0], pc[1]],
                        &mut jets,
                    )?;
                    for c in jets {
                        if let Constraint::Jet { uv, equation } = c {
                            jet_row(s, uv, &equation, origin, &current, &mut rows)?;
                        }
                    }
                }
            }
        }
    }
    for c in hard {
        match c {
            Constraint::Point { uv, point } => point_row(s, *uv, *point, origin, &mut rows)?,
            Constraint::ControlPoint {
                index: [i, j],
                point,
            } => {
                if *i >= s.control_points.len() || *j >= s.control_points[0].len() {
                    return Err(error("Unknown homogeneous fixed control"));
                }
                let index = i * s.control_points[0].len() + j;
                for k in 0..3 {
                    let mut row = vec![0.; 4 * n];
                    row[k * n + index] = 1.;
                    row[3 * n + index] = -(point[k] - origin[k]);
                    rows.add(row, 0.)?;
                }
            }
            Constraint::Derivative { uv, axis, value } => {
                if !newton {
                    continue;
                }
                for k in 0..3 {
                    let mut e = JetEquation {
                        first: [[0.; 3]; 2],
                        second: [[0.; 3]; 3],
                        value: value[k],
                    };
                    e.first[if matches!(axis, super::super::Axis::U) {
                        0
                    } else {
                        1
                    }][k] = 1.;
                    jet_row(s, *uv, &e, origin, &current, &mut rows)?;
                }
            }
            Constraint::TangentPlane { uv, normal } => {
                if !newton {
                    continue;
                }
                for axis in 0..2 {
                    let mut equation = JetEquation {
                        first: [[0.; 3]; 2],
                        second: [[0.; 3]; 3],
                        value: 0.,
                    };
                    equation.first[axis] = *normal;
                    jet_row(s, *uv, &equation, origin, &current, &mut rows)?;
                }
            }
            Constraint::Jet { uv, equation } => {
                if !newton {
                    continue;
                }
                jet_row(s, *uv, equation, origin, &current, &mut rows)?
            }
        }
    }
    Ok(rows)
}
