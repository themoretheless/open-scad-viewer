//! Original-source contact isolation by supporting planes and actual trim traces.
use super::*;
use nurbs_core::{
    original_graph_transition::{
        PolynomialPlaneTrace, polynomial_supporting_trace, polynomial_trim_trace_covered,
    },
    surface::Surface,
};
pub struct OriginalPairAudit {
    retained: RetainedPairAudit,
    contacts: Vec<([usize; 2], Vec<usize>)>,
    unresolved: Vec<[usize; 2]>,
    charts: FaceChartAudit,
}
impl OriginalPairAudit {
    pub fn retained(&self) -> &RetainedPairAudit {
        &self.retained
    }
    pub fn contacts(&self) -> &[([usize; 2], Vec<usize>)] {
        &self.contacts
    }
    pub fn unresolved(&self) -> &[[usize; 2]] {
        &self.unresolved
    }
    pub fn charts(&self) -> &FaceChartAudit {
        &self.charts
    }
    /// Geometric embedding of this native closed shell only. Radius/thickness,
    /// material volume and application/STEP publication are separate gates.
    pub fn globally_embedded(&self) -> bool {
        self.unresolved.is_empty() && self.charts.unresolved().is_empty()
    }
}
fn source(face: &FaceGeometry) -> Option<&Surface> {
    match face {
        FaceGeometry::Retained(r) => Some(r.source()),
        FaceGeometry::Imported(f) => Some(f.source()),
        _ => None,
    }
}
fn covered(
    top: &RoofTopology,
    face: usize,
    shared: &[usize],
    trace: &PolynomialPlaneTrace,
    b: &mut Budget,
) -> Result<bool> {
    let PolynomialPlaneTrace::Boundary {
        parameter_axis,
        at_end,
        ..
    } = trace
    else {
        return Ok(false);
    };
    let f = &top.faces[face];
    let Some(s) = source(&f.surface) else {
        return Ok(false);
    };
    let curves = match &f.surface {
        FaceGeometry::Retained(r) => r
            .uses()
            .iter()
            .map(|(u, reversed)| {
                let mut p = u.uv_points().to_vec();
                if *reversed {
                    p.reverse();
                }
                p
            })
            .collect::<Vec<_>>(),
        FaceGeometry::Imported(f) => f.uses.iter().map(|u| u.uv.to_vec()).collect(),
        _ => return Ok(false),
    };
    let flags = top.loops[f.outer]
        .coedges
        .iter()
        .map(|c| shared.contains(&c.edge))
        .collect::<Vec<_>>();
    let value = if *parameter_axis == 0 {
        s.knots_u[if *at_end {
            s.control_points.len()
        } else {
            s.degree_u
        }]
    } else {
        s.knots_v[if *at_end {
            s.control_points[0].len()
        } else {
            s.degree_v
        }]
    };
    polynomial_trim_trace_covered(
        &curves,
        &flags,
        *parameter_axis,
        *at_end,
        &Q::from_float(value).unwrap(),
        b,
    )
}
fn qualify(
    top: &RoofTopology,
    faces: [usize; 2],
    shared: &[usize],
    b: &mut Budget,
) -> Result<bool> {
    if shared.is_empty() {
        return Ok(false);
    }
    let [i, j] = faces;
    let (Some(a), Some(z)) = (source(&top.faces[i].surface), source(&top.faces[j].surface)) else {
        return Ok(false);
    };
    for axis in 0..3 {
        let plane = top.edges[shared[0]].curve.points()[0][axis].clone();
        if !shared.iter().all(|edge| {
            top.edges[*edge]
                .curve
                .points()
                .iter()
                .all(|p| p[axis] == plane)
        }) {
            continue;
        }
        let (Some(at), Some(zt)) = (
            polynomial_supporting_trace(a, axis, &plane, b)?,
            polynomial_supporting_trace(z, axis, &plane, b)?,
        ) else {
            continue;
        };
        let side = |t: &PolynomialPlaneTrace| match t {
            PolynomialPlaneTrace::Plane => 0,
            PolynomialPlaneTrace::Boundary { side, .. } => *side,
        };
        let (sa, sz) = (side(&at), side(&zt));
        if (sa == 0 && sz == 0) || (sa != 0 && sa == sz) {
            continue;
        }
        // Either complete trim trace suffices: all possible pair intersections
        // are in the plane, and this face reaches it only on shared edges.
        if covered(top, i, shared, &at, b)? || covered(top, j, shared, &zt, b)? {
            return Ok(true);
        }
    }
    Ok(false)
}
impl ClosedShell {
    /// Complete all-pair accounting, exact contact isolation and within-face
    /// injectivity. No body is created and no approximate Model is substituted.
    pub fn inspect_original_pairs(
        &self,
        max_pairs: usize,
        b: &mut Budget,
    ) -> Result<OriginalPairAudit> {
        let retained = self.inspect_retained_pairs(max_pairs, b)?;
        let top = self.topology();
        let mut contacts = Vec::new();
        let mut unresolved = Vec::new();
        for pair in retained.unresolved() {
            let hull = retained
                .transverse()
                .hulls()
                .pairs()
                .iter()
                .find(|p| p.faces == *pair)
                .unwrap();
            if qualify(top, *pair, &hull.shared_edges, b)? {
                contacts.push((*pair, hull.shared_edges.clone()));
            } else {
                unresolved.push(*pair);
            }
        }
        let charts = self.inspect_face_charts(b)?;
        Ok(OriginalPairAudit {
            retained,
            contacts,
            unresolved,
            charts,
        })
    }
}
