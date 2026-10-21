//! Exact image partitions of original axis-aligned rational boundary curves.
use super::{BoundaryAddress, inspect};
use crate::{Model, Result};
use nurbs_core::{Error, curve::Curve};
use std::collections::BTreeMap;
#[derive(Clone, Debug)]
pub struct LinearMatch {
    pub boundaries: [usize; 2],
    pub axis: usize,
    /// Exact nominal binary64 world coordinates, not rounded trim fractions.
    pub range: [f64; 2],
}
#[derive(Clone, Debug)]
pub struct LinearPatchSetReport {
    pub boundaries: Vec<BoundaryAddress>,
    pub matches: Vec<LinearMatch>,
    pub unsupported: Vec<usize>,
    pub open_intervals: usize,
    pub ambiguous_intervals: usize,
    pub cells: usize,
    pub source_agreement_complete: bool,
    pub all_boundaries_paired: bool,
    pub edge_tolerance_mm: f64,
}
struct Segment {
    id: usize,
    range: [f64; 2],
    positive: bool,
}
fn image(c: &Curve) -> Option<(usize, [u64; 2], [f64; 2], bool)> {
    if c.periodic {
        return None;
    }
    let [a, b] = c.domain();
    let n = c.control_points.len();
    if !c.knots[..=c.degree].iter().all(|&t| t == a) || !c.knots[n..].iter().all(|&t| t == b) {
        return None;
    }
    let p = &c.control_points[0];
    let q = c.control_points.last()?;
    let axes: Vec<_> = (0..3).filter(|&k| p[k] != q[k]).collect();
    if axes.len() != 1 {
        return None;
    }
    let axis = axes[0];
    let range = [p[axis].min(q[axis]), p[axis].max(q[axis])];
    if c.control_points.iter().any(|v| {
        (0..3).any(|k| {
            if k == axis {
                v[k] < range[0] || v[k] > range[1]
            } else {
                v[k] != p[k]
            }
        })
    }) {
        return None;
    }
    let fixed: Vec<_> = (0..3)
        .filter(|&k| k != axis)
        .map(|k| {
            if p[k] == 0. {
                0_f64.to_bits()
            } else {
                p[k].to_bits()
            }
        })
        .collect();
    // Positive rational convex hull gives image inclusion. Exact endpoint
    // values plus continuity and IVT give surjectivity onto this whole segment.
    Some((axis, [fixed[0], fixed[1]], range, q[axis] > p[axis]))
}
/// Automatic one-to-many seam partitioning for original straight axis-aligned
/// boundary IMAGES, including nonlinear rational parameterizations. Exact
/// coordinate comparisons partition images at every authored endpoint; no
/// projection, fitted proxy, midpoint classification or rounded trim is used.
/// Every positive interval must have exactly two uses from distinct models
/// with opposite face orientation. Shared endpoints do not count as intervals.
/// Full original curve/surface agreement is independently required for all
/// source faces. Source lifts are consequently within twice model tolerance as
/// sets; parameter bijection, trim simplicity, embedding and volume are separate.
/// Unsupported curved images remain explicit and cannot grant closure.
/// Limits: 1..256 single-shell models, <=1024 free boundaries, <=100000
/// partition intervals, and a shared 1..1000000 agreement-cell budget.
pub fn inspect_linear_patch_set(
    models: &[Model],
    max_intervals: usize,
    max_cells: usize,
) -> Result<LinearPatchSetReport> {
    if models.is_empty()
        || models.len() > 256
        || !(1..=100000).contains(&max_intervals)
        || !(1..=1000000).contains(&max_cells)
        || models.iter().any(|m| {
            m.tolerance_mm != models[0].tolerance_mm || m.shells.len() != 1 || m.faces.is_empty()
        })
    {
        return Err(Error::new(
            "BREP_INVALID_INPUT",
            "Linear patch closure needs 1..256 nonempty single-shell models, equal tolerance and bounded work",
        ));
    }
    let mut out = LinearPatchSetReport {
        boundaries: vec![],
        matches: vec![],
        unsupported: vec![],
        open_intervals: 0,
        ambiguous_intervals: 0,
        cells: 0,
        source_agreement_complete: true,
        all_boundaries_paired: false,
        edge_tolerance_mm: models[0].tolerance_mm,
    };
    let mut groups = BTreeMap::<(usize, [u64; 2]), Vec<Segment>>::new();
    for (id, m) in models.iter().enumerate() {
        m.validate_boundary_diagnostic_inputs()?;
        let mut selected_faces = vec![false; m.faces.len()];
        for face_use in &m.shells[0].faces {
            if selected_faces[face_use.face] {
                return Err(Error::new(
                    "BREP_INVALID_INPUT",
                    "Every source face must occur exactly once in its shell",
                ));
            }
            selected_faces[face_use.face] = true;
        }
        if selected_faces.iter().any(|selected| !selected) {
            return Err(Error::new(
                "BREP_INVALID_INPUT",
                "Patch closure cannot omit source faces",
            ));
        }
        if out.cells < max_cells {
            let r = inspect(m, max_cells - out.cells)?;
            out.cells += r.agreement.cells;
            out.source_agreement_complete &=
                r.agreement.complete && r.shells.iter().all(|s| s.incompatible_edges.is_empty());
        } else {
            out.source_agreement_complete = false;
        }
        let mut incidence = vec![0usize; m.edges.len()];
        for c in m.loops.iter().flat_map(|l| &l.coedges) {
            incidence[c.edge] += 1;
        }
        for face_use in &m.shells[0].faces {
            let f = &m.faces[face_use.face];
            for wire in std::iter::once(f.outer).chain(f.holes.iter().copied()) {
                for (coedge, c) in m.loops[wire].coedges.iter().enumerate() {
                    let edge = &m.edges[c.edge];
                    if incidence[c.edge] != 1 || edge.degenerate {
                        continue;
                    }
                    if out.boundaries.len() == 1024 {
                        return Err(Error::new(
                            "BREP_RESOURCE_LIMIT",
                            "Linear patch closure supports at most 1024 free boundaries",
                        ));
                    }
                    let index = out.boundaries.len();
                    out.boundaries.push(BoundaryAddress {
                        model: id,
                        face: face_use.face,
                        wire,
                        coedge,
                    });
                    if let Some((axis, support, range, positive)) = image(&edge.curve) {
                        groups.entry((axis, support)).or_default().push(Segment {
                            id: index,
                            range,
                            positive: positive ^ c.reversed ^ face_use.reversed,
                        });
                    } else {
                        out.unsupported.push(index);
                    }
                }
            }
        }
    }
    let mut intervals = 0;
    for ((axis, _), segments) in groups {
        let mut cuts: Vec<_> = segments.iter().flat_map(|s| s.range).collect();
        cuts.sort_by(f64::total_cmp);
        cuts.dedup_by(|a, b| *a == *b);
        for cut in cuts.windows(2) {
            if cut[0] >= cut[1] {
                continue;
            }
            let active: Vec<_> = segments
                .iter()
                .filter(|s| s.range[0] <= cut[0] && s.range[1] >= cut[1])
                .collect();
            if active.is_empty() {
                continue;
            }
            if intervals == max_intervals {
                return Err(Error::new(
                    "BREP_RESOURCE_LIMIT",
                    "Linear seam partition interval budget exhausted",
                ));
            }
            intervals += 1;
            if active.len() == 1 {
                out.open_intervals += 1;
                continue;
            }
            if active.len() != 2
                || active[0].positive == active[1].positive
                || out.boundaries[active[0].id].model == out.boundaries[active[1].id].model
            {
                out.ambiguous_intervals += 1;
                continue;
            }
            out.matches.push(LinearMatch {
                boundaries: [active[0].id, active[1].id],
                axis,
                range: [cut[0], cut[1]],
            });
        }
    }
    out.all_boundaries_paired = out.source_agreement_complete
        && out.unsupported.is_empty()
        && out.open_intervals == 0
        && out.ambiguous_intervals == 0
        && !out.boundaries.is_empty();
    Ok(out)
}
