//! Independent continuous original-guide admission, never site acceptance.
use super::*;
pub(super) fn budget(o: Options) -> Result<()> {
    if !(1..=8).contains(&o.max_iterations)
        || !(3..=129).contains(&o.max_sites_per_curve)
        || !(3..=192).contains(&o.max_total_sites)
        || !(1..=100000).contains(&o.max_cells)
        || o.max_exact_work > 1000000
        || !o.normal_chord_tolerance.is_finite()
        || o.normal_chord_tolerance <= 0.
        || o.normal_chord_tolerance > 2.
        || !o.fitting.position_tolerance.is_finite()
        || o.fitting.position_tolerance <= 0.
    {
        return Err(error(
            "Choose finite whole-network tolerances and bounded native work",
        ));
    }
    Ok(())
}
fn curve(c: &Curve, dimension: usize) -> Result<()> {
    c.validate()?;
    if c.periodic
        || c.control_points[0].len() != dimension
        || c.control_points.len() > 256
        || c.degree > 8
        || c.weights.iter().any(|w| *w <= 0.)
    {
        return Err(error(
            "Whole guides need bounded nonperiodic positive rational original curves",
        ));
    }
    Ok(())
}
fn chart(s: &Surface, p: &Curve) -> Result<()> {
    s.validate()?;
    curve(p, 2)?;
    let d = [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ];
    if s.periodic_u
        || s.periodic_v
        || s.control_points[0][0].len() != 3
        || s.weights.iter().flatten().any(|w| *w <= 0.)
        || s.control_points.len() * s.control_points[0].len() > 4096
        || p.control_points
            .iter()
            .any(|p| (0..2).any(|k| p[k] < d[k][0] || p[k] > d[k][1]))
    {
        return Err(error(
            "Original guide pcurve control hull must lie in a bounded positive original chart",
        ));
    }
    Ok(())
}
pub(super) fn validate(s: &Surface, g: &OwnedGuide) -> Result<()> {
    curve(&g.spatial, 3)?;
    chart(s, &g.parameters)?;
    if let Some(t) = &g.tangent {
        chart(&t.surface, &t.parameters)?;
    }
    Ok(())
}
pub(super) fn verify(
    s: &Surface,
    guides: &[OwnedGuide],
    options: Options,
    curvature_tolerance: Option<f64>,
    max_cells: usize,
    max_exact: u64,
) -> Result<Verification> {
    let mut out = Verification {
        loading: None,
        diagnostics: Vec::new(),
        evidence: Vec::new(),
        boundary_seams: Vec::new(),
        admitted: false,
        cells: 0,
        exact_work: 0,
    };
    for (i, g) in guides.iter().enumerate() {
        let remaining = guides.len() - i;
        let cells = (max_cells - out.cells) / remaining;
        let exact = (max_exact - out.exact_work) / remaining as u64;
        let e = if g.curvature {
            let t = g
                .tangent
                .as_ref()
                .ok_or_else(|| error("Missing original curvature support"))?;
            let report = crate::surface_g2_pcurves::inspect(
                &g.spatial,
                [&g.parameters, &t.parameters],
                [s, &t.surface],
                [g.reverse, t.reverse],
                options.fitting.position_tolerance,
                options.normal_chord_tolerance,
                curvature_tolerance.ok_or_else(|| error("Missing G2 tolerance"))?,
                cells,
                exact,
            )?;
            out.cells += report.geometric.cells;
            out.exact_work += report.positions.exact_work;
            if report.geometric.decision != crate::surface_join::Decision::WithinTolerance {
                out.diagnostics.push(format!(
                    "guide {i}: original positions {}, G2 {:?}, cells {}",
                    report.positions.proven, report.geometric.decision, report.geometric.cells
                ));
            }
            Evidence::Curvature(report)
        } else if let Some(t) = &g.tangent {
            let report = crate::surface_g1_pcurves::inspect(
                &g.spatial,
                [&g.parameters, &t.parameters],
                [s, &t.surface],
                [g.reverse, t.reverse],
                options.fitting.position_tolerance,
                options.normal_chord_tolerance,
                cells,
                exact,
            )?;
            out.cells += report.cells;
            out.exact_work += report.work();
            Evidence::TangentPlane(report)
        } else {
            let report = crate::curve_surface_agreement::verify(
                &g.spatial,
                &g.parameters,
                s,
                g.reverse,
                options.fitting.position_tolerance,
                cells,
            )?;
            out.cells += report.cells;
            Evidence::Position(report)
        };
        out.evidence.push(e);
    }
    out.admitted = out.evidence.iter().all(Evidence::admitted);
    Ok(out)
}
