//! Planar trimmed-face definitions from exact closed curve chains; no shell claim.
use crate::{planar_trim, prism};
use nurbs_core::{
    Error, Result,
    curve::Curve,
    curve_surface_agreement::{self, Report, Status},
    surface::Surface,
};
#[derive(Clone, Debug)]
pub struct Cap {
    pub surface: Surface,
    /// Outer followed by holes, preserving material-left source orientation.
    pub boundaries: Vec<Vec<Curve>>,
    pub pcurves: Vec<Vec<Curve>>,
    pub agreements: Vec<Vec<Report>>,
}
/// Author independently validated open sheet faces. Each cap owns its boundary
/// topology; sewing to neighbouring faces is a separate operation.
pub fn build_sheet(
    loops: &[Vec<Curve>],
    z_plane: f64,
    tolerance: f64,
    max_verification_cells: usize,
) -> Result<crate::Model> {
    Ok(build_sheet_with_report(loops, z_plane, tolerance, max_verification_cells)?.0)
}
/// Return cumulative continuous agreement cells alongside the authored sheet.
pub fn build_sheet_with_report(
    loops: &[Vec<Curve>],
    z_plane: f64,
    tolerance: f64,
    max_verification_cells: usize,
) -> Result<(crate::Model, usize)> {
    if !(1e-10..=1e-2).contains(&tolerance) {
        return Err(Error::new(
            "BREP_INVALID_PLANAR_CAP",
            "Sheet tolerance must be 1e-10..1e-2 mm",
        ));
    }
    let caps = build(loops, z_plane, tolerance, max_verification_cells)?;
    author_caps(caps, tolerance)
}
fn author_caps(caps: Vec<Cap>, tolerance: f64) -> Result<(crate::Model, usize)> {
    let cells = caps
        .iter()
        .flat_map(|c| &c.agreements)
        .flatten()
        .map(|r| r.cells)
        .sum();
    let mut model = crate::Model::empty(tolerance)?;
    for cap in caps {
        let mut loop_ids = Vec::new();
        for (wire, pcurves) in cap.boundaries.into_iter().zip(cap.pcurves) {
            let base = model.vertices.len();
            let count = wire.len();
            for curve in &wire {
                let p = &curve.control_points[0];
                model.vertices.push(crate::Vertex {
                    point: [p[0], p[1], p[2]],
                });
            }
            let mut coedges = Vec::new();
            for (i, (curve, pcurve)) in wire.into_iter().zip(pcurves).enumerate() {
                let edge = model.edges.len();
                model.edges.push(crate::Edge {
                    vertices: [base + i, base + (i + 1) % count],
                    curve,
                    degenerate: false,
                });
                coedges.push(crate::Coedge {
                    edge,
                    reversed: false,
                    pcurve,
                });
            }
            loop_ids.push(model.loops.len());
            model.loops.push(crate::Loop { coedges });
        }
        let face = model.faces.len();
        model.faces.push(crate::Face {
            surface: cap.surface,
            outer: loop_ids[0],
            holes: loop_ids[1..].to_vec(),
        });
        model.shells.push(crate::Shell {
            faces: vec![crate::FaceUse {
                face,
                reversed: false,
            }],
            closed: false,
        });
    }
    model.rebuild_topology_ids();
    model.validate()?;
    Ok((model, cells))
}
/// Validate simple analytic material-left loops and construct planar caps for
/// each connected component. Inputs must have exact closed ordered endpoints
/// and all original Z controls equal z_plane. Lines/circular arcs are admitted
/// by the existing planar arrangement path. Agreement budget is shared globally.
pub fn build(
    loops: &[Vec<Curve>],
    z_plane: f64,
    tolerance: f64,
    max_verification_cells: usize,
) -> Result<Vec<Cap>> {
    if !z_plane.is_finite()
        || !tolerance.is_finite()
        || tolerance <= 0.
        || loops.is_empty()
        || max_verification_cells > 100000
    {
        return Err(Error::new(
            "BREP_INVALID_PLANAR_CAP",
            "Cap needs finite plane, positive tolerance, nonempty loops and <=100000 verifier cells",
        ));
    }
    let projected = project_loops(loops, z_plane)?;
    let components = planar_trim::components(&projected, tolerance)?;
    construct_caps(
        loops,
        &projected,
        z_plane,
        tolerance,
        max_verification_cells,
        components,
    )
}
fn construct_caps(
    loops: &[Vec<Curve>],
    projected: &[Vec<Curve>],
    z_plane: f64,
    tolerance: f64,
    max_verification_cells: usize,
    components: Vec<(usize, Vec<usize>)>,
) -> Result<Vec<Cap>> {
    let mut caps = Vec::new();
    let mut used = 0;
    for (outer, holes) in components {
        let members: Vec<_> = std::iter::once(outer).chain(holes).collect();
        let mut bounds = [[f64::INFINITY; 2], [f64::NEG_INFINITY; 2]];
        for &m in &members {
            for c in &projected[m] {
                for p in &c.control_points {
                    for j in 0..2 {
                        bounds[0][j] = bounds[0][j].min(p[j]);
                        bounds[1][j] = bounds[1][j].max(p[j]);
                    }
                }
            }
        }
        if (0..2).any(|j| bounds[0][j] >= bounds[1][j]) {
            return Err(Error::new(
                "BREP_INVALID_PLANAR_CAP",
                "Degenerate cap bounds",
            ));
        }
        let surface = prism::cap_surface(bounds, z_plane);
        surface.validate()?;
        let mut boundaries = Vec::new();
        let mut pcurves = Vec::new();
        let mut agreements = Vec::new();
        for m in members {
            let mut uv = Vec::new();
            let mut proofs = Vec::new();
            for (c, p) in loops[m].iter().zip(&projected[m]) {
                if used == max_verification_cells {
                    return Err(Error::new(
                        "BREP_CAP_UNRESOLVED",
                        "Cap agreement budget exhausted",
                    ));
                }
                let pcurve = prism::cap_pcurve(p, bounds);
                pcurve.validate()?;
                let report = curve_surface_agreement::verify(
                    c,
                    &pcurve,
                    &surface,
                    false,
                    tolerance,
                    max_verification_cells - used,
                )?;
                used += report.cells;
                if report.status != Status::WithinTolerance {
                    return Err(Error::new(
                        "BREP_CAP_UNRESOLVED",
                        "Cap pcurve/world boundary agreement is unproved",
                    ));
                }
                uv.push(pcurve);
                proofs.push(report);
            }
            boundaries.push(loops[m].clone());
            pcurves.push(uv);
            agreements.push(proofs);
        }
        caps.push(Cap {
            surface,
            boundaries,
            pcurves,
            agreements,
        });
    }
    Ok(caps)
}

fn project_loops(loops: &[Vec<Curve>], z_plane: f64) -> Result<Vec<Vec<Curve>>> {
    let mut projected = Vec::new();
    for wire in loops {
        if wire.is_empty() {
            return Err(Error::new("BREP_INVALID_PLANAR_CAP", "Empty cap boundary"));
        }
        let mut xy = Vec::new();
        for (i, c) in wire.iter().enumerate() {
            c.validate()?;
            let [a, b] = c.domain();
            if c.control_points[0].len() != 3
                || c.control_points.iter().any(|p| p[2] != z_plane)
                || !c.knots[..=c.degree].iter().all(|&t| t == a)
                || !c.knots[c.knots.len() - c.degree - 1..]
                    .iter()
                    .all(|&t| t == b)
                || c.control_points.last() != wire[(i + 1) % wire.len()].control_points.first()
            {
                return Err(Error::new(
                    "BREP_INVALID_PLANAR_CAP",
                    "Cap requires clamped exactly connected curves on the plane",
                ));
            }
            let mut p = c.clone();
            for point in &mut p.control_points {
                point.pop();
            }
            xy.push(p);
        }
        projected.push(xy);
    }
    Ok(projected)
}

/// General NURBS contours use a continuous trim-region audit. Inputs are one
/// outer loop followed by holes; unresolved stages retain their diagnostics.
pub struct GeneralCap {
    pub cap: Option<Cap>,
    pub trim: nurbs_core::trim_region_audit::Report,
    pub agreement_cells: usize,
    /// True when all input loops were reversed after a proven clockwise winding.
    pub orientation_reversed: bool,
}
pub fn build_general(
    loops: &[Vec<Curve>],
    z_plane: f64,
    tolerance: f64,
    max_trim_pairs: usize,
    max_trim_cells: usize,
    max_domain_cells: usize,
    max_agreement_cells: usize,
) -> Result<GeneralCap> {
    build_general_impl(
        loops,
        z_plane,
        tolerance,
        max_trim_pairs,
        max_trim_cells,
        max_domain_cells,
        max_agreement_cells,
        false,
    )
}
fn build_general_impl(
    loops: &[Vec<Curve>],
    z_plane: f64,
    tolerance: f64,
    max_trim_pairs: usize,
    max_trim_cells: usize,
    max_domain_cells: usize,
    max_agreement_cells: usize,
    auto_orientation: bool,
) -> Result<GeneralCap> {
    if !z_plane.is_finite()
        || !tolerance.is_finite()
        || tolerance <= 0.
        || loops.is_empty()
        || max_agreement_cells > 100000
    {
        return Err(Error::new(
            "BREP_INVALID_PLANAR_CAP",
            "General cap requires finite plane, positive tolerance and bounded agreement cells",
        ));
    }
    let projected = project_loops(loops, z_plane)?;
    let trim = nurbs_core::trim_region_audit::inspect(
        &projected,
        tolerance,
        max_trim_pairs,
        max_trim_cells,
        max_domain_cells,
    )?;
    let mut report = GeneralCap {
        cap: None,
        trim,
        agreement_cells: 0,
        orientation_reversed: false,
    };
    if report.trim.valid != Some(true) {
        return Ok(report);
    }
    let winding = report.trim.winding.first().copied().flatten();
    let reverse = auto_orientation && winding == Some(-1);
    if winding != Some(1) && !reverse {
        return Ok(report);
    }
    let reversed;
    let reversed_projected;
    let (loops, projected) = if reverse {
        reversed = loops
            .iter()
            .map(|wire| {
                wire.iter()
                    .rev()
                    .map(Curve::reverse)
                    .collect::<Result<Vec<_>>>()
            })
            .collect::<Result<Vec<_>>>()?;
        reversed_projected = project_loops(&reversed, z_plane)?;
        (reversed.as_slice(), reversed_projected.as_slice())
    } else {
        (loops, projected.as_slice())
    };
    report.orientation_reversed = reverse;
    let caps = construct_caps(
        loops,
        projected,
        z_plane,
        tolerance,
        max_agreement_cells,
        vec![(0, (1..loops.len()).collect())],
    )?;
    report.agreement_cells = caps
        .iter()
        .flat_map(|c| &c.agreements)
        .flatten()
        .map(|r| r.cells)
        .sum();
    report.cap = caps.into_iter().next();
    Ok(report)
}

/// An audited general contour may remain unresolved without authoring topology.
pub struct GeneralSheet {
    pub model: Option<crate::Model>,
    pub trim: nurbs_core::trim_region_audit::Report,
    pub agreement_cells: usize,
    /// True when all input loops were reversed after a proven clockwise winding.
    pub orientation_reversed: bool,
}
/// Author a single planar face with arbitrary supported NURBS boundaries.
pub fn build_sheet_general(
    loops: &[Vec<Curve>],
    z_plane: f64,
    tolerance: f64,
    max_trim_pairs: usize,
    max_trim_cells: usize,
    max_domain_cells: usize,
    max_agreement_cells: usize,
) -> Result<GeneralSheet> {
    build_sheet_general_impl(
        loops,
        z_plane,
        tolerance,
        max_trim_pairs,
        max_trim_cells,
        max_domain_cells,
        max_agreement_cells,
        false,
    )
}
/// Normalize either proven winding to material-left without a control-polygon heuristic.
/// The trim report describes the original input, and reversal is explicit.
pub fn build_sheet_general_oriented(
    loops: &[Vec<Curve>],
    z_plane: f64,
    tolerance: f64,
    max_trim_pairs: usize,
    max_trim_cells: usize,
    max_domain_cells: usize,
    max_agreement_cells: usize,
) -> Result<GeneralSheet> {
    build_sheet_general_impl(
        loops,
        z_plane,
        tolerance,
        max_trim_pairs,
        max_trim_cells,
        max_domain_cells,
        max_agreement_cells,
        true,
    )
}
fn build_sheet_general_impl(
    loops: &[Vec<Curve>],
    z_plane: f64,
    tolerance: f64,
    max_trim_pairs: usize,
    max_trim_cells: usize,
    max_domain_cells: usize,
    max_agreement_cells: usize,
    auto_orientation: bool,
) -> Result<GeneralSheet> {
    if !(1e-10..=1e-2).contains(&tolerance) {
        return Err(Error::new(
            "BREP_INVALID_PLANAR_CAP",
            "Sheet tolerance must be 1e-10..1e-2 mm",
        ));
    }
    let report = build_general_impl(
        loops,
        z_plane,
        tolerance,
        max_trim_pairs,
        max_trim_cells,
        max_domain_cells,
        max_agreement_cells,
        auto_orientation,
    )?;
    let model = match report.cap {
        Some(cap) => Some(author_caps(vec![cap], tolerance)?.0),
        None => None,
    };
    Ok(GeneralSheet {
        model,
        trim: report.trim,
        agreement_cells: report.agreement_cells,
        orientation_reversed: report.orientation_reversed,
    })
}
