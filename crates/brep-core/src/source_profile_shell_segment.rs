//! Finite-segment exclusion on complete exact expression support domains.
//! Possible contacts are not isolated roots; material parity remains separate.
use super::*;
pub struct ExpressionSegmentCandidate {
    pub face: usize,
    pub uv: [[f64; 2]; 2],
    pub parameter: [f64; 2],
}
pub struct ExpressionSegmentReport {
    pub boundary_excluded: bool,
    pub candidates: Vec<ExpressionSegmentCandidate>,
    pub pending: Vec<(usize, [[f64; 2]; 2])>,
    pub cells: usize,
    pub spans: usize,
    pub reason: &'static str,
}
impl ExpressionShell<'_, '_, '_, '_, '_, '_, '_, '_> {
    /// Trim supersets conservatively retain possible contacts. No interior
    /// material or thickness certificate can be issued from this report.
    pub fn inspect_segment_supports(
        &self,
        origin: [f64; 3],
        direction: [f64; 3],
        tolerance_uv: f64,
        controls: usize,
        max_cells: usize,
        max_spans: usize,
    ) -> Result<ExpressionSegmentReport> {
        if !origin.iter().chain(&direction).all(|v| v.is_finite())
            || direction.iter().all(|v| *v == 0.)
            || !tolerance_uv.is_finite()
            || tolerance_uv <= 0.
            || tolerance_uv > 1.
            || controls > 1000000
            || max_cells > 100000
            || max_spans > 1000000
        {
            return Err(Error::new(
                "BREP_EXPRESSION_SEGMENT",
                "Choose a finite segment and bounded coverage work",
            ));
        }
        let natural = (0..self.faces.len())
            .map(|face| self.support_domain(face))
            .collect::<Result<Vec<_>>>()?;
        let mut out = ExpressionSegmentReport {
            boundary_excluded: false,
            candidates: Vec::new(),
            pending: natural.iter().copied().enumerate().rev().collect(),
            cells: 0,
            spans: 0,
            reason: "expression-segment-work-limit",
        };
        while let Some((face, uv)) = out.pending.pop() {
            if out.cells == max_cells || out.spans == max_spans {
                out.pending.push((face, uv));
                return Ok(out);
            }
            out.cells += 1;
            let (position, used) =
                self.enclose_support(face, uv, controls, (max_spans - out.spans).min(100000))?;
            out.spans += used;
            let Some(position) = position else {
                out.pending.push((face, uv));
                return Ok(out);
            };
            let Some(parameter) = nurbs_core::ray_surface::finite_line_enclosure_parameters(
                position, origin, direction,
            )?
            else {
                continue;
            };
            let width = std::array::from_fn::<_, 2, _>(|axis| {
                (uv[axis][1] - uv[axis][0]) / (natural[face][axis][1] - natural[face][axis][0])
            });
            if width.iter().all(|w| *w <= tolerance_uv) {
                out.candidates.push(ExpressionSegmentCandidate {
                    face,
                    uv,
                    parameter,
                });
                continue;
            }
            let axis = usize::from(width[1] > width[0]);
            let midpoint = uv[axis][0] * 0.5 + uv[axis][1] * 0.5;
            if midpoint <= uv[axis][0] || midpoint >= uv[axis][1] {
                out.pending.push((face, uv));
                out.reason = "expression-segment-resolution-limit";
                return Ok(out);
            }
            let mut left = uv;
            let mut right = uv;
            left[axis][1] = midpoint;
            right[axis][0] = midpoint;
            out.pending.push((face, right));
            out.pending.push((face, left));
        }
        out.boundary_excluded = out.candidates.is_empty();
        out.reason = if out.boundary_excluded {
            "expression-segment-boundary-excluded"
        } else {
            "expression-segment-contacts-not-isolated"
        };
        Ok(out)
    }
}

/// Infinite-line support roots. Trim membership and crossing orientation are
/// deliberately separate from isolation; retained expressions stay pending.
pub struct ExpressionLineReport {
    pub roots: Vec<(usize, nurbs_core::ray_surface::Root)>,
    pub pending: Vec<(usize, [[f64; 2]; 2])>,
    pub cells: usize,
    pub spans: usize,
    pub coefficient_work: usize,
}
impl ExpressionShell<'_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn inspect_line_support_roots(
        &self,
        origin: [f64; 3],
        direction: [f64; 3],
        tolerance_uv: f64,
        controls: usize,
        max_cells: usize,
        max_spans: usize,
        max_work: usize,
    ) -> Result<ExpressionLineReport> {
        if !origin.iter().chain(&direction).all(|v| v.is_finite())
            || direction.iter().all(|v| *v == 0.)
            || !tolerance_uv.is_finite()
            || tolerance_uv <= 0.
            || tolerance_uv > 1.
            || controls > 1000000
            || max_cells > 100000
            || max_spans > 1000000
            || max_work > 100000000
        {
            return Err(Error::new(
                "BREP_EXPRESSION_LINE",
                "Choose finite line parameters and bounded work",
            ));
        }
        let mut out = ExpressionLineReport {
            roots: Vec::new(),
            pending: Vec::new(),
            cells: 0,
            spans: 0,
            coefficient_work: 0,
        };
        let root = self.assembly.patch.rails.face.boundary.rooted[0];
        let profile = root.restricted.proposal.profile(root.face).unwrap();
        for (face, entry) in self.faces.iter().enumerate() {
            let domain = self.support_domain(face)?;
            if out.cells == max_cells {
                out.pending.push((face, domain));
                continue;
            }
            let (position, used) =
                self.enclose_support(face, domain, controls, (max_spans - out.spans).min(100000))?;
            out.spans += used;
            if let Some(position) = position {
                if nurbs_core::ray_surface::infinite_line_enclosure_parameters(
                    position, origin, direction,
                )?
                .is_none()
                {
                    continue;
                }
            }
            let report = match entry.kind {
                ShellFaceKind::Original(index) => {
                    let original = root.restricted.proposal.body.geometry().shell();
                    let surface = original.faces()[index][0].edges()[0].surface();
                    let count = |knots: &[f64], degree: usize, n: usize| {
                        (degree..n).filter(|&i| knots[i] < knots[i + 1]).count()
                    };
                    let initial = count(
                        &surface.knots_u,
                        surface.degree_u,
                        surface.control_points.len(),
                    )
                    .saturating_mul(count(
                        &surface.knots_v,
                        surface.degree_v,
                        surface.control_points[0].len(),
                    ));
                    if initial > max_cells - out.cells {
                        None
                    } else {
                        Some(nurbs_core::ray_surface::intersections(
                            surface,
                            origin,
                            direction,
                            tolerance_uv,
                            max_cells - out.cells,
                        )?)
                    }
                }
                ShellFaceKind::Transition => {
                    let bridge = nurbs_core::normal_alignment::ProfileSectionBridge::new(
                        profile.section(root.axis, root.parameters[0])?,
                        profile.section(root.axis, root.parameters[1])?,
                        [root.parameters[1] - root.parameters[0]; 2],
                    )?;
                    let (bridge, _) = bridge.constrain_original_rails(controls)?;
                    match bridge {
                        Some(bridge) => {
                            let line = bridge.inspect_line_intersections(
                                domain,
                                origin,
                                direction,
                                tolerance_uv,
                                max_cells - out.cells,
                                max_spans - out.spans,
                                max_work - out.coefficient_work,
                            )?;
                            out.spans += line.spans;
                            out.coefficient_work += line.coefficient_work;
                            line.intersections
                        }
                        None => None,
                    }
                }
                ShellFaceKind::Retained(_) => {
                    let line = profile.inspect_line_intersections(
                        domain,
                        origin,
                        direction,
                        tolerance_uv,
                        max_cells - out.cells,
                        max_spans - out.spans,
                        max_work - out.coefficient_work,
                    )?;
                    out.spans += line.spans;
                    out.coefficient_work += line.coefficient_work;
                    line.intersections
                }
            };
            match report {
                Some(report) => {
                    out.cells += report.cells;
                    out.roots
                        .extend(report.roots.into_iter().map(|r| (face, r)));
                    out.pending
                        .extend(report.unresolved.into_iter().map(|uv| (face, uv)));
                }
                None => out.pending.push((face, domain)),
            }
        }
        Ok(out)
    }
}

pub struct ExpressionTrimmedLineReport {
    pub roots: Vec<(usize, nurbs_core::ray_surface::Root)>,
    pub pending: Vec<(usize, [[f64; 2]; 2])>,
    pub cells: usize,
    pub spans: usize,
    pub coefficient_work: usize,
    pub domain_cells: usize,
}
impl ExpressionShell<'_, '_, '_, '_, '_, '_, '_, '_> {
    /// Only complete owned contour winding can exclude an entire rectangle.
    /// Mixed transition/source loops and exhausted work remain candidates.
    pub fn inspect_support_trim_outside(
        &self, face:usize, uv:[[f64;2];2], tolerance_uv:f64, max_cells:usize,
    ) -> Result<(Option<bool>,usize)> {
        self.enclose_support_derivatives(face,uv,0,0)?;
        if !tolerance_uv.is_finite() || tolerance_uv<=0. || max_cells>100000 {
            return Err(Error::new("BREP_EXPRESSION_TRIM","Bound trim tolerance and work"));
        }
        if max_cells==0 || self.faces[face].kind==ShellFaceKind::Transition {
            return Ok((None,0));
        }
        let mut loops=Vec::new();
        for wire in &self.faces[face].loops {
            let mut fragments=Vec::new();
            for usage in wire {
                match usage {
                    ShellUse::Source(fragment)=>fragments.push(fragment.clone()),
                    ShellUse::Transition(_)=>return Ok((None,0)),
                }
            }
            loops.push(fragments);
        }
        if loops.is_empty() || loops.len()>16 || loops.iter().map(Vec::len).sum::<usize>()>256 {
            return Ok((None,0));
        }
        if loops.iter().flatten().any(|f|f.curve().degree>25 || f.curve().control_points.len()>256) {
            return Ok((None,0));
        }
        let report=crate::source_contour_winding::classify(&loops,uv,tolerance_uv,max_cells)?;
        let outside=match report.location {
            nurbs_core::trim_domain::Location::Outside=>Some(true),
            nurbs_core::trim_domain::Location::Inside=>Some(false),
            _=>None,
        };
        Ok((outside,report.cells))
    }
}
impl ExpressionShell<'_, '_, '_, '_, '_, '_, '_, '_> {
    /// Fresh root isolation followed by owned trim classification. This does
    /// not assert transverse crossings, material parity or a solid certificate.
    pub fn inspect_trimmed_line_roots(
        &self,
        origin: [f64; 3],
        direction: [f64; 3],
        tolerance_uv: f64,
        controls: usize,
        max_cells: usize,
        max_spans: usize,
        max_work: usize,
        max_domain_cells: usize,
    ) -> Result<ExpressionTrimmedLineReport> {
        if max_domain_cells > 1000000 {
            return Err(Error::new(
                "BREP_EXPRESSION_LINE",
                "Bound trim classification work",
            ));
        }
        let support = self.inspect_line_support_roots(
            origin,
            direction,
            tolerance_uv,
            controls,
            max_cells,
            max_spans,
            max_work,
        )?;
        let mut out = ExpressionTrimmedLineReport {
            roots: Vec::new(),
            pending: Vec::new(),
            cells: support.cells,
            spans: support.spans,
            coefficient_work: support.coefficient_work,
            domain_cells: 0,
        };
        let root = self.assembly.patch.rails.face.boundary.rooted[0];
        let source = root.restricted.proposal.body.geometry().shell();
        let mut classify = |face: usize, uv: [[f64; 2]; 2]| -> Result<Option<bool>> {
            let domain = self.support_domain(face)?;
            if self.faces[face].kind == ShellFaceKind::Transition {
                return Ok(
                    if (0..2).all(|k| uv[k][0] > domain[k][0] && uv[k][1] < domain[k][1]) {
                        Some(true)
                    } else {
                        None
                    },
                );
            }
            if out.domain_cells == max_domain_cells {
                return Ok(None);
            }
            let mut loops = Vec::new();
            for wire in &self.faces[face].loops {
                let mut fragments = Vec::new();
                for usage in wire {
                    match usage {
                        ShellUse::Source(f) => fragments.push(f.clone()),
                        ShellUse::Transition(_) => return Ok(None),
                    }
                }
                loops.push(fragments);
            }
            let report = crate::source_contour_winding::classify(
                &loops,
                uv,
                tolerance_uv,
                (max_domain_cells - out.domain_cells).min(100000),
            )?;
            out.domain_cells += report.cells;
            use nurbs_core::trim_domain::Location;
            let original = match self.faces[face].kind {
                ShellFaceKind::Original(i) => i,
                _ => root.face,
            };
            Ok(if report.location == Location::Outside {
                Some(false)
            } else if report.location == Location::Inside
                && report.winding == Some(source.regions().unwrap()[original].chart_winding())
            {
                Some(true)
            } else {
                None
            })
        };
        for (face, uv) in support.pending {
            if classify(face, uv)? != Some(false) {
                out.pending.push((face, uv));
            }
        }
        for (face, root) in support.roots {
            match classify(face, root.uv)? {
                Some(true) => out.roots.push((face, root)),
                Some(false) => {}
                None => out.pending.push((face, root.uv)),
            }
        }
        let mut crossings = Vec::new();
        for (face, root) in out.roots.drain(..) {
            let (derivatives, used) = self.enclose_support_derivatives(
                face,
                root.uv,
                controls,
                (max_spans - out.spans).min(100000),
            )?;
            out.spans += used;
            if let Some(derivatives) = derivatives {
                if transverse(derivatives, direction)? {
                    crossings.push((face, root));
                    continue;
                }
            }
            out.pending.push((face, root.uv));
        }
        crossings.sort_by(|a, b| a.1.parameter[0].total_cmp(&b.1.parameter[0]));
        let ambiguous = (0..crossings.len())
            .map(|i| {
                (i > 0 && crossings[i - 1].1.parameter[1] >= crossings[i].1.parameter[0])
                    || (i + 1 < crossings.len()
                        && crossings[i].1.parameter[1] >= crossings[i + 1].1.parameter[0])
            })
            .collect::<Vec<_>>();
        for (i, (face, root)) in crossings.into_iter().enumerate() {
            if ambiguous[i] {
                out.pending.push((face, root.uv));
            } else {
                out.roots.push((face, root));
            }
        }
        Ok(out)
    }
}

pub(super) fn normal_from_derivatives(derivatives: [[[f64; 2]; 3]; 2]) -> Result<[[f64; 2]; 3]> {
    use nurbs_core::interval_eval::Interval as I;
    let mut normal = [[0.; 2]; 3];
    for k in 0..3 {
        let a = (k + 1) % 3;
        let b = (k + 2) % 3;
        let n = I::new(derivatives[0][a][0], derivatives[0][a][1])?
            .mul(I::new(derivatives[1][b][0], derivatives[1][b][1])?)?
            .sub(
                I::new(derivatives[0][b][0], derivatives[0][b][1])?
                    .mul(I::new(derivatives[1][a][0], derivatives[1][a][1])?)?,
            )?;
        normal[k] = [n.lo, n.hi];
    }
    Ok(normal)
}
fn transverse(derivatives: [[[f64; 2]; 3]; 2], direction: [f64; 3]) -> Result<bool> {
    use nurbs_core::interval_eval::Interval as I;
    let normal = normal_from_derivatives(derivatives)?;
    let mut dot = I::point(0.);
    for k in 0..3 {
        dot = dot.add(I::new(normal[k][0], normal[k][1])?.mul(I::point(direction[k]))?)?;
    }
    Ok(dot.lo > 0. || dot.hi < 0.)
}

#[cfg(test)]
mod line_tests {
    use super::*;
    #[test]
    fn transverse_requires_strict_whole_interval_sign() {
        let plane = [
            [[1., 1.], [0., 0.], [0., 0.]],
            [[0., 0.], [1., 1.], [0., 0.]],
        ];
        assert!(transverse(plane, [0., 0., 1.]).unwrap());
        assert!(transverse(plane, [0., 0., -1.]).unwrap());
        assert!(!transverse(plane, [1., 0., 0.]).unwrap());
        let uncertain = [
            [[1., 1.], [0., 0.], [0., 0.]],
            [[0., 0.], [-1., 1.], [0., 0.]],
        ];
        assert!(!transverse(uncertain, [0., 0., 1.]).unwrap());
    }
}

#[derive(Clone)]
pub struct ExpressionMaterialInterval {
    pub entry: (usize, nurbs_core::ray_surface::Root),
    pub exit: (usize, nurbs_core::ray_surface::Root),
}
pub struct ExpressionMaterialLineReport {
    pub boundary: ExpressionTrimmedLineReport,
    pub intervals: Option<Vec<ExpressionMaterialInterval>>,
    pub reason: &'static str,
}
impl EmbeddedExpressionGeometry<'_, '_, '_, '_, '_, '_, '_, '_, '_> {
    /// Complete positive-ray parity from an exterior hull seed on this owned,
    /// embedded shell. Interval endpoints are uncertain root bands. A local
    /// material interval is not a certificate of minimum wall thickness.
    pub fn inspect_material_line(
        &self,
        origin: [f64; 3],
        direction: [f64; 3],
        tolerance_uv: f64,
        controls: usize,
        cells: usize,
        spans: usize,
        work: usize,
        domain_cells: usize,
    ) -> Result<ExpressionMaterialLineReport> {
        let shell = self.shell();
        let boundary = shell.inspect_trimmed_line_roots(
            origin,
            direction,
            tolerance_uv,
            controls,
            cells,
            spans,
            work,
            domain_cells,
        )?;
        let mut out = ExpressionMaterialLineReport {
            boundary,
            intervals: None,
            reason: "expression-material-seed-unresolved",
        };
        let mut hull = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
        for face in 0..shell.faces.len() {
            let (position, used) = shell.enclose_support(
                face,
                shell.support_domain(face)?,
                controls,
                (spans - out.boundary.spans).min(100000),
            )?;
            out.boundary.spans += used;
            let Some(position) = position else {
                return Ok(out);
            };
            for k in 0..3 {
                hull[k][0] = hull[k][0].min(position[k][0]);
                hull[k][1] = hull[k][1].max(position[k][1]);
            }
        }
        if !(0..3).any(|k| origin[k] < hull[k][0] || origin[k] > hull[k][1]) {
            return Ok(out);
        }
        if !out.boundary.pending.is_empty() {
            out.reason = "expression-material-boundary-unresolved";
            return Ok(out);
        }
        let mut positive = Vec::new();
        let audit_roots = out
            .boundary
            .roots
            .iter()
            .map(|(face, root)| (*face, root.clone()))
            .collect::<Vec<_>>();
        for (face, root) in audit_roots {
            if root.parameter[1] < 0. {
                continue;
            }
            if root.parameter[0] <= 0. {
                out.boundary.pending.push((face, root.uv));
                out.reason = "expression-material-seed-band";
                return Ok(out);
            }
            positive.push((face, root));
        }
        if positive.len() % 2 != 0 {
            out.reason = "expression-material-odd-crossings";
            return Ok(out);
        }
        let mut roots = positive.into_iter();
        let mut intervals = Vec::new();
        while let Some(entry) = roots.next() {
            intervals.push(ExpressionMaterialInterval {
                entry,
                exit: roots.next().unwrap(),
            });
        }
        out.intervals = Some(intervals);
        out.reason = "expression-material-intervals-qualified";
        Ok(out)
    }
}

/// Owned material intervals from fresh whole-boundary evidence. This does not
/// certify minimum thickness, endpoint normal alignment or a general Body.
pub struct ExpressionMaterialLine<'v, 'g, 'x, 'o, 'i, 'f, 'a, 's, 'p, 'b> {
    geometry: &'v EmbeddedExpressionGeometry<'g, 'x, 'o, 'i, 'f, 'a, 's, 'p, 'b>,
    origin: [f64; 3],
    direction: [f64; 3],
    intervals: Vec<ExpressionMaterialInterval>,
}
impl ExpressionMaterialLine<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn geometry(&self) -> &EmbeddedExpressionGeometry<'_, '_, '_, '_, '_, '_, '_, '_, '_> {
        self.geometry
    }
    pub fn origin(&self) -> [f64; 3] {
        self.origin
    }
    pub fn direction(&self) -> [f64; 3] {
        self.direction
    }
    pub fn intervals(&self) -> &[ExpressionMaterialInterval] {
        &self.intervals
    }
}
pub struct ExpressionMaterialLineQualification<'v, 'g, 'x, 'o, 'i, 'f, 'a, 's, 'p, 'b> {
    pub report: ExpressionMaterialLineReport,
    pub certificate: Option<ExpressionMaterialLine<'v, 'g, 'x, 'o, 'i, 'f, 'a, 's, 'p, 'b>>,
}
impl<'g, 'x, 'o, 'i, 'f, 'a, 's, 'p, 'b>
    EmbeddedExpressionGeometry<'g, 'x, 'o, 'i, 'f, 'a, 's, 'p, 'b>
{
    /// Recompute fresh immutable owner evidence. Caller reports cannot issue
    /// a certificate, including a report whose pending fields were cleared.
    pub fn qualify_material_line<'v>(
        &'v self,
        origin: [f64; 3],
        direction: [f64; 3],
        tolerance_uv: f64,
        controls: usize,
        cells: usize,
        spans: usize,
        work: usize,
        domain_cells: usize,
    ) -> Result<ExpressionMaterialLineQualification<'v, 'g, 'x, 'o, 'i, 'f, 'a, 's, 'p, 'b>> {
        let report = self.inspect_material_line(
            origin,
            direction,
            tolerance_uv,
            controls,
            cells,
            spans,
            work,
            domain_cells,
        )?;
        let certificate = report
            .intervals
            .as_ref()
            .map(|intervals| ExpressionMaterialLine {
                geometry: self,
                origin,
                direction,
                intervals: intervals.clone(),
            });
        Ok(ExpressionMaterialLineQualification {
            report,
            certificate,
        })
    }
}

fn material_interval_length(
    origin: [f64; 3],
    direction: [f64; 3],
    entry: [f64; 2],
    exit: [f64; 2],
) -> Result<[f64; 2]> {
    use nurbs_core::interval_eval::Interval as I;
    let point = |parameter: [f64; 2]| -> Result<Vec<[f64; 2]>> {
        (0..3)
            .map(|k| {
                let x = I::point(origin[k])
                    .add(I::new(parameter[0], parameter[1])?.mul(I::point(direction[k]))?)?;
                Ok([x.lo, x.hi])
            })
            .collect()
    };
    let (lo, hi) = nurbs_core::surface_distance::enclosure_distance(&point(entry)?, &point(exit)?)?;
    Ok([lo, hi])
}
impl ExpressionMaterialLine<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    /// Outward distance bounds between the uncertain endpoint positions of
    /// each material interval. These are local chord lengths, not whole-wall
    /// minima and not normal-thickness admission.
    pub fn lengths_mm(&self) -> Result<Vec<[f64; 2]>> {
        self.intervals
            .iter()
            .map(|interval| {
                material_interval_length(
                    self.origin,
                    self.direction,
                    interval.entry.1.parameter,
                    interval.exit.1.parameter,
                )
            })
            .collect()
    }
}
#[cfg(test)]
mod material_length_tests {
    use super::*;
    #[test]
    fn chord_length_preserves_endpoint_uncertainty_and_direction_scale() {
        let exact = material_interval_length([0.; 3], [3., 4., 0.], [0.25; 2], [0.75; 2]).unwrap();
        assert!(exact[0] <= 2.5 && exact[1] >= 2.5);
        let band = material_interval_length([0.; 3], [3., 4., 0.], [0.2, 0.3], [0.7, 0.8]).unwrap();
        assert!(band[0] <= 2. && band[1] >= 3.);
        let reversed =
            material_interval_length([3., 4., 0.], [-3., -4., 0.], [0.25; 2], [0.75; 2]).unwrap();
        assert!(reversed[0] <= 2.5 && reversed[1] >= 2.5);
    }
}

pub struct ExpressionEndpointNormalReport {
    pub normals: Vec<[Option<nurbs_core::normal_alignment::Report>; 2]>,
    pub spans: usize,
    pub complete: bool,
}
impl ExpressionMaterialLine<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    /// Fresh whole-root-rectangle normal alignment on the same immutable
    /// geometry. A successful local report does not cover the whole wall.
    pub fn inspect_endpoint_normals(
        &self,
        max_sine_squared: f64,
        controls: usize,
        max_spans: usize,
    ) -> Result<ExpressionEndpointNormalReport> {
        if !max_sine_squared.is_finite()
            || !(0. ..1.).contains(&max_sine_squared)
            || controls > 1000000
            || max_spans > 1000000
        {
            return Err(Error::new(
                "BREP_EXPRESSION_CHORD",
                "Choose bounded normal work and an angle tolerance below one",
            ));
        }
        let mut out = ExpressionEndpointNormalReport {
            normals: Vec::new(),
            spans: 0,
            complete: true,
        };
        for interval in &self.intervals {
            let mut normals = [None, None];
            for (end, (face, root)) in [&interval.entry, &interval.exit].into_iter().enumerate() {
                let (derivatives, used) = self.geometry.shell().enclose_support_derivatives(
                    *face,
                    root.uv,
                    controls,
                    (max_spans - out.spans).min(100000),
                )?;
                out.spans += used;
                if let Some(derivatives) = derivatives {
                    let mut report = nurbs_core::normal_alignment::inspect_normal_enclosure(
                        normal_from_derivatives(derivatives)?,
                        self.direction,
                        max_sine_squared,
                    )?;
                    report.spans = used;
                    if report.aligned != Some(true) {
                        out.complete = false;
                    }
                    normals[end] = Some(report);
                } else {
                    out.complete = false;
                }
            }
            out.normals.push(normals);
        }
        Ok(out)
    }
}
