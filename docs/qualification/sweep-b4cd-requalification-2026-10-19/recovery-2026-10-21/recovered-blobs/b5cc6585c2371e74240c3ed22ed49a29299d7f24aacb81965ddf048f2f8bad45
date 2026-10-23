//! Bind recomputed contact-end roots to vertices authored on original body edges.
//! No nearest-point merge or caller-supplied proof authorizes an edit.
use crate::{
    Model, TopoId,
    contact_trim::{self, Restriction},
    edge_partition::{self, PcurveCut},
    fillet_contact::Contact,
};
use nurbs_core::{
    Error, Result,
    curve::Curve,
    curve_surface_agreement::{self, Status},
    surface::Surface,
    surface_g2_pcurves,
};
pub struct Binding {
    pub source: Model,
    pub restriction: Restriction,
    pub vertices: [usize; 2],
    pub source_edge_identities: [TopoId; 2],
    pub source_edge_parameters: [f64; 2],
    pub cells: usize,
    pub exact_work: u64,
}
fn invalid(s: &str) -> Error {
    Error::new("BREP_CONTACT_BIND_INPUT", s)
}
fn unproven(s: &str) -> Error {
    Error::new("BREP_CONTACT_BIND_UNPROVEN", s)
}
fn carrier(c: &Curve) -> Surface {
    Surface {
        degree_u: c.degree,
        degree_v: 1,
        knots_u: c.knots.clone(),
        knots_v: vec![0., 0., 1., 1.],
        control_points: c
            .control_points
            .iter()
            .map(|p| vec![p.clone(), p.clone()])
            .collect(),
        weights: c.weights.iter().map(|&w| vec![w, w]).collect(),
        periodic_u: false,
        periodic_v: false,
    }
}
fn root_distance(point: [f64; 3], bounds: [[f64; 2]; 3]) -> Result<f64> {
    use nurbs_core::interval_eval::Interval as I;
    let mut squared = I::point(0.);
    for k in 0..3 {
        let d = I::point(point[k]).sub(I::new(bounds[k][0], bounds[k][1])?)?;
        squared = squared.add(d.mul(d)?)?;
    }
    Ok(squared.hi.max(0.).sqrt().next_up())
}
/// Find original-edge parameter proposals by bounded subdivision of every
/// nondegenerate nonperiodic original edge. Interval boxes exclude remote
/// regions; fresh root-distance and both incident pcurve checks admit edits.
/// Multiple distinct owners refuse; incident endpoints sharing the same source
/// vertex count as one owner. No nearest-edge or exact root identity is claimed.
pub fn automatically_between_faces(
    source: &Model,
    contact: &Contact,
    supports: [&Surface; 2],
    cutters: [(&Model, usize, usize); 2],
    tolerance_uv: f64,
    max_cells: usize,
    max_exact: u64,
) -> Result<Binding> {
    use nurbs_core::interval_eval::{Interval as I, evaluate_interval};
    source.validate()?;
    if !source.persistent_naming_complete()
        || !(1..=100000).contains(&max_cells)
        || max_exact > 1000000
        || source.edges.len() > 4096
    {
        return Err(invalid(
            "Automatic root binding needs owned bounded source topology",
        ));
    }
    let mut restriction = contact_trim::between_faces(
        contact,
        supports,
        cutters,
        source.tolerance_mm,
        tolerance_uv,
        max_cells,
        max_exact,
    )?;
    let mut proposals = [(0usize, 0.); 2];
    // Leave most of the admission tolerance for the two saved support lifts.
    let search_tolerance = source.tolerance_mm / 64.;
    for k in 0..2 {
        let report = &restriction.coverage[k];
        let bounds = report.roots[report.eligible[cutters[k].2]]
            .root
            .position_bounds;
        let mut found: Option<(usize, f64, Option<usize>)> = None;
        for (edge, e) in source.edges.iter().enumerate() {
            if e.degenerate {
                continue;
            }
            // Unsupported charts cannot be silently omitted from ownership search.
            if e.curve.periodic {
                return Err(unproven(
                    "Periodic source-edge ownership remains unresolved",
                ));
            }
            let domain = e.curve.domain();
            let mut at_vertex = false;
            for end in 0..2 {
                if restriction.cells == max_cells {
                    return Err(unproven("Automatic endpoint search work exhausted"));
                }
                restriction.cells += 1;
                let vertex = e.vertices[end];
                if root_distance(source.vertices[vertex].point, bounds)? <= search_tolerance {
                    if found.is_some_and(|(_, _, v)| v != Some(vertex)) {
                        return Err(unproven("Contact root has distinct eligible source owners"));
                    }
                    found = Some((edge, domain[end], Some(vertex)));
                    at_vertex = true;
                }
            }
            if at_vertex {
                continue;
            }
            let mut pending = vec![domain];
            while let Some(range) = pending.pop() {
                if restriction.cells == max_cells {
                    return Err(unproven("Automatic original-edge search work exhausted"));
                }
                restriction.cells += 1;
                let b = evaluate_interval(&e.curve, I::new(range[0], range[1])?)?;
                if (0..3).any(|j| {
                    b[j].hi < (bounds[j][0] - search_tolerance).next_down()
                        || b[j].lo > (bounds[j][1] + search_tolerance).next_up()
                }) {
                    continue;
                }
                let t = range[0] * 0.5 + range[1] * 0.5;
                let p = e.curve.evaluate(t)?.point;
                if domain[0] < t
                    && t < domain[1]
                    && root_distance([p[0], p[1], p[2]], bounds)? <= search_tolerance
                {
                    if found.is_some() {
                        return Err(unproven(
                            "Contact root has multiple eligible original edges",
                        ));
                    }
                    found = Some((edge, t, None));
                    break;
                }
                if !(range[0] < t && t < range[1]) {
                    return Err(unproven("Original-edge root ownership cannot be separated"));
                }
                pending.push([t, range[1]]);
                pending.push([range[0], t]);
            }
        }
        let (edge, t, _) =
            found.ok_or_else(|| unproven("No original edge admits the contact root"))?;
        proposals[k] = (edge, t);
    }
    if proposals[0] == proposals[1] {
        return Err(unproven("Distinct contact roots selected one cut"));
    }
    bind_restriction(
        source,
        contact,
        supports,
        cutters,
        proposals,
        restriction,
        max_cells,
        max_exact,
    )
}
/// Body-edge indices and original natural parameters are proposals. Fresh roots,
/// root-position bounds, all incident pcurve splits, final original-parent
/// agreement and both returned contact lifts are mandatory. Both ends are one
/// atomic transaction. Equal-index edges are processed in descending parameter
/// order and every final child is checked against the ORIGINAL source curve.
/// Endpoint parameters reuse the exact existing source vertex after root-bound
/// and full incident-edge lift checks; they never create a duplicate vertex.
pub fn between_faces(
    source: &Model,
    contact: &Contact,
    supports: [&Surface; 2],
    cutters: [(&Model, usize, usize); 2],
    edge_parameters: [(usize, f64); 2],
    tolerance_uv: f64,
    max_cells: usize,
    max_exact: u64,
) -> Result<Binding> {
    source.validate()?;
    if !source.persistent_naming_complete()
        || !(1..=100000).contains(&max_cells)
        || max_exact > 1000000
    {
        return Err(invalid(
            "Need complete source identities and bounded shared work",
        ));
    }
    for &(edge, t) in &edge_parameters {
        let e = source
            .edges
            .get(edge)
            .ok_or_else(|| invalid("Original source edge is missing"))?;
        let [a, b] = e.curve.domain();
        if e.degenerate || e.curve.periodic || !t.is_finite() || !(a <= t && t <= b) {
            return Err(invalid(
                "Binding requires nondegenerate source edges and in-domain natural parameters",
            ));
        }
    }
    if edge_parameters[0] == edge_parameters[1] {
        return Err(invalid("Distinct end roots cannot share one proposed cut"));
    }
    let tolerance = source.tolerance_mm;
    let restriction = contact_trim::between_faces(
        contact,
        supports,
        cutters,
        tolerance,
        tolerance_uv,
        max_cells,
        max_exact,
    )?;
    bind_restriction(
        source,
        contact,
        supports,
        cutters,
        edge_parameters,
        restriction,
        max_cells,
        max_exact,
    )
}
fn bind_restriction(
    source: &Model,
    contact: &Contact,
    supports: [&Surface; 2],
    cutters: [(&Model, usize, usize); 2],
    edge_parameters: [(usize, f64); 2],
    mut restriction: Restriction,
    max_cells: usize,
    max_exact: u64,
) -> Result<Binding> {
    let tolerance = source.tolerance_mm;
    let mut cells = restriction.cells;
    let mut exact_work = restriction.exact_work;
    let bounds = std::array::from_fn::<_, 2, _>(|k| {
        let report = &restriction.coverage[k];
        let index = report.eligible[cutters[k].2];
        report.roots[index].root.position_bounds
    });
    for k in 0..2 {
        let (edge, t) = edge_parameters[k];
        let p = source.edges[edge].curve.evaluate(t)?.point;
        if root_distance([p[0], p[1], p[2]], bounds[k])? > tolerance {
            return Err(unproven(
                "Proposed original-edge point is not bounded to the recomputed contact root",
            ));
        }
    }
    let source_edge_identities = edge_parameters.map(|(edge, _)| source.1.edges[edge]);
    let mut order = [0usize, 1];
    order.sort_by(|&a, &b| {
        edge_parameters[a]
            .0
            .cmp(&edge_parameters[b].0)
            .then_with(|| edge_parameters[b].1.total_cmp(&edge_parameters[a].1))
    });
    let mut model = source.clone();
    let mut vertices = [0usize; 2];
    let mut children = std::collections::BTreeMap::<usize, Vec<usize>>::new();
    for k in order {
        if cells == max_cells {
            return Err(unproven("Shared root binding split work exhausted"));
        }
        let (edge, t) = edge_parameters[k];
        let [a, b] = model.edges[edge].curve.domain();
        if t == a || t == b {
            let vertex = model.edges[edge].vertices[usize::from(t == b)];
            let point = model.vertices[vertex].point;
            let error = root_distance(point, bounds[k])?;
            if error > tolerance {
                return Err(unproven(
                    "Existing source vertex escapes contact-root bounds",
                ));
            }
            // Verify every original incident edge/use, including edges not
            // named by the caller. Reusing topology cannot bypass seam checks.
            let mut uses = 0;
            for face in &model.faces {
                for wire in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
                    for c in &model.loops[wire].coedges {
                        let e = &model.edges[c.edge];
                        if !e.vertices.contains(&vertex) {
                            continue;
                        }
                        let endpoint = e.curve.domain()[usize::from(e.vertices[1] == vertex)];
                        let ep = e.curve.evaluate(endpoint)?.point;
                        if root_distance([ep[0], ep[1], ep[2]], point.map(|x| [x, x]))? > tolerance
                        {
                            return Err(unproven(
                                "Incident edge endpoint disagrees with owned vertex",
                            ));
                        }
                        if cells == max_cells {
                            return Err(unproven("Existing-vertex incident lift work exhausted"));
                        }
                        let proof = curve_surface_agreement::verify(
                            &e.curve,
                            &c.pcurve,
                            &face.surface,
                            c.reversed,
                            tolerance,
                            max_cells - cells,
                        )?;
                        cells += proof.cells;
                        if proof.status != Status::WithinTolerance {
                            return Err(unproven("Existing-vertex incident edge lift is unproven"));
                        }
                        uses += 1;
                    }
                }
            }
            if uses == 0 {
                return Err(unproven("Existing source vertex has no incident face"));
            }
            vertices[k] = vertex;
            restriction.endpoints[k] = point;
            restriction.root_position_error_upper[k] = error;
            continue;
        }
        let fraction = (t - a) / (b - a);
        let cuts = model
            .loops
            .iter()
            .enumerate()
            .flat_map(|(wire, l)| {
                l.coedges
                    .iter()
                    .enumerate()
                    .filter(move |(_, c)| c.edge == edge)
                    .map(move |(coedge, c)| {
                        let [a, b] = c.pcurve.domain();
                        let f = if c.reversed { 1. - fraction } else { fraction };
                        PcurveCut {
                            wire,
                            coedge,
                            parameter: a + (b - a) * f,
                        }
                    })
            })
            .collect::<Vec<_>>();
        let split = edge_partition::partition(&model, edge, t, &cuts, max_cells - cells)?;
        cells += split.cells;
        vertices[k] = split.vertex;
        children
            .entry(edge)
            .or_insert_with(|| vec![edge])
            .push(split.child_edges[1]);
        model = split.model;
        let point = model.vertices[vertices[k]].point;
        let error = root_distance(point, bounds[k])?;
        if error > tolerance {
            return Err(unproven(
                "Actual shared vertex escaped its original contact-root bound",
            ));
        }
        restriction.endpoints[k] = point;
        restriction.root_position_error_upper[k] = error;
    }
    if vertices[0] == vertices[1] {
        return Err(unproven(
            "Distinct roots cannot reuse the same owned vertex",
        ));
    }
    for (parent, children) in children {
        let original = carrier(&source.edges[parent].curve);
        for child in children {
            if cells == max_cells {
                return Err(unproven("Original body-edge agreement work exhausted"));
            }
            let c = &model.edges[child].curve;
            let [a, b] = c.domain();
            let pc = Curve::from_polyline(vec![vec![a, 0.], vec![b, 0.]])?;
            let proof = curve_surface_agreement::verify(
                c,
                &pc,
                &original,
                false,
                tolerance,
                max_cells - cells,
            )?;
            cells += proof.cells;
            if proof.status != Status::WithinTolerance {
                return Err(unproven(
                    "Final body edge child disagrees with its immutable original",
                ));
            }
        }
    }
    let last = restriction.contact.curve.control_points.len() - 1;
    restriction.contact.curve.control_points[0] = restriction.endpoints[0].to_vec();
    restriction.contact.curve.control_points[last] = restriction.endpoints[1].to_vec();
    if cells == max_cells {
        return Err(unproven("No contact original-parent work remains"));
    }
    let original = carrier(&contact.curve);
    let t = restriction.parameters;
    let pc = Curve::from_polyline(vec![vec![t[0], 0.], vec![t[1], 0.]])?;
    let proof = curve_surface_agreement::verify(
        &restriction.contact.curve,
        &pc,
        &original,
        false,
        tolerance,
        max_cells - cells,
    )?;
    cells += proof.cells;
    if proof.status != Status::WithinTolerance {
        return Err(unproven(
            "Vertex-bound contact disagrees with its immutable original",
        ));
    }
    restriction.original_agreement = proof;
    let positions = surface_g2_pcurves::verify_positions(
        &restriction.contact.curve,
        [
            &restriction.contact.support_pcurve,
            &restriction.contact.fillet_pcurve,
        ],
        supports,
        [false, false],
        tolerance,
        max_cells - cells,
        max_exact - exact_work,
    )?;
    cells += positions.cells;
    exact_work += positions.exact_work;
    if !positions.proven {
        return Err(unproven("Vertex-bound contact lifts are unproven"));
    }
    restriction.contact.positions = positions;
    restriction.cells = cells;
    restriction.exact_work = exact_work;
    model.validate()?;
    Ok(Binding {
        source: model,
        restriction,
        vertices,
        source_edge_identities,
        source_edge_parameters: edge_parameters.map(|(_, t)| t),
        cells,
        exact_work,
    })
}
