//! Exact C2 clock conjugacy of ORIGINAL filled affine volumes.
//! F_target(q,t)=F_reference(q,phi(t)) for every carrier point q.
//! The reference global proof is recomputed; a retained model or caller flags
//! cannot substitute for it. Source exact work is shared, never enlarged.
use super::{
    RationalClosedAttempt, RationalClosedBody, RationalClosedLimits, RationalClosedReport, joint,
};
use crate::{LoftCap, Model, Result};
use cad_predicates::{
    AuthoredScalar as A, BezierIdentity as Identity, Limits, PredicateContext, SourceArena,
    ToleranceContext,
};
use nurbs_core::{
    affine_volume::{Pose, RationalPiece},
    curve::Curve,
};

pub struct ClockProof {
    pub certified: bool,
    pub exact_work: u64,
    pub c2_clock: bool,
    pub periodic_derivatives: bool,
    pub columns: usize,
    pub reason: Option<&'static str>,
}
pub struct Refused {
    pub proof: ClockProof,
    pub reference: Option<Box<RationalClosedReport>>,
    pub reason: &'static str,
}
pub struct Body {
    reference: Box<RationalClosedBody>,
    target: Vec<RationalPiece>,
    map: Curve,
    proof: ClockProof,
}
impl Body {
    pub fn model(&self) -> &Model {
        self.reference.model()
    }
    pub fn reference_report(&self) -> &RationalClosedReport {
        self.reference.report()
    }
    pub fn source_motion(&self) -> &[RationalPiece] {
        &self.target
    }
    pub fn parameter_map(&self) -> &Curve {
        &self.map
    }
    pub fn clock_proof(&self) -> &ClockProof {
        &self.proof
    }
    pub fn source_exact_work(&self) -> u64 {
        self.proof.exact_work + self.reference.report().work.source_exact_work
    }
    /// Both original and actual image/global/differential evidence belongs to this
    /// immutable owner. C2 positive one-cover clock changes preserve its grades.
    pub fn joint_certificate(
        &self,
        order: usize,
    ) -> std::result::Result<joint::Certificate<'_>, joint::Refusal> {
        self.reference.joint_certificate(order)
    }
}
pub enum Attempt {
    Certified(Box<Body>),
    Refused(Box<Refused>),
}
struct Motion {
    poses: Vec<Pose>,
    weights: Vec<f64>,
    knots: Vec<A>,
    degree: usize,
}
fn motion(pieces: &[RationalPiece]) -> Result<Option<Motion>> {
    if pieces.is_empty() || pieces.len() > 32 {
        return Err(super::fail("Invalid mapped motion piece census"));
    }
    let degree = pieces[0].controls.len().saturating_sub(1);
    if !(1..=8).contains(&degree) {
        return Err(super::fail("Invalid mapped motion degree"));
    }
    let mut poses = Vec::new();
    let mut weights = Vec::new();
    let mut knots = vec![A::Binary64Bits(0f64.to_bits()); degree + 1];
    for (j, p) in pieces.iter().enumerate() {
        if !(2..=9).contains(&p.controls.len())
            || p.weights.len() != p.controls.len()
            || p.controls
                .iter()
                .flatten()
                .flatten()
                .any(|v| !v.is_finite() || v.abs() > 1e9)
            || p.weights
                .iter()
                .any(|w| !w.is_finite() || !(1e-50..=1e9).contains(w))
        {
            return Err(super::fail("Invalid original mapped motion coefficients"));
        }
        if p.controls.len() != degree + 1 {
            return Ok(None);
        }
        let share =
            j > 0 && poses.last() == p.controls.first() && weights.last() == p.weights.first();
        if j > 0 {
            let mut gcd = j;
            let mut divisor = pieces.len();
            while divisor != 0 {
                (gcd, divisor) = (divisor, gcd % divisor);
            }
            knots.extend(vec![
                A::RationalConstant {
                    numerator: (j / gcd) as i64,
                    denominator: (pieces.len() / gcd) as u64
                };
                if share { degree } else { degree + 1 }
            ]);
        }
        poses.extend_from_slice(&p.controls[usize::from(share)..]);
        weights.extend_from_slice(&p.weights[usize::from(share)..]);
    }
    if poses.len() > 33 {
        return Ok(None);
    }
    knots.extend(vec![A::Binary64Bits(1f64.to_bits()); degree + 1]);
    Ok(Some(Motion {
        poses,
        weights,
        knots,
        degree,
    }))
}
fn prove(
    reference: &[RationalPiece],
    target: &[RationalPiece],
    map: &Curve,
    max_work: u64,
) -> Result<ClockProof> {
    let mut report = ClockProof {
        certified: false,
        exact_work: 0,
        c2_clock: false,
        periodic_derivatives: false,
        columns: 0,
        reason: Some("clock-correspondence-unproved"),
    };
    map.validate()?;
    if map.periodic
        || map.control_points.len() > 33
        || map
            .control_points
            .iter()
            .any(|p| p.len() != 2 || p[1] != 0.)
        || map.degree > 32
        || max_work > 1000000
    {
        return Err(super::fail(
            "Clock map must be a bounded nonperiodic [value,0] curve",
        ));
    }
    let (Some(a), Some(b)) = (motion(reference)?, motion(target)?) else {
        report.reason = Some("clock-motion-original-basis-size-unproved");
        return Ok(report);
    };
    let mut values = Vec::new();
    let mut starts = [[0; 4]; 2];
    let mut ki = [0; 2];
    for (side, m) in [&a, &b].into_iter().enumerate() {
        for column in 0..4 {
            starts[side][column] = values.len();
            for (pose, w) in m.poses.iter().zip(&m.weights) {
                values.extend(
                    pose[column]
                        .iter()
                        .chain(std::iter::once(w))
                        .map(|v| A::Binary64Bits(v.to_bits())),
                );
            }
        }
        ki[side] = values.len();
        values.extend(m.knots.clone());
    }
    let ci = values.len();
    for (p, w) in map.control_points.iter().zip(&map.weights) {
        values.extend([
            A::Binary64Bits(p[0].to_bits()),
            A::Binary64Bits(w.to_bits()),
        ]);
    }
    let mi = values.len();
    values.extend(map.knots.iter().map(|v| A::Binary64Bits(v.to_bits())));
    let zi = values.len();
    values.push(A::Binary64Bits(0f64.to_bits()));
    let source = SourceArena::authored("original-filled-motion-clock-conjugacy", 1, values)
        .map_err(|e| crate::Error::new("BREP_SWEEP_BOUNDARY_INVALID", e.to_string()))?;
    let refs = |at: usize, n: usize| {
        (0..n)
            .map(|i| source.leaf(at + i).unwrap())
            .collect::<Vec<_>>()
    };
    let ak = refs(ki[0], a.knots.len());
    let bk = refs(ki[1], b.knots.len());
    let mk = refs(mi, map.knots.len());
    let mc = (0..map.control_points.len())
        .map(|i| {
            [
                source.leaf(ci + 2 * i).unwrap(),
                source.leaf(ci + 2 * i + 1).unwrap(),
            ]
        })
        .collect::<Vec<_>>();
    let c3 = mc
        .iter()
        .map(|r| {
            [
                r[0],
                source.leaf(zi).unwrap(),
                source.leaf(zi).unwrap(),
                r[1],
            ]
        })
        .collect::<Vec<_>>();
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(
        &source,
        &tolerance,
        Limits {
            max_work,
            ..Limits::default()
        },
        None,
    );
    let internal =
        cad_predicates::rational_bspline_internal_jet_identity(&mut ctx, &c3, &mk, map.degree, 2)
            .map_err(|e| crate::Error::new("BREP_SWEEP_BOUNDARY_INVALID", e.to_string()))?;
    report.exact_work = ctx.work_used();
    if internal.outcome != Identity::Equal {
        report.reason = Some("clock-internal-c2-unproved");
        return Ok(report);
    }
    report.c2_clock = true;
    let closure = cad_predicates::rational_bspline_cartesian_endpoint_derivative_identity(
        &mut ctx, &c3, &mk, map.degree, 2,
    )
    .map_err(|e| crate::Error::new("BREP_SWEEP_BOUNDARY_INVALID", e.to_string()))?;
    report.exact_work = ctx.work_used();
    if closure.outcome != Identity::Equal {
        report.reason = Some("clock-periodic-derivatives-unproved");
        return Ok(report);
    }
    report.periodic_derivatives = true;
    for col in 0..4 {
        let poles = |side: usize, n: usize| {
            (0..n)
                .map(|i| {
                    std::array::from_fn(|k| source.leaf(starts[side][col] + 4 * i + k).unwrap())
                })
                .collect::<Vec<_>>()
        };
        let ac = poles(0, a.poses.len());
        let bc = poles(1, b.poses.len());
        if col > 0
            && a.poses
                .iter()
                .chain(&b.poses)
                .all(|p| p[col] == a.poses[0][col])
        {
            // The first full identity already owns the positive full-cover map.
            // Constant positive-weight source nets need only constant equality.
            let constant =
                cad_predicates::rational_control_nets_constant_identity(&mut ctx, [&ac, &bc])
                    .map_err(|e| crate::Error::new("BREP_SWEEP_BOUNDARY_INVALID", e.to_string()))?;
            report.exact_work = ctx.work_used();
            if constant.outcome != Identity::Equal {
                report.reason = Some("original-pose-clock-identity-unproved");
                return Ok(report);
            }
            report.columns += 1;
            continue;
        }
        let proof = cad_predicates::rational_bspline_mapped_curve_identity(
            &mut ctx,
            [&ac, &bc],
            [&ak, &bk],
            [a.degree, b.degree],
            cad_predicates::BsplineParameterMap {
                controls: &mc,
                knots: &mk,
                degree: map.degree,
            },
        )
        .map_err(|e| crate::Error::new("BREP_SWEEP_BOUNDARY_INVALID", e.to_string()))?;
        report.exact_work = ctx.work_used();
        if proof.outcome != Identity::Equal {
            report.reason = Some("original-pose-clock-identity-unproved");
            return Ok(report);
        }
        report.columns += 1;
    }
    report.certified = true;
    report.reason = None;
    Ok(report)
}
/// Reference and target rational pose pieces and the clock map between them.
#[derive(Clone, Copy)]
pub struct ClockTransfer<'a> {
    pub reference: &'a [RationalPiece],
    pub target: &'a [RationalPiece],
    pub map: &'a Curve,
}
/// Recompute reference global/material proofs, then transfer only through an
/// exact positive C2 periodic-safe clock. Every source column is checked.
pub fn construct(
    profiles: &[Vec<Curve>],
    material: &LoftCap,
    origin: [f64; 3],
    transfer: ClockTransfer<'_>,
    cap_profiles: &[[usize; 2]],
    tolerance_uv: f64,
    mut limits: RationalClosedLimits,
) -> Result<Attempt> {
    let ClockTransfer {
        reference,
        target,
        map,
    } = transfer;
    let proof = prove(reference, target, map, limits.source_exact_work)?;
    if !proof.certified {
        return Ok(Attempt::Refused(Box::new(Refused {
            reason: proof.reason.unwrap(),
            proof,
            reference: None,
        })));
    }
    limits.source_exact_work -= proof.exact_work;
    match super::construct_rational_closed_material(
        profiles,
        material,
        origin,
        reference,
        cap_profiles,
        tolerance_uv,
        limits,
    )? {
        RationalClosedAttempt::Refused(r) => Ok(Attempt::Refused(Box::new(Refused {
            proof,
            reference: Some(r),
            reason: "reference-original-global-proof-unproved",
        }))),
        RationalClosedAttempt::Certified(b) => {
            if b.joint_certificate(0).is_err() {
                return Ok(Attempt::Refused(Box::new(Refused {
                    proof,
                    reference: None,
                    reason: "reference-joint-global-proof-unproved",
                })));
            }
            Ok(Attempt::Certified(Box::new(Body {
                reference: b,
                target: target.to_vec(),
                map: map.clone(),
                proof,
            })))
        }
    }
}
