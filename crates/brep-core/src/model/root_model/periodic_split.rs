//! Periodic subdivision with boundary contact and hole placement checks.
use super::*;

/// A periodic chord subdivision with its boundary and hole evidence.
#[derive(Clone, Debug)]
pub struct QualifiedRootPeriodicChordSplit {
    boundary: QualifiedRootChordBoundary,
    holes: QualifiedRootHoleAssignment,
    trim_limits: ss::OriginalSsCurveLimits,
    source_face: usize,
}

impl QualifiedRootPeriodicChordSplit {
    pub fn qualify(
        source: Arc<RootBrep>,
        face: usize,
        chord: ss::OriginalSsRootEdge,
        support: usize,
        boundary_limits: RootChordBoundaryLimits,
        hole_limits: RootHoleAssignmentLimits,
    ) -> Result<Self> {
        let hole_count = source
            .topology()
            .faces
            .get(face)
            .ok_or_else(|| unproved("Missing periodic chord face"))?
            .holes
            .len();
        let mut trim_limits = boundary_limits.geometry;
        source.audit_face_trim_arcs(face, &mut trim_limits)?;
        let candidate = RootFaceChordSplit::split_periodic(
            source,
            face,
            chord,
            support,
            vec![false; hole_count],
        )?;
        let boundary = QualifiedRootChordBoundary::qualify(candidate.clone(), boundary_limits)?;
        let holes = QualifiedRootHoleAssignment::qualify(candidate, hole_limits)?;
        let split = holes.output();
        for child in split.face_children() {
            split
                .output()
                .audit_face_trim_arcs(child, &mut trim_limits)?;
        }
        Ok(Self {
            boundary,
            holes,
            trim_limits: boundary_limits.geometry,
            source_face: face,
        })
    }

    pub fn boundary(&self) -> &QualifiedRootChordBoundary {
        &self.boundary
    }

    pub fn hole_assignment(&self) -> &QualifiedRootHoleAssignment {
        &self.holes
    }

    pub fn split(&self) -> &RootFaceChordSplit {
        self.holes.output()
    }

    /// Extract the child containing the entire supplied UV box.
    /// The box must be outside the other child's outer boundary.
    pub fn retain_child_containing(
        &self,
        query: [[f64; 2]; 2],
        limits: ss::OriginalSsWindingLimits,
    ) -> Result<RootFaceRetention> {
        Ok(self.select_child(query, limits)?.retention)
    }

    /// Preserve the evidence for the UV selection and extracted topology.
    pub fn select_child(
        &self,
        query: [[f64; 2]; 2],
        limits: ss::OriginalSsWindingLimits,
    ) -> Result<QualifiedRootPeriodicChildSelection> {
        let split = self.split();
        let output = split.output();
        let children = split.face_children();
        let classify = |face| {
            Arc::new(QualifiedRootPeriodicFace::qualify(
                output.clone(),
                face,
                limits,
            )?)
            .classify_box(query)
        };
        let first = classify(children[0])?;
        let second = classify(children[1])?;
        let child = match (first.region(), second.region()) {
            (RootFaceRegion::InsideSupport, RootFaceRegion::OutsideOuter) => children[0],
            (RootFaceRegion::OutsideOuter, RootFaceRegion::InsideSupport) => children[1],
            _ => return Err(unproved("UV box must select exactly one periodic child")),
        };
        let retention = RootFaceRetention::retain(output.clone(), vec![child])?;
        let topology = retention.output().topology();
        let max_joints = topology
            .loops
            .iter()
            .map(|contour| contour.coedges.len())
            .sum();
        let closure =
            QualifiedRootPeriodicUvClosure::qualify(retention.output().clone(), max_joints)?;
        Ok(QualifiedRootPeriodicChildSelection {
            split: self.clone(),
            query,
            limits,
            child,
            boxes: [first, second],
            retention,
            closure,
        })
    }

    pub fn reverify(&self) -> Result<()> {
        self.boundary.reverify()?;
        self.holes.reverify()?;
        let split = self.split();
        let mut remaining = self.trim_limits;
        split
            .source()
            .audit_face_trim_arcs(self.source_face, &mut remaining)?;
        for child in split.face_children() {
            split.output().audit_face_trim_arcs(child, &mut remaining)?;
        }
        Ok(())
    }
}

/// Evidence for extracting a periodic child from a UV box.
#[derive(Clone, Debug)]
pub struct QualifiedRootPeriodicChildSelection {
    split: QualifiedRootPeriodicChordSplit,
    query: [[f64; 2]; 2],
    limits: ss::OriginalSsWindingLimits,
    child: usize,
    boxes: [QualifiedRootPeriodicFaceBox; 2],
    retention: RootFaceRetention,
    closure: QualifiedRootPeriodicUvClosure,
}

impl QualifiedRootPeriodicChildSelection {
    pub fn child(&self) -> usize {
        self.child
    }

    pub fn retention(&self) -> &RootFaceRetention {
        &self.retention
    }

    /// Certify the material of the entire selection box against a closed body.
    /// This evidence is local to the box and can seed a face arrangement.
    pub fn qualify_material_seed(
        &self,
        solid: &crate::closed_solid::QualifiedClosedSolid,
        directions: &[[f64; 3]],
        limits: crate::closed_solid::SurfaceMaterialBoxLimits,
    ) -> Result<QualifiedRootPeriodicChildMaterialSeed> {
        let surface = &self.split.split().output().topology().faces[self.child].surface;
        let material = crate::closed_solid::qualify_surface_material_box(
            solid, surface, self.query, directions, limits,
        )?;
        Ok(QualifiedRootPeriodicChildMaterialSeed {
            selection: self.clone(),
            material,
        })
    }

    /// Apply the selection to the subdivided model, removing only the sibling.
    pub fn trim(&self) -> Result<RootFaceRetention> {
        let split = self.split.split();
        let sibling = split
            .face_children()
            .into_iter()
            .find(|&face| face != self.child)
            .ok_or_else(|| unproved("Missing periodic sibling face"))?;
        let output = split.output();
        let faces = (0..output.topology().faces.len())
            .filter(|&face| face != sibling)
            .collect();
        RootFaceRetention::retain(output.clone(), faces)
    }

    pub fn reverify(&self) -> Result<()> {
        self.split.reverify()?;
        self.retention.reverify()?;
        self.closure.reverify()?;
        let fresh = self.split.select_child(self.query, self.limits)?;
        if fresh.child != self.child
            || fresh.retention.face_map() != self.retention.face_map()
            || self.boxes.iter().zip(&fresh.boxes).any(|(old, new)| {
                old.region() != new.region() || old.image_offset() != new.image_offset()
            })
        {
            return Err(unproved("Periodic child selection changed"));
        }
        Ok(())
    }
}

/// A material seed bound to a selected periodic child and an independent body.
#[derive(Clone)]
pub struct QualifiedRootPeriodicChildMaterialSeed {
    selection: QualifiedRootPeriodicChildSelection,
    material: crate::closed_solid::QualifiedSurfaceMaterialBox,
}

impl QualifiedRootPeriodicChildMaterialSeed {
    pub fn child(&self) -> usize {
        self.selection.child()
    }

    pub fn material(&self) -> &crate::closed_solid::QualifiedSurfaceMaterialBox {
        &self.material
    }

    pub fn reverify(&self) -> Result<()> {
        self.selection.reverify()?;
        self.material.reverify()
    }
}
