//! Bounded, direct Part 21 topology interchange.
//!
//! Unlike the compatibility `/1` and `/2` routes this module never recognizes
//! constructors or reconstructs an AABB.  It translates the selected
//! MANIFOLD_SOLID_BREP/BREP_WITH_VOIDS graph directly to/from `Model`.

use crate::analytic_features::FeatureCertificate;
use crate::step_part21::{
    Entity, MAX_GRAPH_DEPTH, Value, boolean, call, component, components, expand, ints, list, name,
    number, one_ref, parse, reachable, refs, string_value, usize_value, values,
};
pub(crate) use crate::step_v3_export::{
    Writer, canonical_face_senses, certify_periodic_surface, certify_step_v8_topology,
    emit_curve_source, emit_surface_source, refs_text,
};
pub use crate::step_v3_export::{
    export_step_v3, export_step_v4, export_step_v5, export_step_v6, export_step_v7, export_step_v8,
    export_step_v9,
};
use crate::{Body, Coedge, Edge, Face, FaceUse, Loop, Model, Shell, TopoId, TopoKind, TopologyIds};
use brep_topology::{CoedgeTrim, Vertex};
use nurbs_core::{Error, Result, curve::Curve, surface::Surface};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const STEP_INTERCHANGE_V3_CAPABILITY: &str = "step-interchange/3";
pub const STEP_INTERCHANGE_V4_CAPABILITY: &str = "step-interchange/4";
pub const STEP_INTERCHANGE_V5_CAPABILITY: &str = "step-interchange/5";
pub const STEP_INTERCHANGE_V6_CAPABILITY: &str = "step-interchange/6";
pub const STEP_INTERCHANGE_V7_CAPABILITY: &str = "step-interchange/7";
pub const STEP_INTERCHANGE_V8_CAPABILITY: &str = "step-interchange/8";
pub const STEP_INTERCHANGE_V9_CAPABILITY: &str = "step-interchange/9";
pub const STEP_INTERCHANGE_V10_CAPABILITY: &str = "step-interchange/10";
const MAX_OCCURRENCES: usize = 256;
pub(crate) const MAX_OUTPUT_BYTES: usize = 16 * 1024 * 1024;
// A missing uncertainty context retains the historical kernel default. Present
// contexts instead use their tightest valid measure, converted to millimetres.
const DEFAULT_STEP_TOLERANCE_MM: f64 = 1e-7;

type TopologyMapping = Vec<(TopoKind, usize, usize)>;

struct DirectBuildOptions {
    analytic_surfaces: bool,
    ap242_composition: bool,
    allow_degenerate: bool,
    whole_domain_proofs: bool,
    interior_point_selectors: bool,
    allow_open_shells: bool,
}

pub(crate) fn refuse(message: impl Into<String>) -> Error {
    Error::new("BREP_STEP_V3_REFUSED", message)
}

#[derive(Clone, Debug)]
pub struct StepV3Report {
    pub identity: crate::step_interchange::StepIdentityReport,
    pub ignored_entities: Vec<String>,
    pub metadata_loss: Vec<String>,
    pub instance_count: usize,
    pub reachable_count: usize,
    pub definition_identities: Vec<String>,
    pub occurrence_identities: Vec<String>,
    pub product_hierarchy: Vec<String>,
    pub external_references: Vec<String>,
}

/// Lossless `/10` product envelope. The parsed B-rep is a derived editing
/// projection; `source` remains authoritative for occurrence and presentation
/// graphs that cannot be represented by `Model`.
#[derive(Clone, Debug)]
pub struct StepV10Document {
    pub source: String,
    pub graph_identity: String,
    pub definition_identities: Vec<String>,
    pub occurrence_identities: Vec<String>,
    pub product_hierarchy: Vec<String>,
    pub operator_identities: Vec<String>,
    pub metadata_loss: Vec<String>,
}

/// Algebraic, whole-parameter-domain evidence carried by STEP `/8`.
///
/// `identity` names the exact rational tensor identity checked by the kernel;
/// it is deliberately not a sampling result.  A zero Jacobian lower bound is
/// permitted only for an explicitly named collapsed boundary.  In that case
/// `regular_open_domain` certifies the punctured parameter neighbourhood and
/// topology validation certifies that the whole boundary is one pole vertex.
#[derive(Clone, Debug)]
pub struct StepRegularityEvidence {
    pub carrier: &'static str,
    pub parameter_u: [f64; 2],
    pub parameter_v: [f64; 2],
    pub lifted_periods: [f64; 2],
    pub denominator_lower_bound: f64,
    pub jacobian_lower_bound: f64,
    pub collapsed_boundaries: Vec<&'static str>,
    pub regular_open_domain: bool,
    pub identity: &'static str,
}

#[derive(Clone, Debug)]
pub struct StepV8Certificate {
    pub capability: &'static str,
    pub complete: bool,
    pub regularity: Vec<StepRegularityEvidence>,
    pub sense_layers: [&'static str; 7],
    pub coupled_sense_cases: usize,
    pub notes: Vec<&'static str>,
}

fn point(
    entities: &BTreeMap<usize, Entity>,
    id: usize,
    dim: usize,
    scale: f64,
) -> Result<Vec<f64>> {
    let (ty, args) = call(entities, id)?;
    if ty != "CARTESIAN_POINT" || args.len() != 2 {
        return Err(refuse("Expected CARTESIAN_POINT"));
    }
    let mut p = values(&args[1], "CARTESIAN_POINT coordinates")?;
    if p.len() != dim {
        return Err(refuse("CARTESIAN_POINT dimension mismatch"));
    }
    if dim == 3 {
        for x in &mut p {
            *x *= scale;
        }
    }
    Ok(p)
}
fn direction(entities: &BTreeMap<usize, Entity>, id: usize, dim: usize) -> Result<Vec<f64>> {
    let (ty, args) = call(entities, id)?;
    if ty != "DIRECTION" || args.len() != 2 {
        return Err(refuse("Expected DIRECTION"));
    }
    let d = values(&args[1], "DIRECTION ratios")?;
    if d.len() != dim {
        return Err(refuse("DIRECTION dimension mismatch"));
    }
    let n = d.iter().map(|x| x * x).sum::<f64>().sqrt();
    if !n.is_finite() || n <= 1e-12 {
        return Err(refuse("Degenerate DIRECTION"));
    }
    Ok(d.into_iter().map(|x| x / n).collect())
}
fn vector(
    entities: &BTreeMap<usize, Entity>,
    id: usize,
    dim: usize,
    scale: f64,
) -> Result<Vec<f64>> {
    let (ty, args) = call(entities, id)?;
    if ty != "VECTOR" || args.len() != 3 {
        return Err(refuse("Expected VECTOR"));
    }
    let d = direction(entities, one_ref(&args[1], "VECTOR orientation")?, dim)?;
    let magnitude = number(&args[2], "VECTOR magnitude")? * if dim == 3 { scale } else { 1. };
    if magnitude <= 0. {
        return Err(refuse("VECTOR magnitude must be positive"));
    }
    Ok(d.into_iter().map(|x| x * magnitude).collect())
}
fn axis2(
    entities: &BTreeMap<usize, Entity>,
    id: usize,
    dim: usize,
    scale: f64,
) -> Result<(Vec<f64>, Vec<f64>, Vec<f64>)> {
    let (ty, args) = call(entities, id)?;
    let expected = if dim == 3 {
        "AXIS2_PLACEMENT_3D"
    } else {
        "AXIS2_PLACEMENT_2D"
    };
    if ty != expected || args.len() < 2 {
        return Err(refuse(format!("Expected {expected}")));
    }
    let origin = point(entities, one_ref(&args[1], "AXIS2 location")?, dim, scale)?;
    if dim == 2 {
        let x = if args.len() > 2 && !matches!(args[2], Value::Omitted) {
            direction(entities, one_ref(&args[2], "AXIS2 ref direction")?, 2)?
        } else {
            vec![1., 0.]
        };
        return Ok((origin, x.clone(), vec![-x[1], x[0]]));
    }
    let z = if args.len() > 2 && !matches!(args[2], Value::Omitted) {
        direction(entities, one_ref(&args[2], "AXIS2 axis")?, 3)?
    } else {
        vec![0., 0., 1.]
    };
    let mut x = if args.len() > 3 && !matches!(args[3], Value::Omitted) {
        direction(entities, one_ref(&args[3], "AXIS2 ref direction")?, 3)?
    } else {
        vec![1., 0., 0.]
    };
    let dot = x.iter().zip(&z).map(|(a, b)| a * b).sum::<f64>();
    for i in 0..3 {
        x[i] -= dot * z[i];
    }
    let nx = x.iter().map(|v| v * v).sum::<f64>().sqrt();
    if nx <= 1e-12 {
        return Err(refuse("AXIS2 directions are parallel"));
    }
    for v in &mut x {
        *v /= nx;
    }
    let y = vec![
        z[1] * x[2] - z[2] * x[1],
        z[2] * x[0] - z[0] * x[2],
        z[0] * x[1] - z[1] * x[0],
    ];
    Ok((origin, x, y))
}
fn conic_curve(origin: &[f64], x: &[f64], y: &[f64], a: f64, b: f64) -> Result<Curve> {
    if !(a > 0. && b > 0. && a.is_finite() && b.is_finite()) {
        return Err(refuse("Conic radii must be positive"));
    }
    let endpoints = [[1., 0.], [0., 1.], [-1., 0.], [0., -1.], [1., 0.]];
    let shoulders = [[1., 1.], [-1., 1.], [-1., -1.], [1., -1.]];
    let map = |q: [f64; 2]| {
        (0..origin.len())
            .map(|i| origin[i] + a * q[0] * x[i] + b * q[1] * y[i])
            .collect::<Vec<_>>()
    };
    let mut cps = vec![map(endpoints[0])];
    let mut weights = vec![1.];
    for i in 0..4 {
        cps.push(map(shoulders[i]));
        weights.push(std::f64::consts::FRAC_1_SQRT_2);
        cps.push(map(endpoints[i + 1]));
        weights.push(1.);
    }
    let c = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 2., 2., 3., 3., 4., 4., 4.],
        control_points: cps,
        weights,
        periodic: false,
    };
    c.validate()?;
    Ok(c)
}
/// Isolate a point inverse with positive-weight rational convex-hull
/// exclusion.  Every retained interval is produced by exact knot insertion
/// (`Curve::trim`); therefore its Euclidean control hull encloses the complete
/// rational image.  Ambiguous, non-isolated and non-representable roots refuse.
fn isolate_curve_point(base: &Curve, target: &[f64]) -> Result<f64> {
    const MAX_CELLS: usize = 16_384;
    const PARAMETER_WIDTH: f64 = 1e-12;
    const GEOMETRIC_WIDTH: f64 = 1e-9;
    fn contains(curve: &Curve, target: &[f64]) -> bool {
        (0..target.len()).all(|axis| {
            let lo = curve
                .control_points
                .iter()
                .map(|point| point[axis])
                .fold(f64::INFINITY, f64::min);
            let hi = curve
                .control_points
                .iter()
                .map(|point| point[axis])
                .fold(f64::NEG_INFINITY, f64::max);
            target[axis] >= lo - 1e-12 && target[axis] <= hi + 1e-12
        })
    }
    let domain = base.domain();
    let mut breaks = base
        .knots
        .iter()
        .copied()
        .filter(|value| *value >= domain[0] && *value <= domain[1])
        .collect::<Vec<_>>();
    breaks.dedup();
    let mut pending = breaks
        .array_windows()
        .map(|[a, b]| base.trim(*a, *b))
        .collect::<Result<Vec<_>>>()?;
    let mut isolated = Vec::<[f64; 2]>::new();
    let mut visited = 0usize;
    while let Some(cell) = pending.pop() {
        visited += 1;
        if visited > MAX_CELLS {
            return Err(refuse(
                "TRIMMED_CURVE point inverse exceeds certified isolation budget",
            ));
        }
        if !contains(&cell, target) {
            continue;
        }
        let domain = cell.domain();
        let hull_width = (0..target.len())
            .map(|axis| {
                let lo = cell
                    .control_points
                    .iter()
                    .map(|point| point[axis])
                    .fold(f64::INFINITY, f64::min);
                let hi = cell
                    .control_points
                    .iter()
                    .map(|point| point[axis])
                    .fold(f64::NEG_INFINITY, f64::max);
                hi - lo
            })
            .fold(0., f64::max);
        if domain[1] - domain[0] <= PARAMETER_WIDTH || hull_width <= GEOMETRIC_WIDTH {
            isolated.push(domain);
            continue;
        }
        let mid = (domain[0] + domain[1]) / 2.;
        if mid == domain[0] || mid == domain[1] {
            return Err(refuse(
                "TRIMMED_CURVE point inverse exhausted parameter precision",
            ));
        }
        let [left, right] = cell.split(mid)?;
        pending.push(right);
        pending.push(left);
    }
    isolated.sort_by(|left, right| left[0].total_cmp(&right[0]));
    let mut merged = Vec::<[f64; 2]>::new();
    for interval in isolated {
        if let Some(last) = merged.last_mut()
            && interval[0] <= last[1] + PARAMETER_WIDTH
        {
            last[1] = last[1].max(interval[1]);
            continue;
        }
        merged.push(interval)
    }
    if merged.len() != 1 {
        return Err(refuse(if merged.is_empty() {
            "TRIMMED_CURVE point selector is not on the basis curve"
        } else {
            "TRIMMED_CURVE point selector inverse is not unique"
        }));
    }
    let interval = merged[0];
    let mut parameter = (interval[0] + interval[1]) / 2.;
    for _ in 0..12 {
        let evaluation = base.evaluate(parameter)?;
        let Some(derivative) = evaluation.d1 else {
            break;
        };
        let denominator = derivative.iter().map(|value| value * value).sum::<f64>();
        if denominator <= 1e-24 {
            break;
        }
        let correction = evaluation
            .point
            .iter()
            .zip(target)
            .zip(&derivative)
            .map(|((point, wanted), tangent)| (point - wanted) * tangent)
            .sum::<f64>()
            / denominator;
        let next = (parameter - correction).clamp(interval[0], interval[1]);
        if next == parameter {
            break;
        }
        parameter = next;
    }
    let residual = base
        .evaluate(parameter)?
        .point
        .iter()
        .zip(target)
        .map(|(point, wanted)| (point - wanted) * (point - wanted))
        .sum::<f64>()
        .sqrt();
    if !residual.is_finite() || residual > GEOMETRIC_WIDTH {
        return Err(refuse(
            "TRIMMED_CURVE isolated point inverse failed certified residual bound",
        ));
    }
    Ok(parameter)
}
fn trim_parameters(
    entities: &BTreeMap<usize, Entity>,
    v: &Value,
    base: &Curve,
    dim: usize,
    scale: f64,
    interior_points: bool,
) -> Result<f64> {
    let items = list(v, "TRIMMED_CURVE trim selector")?;
    if items.len() != 1 {
        return Err(refuse(
            "TRIMMED_CURVE admits one parameter selector per end",
        ));
    }
    match &items[0] {
        Value::Number(_) | Value::Integer(_) => number(&items[0], "TRIMMED_CURVE parameter"),
        Value::Call(name, args) if name == "PARAMETER_VALUE" && args.len() == 1 => {
            number(&args[0], "PARAMETER_VALUE")
        }
        Value::Ref(id) => {
            let target = point(entities, *id, dim, scale)?;
            let domain = base.domain();
            let start = base.evaluate(domain[0])?.point;
            let end = base.evaluate(domain[1])?.point;
            let distance = |p: &[f64]| {
                p.iter()
                    .zip(&target)
                    .map(|(a, b)| (a - b) * (a - b))
                    .sum::<f64>()
                    .sqrt()
            };
            if distance(&start) <= 1e-7 {
                Ok(domain[0])
            } else if distance(&end) <= 1e-7 {
                Ok(domain[1])
            } else if interior_points {
                isolate_curve_point(base, &target)
            } else {
                Err(refuse(
                    "TRIMMED_CURVE point selector is not an exact basis endpoint",
                ))
            }
        }
        _ => Err(refuse(
            "TRIMMED_CURVE selector is outside the parameter/exact-endpoint subset",
        )),
    }
}
fn trim_conic(base: &Curve, start: f64, end: f64, sense: bool) -> Result<Curve> {
    let mut a0 = start;
    let mut a1 = end;
    if !sense {
        std::mem::swap(&mut a0, &mut a1);
    }
    while a1 <= a0 {
        a1 += std::f64::consts::TAU;
    }
    if a1 - a0 > std::f64::consts::TAU + 1e-10 {
        return Err(refuse("TRIMMED_CURVE conic sweep exceeds one revolution"));
    }
    let center = {
        let p0 = &base.control_points[0];
        let p4 = &base.control_points[4];
        p0.iter()
            .zip(p4)
            .map(|(a, b)| (a + b) / 2.)
            .collect::<Vec<_>>()
    };
    let ex: Vec<_> = base.control_points[0]
        .iter()
        .zip(&center)
        .map(|(a, b)| a - b)
        .collect();
    let ey: Vec<_> = base.control_points[2]
        .iter()
        .zip(&center)
        .map(|(a, b)| a - b)
        .collect();
    let pieces = ((a1 - a0) / std::f64::consts::FRAC_PI_2).ceil().max(1.) as usize;
    let mut cps = Vec::new();
    let mut weights = Vec::new();
    let mut knots = vec![0., 0., 0.];
    for i in 0..pieces {
        let s = a0 + (a1 - a0) * i as f64 / pieces as f64;
        let e = a0 + (a1 - a0) * (i + 1) as f64 / pieces as f64;
        let m = (s + e) / 2.;
        let w = ((e - s) / 2.).cos();
        let p = |angle: f64, factor: f64| {
            center
                .iter()
                .enumerate()
                .map(|(j, c)| c + factor * (ex[j] * angle.cos() + ey[j] * angle.sin()))
                .collect::<Vec<_>>()
        };
        if i == 0 {
            cps.push(p(s, 1.));
            weights.push(1.);
        }
        cps.push(p(m, 1. / w));
        weights.push(w);
        cps.push(p(e, 1.));
        weights.push(1.);
        if i + 1 < pieces {
            knots.push((i + 1) as f64);
            knots.push((i + 1) as f64);
        }
    }
    knots.extend([pieces as f64; 3]);
    let c = Curve {
        degree: 2,
        knots,
        control_points: cps,
        weights,
        periodic: false,
    };
    c.validate()?;
    Ok(c)
}
fn curve_with_selectors(
    entities: &BTreeMap<usize, Entity>,
    id: usize,
    dim: usize,
    scale: f64,
    interior_points: bool,
) -> Result<Curve> {
    let (ty, a) = call(entities, id)?;
    if ty == "LINE" {
        if a.len() != 3 {
            return Err(refuse("LINE argument count mismatch"));
        }
        let p = point(entities, one_ref(&a[1], "LINE point")?, dim, scale)?;
        let d = vector(entities, one_ref(&a[2], "LINE vector")?, dim, scale)?;
        return Curve::from_polyline(vec![
            p.clone(),
            p.iter().zip(d).map(|(x, d)| x + d).collect(),
        ]);
    }
    if matches!(ty, "CIRCLE" | "ELLIPSE") {
        let expected = if ty == "CIRCLE" { 3 } else { 4 };
        if a.len() != expected {
            return Err(refuse(format!("{ty} argument count mismatch")));
        }
        let (o, x, y) = axis2(entities, one_ref(&a[1], "conic placement")?, dim, scale)?;
        let major = number(&a[2], "conic radius")? * if dim == 3 { scale } else { 1. };
        let minor = if ty == "ELLIPSE" {
            number(&a[3], "ellipse minor radius")? * if dim == 3 { scale } else { 1. }
        } else {
            major
        };
        return conic_curve(&o, &x, &y, major, minor);
    }
    if ty == "TRIMMED_CURVE" {
        if a.len() != 6 {
            return Err(refuse("TRIMMED_CURVE argument count mismatch"));
        }
        let basis_id = one_ref(&a[1], "TRIMMED_CURVE basis")?;
        let (basis_ty, _) = call(entities, basis_id)?;
        let base = curve_with_selectors(entities, basis_id, dim, scale, interior_points)?;
        let start = trim_parameters(entities, &a[2], &base, dim, scale, interior_points)?;
        let end = trim_parameters(entities, &a[3], &base, dim, scale, interior_points)?;
        let sense = boolean(&a[4], "TRIMMED_CURVE sense")?;
        if matches!(basis_ty, "CIRCLE" | "ELLIPSE") {
            return trim_conic(&base, start, end, sense);
        }
        if basis_ty == "LINE" {
            let p0 = base.evaluate(start)?.point;
            let p1 = base.evaluate(end)?.point;
            return Curve::from_polyline(if sense { vec![p0, p1] } else { vec![p1, p0] });
        }
        return Err(refuse(
            "TRIMMED_CURVE basis is outside the exact direct subset",
        ));
    }
    if ty != "B_SPLINE_CURVE_WITH_KNOTS" {
        return Err(refuse(format!(
            "Reachable curve {ty} is typed-refused by the direct native subset"
        )));
    }
    if a.len() != 11 {
        return Err(refuse("B_SPLINE_CURVE_WITH_KNOTS argument count mismatch"));
    }
    let degree = usize_value(&a[1], "curve degree")?;
    let cps = list(&a[2], "curve control points")?
        .iter()
        .map(|v| point(entities, one_ref(v, "curve control point")?, dim, scale))
        .collect::<Result<Vec<_>>>()?;
    let weights = values(&a[7], "curve weights")?;
    let knots = expand(
        &ints(&a[8], "curve multiplicities")?,
        &values(&a[9], "curve knots")?,
    )?;
    let c = Curve {
        degree,
        knots,
        control_points: cps,
        weights,
        periodic: boolean(&a[5], "curve closed flag")?,
    };
    c.validate()?;
    Ok(c)
}
fn curve(entities: &BTreeMap<usize, Entity>, id: usize, dim: usize, scale: f64) -> Result<Curve> {
    curve_with_selectors(entities, id, dim, scale, false)
}
fn curve_v5(
    entities: &BTreeMap<usize, Entity>,
    id: usize,
    dim: usize,
    scale: f64,
    interior_points: bool,
) -> Result<Curve> {
    let parts = components(entities, id)?;
    if parts.len() == 1 && !matches!(parts[0].0, "B_SPLINE_CURVE_WITH_KNOTS") {
        return curve_with_selectors(entities, id, dim, scale, interior_points);
    }
    let (degree, cp_values, closed, multiplicities, knot_values, weights) = if parts.len() == 1 {
        let a = parts[0].1;
        if a.len() != 9 {
            return Err(refuse(
                "AP242 B_SPLINE_CURVE_WITH_KNOTS requires nine inherited/explicit attributes",
            ));
        }
        (
            usize_value(&a[1], "curve degree")?,
            &a[2],
            boolean(&a[4], "curve closed flag")?,
            &a[6],
            &a[7],
            None,
        )
    } else {
        let base = component(&parts, "B_SPLINE_CURVE").ok_or_else(|| {
            refuse("RATIONAL_B_SPLINE_CURVE complex lacks B_SPLINE_CURVE component")
        })?;
        let knots = component(&parts, "B_SPLINE_CURVE_WITH_KNOTS")
            .ok_or_else(|| refuse("RATIONAL_B_SPLINE_CURVE complex lacks knot component"))?;
        let rational = component(&parts, "RATIONAL_B_SPLINE_CURVE").ok_or_else(|| {
            refuse("Complex B-spline curve lacks RATIONAL_B_SPLINE_CURVE component")
        })?;
        if base.len() != 5 || knots.len() != 3 || rational.len() != 1 {
            return Err(refuse(
                "Malformed AP242 rational B-spline curve composition",
            ));
        }
        (
            usize_value(&base[0], "curve degree")?,
            &base[1],
            boolean(&base[3], "curve closed flag")?,
            &knots[0],
            &knots[1],
            Some(&rational[0]),
        )
    };
    let cps = list(cp_values, "curve control points")?
        .iter()
        .map(|v| point(entities, one_ref(v, "curve control point")?, dim, scale))
        .collect::<Result<Vec<_>>>()?;
    let weights = match weights {
        Some(weight_data) => values(weight_data, "curve weights")?,
        None => vec![1.; cps.len()],
    };
    if weights.len() != cps.len() {
        return Err(refuse(
            "RATIONAL_B_SPLINE_CURVE weight/control-point count mismatch",
        ));
    }
    let c = Curve {
        degree,
        knots: expand(
            &ints(multiplicities, "curve multiplicities")?,
            &values(knot_values, "curve knots")?,
        )?,
        control_points: cps,
        weights,
        periodic: closed,
    };
    c.validate()?;
    Ok(c)
}
fn analytic_surface(
    entities: &BTreeMap<usize, Entity>,
    ty: &str,
    a: &[Value],
    scale: f64,
) -> Result<Surface> {
    let expected = if matches!(ty, "TOROIDAL_SURFACE" | "CONICAL_SURFACE") {
        4
    } else {
        3
    };
    if a.len() != expected {
        return Err(refuse(format!("{ty} argument count mismatch")));
    }
    let (o, x, y) = axis2(
        entities,
        one_ref(&a[1], "analytic surface placement")?,
        3,
        scale,
    )?;
    let z = [
        x[1] * y[2] - x[2] * y[1],
        x[2] * y[0] - x[0] * y[2],
        x[0] * y[1] - x[1] * y[0],
    ];
    let radius = number(&a[2], "analytic surface radius")? * scale;
    if !radius.is_finite() || radius <= 0. {
        return Err(refuse("Analytic surface radius must be positive"));
    }
    let angles = [
        0.,
        std::f64::consts::FRAC_PI_4,
        std::f64::consts::FRAC_PI_2,
        3. * std::f64::consts::FRAC_PI_4,
        std::f64::consts::PI,
        5. * std::f64::consts::FRAC_PI_4,
        3. * std::f64::consts::FRAC_PI_2,
        7. * std::f64::consts::FRAC_PI_4,
        std::f64::consts::TAU,
    ];
    let wu = [
        1.,
        std::f64::consts::FRAC_1_SQRT_2,
        1.,
        std::f64::consts::FRAC_1_SQRT_2,
        1.,
        std::f64::consts::FRAC_1_SQRT_2,
        1.,
        std::f64::consts::FRAC_1_SQRT_2,
        1.,
    ];
    let ku = vec![
        0.,
        0.,
        0.,
        std::f64::consts::FRAC_PI_2,
        std::f64::consts::FRAC_PI_2,
        std::f64::consts::PI,
        std::f64::consts::PI,
        3. * std::f64::consts::FRAC_PI_2,
        3. * std::f64::consts::FRAC_PI_2,
        std::f64::consts::TAU,
        std::f64::consts::TAU,
        std::f64::consts::TAU,
    ];
    let (v_angles, wv, kv) = if matches!(ty, "SPHERICAL_SURFACE") {
        (
            vec![
                -std::f64::consts::FRAC_PI_2,
                -std::f64::consts::FRAC_PI_4,
                0.,
                std::f64::consts::FRAC_PI_4,
                std::f64::consts::FRAC_PI_2,
            ],
            vec![
                1.,
                std::f64::consts::FRAC_1_SQRT_2,
                1.,
                std::f64::consts::FRAC_1_SQRT_2,
                1.,
            ],
            vec![
                -std::f64::consts::FRAC_PI_2,
                -std::f64::consts::FRAC_PI_2,
                -std::f64::consts::FRAC_PI_2,
                0.,
                0.,
                std::f64::consts::FRAC_PI_2,
                std::f64::consts::FRAC_PI_2,
                std::f64::consts::FRAC_PI_2,
            ],
        )
    } else if ty == "TOROIDAL_SURFACE" {
        (angles.to_vec(), wu.to_vec(), ku.clone())
    } else {
        (vec![-1e6, 1e6], vec![1., 1.], vec![-1e6, -1e6, 1e6, 1e6])
    };
    let minor = if ty == "TOROIDAL_SURFACE" {
        let r = number(&a[3], "torus minor radius")? * scale;
        if !(r > 0. && r < radius) {
            return Err(refuse("TOROIDAL_SURFACE requires 0 < minor < major"));
        }
        r
    } else {
        radius
    };
    let semi = if ty == "CONICAL_SURFACE" {
        number(&a[3], "cone semi-angle")?
    } else {
        0.
    };
    let mut cps = Vec::new();
    let mut weights = Vec::new();
    for (iu, &angle) in angles.iter().enumerate() {
        let radial = [
            x[0] * angle.cos() + y[0] * angle.sin(),
            x[1] * angle.cos() + y[1] * angle.sin(),
            x[2] * angle.cos() + y[2] * angle.sin(),
        ];
        let mut row = Vec::new();
        let mut wr = Vec::new();
        for (iv, &v) in v_angles.iter().enumerate() {
            let shoulder_scale = if iu % 2 == 1 { 1. / wu[iu] } else { 1. };
            let meridian_scale =
                if iv % 2 == 1 && matches!(ty, "SPHERICAL_SURFACE" | "TOROIDAL_SURFACE") {
                    1. / wv[iv]
                } else {
                    1.
                };
            let (radial_distance, height) = match ty {
                "CYLINDRICAL_SURFACE" => (radius * shoulder_scale, v),
                "CONICAL_SURFACE" => ((radius + v * semi.tan()) * shoulder_scale, v),
                "SPHERICAL_SURFACE" => (
                    radius * v.cos() * shoulder_scale * meridian_scale,
                    radius * v.sin() * meridian_scale,
                ),
                "TOROIDAL_SURFACE" => (
                    (radius + minor * v.cos() * meridian_scale) * shoulder_scale,
                    minor * v.sin() * meridian_scale,
                ),
                _ => return Err(refuse("Unsupported analytic surface")),
            };
            row.push(
                (0..3)
                    .map(|j| o[j] + radial_distance * radial[j] + height * z[j])
                    .collect(),
            );
            wr.push(wu[iu] * wv[iv]);
        }
        cps.push(row);
        weights.push(wr);
    }
    // Full rational circles are clamped at an explicit seam. Periodic lifts
    // remain carried by CoedgeTrim; marking this net periodic would require
    // repeating `degree` control rows and would change the STEP angle domain.
    let s = Surface {
        degree_u: 2,
        degree_v: if v_angles.len() > 2 { 2 } else { 1 },
        knots_u: ku,
        knots_v: kv,
        control_points: cps,
        weights,
        periodic_u: false,
        periodic_v: false,
    };
    s.validate()?;
    Ok(s)
}
fn surface(entities: &BTreeMap<usize, Entity>, id: usize, scale: f64) -> Result<Surface> {
    let (ty, a) = call(entities, id)?;
    if ty == "PLANE" {
        if a.len() != 2 {
            return Err(refuse("PLANE argument count mismatch"));
        }
        let (o, x, y) = axis2(entities, one_ref(&a[1], "PLANE placement")?, 3, scale)?;
        let p = |u: f64, v: f64| {
            (0..3)
                .map(|i| o[i] + u * x[i] + v * y[i])
                .collect::<Vec<_>>()
        };
        const LIMIT: f64 = 1e6;
        let s = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![-LIMIT, -LIMIT, LIMIT, LIMIT],
            knots_v: vec![-LIMIT, -LIMIT, LIMIT, LIMIT],
            control_points: vec![
                vec![p(-LIMIT, -LIMIT), p(-LIMIT, LIMIT)],
                vec![p(LIMIT, -LIMIT), p(LIMIT, LIMIT)],
            ],
            weights: vec![vec![1., 1.], vec![1., 1.]],
            periodic_u: false,
            periodic_v: false,
        };
        s.validate()?;
        return Ok(s);
    }
    if matches!(
        ty,
        "CYLINDRICAL_SURFACE" | "CONICAL_SURFACE" | "SPHERICAL_SURFACE" | "TOROIDAL_SURFACE"
    ) {
        return Err(refuse(format!(
            "{ty} periodic/angular parameterization is available only in step-interchange/4"
        )));
    }
    if ty != "B_SPLINE_SURFACE_WITH_KNOTS" {
        return Err(refuse(format!(
            "Reachable surface {ty} is typed-refused by the direct native subset"
        )));
    }
    if a.len() != 16 {
        return Err(refuse(
            "B_SPLINE_SURFACE_WITH_KNOTS argument count mismatch",
        ));
    }
    let rows = list(&a[3], "surface control net")?;
    let cps = rows
        .iter()
        .map(|row| {
            list(row, "surface control row")?
                .iter()
                .map(|v| point(entities, one_ref(v, "surface control point")?, 3, scale))
                .collect::<Result<Vec<_>>>()
        })
        .collect::<Result<Vec<_>>>()?;
    let flat = values(&a[10], "surface weights")?;
    let nu = cps.len();
    let nv = cps.first().map_or(0, Vec::len);
    if nu == 0 || nv == 0 || cps.iter().any(|r| r.len() != nv) || flat.len() != nu * nv {
        return Err(refuse("Surface control/weight shape mismatch"));
    }
    let weights = flat.chunks(nv).map(|r| r.to_vec()).collect();
    let s = Surface {
        degree_u: usize_value(&a[1], "surface U degree")?,
        degree_v: usize_value(&a[2], "surface V degree")?,
        knots_u: expand(
            &ints(&a[11], "surface U multiplicities")?,
            &values(&a[13], "surface U knots")?,
        )?,
        knots_v: expand(
            &ints(&a[12], "surface V multiplicities")?,
            &values(&a[14], "surface V knots")?,
        )?,
        control_points: cps,
        weights,
        periodic_u: boolean(&a[6], "surface U closed flag")?,
        periodic_v: boolean(&a[7], "surface V closed flag")?,
    };
    s.validate()?;
    Ok(s)
}
fn surface_v4(entities: &BTreeMap<usize, Entity>, id: usize, scale: f64) -> Result<Surface> {
    let (ty, a) = call(entities, id)?;
    if matches!(
        ty,
        "CYLINDRICAL_SURFACE" | "CONICAL_SURFACE" | "SPHERICAL_SURFACE" | "TOROIDAL_SURFACE"
    ) {
        analytic_surface(entities, ty, a, scale)
    } else {
        surface(entities, id, scale)
    }
}
fn surface_v5(entities: &BTreeMap<usize, Entity>, id: usize, scale: f64) -> Result<Surface> {
    let parts = components(entities, id)?;
    if parts.len() == 1 && !matches!(parts[0].0, "B_SPLINE_SURFACE_WITH_KNOTS") {
        return surface_v4(entities, id, scale);
    }
    let (du, dv, net, pu, pv, mu, mv, ku, kv, weight_value) = if parts.len() == 1 {
        let a = parts[0].1;
        if a.len() != 13 {
            return Err(refuse(
                "AP242 B_SPLINE_SURFACE_WITH_KNOTS requires thirteen inherited/explicit attributes",
            ));
        }
        (
            usize_value(&a[1], "surface U degree")?,
            usize_value(&a[2], "surface V degree")?,
            &a[3],
            boolean(&a[5], "surface U closed flag")?,
            boolean(&a[6], "surface V closed flag")?,
            &a[8],
            &a[9],
            &a[10],
            &a[11],
            None,
        )
    } else {
        let base = component(&parts, "B_SPLINE_SURFACE").ok_or_else(|| {
            refuse("RATIONAL_B_SPLINE_SURFACE complex lacks B_SPLINE_SURFACE component")
        })?;
        let knots = component(&parts, "B_SPLINE_SURFACE_WITH_KNOTS")
            .ok_or_else(|| refuse("RATIONAL_B_SPLINE_SURFACE complex lacks knot component"))?;
        let rational = component(&parts, "RATIONAL_B_SPLINE_SURFACE").ok_or_else(|| {
            refuse("Complex B-spline surface lacks RATIONAL_B_SPLINE_SURFACE component")
        })?;
        if base.len() != 7 || knots.len() != 5 || rational.len() != 1 {
            return Err(refuse(
                "Malformed AP242 rational B-spline surface composition",
            ));
        }
        (
            usize_value(&base[0], "surface U degree")?,
            usize_value(&base[1], "surface V degree")?,
            &base[2],
            boolean(&base[4], "surface U closed flag")?,
            boolean(&base[5], "surface V closed flag")?,
            &knots[0],
            &knots[1],
            &knots[2],
            &knots[3],
            Some(&rational[0]),
        )
    };
    let cps = list(net, "surface control net")?
        .iter()
        .map(|row| {
            list(row, "surface control row")?
                .iter()
                .map(|v| point(entities, one_ref(v, "surface control point")?, 3, scale))
                .collect::<Result<Vec<_>>>()
        })
        .collect::<Result<Vec<_>>>()?;
    let nu = cps.len();
    let nv = cps.first().map_or(0, Vec::len);
    if nu == 0 || nv == 0 || cps.iter().any(|row| row.len() != nv) {
        return Err(refuse("Surface control net is empty or ragged"));
    }
    let weights = if let Some(value) = weight_value {
        let rows = list(value, "surface weights")?;
        if rows.len() != nu {
            return Err(refuse(
                "RATIONAL_B_SPLINE_SURFACE weight row count mismatch",
            ));
        }
        rows.iter()
            .map(|row| values(row, "surface weight row"))
            .collect::<Result<Vec<_>>>()?
    } else {
        vec![vec![1.; nv]; nu]
    };
    if weights.iter().any(|row| row.len() != nv) {
        return Err(refuse(
            "RATIONAL_B_SPLINE_SURFACE weight/control-net shape mismatch",
        ));
    }
    let s = Surface {
        degree_u: du,
        degree_v: dv,
        knots_u: expand(
            &ints(mu, "surface U multiplicities")?,
            &values(ku, "surface U knots")?,
        )?,
        knots_v: expand(
            &ints(mv, "surface V multiplicities")?,
            &values(kv, "surface V knots")?,
        )?,
        control_points: cps,
        weights,
        periodic_u: pu,
        periodic_v: pv,
    };
    s.validate()?;
    Ok(s)
}

mod units;
#[cfg(test)]
use units::unit_scale;
use units::{length_scale, uncertainty_mm};

fn part21_views(text: &str) -> (String, String) {
    let bytes = text.as_bytes();
    let mut clean = bytes.to_vec();
    let mut structural = bytes.to_vec();
    let mut index = 0;
    let mut in_string = false;
    while index < bytes.len() {
        if !in_string && index + 1 < bytes.len() && bytes[index] == b'/' && bytes[index + 1] == b'*'
        {
            clean[index] = b' ';
            clean[index + 1] = b' ';
            structural[index] = b' ';
            structural[index + 1] = b' ';
            index += 2;
            while index + 1 < bytes.len() && !(bytes[index] == b'*' && bytes[index + 1] == b'/') {
                if bytes[index] != b'\n' {
                    clean[index] = b' ';
                    structural[index] = b' ';
                }
                index += 1;
            }
            if index + 1 < bytes.len() {
                clean[index] = b' ';
                clean[index + 1] = b' ';
                structural[index] = b' ';
                structural[index + 1] = b' ';
                index += 2
            }
            continue;
        }
        if bytes[index] == b'\'' {
            if in_string && index + 1 < bytes.len() && bytes[index + 1] == b'\'' {
                structural[index] = b' ';
                structural[index + 1] = b' ';
                index += 2;
                continue;
            }
            in_string = !in_string;
            structural[index] = b' ';
            index += 1;
            continue;
        }
        if in_string && bytes[index] != b'\n' {
            structural[index] = b' '
        }
        index += 1;
    }
    (
        String::from_utf8_lossy(&clean).to_ascii_uppercase(),
        String::from_utf8_lossy(&structural).to_ascii_uppercase(),
    )
}
fn validate_exchange_structure(text: &str) -> Result<()> {
    let (upper, structural) = part21_views(text);
    for section in ["ISO-10303-21;", "HEADER;", "DATA;", "END-ISO-10303-21;"] {
        if structural
            .lines()
            .filter(|line| line.trim() == section)
            .count()
            != 1
        {
            return Err(refuse(format!(
                "Part 21 requires exactly one {section} marker"
            )));
        }
    }
    if structural
        .lines()
        .filter(|line| line.trim() == "ENDSEC;")
        .count()
        != 2
    {
        return Err(refuse("Part 21 requires exactly two ENDSEC markers"));
    }
    let iso = structural
        .find("ISO-10303-21;")
        .ok_or_else(|| refuse("Missing ISO-10303-21 prologue"))?;
    let header = structural
        .find("HEADER;")
        .ok_or_else(|| refuse("Missing HEADER section"))?;
    let first_end = structural[header + 7..]
        .find("ENDSEC;")
        .map(|i| i + header + 7)
        .ok_or_else(|| refuse("Unterminated HEADER section"))?;
    let data = structural[first_end + 7..]
        .find("DATA;")
        .map(|i| i + first_end + 7)
        .ok_or_else(|| refuse("Missing DATA section"))?;
    let data_end = structural[data + 5..]
        .find("ENDSEC;")
        .map(|i| i + data + 5)
        .ok_or_else(|| refuse("Unterminated DATA section"))?;
    let end = structural
        .find("END-ISO-10303-21;")
        .ok_or_else(|| refuse("Missing exchange terminator"))?;
    if !(iso < header
        && header < first_end
        && first_end < data
        && data < data_end
        && data_end < end)
    {
        return Err(refuse("Part 21 sections are duplicated or out of order"));
    }
    let header_text = &structural[header..first_end];
    if !header_text.contains("FILE_DESCRIPTION(")
        || !header_text.contains("FILE_NAME(")
        || !header_text.contains("FILE_SCHEMA(")
    {
        return Err(refuse(
            "HEADER requires FILE_DESCRIPTION, FILE_NAME, and FILE_SCHEMA",
        ));
    }
    let schema = header_text.find("FILE_SCHEMA(").unwrap() + header;
    let schema_end = structural[schema..first_end]
        .find(';')
        .map(|offset| schema + offset)
        .unwrap_or(first_end);
    if !upper[schema..schema_end].contains("'AP242_MANAGED_MODEL_BASED_3D_ENGINEERING'")
        && !upper[schema..schema_end].contains("'AP242_MANAGED_MODEL_BASED_3D_ENGINEERING_MIM_LF'")
    {
        return Err(refuse(
            "Direct retained B-rep requires declared AP242 schema",
        ));
    }
    Ok(())
}
fn data_payload(text: &str) -> Result<&str> {
    let (_, structural) = part21_views(text);
    let start = structural
        .find("DATA;")
        .ok_or_else(|| refuse("Missing DATA section"))?
        + 5;
    let end = structural[start..]
        .find("ENDSEC;")
        .map(|offset| start + offset)
        .ok_or_else(|| refuse("Unterminated DATA section"))?;
    Ok(&text[start..end])
}

fn rigid_frame(entities: &BTreeMap<usize, Entity>, id: usize, scale: f64) -> Result<[[f64; 4]; 4]> {
    let (o, x, y) = axis2(entities, id, 3, scale)?;
    let z = [
        x[1] * y[2] - x[2] * y[1],
        x[2] * y[0] - x[0] * y[2],
        x[0] * y[1] - x[1] * y[0],
    ];
    Ok([
        [x[0], y[0], z[0], o[0]],
        [x[1], y[1], z[1], o[1]],
        [x[2], y[2], z[2], o[2]],
        [0., 0., 0., 1.],
    ])
}
fn mul(a: [[f64; 4]; 4], b: [[f64; 4]; 4]) -> [[f64; 4]; 4] {
    std::array::from_fn(|i| std::array::from_fn(|j| (0..4).map(|k| a[i][k] * b[k][j]).sum()))
}
fn inverse_rigid(m: [[f64; 4]; 4]) -> [[f64; 4]; 4] {
    let mut r = [[0.; 4]; 4];
    for i in 0..3 {
        for j in 0..3 {
            r[i][j] = m[j][i]
        }
        r[i][3] = -(0..3).map(|j| r[i][j] * m[j][3]).sum::<f64>();
    }
    r[3][3] = 1.;
    r
}
fn cartesian_operator_3d(
    entities: &BTreeMap<usize, Entity>,
    args: &[Value],
    scale: f64,
) -> Result<[[f64; 4]; 4]> {
    let offset = match args.len() {
        6 => 0,
        8 => 2,
        _ => {
            return Err(refuse(
                "CARTESIAN_TRANSFORMATION_OPERATOR_3D inherited/explicit attribute count mismatch",
            ));
        }
    };
    let factor = if matches!(args[4 + offset], Value::Omitted) {
        1.
    } else {
        number(&args[4 + offset], "transformation scale")?
    };
    if (factor - 1.).abs() > 1e-12 {
        return Err(refuse(
            "Non-rigid CARTESIAN_TRANSFORMATION_OPERATOR_3D scale is refused",
        ));
    }
    let axis = |index: usize, default: Vec<f64>| -> Result<Vec<f64>> {
        if matches!(args[index], Value::Omitted) {
            Ok(default)
        } else {
            direction(entities, one_ref(&args[index], "transformation axis")?, 3)
        }
    };
    let x = axis(1 + offset, vec![1., 0., 0.])?;
    let y = axis(2 + offset, vec![0., 1., 0.])?;
    let z = axis(5 + offset, vec![0., 0., 1.])?;
    let dot = |a: &[f64], b: &[f64]| a.iter().zip(b).map(|(x, y)| x * y).sum::<f64>();
    let cross = [
        x[1] * y[2] - x[2] * y[1],
        x[2] * y[0] - x[0] * y[2],
        x[0] * y[1] - x[1] * y[0],
    ];
    if dot(&x, &y).abs() > 1e-10
        || dot(&x, &z).abs() > 1e-10
        || dot(&y, &z).abs() > 1e-10
        || cross
            .iter()
            .zip(&z)
            .map(|(a, b)| (a - b) * (a - b))
            .sum::<f64>()
            .sqrt()
            > 1e-10
    {
        return Err(refuse(
            "CARTESIAN_TRANSFORMATION_OPERATOR_3D axes are non-rigid or reflected",
        ));
    }
    let o = point(
        entities,
        one_ref(&args[3 + offset], "transformation local origin")?,
        3,
        scale,
    )?;
    Ok([
        [x[0], y[0], z[0], o[0]],
        [x[1], y[1], z[1], o[1]],
        [x[2], y[2], z[2], o[2]],
        [0., 0., 0., 1.],
    ])
}
fn cartesian_operator_affine(
    entities: &BTreeMap<usize, Entity>,
    id: usize,
    scale: f64,
    general: bool,
) -> Result<[[f64; 4]; 4]> {
    let parts = components(entities, id)?;
    let base = component(&parts, "CARTESIAN_TRANSFORMATION_OPERATOR")
        .ok_or_else(|| refuse("Affine complex lacks CARTESIAN_TRANSFORMATION_OPERATOR"))?;
    let spatial = component(&parts, "CARTESIAN_TRANSFORMATION_OPERATOR_3D")
        .ok_or_else(|| refuse("Affine complex lacks CARTESIAN_TRANSFORMATION_OPERATOR_3D"))?;
    let nonuniform = component(&parts, "CARTESIAN_TRANSFORMATION_OPERATOR_3DNON_UNIFORM")
        .ok_or_else(|| {
            refuse("Affine complex lacks CARTESIAN_TRANSFORMATION_OPERATOR_3DNON_UNIFORM")
        })?;
    if !matches!(base.len(), 4 | 5) || spatial.len() != 1 || nonuniform.len() != 2 {
        return Err(refuse(
            "Malformed CARTESIAN_TRANSFORMATION_OPERATOR_3DNON_UNIFORM complex",
        ));
    }
    let axis = |value: &Value, default: Vec<f64>| -> Result<Vec<f64>> {
        if matches!(value, Value::Omitted) {
            Ok(default)
        } else {
            direction(entities, one_ref(value, "affine transformation axis")?, 3)
        }
    };
    let offset = base.len() - 4;
    let x = axis(&base[offset], vec![1., 0., 0.])?;
    let y = axis(&base[offset + 1], vec![0., 1., 0.])?;
    let z = axis(&spatial[0], vec![0., 0., 1.])?;
    let origin = point(
        entities,
        one_ref(&base[offset + 2], "affine transformation origin")?,
        3,
        scale,
    )?;
    if !general {
        let dot = |a: &[f64], b: &[f64]| {
            a.iter()
                .zip(b)
                .map(|(left, right)| left * right)
                .sum::<f64>()
        };
        let cross = [
            x[1] * y[2] - x[2] * y[1],
            x[2] * y[0] - x[0] * y[2],
            x[0] * y[1] - x[1] * y[0],
        ];
        if dot(&x, &y).abs() > 1e-10
            || dot(&x, &z).abs() > 1e-10
            || dot(&y, &z).abs() > 1e-10
            || cross
                .iter()
                .zip(&z)
                .map(|(a, b)| (a - b) * (a - b))
                .sum::<f64>()
                .sqrt()
                > 1e-10
        {
            return Err(refuse(
                "Affine transformation axes are non-orthogonal or reflected",
            ));
        }
    }
    let scales = [
        if matches!(base[offset + 3], Value::Omitted) {
            1.
        } else {
            number(&base[offset + 3], "scale")?
        },
        if matches!(nonuniform[0], Value::Omitted) {
            1.
        } else {
            number(&nonuniform[0], "scale2")?
        },
        if matches!(nonuniform[1], Value::Omitted) {
            1.
        } else {
            number(&nonuniform[1], "scale3")?
        },
    ];
    if scales
        .iter()
        .any(|value| (!general && *value <= 0.) || *value == 0. || !value.is_finite())
    {
        return Err(refuse(if general {
            "Affine scales must be finite and nonzero"
        } else {
            "Affine scales must be finite and positive"
        }));
    }
    let m = [
        [
            x[0] * scales[0],
            y[0] * scales[1],
            z[0] * scales[2],
            origin[0],
        ],
        [
            x[1] * scales[0],
            y[1] * scales[1],
            z[1] * scales[2],
            origin[1],
        ],
        [
            x[2] * scales[0],
            y[2] * scales[1],
            z[2] * scales[2],
            origin[2],
        ],
        [0., 0., 0., 1.],
    ];
    let determinant = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
    if !determinant.is_finite() || determinant.abs() <= 1e-14 {
        return Err(refuse("Affine transformation is singular"));
    }
    Ok(m)
}
fn reachable_transform(
    entities: &BTreeMap<usize, Entity>,
    linked: &BTreeSet<usize>,
    scale: f64,
    strict: bool,
) -> Result<Option<[[f64; 4]; 4]>> {
    let mut transforms = Vec::new();
    for id in linked {
        let (ty, a) = call(entities, *id)?;
        if ty == "CARTESIAN_TRANSFORMATION_OPERATOR_3D" && strict {
            transforms.push(cartesian_operator_3d(entities, a, scale)?);
            continue;
        }
        if ty.starts_with("CARTESIAN_TRANSFORMATION_OPERATOR") {
            return Err(refuse(
                "Reachable generic transform is refused: reflection/nonuniform/singular transforms are outside /3",
            ));
        }
        if ty == "ITEM_DEFINED_TRANSFORMATION" {
            if a.len() != 4 {
                return Err(refuse(
                    "ITEM_DEFINED_TRANSFORMATION argument count mismatch",
                ));
            }
            let from = rigid_frame(entities, one_ref(&a[2], "transformation source")?, scale)?;
            let to = rigid_frame(entities, one_ref(&a[3], "transformation target")?, scale)?;
            transforms.push(mul(to, inverse_rigid(from)));
            if transforms.len() > 32 {
                return Err(refuse("Nested rigid transformation depth exceeds 32"));
            }
        }
    }
    if transforms.is_empty() {
        return Ok(None);
    }
    if strict && transforms.len() != 1 {
        return Err(refuse(
            "Ambiguous STEP placement: selected representation reaches multiple transforms",
        ));
    }
    let mut total = [
        [1., 0., 0., 0.],
        [0., 1., 0., 0.],
        [0., 0., 1., 0.],
        [0., 0., 0., 1.],
    ];
    for t in transforms {
        total = mul(t, total);
    }
    Ok(Some(total))
}

fn exact_boundary_correspondence(
    c3: &Curve,
    pc: &Curve,
    s: &Surface,
    reversed: bool,
) -> Result<bool> {
    if s.control_points.len() < 2 || s.control_points[0].len() < 2 {
        return Ok(false);
    }
    let d2 = pc.domain();
    let a = pc.evaluate(d2[0])?.point;
    let b = pc.evaluate(d2[1])?.point;
    if a.len() != 2 || b.len() != 2 {
        return Ok(false);
    }
    let du = [
        s.knots_u[s.degree_u],
        s.knots_u[s.knots_u.len() - s.degree_u - 1],
    ];
    let dv = [
        s.knots_v[s.degree_v],
        s.knots_v[s.knots_v.len() - s.degree_v - 1],
    ];
    let close = |x: f64, y: f64| (x - y).abs() <= 1e-10;
    let mut expected = if close(a[0], b[0])
        && (close(a[0], du[0]) || close(a[0], du[1]))
        && ((close(a[1], dv[0]) && close(b[1], dv[1]))
            || (close(a[1], dv[1]) && close(b[1], dv[0])))
    {
        let row = if close(a[0], du[0]) {
            0
        } else {
            s.control_points.len() - 1
        };
        s.control_points[row]
            .iter()
            .zip(&s.weights[row])
            .map(|(point, weight)| (point.clone(), *weight))
            .collect::<Vec<_>>()
    } else if close(a[1], b[1])
        && (close(a[1], dv[0]) || close(a[1], dv[1]))
        && ((close(a[0], du[0]) && close(b[0], du[1]))
            || (close(a[0], du[1]) && close(b[0], du[0])))
    {
        let column = if close(a[1], dv[0]) {
            0
        } else {
            s.control_points[0].len() - 1
        };
        s.control_points
            .iter()
            .zip(&s.weights)
            .map(|(row, weights)| (row[column].clone(), weights[column]))
            .collect::<Vec<_>>()
    } else {
        return Ok(false);
    };
    let increasing = if close(a[0], b[0]) {
        b[1] > a[1]
    } else {
        b[0] > a[0]
    };
    if !increasing {
        expected.reverse()
    }
    if reversed {
        expected.reverse()
    }
    let constant = |points: &[(Vec<f64>, f64)]| {
        points.iter().all(|(point, _)| {
            point
                .iter()
                .zip(&points[0].0)
                .all(|(left, right)| close(*left, *right))
        })
    };
    let actual = c3
        .control_points
        .iter()
        .cloned()
        .zip(c3.weights.iter().copied())
        .collect::<Vec<_>>();
    if constant(&expected) && constant(&actual) {
        if !actual[0]
            .0
            .iter()
            .zip(&expected[0].0)
            .all(|(left, right)| close(*left, *right))
        {
            return Err(refuse("Whole-domain pole identity failed"));
        }
        return Ok(true);
    }
    if actual.len() != expected.len() {
        return Ok(false);
    }
    let scale = actual[0].1 / expected[0].1;
    if actual
        .iter()
        .zip(&expected)
        .any(|((point, weight), (wanted, wanted_weight))| {
            !point
                .iter()
                .zip(wanted)
                .all(|(left, right)| close(*left, *right))
                || !close(*weight, scale * wanted_weight)
        })
    {
        return Err(refuse("Whole-domain rational boundary identity failed"));
    }
    Ok(true)
}

fn verify_correspondence(
    c3: &Curve,
    pc: &Curve,
    s: &Surface,
    reversed: bool,
    whole_domain: bool,
    require_exact: bool,
) -> Result<CoedgeTrim> {
    let d3 = c3.domain();
    let d2 = pc.domain();
    let uv_start = pc.evaluate(d2[0])?.point;
    let uv_end = pc.evaluate(d2[1])?.point;
    let domain_u = [
        s.knots_u[s.degree_u],
        s.knots_u[s.knots_u.len() - s.degree_u - 1],
    ];
    let domain_v = [
        s.knots_v[s.degree_v],
        s.knots_v[s.knots_v.len() - s.degree_v - 1],
    ];
    let lift = |uv: &[f64]| -> [i32; 2] {
        let component = |value: f64, domain: [f64; 2], periodic: bool| {
            if periodic {
                ((value - domain[0]) / (domain[1] - domain[0])).floor() as i32
            } else {
                0
            }
        };
        [
            component(uv[0], domain_u, s.periodic_u),
            component(uv[1], domain_v, s.periodic_v),
        ]
    };
    let exact = whole_domain && exact_boundary_correspondence(c3, pc, s, reversed)?;
    if require_exact && !exact {
        return Err(refuse(
            "Collapsed freeform boundary lacks an exact whole-domain rational control identity",
        ));
    }
    for q in 0..=if exact { 0 } else { 8 } {
        let t = q as f64 / 8.;
        let t3 = if reversed {
            d3[1] - t * (d3[1] - d3[0])
        } else {
            d3[0] + t * (d3[1] - d3[0])
        };
        let uv = pc.evaluate(d2[0] + t * (d2[1] - d2[0]))?.point;
        let p = c3.evaluate(t3)?.point;
        let sp = s.evaluate(uv[0], uv[1])?.point;
        let error = (p[0] - sp[0]).hypot(p[1] - sp[1]).hypot(p[2] - sp[2]);
        if !error.is_finite() || error > 1e-5 {
            return Err(refuse(format!(
                "Independent 3D curve↔pcurve correspondence failed ({error:.3e})"
            )));
        }
    }
    let trim = CoedgeTrim {
        curve_parameter: d3,
        pcurve_parameter: d2,
        periodic_lift: [lift(&uv_start), lift(&uv_end)],
    };
    trim.validate()?;
    Ok(trim)
}

struct DirectBuilder<'a> {
    entities: &'a BTreeMap<usize, Entity>,
    scale: f64,
    analytic_surfaces: bool,
    ap242_composition: bool,
    allow_degenerate: bool,
    whole_domain_proofs: bool,
    interior_point_selectors: bool,
    allow_open_shells: bool,
    vertices: Vec<Vertex>,
    vertex_map: BTreeMap<usize, usize>,
    edges: Vec<Edge>,
    edge_map: BTreeMap<usize, usize>,
    seam_use: BTreeMap<(usize, usize), usize>,
    loops: Vec<Loop>,
    loop_entity: Vec<usize>,
    faces: Vec<Face>,
    face_entity: Vec<usize>,
    shells: Vec<Shell>,
    shell_entity: Vec<usize>,
    bodies: Vec<Body>,
    body_entity: Vec<usize>,
}
impl<'a> DirectBuilder<'a> {
    fn vertex(&mut self, id: usize) -> Result<usize> {
        if let Some(v) = self.vertex_map.get(&id) {
            return Ok(*v);
        }
        let (ty, a) = call(self.entities, id)?;
        if ty != "VERTEX_POINT" || a.len() != 2 {
            return Err(refuse("EDGE_CURVE vertex is not VERTEX_POINT"));
        }
        let p = point(
            self.entities,
            one_ref(&a[1], "VERTEX_POINT geometry")?,
            3,
            self.scale,
        )?;
        let index = self.vertices.len();
        self.vertices.push(Vertex {
            point: [p[0], p[1], p[2]],
        });
        self.vertex_map.insert(id, index);
        Ok(index)
    }
    fn edge(
        &mut self,
        id: usize,
        surface_id: usize,
        reversed: bool,
        surface: &Surface,
    ) -> Result<(usize, Curve)> {
        let (ty, a) = call(self.entities, id)?;
        if ty != "EDGE_CURVE" || a.len() != 5 {
            return Err(refuse("ORIENTED_EDGE target is not EDGE_CURVE"));
        }
        let same_sense = boolean(&a[4], "EDGE_CURVE same_sense")?;
        let va = self.vertex(one_ref(&a[1], "EDGE_CURVE start")?)?;
        let vb = self.vertex(one_ref(&a[2], "EDGE_CURVE end")?)?;
        let geometry = one_ref(&a[3], "EDGE_CURVE geometry")?;
        let (gty, ga) = call(self.entities, geometry)?;
        if gty != "SURFACE_CURVE" && gty != "SEAM_CURVE" {
            return Err(refuse("EDGE_CURVE must use SURFACE_CURVE or SEAM_CURVE"));
        }
        if ga.len() < 3 {
            return Err(refuse("SURFACE_CURVE argument count mismatch"));
        }
        let mut c3 = if self.ap242_composition {
            curve_v5(
                self.entities,
                one_ref(&ga[1], "SURFACE_CURVE 3D curve")?,
                3,
                self.scale,
                self.interior_point_selectors,
            )?
        } else {
            curve(
                self.entities,
                one_ref(&ga[1], "SURFACE_CURVE 3D curve")?,
                3,
                self.scale,
            )?
        };
        let mut matching = Vec::new();
        for p in list(&ga[2], "SURFACE_CURVE pcurves")? {
            let (_, pa) = call(self.entities, one_ref(p, "SURFACE_CURVE pcurve")?)?;
            if pa.len() != 3 {
                return Err(refuse("Malformed PCURVE"));
            }
            if one_ref(&pa[1], "PCURVE surface")? == surface_id {
                let mut curve_id = one_ref(&pa[2], "PCURVE reference_to_curve")?;
                if self.ap242_composition {
                    let (reference_type, reference_args) = call(self.entities, curve_id)?;
                    if reference_type != "DEFINITIONAL_REPRESENTATION" || reference_args.len() != 3
                    {
                        return Err(refuse(
                            "AP242 PCURVE reference_to_curve must be a DEFINITIONAL_REPRESENTATION",
                        ));
                    }
                    let items = list(&reference_args[1], "DEFINITIONAL_REPRESENTATION items")?;
                    if items.len() != 1 {
                        return Err(refuse(
                            "PCURVE definitional representation must contain one curve",
                        ));
                    }
                    curve_id = one_ref(&items[0], "DEFINITIONAL_REPRESENTATION curve")?;
                }
                matching.push(if self.ap242_composition {
                    curve_v5(
                        self.entities,
                        curve_id,
                        2,
                        1.,
                        self.interior_point_selectors,
                    )?
                } else {
                    curve(self.entities, curve_id, 2, 1.)?
                });
            }
        }
        if matching.is_empty() {
            return Err(refuse("No PCURVE corresponds to ADVANCED_FACE surface"));
        }
        let mut pc = if gty == "SEAM_CURVE" || matching.len() > 1 {
            let use_index = self.seam_use.entry((id, surface_id)).or_insert(0);
            if *use_index >= matching.len() {
                return Err(refuse("Surface curve has fewer pcurves than oriented uses"));
            }
            let selected = matching[*use_index].clone();
            *use_index += 1;
            selected
        } else {
            matching.pop().unwrap()
        };
        if !same_sense {
            c3 = c3.reverse()?;
            pc = pc.reverse()?;
        }
        let curve_domain = c3.domain();
        let curve_start = c3.evaluate(curve_domain[0])?.point;
        let curve_end = c3.evaluate(curve_domain[1])?.point;
        let endpoint_error = |point: &[f64], vertex: &Vertex| {
            point
                .iter()
                .zip(vertex.point)
                .map(|(left, right)| (left - right) * (left - right))
                .sum::<f64>()
                .sqrt()
        };
        if endpoint_error(&curve_start, &self.vertices[va]) > 1e-7
            || endpoint_error(&curve_end, &self.vertices[vb]) > 1e-7
        {
            return Err(refuse(
                "EDGE_CURVE same_sense does not orient geometry from edge_start to edge_end",
            ));
        }
        // A closed edge may share its endpoint vertex while tracing a full
        // nonconstant curve. Only a constant control hull is a pole edge.
        let pole = self.vertices[va].point;
        let degenerate = va == vb
            && c3
                .control_points
                .iter()
                .all(|point| point.len() == 3 && point.iter().zip(pole).all(|(a, b)| *a == b));
        verify_correspondence(
            &c3,
            &pc,
            surface,
            reversed,
            self.whole_domain_proofs,
            degenerate
                && self.interior_point_selectors
                && certify_periodic_surface(surface)?.is_none(),
        )
        .map_err(|e| {
            refuse(format!(
                "EDGE_CURVE #{id} on surface #{surface_id}: {}",
                e.message
            ))
        })?;
        if degenerate && !self.allow_degenerate {
            return Err(refuse(
                "Pole/degenerate edges are typed-refused before STEP /6",
            ));
        }
        if degenerate {
            let pole = self.vertices[va].point;
            if c3.control_points.iter().any(|point| {
                point.len() != 3 || point.iter().zip(pole).any(|(a, b)| (*a - b).abs() > 1e-9)
            }) {
                return Err(refuse(
                    "Degenerate EDGE_CURVE geometry is not identically its pole vertex",
                ));
            }
        }
        let index = if let Some(index) = self.edge_map.get(&id) {
            let prior = &self.edges[*index];
            if prior.vertices != [va, vb] {
                return Err(refuse("Shared EDGE_CURVE has inconsistent vertices"));
            }
            *index
        } else {
            let index = self.edges.len();
            self.edges.push(Edge {
                degenerate,
                vertices: [va, vb],
                curve: c3,
            });
            self.edge_map.insert(id, index);
            index
        };
        Ok((index, pc))
    }
    fn loop_(&mut self, id: usize, surface_id: usize, surface: &Surface) -> Result<usize> {
        let (ty, a) = call(self.entities, id)?;
        if ty != "EDGE_LOOP" || a.len() != 2 {
            return Err(refuse("FACE_BOUND target is not EDGE_LOOP"));
        }
        let mut coedges = Vec::new();
        for oriented in list(&a[1], "EDGE_LOOP members")? {
            let oid = one_ref(oriented, "EDGE_LOOP member")?;
            let (oty, oa) = call(self.entities, oid)?;
            if oty != "ORIENTED_EDGE" || oa.len() != 5 {
                return Err(refuse("EDGE_LOOP member is not ORIENTED_EDGE"));
            }
            let reversed = !boolean(&oa[4], "ORIENTED_EDGE orientation")?;
            let (edge, pcurve) = self.edge(
                one_ref(&oa[3], "ORIENTED_EDGE edge")?,
                surface_id,
                reversed,
                surface,
            )?;
            coedges.push(Coedge::new(
                edge,
                reversed,
                pcurve,
            ));
        }
        if coedges.is_empty() {
            return Err(refuse("EDGE_LOOP cannot be empty"));
        }
        let index = self.loops.len();
        self.loops.push(Loop { coedges });
        self.loop_entity.push(id);
        Ok(index)
    }
    fn face(&mut self, id: usize) -> Result<usize> {
        let (ty, a) = call(self.entities, id)?;
        if ty != "ADVANCED_FACE" || a.len() != 4 {
            return Err(refuse("CLOSED_SHELL member is not ADVANCED_FACE"));
        }
        let sid = one_ref(&a[2], "ADVANCED_FACE surface")?;
        let mut surface = if self.ap242_composition {
            surface_v5(self.entities, sid, self.scale)?
        } else if self.analytic_surfaces {
            surface_v4(self.entities, sid, self.scale)?
        } else {
            surface(self.entities, sid, self.scale)?
        };
        let mut outer = None;
        let mut holes = Vec::new();
        for bound in list(&a[1], "ADVANCED_FACE bounds")? {
            let bid = one_ref(bound, "ADVANCED_FACE bound")?;
            let (bty, ba) = call(self.entities, bid)?;
            if !matches!(bty, "FACE_OUTER_BOUND" | "FACE_BOUND") || ba.len() != 3 {
                return Err(refuse("ADVANCED_FACE bound type is unsupported"));
            }
            let loop_index = self.loop_(one_ref(&ba[1], "FACE_BOUND loop")?, sid, &surface)?;
            if !boolean(&ba[2], "FACE_BOUND orientation")? {
                for c in &mut self.loops[loop_index].coedges {
                    c.reversed = !c.reversed;
                    c.pcurve = c.pcurve.reverse()?;
                }
                self.loops[loop_index].coedges.reverse();
            }
            if bty == "FACE_OUTER_BOUND" {
                if outer.replace(loop_index).is_some() {
                    return Err(refuse("ADVANCED_FACE has multiple outer bounds"));
                }
            } else {
                holes.push(loop_index);
            }
        }
        let same_sense = boolean(&a[3], "ADVANCED_FACE same_sense")?;
        let authored_mirror = name(a).is_some_and(|name| name.starts_with("OSCAD_TOPO/3|"));
        if !same_sense && (self.ap242_composition || authored_mirror) {
            let sum = surface.knots_u[surface.degree_u]
                + surface.knots_u[surface.knots_u.len() - surface.degree_u - 1];
            surface.control_points.reverse();
            surface.weights.reverse();
            surface.knots_u.reverse();
            for knot in &mut surface.knots_u {
                *knot = sum - *knot
            }
            for loop_index in std::iter::once(outer.unwrap()).chain(holes.iter().copied()) {
                for coedge in &mut self.loops[loop_index].coedges {
                    for point in &mut coedge.pcurve.control_points {
                        point[0] = sum - point[0]
                    }
                    if authored_mirror {
                        coedge.pcurve = coedge.pcurve.reverse()?;
                        coedge.reversed = !coedge.reversed;
                    }
                }
                if authored_mirror {
                    self.loops[loop_index].coedges.reverse();
                }
            }
        }
        let index = self.faces.len();
        self.faces.push(Face {
            surface,
            outer: outer.ok_or_else(|| refuse("ADVANCED_FACE has no outer bound"))?,
            holes,
        });
        self.face_entity.push(id);
        Ok(index)
    }
    fn shell(&mut self, id: usize) -> Result<usize> {
        let (ty, a) = call(self.entities, id)?;
        let closed = match ty {
            "CLOSED_SHELL" => true,
            "OPEN_SHELL" if self.allow_open_shells => false,
            _ => {
                return Err(refuse(
                    "Shell is outside the admitted CLOSED_SHELL/OPEN_SHELL subset",
                ));
            }
        };
        if a.len() != 2 {
            return Err(refuse("Shell argument count mismatch"));
        }
        let mut faces = Vec::new();
        for f in list(&a[1], "shell faces")? {
            let fid = one_ref(f, "shell face")?;
            let (_, fa) = call(self.entities, fid)?;
            faces.push(FaceUse {
                face: self.face(fid)?,
                reversed: !boolean(&fa[3], "ADVANCED_FACE sense")?,
            });
        }
        let index = self.shells.len();
        self.shells.push(Shell { faces, closed });
        self.shell_entity.push(id);
        Ok(index)
    }
    fn body(&mut self, id: usize) -> Result<()> {
        let (ty, a) = call(self.entities, id)?;
        let (outer, inners) = match ty {
            "MANIFOLD_SOLID_BREP" if a.len() == 2 => (one_ref(&a[1], "solid shell")?, Vec::new()),
            "BREP_WITH_VOIDS" if a.len() == 3 => {
                let mut voids = Vec::new();
                for value in list(&a[2], "void shells")? {
                    let void_id = one_ref(value, "void shell")?;
                    if self.ap242_composition {
                        let (vty, va) = call(self.entities, void_id)?;
                        if vty != "ORIENTED_CLOSED_SHELL" || va.len() != 4 {
                            return Err(refuse(
                                "AP242 BREP_WITH_VOIDS requires ORIENTED_CLOSED_SHELL voids",
                            ));
                        }
                        let orientation = boolean(&va[3], "ORIENTED_CLOSED_SHELL orientation")?;
                        if orientation && (!self.allow_degenerate || self.whole_domain_proofs) {
                            return Err(refuse(
                                "Void ORIENTED_CLOSED_SHELL must oppose its CLOSED_SHELL",
                            ));
                        }
                        voids.push(one_ref(&va[2], "ORIENTED_CLOSED_SHELL element")?);
                    } else {
                        voids.push(void_id);
                    }
                }
                (one_ref(&a[1], "outer shell")?, voids)
            }
            _ => {
                return Err(refuse(
                    "Selected representation contains unsupported body root",
                ));
            }
        };
        let outer_shell = self.shell(outer)?;
        let inner_shells = inners
            .into_iter()
            .map(|s| self.shell(s))
            .collect::<Result<Vec<_>>>()?;
        self.bodies.push(Body {
            outer_shell,
            inner_shells,
        });
        self.body_entity.push(id);
        Ok(())
    }
    fn root(&mut self, id: usize) -> Result<()> {
        let (ty, args) = call(self.entities, id)?;
        if ty == "SHELL_BASED_SURFACE_MODEL" && self.allow_open_shells {
            if args.len() != 2 {
                return Err(refuse("SHELL_BASED_SURFACE_MODEL argument count mismatch"));
            }
            for shell in list(&args[1], "SHELL_BASED_SURFACE_MODEL boundaries")? {
                let shell_id = one_ref(shell, "surface-model shell")?;
                if call(self.entities, shell_id)?.0 != "OPEN_SHELL" {
                    return Err(refuse(
                        "SHELL_BASED_SURFACE_MODEL requires OPEN_SHELL boundaries",
                    ));
                }
                self.shell(shell_id)?;
            }
            Ok(())
        } else {
            self.body(id)
        }
    }
}

fn metadata_entity_map(builder: &DirectBuilder<'_>) -> Vec<(TopoKind, usize, usize)> {
    let mut out = Vec::new();
    out.extend(
        builder
            .vertex_map
            .iter()
            .map(|(entity, index)| (TopoKind::Vertex, *index, *entity)),
    );
    out.extend(
        builder
            .edge_map
            .iter()
            .map(|(entity, index)| (TopoKind::Edge, *index, *entity)),
    );
    out.extend(
        builder
            .loop_entity
            .iter()
            .enumerate()
            .map(|(i, e)| (TopoKind::Loop, i, *e)),
    );
    out.extend(
        builder
            .face_entity
            .iter()
            .enumerate()
            .map(|(i, e)| (TopoKind::Face, i, *e)),
    );
    out.extend(
        builder
            .shell_entity
            .iter()
            .enumerate()
            .map(|(i, e)| (TopoKind::Shell, i, *e)),
    );
    out.extend(
        builder
            .body_entity
            .iter()
            .enumerate()
            .map(|(i, e)| (TopoKind::Body, i, *e)),
    );
    out
}

fn build_direct_roots(
    entities: &BTreeMap<usize, Entity>,
    roots: &[usize],
    scale: f64,
    options: DirectBuildOptions,
) -> Result<(Model, TopologyMapping)> {
    let DirectBuildOptions {
        analytic_surfaces,
        ap242_composition,
        allow_degenerate,
        whole_domain_proofs,
        interior_point_selectors,
        allow_open_shells,
    } = options;
    let mut builder = DirectBuilder {
        entities,
        scale,
        analytic_surfaces,
        ap242_composition,
        allow_degenerate,
        whole_domain_proofs,
        interior_point_selectors,
        allow_open_shells,
        vertices: vec![],
        vertex_map: BTreeMap::new(),
        edges: vec![],
        edge_map: BTreeMap::new(),
        seam_use: BTreeMap::new(),
        loops: vec![],
        loop_entity: vec![],
        faces: vec![],
        face_entity: vec![],
        shells: vec![],
        shell_entity: vec![],
        bodies: vec![],
        body_entity: vec![],
    };
    for root in roots {
        builder.root(*root)?
    }
    // Faces discover vertices and edges in traversal order, which need not
    // match their STEP entity order. Restore those stable orders and remap
    // topology references; unused entities remain excluded from the model.
    let vertex_order = builder
        .vertex_map
        .iter()
        .map(|(&entity, &old)| (entity, old))
        .collect::<Vec<_>>();
    let old_vertices = std::mem::take(&mut builder.vertices);
    let mut old_to_new = vec![0; old_vertices.len()];
    builder.vertex_map.clear();
    for (new, (entity, old)) in vertex_order.into_iter().enumerate() {
        old_to_new[old] = new;
        builder.vertices.push(old_vertices[old].clone());
        builder.vertex_map.insert(entity, new);
    }
    for edge in &mut builder.edges {
        for vertex in &mut edge.vertices {
            *vertex = old_to_new[*vertex];
        }
    }
    let edge_order = builder
        .edge_map
        .iter()
        .map(|(&entity, &old)| (entity, old))
        .collect::<Vec<_>>();
    let old_edges = std::mem::take(&mut builder.edges);
    let mut old_to_new = vec![0; old_edges.len()];
    builder.edge_map.clear();
    for (new, (entity, old)) in edge_order.into_iter().enumerate() {
        old_to_new[old] = new;
        builder.edges.push(old_edges[old].clone());
        builder.edge_map.insert(entity, new);
    }
    for loop_ in &mut builder.loops {
        for coedge in &mut loop_.coedges {
            coedge.edge = old_to_new[coedge.edge];
        }
    }
    let map = metadata_entity_map(&builder);
    let topology = brep_topology::Model {
        vertices: builder.vertices,
        edges: builder.edges,
        loops: builder.loops,
        faces: builder.faces,
        shells: builder.shells,
        bodies: builder.bodies,
        tolerance_mm: 1e-7,
    };
    let mut model = Model(topology, TopologyIds::default());
    model.rebuild_topology_ids();
    model.validate()?;
    Ok((model, map))
}

fn append_direct_model(
    target: &mut Model,
    source: Model,
    source_map: &[(TopoKind, usize, usize)],
    target_map: &mut Vec<(TopoKind, usize, usize)>,
) {
    let Model(mut topology, _) = source;
    let (vo, eo, lo, fo, so, bo) = (
        target.vertices.len(),
        target.edges.len(),
        target.loops.len(),
        target.faces.len(),
        target.shells.len(),
        target.bodies.len(),
    );
    for edge in &mut topology.edges {
        edge.vertices = [edge.vertices[0] + vo, edge.vertices[1] + vo]
    }
    for loop_ in &mut topology.loops {
        for coedge in &mut loop_.coedges {
            coedge.edge += eo
        }
    }
    for face in &mut topology.faces {
        face.outer += lo;
        for hole in &mut face.holes {
            *hole += lo
        }
    }
    for shell in &mut topology.shells {
        for face in &mut shell.faces {
            face.face += fo
        }
    }
    for body in &mut topology.bodies {
        body.outer_shell += so;
        for shell in &mut body.inner_shells {
            *shell += so
        }
    }
    target.0.vertices.extend(topology.vertices);
    target.0.edges.extend(topology.edges);
    target.0.loops.extend(topology.loops);
    target.0.faces.extend(topology.faces);
    target.0.shells.extend(topology.shells);
    target.0.bodies.extend(topology.bodies);
    for &(kind, index, entity) in source_map {
        let offset = match kind {
            TopoKind::Vertex => vo,
            TopoKind::Edge => eo,
            TopoKind::Loop => lo,
            TopoKind::Face => fo,
            TopoKind::Shell => so,
            TopoKind::Body => bo,
            TopoKind::ControlPoint => 0,
        };
        target_map.push((kind, index + offset, entity));
    }
}

/// Compose already validated document occurrences into one authoritative
/// bounded model and assign occurrence-local topology identities.
fn compose_step_occurrences(models: &[Model], capability: &str) -> Result<Model> {
    if models.is_empty() {
        return Err(refuse(
            "STEP /7 composition requires at least one occurrence",
        ));
    }
    if models.len() > MAX_OCCURRENCES {
        return Err(refuse("STEP /7 composed occurrence count exceeds 256"));
    }
    let mut aggregate = Model::empty(1e-7)?;
    let mut ignored = Vec::new();
    for model in models {
        model.validate()?;
        append_direct_model(&mut aggregate, model.clone(), &[], &mut ignored);
    }
    aggregate.rebuild_topology_ids();
    let occurrence_ids = |kind: TopoKind, ids: &[TopoId]| {
        ids.iter()
            .enumerate()
            .map(|(index, id)| {
                TopoId::derive(
                    kind,
                    capability,
                    &format!("occurrence-entity-{index}"),
                    kind.as_str(),
                    id.to_string().as_bytes(),
                )
            })
            .collect::<Vec<_>>()
    };
    aggregate.1.vertices = occurrence_ids(TopoKind::Vertex, &aggregate.1.vertices);
    aggregate.1.edges = occurrence_ids(TopoKind::Edge, &aggregate.1.edges);
    aggregate.1.loops = occurrence_ids(TopoKind::Loop, &aggregate.1.loops);
    aggregate.1.faces = occurrence_ids(TopoKind::Face, &aggregate.1.faces);
    aggregate.1.shells = occurrence_ids(TopoKind::Shell, &aggregate.1.shells);
    aggregate.1.bodies = occurrence_ids(TopoKind::Body, &aggregate.1.bodies);
    aggregate.1.lineage.clear();
    aggregate.1.change_set = brep_topology::ChangeSet::default();
    for (kind, ids) in [
        (TopoKind::Vertex, &aggregate.1.vertices),
        (TopoKind::Edge, &aggregate.1.edges),
        (TopoKind::Loop, &aggregate.1.loops),
        (TopoKind::Face, &aggregate.1.faces),
        (TopoKind::Shell, &aggregate.1.shells),
        (TopoKind::Body, &aggregate.1.bodies),
    ] {
        for id in ids {
            aggregate.1.change_set.nodes.insert(*id, kind);
        }
    }
    aggregate.validate()?;
    Ok(aggregate)
}
pub fn compose_step_v7_occurrences(models: &[Model]) -> Result<Model> {
    compose_step_occurrences(models, STEP_INTERCHANGE_V7_CAPABILITY)
}
pub fn compose_step_v8_occurrences(models: &[Model]) -> Result<Model> {
    let aggregate = compose_step_occurrences(models, STEP_INTERCHANGE_V8_CAPABILITY)?;
    certify_step_v8_topology(&aggregate)?;
    Ok(aggregate)
}
pub fn compose_step_v9_occurrences(models: &[Model]) -> Result<Model> {
    let aggregate = compose_step_occurrences(models, STEP_INTERCHANGE_V9_CAPABILITY)?;
    certify_step_v8_topology(&aggregate)?;
    Ok(aggregate)
}

#[derive(Clone)]
struct RepresentationOccurrence {
    representation: usize,
    bodies: Vec<usize>,
    transform: [[f64; 4]; 4],
    linked: BTreeSet<usize>,
    path: Vec<usize>,
}

fn relationship_components(
    entities: &BTreeMap<usize, Entity>,
    id: usize,
) -> Result<Option<(usize, usize, usize)>> {
    let parts = components(entities, id)?;
    let Some(base) = component(&parts, "REPRESENTATION_RELATIONSHIP") else {
        return Ok(None);
    };
    let Some(with_transform) = component(&parts, "REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION")
    else {
        return Ok(None);
    };
    if base.len() != 4 || with_transform.len() != 1 {
        return Err(refuse(
            "Malformed REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION complex",
        ));
    }
    Ok(Some((
        one_ref(&base[2], "relationship source representation")?,
        one_ref(&base[3], "relationship target representation")?,
        one_ref(&with_transform[0], "relationship transformation operator")?,
    )))
}

fn representation_occurrences(
    entities: &BTreeMap<usize, Entity>,
    selected: &[usize],
    allow_affine: bool,
    general_affine: bool,
    allow_open_shells: bool,
) -> Result<Vec<RepresentationOccurrence>> {
    // CDSR identifies product occurrences. Its relationship runs child -> parent;
    // traverse parent -> child while retaining the child-to-parent placement.
    let mut products = BTreeMap::new();
    let mut assembly = BTreeMap::new();
    if general_affine {
        for entity in entities.values() {
            if let Value::Call(name, args) = &entity.value
                && name == "SHAPE_DEFINITION_REPRESENTATION"
                && args.len() == 2
            {
                let (kind, shape) = call(entities, one_ref(&args[0], "product shape")?)?;
                if kind == "PRODUCT_DEFINITION_SHAPE" && shape.len() == 3 {
                    products.insert(
                        one_ref(&args[1], "product representation")?,
                        one_ref(&shape[2], "product definition")?,
                    );
                }
            }
        }
        for entity in entities.values() {
            if let Value::Call(name, args) = &entity.value
                && name == "CONTEXT_DEPENDENT_SHAPE_REPRESENTATION"
            {
                if args.len() != 2 {
                    return Err(refuse("Malformed product occurrence"));
                }
                let relation = one_ref(&args[0], "occurrence relationship")?;
                let (child, parent, _) = relationship_components(entities, relation)?
                    .ok_or_else(|| refuse("Occurrence requires a placement relationship"))?;
                let (kind, shape) = call(entities, one_ref(&args[1], "occurrence shape")?)?;
                if kind != "PRODUCT_DEFINITION_SHAPE" || shape.len() != 3 {
                    return Err(refuse("Invalid occurrence product shape"));
                }
                let (kind, usage) = call(entities, one_ref(&shape[2], "occurrence usage")?)?;
                if kind != "NEXT_ASSEMBLY_USAGE_OCCURRENCE"
                    || usage.len() != 6
                    || products.get(&parent) != Some(&one_ref(&usage[3], "parent product")?)
                    || products.get(&child) != Some(&one_ref(&usage[4], "child product")?)
                {
                    return Err(refuse(
                        "Occurrence product endpoints disagree with placement",
                    ));
                }
                if assembly.insert(relation, (parent, child)).is_some() {
                    return Err(refuse("Duplicate product occurrence relationship"));
                }
            }
        }
    }
    let mut outgoing: BTreeMap<usize, Vec<(usize, usize, usize, bool)>> = BTreeMap::new();
    let mut component_representations = BTreeSet::new();
    for id in entities.keys() {
        if let Some((from, to, operator)) = relationship_components(entities, *id)? {
            if let Some(&(parent, child)) = assembly.get(id) {
                outgoing
                    .entry(parent)
                    .or_default()
                    .push((*id, child, operator, true));
                component_representations.insert(child);
            } else {
                outgoing
                    .entry(from)
                    .or_default()
                    .push((*id, to, operator, false));
            }
        }
    }
    for edges in outgoing.values_mut() {
        edges.sort_by_key(|edge| edge.0)
    }
    #[expect(
        clippy::too_many_arguments,
        reason = "recursive STEP graph walk keeps traversal state explicit"
    )]
    fn walk(
        rep: usize,
        matrix: [[f64; 4]; 4],
        entities: &BTreeMap<usize, Entity>,
        outgoing: &BTreeMap<usize, Vec<(usize, usize, usize, bool)>>,
        active: &mut BTreeSet<usize>,
        depth: usize,
        allow_affine: bool,
        general_affine: bool,
        allow_open_shells: bool,
        path: &[usize],
        out: &mut Vec<RepresentationOccurrence>,
    ) -> Result<()> {
        if depth > 32 {
            return Err(refuse(
                "Representation relationship placement depth exceeds 32",
            ));
        }
        if !active.insert(rep) {
            return Err(refuse(
                "Cycle in selected representation-relationship placement graph",
            ));
        }
        let (ty, args) = call(entities, rep)?;
        if ty != "ADVANCED_BREP_SHAPE_REPRESENTATION"
            && !(general_affine && ty == "SHAPE_REPRESENTATION")
            && !(allow_open_shells && ty == "MANIFOLD_SURFACE_SHAPE_REPRESENTATION")
        {
            return Err(refuse(
                "Placement graph reaches an unsupported representation",
            ));
        }
        if args.len() != 3 {
            return Err(refuse("Shape representation argument count mismatch"));
        }
        if ty == "SHAPE_REPRESENTATION" {
            if !outgoing.contains_key(&rep) {
                return Err(refuse("Empty product assembly container"));
            }
            for item in list(&args[1], "assembly placement items")? {
                if call(entities, one_ref(item, "assembly item")?)?.0 != "AXIS2_PLACEMENT_3D" {
                    return Err(refuse("Unsupported geometry in product assembly container"));
                }
            }
        }
        let linked = reachable(entities, &[rep])?;
        let scale = length_scale(entities, &linked, false)?;
        let bodies = list(&args[1], "representation items")?
            .iter()
            .filter_map(|value| match value {
                Value::Ref(id) => call(entities, *id).ok().and_then(|(name, _)| {
                    (matches!(name, "MANIFOLD_SOLID_BREP" | "BREP_WITH_VOIDS")
                        || allow_open_shells && name == "SHELL_BASED_SURFACE_MODEL")
                        .then_some(*id)
                }),
                _ => None,
            })
            .collect::<Vec<_>>();
        if !bodies.is_empty() {
            if out.len() >= MAX_OCCURRENCES {
                return Err(refuse("Assembly occurrence count exceeds 256"));
            }
            out.push(RepresentationOccurrence {
                representation: rep,
                bodies,
                transform: matrix,
                linked,
                path: path.to_vec(),
            })
        }
        if let Some(edges) = outgoing.get(&rep) {
            for (relationship, child, operator, product_occurrence) in edges {
                let affine_complex = allow_affine
                    && component(
                        &components(entities, *operator)?,
                        "CARTESIAN_TRANSFORMATION_OPERATOR_3DNON_UNIFORM",
                    )
                    .is_some();
                let edge_matrix = if affine_complex {
                    cartesian_operator_affine(entities, *operator, scale, general_affine)?
                } else {
                    let (operator_type, operator_args) = call(entities, *operator)?;
                    match operator_type {
                        "CARTESIAN_TRANSFORMATION_OPERATOR_3D" => {
                            cartesian_operator_3d(entities, operator_args, scale)?
                        }
                        "ITEM_DEFINED_TRANSFORMATION" => {
                            if operator_args.len() != 4 {
                                return Err(refuse(
                                    "ITEM_DEFINED_TRANSFORMATION argument count mismatch",
                                ));
                            }
                            let source_scale = if *product_occurrence {
                                length_scale(entities, &reachable(entities, &[*child])?, false)?
                            } else {
                                scale
                            };
                            let from = rigid_frame(
                                entities,
                                one_ref(&operator_args[2], "transformation source")?,
                                source_scale,
                            )?;
                            let to = rigid_frame(
                                entities,
                                one_ref(&operator_args[3], "transformation target")?,
                                scale,
                            )?;
                            mul(to, inverse_rigid(from))
                        }
                        _ => {
                            return Err(refuse(
                                "Representation relationship uses a non-rigid or unsupported transformation operator",
                            ));
                        }
                    }
                };
                let mut child_path = path.to_vec();
                child_path.extend([*relationship, *child]);
                walk(
                    *child,
                    mul(matrix, edge_matrix),
                    entities,
                    outgoing,
                    active,
                    depth + 1,
                    allow_affine,
                    general_affine,
                    allow_open_shells,
                    &child_path,
                    out,
                )?;
            }
        }
        active.remove(&rep);
        Ok(())
    }
    let identity = [
        [1., 0., 0., 0.],
        [0., 1., 0., 0.],
        [0., 0., 1., 0.],
        [0., 0., 0., 1.],
    ];
    let roots: Vec<_> = selected
        .iter()
        .filter(|rep| !component_representations.contains(rep))
        .copied()
        .collect();
    let mut covered = BTreeSet::new();
    let mut pending = roots.clone();
    while let Some(rep) = pending.pop() {
        if covered.insert(rep) {
            if let Some(edges) = outgoing.get(&rep) {
                pending.extend(edges.iter().map(|edge| edge.1));
            }
        }
    }
    if selected.iter().any(|rep| !covered.contains(rep)) {
        return Err(refuse("Cyclic or unreachable product assembly"));
    }
    let mut out = Vec::new();
    for rep in &roots {
        walk(
            *rep,
            identity,
            entities,
            &outgoing,
            &mut BTreeSet::new(),
            0,
            allow_affine,
            general_affine,
            allow_open_shells,
            &[*rep],
            &mut out,
        )?
    }
    let mut paths = BTreeSet::new();
    for occurrence in &out {
        let key = (
            occurrence.representation,
            occurrence.bodies.clone(),
            occurrence
                .transform
                .iter()
                .flatten()
                .map(|v| v.to_bits())
                .collect::<Vec<_>>(),
        );
        if !paths.insert(key) {
            return Err(refuse(
                "Ambiguous duplicate selected representation occurrence",
            ));
        }
    }
    Ok(out)
}

fn digest_value(
    value: &Value,
    entities: &BTreeMap<usize, Entity>,
    active: &mut BTreeSet<usize>,
    depth: usize,
) -> Result<String> {
    if depth > MAX_GRAPH_DEPTH {
        return Err(refuse("Canonical graph digest depth exceeds 64"));
    }
    let text = match value {
        Value::String(_) => "''".into(), // entity names are non-geometric metadata
        Value::Number(n) => format!("{n:.17e}"),
        Value::Integer(n) => n.to_string(),
        Value::Enum(v) => format!(".{v}."),
        Value::Omitted => "$".into(),
        Value::Derived => "*".into(),
        Value::List(v) => format!(
            "({})",
            v.iter()
                .map(|x| digest_value(x, entities, active, depth + 1))
                .collect::<Result<Vec<_>>>()?
                .join(",")
        ),
        Value::Call(n, v) => {
            let args = v
                .iter()
                .enumerate()
                .map(|(i, x)| {
                    if i == 0 && matches!(x, Value::String(_)) {
                        Ok("''".into())
                    } else {
                        digest_value(x, entities, active, depth + 1)
                    }
                })
                .collect::<Result<Vec<_>>>()?;
            format!("{n}({})", args.join(","))
        }
        Value::Ref(id) => {
            if !active.insert(*id) {
                return Err(refuse("Cycle while computing canonical graph digest"));
            }
            let e = entities
                .get(id)
                .ok_or_else(|| refuse("Missing digest dependency"))?;
            let result = digest_value(&e.value, entities, active, depth + 1)?;
            active.remove(id);
            format!("#{result}")
        }
    };
    Ok(text)
}
fn hash(text: &str) -> String {
    Sha256::digest(text.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
pub(crate) fn entity_digest(entities: &BTreeMap<usize, Entity>, id: usize) -> Result<String> {
    let entity = entities
        .get(&id)
        .ok_or_else(|| refuse("Missing identity entity"))?;
    Ok(hash(&digest_value(
        &entity.value,
        entities,
        &mut BTreeSet::from([id]),
        0,
    )?))
}

fn restore_identity(
    entities: &BTreeMap<usize, Entity>,
    map: &[(TopoKind, usize, usize)],
    model: &mut Model,
) -> Result<crate::step_interchange::StepIdentityReport> {
    let mut metadata = Vec::new();
    for &(kind, index, entity_id) in map {
        let (_, args) = call(entities, entity_id)?;
        let Some(n) = name(args) else {
            continue;
        };
        let Some(payload) = n.strip_prefix("OSCAD_TOPO/3|") else {
            continue;
        };
        let fields: Vec<_> = payload.split('|').collect();
        if fields.len() != 3 {
            return Err(refuse("Malformed STEP /3 identity metadata"));
        }
        let mk =
            TopoKind::parse(fields[0]).ok_or_else(|| refuse("Unknown STEP /3 identity kind"))?;
        let id = TopoId::parse(fields[1]).map_err(refuse)?;
        if mk != kind || id.kind() != kind || fields[2] != entity_digest(entities, entity_id)? {
            return Err(refuse("Transplanted STEP /3 identity metadata"));
        }
        metadata.push((kind, index, id));
    }
    let count = map.len();
    if metadata.is_empty() {
        model.rebuild_topology_ids();
        return Ok(crate::step_interchange::StepIdentityReport {
            preserved: false,
            source: "external-step",
            preserved_count: 0,
            created_count: count,
            lost_count: count,
        });
    }
    if metadata.len() != count {
        return Err(refuse("Partial STEP /3 identity metadata"));
    }
    let mut seen = BTreeSet::new();
    let mut remap = BTreeMap::new();
    for (kind, index, id) in metadata {
        if !seen.insert(id) {
            return Err(refuse("Duplicate STEP /3 identity metadata"));
        }
        let old = match kind {
            TopoKind::Vertex => model.1.vertices[index],
            TopoKind::Edge => model.1.edges[index],
            TopoKind::Loop => model.1.loops[index],
            TopoKind::Face => model.1.faces[index],
            TopoKind::Shell => model.1.shells[index],
            TopoKind::Body => model.1.bodies[index],
            TopoKind::ControlPoint => {
                return Err(refuse(
                    "Control-point identity is not STEP topology identity",
                ));
            }
        };
        remap.insert(old, id);
        match kind {
            TopoKind::Vertex => model.1.vertices[index] = id,
            TopoKind::Edge => model.1.edges[index] = id,
            TopoKind::Loop => model.1.loops[index] = id,
            TopoKind::Face => model.1.faces[index] = id,
            TopoKind::Shell => model.1.shells[index] = id,
            TopoKind::Body => model.1.bodies[index] = id,
            TopoKind::ControlPoint => {
                return Err(refuse(
                    "Control-point identity is not STEP topology identity",
                ));
            }
        }
    }
    model.1.change_set.nodes = model
        .1
        .change_set
        .nodes
        .iter()
        .map(|(id, kind)| (remap.get(id).copied().unwrap_or(*id), *kind))
        .collect();
    for change in &mut model.1.change_set.changes {
        for id in change.parents.iter_mut().chain(&mut change.children) {
            if let Some(replacement) = remap.get(id) {
                *id = *replacement;
            }
        }
    }
    Ok(crate::step_interchange::StepIdentityReport {
        preserved: true,
        source: "internal-metadata",
        preserved_count: count,
        created_count: 0,
        lost_count: 0,
    })
}

const UNREACHABLE_ALLOWLIST: &[&str] = &[
    "APPLICATION_CONTEXT",
    "PRODUCT",
    "PRODUCT_DEFINITION",
    "PRODUCT_DEFINITION_FORMATION",
    "PRODUCT_DEFINITION_FORMATION_WITH_SPECIFIED_SOURCE",
    "PRODUCT_RELATED_PRODUCT_CATEGORY",
    "PRODUCT_CONTEXT",
    "PRODUCT_DEFINITION_CONTEXT",
    "PRODUCT_DEFINITION_SHAPE",
    "SHAPE_DEFINITION_REPRESENTATION",
    "ADVANCED_BREP_SHAPE_REPRESENTATION",
    "MANIFOLD_SURFACE_SHAPE_REPRESENTATION",
    "SHELL_BASED_SURFACE_MODEL",
    "GEOMETRIC_REPRESENTATION_CONTEXT",
    "GLOBAL_UNIT_ASSIGNED_CONTEXT",
    "SI_UNIT",
    "PRESENTATION_LAYER_ASSIGNMENT",
    "STYLED_ITEM",
    "COLOUR_RGB",
    "SURFACE_STYLE_USAGE",
    "SURFACE_SIDE_STYLE",
    "SURFACE_STYLE_FILL_AREA",
    "FILL_AREA_STYLE",
    "FILL_AREA_STYLE_COLOUR",
    "DRAUGHTING_PRE_DEFINED_COLOUR",
    "PROPERTY_DEFINITION",
    "PROPERTY_DEFINITION_REPRESENTATION",
    "DESCRIPTIVE_REPRESENTATION_ITEM",
    "MEASURE_REPRESENTATION_ITEM",
    "MATERIAL_DESIGNATION",
    "APPLIED_NAME_ASSIGNMENT",
    "APPLIED_DESCRIPTION_TEXT_ASSIGNMENT",
    "PERSON",
    "ORGANIZATION",
    "PERSON_AND_ORGANIZATION",
    "PERSON_AND_ORGANIZATION_ROLE",
    "APPLIED_PERSON_AND_ORGANIZATION_ASSIGNMENT",
    "DATE_AND_TIME",
    "CALENDAR_DATE",
    "LOCAL_TIME",
    "COORDINATED_UNIVERSAL_TIME_OFFSET",
    "DATE_TIME_ROLE",
    "APPLIED_DATE_AND_TIME_ASSIGNMENT",
    "APPROVAL",
    "APPROVAL_STATUS",
    "APPROVAL_ROLE",
    "APPLIED_APPROVAL_ASSIGNMENT",
    "SECURITY_CLASSIFICATION",
    "SECURITY_CLASSIFICATION_LEVEL",
    "APPLIED_SECURITY_CLASSIFICATION_ASSIGNMENT",
    "EXTERNAL_CLASS_LIBRARY",
    "ITEM_DEFINED_TRANSFORMATION",
    "AXIS2_PLACEMENT_3D",
    "CARTESIAN_POINT",
    "DIRECTION",
    "REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION",
    "CARTESIAN_TRANSFORMATION_OPERATOR_3D",
];

fn import_step_direct(
    text: &str,
    analytic_surfaces: bool,
    ap242_composition: bool,
    allow_degenerate: bool,
    capability: &'static str,
) -> Result<(Model, FeatureCertificate, StepV3Report)> {
    if !text.to_ascii_uppercase().contains("ISO-10303-21") {
        return Err(refuse("Not an ISO-10303-21 exchange"));
    }
    if ap242_composition {
        validate_exchange_structure(text)?;
    }
    let entities = parse(if ap242_composition {
        data_payload(text)?
    } else {
        text
    })?;
    let representations: Vec<_> = if ap242_composition {
        let mut selected = Vec::<usize>::new();
        for entity in entities.values() {
            if let Value::Call(name, args) = &entity.value
                && name == "SHAPE_DEFINITION_REPRESENTATION"
                && args.len() == 2
            {
                selected.push(one_ref(&args[1], "selected shape representation")?);
            }
        }
        selected.sort_unstable();
        selected.dedup();
        if selected.is_empty() {
            return Err(refuse(
                "AP242 import requires SHAPE_DEFINITION_REPRESENTATION selection",
            ));
        }
        let allow_open_shells = capability == STEP_INTERCHANGE_V9_CAPABILITY;
        for id in &selected {
            let (ty, _) = call(&entities, *id)?;
            if ty != "ADVANCED_BREP_SHAPE_REPRESENTATION"
                && !(allow_open_shells && ty == "SHAPE_REPRESENTATION")
                && !(allow_open_shells && ty == "MANIFOLD_SURFACE_SHAPE_REPRESENTATION")
            {
                return Err(refuse(
                    "Selected representation is outside the admitted direct B-rep products",
                ));
            }
        }
        selected
    } else {
        entities
            .keys()
            .filter_map(|id| {
                call(&entities, *id)
                    .ok()
                    .and_then(|(ty, _)| (ty == "ADVANCED_BREP_SHAPE_REPRESENTATION").then_some(*id))
            })
            .collect()
    };
    let allow_open_shells = capability == STEP_INTERCHANGE_V9_CAPABILITY;
    let all_bodies: Vec<_> = entities
        .keys()
        .filter_map(|id| {
            call(&entities, *id).ok().and_then(|(ty, _)| {
                (matches!(ty, "MANIFOLD_SOLID_BREP" | "BREP_WITH_VOIDS")
                    || allow_open_shells && ty == "SHELL_BASED_SURFACE_MODEL")
                    .then_some(*id)
            })
        })
        .collect();
    let graph_roots = if representations.is_empty() {
        all_bodies.clone()
    } else {
        representations.clone()
    };
    if graph_roots.is_empty() {
        return Err(refuse("STEP /3 requires a manifold advanced B-rep root"));
    }
    let occurrences = if ap242_composition {
        representation_occurrences(
            &entities,
            &representations,
            allow_degenerate && capability != STEP_INTERCHANGE_V8_CAPABILITY,
            capability == STEP_INTERCHANGE_V9_CAPABILITY,
            allow_open_shells,
        )?
    } else {
        Vec::new()
    };
    let occurrence_count = occurrences.len();
    let mut definition_identities = Vec::new();
    let mut occurrence_identities = Vec::new();
    if allow_degenerate {
        for occurrence in &occurrences {
            let definition = format!(
                "representation:{}",
                entity_digest(&entities, occurrence.representation)?
            );
            if !definition_identities.contains(&definition) {
                definition_identities.push(definition)
            }
            let path = occurrence
                .path
                .iter()
                .map(|id| entity_digest(&entities, *id))
                .collect::<Result<Vec<_>>>()?
                .join("/");
            let matrix = occurrence
                .transform
                .iter()
                .flatten()
                .map(|value| format!("{:016x}", value.to_bits()))
                .collect::<String>();
            occurrence_identities.push(format!("occurrence:{}", hash(&(path + &matrix))));
        }
    }
    let product_hierarchy = if allow_degenerate {
        entities
            .keys()
            .filter_map(|id| {
                call(&entities, *id).ok().and_then(|(ty, args)| {
                    if ty == "PRODUCT" && args.len() >= 3 {
                        Some(format!(
                            "product:#{}:{}:{}",
                            id,
                            string_value(&args[0], "product id").ok()?,
                            string_value(&args[1], "product name").ok()?
                        ))
                    } else if matches!(
                        ty,
                        "PRODUCT_DEFINITION"
                            | "PRODUCT_DEFINITION_FORMATION"
                            | "PRODUCT_DEFINITION_FORMATION_WITH_SPECIFIED_SOURCE"
                    ) && args.len() >= 2
                    {
                        Some(format!(
                            "{}:#{}:{}:{}",
                            ty.to_ascii_lowercase(),
                            id,
                            string_value(&args[0], "product metadata id").ok()?,
                            string_value(&args[1], "product metadata description").ok()?
                        ))
                    } else if ty == "PRODUCT_RELATED_PRODUCT_CATEGORY" && args.len() >= 3 {
                        Some(format!(
                            "product-category:#{}:{}",
                            id,
                            string_value(&args[0], "product category").ok()?
                        ))
                    } else if ty == "PROPERTY_DEFINITION" && args.len() >= 2 {
                        Some(format!(
                            "property-definition:#{}:{}",
                            id,
                            string_value(&args[0], "property name").ok()?
                        ))
                    } else if ty == "NEXT_ASSEMBLY_USAGE_OCCURRENCE" && args.len() >= 5 {
                        Some(format!(
                            "assembly-use:#{}:#{}:#{}",
                            id,
                            one_ref(&args[3], "relating product").ok()?,
                            one_ref(&args[4], "related product").ok()?
                        ))
                    } else {
                        None
                    }
                })
            })
            .collect()
    } else {
        Vec::new()
    };
    let external_references = if allow_degenerate {
        entities
            .keys()
            .filter_map(|id| {
                let (ty, args) = call(&entities, *id).ok()?;
                if ty == "DOCUMENT_FILE" && !args.is_empty() {
                    Some(format!(
                        "#{}:{}",
                        id,
                        string_value(&args[0], "document URI").ok()?
                    ))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let mut linked = reachable(&entities, &graph_roots)?;
    if ap242_composition {
        if capability != STEP_INTERCHANGE_V9_CAPABILITY {
            linked.clear();
        }
        for occurrence in &occurrences {
            linked.extend(occurrence.linked.iter().copied())
        }
    }
    let roots: Vec<_> = all_bodies
        .into_iter()
        .filter(|id| linked.contains(id))
        .collect();
    if roots.is_empty() {
        return Err(refuse(
            "Selected representation contains no manifold B-rep body",
        ));
    }
    const REACHABLE_TYPES: &[&str] = &[
        "SHAPE_REPRESENTATION",
        "ADVANCED_BREP_SHAPE_REPRESENTATION",
        "MANIFOLD_SURFACE_SHAPE_REPRESENTATION",
        "MANIFOLD_SOLID_BREP",
        "BREP_WITH_VOIDS",
        "SHELL_BASED_SURFACE_MODEL",
        "CLOSED_SHELL",
        "OPEN_SHELL",
        "ORIENTED_CLOSED_SHELL",
        "ADVANCED_FACE",
        "FACE_OUTER_BOUND",
        "FACE_BOUND",
        "EDGE_LOOP",
        "ORIENTED_EDGE",
        "EDGE_CURVE",
        "VERTEX_POINT",
        "CARTESIAN_POINT",
        "DIRECTION",
        "VECTOR",
        "AXIS2_PLACEMENT_2D",
        "AXIS2_PLACEMENT_3D",
        "LINE",
        "CIRCLE",
        "ELLIPSE",
        "TRIMMED_CURVE",
        "B_SPLINE_CURVE_WITH_KNOTS",
        "PCURVE",
        "SURFACE_CURVE",
        "SEAM_CURVE",
        "DEFINITIONAL_REPRESENTATION",
        "PLANE",
        "CYLINDRICAL_SURFACE",
        "CONICAL_SURFACE",
        "SPHERICAL_SURFACE",
        "TOROIDAL_SURFACE",
        "B_SPLINE_SURFACE_WITH_KNOTS",
        "GEOMETRIC_REPRESENTATION_CONTEXT",
        "GLOBAL_UNIT_ASSIGNED_CONTEXT",
        "SI_UNIT",
        "CONVERSION_BASED_UNIT",
        "GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT",
        "UNCERTAINTY_MEASURE_WITH_UNIT",
        "LENGTH_MEASURE_WITH_UNIT",
        "MEASURE_WITH_UNIT",
        "ITEM_DEFINED_TRANSFORMATION",
        "REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION",
        "CARTESIAN_TRANSFORMATION_OPERATOR_3D",
    ];
    for id in &linked {
        if allow_degenerate
            && capability != STEP_INTERCHANGE_V8_CAPABILITY
            && component(
                &components(&entities, *id)?,
                "CARTESIAN_TRANSFORMATION_OPERATOR_3DNON_UNIFORM",
            )
            .is_some()
        {
            continue;
        }
        let (ty, _) = call(&entities, *id)?;
        if !REACHABLE_TYPES.contains(&ty) && !matches!(ty, "COMPLEX") {
            return Err(refuse(format!(
                "Reachable entity {ty} is outside the direct finite graph subset"
            )));
        }
    }
    let mut ignored = BTreeSet::new();
    for id in entities.keys() {
        if linked.contains(id) {
            continue;
        }
        if ap242_composition && relationship_components(&entities, *id)?.is_some() {
            continue;
        }
        if allow_degenerate
            && capability != STEP_INTERCHANGE_V8_CAPABILITY
            && component(
                &components(&entities, *id)?,
                "CARTESIAN_TRANSFORMATION_OPERATOR_3DNON_UNIFORM",
            )
            .is_some()
        {
            continue;
        }
        let (ty, _) = call(&entities, *id)?;
        if allow_degenerate
            && matches!(
                ty,
                "NEXT_ASSEMBLY_USAGE_OCCURRENCE" | "CONTEXT_DEPENDENT_SHAPE_REPRESENTATION"
            )
        {
            continue;
        }
        if allow_degenerate
            && matches!(
                ty,
                "DOCUMENT_FILE"
                    | "DOCUMENT"
                    | "DOCUMENT_TYPE"
                    | "DOCUMENT_REPRESENTATION_TYPE"
                    | "EXTERNALLY_DEFINED_ITEM"
                    | "EXTERNAL_SOURCE"
                    | "APPLIED_EXTERNAL_IDENTIFICATION_ASSIGNMENT"
            )
        {
            continue;
        }
        if UNREACHABLE_ALLOWLIST.contains(&ty) {
            ignored.insert(ty.to_string());
        } else {
            return Err(refuse(format!(
                "Unreachable entity {ty} is not presentation metadata allowlisted by /3"
            )));
        }
    }
    let (mut model, identity_map) = if ap242_composition {
        if occurrences.is_empty() {
            return Err(refuse(
                "Selected placement graph contains no manifold B-rep occurrence",
            ));
        }
        let mut aggregate = Model::empty(DEFAULT_STEP_TOLERANCE_MM)?;
        let mut aggregate_tolerance = None::<f64>;
        let mut aggregate_map = Vec::new();
        for occurrence in occurrences {
            let scale = length_scale(&entities, &occurrence.linked, false)?;
            let tolerance_mm = uncertainty_mm(&entities, &occurrence.linked)?
                .unwrap_or(DEFAULT_STEP_TOLERANCE_MM);
            aggregate_tolerance = Some(
                aggregate_tolerance.map_or(tolerance_mm, |current| current.min(tolerance_mm)),
            );
            let (mut component, map) = build_direct_roots(
                &entities,
                &occurrence.bodies,
                scale,
                DirectBuildOptions {
                    analytic_surfaces,
                    ap242_composition: true,
                    allow_degenerate,
                    whole_domain_proofs: matches!(
                        capability,
                        STEP_INTERCHANGE_V7_CAPABILITY
                            | STEP_INTERCHANGE_V8_CAPABILITY
                            | STEP_INTERCHANGE_V9_CAPABILITY
                    ),
                    interior_point_selectors: capability == STEP_INTERCHANGE_V9_CAPABILITY,
                    allow_open_shells,
                },
            )?;
            component.tolerance_mm = tolerance_mm;
            let component = crate::transform::affine(&component, occurrence.transform)?;
            append_direct_model(&mut aggregate, component, &map, &mut aggregate_map);
        }
        aggregate.tolerance_mm = aggregate_tolerance.unwrap_or(DEFAULT_STEP_TOLERANCE_MM);
        aggregate.rebuild_topology_ids();
        (aggregate, aggregate_map)
    } else {
        let scale = length_scale(&entities, &linked, true)?;
        let tolerance_mm =
            uncertainty_mm(&entities, &linked)?.unwrap_or(DEFAULT_STEP_TOLERANCE_MM);
        let placement = reachable_transform(&entities, &linked, scale, false)?;
        let (mut model, map) = build_direct_roots(
            &entities,
            &roots,
            scale,
            DirectBuildOptions {
                analytic_surfaces,
                ap242_composition: false,
                allow_degenerate,
                whole_domain_proofs: matches!(
                    capability,
                    STEP_INTERCHANGE_V7_CAPABILITY
                        | STEP_INTERCHANGE_V8_CAPABILITY
                        | STEP_INTERCHANGE_V9_CAPABILITY
                ),
                interior_point_selectors: capability == STEP_INTERCHANGE_V9_CAPABILITY,
                allow_open_shells,
            },
        )?;
        model.tolerance_mm = tolerance_mm;
        if let Some(matrix) = placement {
            model = crate::transform::affine(&model, matrix)?
        }
        (model, map)
    };
    // One occurrence can restore its original topology metadata. Repeated
    // definitions require occurrence-specific identities to avoid collisions.
    let identity = if occurrence_count > 1
        && (capability == STEP_INTERCHANGE_V9_CAPABILITY || allow_degenerate)
    {
        crate::step_interchange::StepIdentityReport {
            preserved: false,
            source: "assembly-definition-plus-occurrence",
            preserved_count: 0,
            created_count: identity_map.len(),
            lost_count: 0,
        }
    } else {
        restore_identity(&entities, &identity_map, &mut model)?
    };
    model.validate()?;
    let metadata_loss = if identity.preserved {
        vec![]
    } else if allow_degenerate {
        vec![
            "topology.occurrence_instance_ids_created".into(),
            "presentation_style".into(),
        ]
    } else {
        vec![
            "representation_item.name".into(),
            "product.id".into(),
            "product.name".into(),
            "product.description".into(),
            "presentation_style".into(),
        ]
    };
    let mut certificate_notes = vec![
        "direct_part21_topology",
        "line_circle_ellipse_trimmed_curve",
        "plane_and_rational_multispan_bspline_surface",
        "periodic_analytic_surfaces_typed_refused_parameterization",
        "shared_topology_senses_holes",
        "multi_body_multi_cavity",
        "si_and_positive_conversion_units",
        "reachable_rigid_placements_only",
        "independent_curve_pcurve_check",
        "bounded_reachable_graph",
    ];
    if matches!(
        capability,
        STEP_INTERCHANGE_V7_CAPABILITY
            | STEP_INTERCHANGE_V8_CAPABILITY
            | STEP_INTERCHANGE_V9_CAPABILITY
    ) {
        certificate_notes.push("whole_domain_rational_boundary_and_constant_pole_identity");
    }
    if capability == STEP_INTERCHANGE_V9_CAPABILITY {
        certificate_notes.extend([
            "certified_trimmed_curve_point_inverse",
            "bounded_freeform_collapsed_boundary_ownership",
            "invertible_general_affine_occurrences",
            "open_shell_surface_products",
        ]);
    }
    Ok((
        model,
        FeatureCertificate {
            capability,
            complete: true,
            notes: certificate_notes,
        },
        StepV3Report {
            identity,
            ignored_entities: ignored.into_iter().collect(),
            metadata_loss,
            instance_count: entities.len(),
            reachable_count: linked.len(),
            definition_identities,
            occurrence_identities,
            product_hierarchy,
            external_references,
        },
    ))
}

pub fn import_step_v3(text: &str) -> Result<(Model, FeatureCertificate, StepV3Report)> {
    import_step_direct(text, false, false, false, STEP_INTERCHANGE_V3_CAPABILITY)
}

pub fn import_step_v4(text: &str) -> Result<(Model, FeatureCertificate, StepV3Report)> {
    import_step_direct(text, true, false, false, STEP_INTERCHANGE_V4_CAPABILITY)
}

pub fn import_step_v5(text: &str) -> Result<(Model, FeatureCertificate, StepV3Report)> {
    import_step_direct(text, true, true, false, STEP_INTERCHANGE_V5_CAPABILITY)
}

pub fn import_step_v6(text: &str) -> Result<(Model, FeatureCertificate, StepV3Report)> {
    import_step_direct(text, true, true, true, STEP_INTERCHANGE_V6_CAPABILITY)
}

/// Append-only `/7` product route. The native direct graph subset is identical
/// to `/6`; `/7` adds bundle composition, durable lifecycle, and independent
/// semantic-conformance status in the product adapters. Keeping this entry
/// point distinct prevents a future `/7` claim from rewriting `/6` evidence.
pub fn import_step_v7(text: &str) -> Result<(Model, FeatureCertificate, StepV3Report)> {
    import_step_direct(text, true, true, true, STEP_INTERCHANGE_V7_CAPABILITY)
}

/// Qualified successor: `/7` remains rejected and immutable.
pub fn import_step_v8(text: &str) -> Result<(Model, StepV8Certificate, StepV3Report)> {
    let (model, _, report) =
        import_step_direct(text, true, true, true, STEP_INTERCHANGE_V8_CAPABILITY)?;
    let certificate = certify_step_v8_topology(&model)?;
    Ok((model, certificate, report))
}

pub fn import_step_v9(text: &str) -> Result<(Model, StepV8Certificate, StepV3Report)> {
    let (model, _, report) =
        import_step_direct(text, true, true, true, STEP_INTERCHANGE_V9_CAPABILITY)?;
    let mut certificate = certify_step_v8_topology(&model)?;
    certificate.capability = STEP_INTERCHANGE_V9_CAPABILITY;
    certificate.notes.extend([
        "certified_interior_point_selectors",
        "bounded_freeform_poles",
        "general_affine_occurrences",
        "retained_open_shell_products",
    ]);
    Ok((model, certificate, report))
}

fn step_v10_graph_identity(entities: &BTreeMap<usize, Entity>) -> Result<String> {
    let mut nodes = entities
        .keys()
        .map(|id| entity_digest(entities, *id))
        .collect::<Result<Vec<_>>>()?;
    nodes.sort();
    Ok(hash(&nodes.join("\n")))
}

/// Imports the editable B-rep projection while retaining the complete Part 21
/// graph. Entity-number changes do not alter `graph_identity`.
pub fn import_step_v10(text: &str) -> Result<(Model, StepV8Certificate, StepV10Document)> {
    let (model, mut certificate, report) = import_step_v9(text)?;
    let entities = parse(data_payload(text)?)?;
    let mut operator_identities = Vec::new();
    for id in entities.keys() {
        if let Some((_, _, operator)) = relationship_components(&entities, *id)? {
            operator_identities.push(format!("operator:{}", entity_digest(&entities, operator)?));
        }
    }
    operator_identities.sort();
    operator_identities.dedup();
    certificate.capability = STEP_INTERCHANGE_V10_CAPABILITY;
    certificate.notes.extend([
        "retained_affine_occurrence_graph",
        "exact_graph_isomorphism_identity",
    ]);
    let document = StepV10Document {
        source: text.to_string(),
        graph_identity: step_v10_graph_identity(&entities)?,
        definition_identities: report.definition_identities,
        occurrence_identities: report.occurrence_identities,
        product_hierarchy: report.product_hierarchy,
        operator_identities,
        metadata_loss: Vec::new(),
    };
    Ok((model, certificate, document))
}

/// Exports the authoritative graph byte-for-byte after revalidating its
/// identifier-independent canonical identity.
pub fn export_step_v10(document: &StepV10Document) -> Result<String> {
    if document.source.len() > MAX_OUTPUT_BYTES {
        return Err(refuse("STEP /10 output exceeds 16 MiB"));
    }
    let entities = parse(data_payload(&document.source)?)?;
    if step_v10_graph_identity(&entities)? != document.graph_identity {
        return Err(refuse("STEP /10 retained graph identity mismatch"));
    }
    Ok(document.source.clone())
}

#[cfg(test)]
#[path = "tests/step_interchange_v3.rs"]
mod tests;
