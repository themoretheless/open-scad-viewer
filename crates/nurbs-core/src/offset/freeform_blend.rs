//! Retained rolling-ball sections and ruled chamfers on original freeform charts.
//! Local numeric queries do not authorize trimming/sewing a closed B-rep body.
use super::freeform::NormalOffset;
use crate::{Error, Result, curve::Curve, surface::Surface};
fn error(message: &str) -> Error {
    Error::new("NURBS_FREEFORM_BLEND_REFUSED", message)
}
fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|i| a[i] - b[i])
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|i| a[i] * b[i]).sum()
}
fn norm(a: [f64; 3]) -> f64 {
    a[0].hypot(a[1]).hypot(a[2])
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn unit(a: [f64; 3]) -> Result<[f64; 3]> {
    let n = norm(a);
    if !n.is_finite() || n <= 0. {
        return Err(error("Degenerate blend frame"));
    }
    Ok(a.map(|v| v / n))
}
pub(super) fn solve3(mut a: [[f64; 3]; 3], mut b: [f64; 3]) -> Result<[f64; 3]> {
    let scale = a.iter().flatten().map(|x| x.abs()).fold(0f64, f64::max);
    if !scale.is_finite() || scale == 0. {
        return Err(error("Degenerate offset intersection Jacobian"));
    }
    for col in 0..3 {
        let row = (col..3)
            .max_by(|&i, &j| a[i][col].abs().total_cmp(&a[j][col].abs()))
            .unwrap();
        if !scale.is_finite() || scale == 0. || a[row][col].abs() < scale * 1e-12 {
            return Err(error(
                "Offset intersection is singular in the selected driver chart",
            ));
        }
        a.swap(row, col);
        b.swap(row, col);
        let divisor = a[col][col];
        for j in col..3 {
            a[col][j] /= divisor;
        }
        b[col] /= divisor;
        for i in 0..3 {
            if i != col {
                let f = a[i][col];
                for j in col..3 {
                    a[i][j] -= f * a[col][j];
                }
                b[i] -= f * b[col];
            }
        }
    }
    if b.iter().any(|v| !v.is_finite()) {
        return Err(error("Blend Newton solve overflow"));
    }
    Ok(b)
}

/// Exact-real ruled surface between two supplied original-support UV rails.
/// This defines contact geometry; setback selection and closed-body authoring
/// are explicit callers' responsibilities, not inferred from a display mesh.
#[derive(Clone)]
pub struct ContactChamfer {
    supports: [Surface; 2],
    pcurves: [Curve; 2],
}
pub struct ChamferEvaluation {
    pub point: [f64; 3],
    pub tangent_u: [f64; 3],
    pub tangent_v: [f64; 3],
    pub normal: [f64; 3],
    pub contacts: [[f64; 3]; 2],
}
impl ContactChamfer {
    pub fn new(supports: [&Surface; 2], pcurves: [&Curve; 2]) -> Result<Self> {
        for s in supports {
            s.validate()?;
            if s.control_points[0][0].len() != 3 || s.periodic_u || s.periodic_v {
                return Err(error("Chamfer requires nonperiodic 3D support charts"));
            }
        }
        for k in 0..2 {
            let pc = pcurves[k];
            pc.validate()?;
            if pc.control_points[0].len() != 2 || pc.periodic {
                return Err(error("Chamfer contact rails require nonperiodic 2D curves"));
            }
            let s = supports[k];
            let d = [
                [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
                [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
            ];
            // Positive rational convex hull keeps the ENTIRE rail in chart.
            if pc
                .control_points
                .iter()
                .any(|p| (0..2).any(|a| p[a] < d[a][0] || p[a] > d[a][1]))
            {
                return Err(error("Whole chamfer contact chart containment is unproven"));
            }
        }
        Ok(Self {
            supports: supports.map(Clone::clone),
            pcurves: pcurves.map(Clone::clone),
        })
    }
    pub fn supports(&self) -> &[Surface; 2] {
        &self.supports
    }
    pub fn contact_pcurves(&self) -> &[Curve; 2] {
        &self.pcurves
    }
    pub fn evaluate(&self, u: f64, v: f64) -> Result<ChamferEvaluation> {
        if [u, v]
            .iter()
            .any(|x| !x.is_finite() || !(0. ..=1.).contains(x))
        {
            return Err(error("Chamfer query outside [0,1]^2"));
        }
        let mut contacts = [[0.; 3]; 2];
        let mut tangents = [[0.; 3]; 2];
        for k in 0..2 {
            let pc = &self.pcurves[k];
            let d = pc.domain();
            let e = pc.evaluate(d[0] + u * (d[1] - d[0]))?;
            let velocity =
                e.d1.as_ref()
                    .ok_or_else(|| error("Contact rail derivative unresolved"))?;
            let p = self.supports[k].evaluate(e.point[0], e.point[1])?;
            let (su, sv) = p
                .first_derivatives()
                .ok_or_else(|| error("Chamfer support derivatives unresolved"))?;
            contacts[k] = p.point;
            tangents[k] = std::array::from_fn(|i| {
                (su[i] * velocity[0] + sv[i] * velocity[1]) * (d[1] - d[0])
            });
        }
        let point = std::array::from_fn(|i| (1. - v) * contacts[0][i] + v * contacts[1][i]);
        let tangent_u = std::array::from_fn(|i| (1. - v) * tangents[0][i] + v * tangents[1][i]);
        let tangent_v = sub(contacts[1], contacts[0]);
        let normal = unit(cross(tangent_u, tangent_v))?;
        if point.iter().chain(&tangent_u).any(|x| !x.is_finite()) {
            return Err(error("Chamfer evaluation overflow"));
        }
        Ok(ChamferEvaluation {
            point,
            tangent_u,
            tangent_v,
            normal,
            contacts,
        })
    }
}

/// Two immutable original supports and a fixed-radius rolling-ball family.
/// A chosen support coordinate drives the one-dimensional center intersection.
/// Explicit seeds choose a local branch; no global branch uniqueness is claimed.
pub struct RollingBall {
    supports: [Surface; 2],
    offsets: [NormalOffset; 2],
    radius: f64,
    driver: usize,
    tolerance: f64,
}
pub struct BallSection {
    center: [f64; 3],
    contacts: [[f64; 3]; 2],
    parameters: [f64; 4],
    frame: [[f64; 3]; 2],
    radius: f64,
    angle: f64,
    residual: f64,
}
impl BallSection {
    pub fn center(&self) -> [f64; 3] {
        self.center
    }
    pub fn contacts(&self) -> [[f64; 3]; 2] {
        self.contacts
    }
    pub fn parameters(&self) -> [f64; 4] {
        self.parameters
    }
    pub fn radius(&self) -> f64 {
        self.radius
    }
    pub fn offset_residual(&self) -> f64 {
        self.residual
    }
    pub fn evaluate(&self, t: f64) -> Result<([f64; 3], [f64; 3])> {
        if !t.is_finite() || !(0. ..=1.).contains(&t) {
            return Err(error("Ball section parameter outside [0,1]"));
        }
        let a = t * self.angle;
        let normal =
            std::array::from_fn(|i| self.frame[0][i] * a.cos() + self.frame[1][i] * a.sin());
        let point = std::array::from_fn(|i| self.center[i] + self.radius * normal[i]);
        Ok((point, normal))
    }
    /// Rational circular proposal from the solved local ball. Contact/radius
    /// agreement with the original supports retains the reported solve residual.
    pub fn curve(&self) -> Result<Curve> {
        let start = self.evaluate(0.)?.0;
        let end = self.evaluate(1.)?.0;
        let w = (self.angle / 2.).cos();
        if w <= 1e-8 {
            return Err(error(
                "Circular section approaches a semicircle; split required",
            ));
        }
        let middle = std::array::from_fn::<_, 3, _>(|i| {
            self.center[i]
                + self.radius / w
                    * (self.frame[0][i] * (self.angle / 2.).cos()
                        + self.frame[1][i] * (self.angle / 2.).sin())
        });
        let c = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![start.to_vec(), middle.to_vec(), end.to_vec()],
            weights: vec![1., w, 1.],
            periodic: false,
        };
        c.validate()?;
        Ok(c)
    }
}
impl RollingBall {
    pub fn new(
        supports: [&Surface; 2],
        radius: f64,
        sides: [f64; 2],
        driver: usize,
        tolerance: f64,
    ) -> Result<Self> {
        if !radius.is_finite()
            || radius <= 0.
            || !tolerance.is_finite()
            || tolerance <= 0.
            || tolerance >= radius * 0.01
            || driver >= 4
            || sides.iter().any(|x| !x.is_finite() || x.abs() != 1.)
        {
            return Err(error(
                "Rolling ball needs positive radius, ±1 sides, driver 0..3 and a small positive tolerance",
            ));
        }
        let offsets = [
            NormalOffset::new(supports[0], radius * sides[0])?,
            NormalOffset::new(supports[1], radius * sides[1])?,
        ];
        Ok(Self {
            supports: supports.map(Clone::clone),
            offsets,
            radius,
            driver,
            tolerance,
        })
    }
    pub fn section(&self, driver_value: f64, seed: [f64; 4]) -> Result<BallSection> {
        let d0 = self.offsets[0].domain();
        let d1 = self.offsets[1].domain();
        let domains = [d0[0], d0[1], d1[0], d1[1]];
        let inside = |x: [f64; 4]| {
            (0..4).all(|i| x[i].is_finite() && domains[i][0] <= x[i] && x[i] <= domains[i][1])
        };
        let mut x = seed;
        x[self.driver] = driver_value;
        if !inside(x) {
            return Err(error("Ball seed/driver outside original support domains"));
        }
        let free = (0..4).filter(|i| *i != self.driver).collect::<Vec<_>>();
        let residual = |x: [f64; 4]| -> Result<([f64; 3], [[f64; 3]; 4], [[f64; 3]; 2])> {
            let a = self.offsets[0].evaluate([x[0], x[1]])?;
            let b = self.offsets[1].evaluate([x[2], x[3]])?;
            Ok((
                sub(a.point, b.point),
                [
                    a.tangent_u,
                    a.tangent_v,
                    b.tangent_u.map(|v| -v),
                    b.tangent_v.map(|v| -v),
                ],
                [a.point, b.point],
            ))
        };
        let mut converged = false;
        for _ in 0..32 {
            let (f, columns, _) = residual(x)?;
            let old = norm(f);
            let jac = std::array::from_fn(|i| std::array::from_fn(|j| columns[free[j]][i]));
            if old <= self.tolerance {
                let _ = solve3(jac, [0.; 3])?;
                converged = true;
                break;
            }
            let step = solve3(jac, f.map(|v| -v))?;
            let mut scale = 1.;
            let mut accepted = false;
            for _ in 0..24 {
                let mut next = x;
                for j in 0..3 {
                    next[free[j]] += scale * step[j];
                }
                if inside(next) {
                    if let Ok((f, _, _)) = residual(next) {
                        if norm(f) < old {
                            x = next;
                            accepted = true;
                            break;
                        }
                    }
                }
                scale *= 0.5;
            }
            if !accepted {
                return Err(error(
                    "Ball correction left support domains or lost descent",
                ));
            }
        }
        let (f, _, centers) = residual(x)?;
        let residual = norm(f);
        if (!converged && residual > self.tolerance) || !residual.is_finite() {
            return Err(error("Ball intersection Newton budget exhausted"));
        }
        let center = std::array::from_fn(|i| centers[0][i] / 2. + centers[1][i] / 2.);
        let contacts = [
            self.supports[0].evaluate(x[0], x[1])?.point,
            self.supports[1].evaluate(x[2], x[3])?.point,
        ];
        let radial = [sub(contacts[0], center), sub(contacts[1], center)];
        if radial
            .iter()
            .any(|v| (norm(*v) - self.radius).abs() > self.tolerance * 2.)
        {
            return Err(error("Solved ball contacts exceed radius tolerance"));
        }
        let a = unit(radial[0])?;
        let b = unit(radial[1])?;
        let cosine = dot(a, b).clamp(-1., 1.);
        let transverse = sub(b, a.map(|v| v * cosine));
        if norm(transverse) < 1e-8 {
            return Err(error("Ball contact angle is collapsed or antipodal"));
        }
        let frame = [a, unit(transverse)?];
        let angle = cosine.acos();
        Ok(BallSection {
            center,
            contacts,
            parameters: x,
            frame,
            radius: self.radius,
            angle,
            residual,
        })
    }
}
