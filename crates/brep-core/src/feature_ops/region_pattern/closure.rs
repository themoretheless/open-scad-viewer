//! Original rim cycles grouped by retained charts or certified coplanar
//! carriers. Intra-chart nesting and cross-chart world separation are separate.
use super::*;
use nurbs_core::surface::Surface;
pub(super) struct Cap {
    pub chart: usize,
    pub supports: Vec<usize>,
    pub loop_parents: Vec<Vec<usize>>,
    pub surface: Surface,
    pub rings: Vec<Vec<crate::Coedge>>,
    pub reversed: bool,
}
pub(super) struct Closure {
    pub supports: Vec<usize>,
    pub source_loop: Option<usize>,
    pub caps: Vec<Cap>,
    pub work: ProofWork,
}
fn reverse(ring: &mut Vec<crate::Coedge>) -> Result<()> {
    ring.reverse();
    for c in ring {
        c.reversed = !c.reversed;
        c.pcurve = c.pcurve.reverse()?;
    }
    Ok(())
}
pub(super) fn prepare(
    source: &Model,
    selected: &BTreeSet<usize>,
    uses: &BTreeMap<usize, bool>,
    incidence: &BTreeMap<usize, Vec<(usize, usize)>>,
    rim: &BTreeSet<usize>,
    neighbors: &BTreeSet<(usize, usize)>,
    limits: Limits,
) -> Result<Closure> {
    if rim.len() < 2 || rim.len() > 256 {
        return Err(error("Bounded original feature rim cycles required"));
    }
    let supports = neighbors
        .iter()
        .map(|(f, _)| *f)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    if supports.is_empty() || supports.len() > 64 {
        return Err(error("Original closure support allowance exceeded"));
    }
    let mut next = BTreeMap::new();
    for &edge in rim {
        let (f, w) = incidence[&edge]
            .iter()
            .find(|(f, _)| selected.contains(f))
            .copied()
            .unwrap();
        let c = source.loops[w]
            .coedges
            .iter()
            .find(|c| c.edge == edge)
            .unwrap();
        let reversed = !(c.reversed ^ uses[&f]);
        let vertex = source.edges[edge].vertices[usize::from(reversed)];
        if next.insert(vertex, (edge, reversed)).is_some() {
            return Err(error("Original feature rim is pinched"));
        }
    }
    let mut cycles = Vec::new();
    let mut visited = BTreeSet::new();
    for &start in next.keys() {
        if visited.contains(&next[&start].0) {
            continue;
        }
        if cycles.len() == 16 {
            return Err(error("At most 16 original rim cycles supported"));
        }
        let mut v = start;
        let mut cycle = Vec::new();
        loop {
            let &(edge, reversed) = next
                .get(&v)
                .ok_or_else(|| error("Original feature rim is open"))?;
            if !visited.insert(edge) {
                return Err(error("Original feature rim repeats an edge"));
            }
            cycle.push((edge, reversed));
            v = source.edges[edge].vertices[usize::from(!reversed)];
            if v == start {
                break;
            }
        }
        if cycle.len() < 2 {
            return Err(error("Original cap needs at least two curves per rim"));
        }
        cycles.push(cycle);
    }
    if visited != *rim {
        return Err(error("Original feature rim coverage is incomplete"));
    }
    let mut work = ProofWork::default();
    cycles.sort_by_key(|r| {
        let mut ids = r
            .iter()
            .map(|(e, _)| source.1.edges[*e])
            .collect::<Vec<_>>();
        ids.sort();
        ids
    });
    let mut groups = Vec::new();
    if let Some(carrier) =
        super::cap_carrier::propose(source, selected, incidence, rim, limits, &mut work)?
    {
        groups.push((cycles, carrier));
    } else {
        // These are proposals only. Different proposed charts must later prove
        // strict separation of their ACTUAL trimmed material caps in world space.
        for cycle in cycles {
            let mut added = false;
            for (members, carrier) in &mut groups {
                let edges = members
                    .iter()
                    .flatten()
                    .chain(cycle.iter())
                    .map(|(e, _)| *e)
                    .collect();
                if let Some(candidate) = super::cap_carrier::propose(
                    source, selected, incidence, &edges, limits, &mut work,
                )? {
                    members.push(cycle.clone());
                    *carrier = candidate;
                    added = true;
                    break;
                }
            }
            if !added {
                let edges = cycle.iter().map(|(e, _)| *e).collect();
                let carrier = super::cap_carrier::propose(
                    source, selected, incidence, &edges, limits, &mut work,
                )?
                .ok_or_else(|| {
                    error("Original rim needs a supported carrier or additional closure seams")
                })?;
                groups.push((vec![cycle], carrier));
            }
        }
    }
    let source_loop = if groups.len() == 1 {
        groups[0].1.source_loop
    } else {
        None
    };
    let mut caps = Vec::new();
    for (chart, (cycles, carrier)) in groups.into_iter().enumerate() {
        let surface = carrier.surface;
        let curves = carrier.curves;
        let mut rings = Vec::new();
        let mut parents = Vec::new();
        let mut areas = Vec::new();
        for cycle in cycles {
            let mut ring = Vec::new();
            let mut loop_parents = BTreeSet::new();
            for (edge, reversed) in cycle {
                let (_, w) = incidence[&edge]
                    .iter()
                    .find(|(f, _)| !selected.contains(f))
                    .copied()
                    .ok_or_else(|| error("Original rim support is missing"))?;
                loop_parents.insert(w);
                let pcurve = if let Some(curves) = &curves {
                    let p = &curves[&edge];
                    if reversed { p.reverse()? } else { p.clone() }
                } else {
                    let c = source.loops[w]
                        .coedges
                        .iter()
                        .find(|c| c.edge == edge)
                        .unwrap();
                    if c.reversed == reversed {
                        c.pcurve.clone()
                    } else {
                        c.pcurve.reverse()?
                    }
                };
                ring.push(crate::Coedge::new(edge, reversed, pcurve));
            }
            let mut box_uv = [[f64::INFINITY; 2], [f64::NEG_INFINITY; 2]];
            for c in &ring {
                for p in &c.pcurve.control_points {
                    for k in 0..2 {
                        box_uv[0][k] = box_uv[0][k].min(p[k]);
                        box_uv[1][k] = box_uv[1][k].max(p[k]);
                    }
                }
            }
            let scale = (box_uv[1][0] - box_uv[0][0]) * (box_uv[1][1] - box_uv[0][1]);
            if !scale.is_finite() || scale <= 0. {
                return Err(error("Original closure UV extent is unresolved"));
            }
            if work.cells == limits.max_cells {
                return Err(error("Original rim orientation proof work exhausted"));
            }
            let area = nurbs_core::planar_area::measure(
                &ring.iter().map(|c| c.pcurve.clone()).collect::<Vec<_>>(),
                (scale * 1e-3).max(f64::MIN_POSITIVE),
                (limits.max_cells - work.cells).min(4096),
            )?;
            work.cells += area.cells;
            let [lo, hi] = area.area_interval_mm2;
            if lo <= 0. && hi >= 0. {
                return Err(error("Original closure winding is unresolved"));
            }
            areas.push([lo, hi]);
            rings.push(ring);
            parents.push(loop_parents.into_iter().collect::<Vec<_>>());
        }
        let components =
            super::cap_components::classify(source, &rings, &areas, limits, &mut work)?;
        for component in components {
            let mut cap_rings = Vec::new();
            let mut cap_parents = Vec::new();
            let mut cap_supports = BTreeSet::new();
            for i in component.members {
                let mut ring = rings[i].clone();
                for c in &ring {
                    let (f, _) = incidence[&c.edge]
                        .iter()
                        .find(|(f, _)| !selected.contains(f))
                        .unwrap();
                    cap_supports.insert(*f);
                }
                if component.reversed {
                    reverse(&mut ring)?;
                }
                cap_rings.push(ring);
                cap_parents.push(parents[i].clone());
            }
            caps.push(Cap {
                chart,
                supports: cap_supports.into_iter().collect(),
                loop_parents: cap_parents,
                surface: surface.clone(),
                rings: cap_rings,
                reversed: component.reversed,
            });
        }
    }
    caps.sort_by_key(|c| {
        let mut ids = c.rings[0]
            .iter()
            .map(|e| source.1.edges[e.edge])
            .collect::<Vec<_>>();
        ids.sort();
        ids
    });
    super::trim_clocks::transfer(source, selected, incidence, &mut caps, limits, &mut work)?;
    super::cap_separation::verify(&caps, source.tolerance_mm, limits, &mut work)?;
    Ok(Closure {
        supports,
        source_loop,
        caps,
        work,
    })
}
