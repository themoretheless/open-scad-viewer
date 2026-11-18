use super::*;

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
    partition_impl(
        source,
        edge_id,
        parameter,
        cuts,
        Some(vertex),
        None,
        max_cells,
    )
}

/// Author matching world and face-local UV endpoints for independently certified
/// trace ports. Every UV proposal corresponds to one `cuts` entry. The complete
/// child curves and face lifts are still checked; no proximity-only snapping or
/// partial result is allowed.
pub fn partition_to_vertex_and_uv(
    source: &Model,
    edge_id: usize,
    parameter: f64,
    cuts: &[PcurveCut],
    vertex: [f64; 3],
    uv_vertices: &[[f64; 2]],
    max_cells: usize,
) -> Result<Partition> {
    if vertex.iter().any(|x| !x.is_finite()) {
        return Err(invalid("Choose a finite shared vertex proposal"));
    }
    if uv_vertices.len() != cuts.len() || uv_vertices.iter().flatten().any(|x| !x.is_finite()) {
        return Err(invalid(
            "Choose one finite UV vertex proposal for every pcurve cut",
        ));
    }
    partition_impl(
        source,
        edge_id,
        parameter,
        cuts,
        Some(vertex),
        Some(uv_vertices),
        max_cells,
    )
}
fn partition_impl(
    source: &Model,
    edge_id: usize,
    parameter: f64,
    cuts: &[PcurveCut],
    vertex: Option<[f64; 3]>,
    uv_vertices: Option<&[[f64; 2]]>,
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
        let cut_index = cuts
            .iter()
            .position(|c| c.wire == wire && c.coedge == coedge)
            .unwrap();
        let cut = cuts[cut_index];
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
        let authored_curve_cut = if original.reversed {
            a + b - parameter
        } else {
            parameter
        };
        if let Some(trim) = original.trim() {
            trim.validate()?;
            let inside = |value: f64, range: [f64; 2]| {
                range[0].min(range[1]) < value && value < range[0].max(range[1])
            };
            if trim
                .curve_parameter
                .iter()
                .any(|&value| value < a || value > b)
                || trim
                    .pcurve_parameter
                    .iter()
                    .any(|&value| value < p0 || value > p1)
                || !inside(authored_curve_cut, trim.curve_parameter)
                || !inside(cut.parameter, trim.pcurve_parameter)
            {
                return Err(invalid(
                    "Every authored trim must contain its cut strictly inside the original domains",
                ));
            }
            if crate::boundary_agreement::periodic_lifts(
                &source.faces[face].surface,
                &original.pcurve,
                trim.pcurve_parameter,
            )? != trim.periodic_lift
            {
                return Err(invalid(
                    "Original coedge periodic lifts do not match their authored UV endpoints",
                ));
            }
        }
        let mut pc = original.pcurve.split(cut.parameter)?;
        if let Some(uv_vertices) = uv_vertices {
            let last = pc[0].control_points.len() - 1;
            pc[0].control_points[last] = uv_vertices[cut_index].to_vec();
            pc[1].control_points[0] = uv_vertices[cut_index].to_vec();
        }
        let order = if original.reversed { [1, 0] } else { [0, 1] };
        let replacements: Vec<_> = pc
            .into_iter()
            .zip(order)
            .enumerate()
            .map(|(fragment, (pcurve, i))| {
                let child = Coedge::new(child_edges[i], original.reversed, pcurve.clone());
                let Some(trim) = original.trim() else {
                    return Ok(child);
                };
                let curve = if fragment == 0 {
                    [trim.curve_parameter[0], authored_curve_cut]
                } else {
                    [authored_curve_cut, trim.curve_parameter[1]]
                };
                let child_domain = children[i].domain();
                let shift = if original.reversed {
                    child_domain[0] + child_domain[1] - a - b
                } else {
                    0.
                };
                let curve_parameter = curve.map(|value| value + shift);
                let pcurve_parameter = if fragment == 0 {
                    [trim.pcurve_parameter[0], cut.parameter]
                } else {
                    [cut.parameter, trim.pcurve_parameter[1]]
                };
                if curve_parameter
                    .iter()
                    .any(|&value| value < child_domain[0] || value > child_domain[1])
                    || pcurve_parameter.iter().any(|&value| {
                        let domain = pcurve.domain();
                        value < domain[0] || value > domain[1]
                    })
                {
                    return Err(invalid(
                        "Transferred coedge trim leaves its actual child domain",
                    ));
                }
                let periodic_lift = crate::boundary_agreement::periodic_lifts(
                    &source.faces[face].surface,
                    &pcurve,
                    pcurve_parameter,
                )?;
                child.with_trim(brep_topology::CoedgeTrim {
                    curve_parameter,
                    pcurve_parameter,
                    periodic_lift,
                })
            })
            .collect::<Result<_>>()?;
        for c in &replacements {
            if let Some(trim) = c.trim() {
                check_trimmed_agreement(
                    &model.edges[c.edge].curve,
                    &c.pcurve,
                    &source.faces[face].surface,
                    c.reversed,
                    [
                        [trim.curve_parameter[0], trim.pcurve_parameter[0]],
                        [trim.curve_parameter[1], trim.pcurve_parameter[1]],
                    ],
                    source.tolerance_mm,
                    &mut cells,
                    max_cells,
                )?;
            } else {
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
