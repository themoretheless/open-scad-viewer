//! source profile shell operations: enclose_support_flux.
use super::*;
impl ExpressionShell<'_, '_, '_, '_, '_, '_, '_, '_> {
/// Flux of a complete support rectangle in its native chart orientation.
    /// The caller must classify material trim and apply shell orientation before
    /// summing volume. A support rectangle alone never admits a solid.
    pub fn enclose_support_flux(
        &self,
        face: usize,
        domain: [[f64; 2]; 2],
        axis: usize,
        origin: f64,
        max_controls: usize,
        max_spans: usize,
    ) -> Result<(Option<nurbs_core::surface_flux::Bound>, usize)> {
        if axis >= 3 || !origin.is_finite() {
            return Err(Error::new(
                "BREP_PROFILE_SHELL",
                "Choose a finite support flux axis and origin",
            ));
        }
        let (position, position_spans) =
            self.enclose_support(face, domain, max_controls, max_spans)?;
        let Some(position) = position else {
            return Ok((None, position_spans));
        };
        let (derivatives, derivative_spans) = self.enclose_support_derivatives(
            face,
            domain,
            max_controls,
            max_spans - position_spans,
        )?;
        let spent = position_spans + derivative_spans;
        let Some(derivatives) = derivatives else {
            return Ok((None, spent));
        };
        let bound = nurbs_core::surface_flux::bound_expression(
            position,
            derivatives,
            domain,
            axis,
            origin,
            false,
        )?;
        Ok((Some(bound), spent))
    }
/// First derivatives in the same parameters as `enclose_support`.
    /// These enclose the supporting expression, without a trim classification.
    pub fn enclose_support_derivatives(
        &self,
        face: usize,
        domain: [[f64; 2]; 2],
        max_controls: usize,
        max_spans: usize,
    ) -> Result<(Option<[[[f64; 2]; 3]; 2]>, usize)> {
        if max_controls > 1000000
            || max_spans > 100000
            || domain
                .iter()
                .any(|r| !r.iter().all(|v| v.is_finite()) || r[0] > r[1])
        {
            return Err(Error::new(
                "BREP_PROFILE_SHELL",
                "Bound support derivative parameters and work",
            ));
        }
        let kind = self
            .faces
            .get(face)
            .ok_or_else(|| Error::new("BREP_PROFILE_SHELL", "Unknown expression face"))?
            .kind;
        if max_spans == 0 {
            let natural = self.support_domain(face)?;
            if (0..2).any(|k| domain[k][0] < natural[k][0] || domain[k][1] > natural[k][1]) {
                return Err(Error::new(
                    "BREP_PROFILE_SHELL",
                    "Keep exhausted derivative queries in the owned support domain",
                ));
            }
            return Ok((None, 0));
        }

        let root = self.assembly.patch.rails.face.boundary.rooted[0];
        let profile = root.restricted.proposal.profile(root.face).unwrap();
        match kind {
            ShellFaceKind::Retained(_) => {
                let (bounds, spent) = profile.derivative_bounds(domain, max_spans)?;
                Ok((bounds.map(|b| std::array::from_fn(|axis| std::array::from_fn(|k| b[k][axis]))), spent))
            }
            ShellFaceKind::Transition => {
                let first = profile.section(root.axis, root.parameters[0])?;
                let last = profile.section(root.axis, root.parameters[1])?;
                let width = root.parameters[1] - root.parameters[0];
                let bridge = nurbs_core::normal_alignment::ProfileSectionBridge::new(first, last, [width; 2])?;
                let (constrained, _) = bridge.constrain_original_rails(max_controls)?;
                let Some(constrained) = constrained else { return Ok((None, 0)); };
                constrained.derivatives_with_transverse_correction(domain[0], domain[1], max_spans)
            }
            ShellFaceKind::Original(index) => {
                let source = root.restricted.proposal.body.geometry().shell().faces()[index][0].edges()[0].surface();
                original_support_derivatives(source,domain,max_spans)
            }
        }
    }
pub fn assembly(&self) -> &OuterEdgeAssembly<'_, '_, '_, '_, '_, '_, '_> {
        self.assembly
    }
/// Enclose original/exact-profile normals per knot span before hulling.
    /// Corrected transitions retain their own expression derivative bounds.
    pub fn enclose_support_normals(
        &self, face:usize, domain:[[f64;2];2], max_controls:usize, max_spans:usize,
    ) -> Result<(Option<[[f64;2];3]>,usize)> {
        // Reuse owner/domain validation, including the exhausted-budget path.
        self.enclose_support_derivatives(face,domain,max_controls,0)?;
        if max_spans>100000 {return Err(Error::new("BREP_PROFILE_SHELL","Bound normal span work"));}
        if max_spans==0 {return Ok((None,0));}
        let root=self.assembly.patch.rails.face.boundary.rooted[0];
        match self.faces[face].kind {
            ShellFaceKind::Original(index)=> {
                let source=root.restricted.proposal.body.geometry().shell();
                nurbs_core::normal_alignment::normal_bounds(source.faces()[index][0].edges()[0].surface(),domain,max_spans)
                    .map_err(Into::into)
            }
            ShellFaceKind::Retained(_)=>root.restricted.proposal.profile(root.face).unwrap()
                .normal_bounds(domain,max_spans).map_err(Into::into),
            ShellFaceKind::Transition=> {
                let (jet,spent)=self.enclose_support_derivatives(face,domain,max_controls,max_spans)?;
                Ok((jet.map(segment::normal_from_derivatives).transpose()?,spent))
            }
        }
    }
/// Direction bounds may divide out a proven nonzero interior pole factor.
    /// Boundary rectangles keep the ordinary normal enclosure and uncertainty.
    pub fn enclose_support_normal_directions(
        &self,face:usize,domain:[[f64;2];2],max_controls:usize,max_spans:usize,
    ) -> Result<(Option<[[f64;2];3]>,usize)> {
        self.enclose_support_derivatives(face,domain,max_controls,0)?;
        if matches!(self.faces[face].kind,ShellFaceKind::Retained(_)) {
            let root=self.assembly.patch.rails.face.boundary.rooted[0];
            return root.restricted.proposal.profile(root.face).unwrap()
                .interior_normal_direction_bounds(domain,max_spans).map_err(Into::into);
        }
        self.enclose_support_normals(face,domain,max_controls,max_spans)
    }
pub fn faces(&self) -> &[ShellFace] {
        &self.faces
    }
pub fn pairs(&self) -> &[ShellPair] {
        &self.pairs
    }
pub fn poles(&self) -> &[Address] {
        &self.poles
    }
pub fn vertices(&self) -> usize {
        self.vertices
    }
/// Qualified shell vertex at a traversal endpoint. IDs are local to this
    /// shell and come from exact incidence, never Cartesian proximity.
    pub fn boundary_vertex(&self, address: Address, end: usize) -> Option<usize> {
        if end > 1 { return None; }
        let wire = self.faces.get(address.face)?.loops.get(address.wire)?;
        if address.edge >= wire.len() { return None; }
        let edge = (address.edge + end) % wire.len();
        self.vertex_ids.get(&(address.face, address.wire, edge)).copied()
    }
pub fn components(&self) -> usize {
        self.components
    }
/// Conservative position enclosure of the exact supporting expression.
    /// This does not assert that the rectangle lies inside its trimmed region.
    /// Original/retained coordinates are (u,v); transition coordinates are
    /// (source free parameter, normalized across parameter).
    pub fn enclose_support(
        &self,
        face: usize,
        domain: [[f64; 2]; 2],
        max_controls: usize,
        max_spans: usize,
    ) -> Result<(Option<[[f64; 2]; 3]>, usize)> {
        use nurbs_core::interval_eval::{Interval, evaluate_surface_interval};
        if max_controls > 1000000
            || max_spans > 100000
            || domain
                .iter()
                .any(|r| !r.iter().all(|x| x.is_finite()) || r[0] > r[1])
        {
            return Err(Error::new(
                "BREP_PROFILE_SHELL",
                "Bound support enclosure parameters and work",
            ));
        }
        let kind = self
            .faces
            .get(face)
            .ok_or_else(|| Error::new("BREP_PROFILE_SHELL", "Unknown expression face"))?
            .kind;
        let boundary = self.assembly.patch.rails.face.boundary;
        let root = boundary.rooted[0];
        let profile = root.restricted.proposal.profile(root.face).unwrap();
        match kind {
            ShellFaceKind::Retained(_) => profile.enclose(domain, max_spans),
            ShellFaceKind::Transition => {
                let first = profile.section(root.axis, root.parameters[0])?;
                let last = profile.section(root.axis, root.parameters[1])?;
                let width = root.parameters[1] - root.parameters[0];
                let bridge = nurbs_core::normal_alignment::ProfileSectionBridge::new(
                    first, last, [width; 2],
                )?;
                let (constrained, _) = bridge.constrain_original_rails(max_controls)?;
                let Some(constrained) = constrained else {
                    return Ok((None, 0));
                };
                constrained.enclose_with_transverse_correction(domain[0], domain[1], max_spans)
            }
            ShellFaceKind::Original(index) => {
                let original = root.restricted.proposal.body.geometry().shell();
                let surface = original.faces()[index][0].edges()[0].surface();
                let spans = [
                    (
                        &surface.knots_u,
                        surface.degree_u,
                        surface.control_points.len(),
                    ),
                    (
                        &surface.knots_v,
                        surface.degree_v,
                        surface.control_points[0].len(),
                    ),
                ];
                let mut counts = [0usize; 2];
                for axis in 0..2 {
                    let (knots, degree, count) = spans[axis];
                    if domain[axis][0] < knots[degree] || domain[axis][1] > knots[count] {
                        return Err(Error::new(
                            "BREP_PROFILE_SHELL",
                            "Keep support rectangle in active knot domain",
                        ));
                    }
                    counts[axis] = (degree..count)
                        .filter(|&i| {
                            knots[i] < knots[i + 1]
                                && knots[i] <= domain[axis][1]
                                && knots[i + 1] >= domain[axis][0]
                        })
                        .count();
                }
                let needed = counts[0] * counts[1];
                if needed > max_spans {
                    return Ok((None, 0));
                }
                let result = evaluate_surface_interval(
                    surface,
                    Interval::new(domain[0][0], domain[0][1])?,
                    Interval::new(domain[1][0], domain[1][1])?,
                )?;
                let bounds = std::array::from_fn(|i| [result[i].lo, result[i].hi]);
                Ok((Some(bounds), needed))
            }
        }
    }
}
