use super::*;
struct Work {
    used: usize,
    max: usize,
    candidates: usize,
}
fn point(f: &QualifiedFiber, t: f64, w: &mut Work) -> Result<ClosedFiberRootPort> {
    if w.used == w.max {
        return Err(crate::resource("Closed traversal point budget exhausted"));
    }
    let r = super::super::band::refine_band(f, [t, t], w.max - w.used)?;
    w.used += r.cells;
    let point = r.fiber;
    let bounds = point_bounds(&point)?;
    Ok(ClosedFiberRootPort { point, bounds })
}
fn transition(
    owner: &QualifiedClosedFiberBranches,
    a: &QualifiedFiber,
    b: &QualifiedFiber,
    w: &mut Work,
) -> Result<Option<QualifiedFiberTransition>> {
    for t in owner.owner().proof().transitions() {
        if orientation(t, a, b).is_some() {
            if t.rectangles() > w.max - w.used {
                return Err(crate::resource("Closed transition replay budget exhausted"));
            }
            t.reverify()?;
            w.used += t.rectangles();
            return Ok(Some(t.clone()));
        }
    }
    let allowance = (w.max - w.used).min(4096);
    if allowance == 0 {
        return Err(crate::resource(
            "Closed transition proposal budget exhausted",
        ));
    }
    let result = discover_fiber_transition(
        a,
        b,
        FiberTransitionDiscoveryLimits {
            max_rectangles: allowance,
            max_proposals: 64,
            max_depth: 12,
        },
    );
    let r = match result {
        Ok(r) => r,
        Err(e) => {
            w.used += allowance;
            return Err(e);
        }
    };
    w.used += r.rectangles();
    Ok(r.into_transition().map(|t| t.proof().clone()))
}
fn walk(
    owner: &QualifiedClosedFiberBranches,
    branch: usize,
    limits: ClosedFiberCycleLimits,
    start_index: usize,
    fraction: f64,
    start_forward: bool,
    w: &mut Work,
) -> Result<Option<OrderedClosedFiberCycle>> {
    let ids = &owner.branches()[branch].charts;
    let all = owner.owner().proof().coverage().fibers();
    let mut pool: Vec<_> = ids.iter().map(|i| all[*i].clone()).collect();
    pool.swap(0, start_index);
    let free = pool[0].free_axis().index();
    let [a, b] = pool[0].domain()[free];
    let t = a + (b - a) * fraction;
    if !(a < t && t < b) {
        return Ok(None);
    }
    let start = point(&pool[0], t, w)?;
    if !inside(start.bounds, pool[0].domain(), true) {
        return Ok(None);
    }
    let mut ports = vec![start];
    let mut arcs = Vec::new();
    let mut transitions = Vec::new();
    let mut current = 0;
    let mut forward = start_forward;
    let mut visited = vec![false; pool.len()];
    for _ in 0..limits.max_steps {
        let f = &pool[current];
        let axis = f.free_axis().index();
        let incoming = ports.last().unwrap().bounds;
        if !arcs.is_empty()
            && inside(ports[0].bounds, f.domain(), true)
            && before(incoming, ports[0].bounds, axis, forward)
        {
            let Some(close) = transition(owner, f, &pool[0], w)? else {
                return Ok(None);
            };
            let agrees = orientation(&close, f, &pool[0]).unwrap();
            if (if agrees { forward } else { !forward }) != start_forward {
                return Ok(None);
            }
            arcs.push(OrderedClosedFiberArc {
                fiber: f.clone(),
                forward,
            });
            transitions.push(close);
            return Ok(Some(OrderedClosedFiberCycle {
                owner: owner.clone(),
                branch,
                first_chart: ids[start_index],
                arcs,
                ports,
                transitions,
                limits,
                rectangles: w.used,
            }));
        }
        if visited[current] {
            return Ok(None);
        }
        visited[current] = true;
        let tip = f.domain()[axis][usize::from(forward)];
        let entry = incoming[axis][usize::from(forward)];
        let mut clocks = [1., 0.875, 0.75, 0.5, 0.25]
            .map(|fraction| {
                if fraction == 1. {
                    tip
                } else {
                    entry + (tip - entry) * fraction
                }
            })
            .to_vec();
        let travel = [entry.min(tip), entry.max(tip)];
        for (next, candidate) in pool.iter().enumerate() {
            if next == current {
                continue;
            }
            let domain = candidate.domain();
            if disjoint(f.domain(), domain) {
                continue;
            }
            let lo = travel[0].max(domain[axis][0]);
            let hi = travel[1].min(domain[axis][1]);
            let mid = lo * 0.5 + hi * 0.5;
            if lo < mid && mid < hi && !clocks.contains(&mid) {
                clocks.push(mid);
            }
        }
        let mut chosen = None;
        'handoff: for t in clocks {
            let endpoint = point(f, t, w)?;
            if !before(incoming, endpoint.bounds, axis, forward) {
                continue;
            }
            let start_bounds = ports[0].bounds;
            if !arcs.is_empty()
                && !disjoint(start_bounds, f.domain())
                && !before(start_bounds, incoming, axis, forward)
                && !before(endpoint.bounds, start_bounds, axis, forward)
            {
                continue;
            }
            let mut candidates: Vec<_> = (0..pool.len())
                .filter(|i| *i != current && inside(endpoint.bounds, pool[*i].domain(), true))
                .collect();
            let margin = |i: usize| {
                (0..4)
                    .map(|k| {
                        let d = pool[i].domain()[k];
                        let width = d[1] - d[0];
                        ((endpoint.bounds[k][0] - d[0]).min(d[1] - endpoint.bounds[k][1])) / width
                    })
                    .fold(f64::INFINITY, f64::min)
            };
            candidates.sort_by(|a, b| margin(*b).total_cmp(&margin(*a)).then(a.cmp(b)));

            for next in candidates {
                if w.candidates == limits.max_candidates {
                    return Err(crate::resource(
                        "Closed traversal candidate budget exhausted",
                    ));
                }
                w.candidates += 1;
                let Some(tr) = transition(owner, f, &pool[next], w)? else {
                    continue;
                };
                let agrees = orientation(&tr, f, &pool[next]).unwrap();
                let direction = if agrees { forward } else { !forward };
                let p0 = ports[0].bounds;
                let domain = pool[next].domain();
                let next_axis = pool[next].free_axis().index();
                let closes =
                    inside(p0, domain, true) && before(endpoint.bounds, p0, next_axis, direction);
                if !closes && visited[next] {
                    continue;
                }
                chosen = Some((next, direction, tr, endpoint));
                break 'handoff;
            }
        }
        let Some((next, direction, tr, endpoint)) = chosen else {
            return Ok(None);
        };
        arcs.push(OrderedClosedFiberArc {
            fiber: f.clone(),
            forward,
        });
        ports.push(endpoint);
        transitions.push(tr);
        current = next;
        forward = direction;
    }
    Ok(None)
}
/// Orient one whole embedded closed component through shared original roots.
/// Every intermediate arc certifies that the start root has not been passed;
/// the final arc reaches that SAME selector for the first time. No graph-cycle
/// inference, midpoint welding or world-point proximity authorizes closure.
pub fn order_closed_fiber_cycle(
    owner: &QualifiedClosedFiberBranches,
    branch: usize,
    limits: ClosedFiberCycleLimits,
) -> Result<OrderedClosedFiberCycle> {
    check(
        branch < owner.branches().len()
            && (1..=100000).contains(&limits.max_rectangles)
            && (2..=256).contains(&limits.max_steps)
            && (1..=8192).contains(&limits.max_candidates)
            && (1..=128).contains(&limits.max_start_attempts),
        "Invalid closed traversal inputs or limits",
    )?;
    let mut w = Work {
        used: 0,
        max: limits.max_rectangles,
        candidates: 0,
    };
    let count = owner.branches()[branch].charts.len();
    let fractions = [
        0.4375, 0.3125, 0.5625, 0.6875, 0.1875, 0.8125, 0.0625, 0.9375,
    ];
    for attempt in 0..limits.max_start_attempts {
        let start = (attempt / 2) % count;
        let forward = attempt % 2 == 0;
        let fraction = fractions[(attempt / 2) % fractions.len()];
        if let Some(cycle) = walk(owner, branch, limits, start, fraction, forward, &mut w)? {
            cycle.verify(false)?;
            return Ok(cycle);
        }
    }
    Err(crate::input(&format!(
        "No fully oriented original first-return cycle for branch {branch} ({count} charts, {} candidate transitions, {} rectangles)",
        w.candidates, w.used,
    )))
}
