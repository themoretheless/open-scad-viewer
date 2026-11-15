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
                self.clone(), child, solid, directions, geometry, limits,
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
        let periods = winding::face_periods(surface);
        let periodic = [surface.periodic_u, surface.periodic_v];
        let ranges = face_region::overlapping_period_offsets(lift, natural, &periods, periodic)?;
        let mut pending = Vec::new();
        face_region::visit_period_offsets(ranges, |offset| {
            if pending.len() == 64 {
                return Err(unproved("Periodic child material cover budget exhausted"));
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
        let domain = Arc::new(QualifiedRootPeriodicFace::qualify(
            source.clone(),
            child,
            ss::OriginalSsWindingLimits {
                geometry,
                ..Default::default()
            },
        )?);
        let mut boxes = Vec::new();
        let mut excluded = Vec::new();
        let mut inside = None;
        let mut cells = 0;
        while let Some(rectangle) = pending.pop() {
            if cells == 64 {
                return Err(unproved("Periodic child material cover budget exhausted"));
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
            if outside_trim(&domain, rectangle) {
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
        if disposition == BooleanFaceDisposition::Discard {
            return Ok(RootPeriodicBooleanContribution {
                disposition,
                patch: None,
            });
        }
        let source = self.split.split().output();
        let lineage = RootFaceRetention::retain(source.clone(), vec![self.child])?;
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
