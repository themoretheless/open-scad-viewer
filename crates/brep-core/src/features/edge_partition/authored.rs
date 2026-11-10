//! Atomic authored edge subdivisions with fresh original-curve and face lifts.
use super::*;
/// Split using one evaluation of the original 3D curve. Existing API and proof
/// obligations are unchanged; canonical source-root points use the crate path.
pub fn partition(
    source: &Model,
    edge: usize,
    parameter: f64,
    cuts: &[PcurveCut],
    max_cells: usize,
) -> Result<Partition> {
    partition_impl(source, edge, parameter, cuts, max_cells, None)
}
pub(crate) fn partition_with_position(
    source: &Model,
    edge: usize,
    parameter: f64,
    cuts: &[PcurveCut],
    position: [f64; 3],
    tolerance: f64,
    max_cells: usize,
) -> Result<Partition> {
    partition_impl(
        source,
        edge,
        parameter,
        cuts,
        max_cells,
        Some((position, tolerance)),
    )
}
/// Split a nonperiodic 3D edge and all one or two incident face-local pcurves.
/// Independent natural pcurve cut parameters must be supplied for every use.
/// Children share one authored vertex, including both adjacent faces. Their
/// endpoints are assigned from one evaluation of the original 3D curve; every
/// resulting child is independently checked against the complete original
/// representation and each original support surface. No endpoint-only proof,
/// snapping authorization, exact-identity claim or solid certificate is implied.
/// Returns no model if any continuous proof or structural check fails.
fn partition_impl(
    source: &Model,
    edge_id: usize,
    parameter: f64,
    cuts: &[PcurveCut],
    max_cells: usize,
    authored: Option<([f64; 3], f64)>,
) -> Result<Partition> {
    source.validate()?;
    if !source.persistent_naming_complete() {
        return Err(invalid("Source topology identities must be complete"));
    }
    if !(1..=1000000).contains(&max_cells) {
        return Err(invalid("Work must be in 1..1000000 cells"));
    }
    let edge = source
        .edges
        .get(edge_id)
        .ok_or_else(|| invalid("Edge index is out of range"))?;
    let [a, b] = edge.curve.domain();
    if edge.degenerate
        || edge.curve.periodic
        || !parameter.is_finite()
        || !(a < parameter && parameter < b)
    {
        return Err(invalid(
            "Need a nondegenerate nonperiodic edge and an interior natural parameter",
        ));
    }
    let mut uses = vec![];
    for (face, f) in source.faces.iter().enumerate() {
        for wire in std::iter::once(f.outer).chain(f.holes.iter().copied()) {
            for (coedge, c) in source.loops[wire].coedges.iter().enumerate() {
                if c.edge == edge_id {
                    uses.push((face, wire, coedge));
                }
            }
        }
    }
    if !(1..=2).contains(&uses.len())
        || cuts.len() != uses.len()
        || cuts.iter().any(|c| {
            cuts.iter()
                .filter(|d| d.wire == c.wire && d.coedge == c.coedge)
                .count()
                != 1
        })
        || cuts
            .iter()
            .any(|c| !uses.iter().any(|&(_, w, k)| w == c.wire && k == c.coedge))
    {
        return Err(invalid(
            "Exactly one independent pcurve cut is required for every incident use",
        ));
    }

    let tolerance = authored.map_or(source.tolerance_mm, |(_, t)| t);
    if !tolerance.is_finite()
        || tolerance <= 0.
        || tolerance > source.tolerance_mm
        || authored.is_some_and(|(_, t)| {
            source
                .1
                .local_tolerances_mm
                .get(&source.1.edges[edge_id])
                .is_some_and(|local| t > *local)
        })
    {
        return Err(invalid(
            "Authored partition may not enlarge model or local edge tolerance",
        ));
    }
    if authored.is_some() {
        for &(_, wire, coedge) in &uses {
            let c = &source.loops[wire].coedges[coedge];
            if c.trim().is_some_and(|t| {
                t.curve_parameter != edge.curve.domain()
                    || t.pcurve_parameter != c.pcurve.domain()
                    || t.periodic_lift != [[0; 2]; 2]
            }) {
                return Err(unresolved(
                    "Authored edge partition needs complete original coedge windows",
                ));
            }
        }
    }
    let position: [f64; 3] = if let Some((p, _)) = authored {
        p
    } else {
        edge.curve
            .evaluate(parameter)?
            .point
            .try_into()
            .map_err(|_| invalid("Edge must be three-dimensional"))?
    };
    if position.iter().any(|v| !v.is_finite() || v.abs() > 1e6) {
        return Err(invalid("Invalid authored vertex position"));
    }
    let mut children = edge.curve.split(parameter)?;
    let last = children[0].control_points.len() - 1;
    children[0].control_points[last] = position.to_vec();
    children[1].control_points[0] = position.to_vec();

    // An exact representation copy of C(u), independent of v. This proof-only
    // carrier retains the original knots, weights and poles, rather than an
    // extracted/rounded proxy for the original restriction.
    let carrier = Surface {
        degree_u: edge.curve.degree,
        degree_v: 1,
        knots_u: edge.curve.knots.clone(),
        knots_v: vec![0., 0., 1., 1.],
        control_points: edge
            .curve
            .control_points
            .iter()
            .map(|p| vec![p.clone(), p.clone()])
            .collect(),
        weights: edge.curve.weights.iter().map(|&w| vec![w, w]).collect(),
        periodic_u: false,
        periodic_v: false,
    };
    let mut cells = 0;
    for (child, domain) in children.iter().zip([[a, parameter], [parameter, b]]) {
        let pcurve = crate::line(vec![domain[0], 0.], vec![domain[1], 0.]);
        check_agreement(
            child, &pcurve, &carrier, false, tolerance, &mut cells, max_cells,
        )?;
    }
    let mut model = source.clone();
    let vertex = model.vertices.len();
    model.vertices.push(Vertex { point: position });
    let child_edges = [edge_id, model.edges.len()];
    model.edges[edge_id] = Edge {
        degenerate: false,
        vertices: [edge.vertices[0], vertex],
        curve: children[0].clone(),
    };
    model.edges.push(Edge {
        degenerate: false,
        vertices: [vertex, edge.vertices[1]],
        curve: children[1].clone(),
    });
    for (face, wire, coedge) in uses {
        let original = &source.loops[wire].coedges[coedge];
        let cut = cuts
            .iter()
            .find(|c| c.wire == wire && c.coedge == coedge)
            .unwrap();
        if original.pcurve.periodic {
            return Err(invalid(
                "Periodic pcurve partition needs explicit lifted chart ownership",
            ));
        }
        let [p0, p1] = original.pcurve.domain();
        if !cut.parameter.is_finite() || !(p0 < cut.parameter && cut.parameter < p1) {
            return Err(invalid(
                "Every pcurve cut must be strictly inside its original natural domain",
            ));
        }
        let pc = original.pcurve.split(cut.parameter)?;
        let order = if original.reversed { [1, 0] } else { [0, 1] };
        let replacements: Vec<_> = pc
            .into_iter()
            .zip(order)
            .map(|(pcurve, i)| {
                let window = brep_topology::CoedgeTrim {
                    curve_parameter: children[i].domain(),
                    pcurve_parameter: pcurve.domain(),
                    periodic_lift: [[0; 2]; 2],
                };
                let c = Coedge::new(child_edges[i], original.reversed, pcurve);
                if authored.is_some() && original.trim().is_some() {
                    c.with_trim(window)
                } else {
                    Ok(c)
                }
            })
            .collect::<Result<Vec<_>>>()?;
        for c in &replacements {
            check_agreement(
                &model.edges[c.edge].curve,
                &c.pcurve,
                &source.faces[face].surface,
                c.reversed,
                tolerance,
                &mut cells,
                max_cells,
            )?;
        }
        // A valid face uses this source edge at most once in each loop. Reject
        // ambiguous self-seams instead of shifting addresses during mutation.
        if model.loops[wire]
            .coedges
            .iter()
            .filter(|c| c.edge == edge_id)
            .count()
            != 1
        {
            return Err(invalid(
                "Repeated same-loop edge uses need explicit seam ownership",
            ));
        }
        model.loops[wire]
            .coedges
            .splice(coedge..=coedge, replacements);
    }
    model.inherit_topology_ids(&[source]);
    if authored.is_some() {
        // Rebuilding identity must not discard unrelated tolerance/sew policy.
        let ids: std::collections::BTreeSet<_> = model
            .1
            .vertices
            .iter()
            .chain(&model.1.edges)
            .copied()
            .collect();
        for (&id, &tol) in &source.1.local_tolerances_mm {
            if ids.contains(&id) {
                model.1.local_tolerances_mm.insert(id, tol);
            }
        }
        for &id in &source.1.sewed_edges {
            if model.1.edges.contains(&id) {
                model.1.sewed_edges.insert(id);
            }
        }
        let stamp = tolerance.max(1e-10);
        model
            .1
            .local_tolerances_mm
            .insert(model.1.vertices[vertex], stamp);
        for &child in &child_edges {
            model
                .1
                .local_tolerances_mm
                .insert(model.1.edges[child], stamp);
            if source.1.sewed_edges.contains(&source.1.edges[edge_id]) {
                model.1.sewed_edges.insert(model.1.edges[child]);
            }
        }
    }
    let source_identity = source.1.edges[edge_id];
    let child_identities = child_edges.map(|i| model.1.edges[i]);
    model.1.lineage.push(crate::TopologyLineageRecord {
        operation: "split".into(),
        entity_kind: "edge".into(),
        parents: vec![source_identity],
        children: child_identities.to_vec(),
    });
    model.refresh_change_set(&[source]);
    if !model.persistent_naming_complete() {
        return Err(unresolved(
            "Partition did not retain complete persistent topology identities",
        ));
    }
    model.validate()?;
    Ok(Partition {
        model,
        source_edge: edge_id,
        source_identity,
        child_identities,
        child_edges,
        vertex,
        source_parameter: parameter,
        cells,
    })
}

#[cfg(test)]
mod tests;
