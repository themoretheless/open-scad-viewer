use super::*;
fn anchor(
    input: &QualifiedCutPartitions<'_, '_, '_>,
    node: usize,
    side: usize,
) -> Result<Option<(usize, Vec<f64>)>> {
    let model = &input.models[input.side_model[side]];
    let face = &model.faces[input.source.roots.faces[side].1];
    let mut hits = Vec::new();
    for v in input
        .vertices
        .iter()
        .filter(|v| v.model == input.side_model[side] && v.node == node)
    {
        let mut outgoing = Vec::new();
        let mut incoming = Vec::new();
        for wire in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
            for c in &model.loops[wire].coedges {
                let e = &model.edges[c.edge];
                if e.vertices[usize::from(c.reversed)] == v.vertex {
                    outgoing.push(c.pcurve.control_points[0].clone());
                }
                if e.vertices[usize::from(!c.reversed)] == v.vertex {
                    incoming.push(c.pcurve.control_points.last().unwrap().clone());
                }
            }
        }
        if outgoing.is_empty() && incoming.is_empty() {
            continue;
        }
        if outgoing.len() != 1 || incoming.len() != 1 || outgoing[0] != incoming[0] {
            return Err(unproven(
                "SS endpoint needs one exact actual boundary UV port",
            ));
        }
        if model.vertices[v.vertex].point.map(f64::to_bits)
            != input.source.nodes[node].position.map(f64::to_bits)
        {
            return Err(unproven(
                "Actual SS endpoint lost its canonical world position",
            ));
        }
        hits.push((v.vertex, outgoing.remove(0)));
    }
    if hits.len() > 1 {
        return Err(unproven(
            "SS endpoint has ambiguous actual boundary ownership",
        ));
    }
    Ok(hits.pop())
}
fn endpoint(
    input: &QualifiedCutPartitions<'_, '_, '_>,
    node: usize,
    step: &OpenPathStep,
    start: bool,
) -> Result<End> {
    let n = input
        .source
        .nodes
        .get(node)
        .ok_or_else(|| invalid("Unknown source SS cut node"))?;
    let original = input.source.roots.original;
    let edge = original
        .edges()
        .get(step.original_edge)
        .ok_or_else(|| invalid("Unknown original SS route edge"))?;
    let (clock, band) = if let Some(v) = n.original_vertex {
        let at = usize::from(start == step.reversed);
        if edge.vertices[at] != v {
            return Err(unproven(
                "Source SS root vertex is not the requested edge port",
            ));
        }
        let t = at as f64;
        (t, [t, t])
    } else {
        if n.original_edge != Some(step.original_edge) {
            return Err(unproven("Source SS root belongs to another original edge"));
        }
        let mut band = [0f64, 1f64];
        let mut found = false;
        for c in input.source.cuts.iter().filter(|c| c.node == node) {
            let r = &input.source.roots.records[c.record];
            let event = &r.events[c.event];
            let binding = r
                .bindings
                .bindings()
                .iter()
                .find(|b| b.original_root == event.original_root)
                .unwrap();
            let GraphRootLocation::Interior(b) = &binding.location else {
                return Err(unproven("Source SS node incidence changed"));
            };
            if b.original_edge != step.original_edge {
                return Err(unproven("Source SS endpoint clock changed edges"));
            }
            band = [band[0].max(b.clock[0]), band[1].min(b.clock[1])];
            found = true;
        }
        let t = n
            .original_clock
            .ok_or_else(|| unproven("Source SS root lacks its original clock"))?;
        if !found || !(0. < band[0] && band[0] <= t && t <= band[1] && band[1] < 1.) {
            return Err(unproven(
                "Source SS endpoint clock is not owned by its root bands",
            ));
        }
        (t, band)
    };
    let anchors = [anchor(input, node, 0)?, anchor(input, node, 1)?];
    if anchors.iter().all(Option::is_none) {
        return Err(unproven(
            "Source SS endpoint lost every actual boundary port",
        ));
    }
    Ok(End {
        node,
        clock,
        band,
        position: n.position,
        anchors,
    })
}
pub(super) fn route(
    input: &QualifiedCutPartitions<'_, '_, '_>,
    r: &OpenSSPathRequest,
    used: &mut usize,
    max: usize,
) -> Result<(Vec<Part>, [End; 2])> {
    let original = input.source.roots.original;
    let mut owner = None;
    for s in &r.steps {
        spend(used, max, original.branches().len() + 1)?;
        if s.original_edge >= original.edges().len() {
            return Err(invalid("Unknown original SS route edge"));
        }
        let branches: Vec<_> = original
            .branches()
            .iter()
            .enumerate()
            .filter(|(_, b)| b.edges.contains(&s.original_edge))
            .map(|(i, _)| i)
            .collect();
        if branches.len() != 1 || owner.is_some_and(|i| i != branches[0]) {
            return Err(unproven("An open SS route must follow one original branch"));
        }
        owner = Some(branches[0]);
    }
    for pair in r.steps.windows(2) {
        let a = &original.edges()[pair[0].original_edge];
        let b = &original.edges()[pair[1].original_edge];
        if a.vertices[usize::from(!pair[0].reversed)] != b.vertices[usize::from(pair[1].reversed)] {
            return Err(unproven(
                "Original SS route has disconnected directed ports",
            ));
        }
    }
    let ends = [
        endpoint(input, r.start_node, &r.steps[0], true)?,
        endpoint(input, r.end_node, r.steps.last().unwrap(), false)?,
    ];
    let mut parts = Vec::new();
    for (i, s) in r.steps.iter().enumerate() {
        let mut a = if s.reversed { 1. } else { 0. };
        let mut b = if s.reversed { 0. } else { 1. };
        let mut ca = a;
        let mut cb = b;
        let mut ga = a;
        let mut gb = b;
        if i == 0 {
            a = ends[0].clock;
            ca = ends[0].band[usize::from(!s.reversed)];
            ga = ends[0].band[usize::from(s.reversed)];
        }
        if i + 1 == r.steps.len() {
            b = ends[1].clock;
            cb = ends[1].band[usize::from(s.reversed)];
            gb = ends[1].band[usize::from(!s.reversed)];
        }
        let clock = if s.reversed { [b, a] } else { [a, b] };
        let core = if s.reversed { [cb, ca] } else { [ca, cb] };
        let coverage = if s.reversed { [gb, ga] } else { [ga, gb] };
        if !(0. <= clock[0]
            && clock[0] < clock[1]
            && clock[1] <= 1.
            && clock[0] <= core[0]
            && core[0] < core[1]
            && core[1] <= clock[1])
        {
            return Err(unproven(
                "SS endpoint bands do not prove a directed nonempty core",
            ));
        }
        parts.push(Part {
            step: s.clone(),
            clock,
            core,
            coverage,
        });
    }
    for i in 0..parts.len() {
        for j in 0..i {
            spend(used, max, 1)?;
            if parts[i].step.original_edge == parts[j].step.original_edge
                && parts[i].coverage[0] <= parts[j].coverage[1]
                && parts[j].coverage[0] <= parts[i].coverage[1]
            {
                return Err(unproven(
                    "Original SS route repeats or ambiguously overlaps its clocks",
                ));
            }
        }
    }
    Ok((parts, ends))
}
pub(super) fn no_omitted_events(
    input: &QualifiedCutPartitions<'_, '_, '_>,
    request: &OpenSSPathRequest,
    parts: &[Part],
    used: &mut usize,
    max: usize,
) -> Result<()> {
    let roots = input.source.roots;
    let canonical = canonical_records(roots, used, max)?;
    let mut nodes = std::collections::BTreeMap::new();
    for c in &input.source.cuts {
        let r = &roots.records[c.record];
        nodes.insert(key(r, canonical[c.record], &r.events[c.event])?, c.node);
    }
    for (ri, r) in roots.records.iter().enumerate() {
        for e in &r.events {
            spend(used, max, parts.len() + 1)?;
            let identity = key(r, canonical[ri], e)?;
            if nodes
                .get(&identity)
                .is_some_and(|&n| n == request.start_node || n == request.end_node)
            {
                continue;
            }
            let b = r
                .bindings
                .bindings()
                .iter()
                .find(|b| b.original_root == e.original_root)
                .unwrap();
            let overlaps = match &b.location {
                GraphRootLocation::Interior(c) => parts.iter().any(|p| {
                    p.step.original_edge == c.original_edge
                        && c.clock[0] <= p.coverage[1]
                        && p.coverage[0] <= c.clock[1]
                }),
                GraphRootLocation::Vertex {
                    original_vertex: v, ..
                } => parts.iter().any(|p| {
                    let e = &roots.original.edges()[p.step.original_edge];
                    (p.coverage[0] == 0. && e.vertices[0] == *v)
                        || (p.coverage[1] == 1. && e.vertices[1] == *v)
                }),
            };
            if overlaps {
                return Err(unproven(
                    "An intermediate audited SS boundary event requires an explicit network split",
                ));
            }
        }
    }
    Ok(())
}
