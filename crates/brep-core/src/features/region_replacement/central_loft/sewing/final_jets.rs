//! Fresh two-jet admission on actual sewn surfaces and saved shared coedges.
use super::*;
use nurbs_core::surface_g2_pcurves::{self as g2, ExtrusionBoundaryReport};
fn pcurve(m: &Model, f: usize, e: usize) -> Result<Curve> {
    let f = &m.faces[f];
    let uses = std::iter::once(f.outer)
        .chain(f.holes.iter().copied())
        .flat_map(|w| &m.loops[w].coedges)
        .filter(|c| c.edge == e)
        .collect::<Vec<_>>();
    require(uses.len() == 1, "Final seam lost its unique face use")?;
    let c = uses[0];
    if c.reversed {
        c.pcurve.reverse()
    } else {
        Ok(c.pcurve.clone())
    }
}
fn passed(r: &ExtrusionBoundaryReport) -> bool {
    r.decision == nurbs_core::surface_join::Decision::WithinTolerance
        && r.regularity_proven
        && r.pending == 0
}
pub(super) fn inspect(
    c: &CentralLoftCandidate<'_, '_, '_>,
    m: &Model,
    central: usize,
    strips: [usize; 3],
    cells: &mut usize,
    exact: &mut u64,
    limits: CornerSewLimits,
) -> Result<Vec<ExtrusionBoundaryReport>> {
    let surface = &m.faces[central].surface;
    let poles = c.inspect_surface_pole_limits(surface, 1. / 64., 2000)?;
    *cells += poles.iter().map(|p| p.report.cells).sum::<usize>();
    require(
        poles.iter().all(|p| p.report.proven),
        "Final pole quotient remains unproven",
    )?;
    let certificates = [poles[0].certificate.as_ref(), poles[1].certificate.as_ref()];
    let mut reports = Vec::new();
    for coedge in &m.loops[m.faces[central].outer].coedges {
        let e = coedge.edge;
        if m.edges[e].degenerate {
            continue;
        }
        let neighbors = m
            .faces
            .iter()
            .enumerate()
            .filter(|(id, f)| {
                *id != central && m.loops[f.outer].coedges.iter().any(|co| co.edge == e)
            })
            .map(|(i, _)| i)
            .collect::<Vec<_>>();
        require(
            neighbors.len() == 1,
            "Central seam lost its explicit neighbor",
        )?;
        let other = neighbors[0];
        let a = pcurve(m, other, e)?;
        let b = pcurve(m, central, e)?;
        let supports = [&m.faces[other].surface, surface];
        require(
            *cells < limits.cells && *exact <= limits.exact_work,
            "Final seam work exhausted",
        )?;
        if strips.contains(&other) {
            let source = &m.faces[other].surface;
            let degree = source.degree_u;
            let n = source.control_points.len();
            let axis = (0..3)
                .find(|&k| {
                    m.edges[e]
                        .curve
                        .control_points
                        .iter()
                        .all(|p| p[k] == m.edges[e].curve.control_points[0][k])
                })
                .ok_or_else(|| refuse("Final profile lacks a literal cutter axis"))?;
            require(
                a.degree == 1
                    && b.degree == 1
                    && a.control_points.len() == 2
                    && b.control_points.len() == 2,
                "Final strip seam clocks must be affine",
            )?;
            let ad = a.control_points[1][0] - a.control_points[0][0];
            let bd = b.control_points[1][0] - b.control_points[0][0];
            require(
                ad.abs() == bd.abs() && ad != 0.,
                "Final strip seam clocks must share exact integer intervals",
            )?;
            for span in (degree..n).filter(|&i| source.knots_u[i] < source.knots_u[i + 1]) {
                let lo = source.knots_u[span];
                let hi = source.knots_u[span + 1];
                let mut piece = source.clone();
                piece.control_points = source.control_points[span - degree..=span].to_vec();
                piece.weights = source.weights[span - degree..=span].to_vec();
                piece.knots_u = [vec![lo; degree + 1], vec![hi; degree + 1]].concat();
                // Restriction is a literal original C0 Bernstein slice; no
                // rounded knot insertion/extraction proposal enters the proof.
                require(
                    source.knots_u.iter().filter(|&&x| x == lo).count() >= degree
                        && source.knots_u.iter().filter(|&&x| x == hi).count() >= degree,
                    "Final strip restriction is not an original C0 cell",
                )?;
                let av = a.control_points[0][1];
                require(
                    av == source.knots_v[source.degree_v],
                    "Final near seam is not the actual first strip boundary",
                )?;
                let curve = Curve {
                    degree,
                    knots: piece.knots_u.clone(),
                    control_points: piece.control_points.iter().map(|r| r[0].clone()).collect(),
                    weights: piece.weights.iter().map(|r| r[0]).collect(),
                    periodic: false,
                };
                let ca = Curve::from_polyline(vec![vec![lo, av], vec![hi, av]])?;
                let map = |u: f64| {
                    b.control_points[0][0]
                        + if ad == bd {
                            u - a.control_points[0][0]
                        } else {
                            a.control_points[0][0] - u
                        }
                };
                let cb = Curve::from_polyline(vec![
                    vec![map(lo), b.control_points[0][1]],
                    vec![map(hi), b.control_points[0][1]],
                ])?;
                require(
                    *cells < limits.cells && *exact <= limits.exact_work,
                    "Final seam work exhausted",
                )?;
                let r = g2::inspect_extrusion_boundary(
                    &curve,
                    [&ca, &cb],
                    [&piece, surface],
                    axis,
                    certificates,
                    c.options.tolerances,
                    (limits.cells - *cells).min(6000),
                    (limits.exact_work - *exact).min(100000),
                )?;
                *cells += r.cells;
                *exact += r.positions.exact_work;
                require(passed(&r), "Actual sewn central/strip G2 remains unproven")?;
                reports.push(r);
            }
            continue;
        }
        let plane = &m.faces[other].surface;
        let axis = (0..3)
            .find(|&k| {
                plane
                    .control_points
                    .iter()
                    .flatten()
                    .all(|p| p[k] == plane.control_points[0][0][k])
            })
            .ok_or_else(|| refuse("Final connector has no literal stock support"))?;
        let r = g2::inspect_planar_boundary(
            &m.edges[e].curve,
            [&a, &b],
            supports,
            axis,
            certificates,
            c.options.tolerances,
            (limits.cells - *cells).min(6000),
            (limits.exact_work - *exact).min(100000),
        )?;
        *cells += r.cells;
        *exact += r.positions.exact_work;
        require(passed(&r), "Actual sewn central seam G2 remains unproven")?;
        reports.push(r);
    }
    require(
        reports.len() >= 6,
        "Final central seam coverage is incomplete",
    )?;
    Ok(reports)
}
