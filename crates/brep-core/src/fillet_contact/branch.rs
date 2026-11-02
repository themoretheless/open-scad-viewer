//! Selected-edge ownership through nonoverlapping original-chart collars.
//! This proves a bounded branch ownership class, not continuation uniqueness,
//! radius, G2, wall thickness, or a closed body.
use super::Contact;
use crate::{
    Model, TopoId,
    face_domain::FaceDomain,
    region_replacement::{EdgeRadius, plan_radius_network},
};
use nurbs_core::{
    Result, curve::Curve, surface::Surface, surface_g2_pcurves, surface_linear_monotonicity,
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    Bound,
    Unresolved,
}
pub struct Collar {
    pub face: TopoId,
    pub cross_axis: usize,
    pub source_side: f64,
    /// Cross coordinate of the first contact endpoint (not a constant side).
    pub contact_side: f64,
    pub contact_band: [f64; 2],
    pub clock: nurbs_core::curve_coordinate_clock::Report,
    pub along: [f64; 2],
    pub injectivity: surface_linear_monotonicity::Report,
}
pub struct Report {
    pub decision: Decision,
    pub edge: TopoId,
    pub collars: Vec<Collar>,
    pub positions: Vec<surface_g2_pcurves::Positions>,
    pub cells: usize,
    pub exact_work: u64,
    pub reason: &'static str,
}
fn line(c: &Curve) -> Option<[[f64; 2]; 2]> {
    if c.validate().is_err()
        || c.degree != 1
        || c.periodic
        || c.control_points.len() != 2
        || c.control_points.iter().any(|p| p.len() != 2)
        || c.weights[0] != c.weights[1]
        || c.weights[0] <= 0.
        || c.knots.len() != 4
        || c.knots[0] != c.knots[1]
        || c.knots[2] != c.knots[3]
    {
        return None;
    }
    Some([
        [c.control_points[0][0], c.control_points[0][1]],
        [c.control_points[1][0], c.control_points[1][1]],
    ])
}
/// Bind both contacts to an existing edge via literal interior rectangular
/// ribbons of varying width on injective original supports. Contacts are in plan support order.
/// The selected edge can be a subdivision of a rectangular face side.
/// Each contact may be a nonparallel, multispan rational trace with a freshly
/// proved strict along-coordinate clock and the same endpoint extent as that edge;
/// its collar lies strictly in the selected half of the face's cross interval.
/// Thus an opposite-side or disconnected trimmed branch cannot inherit this
/// ownership receipt. All lifts are recomputed from supplied actual geometry.
/// Other trim/contact shapes remain Unresolved, never guessed from samples.
pub fn inspect(
    source: &Model,
    shell: usize,
    request: EdgeRadius,
    skin: &Surface,
    contacts: [&Contact; 2],
    max_visits: usize,
    max_cells: usize,
    max_exact: u64,
) -> Result<Report> {
    let plan = plan_radius_network(source, shell, &[request], max_visits)?;
    skin.validate()?;
    let selected = &plan.edges[0];
    let mut out = Report {
        decision: Decision::Unresolved,
        edge: selected.identity,
        collars: vec![],
        positions: vec![],
        cells: 0,
        exact_work: 0,
        reason: "Original edge lifts unresolved",
    };
    let original = super::topology::inspect_source_edge(
        source, shell, request, max_visits, max_cells, max_exact,
    )?;
    out.cells += original.cells;
    out.exact_work += original.exact_work;
    let proven = original.proven;
    out.positions.push(original);
    if !proven {
        return Ok(out);
    }
    for (side, &face_index) in selected.supports.iter().enumerate() {
        out.reason = "No literal rectangular contact collar";
        let face = &source.faces[face_index];
        let Some(rect) =
            FaceDomain::new(source, face_index, source.tolerance_mm)?.literal_rectangle()
        else {
            return Ok(out);
        };
        let edge_use = std::iter::once(face.outer)
            .chain(face.holes.iter().copied())
            .flat_map(|w| &source.loops[w].coedges)
            .find(|c| c.edge == request.edge)
            .unwrap();
        let Some(edge) = line(&edge_use.pcurve) else {
            return Ok(out);
        };
        let Some(axis) =
            (0..2).find(|&k| edge[0][k] == edge[1][k] && edge[0][1 - k] != edge[1][1 - k])
        else {
            return Ok(out);
        };
        let trace = &contacts[side].support_pcurve;
        trace.validate()?;
        if trace.periodic
            || !(1..=32).contains(&trace.degree)
            || trace.control_points.len() > 4096
            || trace.control_points.iter().any(|p| p.len() != 2)
            || trace.weights.iter().any(|w| *w <= 0.)
        {
            return Ok(out);
        }
        let first = &trace.control_points[0];
        let last = trace.control_points.last().unwrap();
        let contact = [[first[0], first[1]], [last[0], last[1]]];
        let along = 1 - axis;
        let s = edge[0][axis];
        let c = contact[0][axis];
        let range = [
            edge[0][along].min(edge[1][along]),
            edge[0][along].max(edge[1][along]),
        ];
        let crange = [
            contact[0][along].min(contact[1][along]),
            contact[0][along].max(contact[1][along]),
        ];
        // Contact traversal follows the selected original edge's endpoint roles,
        // not merely the unordered span of a geometrically similar side.
        let start = usize::from(edge_use.reversed);
        if contact[0][along] != edge[start][along] || contact[1][along] != edge[1 - start][along] {
            return Ok(out);
        }
        if range != crange || range[0] < rect[along][0] || range[1] > rect[along][1] {
            return Ok(out);
        }
        // Outward interval arithmetic, not a rounded face midpoint comparison.
        let i = nurbs_core::interval_eval::Interval::point;
        let midpoint = i(rect[axis][0]).add(i(rect[axis][1]))?.mul(i(0.5))?;
        let band = [
            trace
                .control_points
                .iter()
                .map(|p| p[axis])
                .fold(f64::INFINITY, f64::min),
            trace
                .control_points
                .iter()
                .map(|p| p[axis])
                .fold(f64::NEG_INFINITY, f64::max),
        ];
        if trace
            .control_points
            .iter()
            .any(|p| p[along] < range[0] || p[along] > range[1])
        {
            return Ok(out);
        }
        if !((s == rect[axis][0] && s < band[0] && band[1] < midpoint.lo)
            || (s == rect[axis][1] && midpoint.hi < band[0] && band[1] < s))
        {
            return Ok(out);
        }
        out.reason = "Contact coordinate clock unresolved";
        let clock = nurbs_core::curve_coordinate_clock::inspect(
            trace,
            along,
            max_cells.saturating_sub(out.cells),
        )?;
        out.cells += clock.cells;
        if !clock.proven {
            return Ok(out);
        }
        out.reason = "Original support injectivity unresolved";
        if out.cells >= max_cells {
            return Ok(out);
        }
        let injectivity =
            surface_linear_monotonicity::inspect_candidate(&face.surface, max_cells - out.cells)?;
        out.cells += injectivity.cells;
        if !injectivity.certified {
            return Ok(out);
        }
        out.reason = "Actual contact lifts unresolved";
        let positions = surface_g2_pcurves::verify_positions(
            &contacts[side].curve,
            [
                &contacts[side].support_pcurve,
                &contacts[side].fillet_pcurve,
            ],
            [&face.surface, skin],
            [false; 2],
            source.tolerance_mm,
            max_cells.saturating_sub(out.cells),
            max_exact.saturating_sub(out.exact_work),
        )?;
        out.cells += positions.cells;
        out.exact_work += positions.exact_work;
        let proven = positions.proven;
        out.positions.push(positions);
        if !proven {
            return Ok(out);
        }
        out.collars.push(Collar {
            face: source.1.faces[face_index],
            cross_axis: axis,
            source_side: s,
            contact_side: c,
            contact_band: band,
            clock,
            along: range,
            injectivity,
        });
    }
    out.decision = Decision::Bound;
    out.reason = "Both contacts owned by selected-edge chart collars";
    Ok(out)
}
