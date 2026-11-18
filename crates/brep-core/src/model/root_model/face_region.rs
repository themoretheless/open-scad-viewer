//! Whole-box membership in an explicitly authored outer-minus-holes UV domain.
use super::*;
use num_rational::BigRational;
use num_traits::ToPrimitive;
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RootFaceRegion {
    OutsideOuter,
    ExcludedByHoles(Vec<usize>),
    InsideSupport,
}
#[derive(Clone, Debug)]
pub struct QualifiedRootFaceRegion {
    source: Arc<RootBrep>,
    face: usize,
    query: [[f64; 2]; 2],
    limits: ss::OriginalSsWindingLimits,
    loops: Vec<QualifiedRootLoopWinding>,
    region: RootFaceRegion,
    segments: usize,
    hole_offsets: Option<Vec<[i32; 2]>>,
    automatic: bool,
}
impl QualifiedRootFaceRegion {
    /// Classify the complete query box using nonzero outer winding minus the
    /// union of all nonzero hole windings. This defines a UV support; it does
    /// not assert Jordan embedding or physical solid material orientation.
    /// Segment allowance is shared across loops; geometry limits are per loop.
    pub fn qualify(
        source: Arc<RootBrep>,
        face: usize,
        query: [[f64; 2]; 2],
        limits: ss::OriginalSsWindingLimits,
    ) -> Result<Self> {
        Self::qualify_impl(source, face, query, limits, None)
    }

    /// Classify in the outer loop's lift. Each hole offset translates its
    /// first coedge by the given number of native periods in that same lift.
    /// The caller supplies hole placement, and every loop must have zero
    /// net period winding.
    pub fn qualify_lifted(
        source: Arc<RootBrep>,
        face: usize,
        query: [[f64; 2]; 2],
        hole_offsets: Vec<[i32; 2]>,
        limits: ss::OriginalSsWindingLimits,
    ) -> Result<Self> {
        Self::qualify_impl(source, face, query, limits, Some(hole_offsets))
    }

    /// Enumerate candidate hole images by exact period bounds and prove
    /// that exactly one lies inside the outer winding region.
    pub fn qualify_lifted_auto(
        source: Arc<RootBrep>,
        face: usize,
        query: [[f64; 2]; 2],
        limits: ss::OriginalSsWindingLimits,
    ) -> Result<Self> {
        let offsets = infer_hole_offsets(&source, face, limits)?;
        let mut proof = Self::qualify_lifted(source, face, query, offsets, limits)?;
        proof.automatic = true;
        Ok(proof)
    }

    pub fn hole_offsets(&self) -> Option<&[[i32; 2]]> {
        self.hole_offsets.as_deref()
    }

    fn qualify_impl(
        source: Arc<RootBrep>,
        face: usize,
        query: [[f64; 2]; 2],
        limits: ss::OriginalSsWindingLimits,
        hole_offsets: Option<Vec<[i32; 2]>>,
    ) -> Result<Self> {
        let f = source
            .topology()
            .faces
            .get(face)
            .ok_or_else(|| unproved("Missing root-region face"))?;
        if f.holes.len() > 64 {
            return Err(unproved("Root-region hole budget exhausted"));
        }
        if let Some(offsets) = &hole_offsets {
            if offsets.len() != f.holes.len()
                || offsets.iter().any(|[u, v]| {
                    (*u != 0 && !f.surface.periodic_u) || (*v != 0 && !f.surface.periodic_v)
                })
            {
                return Err(unproved("Invalid lifted root-region hole placement"));
            }
        }
        let periods = winding::face_periods(&f.surface);
        let mut loops = Vec::new();
        let mut segments = 0;
        for i in 0..=f.holes.len() {
            let remaining = limits
                .max_segments
                .checked_sub(segments)
                .ok_or_else(|| unproved("Root-region segment budget exhausted"))?;
            let loop_limits = ss::OriginalSsWindingLimits {
                max_segments: remaining,
                ..limits
            };
            let proof = if let Some(offsets) = &hole_offsets {
                let local_query = if i == 0 {
                    query
                } else {
                    let shift = std::array::from_fn(|axis| {
                        -&periods[axis]
                            * BigRational::from_integer(i64::from(offsets[i - 1][axis]).into())
                    });
                    winding::translate(query, &shift)?
                };
                QualifiedRootLoopWinding::qualify_lifted(
                    source.clone(),
                    face,
                    i,
                    local_query,
                    loop_limits,
                )?
            } else {
                QualifiedRootLoopWinding::qualify(source.clone(), face, i, query, loop_limits)?
            };
            segments += proof.segments();
            loops.push(proof);
        }
        let holes = loops
            .iter()
            .enumerate()
            .skip(1)
            .filter_map(|(i, p)| (p.winding() != 0).then_some(i - 1))
            .collect::<Vec<_>>();
        let region = if loops[0].winding() == 0 {
            RootFaceRegion::OutsideOuter
        } else if holes.is_empty() {
            RootFaceRegion::InsideSupport
        } else {
            RootFaceRegion::ExcludedByHoles(holes)
        };
        Ok(Self {
            source,
            face,
            query,
            limits,
            loops,
            region,
            segments,
            hole_offsets,
            automatic: false,
        })
    }
    pub fn region(&self) -> &RootFaceRegion {
        &self.region
    }
    pub fn loop_windings(&self) -> Vec<i32> {
        self.loops.iter().map(|p| p.winding()).collect()
    }
    pub fn segments(&self) -> usize {
        self.segments
    }
    pub fn source(&self) -> &Arc<RootBrep> {
        &self.source
    }
    pub fn reverify(&self) -> Result<()> {
        self.source.reverify()?;
        let fresh = if self.automatic {
            Self::qualify_lifted_auto(self.source.clone(), self.face, self.query, self.limits)?
        } else {
            Self::qualify_impl(
                self.source.clone(),
                self.face,
                self.query,
                self.limits,
                self.hole_offsets.clone(),
            )?
        };
        if self.hole_offsets != fresh.hole_offsets
            || self.region != fresh.region
            || self.segments != fresh.segments
            || self.loop_windings() != fresh.loop_windings()
        {
            return Err(unproved("Root face UV support proof changed"));
        }
        Ok(())
    }
}

fn infer_hole_offsets(
    source: &Arc<RootBrep>,
    face: usize,
    limits: ss::OriginalSsWindingLimits,
) -> Result<Vec<[i32; 2]>> {
    let topology = source.topology();
    let f = topology
        .faces
        .get(face)
        .ok_or_else(|| unproved("Missing root-region face"))?;
    if f.holes.len() > 64 {
        return Err(unproved("Root-region hole budget exhausted"));
    }
    let mut remaining = limits.geometry;
    let periods = winding::face_periods(&f.surface);
    let periodic = [f.surface.periodic_u, f.surface.periodic_v];
    let outer = winding::lifted_loop_bounds(source, face, f.outer, &mut remaining)?;
    let mut segments = limits.max_segments;
    let placements: Vec<_> = f
        .holes
        .iter()
        .map(|&id| {
            let hole = winding::lifted_loop_bounds(source, face, id, &mut remaining)?;
            let ranges = overlapping_period_offsets(hole, outer, &periods, periodic)?;
            let offset = unique_inside_offset(ranges, |offset| {
                let shift = std::array::from_fn(|axis| {
                    &periods[axis] * BigRational::from_integer(offset[axis].into())
                });
                classify_hole_image(
                    source,
                    face,
                    id,
                    0,
                    winding::translate(hole, &shift)?,
                    &shift,
                    &periods,
                    &mut remaining,
                    &mut segments,
                    limits,
                )
            })?;
            Ok((id, hole, offset))
        })
        .collect::<Result<_>>()?;
    let contours: Vec<_> = std::iter::once((f.outer, outer, [0, 0]))
        .chain(placements.iter().copied())
        .collect();
    for (i, &(id, bounds, offset)) in contours.iter().enumerate() {
        for (j, &(_, other, other_offset)) in contours.iter().enumerate() {
            let ranges = overlapping_period_offsets(bounds, other, &periods, periodic)?;
            visit_period_offsets(ranges, |delta| {
                // The selected outer/hole nesting was checked during placement.
                let selected = std::array::from_fn(|axis| {
                    i64::from(offset[axis]) - i64::from(other_offset[axis])
                });
                if (i == j && delta == [0, 0])
                    || ((i == 0 || j == 0) && i != j && delta.map(i64::from) == selected)
                {
                    return Ok(());
                }
                let shift = std::array::from_fn(|axis| {
                    &periods[axis] * BigRational::from_integer(delta[axis].into())
                });
                if classify_hole_image(
                    source,
                    face,
                    id,
                    j,
                    winding::translate(bounds, &shift)?,
                    &shift,
                    &periods,
                    &mut remaining,
                    &mut segments,
                    limits,
                )? {
                    return Err(unproved(
                        if i != 0 && j != 0 && i != j && delta.map(i64::from) == selected {
                            "Lifted hole lies inside another hole"
                        } else {
                            "Periodic contour image lies inside another contour"
                        },
                    ));
                }
                Ok(())
            })?;
        }
    }
    Ok(placements
        .into_iter()
        .map(|(_, _, offset)| offset)
        .collect())
}

pub(super) fn overlapping_period_offsets(
    source: [[f64; 2]; 2],
    target: [[f64; 2]; 2],
    periods: &[BigRational; 2],
    periodic: [bool; 2],
) -> Result<[[i32; 2]; 2]> {
    let mut ranges = [[0; 2]; 2];
    let exact = |x| BigRational::from_float(x).ok_or_else(|| unproved("Nonfinite lifted UV bound"));
    for axis in 0..2 {
        if periodic[axis] {
            let first =
                ((exact(target[axis][0])? - exact(source[axis][1])?) / &periods[axis]).ceil();
            let last =
                ((exact(target[axis][1])? - exact(source[axis][0])?) / &periods[axis]).floor();
            for (bound, value) in ranges[axis].iter_mut().zip([first, last]) {
                *bound = value
                    .to_integer()
                    .to_i32()
                    .ok_or_else(|| unproved("Lifted contour period offset overflow"))?;
            }
        } else if source[axis][1] < target[axis][0] || target[axis][1] < source[axis][0] {
            return Ok([[1, 0], [0, 0]]);
        }
    }
    Ok(ranges)
}

fn loop_membership(
    source: &Arc<RootBrep>,
    face: usize,
    query: [[f64; 2]; 2],
    target_loop: usize,
    segments: &mut usize,
    limits: ss::OriginalSsWindingLimits,
) -> Result<Option<bool>> {
    let topology = source.topology();
    let f = &topology.faces[face];
    let id = if target_loop == 0 {
        f.outer
    } else {
        f.holes[target_loop - 1]
    };
    let coedges = topology.loops[id].coedges.len();
    let allowance = (*segments).min(coedges.saturating_mul(2).clamp(64, 4096));
    if allowance == 0 {
        return Err(unproved("Lifted hole placement segment budget exhausted"));
    }
    match QualifiedRootLoopWinding::qualify_lifted(
        source.clone(),
        face,
        target_loop,
        query,
        ss::OriginalSsWindingLimits {
            max_segments: allowance,
            ..limits
        },
    ) {
        Ok(proof) => {
            *segments -= proof.segments();
            Ok(Some(proof.winding() != 0))
        }
        Err(_) => {
            // Failed queries can still spend their entire allowance.
            *segments -= allowance;
            Ok(None)
        }
    }
}

fn classify_hole_image(
    source: &Arc<RootBrep>,
    face: usize,
    id: usize,
    target_loop: usize,
    bounds: [[f64; 2]; 2],
    image_shift: &[BigRational; 2],
    periods: &[BigRational; 2],
    remaining: &mut ss::OriginalSsCurveLimits,
    segments: &mut usize,
    limits: ss::OriginalSsWindingLimits,
) -> Result<bool> {
    if let Some(inside) = loop_membership(source, face, bounds, target_loop, segments, limits)? {
        return Ok(inside);
    }

    let lift = uv_closure::qualify_loop_lift(source, face, id, false)?;
    let topology = source.topology();
    let mut membership = None;
    for (coedge, offset) in topology.loops[id].coedges.iter().zip(lift.coedge_offsets) {
        let edge = &topology.edges[coedge.edge].curve;
        let range = edge.parameter_bounds();
        let domain = edge.curve().parameter_domain();
        let mut pending = vec![[
            winding::outward(&range[0], true)?.max(domain[0]),
            winding::outward(&range[1], false)?.min(domain[1]),
        ]];
        let shift = std::array::from_fn(|axis| {
            &image_shift[axis] + &periods[axis] * BigRational::from_integer(offset[axis].into())
        });
        while let Some(clock) = pending.pop() {
            let uv = winding::project(edge.curve(), clock, coedge.pcurve.support, remaining)?;
            let query = winding::translate(uv, &shift)?;
            if let Some(inside) =
                loop_membership(source, face, query, target_loop, segments, limits)?
            {
                if membership.replace(inside).is_some_and(|old| old != inside) {
                    return Err(unproved("Lifted hole crosses another contour"));
                }
            } else {
                let mid = clock[0] * 0.5 + clock[1] * 0.5;
                if !(clock[0] < mid && mid < clock[1]) {
                    return Err(unproved("Lifted hole contact remains unresolved"));
                }
                pending.push([mid, clock[1]]);
                pending.push([clock[0], mid]);
            }
        }
    }
    membership.ok_or_else(|| unproved("Lifted hole has no classified boundary"))
}

fn unique_inside_offset(
    [[u_min, u_max], [v_min, v_max]]: [[i32; 2]; 2],
    mut inside: impl FnMut([i32; 2]) -> Result<bool>,
) -> Result<[i32; 2]> {
    let mut selected = None;
    visit_period_offsets([[u_min, u_max], [v_min, v_max]], |offset| {
        if inside(offset)? && selected.replace(offset).is_some() {
            return Err(unproved(
                "Multiple lifted hole images lie inside the outer contour",
            ));
        }
        Ok(())
    })?;
    selected.ok_or_else(|| unproved("Lifted hole enclosure lies outside the outer contour"))
}

pub(super) fn visit_period_offsets(
    [[u_min, u_max], [v_min, v_max]]: [[i32; 2]; 2],
    mut visit: impl FnMut([i32; 2]) -> Result<()>,
) -> Result<()> {
    if u_min > u_max || v_min > v_max {
        return Ok(());
    }
    let u_count = i64::from(u_max) - i64::from(u_min) + 1;
    let v_count = i64::from(v_max) - i64::from(v_min) + 1;
    if u_count
        .checked_mul(v_count)
        .is_none_or(|count| count > 4096)
    {
        return Err(unproved("Lifted contour period candidate budget exhausted"));
    }
    for u in u_min..=u_max {
        for v in v_min..=v_max {
            visit([u, v])?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn period_ranges_include_other_images_and_exact_contact() {
        let periods = [
            BigRational::from_integer(1.into()),
            BigRational::from_integer(1.into()),
        ];
        let source = [[0.25, 0.75], [0.875, 1.125]];
        let target = [[0.5, 0.75], [0., 0.25]];
        assert_eq!(
            overlapping_period_offsets(source, target, &periods, [false, true]).unwrap(),
            [[0, 0], [-1, -1]]
        );
        let wide = [[0.25, 0.75], [0., 1.]];
        assert_eq!(
            overlapping_period_offsets(wide, wide, &periods, [false, true]).unwrap(),
            [[0, 0], [-1, 1]]
        );
    }

    #[test]
    fn hole_image_requires_exclusion_of_every_alternative() {
        let ranges = [[0, 0], [-1, 0]];
        assert_eq!(
            unique_inside_offset(ranges, |offset| Ok(offset == [0, -1])).unwrap(),
            [0, -1]
        );
        assert!(unique_inside_offset(ranges, |_| Ok(true)).is_err());
        assert!(unique_inside_offset(ranges, |_| Ok(false)).is_err());
        assert!(
            unique_inside_offset(ranges, |offset| {
                if offset == [0, -1] {
                    Ok(true)
                } else {
                    Err(unproved("Unresolved boundary contact"))
                }
            })
            .is_err()
        );
    }
}

/// Validated periodic contours and their placement in the outer loop's lift.
#[derive(Clone, Debug)]
pub struct QualifiedRootPeriodicFace {
    source: Arc<RootBrep>,
    face: usize,
    limits: ss::OriginalSsWindingLimits,
    offsets: Vec<[i32; 2]>,
    outer: [[f64; 2]; 2],
    periods: [BigRational; 2],
    periodic: [bool; 2],
}

impl QualifiedRootPeriodicFace {
    pub fn qualify(
        source: Arc<RootBrep>,
        face: usize,
        limits: ss::OriginalSsWindingLimits,
    ) -> Result<Self> {
        let mut domain = Self::outer_only(source, face, limits)?;
        domain.offsets = infer_hole_offsets(&domain.source, face, limits)?;
        Ok(domain)
    }

    pub(super) fn outer_only(
        source: Arc<RootBrep>,
        face: usize,
        limits: ss::OriginalSsWindingLimits,
    ) -> Result<Self> {
        if !(1..=4096).contains(&limits.max_segments) {
            return Err(unproved("Invalid periodic face winding budget"));
        }
        let face_data = source
            .topology()
            .faces
            .get(face)
            .ok_or_else(|| unproved("Missing root-region face"))?;
        let mut geometry = limits.geometry;
        let outer = winding::lifted_loop_bounds(&source, face, face_data.outer, &mut geometry)?;
        let periods = winding::face_periods(&face_data.surface);
        let periodic = [face_data.surface.periodic_u, face_data.surface.periodic_v];
        Ok(Self {
            source,
            face,
            limits,
            offsets: Vec::new(),
            outer,
            periods,
            periodic,
        })
    }

    pub(super) fn outer_contains_box(&self, query: [[f64; 2]; 2]) -> Result<bool> {
        let ranges = overlapping_period_offsets(query, self.outer, &self.periods, self.periodic)?;
        let mut segments = self.limits.max_segments;
        let mut inside = false;
        visit_period_offsets(ranges, |offset| {
            let shift = std::array::from_fn(|axis| {
                &self.periods[axis] * BigRational::from_integer(offset[axis].into())
            });
            let proof = QualifiedRootLoopWinding::qualify_lifted(
                self.source.clone(),
                self.face,
                0,
                winding::translate(query, &shift)?,
                ss::OriginalSsWindingLimits {
                    max_segments: segments,
                    ..self.limits
                },
            )?;
            segments -= proof.segments();
            if proof.winding() != 0 && std::mem::replace(&mut inside, true) {
                return Err(unproved("Query overlaps multiple periodic outer regions"));
            }
            Ok(())
        })?;
        Ok(inside)
    }

    pub fn reverify(&self) -> Result<()> {
        self.source.reverify()?;
        let fresh = Self::qualify(self.source.clone(), self.face, self.limits)?;
        if self.offsets != fresh.offsets
            || self.outer != fresh.outer
            || self.periods != fresh.periods
            || self.periodic != fresh.periodic
        {
            return Err(unproved("Periodic face placement changed"));
        }
        Ok(())
    }

    /// Classify a native point using the validated contour placement.
    pub fn classify_point(
        self: &Arc<Self>,
        point: [f64; 2],
    ) -> Result<QualifiedRootPeriodicFacePoint> {
        Ok(QualifiedRootPeriodicFacePoint {
            bounds: self.classify_box(point.map(|value| [value; 2]))?,
        })
    }

    /// Classify every point in a continuous UV box modulo the native periods.
    /// Bounds are ordered in one chart and may extend across a seam.
    pub fn classify_box(
        self: &Arc<Self>,
        query: [[f64; 2]; 2],
    ) -> Result<QualifiedRootPeriodicFaceBox> {
        if !query
            .iter()
            .all(|[lo, hi]| lo.is_finite() && hi.is_finite() && lo <= hi)
        {
            return Err(unproved("Invalid periodic face query bounds"));
        }
        let ranges = overlapping_period_offsets(query, self.outer, &self.periods, self.periodic)?;
        let mut segments = self.limits.max_segments;
        let mut image = None;
        visit_period_offsets(ranges, |offset| {
            let shift = std::array::from_fn(|axis| {
                &self.periods[axis] * BigRational::from_integer(offset[axis].into())
            });
            let proof = QualifiedRootFaceRegion::qualify_lifted(
                self.source.clone(),
                self.face,
                winding::translate(query, &shift)?,
                self.offsets.clone(),
                ss::OriginalSsWindingLimits {
                    max_segments: segments,
                    ..self.limits
                },
            )?;
            segments -= proof.segments();
            if proof.region() != &RootFaceRegion::OutsideOuter
                && image.replace((offset, proof)).is_some()
            {
                return Err(unproved(
                    "Periodic query has multiple images inside the outer contour",
                ));
            }
            Ok(())
        })?;
        Ok(QualifiedRootPeriodicFaceBox {
            domain: self.clone(),
            query,
            image,
        })
    }
}

/// Classification of a whole UV box modulo the periods of a face.
#[derive(Clone, Debug)]
pub struct QualifiedRootPeriodicFaceBox {
    domain: Arc<QualifiedRootPeriodicFace>,
    query: [[f64; 2]; 2],
    image: Option<([i32; 2], QualifiedRootFaceRegion)>,
}

impl QualifiedRootPeriodicFaceBox {
    pub fn region(&self) -> &RootFaceRegion {
        self.image
            .as_ref()
            .map_or(&RootFaceRegion::OutsideOuter, |(_, proof)| proof.region())
    }

    pub fn image_offset(&self) -> Option<[i32; 2]> {
        self.image.as_ref().map(|(offset, _)| *offset)
    }

    pub fn reverify(&self) -> Result<()> {
        self.domain.reverify()?;
        let fresh = self.domain.classify_box(self.query)?;
        if fresh.region() != self.region() || fresh.image_offset() != self.image_offset() {
            return Err(unproved("Periodic face box classification changed"));
        }
        Ok(())
    }
}

/// Classification of a native UV point modulo the periods of a face.
#[derive(Clone, Debug)]
pub struct QualifiedRootPeriodicFacePoint {
    bounds: QualifiedRootPeriodicFaceBox,
}

impl QualifiedRootPeriodicFacePoint {
    pub fn qualify(
        source: Arc<RootBrep>,
        face: usize,
        point: [f64; 2],
        limits: ss::OriginalSsWindingLimits,
    ) -> Result<Self> {
        Arc::new(QualifiedRootPeriodicFace::qualify(source, face, limits)?).classify_point(point)
    }

    pub fn region(&self) -> &RootFaceRegion {
        self.bounds.region()
    }

    pub fn image_offset(&self) -> Option<[i32; 2]> {
        self.bounds.image_offset()
    }

    pub fn reverify(&self) -> Result<()> {
        self.bounds.reverify()
    }
}

/// Uniform region classification over the complete original SS edge.
#[derive(Clone, Debug)]
pub struct QualifiedRootPeriodicFaceEdge {
    domain: Arc<QualifiedRootPeriodicFace>,
    edge: ss::OriginalSsRootEdge,
    support: usize,
    max_cells: usize,
    cells: usize,
    region: RootFaceRegion,
}

impl QualifiedRootPeriodicFaceEdge {
    pub fn qualify(
        domain: Arc<QualifiedRootPeriodicFace>,
        edge: ss::OriginalSsRootEdge,
        support: usize,
        max_cells: usize,
    ) -> Result<Self> {
        let sources = edge.curve().source();
        let surface = match support {
            0 => sources.first(),
            1 => sources.second(),
            _ => return Err(unproved("Invalid periodic edge support")),
        };
        if surface != &domain.source.topology().faces[domain.face].surface {
            return Err(unproved("Periodic edge support belongs to another surface"));
        }
        if !(1..=4096).contains(&max_cells) {
            return Err(unproved("Invalid periodic edge cell budget"));
        }
        let bounds = edge.parameter_bounds();
        let natural = edge.curve().parameter_domain();
        let mut pending = vec![[
            winding::outward(&bounds[0], true)?.max(natural[0]),
            winding::outward(&bounds[1], false)?.min(natural[1]),
        ]];
        let mut geometry = domain.limits.geometry;
        let mut cells = 0;
        let mut region = None;
        while let Some(clock) = pending.pop() {
            if cells == max_cells {
                return Err(unproved("Periodic edge cell budget exhausted"));
            }
            cells += 1;
            let query = winding::project(edge.curve(), clock, support, &mut geometry)?;
            if let Ok(proof) = domain.classify_box(query) {
                let current = proof.region();
                if region.as_ref().is_some_and(|previous| previous != current) {
                    return Err(unproved("Periodic edge crosses face regions"));
                }
                region = Some(current.clone());
            } else {
                let mid = clock[0] * 0.5 + clock[1] * 0.5;
                if !(clock[0] < mid && mid < clock[1]) {
                    return Err(unproved("Periodic edge classification remains unresolved"));
                }
                pending.push([mid, clock[1]]);
                pending.push([clock[0], mid]);
            }
        }
        let region = region.ok_or_else(|| unproved("Empty periodic edge classification"))?;
        Ok(Self {
            domain,
            edge,
            support,
            max_cells,
            cells,
            region,
        })
    }

    pub fn region(&self) -> &RootFaceRegion {
        &self.region
    }

    pub fn reverify(&self) -> Result<()> {
        self.domain.reverify()?;
        self.edge.reverify()?;
        let fresh = Self::qualify(
            self.domain.clone(),
            self.edge.clone(),
            self.support,
            self.max_cells,
        )?;
        if fresh.region != self.region || fresh.cells != self.cells {
            return Err(unproved("Periodic edge classification changed"));
        }
        Ok(())
    }
}
