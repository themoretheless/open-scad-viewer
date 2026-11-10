//! Local variable-radius contact proposal on two ORIGINAL planar supports.
//! This is not a root certificate, complete branch census or closed B-rep.
use super::{jet, norm, offset, refused, restriction_error, sub, validate};
use crate::{Result, curve::Curve};

pub struct Proposal {
    incoming: Curve,
    outgoing: Curve,
    bridges: [Curve; 2],
    center: [f64; 2],
    radius: f64,
    parameters: [f64; 2],
    contacts: [[f64; 2]; 2],
    source_error_upper: [f64; 2],
    residual: f64,
    iterations: usize,
}
impl Proposal {
    pub fn incoming(&self) -> &Curve {
        &self.incoming
    }
    pub fn outgoing(&self) -> &Curve {
        &self.outgoing
    }
    pub fn bridges(&self) -> &[Curve; 2] {
        &self.bridges
    }
    pub fn center(&self) -> [f64; 2] {
        self.center
    }
    pub fn radius(&self) -> f64 {
        self.radius
    }
    pub fn parameters(&self) -> [f64; 2] {
        self.parameters
    }
    pub fn contacts(&self) -> [[f64; 2]; 2] {
        self.contacts
    }
    pub fn source_error_upper(&self) -> [f64; 2] {
        self.source_error_upper
    }
    pub fn contact_residual(&self) -> f64 {
        self.residual
    }
    pub fn iterations(&self) -> usize {
        self.iterations
    }
}
fn dot(a: [f64; 2], b: [f64; 2]) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}
fn norm3(a: [f64; 3]) -> f64 {
    a[0].hypot(a[1]).hypot(a[2])
}

/// Solve offset(A,ta,r)=offset(B,tb,r) and the crown's contact with
/// the removed middle line. All three parameters are unknown. The fixed seed
/// chooses a LOCAL convex branch; numeric success does not authorize a body.
pub fn propose(a: &Curve, middle: &Curve, b: &Curve, tolerance: f64) -> Result<Proposal> {
    validate(a, middle, tolerance)?;
    validate(middle, b, tolerance)?;
    if middle.degree != 1
        || middle.control_points.len() != 2
        || [a, middle, b]
            .iter()
            .any(|c| c.weights.iter().any(|w| *w <= 0.))
    {
        return Err(refused(
            "Full Round proposal needs positive-weight supports and a straight middle",
        ));
    }
    let ends = [
        jet(middle, middle.domain()[0])?.point,
        jet(middle, middle.domain()[1])?.point,
    ];
    let direction = sub(ends[1], ends[0]);
    let width = norm(direction);
    let scale = width / 2.;
    if !scale.is_finite() || scale <= tolerance * 4. {
        return Err(refused("Degenerate Full Round middle width"));
    }
    let outward = [direction[1] / width, -direction[0] / width];
    let domains = [a.domain(), b.domain()];
    let widths = domains.map(|d| d[1] - d[0]);
    let residual = |x: [f64; 3]| -> Result<([f64; 3], [[f64; 3]; 3], [[f64; 2]; 2])> {
        let r = x[2] * scale;
        let t = std::array::from_fn::<_, 2, _>(|i| domains[i][0] + widths[i] * x[i]);
        let (pa, da) = offset(a, t[0], r)?;
        let (pb, db) = offset(b, t[1], r)?;
        let normals = [jet(a, t[0])?, jet(b, t[1])?].map(|j| {
            let speed = norm(j.d1);
            [-j.d1[1] / speed, j.d1[0] / speed]
        });
        let f = [
            pa[0] - pb[0],
            pa[1] - pb[1],
            dot(outward, sub(pa, ends[0])) + r,
        ];
        let jac = [
            [
                da[0] * widths[0],
                -db[0] * widths[1],
                (normals[0][0] - normals[1][0]) * scale,
            ],
            [
                da[1] * widths[0],
                -db[1] * widths[1],
                (normals[0][1] - normals[1][1]) * scale,
            ],
            [
                dot(outward, da) * widths[0],
                0.,
                (dot(outward, normals[0]) + 1.) * scale,
            ],
        ];
        if f.iter().chain(jac.iter().flatten()).any(|v| !v.is_finite()) {
            return Err(refused("Full Round contact overflow"));
        }
        Ok((f, jac, [pa, pb]))
    };
    let mut x = [0.75, 0.25, 1.];
    let mut iterations = 0;
    let mut solved = false;
    for _ in 0..40 {
        iterations += 1;
        let (f, jac, _) = residual(x)?;
        let step = crate::offset::freeform_blend::solve3(jac, f.map(|v| -v))?;
        let old = norm3(f);
        if old <= tolerance / 8. {
            solved = true;
            break;
        }
        let mut accepted = false;
        let mut damping = 1.;
        for _ in 0..24 {
            let next = std::array::from_fn(|i| x[i] + damping * step[i]);
            if next.iter().all(|v| v.is_finite())
                && next[0] > 0.
                && next[0] < 1.
                && next[1] > 0.
                && next[1] < 1.
                && next[2] * scale > tolerance * 4.
            {
                if let Ok((f, _, _)) = residual(next) {
                    if norm3(f) < old {
                        x = next;
                        accepted = true;
                        break;
                    }
                }
            }
            damping *= 0.5;
        }
        if !accepted {
            return Err(refused(
                "Full Round local contact descent failed or supports are too short",
            ));
        }
    }
    let (f, _, centers) = residual(x)?;
    if !solved && norm3(f) > tolerance / 8. {
        return Err(refused("Full Round contact work exhausted"));
    }
    let parameters = std::array::from_fn(|i| domains[i][0] + widths[i] * x[i]);
    let incoming = a.trim(domains[0][0], parameters[0])?;
    let outgoing = b.trim(parameters[1], domains[1][1])?;
    let source_error_upper = [
        restriction_error(a, &incoming)?,
        restriction_error(b, &outgoing)?,
    ];
    if source_error_upper.iter().any(|e| *e > tolerance) {
        return Err(refused(
            "Full Round retained curves disagree with original restrictions",
        ));
    }
    let contacts = [
        incoming.control_points.last().unwrap(),
        &outgoing.control_points[0],
    ]
    .map(|p| [p[0], p[1]]);
    let center = std::array::from_fn(|i| centers[0][i] / 2. + centers[1][i] / 2.);
    let radius = x[2] * scale;
    let error = norm3(f)
        .max((norm(sub(contacts[0], center)) - radius).abs())
        .max((norm(sub(contacts[1], center)) - radius).abs());
    if error > tolerance {
        return Err(refused("Stored Full Round contacts exceed tolerance"));
    }
    let angle = |p: [f64; 2]| (p[1] - center[1]).atan2(p[0] - center[0]);
    let start = angle(contacts[0]);
    let sweep = (angle(contacts[1]) - start).rem_euclid(std::f64::consts::TAU);
    let crown_angle = outward[1].atan2(outward[0]);
    let split = (crown_angle - start).rem_euclid(std::f64::consts::TAU);
    if split <= 1e-8 || sweep - split <= 1e-8 || sweep >= std::f64::consts::TAU - 1e-8 {
        return Err(refused(
            "Full Round crown is outside the forward contact arc",
        ));
    }
    let crown = std::array::from_fn::<_, 2, _>(|i| center[i] + radius * outward[i]);
    let mut bridges = [(start, split), (start + split, sweep - split)]
        .map(|(a, d)| {
            crate::foundation::conics::circle_arc(
                [center[0], center[1], 0.],
                [0., 0., 1.],
                radius,
                a.to_degrees(),
                d.to_degrees(),
            )
        })
        .into_iter()
        .collect::<Result<Vec<_>>>()?;
    for curve in &mut bridges {
        for p in &mut curve.control_points {
            p.truncate(2);
        }
    }
    bridges[0].control_points[0] = contacts[0].to_vec();
    *bridges[0].control_points.last_mut().unwrap() = crown.to_vec();
    bridges[1].control_points[0] = crown.to_vec();
    *bridges[1].control_points.last_mut().unwrap() = contacts[1].to_vec();
    for curve in &bridges {
        curve.validate()?;
    }
    Ok(Proposal {
        incoming,
        outgoing,
        bridges: bridges.try_into().unwrap_or_else(|_| unreachable!()),
        center,
        radius,
        parameters,
        contacts,
        source_error_upper,
        residual: error,
        iterations,
    })
}
