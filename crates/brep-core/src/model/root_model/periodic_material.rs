//! Whole-child material classification using a conservative UV cover.
use super::*;
use crate::closed_solid::{
    BooleanFaceDisposition, BooleanMaterialOp, BooleanSourceSide, QualifiedClosedSolid,
    QualifiedSurfaceMaterialBox, SurfaceMaterialBoxLimits, qualify_surface_material_box,
};
use num_rational::BigRational;

impl QualifiedRootPeriodicChordSplit {
    /// Classify both children against one operand before emitting any Boolean
    /// patches. An unresolved child fails the entire selection.
    pub fn boolean_contributions(
        &self,
        solid: &QualifiedClosedSolid,
        directions: &[[f64; 3]],
        geometry: ss::OriginalSsCurveLimits,
        limits: SurfaceMaterialBoxLimits,
        operation: BooleanMaterialOp,
        source_side: BooleanSourceSide,
    ) -> Result<[RootPeriodicBooleanContribution; 2]> {
        let [first, second] = self.split().face_children();
        let classify = |child| {
            QualifiedRootPeriodicChildMaterial::qualify(
                self.clone(),
                child,
                solid,
                directions,
                geometry,
                limits,
            )
        };
        let first = classify(first)?;
        let second = classify(second)?;
        Ok([
            first.boolean_contribution(operation, source_side)?,
            second.boolean_contribution(operation, source_side)?,
        ])
    }
}

#[derive(Clone)]
pub struct QualifiedRootPeriodicChildMaterial {
    split: QualifiedRootPeriodicChordSplit,
    child: usize,
    boxes: Vec<QualifiedSurfaceMaterialBox>,
    inside: bool,
    excluded: Vec<[[f64; 2]; 2]>,
    domain: Arc<QualifiedRootPeriodicFace>,
}

impl QualifiedRootPeriodicChildMaterial {
    /// Prove one material value on a cover of the entire child, including its
    /// boundary. The cover can include UV outside the trimmed domain.
    pub fn qualify(
        split: QualifiedRootPeriodicChordSplit,
        child: usize,
        solid: &QualifiedClosedSolid,
        directions: &[[f64; 3]],
        geometry: ss::OriginalSsCurveLimits,
        limits: SurfaceMaterialBoxLimits,
    ) -> Result<Self> {
        if !split.split().face_children().contains(&child) {
            return Err(unproved("Material face is not a child of this split"));
        }
        let source = split.split().output();
        let face = &source.topology().faces[child];
        let lift = winding::lifted_loop_bounds(source, child, face.outer, &mut geometry.clone())?;
        let surface = &face.surface;
        let natural = [
            [
                surface.knots_u[surface.degree_u],
                surface.knots_u[surface.control_points.len()],
            ],
            [
                surface.knots_v[surface.degree_v],
                surface.knots_v[surface.control_points[0].len()],
            ],
        ];
        let pending = native_images(surface, lift, natural)?;
        let domain = Arc::new(QualifiedRootPeriodicFace::qualify(
            source.clone(),
            child,
            ss::OriginalSsWindingLimits {
                geometry,
                ..Default::default()
            },
        )?);
        let (boxes, excluded, inside) = cover_face_material(
            surface,
            pending,
            solid,
            directions,
            limits,
            64,
            None,
            |rectangle| outside_trim(&domain, rectangle),
        )?;
        Ok(Self {
            split,
            child,
            boxes,
            inside,
            excluded,
            domain,
        })
    }

    pub fn child(&self) -> usize {
        self.child
    }

    pub fn inside(&self) -> bool {
        self.inside
    }

    /// Compute the Boolean contribution relative to the source outward
    /// normal. The caller identifies the operand owning this boundary.
    pub fn boolean_disposition(
        &self,
        operation: BooleanMaterialOp,
        source_side: BooleanSourceSide,
    ) -> BooleanFaceDisposition {
        operation.face_disposition(source_side, self.inside)
    }

    /// Extract this boundary contribution with its result face-use orientation.
    /// The source shell use is assumed to describe outward material orientation.
    pub fn boolean_contribution(
        &self,
        operation: BooleanMaterialOp,
        source_side: BooleanSourceSide,
    ) -> Result<RootPeriodicBooleanContribution> {
        let disposition = self.boolean_disposition(operation, source_side);
        periodic_contribution(self.split.split().output(), self.child, disposition)
    }

    pub fn boxes(&self) -> &[QualifiedSurfaceMaterialBox] {
        &self.boxes
    }

    pub fn excluded_cells(&self) -> usize {
        self.excluded.len()
    }

    pub fn reverify(&self) -> Result<()> {
        self.split.reverify()?;
        self.domain.reverify()?;
        for &rectangle in &self.excluded {
            if !outside_trim(&self.domain, rectangle) {
                return Err(unproved("Periodic material exclusion changed"));
            }
        }
        for proof in &self.boxes {
            proof.reverify()?;
        }
        Ok(())
    }
}

/// An oriented boundary patch and its source topology mapping.
#[derive(Clone, Debug)]
pub struct RootPeriodicBooleanContribution {
    disposition: BooleanFaceDisposition,
    patch: Option<(RootFaceRetention, Arc<RootBrep>)>,
}

impl RootPeriodicBooleanContribution {
    pub fn disposition(&self) -> BooleanFaceDisposition {
        self.disposition
    }

    pub fn patch(&self) -> Option<&Arc<RootBrep>> {
        self.patch.as_ref().map(|(_, patch)| patch)
    }

    pub fn lineage(&self) -> Option<&RootFaceRetention> {
        self.patch.as_ref().map(|(lineage, _)| lineage)
    }
}

fn outside_trim(domain: &Arc<QualifiedRootPeriodicFace>, rectangle: [[f64; 2]; 2]) -> bool {
    domain.classify_box(rectangle).is_ok_and(|proof| {
        matches!(
            proof.region(),
            RootFaceRegion::OutsideOuter | RootFaceRegion::ExcludedByHoles(_)
        )
    })
}

/// Material evidence on a complete singly periodic face, including faces
/// bounded by noncontractible loops whose total winding cancels.
pub struct QualifiedRootPeriodicFaceMaterial {
    source: Arc<RootBrep>,
    face: usize,
    seed: QualifiedRootPeriodicFaceMembershipBox,
    boxes: Vec<QualifiedSurfaceMaterialBox>,
    excluded: Vec<[[f64; 2]; 2]>,
    inside: bool,
    cover_root: [[f64; 2]; 2],
    path_limits: ss::OriginalSsPathLimits,
}

impl QualifiedRootPeriodicFaceMaterial {
    /// The seed must be proved inside the source trim. It establishes the
    /// expected opposing material value; the whole-face cover proves it.
    pub fn qualify(
        source: Arc<RootBrep>,
        face: usize,
        seed: [[f64; 2]; 2],
        solid: &QualifiedClosedSolid,
        directions: &[[f64; 3]],
        path_limits: ss::OriginalSsPathLimits,
        material_limits: SurfaceMaterialBoxLimits,
        max_cells: usize,
    ) -> Result<Self> {
        if !(1..=16384).contains(&max_cells) {
            return Err(unproved("Invalid periodic face material cover budget"));
        }
        let seed_proof = source.qualify_periodic_face_membership_box(face, seed, path_limits)?;
        if !seed_proof.inside() {
            return Err(unproved(
                "Periodic face material seed lies outside the trim",
            ));
        }
        let surface = &source.topology().faces[face].surface;
        let natural = crate::boolean_robustness::surface_uv_domains(surface);
        let mut witnesses = Vec::new();
        for rectangle in native_images(surface, seed, natural)? {
            let proof = qualify_surface_material_box(
                solid,
                surface,
                rectangle,
                directions,
                material_limits,
            )?;
            if witnesses
                .first()
                .is_some_and(|first: &QualifiedSurfaceMaterialBox| first.inside() != proof.inside())
            {
                return Err(unproved(
                    "Periodic face seed images disagree on opposing material",
                ));
            }
            witnesses.push(proof);
        }
        let inside = witnesses
            .first()
            .ok_or_else(|| unproved("Periodic face seed has no native image"))?
            .inside();
        let cover_root = periodic_face_cover_root(&source, face, path_limits.geometry)?;
        let (mut boxes, excluded, inside) = cover_face_material(
            surface,
            vec![cover_root],
            solid,
            directions,
            material_limits,
            max_cells,
            Some(inside),
            |rectangle| outside_periodic_face(&source, face, rectangle, path_limits),
        )?;
        boxes.extend(witnesses);
        Ok(Self {
            source,
            face,
            seed: seed_proof,
            boxes,
            excluded,
            inside,
            path_limits,
            cover_root,
        })
    }

    pub fn inside(&self) -> bool {
        self.inside
    }

    pub fn excluded_cells(&self) -> usize {
        self.excluded.len()
    }

    pub fn boolean_contribution(
        &self,
        operation: BooleanMaterialOp,
        side: BooleanSourceSide,
    ) -> Result<RootPeriodicBooleanContribution> {
        periodic_contribution(
            &self.source,
            self.face,
            operation.face_disposition(side, self.inside),
        )
    }

    pub fn reverify(&self) -> Result<()> {
        self.seed.reverify()?;
        if periodic_face_cover_root(&self.source, self.face, self.path_limits.geometry)?
            != self.cover_root
        {
            return Err(unproved("Periodic face transverse cover changed"));
        }
        for &rectangle in &self.excluded {
            if !outside_periodic_face(&self.source, self.face, rectangle, self.path_limits) {
                return Err(unproved("Periodic face material exclusion changed"));
            }
        }
        for proof in &self.boxes {
            proof.reverify()?;
        }
        Ok(())
    }
}

pub(super) fn periodic_face_cover_root(
    source: &RootBrep,
    face_id: usize,
    mut geometry: ss::OriginalSsCurveLimits,
) -> Result<[[f64; 2]; 2]> {
    let t = source.topology();
    let face = &t.faces[face_id];
    if face.surface.periodic_u == face.surface.periodic_v {
        return Err(unproved(
            "Transverse cover needs exactly one periodic UV axis",
        ));
    }
    let total = std::iter::once(face.outer)
        .chain(face.holes.iter().copied())
        .try_fold([0_i64; 2], |sum, loop_id| {
            let lift = uv_closure::qualify_loop_lift(source, face_id, loop_id, false)?;
            Ok::<_, crate::Error>(std::array::from_fn(|axis| {
                sum[axis] + i64::from(lift.winding[axis])
            }))
        })?;
    if total != [0, 0] {
        return Err(unproved("Periodic face boundary has nonzero total winding"));
    }
    let mut cover = crate::boolean_robustness::surface_uv_domains(&face.surface);
    let transverse = usize::from(face.surface.periodic_u);
    let mut extent = [f64::INFINITY, f64::NEG_INFINITY];
    for loop_id in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
        for coedge in &t.loops[loop_id].coedges {
            let edge = &t.edges[coedge.edge].curve;
            let bounds = edge.parameter_bounds();
            let domain = edge.curve().parameter_domain();
            let uv = winding::project(
                edge.curve(),
                [
                    winding::outward(&bounds[0], true)?.max(domain[0]),
                    winding::outward(&bounds[1], false)?.min(domain[1]),
                ],
                coedge.pcurve.support,
                &mut geometry,
            )?;
            extent[0] = extent[0].min(uv[transverse][0]);
            extent[1] = extent[1].max(uv[transverse][1]);
        }
    }
    // A point strictly beyond all transverse boundary coordinates can reach
    // the exterior on the same shore without crossing any contour.
    cover[transverse][0] = cover[transverse][0].max(extent[0]);
    cover[transverse][1] = cover[transverse][1].min(extent[1]);
    if cover[transverse][0] >= cover[transverse][1] {
        return Err(unproved("Empty periodic face transverse cover"));
    }
    Ok(cover)
}

fn native_images(
    surface: &Surface,
    lift: [[f64; 2]; 2],
    natural: [[f64; 2]; 2],
) -> Result<Vec<[[f64; 2]; 2]>> {
    let periods = winding::face_periods(surface);
    let periodic = [surface.periodic_u, surface.periodic_v];
    let ranges = face_region::overlapping_period_offsets(lift, natural, &periods, periodic)?;
    let mut pending = Vec::new();
    face_region::visit_period_offsets(ranges, |offset| {
        if pending.len() == 64 {
            return Err(unproved("Periodic material cover budget exhausted"));
        }
        let shift = std::array::from_fn(|axis| {
            &periods[axis] * BigRational::from_integer(offset[axis].into())
        });
        let image = winding::translate(lift, &shift)?;
        let rectangle = std::array::from_fn(|axis| {
            [
                image[axis][0].max(natural[axis][0]),
                image[axis][1].min(natural[axis][1]),
            ]
        });
        pending.push(rectangle);
        Ok(())
    })?;
    Ok(pending)
}

fn outside_periodic_face(
    source: &Arc<RootBrep>,
    face: usize,
    rectangle: [[f64; 2]; 2],
    limits: ss::OriginalSsPathLimits,
) -> bool {
    source
        .qualify_periodic_face_membership_box(face, rectangle, limits)
        .is_ok_and(|proof| !proof.inside())
}

fn cover_face_material(
    surface: &Surface,
    mut pending: Vec<[[f64; 2]; 2]>,
    solid: &QualifiedClosedSolid,
    directions: &[[f64; 3]],
    limits: SurfaceMaterialBoxLimits,
    max_cells: usize,
    mut inside: Option<bool>,
    outside: impl Fn([[f64; 2]; 2]) -> bool,
) -> Result<(Vec<QualifiedSurfaceMaterialBox>, Vec<[[f64; 2]; 2]>, bool)> {
    let mut boxes = Vec::new();
    let mut excluded = Vec::new();
    let mut cells = 0;
    while let Some(rectangle) = pending.pop() {
        if cells == max_cells {
            return Err(unproved("Periodic material cover budget exhausted"));
        }
        cells += 1;
        match qualify_surface_material_box(solid, surface, rectangle, directions, limits) {
            Ok(proof) if inside.is_none_or(|old| old == proof.inside()) => {
                inside = Some(proof.inside());
                boxes.push(proof);
                continue;
            }
            _ => {}
        }
        if outside(rectangle) {
            excluded.push(rectangle);
            continue;
        }
        let axis =
            usize::from(rectangle[1][1] - rectangle[1][0] > rectangle[0][1] - rectangle[0][0]);
        let [lo, hi] = rectangle[axis];
        let mid = lo * 0.5 + hi * 0.5;
        if !(lo < mid && mid < hi) {
            return Err(unproved("Periodic child material remains unresolved"));
        }
        let mut left = rectangle;
        let mut right = rectangle;
        left[axis][1] = mid;
        right[axis][0] = mid;
        pending.extend([right, left]);
    }
    let inside = inside.ok_or_else(|| unproved("Empty periodic child material cover"))?;
    Ok((boxes, excluded, inside))
}

fn periodic_contribution(
    source: &Arc<RootBrep>,
    face: usize,
    disposition: BooleanFaceDisposition,
) -> Result<RootPeriodicBooleanContribution> {
    if disposition == BooleanFaceDisposition::Discard {
        return Ok(RootPeriodicBooleanContribution {
            disposition,
            patch: None,
        });
    }
    let lineage = RootFaceRetention::retain(source.clone(), vec![face])?;
    let mut topology = lineage.output().topology().clone();
    let mut uses = topology
        .shells
        .iter_mut()
        .flat_map(|shell| &mut shell.faces);
    let face_use = uses
        .next()
        .ok_or_else(|| unproved("Periodic Boolean face has no source shell use"))?;
    if uses.next().is_some() {
        return Err(unproved(
            "Periodic Boolean face has ambiguous source shell use",
        ));
    }
    face_use.reversed ^= disposition == BooleanFaceDisposition::Reverse;
    let patch = Arc::new(RootBrep::from_topology(topology)?);
    let joints = patch
        .topology()
        .loops
        .iter()
        .map(|contour| contour.coedges.len())
        .sum();
    QualifiedRootPeriodicUvClosure::qualify(patch.clone(), joints)?;
    Ok(RootPeriodicBooleanContribution {
        disposition,
        patch: Some((lineage, patch)),
    })
}
