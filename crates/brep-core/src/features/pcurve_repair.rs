//! Standalone periodic pcurve, pole-branch, same-parameter and repair APIs.
//!
//! These routines do not parse STEP and do not alter direct-edge import. A
//! caller owns the decision to apply a returned proposal. Every repair is
//! bounded and is accepted only after a fresh 33-node same-parameter audit.

use nurbs_core::{Error, Result, curve::Curve, curve_pullback, surface::Surface};

pub const PCURVE_AMBIGUOUS: &str = "BREP_PCURVE_REPAIR_AMBIGUOUS";
pub const PCURVE_BUDGET: &str = "BREP_PCURVE_REPAIR_BUDGET";
pub const PCURVE_INVALID: &str = "BREP_PCURVE_REPAIR_INVALID";
pub const PCURVE_UNRESOLVED: &str = "BREP_PCURVE_REPAIR_UNRESOLVED";
pub const AGREEMENT_SAMPLES: usize = 33;

fn error(code: &'static str, message: impl Into<String>) -> Error {
    Error::new(code, message)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PeriodicAxis {
    pub min: f64,
    pub max: f64,
}

impl PeriodicAxis {
    pub fn new(min: f64, max: f64) -> Result<Self> {
        if !min.is_finite() || !max.is_finite() || !(max > min) || !(max - min).is_finite() {
            return Err(error(
                PCURVE_INVALID,
                "Period bounds must be finite and increasing",
            ));
        }
        Ok(Self { min, max })
    }

    pub fn period(self) -> f64 {
        self.max - self.min
    }

    pub fn wrap(self, value: f64) -> Result<f64> {
        if !value.is_finite() {
            return Err(error(PCURVE_INVALID, "Cannot wrap a non-finite coordinate"));
        }
        let wrapped = self.min + (value - self.min).rem_euclid(self.period());
        if wrapped.is_finite() {
            Ok(wrapped)
        } else {
            Err(error(
                PCURVE_INVALID,
                "Periodic wrapping exhausted finite precision",
            ))
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SurfacePeriods {
    pub u: Option<PeriodicAxis>,
    pub v: Option<PeriodicAxis>,
}

impl SurfacePeriods {
    fn axis(self, axis: usize) -> Option<PeriodicAxis> {
        if axis == 0 { self.u } else { self.v }
    }
}

fn domain(surface: &Surface) -> [[f64; 2]; 2] {
    [
        [
            surface.knots_u[surface.degree_u],
            surface.knots_u[surface.control_points.len()],
        ],
        [
            surface.knots_v[surface.degree_v],
            surface.knots_v[surface.control_points[0].len()],
        ],
    ]
}

fn distance(a: &[f64], b: &[f64]) -> Result<f64> {
    if a.len() != b.len() || a.iter().chain(b).any(|x| !x.is_finite()) {
        return Err(error(
            PCURVE_INVALID,
            "Point dimensions differ or contain NaN/Inf",
        ));
    }
    let d = a
        .iter()
        .zip(b)
        .map(|(x, y)| (x - y) * (x - y))
        .sum::<f64>()
        .sqrt();
    if d.is_finite() {
        Ok(d)
    } else {
        Err(error(PCURVE_INVALID, "Distance overflow"))
    }
}

fn norm(v: &[f64]) -> Result<f64> {
    distance(v, &vec![0.; v.len()])
}

fn nearest_translation(period: PeriodicAxis, from: f64, to: f64) -> Result<i32> {
    let q = (to - from) / period.period();
    if !q.is_finite() || q.round() < i32::MIN as f64 || q.round() > i32::MAX as f64 {
        return Err(error(PCURVE_INVALID, "Periodic lift exceeds i32 range"));
    }
    Ok(q.round() as i32)
}

fn unwrap_points(
    points: &[[f64; 2]],
    periods: SurfacePeriods,
) -> Result<(Vec<[f64; 2]>, [[i32; 2]; 2])> {
    if points.is_empty() {
        return Err(error(PCURVE_INVALID, "Cannot unwrap an empty pcurve"));
    }
    let mut out = vec![points[0]];
    let mut lift = [0_i32; 2];
    for (index, point) in points[1..].iter().enumerate() {
        let previous_raw = points[index];
        let mut next = *point;
        for axis in 0..2 {
            if let Some(period) = periods.axis(axis) {
                lift[axis] = lift[axis]
                    .checked_add(nearest_translation(
                        period,
                        point[axis],
                        previous_raw[axis],
                    )?)
                    .ok_or_else(|| error(PCURVE_INVALID, "Periodic lift overflow"))?;
                next[axis] += lift[axis] as f64 * period.period();
            }
        }
        if next.iter().any(|x| !x.is_finite()) {
            return Err(error(PCURVE_INVALID, "Unwrapped coordinate is not finite"));
        }
        out.push(next);
    }
    Ok((out, [[0, 0], lift]))
}

fn curve_points(curve: &Curve) -> Result<Vec<[f64; 2]>> {
    curve.validate()?;
    if curve.control_points[0].len() != 2 {
        return Err(error(PCURVE_INVALID, "A pcurve must be two-dimensional"));
    }
    curve
        .control_points
        .iter()
        .map(|p| Ok([p[0], p[1]]))
        .collect()
}

fn polyline(points: Vec<[f64; 2]>) -> Result<Curve> {
    Curve::from_polyline(points.into_iter().map(|p| p.to_vec()).collect())
}

/// Join exactly two authored polyline pcurves across periodic seams. Integer
/// lifts are chosen independently on each axis. C0 and C1 are checked in the
/// unwrapped chart; no wrapped finite difference is accepted as a tangent.
pub fn join_seam_polylines(
    first: &Curve,
    second: &Curve,
    periods: SurfacePeriods,
    c0_tolerance: f64,
    c1_tolerance: f64,
) -> Result<UnwrappedPcurve> {
    if !c0_tolerance.is_finite()
        || c0_tolerance < 0.
        || !c1_tolerance.is_finite()
        || c1_tolerance < 0.
    {
        return Err(error(
            PCURVE_INVALID,
            "Join tolerances must be finite and nonnegative",
        ));
    }
    if first.degree != 1 || second.degree != 1 || first.periodic || second.periodic {
        return Err(error(
            PCURVE_AMBIGUOUS,
            "Exact seam joining currently requires nonperiodic polylines",
        ));
    }
    let first_points = curve_points(first)?;
    let mut raw = first_points.clone();
    raw.extend(curve_points(second)?);
    let (points, lift) = unwrap_points(&raw, periods)?;
    let join = first_points.len() - 1;
    let end = points[join];
    let start = points[join + 1];
    let gap = distance(&end, &start)?;
    if gap > c0_tolerance {
        return Err(error(
            PCURVE_AMBIGUOUS,
            format!("Seam join C0 gap {gap} exceeds {c0_tolerance}"),
        ));
    }
    let da = [end[0] - points[join - 1][0], end[1] - points[join - 1][1]];
    let db = [
        points[join + 2][0] - start[0],
        points[join + 2][1] - start[1],
    ];
    let tangent_jump = distance(&da, &db)?;
    if tangent_jump > c1_tolerance {
        return Err(error(
            PCURVE_AMBIGUOUS,
            format!("Seam join C1 jump {tangent_jump} exceeds {c1_tolerance}"),
        ));
    }
    let mut deduplicated = Vec::with_capacity(points.len());
    for point in points {
        if deduplicated
            .last()
            .is_none_or(|last: &[f64; 2]| distance(last, &point).map(|d| d > 0.).unwrap_or(false))
        {
            deduplicated.push(point);
        }
    }
    Ok(UnwrappedPcurve {
        curve: polyline(deduplicated)?,
        periodic_lift: lift,
        seam_gap: gap,
        tangent_jump,
    })
}

#[derive(Clone, Debug, PartialEq)]
pub struct UnwrappedPcurve {
    pub curve: Curve,
    pub periodic_lift: [[i32; 2]; 2],
    pub seam_gap: f64,
    pub tangent_jump: f64,
}

impl UnwrappedPcurve {
    pub fn evaluate_lifted(
        &self,
        surface: &Surface,
        parameter: f64,
        periods: SurfacePeriods,
    ) -> Result<Vec<f64>> {
        let uv = self.curve.evaluate(parameter)?.point;
        evaluate_uv(surface, [uv[0], uv[1]], periods)
    }
}

fn evaluate_uv(surface: &Surface, mut uv: [f64; 2], periods: SurfacePeriods) -> Result<Vec<f64>> {
    for axis in 0..2 {
        if let Some(period) = periods.axis(axis) {
            uv[axis] = period.wrap(uv[axis])?;
        }
    }
    Ok(surface.evaluate(uv[0], uv[1])?.point.to_vec())
}

#[derive(Clone, Debug, PartialEq)]
pub struct TrimReplay {
    pub persistent_id: String,
    pub pcurve: Curve,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SeamRelocation {
    pub new_origin: f64,
    pub surface_revision: u64,
    pub trims: Vec<TrimReplay>,
}

/// Relocate a U-chart origin and replay every trim into one continuous chart.
/// The persistent IDs are carried verbatim and the revision is the cache-key
/// invalidation token. A new seam through a trim interior is refused. Polyline
/// trims are exact; nonlinear modulo reparameterization is deliberately refused.
pub fn relocate_seam_u(
    old: PeriodicAxis,
    new_origin: f64,
    surface_revision: u64,
    trims: &[TrimReplay],
    tolerance: f64,
) -> Result<SeamRelocation> {
    if !new_origin.is_finite() || !tolerance.is_finite() || tolerance < 0. {
        return Err(error(PCURVE_INVALID, "Invalid seam origin or tolerance"));
    }
    let relocated_origin = old.wrap(new_origin)?;
    let mut replayed = Vec::with_capacity(trims.len());
    for trim in trims {
        if trim.persistent_id.is_empty() || trim.pcurve.degree != 1 || trim.pcurve.periodic {
            return Err(error(
                PCURVE_AMBIGUOUS,
                "Seam replay needs an ID and an exact nonperiodic polyline",
            ));
        }
        let source = curve_points(&trim.pcurve)?;
        let mut mapped = Vec::with_capacity(source.len());
        for (index, point) in source.iter().enumerate() {
            let unwrapped =
                relocated_origin + (point[0] - relocated_origin).rem_euclid(old.period());
            if index > 0
                && index + 1 < source.len()
                && (unwrapped - relocated_origin).min(old.period() - (unwrapped - relocated_origin))
                    <= tolerance
            {
                return Err(error(
                    PCURVE_AMBIGUOUS,
                    format!("New seam cuts trim {} internally", trim.persistent_id),
                ));
            }
            if let Some(previous) = mapped.last().map(|point: &[f64; 2]| point[0]) {
                let jump = (unwrapped - previous).abs();
                if jump > old.period() * 0.5 + tolerance {
                    return Err(error(
                        PCURVE_AMBIGUOUS,
                        format!("New seam cuts trim {} across a segment", trim.persistent_id),
                    ));
                }
            }
            mapped.push([unwrapped, point[1]]);
        }
        let (mapped, _) = unwrap_points(
            &mapped,
            SurfacePeriods {
                u: Some(PeriodicAxis::new(
                    relocated_origin,
                    relocated_origin + old.period(),
                )?),
                v: None,
            },
        )?;
        replayed.push(TrimReplay {
            persistent_id: trim.persistent_id.clone(),
            pcurve: polyline(mapped)?,
        });
    }
    Ok(SeamRelocation {
        new_origin: relocated_origin,
        surface_revision,
        trims: replayed,
    })
}

#[derive(Clone, Debug, PartialEq)]
pub struct PoleBranch {
    pub curve: Curve,
    /// One-sided derivative at the end away from the pole. There is deliberately
    /// no derivative at the degenerate pole itself.
    pub away_tangent: Vec<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PoleCrossing {
    pub incoming: PoleBranch,
    pub outgoing: PoleBranch,
    pub pole_uv: [f64; 2],
    pub pole_point: Vec<f64>,
}

/// Split at a caller-certified pole parameter. The pole is represented only by
/// its position; branch tangents are evaluated strictly away from it.
pub fn split_at_degenerate_pole(
    pcurve: &Curve,
    surface: &Surface,
    pole_uv: [f64; 2],
    pole_parameter: f64,
    position_tolerance: f64,
) -> Result<PoleCrossing> {
    pcurve.validate()?;
    surface.validate()?;
    if pole_uv.iter().any(|x| !x.is_finite())
        || !position_tolerance.is_finite()
        || position_tolerance < 0.
    {
        return Err(error(PCURVE_INVALID, "Invalid pole data"));
    }
    let [a, b] = pcurve.domain();
    if !pole_parameter.is_finite() || pole_parameter <= a || pole_parameter >= b {
        return Err(error(
            PCURVE_INVALID,
            "Pole parameter must be strictly interior",
        ));
    }
    let authored = pcurve.evaluate(pole_parameter)?.point;
    if distance(&authored, &pole_uv)? > position_tolerance {
        return Err(error(
            PCURVE_AMBIGUOUS,
            "Authored split point is not the certified pole UV",
        ));
    }
    let pole_point = surface.evaluate(pole_uv[0], pole_uv[1])?.point;
    let incoming_curve = pcurve.trim(a, pole_parameter)?;
    let outgoing_curve = pcurve.trim(pole_parameter, b)?;
    let ia = incoming_curve.domain()[0];
    let ob = outgoing_curve.domain()[1];
    let incoming_eval = incoming_curve.evaluate(ia)?;
    let outgoing_eval = outgoing_curve.evaluate(ob)?;
    let incoming_tangent = incoming_eval
        .d1
        .ok_or_else(|| error(PCURVE_INVALID, "Missing incoming derivative"))?;
    let outgoing_tangent = outgoing_eval
        .d1
        .ok_or_else(|| error(PCURVE_INVALID, "Missing outgoing derivative"))?;
    if norm(&incoming_tangent)? == 0. || norm(&outgoing_tangent)? == 0. {
        return Err(error(
            PCURVE_AMBIGUOUS,
            "A zero away-from-pole derivative is ambiguous",
        ));
    }
    Ok(PoleCrossing {
        incoming: PoleBranch {
            curve: incoming_curve,
            away_tangent: incoming_tangent,
        },
        outgoing: PoleBranch {
            curve: outgoing_curve,
            away_tangent: outgoing_tangent,
        },
        pole_uv,
        pole_point: pole_point.to_vec(),
    })
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AuditOptions {
    pub tolerance: f64,
    pub samples: usize,
}

impl Default for AuditOptions {
    fn default() -> Self {
        Self {
            tolerance: 1e-7,
            samples: AGREEMENT_SAMPLES,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AgreementAudit {
    pub consistent: bool,
    pub same_parameter: bool,
    pub max_distance: f64,
    pub worst_sample: usize,
    pub samples: usize,
}

fn lobatto(index: usize, count: usize) -> f64 {
    if index == 0 {
        0.
    } else if index + 1 == count {
        1.
    } else {
        (1. - (std::f64::consts::PI * index as f64 / (count - 1) as f64).cos()) * 0.5
    }
}

/// Check the actual STEP same-parameter contract at Chebyshev--Lobatto nodes.
/// A false `same_sense` reverses the 3D edge before comparison. Periodic UV is
/// wrapped only for surface evaluation, never in the stored pcurve.
pub fn audit_same_parameter(
    edge: &Curve,
    pcurve: &Curve,
    surface: &Surface,
    periods: SurfacePeriods,
    same_sense: bool,
    options: AuditOptions,
) -> Result<AgreementAudit> {
    edge.validate()?;
    pcurve.validate()?;
    surface.validate()?;
    if edge.control_points[0].len() != 3
        || pcurve.control_points[0].len() != 2
        || !options.tolerance.is_finite()
        || options.tolerance < 0.
        || !(2..=257).contains(&options.samples)
    {
        return Err(error(
            PCURVE_INVALID,
            "Invalid same-parameter audit input or sample budget",
        ));
    }
    let oriented_edge;
    let edge = if same_sense {
        edge
    } else {
        oriented_edge = edge.reverse()?;
        &oriented_edge
    };
    let ed = edge.domain();
    let pd = pcurve.domain();
    let mut max_distance = 0_f64;
    let mut worst_sample = 0;
    for index in 0..options.samples {
        let t = lobatto(index, options.samples);
        let world = edge.evaluate(ed[0] + t * (ed[1] - ed[0]))?.point;
        let uv = pcurve.evaluate(pd[0] + t * (pd[1] - pd[0]))?.point;
        let lifted = evaluate_uv(surface, [uv[0], uv[1]], periods)?;
        let d = distance(&world, &lifted)?;
        if d > max_distance {
            max_distance = d;
            worst_sample = index;
        }
    }
    Ok(AgreementAudit {
        consistent: max_distance <= options.tolerance,
        same_parameter: true,
        max_distance,
        worst_sample,
        samples: options.samples,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrimDecision {
    Preserved,
    RebuiltFromEdge,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StepTrimResolution {
    pub edge_id: String,
    pub pcurve: Curve,
    pub decision: TrimDecision,
    pub original: AgreementAudit,
    pub repaired: AgreementAudit,
    pub evaluations: usize,
}

/// Preserve a consistent imported pcurve. On mismatch, rebuild from the 3D
/// edge with the bounded certified pullback, then repeat the 33-node audit.
pub fn resolve_step_trim(
    edge_id: impl Into<String>,
    edge: &Curve,
    imported: &Curve,
    surface: &Surface,
    same_sense: bool,
    options: AuditOptions,
    pullback: curve_pullback::Limits,
) -> Result<StepTrimResolution> {
    let edge_id = edge_id.into();
    let original = audit_same_parameter(
        edge,
        imported,
        surface,
        SurfacePeriods::default(),
        same_sense,
        options,
    )?;
    if original.consistent {
        return Ok(StepTrimResolution {
            edge_id,
            pcurve: imported.clone(),
            decision: TrimDecision::Preserved,
            repaired: original.clone(),
            original,
            evaluations: 0,
        });
    }
    let oriented;
    let edge = if same_sense {
        edge
    } else {
        oriented = edge.reverse()?;
        &oriented
    };
    let report = curve_pullback::project(edge, surface, pullback)?;
    let pcurve = report.pcurve.ok_or_else(|| {
        error(
            PCURVE_AMBIGUOUS,
            format!("No unique certified pullback for edge {edge_id}"),
        )
    })?;
    let repaired = audit_same_parameter(
        edge,
        &pcurve,
        surface,
        SurfacePeriods::default(),
        true,
        options,
    )?;
    if !repaired.consistent {
        return Err(error(
            PCURVE_UNRESOLVED,
            format!("Pullback for edge {edge_id} failed the post-repair audit"),
        ));
    }
    Ok(StepTrimResolution {
        edge_id,
        pcurve,
        decision: TrimDecision::RebuiltFromEdge,
        original,
        repaired,
        evaluations: report.evaluations,
    })
}

#[path = "pcurve_repair/repair.rs"]
mod repair;
pub use repair::*;
