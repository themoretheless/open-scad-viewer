//! Two-sided material selection for every original source point in one finite
//! UV rectangle. This is a cell decision, not complete Boolean assembly.
use super::*;
#[derive(Clone, Copy, Debug)]
pub struct BooleanFaceBoxLimits {
    pub domain_cells: usize,
    pub material: SurfaceMaterialBoxLimits,
}
impl Default for BooleanFaceBoxLimits {
    fn default() -> Self {
        Self {
            domain_cells: 100000,
            material: Default::default(),
        }
    }
}
#[derive(Clone)]
pub struct QualifiedBooleanFaceBox {
    a: QualifiedClosedSolid,
    b: QualifiedClosedSolid,
    operation: BooleanMaterialOp,
    side: BooleanSourceSide,
    face: usize,
    material: QualifiedSurfaceMaterialBox,
    retain: bool,
    reverse: bool,
    reversed_surface: bool,
    domain_cells: usize,
    directions: Vec<[f64; 3]>,
    limits: BooleanFaceBoxLimits,
}
impl QualifiedBooleanFaceBox {
    pub fn retain(&self) -> bool {
        self.retain
    }
    pub fn reverse_material_normal(&self) -> bool {
        self.reverse
    }
    pub fn reversed_surface(&self) -> bool {
        self.reversed_surface
    }
    pub fn source(&self) -> BooleanSourceSide {
        self.side
    }
    pub fn face(&self) -> usize {
        self.face
    }
    pub fn material_box(&self) -> &QualifiedSurfaceMaterialBox {
        &self.material
    }
    pub fn domain_cells(&self) -> usize {
        self.domain_cells
    }
    pub fn reverify(&self) -> Result<()> {
        self.a.reverify()?;
        self.b.reverify()?;
        let fresh = qualify_boolean_face_box(
            &self.a,
            &self.b,
            self.operation,
            self.side,
            self.face,
            self.material.rectangle(),
            &self.directions,
            self.limits,
        )?;
        if self.retain != fresh.retain
            || self.reverse != fresh.reverse
            || self.reversed_surface != fresh.reversed_surface
        {
            return Err(Error::new(
                "BREP_BOOLEAN_BOX_UNPROVED",
                "Whole original face material decision changed",
            ));
        }
        Ok(())
    }
}
pub fn qualify_boolean_face_box(
    a: &QualifiedClosedSolid,
    b: &QualifiedClosedSolid,
    operation: BooleanMaterialOp,
    side: BooleanSourceSide,
    face: usize,
    rectangle: [[f64; 2]; 2],
    directions: &[[f64; 3]],
    limits: BooleanFaceBoxLimits,
) -> Result<QualifiedBooleanFaceBox> {
    let (source, other) = match side {
        BooleanSourceSide::First => (a, b),
        BooleanSourceSide::Second => (b, a),
    };
    let f = source
        .model
        .faces
        .get(face)
        .ok_or_else(|| Error::new("BREP_INVALID_INPUT", "Boolean box source face is absent"))?;
    let domain =
        crate::face_domain::FaceDomain::new(source.model(), face, source.policy.tolerance_uv)?;
    let placement = domain.classify(rectangle, limits.domain_cells)?;
    if placement.location != nurbs_core::trim_domain::Location::Inside {
        return Err(Error::new(
            "BREP_BOOLEAN_BOX_UNPROVED",
            "Whole Boolean source box is not proved inside its finite face",
        ));
    }
    let mut uses = source
        .model
        .shells
        .iter()
        .flat_map(|s| &s.faces)
        .filter(|u| u.face == face);
    let use_ = uses.next().ok_or_else(|| {
        Error::new(
            "BREP_BOOLEAN_BOX_UNPROVED",
            "Source face has no material normal",
        )
    })?;
    if uses.next().is_some() {
        return Err(Error::new(
            "BREP_BOOLEAN_BOX_UNPROVED",
            "Source face has multiple material uses",
        ));
    }
    let material =
        qualify_surface_material_box(other, &f.surface, rectangle, directions, limits.material)?;
    let (inside, outside) = match side {
        BooleanSourceSide::First => (
            operation.material(true, material.inside()),
            operation.material(false, material.inside()),
        ),
        BooleanSourceSide::Second => (
            operation.material(material.inside(), true),
            operation.material(material.inside(), false),
        ),
    };
    let retain = inside != outside;
    let reverse = retain && !inside;
    Ok(QualifiedBooleanFaceBox {
        a: a.clone(),
        b: b.clone(),
        operation,
        side,
        face,
        material,
        retain,
        reverse,
        reversed_surface: use_.reversed ^ reverse,
        domain_cells: placement.cells,
        directions: directions.to_vec(),
        limits,
    })
}
