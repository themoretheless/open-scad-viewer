//! Original-support G2 boundary collars for a six-sided corner fill.
//! These establish boundary data, not the central spanning surface.
use super::{TrihedralStrips, build_trihedral_rim, inspect_rim_supports, inspect_spatial_rim};
use nurbs_core::{Error, Result, curve::Curve, surface::Surface, surface_collar};
pub struct CornerCollar {
    pub curve: Curve,
    pub original_surface: Surface,
    pub original_pcurve: Curve,
    pub direction: [f64; 2],
    pub surface: Surface,
    pub g2: nurbs_core::surface_g2_pcurves::OwnedInspection,
}
pub struct CornerCollars {
    pub boundaries: Vec<CornerCollar>,
    pub cells: usize,
    pub coefficient_work: usize,
    pub exact_work: u64,
}
fn refuse(message: &str) -> Error {
    Error::new("BREP_CORNER_COLLAR_UNPROVEN", message)
}
fn direction(s: &Surface, p: &Curve, width: f64) -> Result<[f64; 2]> {
    let d = [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ];
    let a = &p.control_points[0];
    let b = p.control_points.last().unwrap();
    let delta = [b[0] - a[0], b[1] - a[1]];
    let length = delta[0].hypot(delta[1]);
    if !(length > 0. && length.is_finite()) {
        return Err(refuse("Degenerate boundary chart chord"));
    }
    let scale = width * (d[0][1] - d[0][0]).min(d[1][1] - d[1][0]);
    let proposal = [-delta[1] / length * scale, delta[0] / length * scale];
    // An oblique clipped pcurve can touch both natural U boundaries with
    // slightly different V endpoints. Its chord-normal then leaves one edge
    // for either sign. A coordinate displacement preserves the owned U edge;
    // fresh G2 admission below still has to prove transverse regularity.
    for proposal in [proposal, [scale, 0.], [0., scale]] {
        for sign in [1., -1.] {
            let candidate = proposal.map(|x| sign * x);
            if p.control_points.iter().all(|p| {
                (0..2).all(|k| d[k][0] <= p[k] + candidate[k] && p[k] + candidate[k] <= d[k][1])
            }) {
                return Ok(candidate);
            }
        }
    }
    Err(refuse(
        "No proposed transverse collar stays in original natural chart",
    ))
}
// Trim is a rounded proposal only. The returned collar is always checked
// directly against the complete original support, not this extracted patch.
fn local_proposal(s: &Surface, p: &Curve, direction: [f64; 2]) -> Result<Surface> {
    let mut ranges = [[0.; 2]; 2];
    for axis in 0..2 {
        let low = p
            .control_points
            .iter()
            .map(|p| p[axis].min(p[axis] + direction[axis]))
            .fold(f64::INFINITY, f64::min);
        let high = p
            .control_points
            .iter()
            .map(|p| p[axis].max(p[axis] + direction[axis]))
            .fold(f64::NEG_INFINITY, f64::max);
        let (knots, degree, count) = if axis == 0 {
            (&s.knots_u, s.degree_u, s.control_points.len())
        } else {
            (&s.knots_v, s.degree_v, s.control_points[0].len())
        };
        let found = (degree..count)
            .find(|&i| knots[i] < knots[i + 1] && knots[i] <= low && high <= knots[i + 1]);
        let i = found.ok_or_else(|| refuse("Collar proposal crosses an original knot line"))?;
        ranges[axis] = [knots[i], knots[i + 1]];
    }
    s.trim([ranges[0][0], ranges[0][1], ranges[1][0], ranges[1][1]])
}
/// All six boundary curves, support membership and original-support G2 are
/// freshly inspected. No supplied certificate or mutable admission flag is used.
/// Width is a fraction of the smaller original natural chart span.
pub fn prepare_trihedral_collars(
    source: &crate::Model,
    strips: &TrihedralStrips,
    tolerances: [f64; 3],
    tolerance_uv: f64,
    width: f64,
    max_cells: usize,
    max_coefficients: usize,
    max_exact: u64,
) -> Result<CornerCollars> {
    if !(width.is_finite() && width > 0. && width <= 0.1)
        || !(100..=100000).contains(&max_cells)
        || max_exact > 1000000
        || max_coefficients == 0
        || tolerances[0] <= 0.
        || tolerances[0] > source.tolerance_mm
    {
        return Err(refuse(
            "Need bounded collar width/work and unchanged positional tolerance",
        ));
    }
    let rim = build_trihedral_rim(strips, tolerances)?;
    let mut cells = 0;
    let supports = inspect_rim_supports(source, &rim, tolerance_uv, max_cells / 3)?;
    cells += supports.cells;
    let simple = inspect_spatial_rim(&rim.ordered, tolerances[0], (max_cells - cells) / 3)?;
    cells += simple.cells;
    if !simple.proven_simple {
        return Err(refuse("Original spatial rim simplicity is unproven"));
    }
    let mut originals = Vec::new();
    for (i, c) in rim.cross_sections.iter().enumerate() {
        let strip = &strips.strips[i];
        let near = usize::from(strip.end_order[0] != 0);
        originals.push((
            c.clone(),
            strip.patch.model.faces[0].surface.clone(),
            strip.patch.ends[near].skin_pcurve.clone(),
        ));
    }
    for (i, c) in rim.connectors.iter().enumerate() {
        originals.push((
            c.curve.clone(),
            source.faces[c.source_face].surface.clone(),
            supports.pcurves[i].clone(),
        ));
    }
    let mut out = CornerCollars {
        boundaries: Vec::new(),
        cells,
        coefficient_work: 0,
        exact_work: 0,
    };
    for boundary in &rim.ordered {
        let mut matches = Vec::new();
        for (c, s, p) in &originals {
            if c == boundary {
                matches.push((s.clone(), p.clone()));
            } else if c.reverse()? == *boundary {
                matches.push((s.clone(), p.reverse()?));
            }
        }
        if matches.len() != 1 {
            return Err(refuse("Boundary support definition is not uniquely owned"));
        }
        let (s, p) = matches.remove(0);
        let direction = direction(&s, &p, width)?;
        let remaining = max_coefficients.saturating_sub(out.coefficient_work);
        if remaining == 0 {
            return Err(refuse("Collar construction work exhausted"));
        }
        let local = local_proposal(&s, &p, direction)?;
        let built = surface_collar::construct(
            &local,
            &p,
            direction,
            tolerances[0] / 4.,
            remaining.min(2000000),
        )?;
        out.coefficient_work += built.coefficient_work;
        let remaining = max_coefficients.saturating_sub(out.coefficient_work);
        if remaining == 0 || out.cells >= max_cells {
            return Err(refuse("No original collar proof work remains"));
        }
        let collar_pcurve = Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.]])?;
        // The composed Bernstein owner keeps source/candidate jets correlated.
        // It freshly rechecks the ORIGINAL support and actual saved candidate;
        // construction provenance alone never supplies positive geometry.
        let clamped = |knots: &[f64], degree: usize, n: usize| {
            n == degree + 1
                && knots[..=degree].iter().all(|&v| v == knots[degree])
                && knots[n..].iter().all(|&v| v == knots[n])
        };
        let composed = clamped(&p.knots, p.degree, p.control_points.len())
            && !s.periodic_u
            && !s.periodic_v
            && p.degree.saturating_mul(s.degree_u + s.degree_v) <= 16;
        let proof = if composed {
            let inspected = nurbs_core::surface_g2_pcurves::inspect_owned_collar(
                boundary,
                &p,
                &s,
                direction,
                &built.surface,
                tolerances,
                max_cells - out.cells,
                max_exact.saturating_sub(out.exact_work).min(20000),
                remaining.min(2000000),
            )?;
            out.coefficient_work += inspected.coefficient_work;
            inspected.owned
        } else {
            nurbs_core::surface_g2_pcurves::inspect_owned(
                boundary,
                [&p, &collar_pcurve],
                [&s, &built.surface],
                [false, false],
                tolerances[0],
                tolerances[1],
                tolerances[2],
                max_cells - out.cells,
                max_exact.saturating_sub(out.exact_work).min(20000),
            )?
        };
        out.cells += proof.report.geometric.cells;
        out.exact_work += proof.report.positions.exact_work;
        if proof.positions.is_none()
            || proof.report.geometric.decision
                != nurbs_core::surface_join::Decision::WithinTolerance
        {
            return Err(refuse(&format!(
                "Original support seam {} remains unproven: {:?}, positions {}",
                out.boundaries.len(), proof.report.geometric, proof.report.positions.proven
            )));
        }
        out.boundaries.push(CornerCollar {
            curve: boundary.clone(),
            original_surface: s,
            original_pcurve: p,
            direction,
            surface: built.surface,
            g2: proof,
        });
    }
    Ok(out)
}
