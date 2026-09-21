//! Exact lifted-profile imprint for admitted parallel sweep solids.
//!
//! This is curved topology authorship, not prism fallback: source rational
//! spans are intersected in the planar arrangement and extruded exactly.
//! Unsupported frames or profiles return `None` and remain fail-closed.

use crate::{Model, planar_trim, prism};
use nurbs_core::{Error, Result};

pub(crate) fn boolean(a: &Model, b: &Model, operation: &str) -> Result<Option<Model>> {
    if let Some(result) = local_boolean(a, b, operation)? {
        return Ok(Some(result));
    }
    if let Some(result) = crate::stepped_prism::boolean(a, b, operation)? {
        return Ok(Some(result));
    }
    let Some((local_a, local_b, frame)) = crate::prism_frame::localize(a, b)? else {
        return Ok(None);
    };
    let result = match local_boolean(&local_a, &local_b, operation)? {
        Some(result) => Some(result),
        None => crate::stepped_prism::boolean(&local_a, &local_b, operation)?,
    };
    result.map(|result| frame.restore(&result)).transpose()
}

fn subtract_interval(a: [f64; 2], b: [f64; 2]) -> Vec<[f64; 2]> {
    if b[1] <= a[0] || b[0] >= a[1] {
        return vec![a];
    }
    let mut remaining = Vec::new();
    if a[0] < b[0] {
        remaining.push([a[0], a[1].min(b[0])]);
    }
    if b[1] < a[1] {
        remaining.push([a[0].max(b[1]), a[1]]);
    }
    remaining
}

fn merge_intervals(mut intervals: Vec<[f64; 2]>) -> Vec<[f64; 2]> {
    intervals.retain(|i| i[0] < i[1]);
    intervals.sort_by(|a, b| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])));
    let mut merged = Vec::<[f64; 2]>::new();
    for interval in intervals {
        if let Some(last) = merged.last_mut()
            && interval[0] <= last[1]
        {
            last[1] = last[1].max(interval[1]);
            continue;
        }
        merged.push(interval);
    }
    merged
}

fn interval_result(
    loops: &[Vec<nurbs_core::curve::Curve>],
    intervals: Vec<[f64; 2]>,
    tolerance: f64,
    sources: &[&Model],
) -> Result<Model> {
    let mut result: Option<Model> = None;
    let mut previous_high = f64::NEG_INFINITY;
    for [low, high] in merge_intervals(intervals) {
        let mut part = prism::extrude(loops, low, high)?;
        part.tolerance_mm = tolerance;
        part.inherit_topology_ids(sources);
        part.validate()?;
        result = Some(if let Some(existing) = result {
            if previous_high >= low {
                return Err(Error::new(
                    "BREP_PROFILE_IMPRINT_REFUSED",
                    "Profile interval components require a strict axial gap",
                ));
            }
            crate::boolean_support::separated_union(&existing, &part)?
        } else {
            part
        });
        previous_high = high;
    }
    result.map_or_else(|| Model::empty(tolerance), Ok)
}

fn local_boolean(a: &Model, b: &Model, operation: &str) -> Result<Option<Model>> {
    let Some(pa) = prism::recognize(a)? else {
        return Ok(None);
    };
    let Some(pb) = prism::recognize(b)? else {
        return Ok(None);
    };
    let tolerance = a.tolerance_mm.max(b.tolerance_mm);
    let construct = |loops: &[Vec<nurbs_core::curve::Curve>], lo, hi| -> Result<Model> {
        if loops.is_empty() {
            return Model::empty(tolerance);
        }
        let mut result = prism::extrude(loops, lo, hi)?;
        result.tolerance_mm = tolerance;
        result.inherit_topology_ids(&[a, b]);
        result.validate()?;
        Ok(result)
    };
    let low = pa.z_min.max(pb.z_min);
    let high = pa.z_max.min(pb.z_max);
    if operation == "intersection" {
        if low >= high {
            return Ok(Some(Model::empty(tolerance)?));
        }
        let loops = planar_trim::boolean(&pa.loops, &pb.loops, operation, tolerance)?;
        return Ok(Some(construct(&loops, low, high)?));
    }
    if operation == "difference" && low >= high {
        return Ok(Some(a.clone()));
    }
    if operation == "difference" {
        let overlap = planar_trim::boolean(&pa.loops, &pb.loops, "intersection", tolerance)?;
        if overlap.is_empty() {
            return Ok(Some(a.clone()));
        }
    }
    let same_height = pa.z_min == pb.z_min && pa.z_max == pb.z_max;
    let through_cut = operation == "difference" && pb.z_min <= pa.z_min && pb.z_max >= pa.z_max;
    if same_height || through_cut {
        let loops = planar_trim::boolean(&pa.loops, &pb.loops, operation, tolerance)?;
        return Ok(Some(construct(&loops, pa.z_min, pa.z_max)?));
    }
    let a_only = planar_trim::boolean(&pa.loops, &pb.loops, "difference", tolerance)?;
    let a_interval = [pa.z_min, pa.z_max];
    let b_interval = [pb.z_min, pb.z_max];
    if operation == "difference" && a_only.is_empty() {
        return Ok(Some(interval_result(
            &pa.loops,
            subtract_interval(a_interval, b_interval),
            tolerance,
            &[a, b],
        )?));
    }
    let b_only = planar_trim::boolean(&pb.loops, &pa.loops, "difference", tolerance)?;
    if a_only.is_empty() && b_only.is_empty() {
        let intervals = match operation {
            "union" => vec![a_interval, b_interval],
            "xor" => {
                let mut spans = subtract_interval(a_interval, b_interval);
                spans.extend(subtract_interval(b_interval, a_interval));
                spans
            }
            _ => {
                return Err(Error::new(
                    "BREP_INVALID_OPERATION",
                    "Unsupported profile interval Boolean operation",
                ));
            }
        };
        return Ok(Some(interval_result(
            &pa.loops,
            intervals,
            tolerance,
            &[a, b],
        )?));
    }
    crate::stepped_prism::boolean(a, b, operation)
}

/// Exact planar-profile arrangement entry point for the finite NURBS Boolean
/// contact cell. Callers must first prove that every source face is affine
/// planar. Unlike [`boolean`], this does not try stepped-prism or frame
/// fallbacks: one common +Z profile arrangement must author the result.
pub(crate) fn exact_planar_contact_boolean(a: &Model, b: &Model, operation: &str) -> Result<Model> {
    let pa = prism::recognize(a)?.ok_or_else(|| {
        Error::new(
            "BREP_PROFILE_IMPRINT_REFUSED",
            "First NURBS contact operand is not an exact +Z profile prism",
        )
    })?;
    let pb = prism::recognize(b)?.ok_or_else(|| {
        Error::new(
            "BREP_PROFILE_IMPRINT_REFUSED",
            "Second NURBS contact operand is not an exact +Z profile prism",
        )
    })?;
    let tolerance = a.tolerance_mm.max(b.tolerance_mm);
    if (pa.z_min - pb.z_min).abs() > tolerance || (pa.z_max - pb.z_max).abs() > tolerance {
        return Err(Error::new(
            "BREP_PROFILE_IMPRINT_REFUSED",
            "NURBS contact requires one coincident extrusion span; stepped fallback is forbidden",
        ));
    }
    local_boolean(a, b, operation)?.ok_or_else(|| {
        Error::new(
            "BREP_PROFILE_IMPRINT_REFUSED",
            "NURBS planar contact requires one exact common-axis profile arrangement",
        )
    })
}
