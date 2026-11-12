//! Material selection for a whole UV cell in one exact finite cut piece.
use super::*;
use crate::closed_solid::{
    self, BooleanFaceBoxLimits, BooleanMaterialOp, BooleanSourceSide, QualifiedBooleanFaceBox,
};
#[derive(Clone, Copy, Debug, Default)]
pub struct CutMaterialLimits {
    pub region: CutRectangleLimits,
    pub boolean: BooleanFaceBoxLimits,
}
pub struct QualifiedCutMaterialRectangle {
    region: CutRectangleReport,
    material: QualifiedBooleanFaceBox,
    piece_face: usize,
}
impl QualifiedCutMaterialRectangle {
    pub fn region(&self) -> &CutRectangleReport {
        &self.region
    }
    pub fn boolean_material(&self) -> &QualifiedBooleanFaceBox {
        &self.material
    }
    pub fn piece_face(&self) -> usize {
        self.piece_face
    }
    pub fn retain(&self) -> bool {
        self.material.retain()
    }
    pub fn reverse_material_normal(&self) -> bool {
        self.material.reverse_material_normal()
    }
    pub fn reverify(&self) -> Result<()> {
        self.region.reverify()?;
        self.material.reverify()
    }
}
impl ClosedCutCandidate {
    /// Both original finite-region membership and other-body material are
    /// proved for every source point of the rectangle, not for its midpoint.
    pub fn qualify_boolean_rectangle(
        &self,
        other: &closed_solid::QualifiedClosedSolid,
        operation: BooleanMaterialOp,
        source_side: BooleanSourceSide,
        rectangle: [[f64; 2]; 2],
        directions: &[[f64; 3]],
        limits: CutMaterialLimits,
    ) -> Result<QualifiedCutMaterialRectangle> {
        let region = self.classify_rectangle(rectangle, limits.region)?;
        let piece_face = match (region.inside_piece(), region.outside_piece()) {
            (Location::Inside, Location::Outside) => self.inside,
            (Location::Outside, Location::Inside) => self.face,
            _ => {
                return Err(unproved(
                    "Boolean rectangle has no uniquely proved finite cut piece",
                ));
            }
        };
        let (a, b) = match source_side {
            BooleanSourceSide::First => (&self.source, other),
            BooleanSourceSide::Second => (other, &self.source),
        };
        let material = closed_solid::qualify_boolean_face_box(
            closed_solid::BooleanFace {
                solids: [a, b],
                operation,
                side: source_side,
                face: self.face,
            },
            rectangle,
            directions,
            limits.boolean,
        )?;
        Ok(QualifiedCutMaterialRectangle {
            region,
            material,
            piece_face,
        })
    }
}
