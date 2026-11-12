//! Exact two-sided Boolean material decision at an admitted original face seed.
//! Applying this decision to a full face cell still requires a complete SS
//! arrangement proving that the cell contains no additional material boundary.
use super::material_seed::{
    QualifiedSurfaceMaterialSeed, SurfaceMaterialLimits, qualify_surface_material_seed,
};
use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BooleanMaterialOp {
    Union,
    Intersection,
    Difference,
    Xor,
}
impl BooleanMaterialOp {
    pub fn material(self, a: bool, b: bool) -> bool {
        match self {
            Self::Union => a || b,
            Self::Intersection => a && b,
            Self::Difference => a && !b,
            Self::Xor => a != b,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BooleanSourceSide {
    First,
    Second,
}
#[derive(Clone, Copy, Debug)]
pub struct BooleanFaceSeedLimits {
    pub domain_cells: usize,
    pub material: SurfaceMaterialLimits,
}
impl Default for BooleanFaceSeedLimits {
    fn default() -> Self {
        Self {
            domain_cells: 100000,
            material: Default::default(),
        }
    }
}
#[derive(Clone)]
pub struct QualifiedBooleanFaceSeed {
    a: QualifiedClosedSolid,
    b: QualifiedClosedSolid,
    operation: BooleanMaterialOp,
    side: BooleanSourceSide,
    face: usize,
    seed: QualifiedSurfaceMaterialSeed,
    retain: bool,
    reverse_material_normal: bool,
    reversed_surface: bool,
    domain_cells: usize,
    limits: BooleanFaceSeedLimits,
    directions: Vec<[f64; 3]>,
}
impl QualifiedBooleanFaceSeed {
    pub fn retain(&self) -> bool {
        self.retain
    }
    /// Relative to the original body's material-outward normal.
    pub fn reverse_material_normal(&self) -> bool {
        self.reverse_material_normal
    }
    /// Absolute reversal relative to the original surface parameterization.
    pub fn reversed_surface(&self) -> bool {
        self.reversed_surface
    }
    pub fn source(&self) -> BooleanSourceSide {
        self.side
    }
    pub fn face(&self) -> usize {
        self.face
    }
    pub fn material_seed(&self) -> &QualifiedSurfaceMaterialSeed {
        &self.seed
    }
    pub fn domain_cells(&self) -> usize {
        self.domain_cells
    }
    pub fn reverify(&self) -> Result<()> {
        self.a.reverify()?;
        self.b.reverify()?;
        let f = qualify_boolean_face_seed(
            BooleanFace {
                solids: [&self.a, &self.b],
                operation: self.operation,
                side: self.side,
                face: self.face,
            },
            self.seed.parameters(),
            &self.directions,
            self.limits,
        )?;
        if self.retain != f.retain
            || self.reverse_material_normal != f.reverse_material_normal
            || self.reversed_surface != f.reversed_surface
        {
            return Err(Error::new(
                "BREP_BOOLEAN_MATERIAL_UNPROVED",
                "Two-sided original face material decision changed",
            ));
        }
        Ok(())
    }
}
/// One face of a boolean source: both qualified solids, the material
/// operation, which solid supplies the face, and the face index there.
#[derive(Clone, Copy)]
pub struct BooleanFace<'a> {
    pub solids: [&'a QualifiedClosedSolid; 2],
    pub operation: BooleanMaterialOp,
    pub side: BooleanSourceSide,
    pub face: usize,
}
pub fn qualify_boolean_face_seed(
    query: BooleanFace<'_>,
    uv: [f64; 2],
    directions: &[[f64; 3]],
    limits: BooleanFaceSeedLimits,
) -> Result<QualifiedBooleanFaceSeed> {
    let BooleanFace {
        solids: [a, b],
        operation,
        side,
        face,
    } = query;
    let (source, other) = match side {
        BooleanSourceSide::First => (a, b),
        BooleanSourceSide::Second => (b, a),
    };
    let f = source
        .model
        .faces
        .get(face)
        .ok_or_else(|| Error::new("BREP_INVALID_INPUT", "Boolean seed face index is absent"))?;
    let domain =
        crate::face_domain::FaceDomain::new(source.model(), face, source.policy.tolerance_uv)?;
    let placement = domain.classify(uv.map(|x| [x; 2]), limits.domain_cells)?;
    if placement.location != nurbs_core::trim_domain::Location::Inside {
        return Err(Error::new(
            "BREP_BOOLEAN_MATERIAL_UNPROVED",
            "Boolean seed needs a proved finite-face interior, away from rims",
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
            "BREP_BOOLEAN_MATERIAL_UNPROVED",
            "Boolean seed has no material shell face use",
        )
    })?;
    if uses.next().is_some() {
        return Err(Error::new(
            "BREP_BOOLEAN_MATERIAL_UNPROVED",
            "Boolean face normal has multiple shell uses",
        ));
    }
    let seed = qualify_surface_material_seed(other, &f.surface, uv, directions, limits.material)?;
    let (inside, outside) = match side {
        BooleanSourceSide::First => (
            operation.material(true, seed.inside()),
            operation.material(false, seed.inside()),
        ),
        BooleanSourceSide::Second => (
            operation.material(seed.inside(), true),
            operation.material(seed.inside(), false),
        ),
    };
    let retain = inside != outside;
    let reverse_material_normal = retain && !inside;
    let reversed_surface = use_.reversed ^ reverse_material_normal;
    Ok(QualifiedBooleanFaceSeed {
        a: a.clone(),
        b: b.clone(),
        operation,
        side,
        face,
        seed,
        retain,
        reverse_material_normal,
        reversed_surface,
        domain_cells: placement.cells,
        limits,
        directions: directions.to_vec(),
    })
}
