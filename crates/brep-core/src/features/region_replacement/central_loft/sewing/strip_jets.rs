//! Actual final stock/strip contact jets and prescribed contact-center radius.
use super::*;
use nurbs_core::surface_g2_pcurves::{self as g2, ExtrusionBoundaryReport};
fn pc(m: &Model, f: usize, e: usize) -> Result<Curve> {
    let uses = m.loops[m.faces[f].outer]
        .coedges
        .iter()
        .filter(|co| co.edge == e)
        .collect::<Vec<_>>();
    require(uses.len() == 1, "Final stock/strip contact lost ownership")?;
    let co = uses[0];
    if co.reversed {
        co.pcurve.reverse()
    } else {
        Ok(co.pcurve.clone())
    }
}
pub(super) fn inspect(
    c: &CentralLoftCandidate<'_, '_, '_>,
    m: &Model,
    faces: [usize; 3],
    edges: [[usize; 2]; 3],
    cells: &mut usize,
    exact: &mut u64,
    limits: CornerSewLimits,
) -> Result<(
    Vec<ExtrusionBoundaryReport>,
    Vec<crate::fillet_radius::Report>,
)> {
    let mut jets = Vec::new();
    let mut radius = Vec::new();
    for j in 0..3 {
        let strip = &c.strips.strips[j];
        let s = &m.faces[faces[j]].surface;
        let p = s.degree_u;
        let n = s.control_points.len();
        for side in 0..2 {
            let end = if side == 0 { 0 } else { n - 1 };
            let start = if side == 0 { 0 } else { n - 1 - p };
            let span = start + p;
            let lo = s.knots_u[span];
            let hi = s.knots_u[span + 1];
            let view = Surface {
                degree_u: s.degree_v,
                degree_v: p,
                knots_u: s.knots_v.clone(),
                knots_v: [vec![lo; p + 1], vec![hi; p + 1]].concat(),
                control_points: (0..s.control_points[0].len())
                    .map(|v| {
                        (start..=start + p)
                            .map(|u| s.control_points[u][v].clone())
                            .collect()
                    })
                    .collect(),
                weights: (0..s.weights[0].len())
                    .map(|v| (start..=start + p).map(|u| s.weights[u][v]).collect())
                    .collect(),
                periodic_u: false,
                periodic_v: false,
            };
            require(
                s.knots_u.iter().filter(|&&u| u == lo).count() >= p
                    && s.knots_u.iter().filter(|&&u| u == hi).count() >= p,
                "Stock contact view is not a literal C0 end cell",
            )?;
            let e = edges[j][side];
            let f = strip.support_faces[side];
            let a = pc(m, f, e)?;
            let mut b = pc(m, faces[j], e)?;
            require(
                b.control_points
                    .iter()
                    .all(|uv| uv[0] == if end == 0 { s.knots_u[p] } else { s.knots_u[n] }),
                "Final contact is not the actual natural strip side",
            )?;
            for uv in &mut b.control_points {
                uv.swap(0, 1);
            }
            let support = &m.faces[f].surface;
            let height = (0..3)
                .find(|&k| {
                    support
                        .control_points
                        .iter()
                        .flatten()
                        .all(|p| p[k] == support.control_points[0][0][k])
                })
                .ok_or_else(|| refuse("Final strip stock support is not a coordinate plane"))?;
            require(
                *cells < limits.cells && *exact <= limits.exact_work,
                "Final stock/strip G2 work exhausted",
            )?;
            let r = g2::inspect_planar_boundary(
                &m.edges[e].curve,
                [&a, &b],
                [support, &view],
                height,
                [None, None],
                c.options.tolerances,
                (limits.cells - *cells).min(6000),
                (limits.exact_work - *exact).min(100000),
            )?;
            *cells += r.cells;
            *exact += r.positions.exact_work;
            require(
                r.decision == nurbs_core::surface_join::Decision::WithinTolerance
                    && r.regularity_proven
                    && r.pending == 0,
                "Actual sewn stock/strip G2 remains unproven",
            )?;
            jets.push(r);
        }
        require(
            *cells < limits.cells && *exact <= limits.exact_work,
            "Final radius work exhausted",
        )?;
        let r = crate::fillet_radius::inspect_model(
            m,
            faces[j],
            edges[j],
            &nurbs_core::offset::fillet_surfaces::RadiusLaw::Constant(strip.radius_mm),
            [0., 1.],
            strip.sides,
            c.options.tolerances[0],
            (limits.cells - *cells).min(10000),
            (limits.exact_work - *exact).min(100000),
        )?;
        *cells += r.cells;
        *exact += r.exact_work;
        require(
            r.decision == crate::fillet_radius::Decision::WithinTolerance,
            "Actual sewn prescribed contact radius remains unproven",
        )?;
        radius.push(r);
    }
    Ok((jets, radius))
}
