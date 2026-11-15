//! Geometric chart checks for a sewn original-root shell.
use super::*;

#[derive(Clone, Debug)]
pub struct QualifiedRootShellCharts {
    source: Arc<RootBrep>,
    max_spans: usize,
    bounds: Vec<[[f64; 2]; 2]>,
    trim_domains: Vec<Arc<QualifiedRootPeriodicFace>>,
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
        QualifiedRootPeriodicUvClosure::qualify(
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
            let bounds = winding::lifted_loop_bounds(&source, index, face.outer, &mut remaining)?;
            let report =
                nurbs_core::surface_injectivity::certify_on(&face.surface, bounds, max_spans)?;
            if !report.proven {
                return Err(unproved(
                    "Root shell face carrier injectivity remains unproved",
                ));
            }
            rectangles.push(bounds);
            trim_domains.push(Arc::new(QualifiedRootPeriodicFace::qualify(
                source.clone(),
                index,
                ss::OriginalSsWindingLimits {
                    geometry,
                    ..Default::default()
                },
            )?));
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

    pub(super) fn trim_domain(&self, face: usize) -> &Arc<QualifiedRootPeriodicFace> {
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
        for id in std::iter::once(data.outer).chain(data.holes.iter().copied()) {
            audit_trim_arc_separation(self, face, id, remaining)?;
        }
        Ok(())
    }
}

fn audit_trim_arc_separation(
    source: &RootBrep,
    face_id: usize,
    loop_id: usize,
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
    let lift = uv_closure::qualify_loop_lift(source, face_id, loop_id, false)?;
    if lift.winding != [0, 0] {
        return Err(unproved("Root trim separation requires a closed UV lift"));
    }
    let arcs = arcs
        .iter()
        .zip(lift.coedge_offsets)
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
        .collect::<Result<Vec<_>>>()?;
    for a in 0..arcs.len() {
        for b in a..arcs.len() {
            let incident = b <= a + 1 || (a == 0 && b == arcs.len() - 1);
            if incident && !has_period {
                continue;
            }
            remaining.interval_operations = remaining
                .interval_operations
                .checked_sub(1)
                .ok_or_else(|| unproved("Root trim arc pair budget exhausted"))?;
            let pair = [&arcs[a], &arcs[b]];
            let offsets =
                face_region::overlapping_period_offsets(pair[1].3, pair[0].3, &periods, periodic)?;
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
            })?;
        }
    }
    Ok(())
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
mod tests {
    use super::*;

    #[test]
    fn pinched_nonperiodic_trim_repeats_a_vertex() {
        assert!(require_distinct_trim_vertices([0, 1, 2, 3]).is_ok());
        assert!(require_distinct_trim_vertices([0, 1, 2, 0, 3, 4]).is_err());
    }

    #[test]
    fn census_clock_images_preserve_both_shore_aliases() {
        let q = |a: i32, b: i32| num_rational::BigRational::new(a.into(), b.into());
        let period = q(1, 1);
        assert_eq!(
            intersect_clock_images([0., 1.], [-0.125, 0.125], Some(&period)).unwrap(),
            vec![[q(-1, 8), q(0, 1)], [q(0, 1), q(1, 8)]]
        );
        assert_eq!(
            intersect_clock_images([1., 1.], [-0.125, 0.125], Some(&period)).unwrap(),
            vec![[q(0, 1), q(0, 1)]]
        );
        assert!(
            intersect_clock_images([0.25, 0.5], [-0.125, 0.125], Some(&period))
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn contact_window_accepts_a_different_free_axis_only_for_full_coverage() {
        let mut a =
            nurbs_core::polynomial::graph([0., 1., 0., 1.], &[vec![0., 0.], vec![0., 0.]]).unwrap();
        assert_eq!([a.degree_u, a.degree_v], [1, 1]);
        let mut b = a.clone();
        for point in b.control_points.iter_mut().flatten() {
            point[2] = point[1] - 0.5;
            point[1] = 0.5;
        }
        // Non-unit native clocks distinguish the two U coordinates.
        a.knots_u = vec![0., 0., 2., 2.];
        let sources = ss::FiberSources::new(&a, &b).unwrap();
        let fiber = sources
            .certify(
                [[0.25, 1.75], [0.25, 0.75], [0.25, 0.75], [0.25, 0.75]],
                ss::FiberAxis::SecondU,
                Default::default(),
            )
            .unwrap()
            .into_fiber()
            .unwrap();
        let curve = Arc::new(ss::OriginalSsRootCurve::from_native(fiber));
        let port = |clock| {
            let point = Arc::new(ss::OriginalSsRootPoint::new(curve.clone(), clock).unwrap());
            ss::bind_original_ss_port(point, curve.clone(), [0; 4], Default::default()).unwrap()
        };
        let edge = ss::OriginalSsRootEdge::new(port(0.375), port(0.625)).unwrap();
        let left = ss::OriginalSsRootEdge::new(port(0.375), port(0.5)).unwrap();
        let right = ss::OriginalSsRootEdge::new(port(0.5), port(0.625)).unwrap();
        let retraced = ss::OriginalSsRootEdge::new(port(0.5), port(0.4375)).unwrap();
        assert!(adjacent_trim_arcs_overlap(&left, &retraced, Default::default()).unwrap());
        assert!(!adjacent_trim_arcs_overlap(&left, &right, Default::default()).unwrap());
        let alternate = sources
            .certify(
                [[0.125, 1.875], [0.125, 0.875], [0.25, 0.75], [0.125, 0.875]],
                ss::FiberAxis::SecondU,
                Default::default(),
            )
            .unwrap()
            .into_fiber()
            .unwrap();
        let alternate = Arc::new(ss::OriginalSsRootCurve::from_native(alternate));
        let alternate_port = |clock| {
            let point = Arc::new(ss::OriginalSsRootPoint::new(alternate.clone(), clock).unwrap());
            ss::bind_original_ss_port(point, alternate.clone(), [0; 4], Default::default()).unwrap()
        };
        let retraced_other_chart =
            ss::OriginalSsRootEdge::new(alternate_port(0.5), alternate_port(0.4375)).unwrap();
        assert!(!left.curve().same_definition(&alternate).unwrap());
        assert!(
            adjacent_trim_arcs_overlap(&left, &retraced_other_chart, Default::default()).unwrap()
        );
        let different_axis = sources
            .certify(
                [[0.75, 1.25], [0.25, 0.75], [0.25, 0.75], [0.25, 0.75]],
                ss::FiberAxis::FirstU,
                Default::default(),
            )
            .unwrap()
            .into_fiber()
            .unwrap();
        let different_axis = Arc::new(ss::OriginalSsRootCurve::from_native(different_axis));
        let first_u_port = |clock| {
            let point =
                Arc::new(ss::OriginalSsRootPoint::new(different_axis.clone(), clock).unwrap());
            ss::bind_original_ss_port(point, different_axis.clone(), [0; 4], Default::default())
                .unwrap()
        };
        let reverse_first_u =
            ss::OriginalSsRootEdge::new(first_u_port(1.), first_u_port(0.75)).unwrap();
        let forward_first_u =
            ss::OriginalSsRootEdge::new(first_u_port(1.), first_u_port(1.25)).unwrap();
        assert!(adjacent_trim_arcs_overlap(&left, &reverse_first_u, Default::default()).unwrap());
        assert!(!adjacent_trim_arcs_overlap(&left, &forward_first_u, Default::default()).unwrap());
        let short_retrace =
            ss::OriginalSsRootEdge::new(first_u_port(1.), first_u_port(0.984375)).unwrap();
        assert!(adjacent_trim_arcs_overlap(&left, &short_retrace, Default::default()).unwrap());
        // Two opposed uses of the same segment share both endpoints.
        let return_edge = ss::OriginalSsRootEdge::new(port(0.5), port(0.375)).unwrap();
        assert!(adjacent_trim_arcs_overlap(&left, &return_edge, Default::default()).unwrap());
        assert!(adjacent_trim_arcs_overlap(&return_edge, &left, Default::default()).unwrap());
        let gapped = ss::OriginalSsRootEdge::new(port(0.53125), port(0.625)).unwrap();
        let window = |lo, hi| {
            [[0.75, 1.25], [0.5; 2], [lo, hi], [0.5; 2]].map(|bounds| {
                bounds.map(|value| num_rational::BigRational::from_float(value).unwrap())
            })
        };
        assert!(covers_contact_window(
            &[&edge],
            &sources,
            0,
            window(0.375, 0.625),
            vec![],
            [None; 4]
        ));
        assert!(!covers_contact_window(
            &[&edge],
            &sources,
            0,
            window(0.3125, 0.625),
            vec![],
            [None; 4]
        ));
        assert!(covers_contact_window(
            &[&left, &right],
            &sources,
            0,
            window(0.375, 0.625),
            vec![],
            [None; 4],
        ));
        assert!(!covers_contact_window(
            &[&left, &gapped],
            &sources,
            0,
            window(0.375, 0.625),
            vec![],
            [None; 4],
        ));

        let chart = sources
            .certify_quotient(
                [[0.25, 1.75], [0.25, 0.75], [0.25, 0.75], [0.25, 0.75]],
                ss::FiberAxis::SecondU,
                Default::default(),
            )
            .unwrap()
            .into_fiber()
            .unwrap();
        let mut remaining = 8;
        let outside = [[0., 2.], [0.25, 0.75], [0.375, 0.625], [0.25, 0.75]];
        assert!(!refine_contact_clock(&[&edge], &chart, outside, &mut remaining, &Default::default(), |_| false).unwrap());
        assert_eq!(remaining, 8);
    }

    #[test]
    fn periodic_clock_strips_join_in_the_census_chart() {
        let q = |n: i32| num_rational::BigRational::new(n.into(), 8.into());
        let period = q(8);
        let window = [q(7), q(9)];
        let left = contact_clock_strips(&window, &[q(7), q(8)], Some(&period));
        let right = contact_clock_strips(&window, &[q(0), q(1)], Some(&period));
        assert!(left.contains(&([q(7), q(8)], q(0))));
        assert!(right.contains(&([q(8), q(9)], q(-8))));
        assert!(covers_clock(
            left.into_iter()
                .chain(right)
                .map(|(range, _)| range)
                .collect(),
            window
        ));
    }

    #[test]
    fn contact_window_lift_requires_one_shift_for_the_entire_interval() {
        let q = |n: i32| num_rational::BigRational::new(n.into(), 8.into());
        let period = q(8);
        let mut window = std::array::from_fn(|_| [q(0), q(1)]);
        let mut target = window.clone();
        window[1] = [q(7), q(9)];
        target[1] = [q(-1), q(1)];
        let periods = [None, Some(&period), None, None];
        assert_eq!(
            lift_contact_window(&window, &target, periods),
            Some(target.clone())
        );
        target[1] = [q(0), q(1)];
        assert!(lift_contact_window(&window, &target, periods).is_none());
    }

    #[test]
    fn contact_leaf_clips_nonperiodic_bounds_and_preserves_periodic_aliases() {
        let period = num_rational::BigRational::from_integer(1.into());
        let faces = [[[0.25, 0.75], [-0.125, 0.125]], [[0., 1.]; 2]];
        let mut root = [[0., 1.], [0.875, 1.], [0., 1.], [1., 2.]];
        assert!(
            clip_contact_to_face_bounds(&mut root, faces, [None, Some(&period), None, None])
                .unwrap()
        );
        assert_eq!(root, [[0.25, 0.75], [0.875, 1.], [0., 1.], [1., 1.]]);
        root[1] = [0.25, 0.5];
        assert!(
            !clip_contact_to_face_bounds(&mut root, faces, [None, Some(&period), None, None])
                .unwrap()
        );
    }

    #[test]
    fn contact_boxes_cover_a_window_with_different_split_axes() {
        let q = |n: i32| num_rational::BigRational::from_integer(n.into());
        let window = std::array::from_fn(|_| [q(0), q(2)]);
        let mut left = window.clone();
        left[0][1] = q(1);
        let mut lower_right = window.clone();
        lower_right[0][0] = q(1);
        lower_right[1][1] = q(1);
        let mut upper_right = lower_right.clone();
        upper_right[1] = [q(1), q(2)];
        assert!(!covers_contact_boxes(window.clone(), &[left.clone(), lower_right.clone()]));
        assert!(covers_contact_boxes(window, &[left, lower_right, upper_right]));
    }

    #[test]
    fn shared_clock_coverage_requires_endpoints_and_no_gaps() {
        let q = |n: i32| num_rational::BigRational::from_integer(n.into());
        assert!(covers_clock(vec![[q(1), q(2)], [q(0), q(1)]], [q(0), q(2)]));
        assert!(!covers_clock(
            vec![[q(0), q(1)], [q(2), q(3)]],
            [q(0), q(3)]
        ));
        assert!(!covers_clock(vec![[q(1), q(3)]], [q(0), q(3)]));
        assert!(!covers_clock(vec![[q(0), q(2)]], [q(0), q(3)]));
    }

    #[test]
    fn chart_separation_subdivides_overlapping_world_enclosures() {
        let plane = |shift: f64| Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: [0., 1.]
                .map(|u| [0., 1.].map(|v| vec![u, v, u + shift]).to_vec())
                .to_vec(),
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let a = plane(0.);
        let b = plane(0.5);
        let rectangles = [[[0., 1.]; 2]; 2];
        separate_charts([&a, &b], rectangles, &mut 4096).unwrap();
        assert!(separate_charts([&a, &b], rectangles, &mut 1).is_err());
        assert!(separate_charts([&a, &a], rectangles, &mut 128).is_err());
    }
}

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
        for node in coverage.nodes() {
            let mut root = node.domain();
            let chart = match node.kind() {
                Some(ss::QuotientCoverageKind::Chart(chart)) => *chart,
                Some(ss::QuotientCoverageKind::RootImage {
                    chart,
                    axis,
                    report,
                }) => {
                    let image = report
                        .root_image
                        .ok_or_else(|| unproved("Missing census root enclosure"))?;
                    let free = fiber_axis_index(*axis);
                    for (bounds, image) in root
                        .iter_mut()
                        .enumerate()
                        .filter_map(|(axis, bounds)| (axis != free).then_some(bounds))
                        .zip(image)
                    {
                        bounds[0] = bounds[0].max(image[0]);
                        bounds[1] = bounds[1].min(image[1]);
                    }
                    *chart
                }
                Some(
                    ss::QuotientCoverageKind::Excluded(_) | ss::QuotientCoverageKind::Split { .. },
                ) => continue,
                None => return Err(unproved("Incomplete shared edge contact census")),
            };
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
                    self.trim_domain(face).classify_box(query).is_ok_and(|proof| {
                        matches!(
                            proof.region(),
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
}

impl QualifiedRootShellContacts {
    /// Require a census for each incident face pair and prove separation for
    /// every other pair. Census face indices follow its original surface order.
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
        if !incident.iter().copied().eq(supplied.keys().copied()) {
            return Err(unproved(
                "Shell contact census must cover exactly the incident face pairs",
            ));
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
        })
    }

    pub(super) fn charts(&self) -> &QualifiedRootShellCharts {
        &self.charts
    }

    pub fn separated_pairs(&self) -> usize {
        self.separated_pairs
    }

    pub fn incident_pairs(&self) -> usize {
        self.censuses.len()
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
