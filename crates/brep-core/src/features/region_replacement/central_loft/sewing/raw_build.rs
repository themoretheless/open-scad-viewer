use super::build::{
    cap_strip, central_edge, close_and_audit, insert_patch, line, retarget_source_edge,
};
use super::*;
use topology::edge;
pub(super) fn assemble(
    c: &CentralLoftCandidate<'_, '_, '_>,
    limits: CornerSewLimits,
) -> Result<SewnCornerCandidate> {
    require(
        (100..=100000).contains(&limits.cells)
            && (1..=1000000).contains(&limits.exact_work)
            && (1..=100000).contains(&limits.trim_pairs)
            && (1..=1000000).contains(&limits.domain_cells),
        "Bound corner sewing work",
    )?;
    c.reverify()?;
    let exterior = c.inspect_exterior_jets(6000, 100000)?;
    let joins = c.inspect_internal_span_joins(6000)?;
    require(
        exterior.iter().all(|r| {
            r.decision == nurbs_core::surface_join::Decision::WithinTolerance
                && r.regularity_proven
                && r.pending == 0
        }) && joins.iter().all(|(_, r)| {
            r.decision == nurbs_core::surface_join::Decision::WithinTolerance
                && r.regularity_proven
                && r.pending == 0
        }),
        "Original central G2 remains unproven",
    )?;
    let mut cells = exterior.iter().map(|r| r.cells).sum::<usize>()
        + joins.iter().map(|(_, r)| r.cells).sum::<usize>();
    // Exterior inspection also recomputes the native two-pole proofs.
    let poles = c.inspect_pole_limits(1. / 64., 2000)?;
    cells += 2 * poles.iter().map(|p| p.report.cells).sum::<usize>();
    let mut exact = exterior.iter().map(|r| r.positions.exact_work).sum::<u64>();
    require(
        cells <= limits.cells && exact <= limits.exact_work,
        "Corner G2 exhausted shared sewing work",
    )?;
    let source = c.body.source();
    let corner = c.strips.plan.vertex;
    let mut m = source.clone();
    m.bodies.clear();
    m.shells.clear();
    let mut extra: Vec<Vec<usize>> = vec![Vec::new(); source.faces.len()];
    let mut strip_faces = [0; 3];
    let mut cap_faces = [0; 3];
    let mut near_edges = [0; 3];
    for (j, strip) in c.strips.strips.iter().enumerate() {
        let section = retarget_source_edge(c, &mut m, corner, strip)?;
        let patch = &strip.patch.model;
        let (local_edges, face) = insert_patch(&mut m, patch);
        strip_faces[j] = face;
        for side in 0..2 {
            extra[strip.support_faces[side]]
                .push(local_edges[strip.patch.edges[if side == 0 { 3 } else { 1 }]]);
        }
        let near = usize::from(strip.end_order[0] != 0);
        let far = 1 - near;
        near_edges[j] = local_edges[strip.patch.edges[if near == 0 { 0 } else { 2 }]];
        let far_edge = local_edges[strip.patch.edges[if far == 0 { 0 } else { 2 }]];
        cap_faces[j] = cap_strip(
            c,
            &mut m,
            &mut extra,
            strip.support_faces,
            far_edge,
            &section,
            (&mut cells, limits),
        )?;
    }
    let mut central = crate::Loop {
        coedges: Vec::new(),
    };
    for i in 0..6 {
        let curve = &c.curves[i];
        let (id, reverse) = central_edge(c, &mut m, &mut extra, &near_edges, i, curve)?;
        central
            .coedges
            .push(crate::Coedge::new(id, reverse, c.pcurves[i].clone()));
        if i == 2 || i == 5 {
            let v = m.edges[id].vertices[usize::from(!reverse)];
            let p = m.vertices[v].point;
            let deg = edge(&mut m, line(p, p)?, [v, v], true);
            let a = c.pcurves[i].control_points.last().unwrap().clone();
            let b = c.pcurves[(i + 1) % 6].control_points[0].clone();
            central.coedges.push(crate::Coedge::new(
                deg,
                false,
                Curve::from_polyline(vec![a, b])?,
            ));
        }
    }
    topology::positive_loop(&mut central, c.options.tolerance_uv, &mut cells, limits)?;
    let central_face = m.faces.len();
    let outer = m.loops.len();
    m.loops.push(central);
    m.faces.push(crate::Face {
        surface: c.surface.clone(),
        outer,
        holes: vec![],
    });
    for f in 0..source.faces.len() {
        if extra[f].is_empty() {
            continue;
        }
        let surface = &source.faces[f].surface;
        let ids = source.loops[source.faces[f].outer]
            .coedges
            .iter()
            .map(|u| u.edge)
            .chain(extra[f].iter().copied())
            .collect::<Vec<_>>();
        let uses = ids
            .into_iter()
            .map(|id| Ok((id, planar::project(&m.edges[id].curve, surface)?)))
            .collect::<Result<Vec<_>>>()?;
        let mut wire = topology::loop_from_edges(&m, uses)?;
        topology::positive_loop(&mut wire, c.options.tolerance_uv, &mut cells, limits)?;
        let outer = m.faces[f].outer;
        m.loops[outer] = wire;
    }
    let (source_vertices, closure, trims) =
        close_and_audit(c, &mut m, &mut cells, &mut exact, limits)?;
    Ok(SewnCornerCandidate {
        model: m,
        strip_fits: vec![],
        final_jets: vec![],
        radii: vec![],
        stock_bounds: c.stock,
        stock_volume: c.body.body().volume(),
        surface_fit_upper_mm: 0.,
        corner_point: source.vertices[corner].point,
        closure,
        trims,
        central_face,
        strip_faces,
        cap_faces,
        source_vertices,
        cells,
        exact_work: exact,
    })
}
