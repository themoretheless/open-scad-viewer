//! Exhaustive contour-side signatures for entire retained UV boxes. A signature
//! is not a connected region ID and never assembles a closed solid by itself.
use super::*;
use crate::closed_solid::{self, BooleanMaterialOp, BooleanSourceSide, QualifiedBooleanFaceBox};
pub struct CensusRectangleReport {
    census: Arc<QualifiedInteriorSsCensus>,
    support: usize,
    rectangle: [[f64; 2]; 2],
    limits: CutRectangleLimits,
    reports: Vec<CutRectangleReport>,
    location: Location,
    signature: Option<Vec<bool>>,
}
impl CensusRectangleReport {
    pub fn location(&self) -> Location {
        self.location
    }
    pub fn signature(&self) -> Option<&[bool]> {
        self.signature.as_deref()
    }
    pub fn component_reports(&self) -> &[CutRectangleReport] {
        &self.reports
    }
    pub fn rectangle(&self) -> [[f64; 2]; 2] {
        self.rectangle
    }
    pub fn reverify(&self) -> Result<()> {
        self.census.reverify()?;
        let fresh = self
            .census
            .classify_rectangle(self.support, self.rectangle, self.limits)?;
        if fresh.location != self.location || fresh.signature != self.signature {
            return Err(unproved("Complete SS contour-side signature changed"));
        }
        Ok(())
    }
}
pub struct QualifiedCensusMaterialRectangle {
    region: CensusRectangleReport,
    material: QualifiedBooleanFaceBox,
}
impl QualifiedCensusMaterialRectangle {
    pub fn region(&self) -> &CensusRectangleReport {
        &self.region
    }
    pub fn boolean_material(&self) -> &QualifiedBooleanFaceBox {
        &self.material
    }
    pub fn reverify(&self) -> Result<()> {
        self.region.reverify()?;
        self.material.reverify()
    }
}
impl QualifiedInteriorSsCensus {
    pub(super) fn parent(&self, support: usize) -> Result<&ClosedCutCandidate> {
        if support >= 2 {
            return Err(unproved("Census support must be zero or one"));
        }
        let base = &self.loops[0].cuts[support];
        for component in &self.loops {
            let cut = &component.cuts[support];
            if cut.face != base.face || !std::ptr::eq(cut.source.model(), base.source.model()) {
                return Err(unproved(
                    "Contour arrangement needs one literal retained parent face",
                ));
            }
        }
        Ok(base)
    }
    /// All components of the complete census participate. Unproved trim or
    /// boundary membership keeps the whole box unresolved; no midpoint vote.
    /// The provided work allowance applies per component (at most 64).
    pub fn classify_rectangle(
        self: &Arc<Self>,
        support: usize,
        rectangle: [[f64; 2]; 2],
        limits: CutRectangleLimits,
    ) -> Result<CensusRectangleReport> {
        self.parent(support)?;
        let reports: Vec<_> = self
            .loops
            .iter()
            .map(|c| c.cuts[support].classify_rectangle(rectangle, limits))
            .collect::<Result<_>>()?;
        let mut signature = Vec::new();
        for report in &reports {
            match (report.inside_piece(), report.outside_piece()) {
                (Location::Inside, Location::Outside) => signature.push(true),
                (Location::Outside, Location::Inside) => signature.push(false),
                _ => break,
            }
        }
        let (location, signature) = if signature.len() == reports.len() {
            (Location::Inside, Some(signature))
        } else if reports
            .iter()
            .all(|r| r.source_location() == Location::Outside)
        {
            (Location::Outside, None)
        } else {
            (Location::Unresolved, None)
        };
        Ok(CensusRectangleReport {
            census: self.clone(),
            support,
            rectangle,
            limits,
            reports,
            location,
            signature,
        })
    }
    /// Bind exhaustive contour-side membership and original whole-box material
    /// for the same retained face and the same two closed operands.
    pub fn qualify_boolean_rectangle(
        self: &Arc<Self>,
        operation: BooleanMaterialOp,
        side: BooleanSourceSide,
        rectangle: [[f64; 2]; 2],
        directions: &[[f64; 3]],
        limits: CutMaterialLimits,
    ) -> Result<QualifiedCensusMaterialRectangle> {
        let support = match side {
            BooleanSourceSide::First => 0,
            BooleanSourceSide::Second => 1,
        };
        let first = self.parent(0)?;
        let second = self.parent(1)?;
        let region = self.classify_rectangle(support, rectangle, limits.region)?;
        if region.location() != Location::Inside || region.signature().is_none() {
            return Err(unproved(
                "Boolean box has no proved complete contour-side signature",
            ));
        }
        let face = if support == 0 {
            first.face
        } else {
            second.face
        };
        let material = closed_solid::qualify_boolean_face_box(
            closed_solid::BooleanFace {
                solids: [&first.source, &second.source],
                operation,
                side,
                face,
            },
            rectangle,
            directions,
            limits.boolean,
        )?;
        Ok(QualifiedCensusMaterialRectangle { region, material })
    }
}
