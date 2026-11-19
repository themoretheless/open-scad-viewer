//! Geometric chart checks for a sewn original-root shell.
use super::*;

#[derive(Clone, Debug)]
pub struct QualifiedRootShellCharts {
    source: Arc<RootBrep>,
    max_spans: usize,
    bounds: Vec<[[f64; 2]; 2]>,
    trim_domains: Vec<RootShellTrimDomain>,
}

#[derive(Clone, Debug)]
pub(super) enum RootShellTrimDomain {
    Lifted(Arc<QualifiedRootPeriodicFace>),
    Quotient {
        source: Arc<RootBrep>,
        face: usize,
        limits: ss::OriginalSsPathLimits,
    },
}

impl RootShellTrimDomain {
    pub(super) fn classify_box(&self, query: [[f64; 2]; 2]) -> Result<RootFaceRegion> {
        match self {
            Self::Lifted(domain) => Ok(domain.classify_box(query)?.region().clone()),
            Self::Quotient { source, face, limits } => {
                let proof = source.qualify_periodic_face_membership_box(*face, query, *limits)?;
                Ok(if proof.inside() {
                    RootFaceRegion::InsideSupport
                } else {
                    RootFaceRegion::OutsideOuter
                })
            }
        }
    }
}

impl QualifiedRootShellCharts {
    /// Prove that every face carrier is injective and regular on a rectangle
    /// covering its outer loop. Pairwise contacts require a separate audit.
    pub fn qualify(
        source: Arc<RootBrep>,
        max_spans: usize,
        geometry: ss::OriginalSsCurveLimits,
    ) -> Result<Self> {
        let topology = source.topology();
        if topology.shells.len() != 1 || !topology.shells[0].closed || max_spans == 0 {
            return Err(unproved(
                "Chart qualification requires one closed root shell and a positive span budget",
            ));
        }
        let closure = QualifiedRootPeriodicUvClosure::qualify(
            source.clone(),
            topology
                .loops
                .iter()
                .map(|contour| contour.coedges.len())
                .sum(),
        )?;
        let mut remaining = geometry;
        let mut rectangles = Vec::new();
        let mut trim_domains = Vec::new();
        for (index, face) in topology.faces.iter().enumerate() {
            if !face.surface.periodic_u && !face.surface.periodic_v {
                // On an injective nonperiodic face, a repeated topological
                // vertex is also a repeated UV point. This catches pinches
                // and touching contours before the distinct-face audit.
                let vertices = std::iter::once(face.outer)
                    .chain(face.holes.iter().copied())
                    .flat_map(|id| &topology.loops[id].coedges)
                    .map(|coedge| {
                        topology.edges[coedge.edge].vertices[usize::from(coedge.reversed)]
                    });
                require_distinct_trim_vertices(vertices)?;
            }
            source.audit_face_trim_arcs(index, &mut remaining)?;
            if closure.loops().iter().any(|lift| lift.face == index && lift.winding != [0, 0]) {
                let periodic_axis = match (face.surface.periodic_u, face.surface.periodic_v) {
                    (true, false) => 0,
                    (false, true) => 1,
                    _ => return Err(unproved("Quotient shell chart needs exactly one periodic axis")),
                };
                let report = nurbs_core::surface_injectivity::certify_periodic_extrusion(
                    &face.surface, periodic_axis, max_spans, max_spans,
                )?;
                if !report.proven || !report.regular_interior {
                    return Err(unproved("Root shell quotient carrier injectivity or regularity remains unproved"));
                }
                let cover = periodic_material::periodic_face_cover_root(
                    &source, index, remaining,
                )?;
                if let Some(end) = report.collapsed_end {
                    let transverse = 1 - periodic_axis;
                    let native = crate::boolean_robustness::surface_uv_domains(&face.surface);
                    if cover[transverse][end] == native[transverse][end] {
                        return Err(unproved("Root shell trim cover reaches a collapsed shore"));
                    }
                }
                rectangles.push(cover);
                trim_domains.push(RootShellTrimDomain::Quotient {
                    source: source.clone(),
                    face: index,
                    limits: ss::OriginalSsPathLimits { geometry, ..Default::default() },
                });
                continue;
            }
            let bounds = winding::lifted_loop_bounds(&source, index, face.outer, &mut remaining)?;
            let report =
                nurbs_core::surface_injectivity::certify_on(&face.surface, bounds, max_spans)?;
            if !report.proven {
                return Err(unproved(
                    "Root shell face carrier injectivity remains unproved",
                ));
            }
            rectangles.push(bounds);
            trim_domains.push(RootShellTrimDomain::Lifted(Arc::new(QualifiedRootPeriodicFace::qualify(
                source.clone(),
                index,
                ss::OriginalSsWindingLimits {
                    geometry,
                    ..Default::default()
                },
            )?)));
        }
        Ok(Self {
            source,
            max_spans,
            bounds: rectangles,
            trim_domains,
        })
    }

    pub fn source(&self) -> &Arc<RootBrep> {
        &self.source
    }

    pub(super) fn face_bounds(&self, face: usize) -> [[f64; 2]; 2] {
        self.bounds[face]
    }

    pub(super) fn trim_domain(&self, face: usize) -> &RootShellTrimDomain {
        &self.trim_domains[face]
    }

    pub fn reverify(&self, geometry: ss::OriginalSsCurveLimits) -> Result<()> {
        self.source.reverify()?;
        Self::qualify(self.source.clone(), self.max_spans, geometry)?;
        Ok(())
    }
}

impl RootBrep {
    /// Check nonadjacent trim arcs and nonzero period images on one face.
    pub fn audit_face_trim_arc_separation(
        &self,
        face: usize,
        mut limits: ss::OriginalSsCurveLimits,
    ) -> Result<()> {
        self.audit_face_trim_arcs(face, &mut limits)
    }

    pub(super) fn audit_face_trim_arcs(
        &self,
        face: usize,
        remaining: &mut ss::OriginalSsCurveLimits,
    ) -> Result<()> {
        let data = self
            .topology()
            .faces
            .get(face)
            .ok_or_else(|| unproved("Missing root trim face"))?;
        let ids: Vec<_> = std::iter::once(data.outer)
            .chain(data.holes.iter().copied())
            .collect();
        let mut quotient_boundary = false;
        for &id in &ids {
            let lift = uv_closure::qualify_loop_lift(self, face, id, false)?;
            quotient_boundary |= lift.winding != [0, 0];
            audit_trim_arc_separation(self, face, id, lift, remaining)?;
        }
        if quotient_boundary {
            let periods = winding::face_periods(&data.surface);
            let periodic = [data.surface.periodic_u, data.surface.periodic_v];
            let covers = ids
                .iter()
                .map(|&id| {
                    let lift = uv_closure::qualify_loop_lift(self, face, id, false)?;
                    trim_arc_cover(
                        self.topology(),
                        id,
                        lift.coedge_offsets,
                        &periods,
                        remaining,
                    )
                })
                .collect::<Result<Vec<_>>>()?;
            for (i, first) in covers.iter().enumerate() {
                for second in &covers[i + 1..] {
                    for a in first {
                        for b in second {
                            audit_trim_arc_pair([a, b], &periods, periodic, false, remaining)?;
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

fn audit_trim_arc_separation(
    source: &RootBrep,
    face_id: usize,
    loop_id: usize,
    lift: RootUvLoopLift,
    remaining: &mut ss::OriginalSsCurveLimits,
) -> Result<()> {
    let topology = source.topology();
    let arcs = &topology.loops[loop_id].coedges;
    let surface = &topology.faces[face_id].surface;
    let periods = winding::face_periods(surface);
    let periodic = [surface.periodic_u, surface.periodic_v];
    let has_period = periodic.into_iter().any(|axis| axis);
    let adjacent_pairs = if arcs.len() == 2 { 1 } else { arcs.len() };
    for (index, first) in arcs.iter().take(adjacent_pairs).enumerate() {
        let next = &arcs[(index + 1) % arcs.len()];
        if arcs.len() > 1
            && first.pcurve.support == next.pcurve.support
            && adjacent_trim_arcs_overlap(
                &topology.edges[first.edge].curve,
                &topology.edges[next.edge].curve,
                *remaining,
            )?
        {
            return Err(unproved("Adjacent root trim arcs retrace the same graph"));
        }
    }
    if arcs.len() < 4 && !has_period {
        return Ok(());
    }
    if lift.winding != [0, 0] {
        return qualify_monotone_periodic_trim(topology, loop_id, periodic, lift.winding);
    }
    let arcs = trim_arc_cover(topology, loop_id, lift.coedge_offsets, &periods, remaining)?;
    for a in 0..arcs.len() {
        for b in a..arcs.len() {
            let incident = b <= a + 1 || (a == 0 && b == arcs.len() - 1);
            if !incident || has_period {
                audit_trim_arc_pair(
                    [&arcs[a], &arcs[b]],
                    &periods,
                    periodic,
                    incident,
                    remaining,
                )?;
            }
        }
    }
    Ok(())
}

fn qualify_monotone_periodic_trim(
    topology: &RootTopology,
    loop_id: usize,
    periodic: [bool; 2],
    winding: [i32; 2],
) -> Result<()> {
    let axis = match winding {
        [-1 | 1, 0] if periodic[0] => 0,
        [0, -1 | 1] if periodic[1] => 1,
        _ => return Err(unproved("Root trim needs a simple one-period graph")),
    };
    for coedge in &topology.loops[loop_id].coedges {
        let edge = &topology.edges[coedge.edge].curve;
        let clock = match edge.curve().free_axis() {
            ss::FiberAxis::FirstU => 0,
            ss::FiberAxis::FirstV => 1,
            ss::FiberAxis::SecondU => 2,
            ss::FiberAxis::SecondV => 3,
        };
        if clock != 2 * coedge.pcurve.support + axis
            || (edge.forward() ^ coedge.reversed) != (winding[axis] > 0)
            || [edge.start(), edge.end()].iter().any(|port| {
                let bounds = port.parameter_bounds();
                bounds[0] != bounds[1]
            })
        {
            return Err(unproved(
                "Root trim does not traverse the periodic graph monotonically",
            ));
        }
    }
    // Consecutive exact clock intervals meet in the lifted chart and progress
    // strictly in one direction. Their total length is exactly one period,
    // so each quotient coordinate has just one boundary value.
    Ok(())
}

type TrimArc<'a> = (
    &'a Arc<ss::OriginalSsRootCurve>,
    usize,
    [f64; 2],
    [[f64; 2]; 2],
    [num_rational::BigRational; 2],
);

fn trim_arc_cover<'a>(
    topology: &'a RootTopology,
    loop_id: usize,
    offsets: Vec<[i32; 2]>,
    periods: &[num_rational::BigRational; 2],
    remaining: &mut ss::OriginalSsCurveLimits,
) -> Result<Vec<TrimArc<'a>>> {
    topology.loops[loop_id]
        .coedges
        .iter()
        .zip(offsets)
        .map(|(use_, offset)| {
            let edge = &topology.edges[use_.edge].curve;
            let clock = edge.parameter_bounds();
            let domain = edge.curve().parameter_domain();
            let range = [
                winding::outward(&clock[0], true)?.max(domain[0]),
                winding::outward(&clock[1], false)?.min(domain[1]),
            ];
            let support = use_.pcurve.support;
            let uv = winding::project(edge.curve(), range, support, remaining)?;
            let shift = std::array::from_fn(|axis| {
                &periods[axis] * num_rational::BigRational::from_integer(offset[axis].into())
            });
            let uv = winding::translate(uv, &shift)?;
            Ok((edge.curve(), support, range, uv, shift))
        })
        .collect()
}

fn audit_trim_arc_pair(
    pair: [&TrimArc<'_>; 2],
    periods: &[num_rational::BigRational; 2],
    periodic: [bool; 2],
    incident: bool,
    remaining: &mut ss::OriginalSsCurveLimits,
) -> Result<()> {
    remaining.interval_operations = remaining
        .interval_operations
        .checked_sub(1)
        .ok_or_else(|| unproved("Root trim arc pair budget exhausted"))?;
    let offsets = face_region::overlapping_period_offsets(pair[1].3, pair[0].3, periods, periodic)?;
    face_region::visit_period_offsets(offsets, |offset| {
        if incident && offset == [0, 0] {
            return Ok(());
        }
        let shift = std::array::from_fn(|axis| {
            &periods[axis] * num_rational::BigRational::from_integer(offset[axis].into())
        });
        let mut initial = pair.map(|arc| (arc.2, arc.3));
        initial[1].1 = winding::translate(initial[1].1, &shift)?;
        let mut pending = vec![initial];
        while let Some(mut boxes) = pending.pop() {
            remaining.interval_operations = remaining
                .interval_operations
                .checked_sub(1)
                .ok_or_else(|| unproved("Root trim arc subdivision budget exhausted"))?;
            if (0..2).any(|axis| {
                boxes[0].1[axis][1] < boxes[1].1[axis][0]
                    || boxes[1].1[axis][1] < boxes[0].1[axis][0]
            }) {
                continue;
            }
            let width = |side: usize| boxes[side].0[1] - boxes[side].0[0];
            let side = usize::from(width(1) > width(0));
            let [lo, hi] = boxes[side].0;
            let mid = lo * 0.5 + hi * 0.5;
            if !(lo < mid && mid < hi) {
                return Err(unproved(
                    "Root trim arcs or their period copies may intersect",
                ));
            }
            let (curve, support, _, _, chart_shift) = pair[side];
            for range in [[lo, mid], [mid, hi]] {
                let uv = winding::translate(
                    winding::project(curve, range, *support, remaining)?,
                    chart_shift,
                )?;
                let uv = if side == 1 {
                    winding::translate(uv, &shift)?
                } else {
                    uv
                };
                boxes[side] = (range, uv);
                pending.push(boxes);
            }
        }
        Ok(())
    })
}

fn adjacent_trim_arcs_overlap(
    first: &ss::OriginalSsRootEdge,
    second: &ss::OriginalSsRootEdge,
    limits: ss::OriginalSsCurveLimits,
) -> Result<bool> {
    let first_source = first.curve().source();
    let second_source = second.curve().source();
    if first_source.first() != second_source.first()
        || first_source.second() != second_source.second()
    {
        return Ok(false);
    }
    let (Ok([a, b]), Ok([c, d])) = (first.original_inner_clock(), second.original_inner_clock())
    else {
        return Ok(false);
    };
    let same_axis = first.curve().free_axis() == second.curve().free_axis();
    let (first_range, second_range) = if same_axis {
        let common = [a.clone().max(c.clone()), b.clone().min(d.clone())];
        if common[0] >= common[1] {
            return Ok(false);
        }
        if first.curve().same_definition(second.curve())? {
            return Ok(true);
        }
        (common.clone(), common)
    } else {
        ([a.clone(), b.clone()], [c.clone(), d.clone()])
    };
    Ok(
        trim_subarc_inside_edge(first, second, first_range, [&c, &d], limits)?
            || trim_subarc_inside_edge(second, first, second_range, [&a, &b], limits)?,
    )
}

fn trim_subarc_inside_edge(
    first: &ss::OriginalSsRootEdge,
    second: &ss::OriginalSsRootEdge,
    [lo, hi]: ExactClock,
    [c, d]: [&num_rational::BigRational; 2],
    limits: ss::OriginalSsCurveLimits,
) -> Result<bool> {
    // Prove a common interior subarc using the second chart's uniqueness
    // and an enclosure of the first chart over an inward-rounded clock.
    let axis = fiber_axis_index(first.curve().free_axis());
    let [start, end] = &first.curve().original_graph_domain()?[axis];
    let [local_start, local_end] = first
        .curve()
        .parameter_domain()
        .map(|x| num_rational::BigRational::from_float(x).unwrap());
    let to_local =
        |clock| &local_start + (clock - start) * (&local_end - &local_start) / (end - start);
    let midpoint = (&lo + &hi) / num_rational::BigRational::from_integer(2.into());
    let range = [
        winding::outward(
            &to_local((&lo + &midpoint) / num_rational::BigRational::from_integer(2.into())),
            false,
        )?,
        winding::outward(
            &to_local((&hi + &midpoint) / num_rational::BigRational::from_integer(2.into())),
            true,
        )?,
    ];
    if range[0] >= range[1] {
        return Ok(false);
    }
    let Ok(proof) = second
        .curve()
        .qualify_subarc_window(first.curve(), range, limits)
    else {
        return Ok(false);
    };
    let [lower, upper] = &proof.original_window()[fiber_axis_index(second.curve().free_axis())];
    Ok(c <= lower && upper <= d)
}

fn require_distinct_trim_vertices(vertices: impl IntoIterator<Item = usize>) -> Result<()> {
    let mut seen = std::collections::BTreeSet::new();
    for vertex in vertices {
        if !seen.insert(vertex) {
            return Err(unproved("Nonperiodic root face repeats a trim vertex"));
        }
    }
    Ok(())
}

impl QualifiedRootShellCharts {
    /// Certify separation of all pairs without shared topological vertices.
    /// Returns the number of pairs audited. Incident pairs require seam checks.
    pub fn qualify_nonincident_separation(&self, max_cells: usize) -> Result<usize> {
        if !(1..=1_000_000).contains(&max_cells) {
            return Err(unproved("Invalid root shell separation budget"));
        }
        let topology = self.source.topology();
        let vertices: Vec<std::collections::BTreeSet<usize>> = topology
            .faces
            .iter()
            .map(|face| {
                std::iter::once(face.outer)
                    .chain(face.holes.iter().copied())
                    .flat_map(|id| &topology.loops[id].coedges)
                    .flat_map(|coedge| topology.edges[coedge.edge].vertices)
                    .collect()
            })
            .collect();
        let mut remaining = max_cells;
        let mut pairs = 0;
        for (a, face) in topology.faces.iter().enumerate() {
            for b in a + 1..topology.faces.len() {
                if vertices[a].is_disjoint(&vertices[b]) {
                    separate_charts(
                        [&face.surface, &topology.faces[b].surface],
                        [self.bounds[a], self.bounds[b]],
                        &mut remaining,
                    )?;
                    pairs += 1;
                }
            }
        }
        Ok(pairs)
    }
}

impl QualifiedRootShellCharts {
    pub(super) fn exclude_world_box(&self, world: [[f64; 2]; 3], max_cells: usize) -> Result<()> {
        use nurbs_core::interval_eval::{Interval, evaluate_surface_interval};
        let mut remaining = max_cells;
        for (face, &domain) in self.source.topology().faces.iter().zip(&self.bounds) {
            let mut pending = vec![domain];
            while let Some(mut cell) = pending.pop() {
                remaining = remaining.checked_sub(1)
                    .ok_or_else(|| unproved("Root witness separation budget exhausted"))?;
                let [[u0, u1], [v0, v1]] = cell;
                let enclosure = evaluate_surface_interval(
                    &face.surface, Interval::new(u0, u1)?, Interval::new(v0, v1)?,
                )?;
                if enclosure.iter().zip(world).any(|(v, [lo, hi])| v.hi < lo || hi < v.lo) {
                    continue;
                }
                let axis = usize::from(v1 - v0 > u1 - u0);
                let [lo, hi] = cell[axis];
                let mid = lo * 0.5 + hi * 0.5;
                if !(lo < mid && mid < hi) {
                    return Err(unproved("Root witness separation remains unresolved"));
                }
                let mut upper = cell;
                cell[axis][1] = mid;
                upper[axis][0] = mid;
                pending.extend([upper, cell]);
            }
        }
        Ok(())
    }

    pub(super) fn prove_cross_shell_separation(&self, other: &Self, max_cells: usize) -> Result<()> {
        if !(1..=1_000_000).contains(&max_cells) || Arc::ptr_eq(&self.source, &other.source) {
            return Err(unproved("Invalid cross-shell sources or separation budget"));
        }
        let mut remaining = max_cells;
        for (i, a) in self.source.topology().faces.iter().enumerate() {
            for (j, b) in other.source.topology().faces.iter().enumerate() {
                separate_charts(
                    [&a.surface, &b.surface],
                    [self.bounds[i], other.bounds[j]],
                    &mut remaining,
                )?;
            }
        }
        Ok(())
    }
}

fn separate_charts(
    surfaces: [&Surface; 2],
    rectangles: [[[f64; 2]; 2]; 2],
    remaining: &mut usize,
) -> Result<()> {
    use nurbs_core::interval_eval::{Interval, evaluate_surface_interval};
    let mut pending = vec![rectangles];
    while let Some(pair) = pending.pop() {
        *remaining = remaining
            .checked_sub(1)
            .ok_or_else(|| unproved("Root shell separation budget exhausted"))?;
        let enclose = |i: usize| {
            let [[u0, u1], [v0, v1]] = pair[i];
            evaluate_surface_interval(surfaces[i], Interval::new(u0, u1)?, Interval::new(v0, v1)?)
        };
        let a = enclose(0)?;
        let b = enclose(1)?;
        if a.iter().zip(b).any(|(a, b)| a.hi < b.lo || b.hi < a.lo) {
            continue;
        }
        let (side, axis) = (0..2)
            .flat_map(|side| (0..2).map(move |axis| (side, axis)))
            .max_by(|&(a, i), &(b, j)| {
                let width = |side: usize, axis: usize| pair[side][axis][1] - pair[side][axis][0];
                width(a, i).total_cmp(&width(b, j))
            })
            .unwrap();
        let [lo, hi] = pair[side][axis];
        let mid = lo * 0.5 + hi * 0.5;
        if !(lo < mid && mid < hi) {
            return Err(unproved("Nonincident root face contact remains unresolved"));
        }
        let mut left = pair;
        let mut right = pair;
        left[side][axis][1] = mid;
        right[side][axis][0] = mid;
        pending.extend([right, left]);
    }
    Ok(())
}

#[cfg(test)]
mod tests;

impl QualifiedRootShellCharts {
    /// Prove that the complete SS census for a pair of distinct carriers is
    /// covered by shared edges of these faces. The census must cover both
    /// natural surfaces. Each leaf must be excluded by the trims or covered by shared edges.
    pub fn qualify_shared_edge_contacts(
        &self,
        faces: [usize; 2],
        coverage: &ss::QualifiedQuotientCoverage,
    ) -> Result<()> {
        let topology = self.source.topology();
        let [a, b] = faces.map(|index| topology.faces.get(index));
        let (Some(a), Some(b)) = (a, b) else {
            return Err(unproved("Missing root shell contact face"));
        };
        if faces[0] == faces[1] || a.surface == b.surface || !coverage.covers_entire_surfaces() {
            return Err(unproved(
                "Shared edge contact requires distinct carriers and complete SS coverage",
            ));
        }
        let sources = coverage.fibers()[0].source();
        if sources.first() != &a.surface || sources.second() != &b.surface {
            return Err(unproved("Shared edge contact census has foreign carriers"));
        }
        let edges = |face: &brep_topology::Face<Surface>, support| {
            std::iter::once(face.outer)
                .chain(face.holes.iter().copied())
                .flat_map(|id| &topology.loops[id].coedges)
                .filter(|c| c.pcurve.support == support)
                .map(|c| c.edge)
                .collect::<std::collections::BTreeSet<_>>()
        };
        let first = edges(a, 0);
        let second = edges(b, 1);
        let shared: Vec<_> = first
            .intersection(&second)
            .map(|&id| &topology.edges[id].curve)
            .collect();
        if shared.is_empty() {
            return Err(unproved("SS census has no shared shell edges"));
        }
        let mut refinement_cells = 4096;
        let projected_clocks = std::sync::OnceLock::new();
        for (chart, mut root) in coverage.root_enclosures()? {
            let fiber = &coverage.fibers()[chart];
            let free = fiber_axis_index(fiber.free_axis());
            let periods = [
                winding::face_periods(&a.surface),
                winding::face_periods(&b.surface),
            ];
            let periodic = [
                a.surface.periodic_u,
                a.surface.periodic_v,
                b.surface.periodic_u,
                b.surface.periodic_v,
            ];
            let face_bounds = [self.bounds[faces[0]], self.bounds[faces[1]]];
            let face_periods =
                std::array::from_fn(|axis| periodic[axis].then_some(&periods[axis / 2][axis % 2]));
            if !clip_contact_to_face_bounds(&mut root, face_bounds, face_periods)? {
                continue;
            }
            let clocks =
                intersect_clock_images(root[free], fiber.domain()[free], face_periods[free])?;
            let mut intervals = Vec::new();
            for edge in &shared {
                if edge.curve().matches_quotient_fiber(fiber)? {
                    let (low, high) = if edge.forward() {
                        (edge.start(), edge.end())
                    } else {
                        (edge.end(), edge.start())
                    };
                    intervals.push([
                        low.parameter_bounds()[1].clone(),
                        high.parameter_bounds()[0].clone(),
                    ]);
                }
            }
            if !clocks.is_empty()
                && clocks
                    .into_iter()
                    .all(|clock| covers_clock(intervals.clone(), clock))
            {
                continue;
            }
            let window = root.map(|bounds| {
                bounds.map(|value| num_rational::BigRational::from_float(value).unwrap())
            });
            let native_intervals = if periodic.into_iter().any(|axis| axis) {
                Vec::new()
            } else {
                intervals
            };
            if covers_contact_window(
                &shared,
                sources,
                free,
                window,
                native_intervals,
                face_periods,
            ) {
                continue;
            }
            // Roots on the carriers can lie outside the actual trimmed faces.
            let outside_trim = |window: [[f64; 2]; 4]| {
                faces.into_iter().enumerate().any(|(side, face)| {
                    let query = [window[side * 2], window[side * 2 + 1]];
                    self.trim_domain(face).classify_box(query).is_ok_and(|region| {
                        matches!(
                            region,
                            RootFaceRegion::OutsideOuter | RootFaceRegion::ExcludedByHoles(_)
                        )
                    })
                })
            };
            if !outside_trim(root)
                && (periodic.into_iter().any(|axis| axis)
                    || !refine_contact_clock(
                        &shared, fiber, root, &mut refinement_cells, &projected_clocks, outside_trim,
                    )?)
            {
                return Err(unproved(&format!(
                    "SS census leaf is not fully covered by shared shell edges for faces {faces:?}, chart {chart}, root window {root:?}"
                )));
            }
        }
        Ok(())
    }
}

type ExactClock = [num_rational::BigRational; 2];

/// A census cell can straddle several edge charts even if every physical
/// root belongs to the wire. Contract and subdivide in original coordinates.
fn refine_contact_clock(
    shared: &[&ss::OriginalSsRootEdge],
    fiber: &ss::QualifiedQuotientFiber,
    root: [[f64; 2]; 4],
    remaining: &mut usize,
    projected_clocks: &std::sync::OnceLock<Vec<Option<ExactClock>>>,
    outside_trim: impl Fn([[f64; 2]; 4]) -> bool,
) -> Result<bool> {
    // A RootImage leaf need only enclose roots in the referenced chart.
    // A new contraction can exclude roots only after the entire candidate
    // window is known to belong to that chart.
    if !fiber.contains_original_box(root)? {
        return Ok(false);
    }
    let shared_clocks = projected_clocks.get_or_init(|| {
        shared.iter().map(|edge| edge.original_inner_clock_with_projection().ok()).collect()
    });
    // Split at exact edge-clock and graph-domain boundaries before using
    // midpoints, so the covered side can be checked as a whole.
    use num_traits::ToPrimitive;
    let mut cuts: [Vec<f64>; 4] = std::array::from_fn(|_| Vec::new());
    for (edge, clock) in shared.iter().zip(shared_clocks) {
        let Ok(mut domain) = edge.curve().original_graph_domain() else {
            continue;
        };
        if let Some(clock) = clock {
            let axis = fiber_axis_index(edge.curve().free_axis());
            domain[axis] = intersect_exact_clock(&domain[axis], clock);
        }
        if (0..4).any(|axis| {
            let root = root[axis].map(|value| num_rational::BigRational::from_float(value).unwrap());
            domain[axis][0] > domain[axis][1]
                || domain[axis][1] < root[0] || root[1] < domain[axis][0]
        }) {
            continue;
        }
        for (axis, bounds) in domain.iter().enumerate() {
            for bound in bounds {
                if let Some(value) = bound.to_f64()
                    && num_rational::BigRational::from_float(value).as_ref() == Some(bound)
                    && root[axis][0] < value && value < root[axis][1]
                {
                    cuts[axis].push(value);
                }
            }
        }
    }
    let free = fiber_axis_index(fiber.free_axis());
    let mut pending = vec![(root, 0)];
    'cells: while let Some((mut window, depth)) = pending.pop() {
        if let Ok(Some(corners)) = ss::shared_frame_graph_contact_corners(fiber.source(), window)
            && corners.into_iter().all(|corner| {
                covers_contact_window_with_clocks(
                    shared, fiber.source(), free,
                    corner.map(|value| [value.clone(), value]),
                    Vec::new(), [None; 4], Some(shared_clocks),
                )
            })
        {
            continue;
        }
        // Each operator encloses all roots of its input box, even when
        // its chosen free coordinate cannot prove uniform existence.
        for axis in [
            fiber.free_axis(),
            ss::FiberAxis::FirstU,
            ss::FiberAxis::FirstV,
            ss::FiberAxis::SecondU,
            ss::FiberAxis::SecondV,
        ]
        .into_iter()
        .enumerate()
        .filter_map(|(index, axis)| (index == 0 || axis != fiber.free_axis()).then_some(axis))
        {
            if *remaining == 0 {
                return Ok(false);
            }
            *remaining -= 1;
            let narrowed = fiber.source().certify_quotient(window, axis, Default::default())?;
            let report = narrowed.report();
            if report.status == ss::FiberStatus::Excluded {
                continue 'cells;
            }
            if let Some(image) = report.root_image {
                let axis = fiber_axis_index(axis);
                for (dependent, enclosure) in (0..4).filter(|&k| k != axis).zip(image) {
                    window[dependent][0] = window[dependent][0].max(enclosure[0]);
                    window[dependent][1] = window[dependent][1].min(enclosure[1]);
                }
            }
            if window.iter().any(|bounds| bounds[0] > bounds[1])
                || covers_contact_window_with_clocks(
                    shared,
                    fiber.source(),
                    free,
                    window.map(|bounds| {
                        bounds.map(|value| num_rational::BigRational::from_float(value).unwrap())
                    }),
                    Vec::new(),
                    [None; 4],
                    Some(shared_clocks),
                )
                || outside_trim(window)
            {
                continue 'cells;
            }
        }
        let widest = (0..4).fold(free, |widest, axis| {
            let width = |axis: usize| window[axis][1] - window[axis][0];
            if width(axis) > width(widest) {
                axis
            } else {
                widest
            }
        });
        let [lo, hi] = window[widest];
        let midpoint = lo * 0.5 + hi * 0.5;
        let (axis, split) = cuts.iter().enumerate().find_map(|(axis, cuts)| {
            let [lo, hi] = window[axis];
            cuts.iter().copied().find(|&cut| lo < cut && cut < hi)
                .map(|cut| (axis, cut))
        }).unwrap_or((widest, midpoint));
        if depth == 8 || !(window[axis][0] < split && split < window[axis][1]) {
            return Ok(false);
        }
        let mut upper = window;
        window[axis][1] = split;
        upper[axis][0] = split;
        pending.extend([(upper, depth + 1), (window, depth + 1)]);
    }
    Ok(true)
}

/// Clip a census leaf to the necessary UV bounds of both trimmed faces.
/// Periodic coordinates keep their original representatives for later replay.
fn clip_contact_to_face_bounds(
    root: &mut [[f64; 2]; 4],
    faces: [[[f64; 2]; 2]; 2],
    periods: [Option<&num_rational::BigRational>; 4],
) -> Result<bool> {
    for (axis, bounds) in root.iter_mut().enumerate() {
        let face = faces[axis / 2][axis % 2];
        if let Some(period) = periods[axis] {
            if intersect_clock_images(*bounds, face, Some(period))?.is_empty() {
                return Ok(false);
            }
        } else {
            bounds[0] = bounds[0].max(face[0]);
            bounds[1] = bounds[1].min(face[1]);
            if bounds[0] > bounds[1] {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

fn intersect_clock_images(
    original: [f64; 2],
    chart: [f64; 2],
    period: Option<&num_rational::BigRational>,
) -> Result<Vec<ExactClock>> {
    use num_rational::BigRational;
    use num_traits::{ToPrimitive, Zero};

    let exact = |value| BigRational::from_float(value).unwrap();
    let [lo, hi] = original.map(exact);
    let [start, end] = chart.map(exact);
    let (first, last) = if let Some(period) = period {
        let offset = |value: BigRational| {
            value
                .to_integer()
                .to_i32()
                .ok_or_else(|| unproved("Shared census period offset overflow"))
        };
        (
            offset(((&start - &hi) / period).ceil())?,
            offset(((&end - &lo) / period).floor())?,
        )
    } else {
        (0, 0)
    };
    if i64::from(last) - i64::from(first) >= 4096 {
        return Err(unproved("Shared census period image budget exhausted"));
    }
    Ok((first..=last)
        .filter_map(|offset| {
            let shift = period.map_or_else(BigRational::zero, |period| {
                period * BigRational::from_integer(offset.into())
            });
            let low = (&lo + &shift).max(start.clone());
            let high = (&hi + shift).min(end.clone());
            (low <= high).then_some([low, high])
        })
        .collect())
}

fn fiber_axis_index(axis: ss::FiberAxis) -> usize {
    match axis {
        ss::FiberAxis::FirstU => 0,
        ss::FiberAxis::FirstV => 1,
        ss::FiberAxis::SecondU => 2,
        ss::FiberAxis::SecondV => 3,
    }
}

fn lift_contact_window(
    window: &[ExactClock; 4],
    domain: &[ExactClock; 4],
    periods: [Option<&num_rational::BigRational>; 4],
) -> Option<[ExactClock; 4]> {
    let mut lifted = window.clone();
    for (axis, period) in periods.into_iter().enumerate() {
        let [lo, hi] = &mut lifted[axis];
        let [start, end] = &domain[axis];
        if let Some(period) = period {
            let shift = ((start - &*lo) / period).ceil() * period;
            *lo += &shift;
            *hi += shift;
        }
        if &*lo < start || &*hi > end {
            return None;
        }
    }
    Some(lifted)
}

/// Return covered strips in the census clock and their lifts into the edge chart.
fn contact_clock_strips(
    window: &ExactClock,
    clock: &ExactClock,
    period: Option<&num_rational::BigRational>,
) -> Vec<(ExactClock, num_rational::BigRational)> {
    use num_rational::BigRational;
    use num_traits::{ToPrimitive, Zero};

    let [lo, hi] = window;
    let [start, end] = clock;
    let (first, last) = if let Some(period) = period {
        let first = ((start - hi) / period).ceil().to_integer().to_i32();
        let last = ((end - lo) / period).floor().to_integer().to_i32();
        let (Some(first), Some(last)) = (first, last) else {
            return Vec::new();
        };
        (first, last)
    } else {
        (0, 0)
    };
    if i64::from(last) - i64::from(first) >= 4096 {
        return Vec::new();
    }
    (first..=last)
        .filter_map(|offset| {
            let shift = period.map_or_else(BigRational::zero, |value| {
                value * BigRational::from_integer(offset.into())
            });
            let low = lo.clone().max(start - &shift);
            let high = hi.clone().min(end - &shift);
            (low <= high).then_some(([low, high], shift))
        })
        .collect()
}

fn covers_contact_window(
    shared: &[&ss::OriginalSsRootEdge],
    sources: &ss::FiberSources,
    free: usize,
    window: [ExactClock; 4],
    intervals: Vec<ExactClock>,
    periods: [Option<&num_rational::BigRational>; 4],
) -> bool {
    covers_contact_window_with_clocks(shared, sources, free, window, intervals, periods, None)
}

fn covers_contact_window_with_clocks(
    shared: &[&ss::OriginalSsRootEdge],
    sources: &ss::FiberSources,
    free: usize,
    window: [ExactClock; 4],
    intervals: Vec<ExactClock>,
    periods: [Option<&num_rational::BigRational>; 4],
    clocks: Option<&[Option<ExactClock>]>,
) -> bool {
    let mut by_axis: [Vec<ExactClock>; 4] = std::array::from_fn(|_| Vec::new());
    by_axis[free] = intervals;
    let mut patches = Vec::new();
    for (index, edge) in shared.iter().enumerate() {
        if edge.curve().source().first() != sources.first()
            || edge.curve().source().second() != sources.second()
        {
            continue;
        }
        let axis = fiber_axis_index(edge.curve().free_axis());
        let clock = clocks.map_or_else(
            || edge.original_inner_clock().ok(),
            |clocks| clocks[index].clone(),
        );
        let (Some(clock), Ok(mut domain)) = (
            clock,
            edge.curve().original_graph_domain(),
        ) else {
            continue;
        };
        // Fit the full dependent box, then clip and translate the edge clock.
        domain[axis] = window[axis].clone();
        let mut dependent_periods = periods;
        dependent_periods[axis] = None;
        let nonperiodic = periods.iter().all(Option::is_none);
        let mut lifted = if nonperiodic {
            std::array::from_fn(|k| intersect_exact_clock(&window[k], &domain[k]))
        } else {
            let Some(lifted) = lift_contact_window(&window, &domain, dependent_periods) else {
                continue;
            };
            lifted
        };
        if lifted.iter().any(|range| range[0] > range[1]) {
            continue;
        }
        for (overlap, shift) in contact_clock_strips(&window[axis], &clock, periods[axis]) {
            lifted[axis] = overlap.each_ref().map(|bound| bound + &shift);
            if edge.curve().qualify_graph_window(lifted.clone()).is_ok() {
                if !nonperiodic || (0..4).all(|k| k == axis || lifted[k] == window[k]) {
                    by_axis[axis].push(overlap);
                } else if patches.len() < 64 {
                    patches.push(lifted.clone());
                }
            }
        }
    }
    // Each certified strip spans the other three coordinates of the window.
    // A gap-free union on any one axis therefore covers the entire window.
    for (axis, intervals) in by_axis.into_iter().enumerate() {
        if covers_clock(intervals.clone(), window[axis].clone()) {
            return true;
        }
        for interval in intervals.into_iter().take(64 - patches.len()) {
            let mut patch = window.clone();
            patch[axis] = intersect_exact_clock(&window[axis], &interval);
            if patch[axis][0] <= patch[axis][1] {
                patches.push(patch);
            }
        }
    }
    covers_contact_boxes(window, &patches)
}

fn intersect_exact_clock(a: &ExactClock, b: &ExactClock) -> ExactClock {
    [a[0].clone().max(b[0].clone()), a[1].clone().min(b[1].clone())]
}

fn covers_contact_boxes(window: [ExactClock; 4], patches: &[[ExactClock; 4]]) -> bool {
    let mut pending = vec![window];
    for _ in 0..128 {
        let Some(cell) = pending.pop() else { return true };
        let mut split = None;
        let mut covered = false;
        for patch in patches {
            if (0..4).any(|k| patch[k][1] < cell[k][0] || cell[k][1] < patch[k][0]) {
                continue;
            }
            if (0..4).all(|k| patch[k][0] <= cell[k][0] && cell[k][1] <= patch[k][1]) {
                covered = true;
                break;
            }
            if split.is_none() {
                split = (0..4).find_map(|axis| {
                    patch[axis].iter().find(|bound| cell[axis][0] < **bound && **bound < cell[axis][1])
                        .map(|bound| (axis, bound.clone()))
                });
            }
        }
        if !covered {
            let Some((axis, bound)) = split else { return false };
            let mut right = cell.clone();
            let mut left = cell;
            left[axis][1] = bound.clone();
            right[axis][0] = bound;
            pending.extend([right, left]);
        }
    }
    pending.is_empty()
}

fn covers_clock(
    mut intervals: Vec<[num_rational::BigRational; 2]>,
    [start, end]: [num_rational::BigRational; 2],
) -> bool {
    intervals.sort();
    let mut reached = start;
    for [low, high] in intervals {
        if low > reached {
            break;
        }
        if high >= end {
            return true;
        }
        reached = reached.max(high);
    }
    false
}

/// Pairwise contact evidence for a single sewn shell.
#[derive(Clone, Debug)]
pub struct QualifiedRootShellContacts {
    charts: QualifiedRootShellCharts,
    censuses: Vec<([usize; 2], ss::QualifiedQuotientCoverage)>,
    max_cells: usize,
    separated_pairs: usize,
    section_pairs: usize,
}

impl QualifiedRootShellCharts {
    fn qualify_periodic_section_contact(&self, faces: [usize; 2]) -> Result<()> {
        use num_rational::BigRational as Q;
        use num_traits::ToPrimitive;
        let topology = self.source.topology();
        let [a, b] = faces.map(|id| &topology.faces[id]);
        let sources = ss::FiberSources::new(&a.surface, &b.surface)?;
        let section = ss::qualify_periodic_plane_ss(&sources, Default::default())?;
        if section.kind() != ss::PeriodicPlaneSSKind::ClosedContour {
            return Err(unproved("Incident periodic faces need a complete regular section"));
        }
        let periodic_support = section.periodic_surface();
        let axis = section.periodic_axis();
        let surface = &topology.faces[faces[periodic_support]].surface;
        let bounds = crate::boolean_robustness::surface_uv_domains(surface)[axis]
            .map(|value| Q::from_float(value).unwrap());
        let period = &bounds[1] - &bounds[0];
        let edges = |face: &brep_topology::Face<Surface>, support| {
            std::iter::once(face.outer).chain(face.holes.iter().copied())
                .flat_map(|loop_id| &topology.loops[loop_id].coedges)
                .filter(|coedge| coedge.pcurve.support == support)
                .map(|coedge| coedge.edge)
                .collect::<std::collections::BTreeSet<_>>()
        };
        let mut clocks = Vec::new();
        for &id in edges(a, 0).intersection(&edges(b, 1)) {
            let edge = &topology.edges[id].curve;
            let original = edge.curve().source();
            if original.first() != &a.surface || original.second() != &b.surface
                || fiber_axis_index(edge.curve().free_axis()) != 2 * periodic_support + axis
            {
                return Err(unproved("Periodic section edge has a foreign carrier or clock"));
            }
            let [lo, hi] = edge.original_inner_clock()?;
            let min = ((&bounds[0] - &hi) / &period).ceil().to_i32()
                .ok_or_else(|| unproved("Periodic section clock shift overflow"))?;
            let max = ((&bounds[1] - &lo) / &period).floor().to_i32()
                .ok_or_else(|| unproved("Periodic section clock shift overflow"))?;
            if i64::from(max) - i64::from(min) > 4 {
                return Err(unproved("Periodic section edge crosses too many native periods"));
            }
            for shift in min..=max {
                let offset = &period * Q::from_integer(shift.into());
                clocks.push([&lo + &offset, &hi + &offset]);
            }
        }
        if !covers_clock(clocks, bounds) {
            return Err(unproved("Shared edges do not cover the complete periodic section"));
        }
        Ok(())
    }
}

impl QualifiedRootShellContacts {
    /// Account for each incident face pair through an SS census or a proved
    /// periodic plane section. Nonincident pairs must be separated.
    /// Census face indices follow the original surface order.
    pub fn qualify(
        charts: QualifiedRootShellCharts,
        censuses: Vec<([usize; 2], ss::QualifiedQuotientCoverage)>,
        max_cells: usize,
    ) -> Result<Self> {
        let topology = charts.source().topology();
        let mut supplied = std::collections::BTreeMap::new();
        for (index, (faces, _)) in censuses.iter().enumerate() {
            let mut pair = *faces;
            pair.sort_unstable();
            if pair[0] == pair[1]
                || pair[1] >= topology.faces.len()
                || supplied.insert(pair, index).is_some()
            {
                return Err(unproved("Invalid or duplicate shell contact census pair"));
            }
        }
        let mut incident = std::collections::BTreeSet::new();
        let mut owners = vec![Vec::new(); topology.vertices.len()];
        for (face_id, face) in topology.faces.iter().enumerate() {
            for id in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
                for vertex in topology.loops[id]
                    .coedges
                    .iter()
                    .flat_map(|coedge| topology.edges[coedge.edge].vertices)
                {
                    let adjacent = &mut owners[vertex];
                    for &other in adjacent.iter().filter(|&&other| other != face_id) {
                        incident.insert([other, face_id]);
                    }
                    if adjacent.last() != Some(&face_id) {
                        adjacent.push(face_id);
                    }
                }
            }
        }
        if supplied.keys().any(|pair| !incident.contains(pair)) {
            return Err(unproved("Shell contact census refers to a nonincident face pair"));
        }
        let mut section_pairs = 0;
        for pair in incident.iter().filter(|pair| !supplied.contains_key(*pair)) {
            if !pair.iter().any(|&index| {
                let s = &topology.faces[index].surface;
                s.periodic_u || s.periodic_v
            }) {
                return Err(unproved("Shell contact census must cover exactly the incident face pairs"));
            }
            if let Err(error) = charts.qualify_periodic_section_contact(*pair) {
                charts.qualify_periodic_section_contact([pair[1], pair[0]])
                    .map_err(|_| error)?;
            }
            section_pairs += 1;
        }
        let separated_pairs = charts.qualify_nonincident_separation(max_cells)?;
        for (faces, census) in &censuses {
            charts.qualify_shared_edge_contacts(*faces, census)?;
        }
        Ok(Self {
            charts,
            censuses,
            max_cells,
            separated_pairs,
            section_pairs,
        })
    }

    pub(super) fn charts(&self) -> &QualifiedRootShellCharts {
        &self.charts
    }

    pub fn separated_pairs(&self) -> usize {
        self.separated_pairs
    }

    pub fn incident_pairs(&self) -> usize {
        self.censuses.len() + self.section_pairs
    }

    pub fn reverify(&self, geometry: ss::OriginalSsCurveLimits) -> Result<()> {
        self.charts.reverify(geometry)?;
        for (_, census) in &self.censuses {
            census.reverify()?;
        }
        Self::qualify(self.charts.clone(), self.censuses.clone(), self.max_cells)?;
        Ok(())
    }
}
