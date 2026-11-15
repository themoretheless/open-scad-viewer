//! Interval flux over the native parameter domains of a sewn root shell.
use super::*;
use nurbs_core::interval_eval::Interval;
use std::{cmp::Ordering, collections::BinaryHeap};

#[derive(Clone, Copy, Debug)]
pub struct RootShellFluxLimits {
    pub axis: usize,
    pub origin: f64,
    pub max_cells: usize,
    pub max_spans: usize,
    pub max_depth: usize,
    pub winding: ss::OriginalSsWindingLimits,
}

#[derive(Clone, Debug)]
pub struct RootShellFluxReport {
    pub signed_bounds: [f64; 2],
    pub cells: usize,
    pub spans: usize,
}

impl QualifiedRootShellCharts {
    /// Bound oriented flux on the trimmed supports. Interpreting it as
    /// material volume requires a geometrically admitted boundary.
    pub fn signed_flux(&self, limits: RootShellFluxLimits) -> Result<RootShellFluxReport> {
        if limits.axis >= 3
            || !limits.origin.is_finite()
            || !(1..=65536).contains(&limits.max_cells)
            || !(1..=1_000_000).contains(&limits.max_spans)
            || limits.max_depth > 24
        {
            return Err(unproved("Invalid root shell flux limits"));
        }
        let source = self.source();
        let topology = source.topology();
        let mut work = FluxWork {
            limits,
            cells: 0,
            spans: 0,
        };
        let mut domains = Vec::new();
        let mut pending = BinaryHeap::new();
        let mut tree = Vec::new();
        let mut roots = Vec::new();
        for face_use in &topology.shells[0].faces {
            let face = &topology.faces[face_use.face];
            let surface = &face.surface;
            let native = [
                [
                    surface.knots_u[surface.degree_u],
                    surface.knots_u[surface.control_points.len()],
                ],
                [
                    surface.knots_v[surface.degree_v],
                    surface.knots_v[surface.control_points[0].len()],
                ],
            ];
            // Positive rational weights preserve an exactly constant
            // coordinate of the control net across the entire surface.
            if surface
                .control_points
                .iter()
                .flatten()
                .all(|point| point[limits.axis] == limits.origin)
            {
                continue;
            }
            let domain = Arc::new(QualifiedRootPeriodicFace::qualify(
                source.clone(),
                face_use.face,
                limits.winding,
            )?);
            let ranges = native_flux_cover(
                self.face_bounds(face_use.face),
                native,
                &winding::face_periods(surface),
                [surface.periodic_u, surface.periodic_v],
            )?;
            let face_index = domains.len();
            domains.push((surface, domain, face_use.reversed));
            let (surface, domain, reversed) = &domains[face_index];
            for &u in &ranges[0] {
                for &v in &ranges[1] {
                    let cell = work
                        .evaluate(surface, domain, [u, v], face_index, 0, *reversed)?
                        .ok_or_else(|| unproved("Initial root shell flux cover exceeds budget"))?;
                    roots.push(push_flux_node(&mut tree, &mut pending, cell, None));
                }
            }
        }
        while work.cells < limits.max_cells {
            let Some(priority) = pending.pop() else { break };
            let index = priority.index;
            let parent = &tree[index].cell;
            let Some(rectangles) = parent.split(limits.max_depth) else {
                continue;
            };
            let (surface, domain, reversed) = &domains[parent.face];
            let mut children = Vec::new();
            let mut evaluated = false;
            for rectangle in rectangles {
                let child = work.evaluate(
                    surface,
                    domain,
                    rectangle,
                    parent.face,
                    parent.depth + 1,
                    *reversed,
                )?;
                evaluated |= child.is_some();
                children.push(match child {
                    Some(child) => child,
                    None => parent.enclose_child(rectangle)?,
                });
            }
            if !evaluated {
                continue;
            }
            let [left, right]: [FluxCell; 2] = children
                .try_into()
                .map_err(|_| unproved("Incomplete flux subdivision"))?;
            let left = push_flux_node(&mut tree, &mut pending, left, Some(index));
            let right = push_flux_node(&mut tree, &mut pending, right, Some(index));
            tree[index].children = Some([left, right]);
            refine_flux_ancestors(&mut tree, index)?;
        }
        let sum = roots
            .into_iter()
            .try_fold(Interval::point(0.), |sum, index| {
                sum.add(tree[index].cell.bound)
            })?;
        Ok(RootShellFluxReport {
            signed_bounds: [sum.lo, sum.hi],
            cells: work.cells,
            spans: work.spans,
        })
    }
}

fn native_flux_cover(
    bounds: [[f64; 2]; 2],
    native: [[f64; 2]; 2],
    periods: &[num_rational::BigRational; 2],
    periodic: [bool; 2],
) -> Result<[Vec<[f64; 2]>; 2]> {
    let offsets = face_region::overlapping_period_offsets(bounds, native, periods, periodic)?;
    let mut cover = [Vec::new(), Vec::new()];
    for axis in 0..2 {
        let [first, last] = offsets[axis];
        if i64::from(last) - i64::from(first) >= 4096 {
            return Err(unproved("Root flux period cover budget exhausted"));
        }
        for offset in first..=last {
            let exact = |value| num_rational::BigRational::from_float(value).unwrap();
            let shift = &periods[axis] * num_rational::BigRational::from_integer(offset.into());
            let lo =
                winding::outward(&(exact(bounds[axis][0]) + &shift), true)?.max(native[axis][0]);
            let hi =
                winding::outward(&(exact(bounds[axis][1]) + shift), false)?.min(native[axis][1]);
            if lo >= hi {
                continue;
            }
            // Shifts are ordered. Merge overlap so no positive-area cell
            // is integrated twice, including overlap caused by rounding.
            if let Some(previous) = cover[axis]
                .last_mut()
                .filter(|previous: &&mut [f64; 2]| lo <= previous[1])
            {
                previous[1] = previous[1].max(hi);
            } else {
                cover[axis].push([lo, hi]);
            }
        }
    }
    if cover.iter().any(Vec::is_empty) {
        return Err(unproved("Empty root shell flux cover"));
    }
    Ok(cover)
}

struct FluxCell {
    face: usize,
    rectangle: [[f64; 2]; 2],
    depth: usize,
    bound: Interval,
    density: Interval,
}

impl FluxCell {
    fn enclose_child(&self, rectangle: [[f64; 2]; 2]) -> Result<Self> {
        Ok(Self {
            face: self.face,
            rectangle,
            depth: self.depth + 1,
            bound: self.density.mul(rectangle_area(rectangle)?)?,
            density: self.density,
        })
    }

    fn width(&self) -> f64 {
        self.bound.hi - self.bound.lo
    }

    fn split(&self, max_depth: usize) -> Option<[[[f64; 2]; 2]; 2]> {
        if self.depth >= max_depth || self.width() == 0. {
            return None;
        }
        let mut left = self.rectangle;
        let mut right = self.rectangle;
        let axis = usize::from(left[1][1] - left[1][0] > left[0][1] - left[0][0]);
        let [lo, hi] = left[axis];
        let mid = lo * 0.5 + hi * 0.5;
        if !(lo < mid && mid < hi) {
            return None;
        }
        left[axis][1] = mid;
        right[axis][0] = mid;
        Some([left, right])
    }
}

struct FluxNode {
    cell: FluxCell,
    parent: Option<usize>,
    children: Option<[usize; 2]>,
}

#[derive(Clone, Copy)]
struct FluxPriority {
    width: f64,
    index: usize,
}

impl PartialEq for FluxPriority {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}
impl Eq for FluxPriority {}
impl PartialOrd for FluxPriority {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for FluxPriority {
    fn cmp(&self, other: &Self) -> Ordering {
        self.width
            .total_cmp(&other.width)
            .then_with(|| other.index.cmp(&self.index))
    }
}

fn push_flux_node(
    tree: &mut Vec<FluxNode>,
    pending: &mut BinaryHeap<FluxPriority>,
    cell: FluxCell,
    parent: Option<usize>,
) -> usize {
    let index = tree.len();
    pending.push(FluxPriority {
        width: cell.width(),
        index,
    });
    tree.push(FluxNode {
        cell,
        parent,
        children: None,
    });
    index
}

fn refine_flux_ancestors(tree: &mut [FluxNode], index: usize) -> Result<()> {
    let mut current = Some(index);
    while let Some(index) = current {
        let [left, right] = tree[index]
            .children
            .ok_or_else(|| unproved("Missing flux subdivision children"))?;
        let refined = tree[left].cell.bound.add(tree[right].cell.bound)?;
        let old = tree[index].cell.bound;
        tree[index].cell.bound = Interval::new(old.lo.max(refined.lo), old.hi.min(refined.hi))?;
        current = tree[index].parent;
    }
    Ok(())
}

struct FluxWork {
    limits: RootShellFluxLimits,
    cells: usize,
    spans: usize,
}

impl FluxWork {
    fn evaluate(
        &mut self,
        surface: &Surface,
        domain: &Arc<QualifiedRootPeriodicFace>,
        rectangle: [[f64; 2]; 2],
        face: usize,
        depth: usize,
        reversed: bool,
    ) -> Result<Option<FluxCell>> {
        if self.cells == self.limits.max_cells {
            return Ok(None);
        }
        self.cells += 1;
        let classification = domain.classify_box(rectangle).ok();
        let region = classification.as_ref().map(|proof| proof.region());
        let mut density = Interval::point(0.);
        let interval = match region {
            Some(RootFaceRegion::OutsideOuter | RootFaceRegion::ExcludedByHoles(_)) => {
                Interval::point(0.)
            }
            _ => {
                let remaining = self.limits.max_spans - self.spans;
                if remaining == 0 {
                    return Ok(None);
                }
                let report = nurbs_core::surface_flux::bound(
                    surface,
                    rectangle,
                    self.limits.axis,
                    self.limits.origin,
                    remaining.min(100000),
                )?;
                self.spans += report.spans;
                let Some(bound) = report.bound else {
                    return Ok(None);
                };
                let inside = matches!(region, Some(RootFaceRegion::InsideSupport));
                let [lo, hi] = bound.density;
                density = if inside {
                    Interval::new(lo, hi)?
                } else {
                    Interval::new(lo.min(0.), hi.max(0.))?
                };
                if inside {
                    Interval::new(bound.integral[0], bound.integral[1])?
                } else if bound.density == [0., 0.] {
                    Interval::point(0.)
                } else {
                    masked_flux(bound.density, bound.integral, rectangle_area(rectangle)?)?
                }
            }
        };
        let bound = if reversed {
            density = Interval::new(-density.hi, -density.lo)?;
            Interval::new(-interval.hi, -interval.lo)?
        } else {
            interval
        };
        Ok(Some(FluxCell {
            face,
            rectangle,
            depth,
            bound,
            density,
        }))
    }
}

fn rectangle_area(rectangle: [[f64; 2]; 2]) -> Result<Interval> {
    let width =
        |axis: usize| Interval::point(rectangle[axis][1]).sub(Interval::point(rectangle[axis][0]));
    Ok(width(0)?.mul(width(1)?)?)
}

/// Bound the integral on any measurable subset by enclosing the integrals
/// of the negative and positive parts. Their secants over the density range
/// are affine, so they can use the independently bounded full-cell integral.
fn masked_flux(density: [f64; 2], integral: [f64; 2], area: Interval) -> Result<Interval> {
    let [lo, hi] = density;
    let direct = Interval::new(lo.min(0.), hi.max(0.))?.mul(area)?;
    let full = Interval::new(integral[0], integral[1])?;
    let from_complement = full.sub(direct)?;
    let mut lower = direct.lo.max(from_complement.lo);
    let mut upper = direct.hi.min(from_complement.hi);
    if lo < 0. && hi > 0. {
        let lo = Interval::point(lo);
        let hi = Interval::point(hi);
        let range = hi.sub(lo)?;
        let positive = full.sub(lo.mul(area)?)?.mul(hi.div(range)?)?;
        let negative = hi.mul(area)?.sub(full)?.mul(lo.div(range)?)?;
        lower = lower.max(negative.lo);
        upper = upper.min(positive.hi);
    }
    Ok(Interval::new(lower, upper)?)
}

/// A shell contact audit and a strictly positive oriented flux bound.
#[derive(Clone, Debug)]
pub struct QualifiedRootShellPositiveFlux {
    contacts: QualifiedRootShellContacts,
    limits: RootShellFluxLimits,
    report: RootShellFluxReport,
}

impl QualifiedRootShellPositiveFlux {
    pub fn qualify(
        contacts: QualifiedRootShellContacts,
        limits: RootShellFluxLimits,
    ) -> Result<Self> {
        let report = contacts.charts().signed_flux(limits)?;
        if !report.has_positive_lower_bound() {
            return Err(unproved(
                "Root shell signed flux lacks a strictly positive lower bound",
            ));
        }
        Ok(Self {
            contacts,
            limits,
            report,
        })
    }

    pub fn report(&self) -> &RootShellFluxReport {
        &self.report
    }

    pub fn source(&self) -> &Arc<RootBrep> {
        self.contacts.charts().source()
    }

    /// Prove that no pair of trimmed faces on these two shells can touch.
    /// The test uses a cover by UV rectangles and may conservatively fail.
    /// Interior containment remains possible after boundary separation.
    pub fn prove_boundary_separation(&self, other: &Self, max_cells: usize) -> Result<()> {
        self.contacts.charts().prove_cross_shell_separation(other.contacts.charts(), max_cells)
    }

    /// Determine containment after proving that the two boundaries are disjoint.
    pub fn classify_shell_relation(
        &self,
        other: &Self,
        direction: [f64; 3],
        max_separation_cells: usize,
        limits: crate::mixed_root_model::RootSolidPointLimits,
    ) -> Result<crate::mixed_root_model::RootRelationKind> {
        self.classify_shell_relation_with_rays(
            other, &[direction], max_separation_cells, limits,
        )
    }

    /// Retry each witness with the supplied directions under divided work limits.
    pub fn classify_shell_relation_with_rays(
        &self,
        other: &Self,
        directions: &[[f64; 3]],
        max_separation_cells: usize,
        limits: crate::mixed_root_model::RootSolidPointLimits,
    ) -> Result<crate::mixed_root_model::RootRelationKind> {
        use crate::mixed_root_model::RootRelationKind;
        let count = directions.len();
        if !(1..=16).contains(&count)
            || limits.ray_cells < count
            || limits.domain_queries < count
            || directions.iter().any(|ray| {
                !ray.iter().all(|x| x.is_finite()) || ray.iter().all(|x| *x == 0.)
            })
        {
            return Err(unproved("Invalid root shell witness rays or work limits"));
        }
        self.prove_boundary_separation(other, max_separation_cells)?;
        let inside = |source: &Self, target: &Self| -> Result<bool> {
            let vertex = source.source().topology().vertices.first()
                .ok_or_else(|| unproved("Root shell has no boundary witness"))?;
            let world = vertex.point.selector.enclose(limits.geometry)?.world;
            target.contacts.charts().exclude_world_box(world, max_separation_cells)?;
            // The lower corner is in the same component of the target
            // complement as the exact source vertex.
            let point = world.map(|bounds| bounds[0]);
            for &direction in directions {
                if let Some(parity) = target.classify_point(
                    point, direction, limits.tolerance_uv,
                    limits.ray_cells / count, limits.domain_queries / count,
                )? {
                    return Ok(parity);
                }
            }
            Err(unproved("Root shell witness parity remains unresolved"))
        };
        match (inside(self, other)?, inside(other, self)?) {
            (false, false) => Ok(RootRelationKind::Disjoint),
            (true, false) => Ok(RootRelationKind::FirstInsideSecond),
            (false, true) => Ok(RootRelationKind::SecondInsideFirst),
            (true, true) => Err(unproved("Inconsistent root shell containment")),
        }
    }

    /// Classify a point using a complete ray and the exact trimmed UV domains.
    /// An unresolved ray (including a boundary hit) yields no material value.
    pub fn classify_point(
        &self,
        point: [f64; 3],
        direction: [f64; 3],
        tolerance_uv: f64,
        ray_cells: usize,
        domain_queries: usize,
    ) -> Result<Option<bool>> {
        use nurbs_core::trim_domain::Location;
        let surfaces: Vec<_> = self.source().topology().faces.iter().map(|face| &face.surface).collect();
        let report = crate::ray_parity::classify_surface_ray(
            &surfaces, point, direction, tolerance_uv, ray_cells, domain_queries,
            |face, uv, _| {
                let location = match self.classify_face_box(face, uv) {
                    Ok(RootFaceRegion::InsideSupport) => Location::Inside,
                    Ok(RootFaceRegion::OutsideOuter | RootFaceRegion::ExcludedByHoles(_)) => Location::Outside,
                    _ => Location::Unresolved,
                };
                Ok((location, 1))
            },
        )?;
        Ok(report.parity)
    }

    /// A conservative parameter box covering the trimmed face.
    pub(crate) fn face_bounds(&self, face: usize) -> [[f64; 2]; 2] {
        self.contacts.charts().face_bounds(face)
    }

    pub(crate) fn classify_face_box(
        &self,
        face: usize,
        domain: [[f64; 2]; 2],
    ) -> Result<RootFaceRegion> {
        Ok(self.contacts.charts().trim_domain(face).classify_box(domain)?.region().clone())
    }

    /// Materialize the certified shell as a single outer boundary of a body.
    pub fn to_body(&self) -> Result<RootBrep> {
        let mut topology = self.source().topology().clone();
        topology.bodies = vec![brep_topology::Body {
            outer_shell: 0,
            inner_shells: Vec::new(),
        }];
        RootBrep::from_topology(topology)
    }

    pub fn reverify(&self) -> Result<()> {
        self.contacts.reverify(self.limits.winding.geometry)?;
        let report = self.contacts.charts().signed_flux(self.limits)?;
        if !report.has_positive_lower_bound()
            || report.signed_bounds != self.report.signed_bounds
            || report.cells != self.report.cells
            || report.spans != self.report.spans
        {
            return Err(unproved("Root shell positive flux proof changed"));
        }
        Ok(())
    }
}

impl RootShellFluxReport {
    pub fn has_positive_lower_bound(&self) -> bool {
        let [lo, hi] = self.signed_bounds;
        lo.is_finite() && hi.is_finite() && lo > 0. && lo <= hi
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masked_flux_uses_full_cell_integral_for_both_orientations() {
        // Density varies from -1 to 3, but integrates to 1 on the unit cell.
        // The extremal distribution has half the cell at each endpoint.
        // Selecting just one half attains -0.5 or 1.5.
        let bound = masked_flux([-1., 3.], [1., 1.], Interval::point(1.)).unwrap();
        assert!(bound.lo <= -0.5 && bound.lo > -0.500001);
        assert!(bound.hi >= 1.5 && bound.hi < 1.500001);

        let reversed = masked_flux([-3., 1.], [-1., -1.], Interval::point(1.)).unwrap();
        assert!(reversed.lo <= -1.5 && reversed.lo > -1.500001);
        assert!(reversed.hi >= 0.5 && reversed.hi < 0.500001);
    }

    #[test]
    fn unevaluated_child_uses_parent_density_and_its_own_area() {
        let parent = FluxCell {
            face: 0,
            rectangle: [[0., 2.], [0., 1.]],
            depth: 0,
            bound: Interval::new(-2., 6.).unwrap(),
            density: Interval::new(-1., 3.).unwrap(),
        };
        let [left, right] = parent.split(1).unwrap();
        let fallback = parent.enclose_child(right).unwrap();
        assert_eq!(fallback.rectangle, [[1., 2.], [0., 1.]]);
        assert_eq!(left, [[0., 1.], [0., 1.]]);
        let combined = fallback.bound.add(Interval::point(0.5)).unwrap();
        assert!(combined.lo <= -0.5 && combined.lo > parent.bound.lo);
        assert!(combined.hi >= 3.5 && combined.hi < parent.bound.hi);
    }

    #[test]
    fn flux_tree_keeps_parent_proof_and_uses_later_improvement() {
        let mut tree = Vec::new();
        let mut queue = BinaryHeap::new();
        let cell = |lo, hi| FluxCell {
            face: 0,
            rectangle: [[0., 1.]; 2],
            depth: 0,
            bound: Interval::new(lo, hi).unwrap(),
            density: Interval::new(lo, hi).unwrap(),
        };
        let root = push_flux_node(&mut tree, &mut queue, cell(0., 1.), None);
        let left = push_flux_node(&mut tree, &mut queue, cell(-0.25, 0.75), Some(root));
        let right = push_flux_node(&mut tree, &mut queue, cell(-0.25, 0.75), Some(root));
        tree[root].children = Some([left, right]);
        refine_flux_ancestors(&mut tree, root).unwrap();
        assert_eq!(tree[root].cell.bound.lo, 0.);
        assert_eq!(tree[root].cell.bound.hi, 1.);
        let a = push_flux_node(&mut tree, &mut queue, cell(0.125, 0.125), Some(left));
        let b = push_flux_node(&mut tree, &mut queue, cell(0.125, 0.125), Some(left));
        tree[left].children = Some([a, b]);
        tree[right].cell.bound = Interval::point(0.25);
        refine_flux_ancestors(&mut tree, left).unwrap();
        assert!(tree[root].cell.bound.lo > 0.49);
        assert!(tree[root].cell.bound.hi < 0.51);
    }

    #[test]
    fn native_flux_cover_splits_at_seam_and_merges_overlap() {
        let periods = std::array::from_fn(|_| num_rational::BigRational::from_integer(1.into()));
        let native = [[0., 1.]; 2];
        assert_eq!(
            native_flux_cover(
                [[0.25, 0.75], [-0.125, 0.125]],
                native,
                &periods,
                [false, true]
            )
            .unwrap(),
            [vec![[0.25, 0.75]], vec![[0., 0.125], [0.875, 1.]]]
        );
        assert_eq!(
            native_flux_cover(
                [[0.25, 0.75], [-0.125, 1.125]],
                native,
                &periods,
                [false, true]
            )
            .unwrap(),
            [vec![[0.25, 0.75]], vec![[0., 1.]]]
        );
    }

    #[test]
    fn positive_flux_requires_a_finite_strict_lower_bound() {
        let accepts = |signed_bounds| {
            RootShellFluxReport {
                signed_bounds,
                cells: 1,
                spans: 1,
            }
            .has_positive_lower_bound()
        };
        assert!(accepts([0.01, 0.02]));
        assert!(!accepts([0., 0.02]));
        assert!(!accepts([-0.02, -0.01]));
        assert!(!accepts([0.01, f64::INFINITY]));
        assert!(!accepts([0.02, 0.01]));
    }
}
