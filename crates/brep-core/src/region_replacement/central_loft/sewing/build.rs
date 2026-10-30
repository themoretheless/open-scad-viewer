use super::*;
use topology::{edge, endpoint, vertex};
fn line(a: [f64; 3], b: [f64; 3]) -> Result<Curve> {
    Curve::from_polyline(vec![a.to_vec(), b.to_vec()])
}
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
    let bounded = control_fit::prepare(c)?;
    let exterior = c.inspect_surface_exterior_jets(&bounded.surface, 6000, 100000)?;
    let joins = c.inspect_surface_internal_span_joins(&bounded.surface, 6000)?;
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
    let poles = c.inspect_surface_pole_limits(&bounded.surface, 1. / 64., 2000)?;
    cells += 2 * poles.iter().map(|p| p.report.cells).sum::<usize>();
    let mut exact = exterior.iter().map(|r| r.positions.exact_work).sum::<u64>();
    require(
        cells <= limits.cells && exact <= limits.exact_work,
        "Corner G2 exhausted shared sewing work",
    )?;
    let source = c.body.source();
    let corner = c.strips.plan.vertex;
    let clocks = reclock::prepare(c, &bounded.surface)?;
    let mut m = source.clone();
    for face in &mut m.faces {
        face.surface = reclock::plane(&face.surface)?;
    }
    m.bodies.clear();
    m.shells.clear();
    let mut extra: Vec<Vec<usize>> = vec![Vec::new(); source.faces.len()];
    let mut strip_faces = [0; 3];
    let mut cap_faces = [0; 3];
    let mut near_edges = [0; 3];
    let mut contact_edges = [[0; 2]; 3];
    let mut strip_fits = Vec::new();
    for (j, strip) in c.strips.strips.iter().enumerate() {
        let selected = &source.edges[strip.source_edge];
        let other = *selected
            .vertices
            .iter()
            .find(|&&v| v != corner)
            .ok_or_else(|| refuse("Missing retained source endpoint"))?;
        require(
            selected.curve.degree == 1 && selected.curve.weights[0] == selected.curve.weights[1],
            "Initial corner sewing requires straight affine stock edges",
        )?;
        let axes: Vec<_> = (0..3)
            .filter(|&k| source.vertices[corner].point[k] != source.vertices[other].point[k])
            .collect();
        require(axes.len() == 1, "Corner source edge must be axis aligned")?;
        let axis = axes[0];
        let far_cut = &strip.cutters[1].faces[strip.cutter_faces[1]].surface;
        let fixed = far_cut.control_points[0][0][axis];
        require(
            far_cut
                .control_points
                .iter()
                .flatten()
                .all(|p| p[axis] == fixed)
                && c.stock[axis][0] < fixed
                && fixed < c.stock[axis][1],
            "Actual far cutter must retain one stock-axis section",
        )?;
        let mut sharp = source.vertices[corner].point;
        sharp[axis] = fixed;
        let sharp_id = vertex(&mut m, sharp);
        let mut ends = selected.vertices;
        ends[usize::from(ends[1] == corner)] = sharp_id;
        m.edges[strip.source_edge] = crate::Edge {
            curve: line(m.vertices[ends[0]].point, m.vertices[ends[1]].point)?,
            vertices: ends,
            degenerate: false,
        };
        let near = usize::from(strip.end_order[0] != 0);
        let original = &strip.patch.ends[near].curve;
        let matches = c
            .curves
            .iter()
            .enumerate()
            .filter_map(|(i, curve)| {
                if i % 2 != 0 {
                    return None;
                }
                if curve == original || curve.reverse().ok().as_ref() == Some(original) {
                    Some(i)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        require(
            matches.len() == 1,
            "Structured strip lost its original central section",
        )?;
        let fit = fitted::prepare(
            clocks.curves[matches[0]].clone(),
            original,
            (c.pcurves[matches[0]].control_points[0][0]
                > c.pcurves[matches[0]].control_points.last().unwrap()[0])
                ^ (c.curves[matches[0]] != *original),
            axis,
            fixed,
            m.tolerance_mm,
        )?;
        require(cells < limits.cells, "No full-strip deviation work remains")?;
        let original_near_cut = &strip.cutters[0].faces[strip.cutter_faces[0]].surface;
        let proof = strip_fit::inspect(
            c,
            &clocks,
            matches[0],
            j,
            axis,
            original_near_cut.control_points[0][0][axis],
            fixed,
            limits.cells - cells,
        )?;
        cells += proof.cells;
        strip_fits.push(proof);
        let patch = &fit.model;
        let local_vertices = patch
            .vertices
            .iter()
            .map(|v| vertex(&mut m, v.point))
            .collect::<Vec<_>>();
        let mut local_edges = Vec::new();
        for e in &patch.edges {
            local_edges.push(edge(
                &mut m,
                e.curve.clone(),
                e.vertices.map(|v| local_vertices[v]),
                e.degenerate,
            ));
        }
        contact_edges[j] = fit.contacts.map(|e| local_edges[e]);
        let mut wire = patch.loops[patch.faces[0].outer].clone();
        for use_ in &mut wire.coedges {
            use_.edge = local_edges[use_.edge];
        }
        let wire_id = m.loops.len();
        m.loops.push(wire);
        strip_faces[j] = m.faces.len();
        m.faces.push(crate::Face {
            surface: patch.faces[0].surface.clone(),
            outer: wire_id,
            holes: vec![],
        });
        for side in 0..2 {
            extra[strip.support_faces[side]].push(local_edges[fit.contacts[side]]);
        }
        near_edges[j] = local_edges[fit.near];
        let far_edge = local_edges[fit.far];
        let e = &m.edges[far_edge];
        let ends = e.vertices;
        let mut cap_edges = vec![far_edge];
        for side in 0..2 {
            let v = ends[side];
            let boundary = line(m.vertices[v].point, sharp)?;
            let id = edge(&mut m, boundary, [v, sharp_id], false);
            extra[strip.support_faces[side]].push(id);
            cap_edges.push(id);
        }
        let surface = planar::cap(c.stock, axis, fixed);
        let uses = cap_edges
            .into_iter()
            .map(|e| Ok((e, planar::project(&m.edges[e].curve, &surface)?)))
            .collect::<Result<Vec<_>>>()?;
        let mut wire = topology::loop_from_edges(&m, uses)?;
        topology::positive_loop(&mut wire, c.options.tolerance_uv, &mut cells, limits)?;
        let outer = m.loops.len();
        m.loops.push(wire);
        cap_faces[j] = m.faces.len();
        m.faces.push(crate::Face {
            surface,
            outer,
            holes: vec![],
        });
    }
    let mut central = crate::Loop {
        coedges: Vec::new(),
    };
    for i in 0..6 {
        let curve = &clocks.curves[i];
        let (id, reverse) = if i % 2 == 0 {
            let matches = near_edges
                .iter()
                .filter_map(|&id| {
                    let actual = &m.edges[id].curve;
                    if actual == curve {
                        Some((id, false))
                    } else if actual.reverse().ok().as_ref() == Some(curve) {
                        Some((id, true))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            require(
                matches.len() == 1,
                "Central seam lost its literal original strip curve",
            )?;
            matches[0]
        } else {
            let mut vertices = [0; 2];
            for end in 0..2 {
                let p = endpoint(curve, end);
                let matches = near_edges
                    .iter()
                    .flat_map(|&e| m.edges[e].vertices)
                    .collect::<std::collections::BTreeSet<_>>()
                    .into_iter()
                    .filter(|&v| m.vertices[v].point == p)
                    .collect::<Vec<_>>();
                require(
                    matches.len() == 1,
                    "Central connector endpoint is not uniquely source-owned",
                )?;
                vertices[end] = matches[0];
            }
            let id = edge(&mut m, curve.clone(), vertices, false);
            let face = source
                .faces
                .iter()
                .enumerate()
                .filter(|(_, f)| f.surface == c.originals[i].0)
                .map(|(f, _)| f)
                .collect::<Vec<_>>();
            require(
                face.len() == 1,
                "Central connector lost its original stock plane",
            )?;
            extra[face[0]].push(id);
            (id, false)
        };
        let traversal_reverse = reverse
            ^ (clocks.pcurves[i].control_points[0][0]
                > clocks.pcurves[i].control_points.last().unwrap()[0]);
        central.coedges.push(crate::Coedge::new(
            id,
            traversal_reverse,
            clocks.pcurves[i].clone(),
        ));
        if i == 2 || i == 5 {
            let v = m.edges[id].vertices[usize::from(!traversal_reverse)];
            let p = m.vertices[v].point;
            let deg = edge(&mut m, line(p, p)?, [v, v], true);
            let a = clocks.pcurves[i].control_points.last().unwrap().clone();
            let b = clocks.pcurves[(i + 1) % 6].control_points[0].clone();
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
        surface: clocks.surface.clone(),
        outer,
        holes: vec![],
    });
    for f in 0..source.faces.len() {
        let support = m.faces[f].surface.clone();
        let surface = &support;
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
    let map = topology::compact_vertices(&mut m);
    let source_vertices = map[..source.vertices.len()].to_vec();
    let seed = source.shells[0]
        .faces
        .iter()
        .find(|u| u.face == 0)
        .unwrap()
        .reversed;
    topology::orient(&mut m, seed)?;
    m.inherit_topology_ids(&[source]);
    m.validate()?;
    require(
        cells < limits.cells && exact <= limits.exact_work,
        "No corner closure work remains",
    )?;
    let closure =
        crate::watertight::inspect_with_exact(&m, limits.cells - cells, limits.exact_work - exact)?;
    cells += closure.closure.agreement.cells;
    exact += closure.exact.work;
    require(
        closure.closure.status == crate::watertight::Closure::WithinTolerance,
        "Actual sewn corner boundary remains open or mismatched",
    )?;
    m.shells[0].closed = true;
    m.bodies = vec![crate::Body {
        outer_shell: 0,
        inner_shells: vec![],
    }];
    m.inherit_topology_ids(&[source]);
    m.validate()?;
    require(cells < limits.cells, "No final corner trim work remains")?;
    let allowance = (limits.cells - cells) / 2;
    require(allowance > 0, "No final corner trim work remains")?;
    let trims = crate::face_domain::audit_trim_regions(
        &m,
        c.options.tolerance_uv,
        limits.trim_pairs,
        allowance,
        allowance.min(limits.domain_cells),
    )?;
    cells += trims.cells + trims.domain_cells;
    require(
        cells <= limits.cells,
        "Final corner trim exceeded shared sewing work",
    )?;
    let mut final_jets = final_jets::inspect(
        c,
        &m,
        central_face,
        strip_faces,
        &mut cells,
        &mut exact,
        limits,
    )?;
    let (stock_jets, radii) = strip_jets::inspect(
        c,
        &m,
        strip_faces,
        contact_edges,
        &mut cells,
        &mut exact,
        limits,
    )?;
    final_jets.extend(stock_jets);
    Ok(SewnCornerCandidate {
        model: m,
        strip_fits,
        final_jets,
        radii,
        stock_bounds: c.stock,
        stock_volume: c.body.body().volume(),
        surface_fit_upper_mm: bounded.displacement_upper_mm,
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
