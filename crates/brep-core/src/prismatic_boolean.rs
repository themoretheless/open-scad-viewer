//! Finite analytic CSG envelope for common-axis prismatic solids.
//! The 2D arrangement retains rational source spans; extrusion authors the 3D
//! topology. No tessellation or numeric intersection report authors a boundary.
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
fn interval_result(
    loops: &[Vec<nurbs_core::curve::Curve>],
    intervals: Vec<[f64; 2]>,
    tolerance: f64,
    construct: impl Fn(&[Vec<nurbs_core::curve::Curve>], f64, f64) -> Result<Model>,
) -> Result<Model> {
    crate::boolean_support::extrude_merged_intervals(
        loops,
        intervals,
        tolerance,
        construct,
        // Complete surface control nets remain in these disjoint Z slabs.
        // operations::boolean takes its conservative separation path; no
        // curve intersections, fitting or tessellation join these bodies.
        |existing, part| crate::operations::boolean(existing, part, "union"),
        (
            "BREP_UNSUPPORTED_OPERATION",
            "Interval components require a strict Z gap before topology concatenation",
        ),
    )
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
    if operation == "difference" && (low >= high) {
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
    // Profile containment is established by the retained planar arrangement,
    // never inferred from bounding boxes or a sampled representative point.
    let a_only = planar_trim::boolean(&pa.loops, &pb.loops, "difference", tolerance)?;
    let a_interval = [pa.z_min, pa.z_max];
    let b_interval = [pb.z_min, pb.z_max];
    if operation == "difference" && a_only.is_empty() {
        return Ok(Some(interval_result(
            &pa.loops,
            subtract_interval(a_interval, b_interval),
            tolerance,
            construct,
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
                    "Unsupported interval Boolean operation",
                ));
            }
        };
        return Ok(Some(interval_result(
            &pa.loops, intervals, tolerance, construct,
        )?));
    }
    crate::stepped_prism::boolean(a, b, operation)
}
