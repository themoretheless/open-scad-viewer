use super::*;
use exact::{float, q};
pub(super) fn verify(
    models: [&Model; 2],
    g: &QualifiedNurbsIntersectionGraph,
    p: &NurbsFacePartition,
    side: usize,
) -> Result<()> {
    let source = models[side];
    let m = &p.model;
    let graph = g.certificate();
    m.validate()?;
    if m.faces.len() > 256
        || m.edges.len() > 1024
        || p.face_sources.len() != m.faces.len()
        || p.edge_sources.len() != m.edges.len()
        || !m.persistent_naming_complete()
    {
        return Err(fail(
            "bounded complete partition correspondence is required",
        ));
    }
    for edge in &m.edges {
        if edge.degenerate
            || !exact::clamped(&edge.curve)
            || edge.curve.control_points[0] != m.vertices[edge.vertices[0]].point
            || edge.curve.control_points.last().unwrap() != &m.vertices[edge.vertices[1]].point
        {
            return Err(fail(
                "saved partition edge endpoints are not literal exact vertices",
            ));
        }
    }
    let mut exterior = BTreeMap::<(usize, usize), Vec<[f64; 2]>>::new();
    let mut cuts = BTreeMap::<usize, Vec<(usize, &Coedge)>>::new();
    let mut seen = BTreeSet::new();
    let mut used_edges = BTreeSet::new();
    for (f, &original) in p.face_sources.iter().enumerate() {
        let old = source
            .faces
            .get(original)
            .ok_or_else(|| fail("partition references a missing source face"))?;
        if m.faces[f].surface != old.surface
            || exact::owner(m, f)? != exact::owner(source, original)?
        {
            return Err(fail("partition changed an original carrier or orientation"));
        }
        exact::profile(m, f)?;
        seen.insert(original);
        for w in std::iter::once(m.faces[f].outer).chain(m.faces[f].holes.iter().copied()) {
            let ring = &m.loops[w].coedges;
            for (i, c) in ring.iter().enumerate() {
                let next = &ring[(i + 1) % ring.len()];
                if !exact::clamped(&c.pcurve)
                    || c.pcurve.control_points.last() != Some(&next.pcurve.control_points[0])
                {
                    return Err(fail("region boundary needs literal exact UV closure"));
                }
            }
        }
        for c in exact::uses(m, f) {
            used_edges.insert(c.edge);
            if c.trim().is_some() {
                return Err(fail("partition needs full untrimmed saved uses"));
            }
            let s = &old.surface;
            let natural = [
                [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
                [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
            ];
            if c.pcurve.weights.iter().any(|&w| w <= 0.)
                || c.pcurve
                    .control_points
                    .iter()
                    .any(|p| (0..2).any(|k| p[k] < natural[k][0] || p[k] > natural[k][1]))
            {
                return Err(fail(
                    "saved positive pcurve hull leaves the original natural domain",
                ));
            }
            exact::lift(&m.edges[c.edge].curve, &c.pcurve, &old.surface, c.reversed)?;
            match p.edge_sources[c.edge] {
                RegionEdgeOrigin::Original { edge, range } => {
                    let original_edge = source
                        .edges
                        .get(edge)
                        .ok_or_else(|| fail("missing original edge"))?;
                    let d = original_edge.curve.domain();
                    if !(d[0] <= range[0] && range[0] < range[1] && range[1] <= d[1]) {
                        return Err(fail("original edge restriction leaves its complete domain"));
                    }
                    exact::restriction(&original_edge.curve, &m.edges[c.edge].curve, range)?;
                    let uses = exact::uses(source, original)
                        .filter(|a| a.edge == edge)
                        .collect::<Vec<_>>();
                    if uses.len() != 1 || uses[0].reversed != c.reversed {
                        return Err(fail("original oriented rim correspondence is not unique"));
                    }
                    let a = uses[0];
                    let pc = a.pcurve.domain();
                    let map = |t: f64| -> Result<f64> {
                        let u = (q(t) - q(d[0])) / (q(d[1]) - q(d[0]));
                        float(
                            &(q(pc[0])
                                + (if a.reversed { Q::one() - u } else { u })
                                    * (q(pc[1]) - q(pc[0]))),
                        )
                    };
                    let r = if c.reversed {
                        [map(range[1])?, map(range[0])?]
                    } else {
                        [map(range[0])?, map(range[1])?]
                    };
                    exact::restriction(&a.pcurve, &c.pcurve, r)?;
                    exterior.entry((original, edge)).or_default().push(range);
                }
                RegionEdgeOrigin::Intersection { arc, .. } => {
                    let a = graph
                        .arcs
                        .get(arc)
                        .ok_or_else(|| fail("partition cut has no original graph arc"))?;
                    if graph.pairs[a.pair].faces[side] != original {
                        return Err(fail("graph arc belongs to another original carrier"));
                    }
                    cuts.entry(c.edge).or_default().push((original, c));
                }
            }
        }
    }
    if used_edges.len() != m.edges.len() {
        return Err(fail(
            "partition has unused edges outside the proved arrangement",
        ));
    }
    if seen.len() != source.faces.len() {
        return Err(fail("partition omitted an entire original face"));
    }
    for f in 0..source.faces.len() {
        exact::profile(source, f)?;
        for c in exact::uses(source, f) {
            let mut pieces = exterior
                .remove(&(f, c.edge))
                .ok_or_else(|| fail("partition omitted part of an original rim"))?;
            pieces.sort_by(|a, b| a[0].total_cmp(&b[0]));
            let d = source.edges[c.edge].curve.domain();
            if pieces[0][0] != d[0]
                || pieces.last().unwrap()[1] != d[1]
                || pieces.windows(2).any(|w| w[0][1] != w[1][0])
            {
                return Err(fail(
                    "original rim partition has a gap or overlapping multiplicity",
                ));
            }
        }
    }
    let mut arcs = BTreeSet::new();
    for (edge, uses) in cuts {
        if uses.len() != 2 || uses[0].0 != uses[1].0 || uses[0].1.reversed == uses[1].1.reversed {
            return Err(fail(
                "intersection cut does not have two opposite same-carrier sector uses",
            ));
        }
        exact::same(&uses[0].1.pcurve, &uses[1].1.pcurve, true)?;
        let RegionEdgeOrigin::Intersection { arc, reversed } = p.edge_sources[edge] else {
            unreachable!()
        };
        if !arcs.insert(arc) {
            return Err(fail("one physical graph arc is represented more than once"));
        }
        bind(g, &p.model, edge, uses[0].1, side, arc, reversed)?;
    }
    if arcs.len() != graph.arcs.len() {
        return Err(fail(
            "not every COMPLETE original intersection arc partitions this operand",
        ));
    }
    if !exterior.is_empty() {
        return Err(fail("partition introduced a foreign original rim"));
    }
    // Exact oriented UV chains now sum to the original rim. Every audited
    // region has indicator winding 0/1; their sum equals the original 0/1
    // winding at ALL off-boundary points. This proves coverage and nonoverlap,
    // without a finite UV sample grid or an assumed seed-side classification.
    Ok(())
}

fn bind(
    g: &QualifiedNurbsIntersectionGraph,
    m: &Model,
    e: usize,
    c: &Coedge,
    side: usize,
    arc: usize,
    reverse: bool,
) -> Result<()> {
    let graph = g.certificate();
    let a = &graph.arcs[arc];
    let pair = &graph.pairs[a.pair];
    let piece = &pair.pieces[a.piece];
    let branch = &pair.branches[piece.branch];
    let free = match branch.free_axis {
        nurbs_core::ss_intersection::FiberAxis::FirstU => 0,
        nurbs_core::ss_intersection::FiberAxis::FirstV => 1,
        nurbs_core::ss_intersection::FiberAxis::SecondU => 2,
        nurbs_core::ss_intersection::FiberAxis::SecondV => 3,
    };
    // The free clock must be represented on this side for a literal monotone
    // Bernstein proof. The other side is checked by paired physical identity.
    let pc = if c.reversed {
        c.pcurve.reverse()?
    } else {
        c.pcurve.clone()
    };
    for end in 0..2 {
        let vertex = a.vertices[if reverse { 1 - end } else { end }];
        let key = &graph.vertices[vertex];
        let point = m.vertices[m.edges[e].vertices[end]].point;
        let uv = if end == 0 {
            &pc.control_points[0]
        } else {
            pc.control_points.last().unwrap()
        };
        let event = pair
            .events
            .iter()
            .find(|x| x.key == *key)
            .ok_or_else(|| fail("graph endpoint event is missing"))?;
        if (0..3).any(|k| point[k] < event.world_box[k][0] || point[k] > event.world_box[k][1])
            || (0..2).any(|k| {
                uv[k] < event.parameter_box[2 * side + k][0]
                    || uv[k] > event.parameter_box[2 * side + k][1]
            })
        {
            return Err(fail(
                "saved endpoint is not in its unique ORIGINAL algebraic root enclosure",
            ));
        }
    }
    if free / 2 == side {
        let k = free % 2;
        if pc.weights.windows(2).any(|w| w[0] != w[1]) || pc.control_points.len() != pc.degree + 1 {
            return Err(fail(
                "exact monotone arc clock needs one polynomial Bernstein span",
            ));
        }
        let sign = if reverse { -1. } else { 1. };
        if pc
            .control_points
            .windows(2)
            .any(|w| (w[1][k] - w[0][k]) * sign <= 0.)
        {
            return Err(fail(
                "saved intersection arc clock is not strictly monotone",
            ));
        }
    }
    Ok(())
}
pub(super) fn paired_arcs(
    g: &QualifiedNurbsIntersectionGraph,
    p: &[NurbsFacePartition; 2],
) -> Result<()> {
    for arc in 0..g.certificate().arcs.len() {
        let get = |side: usize| -> Result<(usize, bool)> {
            let ids = p[side]
                .edge_sources
                .iter()
                .enumerate()
                .filter_map(|(e, o)| match o {
                    RegionEdgeOrigin::Intersection { arc: a, reversed } if *a == arc => {
                        Some((e, *reversed))
                    }
                    _ => None,
                })
                .collect::<Vec<_>>();
            if ids.len() != 1 {
                return Err(fail("each arc needs one exact saved edge on each operand"));
            }
            Ok(ids[0])
        };
        let (a, ra) = get(0)?;
        let (b, rb) = get(1)?;
        exact::same(
            &p[0].model.edges[a].curve,
            &p[1].model.edges[b].curve,
            ra != rb,
        )?;
    }
    Ok(())
}
