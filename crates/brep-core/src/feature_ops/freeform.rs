//! Native freeform surface features and boundary-preserving solid replacement.
//! Open sheets/proposals and admitted replacements have distinct result types.
use crate::{Error, Model, Result};
use nurbs_core::{
    constrained_surface::{self, Constraint, Options},
    curve::Curve,
    surface::{Axis, Surface},
    surface_g2_pcurves,
    surface_join::{Boundary, Decision},
    surface_linear_monotonicity,
};
fn refused(message: &str) -> Error {
    Error::new("BREP_FREEFORM_FEATURE_REFUSED", message)
}
pub struct FreeformReplacement {
    replacement: crate::face_replacement::Replacement,
    seams: Vec<surface_g2_pcurves::Report>,
    interior: surface_linear_monotonicity::Report,
    audit: crate::solid_audit::SolidAuditCertificate,
}
impl FreeformReplacement {
    pub fn model(&self) -> &Model {
        &self.replacement.model
    }
    pub fn into_model(self) -> Model {
        self.replacement.model
    }
    pub fn seams(&self) -> &[surface_g2_pcurves::Report] {
        &self.seams
    }
    pub fn interior(&self) -> &surface_linear_monotonicity::Report {
        &self.interior
    }
    pub fn audit(&self) -> &crate::solid_audit::SolidAuditCertificate {
        &self.audit
    }
}
/// Solve an interior freeform edit, preserve natural C2 boundary jets, retain
/// every owned original edge/vertex and requalify G2 on ALL actual trim uses,
/// including holes. Low-degree charts are elevated to provide interior freedom.
/// Unresolved injectivity, boundary ownership or regularity refuses atomically.
pub fn replace_constrained(
    source: &Model,
    face: usize,
    constraints: &[Constraint],
    options: Options,
    tolerances: [f64; 3],
    tolerance_uv: f64,
    max_cells: usize,
) -> Result<FreeformReplacement> {
    source.validate()?;
    if !(100..=100000).contains(&max_cells)
        || tolerances.iter().any(|x| !x.is_finite() || *x < 0.)
        || tolerances[0] > source.tolerance_mm
        || tolerances[1] > 2.
        || !tolerance_uv.is_finite()
        || tolerance_uv <= 0.
    {
        return Err(refused("Invalid freeform replacement tolerances/work"));
    }
    let f = source
        .faces
        .get(face)
        .ok_or_else(|| refused("Unknown replacement face"))?;
    let mut seed = f.surface.clone();
    if seed.degree_u < 6 {
        seed = nurbs_core::surface_edit::elevate(&seed, Axis::U, 6)?;
    }
    if seed.degree_v < 6 {
        seed = nurbs_core::surface_edit::elevate(&seed, Axis::V, 6)?;
    }
    let solved = constrained_surface::boundary::solve_preserving_g2(
        &seed,
        &seed,
        &[
            Boundary::UMin,
            Boundary::UMax,
            Boundary::VMin,
            Boundary::VMax,
        ],
        constraints,
        options,
        tolerances,
        max_cells / 4,
    )?;
    let mut remaining = max_cells - solved.seams().iter().map(|(_, r)| r.cells).sum::<usize>();
    let interior = surface_linear_monotonicity::inspect_candidate(solved.surface(), remaining / 4)?;
    remaining = remaining.saturating_sub(interior.cells);
    if !interior.certified {
        return Err(refused("Replacement whole-chart injectivity unproven"));
    }
    let wires = std::iter::once(f.outer)
        .chain(f.holes.iter().copied())
        .collect::<Vec<_>>();
    let pcurves = wires
        .iter()
        .map(|&w| {
            source.loops[w]
                .coedges
                .iter()
                .map(|c| c.pcurve.clone())
                .collect()
        })
        .collect::<Vec<Vec<Curve>>>();
    let replacement = crate::face_replacement::replace(
        source,
        face,
        solved.surface(),
        &pcurves,
        tolerance_uv,
        remaining / 2,
    )?;
    remaining = remaining.saturating_sub(replacement.cells);
    let mut exact_remaining = 1000000u64.saturating_sub(replacement.exact_work);
    let mut count = wires
        .iter()
        .map(|&w| source.loops[w].coedges.len())
        .sum::<usize>();
    let mut seams = Vec::new();
    for &wire in &wires {
        for c in &source.loops[wire].coedges {
            if count == 0 || remaining < count {
                return Err(refused("Trim G2 proof budget exhausted"));
            }
            let report = surface_g2_pcurves::inspect(
                &source.edges[c.edge].curve,
                [&c.pcurve, &c.pcurve],
                [&f.surface, solved.surface()],
                [c.reversed, c.reversed],
                tolerances[0],
                tolerances[1],
                tolerances[2],
                remaining / count,
                exact_remaining,
            )?;
            remaining = remaining.saturating_sub(report.geometric.cells);
            exact_remaining = exact_remaining.saturating_sub(report.positions.exact_work);
            count -= 1;
            if !report.positions.proven
                || report.geometric.decision != Decision::WithinTolerance
                || !report.geometric.regularity_proven
            {
                return Err(refused("Original trim boundary G2/regularity unproven"));
            }
            seams.push(report);
        }
    }
    let audit = crate::solid_audit::audit_solid(&replacement.model)?;
    Ok(FreeformReplacement {
        replacement,
        seams,
        interior,
        audit,
    })
}

/// Author a single open B-rep face with natural boundaries from a retained
/// nonperiodic chart. No solid body or sewing certificate is manufactured.
pub fn sheet(surface: &Surface, tolerance_mm: f64) -> Result<Model> {
    surface.validate()?;
    if surface.periodic_u || surface.periodic_v || surface.control_points[0][0].len() != 3 {
        return Err(refused(
            "Open feature sheet requires a nonperiodic 3D chart",
        ));
    }
    let du = [
        surface.knots_u[surface.degree_u],
        surface.knots_u[surface.control_points.len()],
    ];
    let dv = [
        surface.knots_v[surface.degree_v],
        surface.knots_v[surface.control_points[0].len()],
    ];
    let corners = [
        [du[0], dv[0]],
        [du[1], dv[0]],
        [du[1], dv[1]],
        [du[0], dv[1]],
    ];
    let mut curves = [
        surface.iso(Axis::V, dv[0])?,
        surface.iso(Axis::U, du[1])?,
        surface.iso(Axis::V, dv[1])?.reverse()?,
        surface.iso(Axis::U, du[0])?.reverse()?,
    ];
    let mut boundaries = Vec::new();
    for (i, c) in curves.iter_mut().enumerate() {
        let [a, b] = c.domain();
        *c = c.trim(a, b)?;
        let pc = Curve {
            degree: 1,
            knots: vec![a, a, b, b],
            control_points: vec![corners[i].to_vec(), corners[(i + 1) % 4].to_vec()],
            weights: vec![1.; 2],
            periodic: false,
        };
        let p = &c.control_points[0];
        let collapsed = c
            .control_points
            .iter()
            .all(|q| q == p)
            .then(|| [p[0], p[1], p[2]]);
        boundaries.push(crate::circular_blend::CircularBlendBoundary {
            curve: c.clone(),
            pcurve: pc,
            collapsed_pole: collapsed,
        });
    }
    crate::circular_blend::open_face(
        surface.clone(),
        boundaries
            .try_into()
            .map_err(|_| refused("Missing sheet boundaries"))?,
        tolerance_mm,
    )
}

pub struct OffsetSheet {
    pub model: Model,
    pub geometry: nurbs_core::offset::freeform::RegularRegion,
    /// Sampled approximation evidence, not a continuous NURBS distance bound.
    pub sampled_deviation_mm: f64,
}
pub fn offset_sheet(
    surface: &Surface,
    distance: f64,
    tolerance_mm: f64,
    max_cells: usize,
) -> Result<OffsetSheet> {
    let offset = nurbs_core::offset::freeform::NormalOffset::new(surface, distance)?;
    let geometry = offset.qualify(offset.domain(), max_cells)?;
    let report = offset.approximate(tolerance_mm)?;
    if !report.max_deviation.is_finite()
        || report.max_deviation > tolerance_mm
        || !report.degenerate_regions.is_empty()
    {
        return Err(refused(
            "Offset NURBS proposal fails sampled approximation acceptance",
        ));
    }
    let model = sheet(&report.surface, tolerance_mm)?;
    Ok(OffsetSheet {
        model,
        geometry,
        sampled_deviation_mm: report.max_deviation,
    })
}

#[derive(Clone, Copy)]
pub struct IsoTrace {
    pub axis: Axis,
    pub at: f64,
    pub along: [f64; 2],
    pub reverse: bool,
}
pub struct ChamferSheet {
    model: Model,
    contacts: Vec<nurbs_core::surface_g2_pcurves::Positions>,
    interior: surface_linear_monotonicity::Report,
}
impl ChamferSheet {
    pub fn model(&self) -> &Model {
        &self.model
    }
    pub fn into_model(self) -> Model {
        self.model
    }
    pub fn contacts(&self) -> &[nurbs_core::surface_g2_pcurves::Positions] {
        &self.contacts
    }
    pub fn interior(&self) -> &surface_linear_monotonicity::Report {
        &self.interior
    }
}
/// Freeform ruled chamfer from explicit original-support contact traces.
/// Every actual boundary is continuously qualified on its original support.
/// This returns an open sheet; it does not replace a solid's sharp edge.
pub fn chamfer_sheet(
    supports: [&Surface; 2],
    traces: [IsoTrace; 2],
    tolerance_mm: f64,
    max_cells: usize,
) -> Result<ChamferSheet> {
    if !(10..=100000).contains(&max_cells) || !tolerance_mm.is_finite() || tolerance_mm <= 0. {
        return Err(refused("Invalid chamfer tolerance/budget"));
    }
    let mut rails = Vec::new();
    let mut pcs = Vec::new();
    for i in 0..2 {
        let t = traces[i];
        let mut rail = supports[i]
            .iso(t.axis, t.at)?
            .trim(t.along[0], t.along[1])?;
        let mut endpoints = t.along;
        if t.reverse {
            rail = rail.reverse()?;
            endpoints.reverse();
        }
        let uv = endpoints.map(|x| match t.axis {
            Axis::U => vec![t.at, x],
            Axis::V => vec![x, t.at],
        });
        pcs.push(Curve::from_polyline(uv.to_vec())?);
        rails.push(rail);
    }
    let aligned = nurbs_core::sections::compatible(&rails)?;
    let surface = Surface::ruled_surface(&aligned[0], &aligned[1])?;
    let interior = surface_linear_monotonicity::inspect_candidate(&surface, max_cells / 3)?;
    if !interior.certified {
        return Err(refused("Chamfer chart injectivity unproven"));
    }
    let mut remaining = max_cells - interior.cells;
    let mut exact = 1000000u64;
    let mut contacts = Vec::new();
    for i in 0..2 {
        let curve = surface.iso(Axis::V, i as f64)?;
        let blend_pc = Curve::from_polyline(vec![vec![0., i as f64], vec![1., i as f64]])?;
        let r = nurbs_core::surface_g2_pcurves::verify_positions(
            &curve,
            [&blend_pc, &pcs[i]],
            [&surface, supports[i]],
            [false, false],
            tolerance_mm,
            remaining / (2 - i),
            exact,
        )?;
        remaining = remaining.saturating_sub(r.cells);
        exact = exact.saturating_sub(r.exact_work);
        if !r.proven {
            return Err(refused(
                "Whole chamfer contact with original support unproven",
            ));
        }
        contacts.push(r);
    }
    let model = sheet(&surface, tolerance_mm)?;
    Ok(ChamferSheet {
        model,
        contacts,
        interior,
    })
}

pub struct FilletSheetProposal {
    pub model: Model,
    pub sections: Vec<nurbs_core::offset::freeform_blend::BallSection>,
    /// Local center/contact solve residuals; not a whole-skin radius certificate.
    pub maximum_section_residual_mm: f64,
}
/// Fit an open NURBS sheet to retained rolling-ball sections. The returned name
/// deliberately preserves its proposal status: interpolation does not certify
/// radius/contact between stations, trim supports or sew a solid.
pub fn fillet_sheet_proposal(
    ball: &nurbs_core::offset::freeform_blend::RollingBall,
    range: [f64; 2],
    mut seed: [f64; 4],
    sections: usize,
    tolerance_mm: f64,
) -> Result<FilletSheetProposal> {
    if !(2..=11).contains(&sections) || range.iter().any(|x| !x.is_finite()) || range[0] >= range[1]
    {
        return Err(refused(
            "Fillet sheet needs 2..11 increasing driver stations",
        ));
    }
    let mut parameters = Vec::new();
    let mut curves = Vec::new();
    let mut retained = Vec::new();
    let mut maximum = 0f64;
    for i in 0..sections {
        let t = range[0] + (range[1] - range[0]) * i as f64 / (sections - 1) as f64;
        let section = ball.section(t, seed)?;
        seed = section.parameters();
        maximum = maximum.max(section.offset_residual());
        curves.push(section.curve()?);
        parameters.push(t);
        retained.push(section);
    }
    let surface = nurbs_core::natural_loft::interpolate(&curves, &parameters)?;
    let model = sheet(&surface, tolerance_mm)?;
    Ok(FilletSheetProposal {
        model,
        sections: retained,
        maximum_section_residual_mm: maximum,
    })
}
