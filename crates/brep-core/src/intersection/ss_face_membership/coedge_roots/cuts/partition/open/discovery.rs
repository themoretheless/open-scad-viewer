use super::*;

/// Discover every retained open interval between consecutive certified
/// boundary events. A branch without two distinct events has no such interval.
/// An unresolved interval fails the entire discovery.
pub fn discover_open_ss_paths<'i, 'p, 'r, 'a>(
    input: &'i QualifiedCutPartitions<'p, 'r, 'a>,
    side: usize,
    limits: OpenSSPathLimits,
) -> Result<Vec<QualifiedOpenSSPath<'i, 'p, 'r, 'a>>> {
    if side > 1
        || !(1..=1000000).contains(&limits.operations)
        || !(1..=1000000).contains(&limits.cells)
        || !limits.world_tolerance.is_finite()
        || limits.world_tolerance <= 0.
        || !limits.uv_tolerance.is_finite()
        || limits.uv_tolerance <= 0.
    {
        return Err(invalid("Need bounded open SS discovery work"));
    }
    let mut operations = 0;
    let mut cells = 0;
    spend(&mut operations, limits.operations, input.operations)?;
    spend(&mut cells, limits.cells, input.cells)?;
    input.reverify()?;
    let original = input.source.roots.original;
    let mut retained = Vec::new();
    for branch in original.branches() {
        let count = branch.edges.len();
        let mut events = Vec::new();
        for (position, &edge) in branch.edges.iter().enumerate() {
            let ports = original.edges()[edge].vertices;
            for (node, event) in input.source.nodes.iter().enumerate() {
                spend(&mut operations, limits.operations, 1)?;
                if event.original_vertex == Some(ports[0]) {
                    events.push((position, 0., node));
                } else if !branch.closed
                    && position + 1 == count
                    && event.original_vertex == Some(ports[1])
                {
                    events.push((count, 0., node));
                } else if event.original_edge == Some(edge) {
                    let clock = event
                        .original_clock
                        .filter(|t| *t > 0. && *t < 1.)
                        .ok_or_else(|| unproven("SS boundary event lost its interior clock"))?;
                    events.push((position, clock, node));
                }
            }
        }
        events.sort_unstable_by(|a, b| a.0.cmp(&b.0).then(a.1.total_cmp(&b.1)));
        if events.len() < 2 {
            continue;
        }
        let intervals = events.len() - usize::from(!branch.closed);
        for index in 0..intervals {
            let (start, start_clock, start_node) = events[index];
            let (mut end, end_clock, end_node) = events[(index + 1) % events.len()];
            if index + 1 == events.len() {
                end += count;
            }
            if start == end && start_clock >= end_clock {
                return Err(unproven("SS boundary events have ambiguous order"));
            }
            let end = end + usize::from(end_clock > 0.);
            spend(&mut operations, limits.operations, end - start)?;
            let request = OpenSSPathRequest {
                side,
                start_node,
                end_node,
                steps: (start..end)
                    .map(|index| OpenPathStep {
                        original_edge: branch.edges[index % count],
                        reversed: false,
                    })
                    .collect(),
            };
            let (parts, _) = route(input, &request, &mut operations, limits.operations)?;
            let uv = limits.uv_tolerance.min(input.source.limits.uv_tolerance);
            // Events already proved outside the opposite face cannot change
            // membership in the common retained region. They may occur on an
            // exterior interval, but any event on a retained path must bind.
            if classify_cores(input, &parts, uv, &mut operations, &mut cells, limits)? {
                no_omitted_events(input, &request, &parts, &mut operations, limits.operations)?;
                retained.push(request);
            }
        }
    }
    if retained.is_empty() {
        return Ok(Vec::new());
    }
    let per_path = OpenSSPathLimits {
        operations: (limits.operations - operations) / retained.len(),
        cells: (limits.cells - cells) / retained.len(),
        ..limits
    };
    retained
        .into_iter()
        .map(|request| qualify_open_ss_path(input, request, per_path))
        .collect()
}

/// Find a proven retained route between two canonical boundary cut nodes.
/// Closed branches may offer two directions. Each candidate goes through the
/// same original-root and retained-face proof as an explicitly supplied route.
pub fn discover_open_ss_path<'i, 'p, 'r, 'a>(
    input: &'i QualifiedCutPartitions<'p, 'r, 'a>,
    side: usize,
    nodes: [usize; 2],
    limits: OpenSSPathLimits,
) -> Result<QualifiedOpenSSPath<'i, 'p, 'r, 'a>> {
    let [start_node, end_node] = nodes;
    let cut = |node| {
        input
            .source
            .nodes
            .get(node)
            .ok_or_else(|| invalid("Unknown SS cut node"))
    };
    let [start, end] = [cut(start_node)?, cut(end_node)?];
    if side > 1
        || start_node == end_node
        || !(1..=1000000).contains(&limits.operations)
        || !(1..=1000000).contains(&limits.cells)
        || !limits.world_tolerance.is_finite()
        || limits.world_tolerance <= 0.
        || !limits.uv_tolerance.is_finite()
        || limits.uv_tolerance <= 0.
    {
        return Err(invalid(
            "Need distinct SS cut nodes and bounded route discovery",
        ));
    }
    let original = input.source.roots.original;
    let mut operations = 0;
    let mut candidates = Vec::new();
    for branch in original.branches() {
        for reversed in [false, true] {
            let edges: Vec<_> = if reversed {
                branch.edges.iter().rev().copied().collect()
            } else {
                branch.edges.clone()
            };
            spend(&mut operations, limits.operations, edges.len())?;
            let start_port = usize::from(reversed);
            let end_port = 1 - start_port;
            let matches = |node: &CutNode, edge: usize, port: usize| {
                node.original_edge == Some(edge)
                    || node.original_vertex == Some(original.edges()[edge].vertices[port])
            };
            for index in 0..edges.len() {
                if !matches(start, edges[index], start_port) {
                    continue;
                }
                let mut steps = Vec::new();
                let count = if branch.closed {
                    edges.len() + 1
                } else {
                    edges.len() - index
                };
                for offset in 0..count.min(256) {
                    spend(&mut operations, limits.operations, 1)?;
                    let edge = edges[(index + offset) % edges.len()];
                    steps.push(OpenPathStep {
                        original_edge: edge,
                        reversed,
                    });
                    if matches(end, edge, end_port) {
                        let ordered = offset != 0
                            || start
                                .original_clock
                                .zip(end.original_clock)
                                .is_none_or(|(a, b)| if reversed { a > b } else { a < b });
                        if ordered {
                            spend(&mut operations, limits.operations, steps.len())?;
                            candidates.push(OpenSSPathRequest {
                                side,
                                start_node,
                                end_node,
                                steps,
                            });
                            break;
                        }
                    }
                }
            }
        }
    }
    if candidates.is_empty() {
        return Err(unproven(
            "No directed original SS route connects the cut nodes",
        ));
    }
    // A failed proof may have spent its entire allowance before refusing.
    let per_candidate = OpenSSPathLimits {
        operations: (limits.operations - operations) / candidates.len(),
        cells: limits.cells / candidates.len(),
        ..limits
    };
    for candidate in candidates {
        if let Ok(path) = qualify_open_ss_path(input, candidate, per_candidate) {
            return Ok(path);
        }
    }
    Err(unproven(
        "No original SS route between the cut nodes was proven retained",
    ))
}
