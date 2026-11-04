//! Whole-cell membership in the two finite regions of an original contour cut.
//! This establishes set membership, not Jordan/physical/material admission.
use super::*;
#[derive(Clone, Copy, Debug)]
pub struct CutRectangleLimits {
    pub domain_cells: usize,
    pub winding: ss::OriginalSsWindingLimits,
}
impl Default for CutRectangleLimits {
    fn default() -> Self {
        Self {
            domain_cells: 100000,
            winding: Default::default(),
        }
    }
}
struct Definition {
    source: crate::closed_solid::QualifiedClosedSolid,
    face: usize,
    wire: ss::OriginalSsRootWire,
    support: usize,
    orientation: i32,
    seed: [f64; 2],
    tolerance_uv: f64,
    orientation_limits: ss::OriginalSsWindingLimits,
}
pub struct CutRectangleReport {
    definition: Definition,
    rectangle: [[f64; 2]; 2],
    limits: CutRectangleLimits,
    source_location: Location,
    inside: Location,
    outside: Location,
    winding: Option<i32>,
    domain_cells: usize,
    root_segments: Option<usize>,
    root_refusal: Option<Error>,
    reason: &'static str,
}
impl CutRectangleReport {
    pub fn inside_piece(&self) -> Location {
        self.inside
    }
    pub fn outside_piece(&self) -> Location {
        self.outside
    }
    pub fn source_location(&self) -> Location {
        self.source_location
    }
    pub fn winding(&self) -> Option<i32> {
        self.winding
    }
    pub fn domain_cells(&self) -> usize {
        self.domain_cells
    }
    pub fn root_segments(&self) -> Option<usize> {
        self.root_segments
    }
    pub fn reason(&self) -> &'static str {
        self.reason
    }
    pub fn root_refusal(&self) -> Option<&Error> {
        self.root_refusal.as_ref()
    }
    pub fn rectangle(&self) -> [[f64; 2]; 2] {
        self.rectangle
    }
    pub fn reverify(&self) -> Result<()> {
        let d = &self.definition;
        d.source.reverify()?;
        d.wire.reverify()?;
        let orientation =
            ss::qualify_original_ss_winding(&d.wire, d.support, d.seed, d.orientation_limits)?;
        if orientation.winding() != d.orientation {
            return Err(unproved("Original cut orientation changed"));
        }
        let fresh = classify(clone_definition(d), self.rectangle, self.limits)?;
        if fresh.inside != self.inside
            || fresh.outside != self.outside
            || fresh.source_location != self.source_location
            || fresh.winding != self.winding
            || fresh.reason != self.reason
            || fresh.domain_cells != self.domain_cells
            || fresh.root_segments != self.root_segments
        {
            return Err(unproved("Original cut rectangle membership changed"));
        }
        Ok(())
    }
}
fn clone_definition(d: &Definition) -> Definition {
    Definition {
        source: d.source.clone(),
        face: d.face,
        wire: d.wire.clone(),
        support: d.support,
        orientation: d.orientation,
        seed: d.seed,
        tolerance_uv: d.tolerance_uv,
        orientation_limits: d.orientation_limits,
    }
}
fn classify(
    d: Definition,
    rectangle: [[f64; 2]; 2],
    limits: CutRectangleLimits,
) -> Result<CutRectangleReport> {
    if !(1..=100000).contains(&limits.domain_cells)
        || rectangle
            .iter()
            .any(|v| !v.iter().all(|x| x.is_finite()) || v[0] > v[1])
    {
        return Err(unproved(
            "Cut rectangle needs finite ordered UV bounds and bounded positive domain work",
        ));
    }
    let domain = crate::face_domain::FaceDomain::new(d.source.model(), d.face, d.tolerance_uv)?;
    let parent = domain.classify(rectangle, limits.domain_cells)?;
    let mut out = CutRectangleReport {
        definition: d,
        rectangle,
        limits,
        source_location: parent.location,
        inside: Location::Unresolved,
        outside: Location::Unresolved,
        winding: None,
        domain_cells: parent.cells,
        root_segments: None,
        root_refusal: None,
        reason: "original-cut-rectangle-unresolved",
    };
    if parent.location == Location::Outside {
        out.inside = Location::Outside;
        out.outside = Location::Outside;
        out.root_segments = Some(0);
        out.reason = "outside-original-retained-domain";
        return Ok(out);
    }
    let d = &out.definition;
    let winding = match d.wire.winding_box(d.support, rectangle, limits.winding) {
        Ok(proof) => {
            out.root_segments = Some(proof.segments());
            proof.winding()
        }
        Err(error) => {
            out.root_refusal = Some(error);
            out.reason = "original-cut-box-winding-unproved";
            return Ok(out);
        }
    };
    out.winding = Some(winding);
    if winding == 0 {
        out.inside = Location::Outside;
        out.outside = parent.location;
        out.reason = "original-cut-exterior-cell";
    } else if winding == d.orientation {
        out.inside = parent.location;
        out.outside = Location::Outside;
        out.reason = "original-cut-interior-cell";
    } else {
        out.reason = "original-cut-multiplicity-unproved";
    }
    Ok(out)
}
impl ClosedCutCandidate {
    /// Classify the ENTIRE original UV box against each finite piece. Unknown
    /// boundary/trim cells stay unresolved; neither a midpoint nor missing work
    /// becomes an outside/material verdict.
    pub fn classify_rectangle(
        &self,
        rectangle: [[f64; 2]; 2],
        limits: CutRectangleLimits,
    ) -> Result<CutRectangleReport> {
        classify(
            Definition {
                source: self.source.clone(),
                face: self.face,
                wire: self.wire.clone(),
                support: self.support,
                orientation: self.orientation,
                seed: self.seed,
                tolerance_uv: self.limits.tolerance_uv,
                orientation_limits: self.limits.per_boundary_winding,
            },
            rectangle,
            limits,
        )
    }
}
