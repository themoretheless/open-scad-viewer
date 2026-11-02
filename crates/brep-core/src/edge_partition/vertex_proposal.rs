use super::*;
/// Split a nonperiodic 3D edge and all one or two incident face-local pcurves.
/// Independent natural pcurve cut parameters must be supplied for every use.
/// Children share one authored vertex, including both adjacent faces. Their
/// endpoints are assigned from one evaluation of the original 3D curve; every
/// resulting child is independently checked against the complete original
/// representation and each original support surface. No endpoint-only proof,
/// snapping authorization, exact-identity claim or solid certificate is implied.
/// Returns no model if any continuous proof or structural check fails.
pub fn partition(
    source: &Model,
    edge_id: usize,
    parameter: f64,
    cuts: &[PcurveCut],
    max_cells: usize,
) -> Result<Partition> {
    partition_impl(source, edge_id, parameter, cuts, None, max_cells)
}

/// A shared target vertex is a proposal, never snapping evidence. Both complete
/// children must still agree with the original curve and every original support.
/// This allows independently rounded incident boundaries to author one literal
/// shared vertex only when the complete continuous checks admit that change.
pub fn partition_to_vertex(
    source: &Model,
    edge_id: usize,
    parameter: f64,
    cuts: &[PcurveCut],
    vertex: [f64; 3],
    max_cells: usize,
) -> Result<Partition> {
    if vertex.iter().any(|x| !x.is_finite()) {
        return Err(invalid("Choose a finite shared vertex proposal"));
    }
    partition_impl(source, edge_id, parameter, cuts, Some(vertex), max_cells)
}
fn partition_impl(
    source: &Model,
    edge_id: usize,
    parameter: f64,
    cuts: &[PcurveCut],
    vertex: Option<[f64; 3]>,
    max_cells: usize,
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

    let position: [f64; 3] = edge
        .curve
        .evaluate(parameter)?
        .point
        .try_into()
        .map_err(|_| invalid("Edge must be three-dimensional"))?;
    let position = vertex.unwrap_or(position);
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
            child,
            &pcurve,
            &carrier,
            false,
            source.tolerance_mm,
            &mut cells,
            max_cells,
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
            .map(|(pcurve, i)| Coedge::new(child_edges[i], original.reversed, pcurve))
            .collect();
        for c in &replacements {
            check_agreement(
                &model.edges[c.edge].curve,
                &c.pcurve,
                &source.faces[face].surface,
                c.reversed,
                source.tolerance_mm,
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
