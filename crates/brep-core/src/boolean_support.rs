//! Regularized empty semantics and conservative carrier separation for CSG.
use crate::{Model, TopologyIds};
use nurbs_core::{Error, Result};

fn require_solid(model: &Model) -> Result<()> {
    if model.is_empty() {
        return Ok(());
    }
    let mut owners = vec![0usize; model.shells.len()];
    for body in &model.bodies {
        for &shell in std::iter::once(&body.outer_shell).chain(&body.inner_shells) {
            owners[shell] += 1;
        }
    }
    if owners.iter().any(|&count| count != 1)
        || model.bodies.is_empty()
        || model.shells.iter().any(|shell| !shell.closed)
    {
        return Err(Error::new(
            "BREP_UNSUPPORTED_OPERATION",
            "Boolean operands require closed bodies with distinct owned shells",
        ));
    }
    Ok(())
}

// Positive rational weights put the complete surface in this control hull.
// Vertex bounds alone do not enclose curved carriers.
fn bounds(model: &Model) -> [[f64; 3]; 2] {
    let mut result = [[f64::INFINITY; 3], [f64::NEG_INFINITY; 3]];
    for point in model
        .faces
        .iter()
        .flat_map(|f| f.surface.control_points.iter().flatten())
    {
        for axis in 0..3 {
            result[0][axis] = result[0][axis].min(point[axis]);
            result[1][axis] = result[1][axis].max(point[axis]);
        }
    }
    result
}

/// Closed interval endpoint contact is regularized: identical cross-sections
/// meeting on a cap form one prism. Separated intervals retain distinct bodies.
pub(crate) fn merge_intervals(mut intervals: Vec<[f64; 2]>) -> Vec<[f64; 2]> {
    intervals.retain(|i| i[0] < i[1]);
    intervals.sort_by(|a, b| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])));
    let mut merged = Vec::<[f64; 2]>::new();
    for interval in intervals {
        if let Some(last) = merged.last_mut() {
            if interval[0] <= last[1] {
                last[1] = last[1].max(interval[1]);
                continue;
            }
        }
        merged.push(interval);
    }
    merged
}

/// Shared "extrude → merge disjoint slabs" loop for the prismatic Boolean
/// cells: each merged interval is extruded through `construct`, then joined
/// to the accumulated result through `join`. Merged intervals are disjoint,
/// but a non-strict axial gap still refuses with `gap_error` instead of
/// concatenating touching topology.
pub(crate) fn extrude_merged_intervals(
    loops: &[Vec<nurbs_core::curve::Curve>],
    intervals: Vec<[f64; 2]>,
    tolerance: f64,
    construct: impl Fn(&[Vec<nurbs_core::curve::Curve>], f64, f64) -> Result<Model>,
    join: impl Fn(&Model, &Model) -> Result<Model>,
    gap_error: (&'static str, &'static str),
) -> Result<Model> {
    let mut result: Option<Model> = None;
    let mut previous_high = f64::NEG_INFINITY;
    for [low, high] in merge_intervals(intervals) {
        let part = construct(loops, low, high)?;
        result = Some(if let Some(existing) = result {
            if !(previous_high < low) {
                return Err(Error::new(gap_error.0, gap_error.1));
            }
            join(&existing, &part)?
        } else {
            part
        });
        previous_high = high;
    }
    result.map_or_else(|| Model::empty(tolerance), Ok)
}

/// Concatenate independently owned topology after proving spatial separation.
/// No coordinate weld, surface fitting or mesh reconstruction is involved.
pub(crate) fn separated_union(a: &Model, b: &Model) -> Result<Model> {
    let mut result = a.clone();
    result.tolerance_mm = a.tolerance_mm.max(b.tolerance_mm);
    let (v, e, l, f, s) = (
        a.vertices.len(),
        a.edges.len(),
        a.loops.len(),
        a.faces.len(),
        a.shells.len(),
    );
    result.vertices.extend(b.vertices.iter().cloned());
    result.edges.extend(b.edges.iter().cloned().map(|mut edge| {
        edge.vertices.iter_mut().for_each(|id| *id += v);
        edge
    }));
    result.loops.extend(b.loops.iter().cloned().map(|mut wire| {
        wire.coedges.iter_mut().for_each(|use_| use_.edge += e);
        wire
    }));
    result.faces.extend(b.faces.iter().cloned().map(|mut face| {
        face.outer += l;
        face.holes.iter_mut().for_each(|id| *id += l);
        face
    }));
    result
        .shells
        .extend(b.shells.iter().cloned().map(|mut shell| {
            shell.faces.iter_mut().for_each(|use_| use_.face += f);
            shell
        }));
    result
        .bodies
        .extend(b.bodies.iter().cloned().map(|mut body| {
            body.outer_shell += s;
            body.inner_shells.iter_mut().for_each(|id| *id += s);
            body
        }));
    result.1 = TopologyIds::default();
    result.rebuild_topology_ids();
    result.inherit_topology_ids(&[a, b]);
    result.validate()?;
    Ok(result)
}

pub(crate) fn simplify(a: &Model, b: &Model, operation: &str) -> Result<Option<Model>> {
    require_solid(a)?;
    require_solid(b)?;
    let empty = || Model::empty(a.tolerance_mm.max(b.tolerance_mm));
    if a.is_empty() || b.is_empty() {
        return Ok(Some(match operation {
            "intersection" => empty()?,
            "difference" => {
                if a.is_empty() {
                    empty()?
                } else {
                    a.clone()
                }
            }
            _ => {
                if a.is_empty() {
                    b.clone()
                } else {
                    a.clone()
                }
            }
        }));
    }
    if value_codec::Serialize::to_value(&a.0) == value_codec::Serialize::to_value(&b.0) {
        return Ok(Some(match operation {
            "difference" | "xor" => empty()?,
            _ => a.clone(),
        }));
    }
    let [amin, amax] = bounds(a);
    let [bmin, bmax] = bounds(b);
    let separated = (0..3).any(|axis| amax[axis] < bmin[axis] || bmax[axis] < amin[axis]);
    let no_interior_overlap =
        (0..3).any(|axis| amax[axis] <= bmin[axis] || bmax[axis] <= amin[axis]);
    if operation == "intersection" && no_interior_overlap {
        return Ok(Some(empty()?));
    }
    if separated {
        return Ok(Some(match operation {
            "difference" => a.clone(),
            "intersection" => empty()?,
            _ => separated_union(a, b)?,
        }));
    }
    Ok(None)
}
