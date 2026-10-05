//! Source-expression contour clipping proposal. Exact joins are separate from
//! region containment, face immersion and closed-solid admission.
use crate::{
    source_boundary_fragment::{Endpoint, Fragment, Role},
    trimmed_face_recipe::{self, Boundary, Limits, SourcePointSearch},
};
use cad_predicates::ToleranceContext;
use nurbs_core::{curve::Curve, surface::Surface, Error, Result};
pub struct Proposal {
    pub search: SourcePointSearch,
    pub candidate_loops: Option<Vec<Vec<Fragment>>>,
    pub source_joins_proven: bool,
    pub reason: &'static str,
}
/// Find both contact parameters from original boundaries, without supplied cuts.
/// Endpoint edges select the cyclic removed arc; each must have one mapped root.
/// The result is a contour candidate, never a qualified replacement face.
pub fn propose(
    context: &ToleranceContext,
    surface: &Surface,
    wires: &[Vec<Boundary>],
    contact: &Curve,
    loop_index: usize,
    start_edge: usize,
    end_edge: usize,
    tolerance_uv: f64,
    limits: Limits,
    max_cells: usize,
    target_width: [f64; 2],
    max_point_checks: usize,
    max_mapping_cells: usize,
) -> Result<Proposal> {
    if loop_index >= wires.len()
        || start_edge == end_edge
        || start_edge >= wires[loop_index].len()
        || end_edge >= wires[loop_index].len()
    {
        return Err(Error::new(
            "BREP_SOURCE_CONTOUR",
            "Choose two distinct original endpoint edges on one contour",
        ));
    }
    let search = trimmed_face_recipe::locate_source_contact_points(
        context,
        surface,
        wires,
        contact,
        tolerance_uv,
        limits,
        max_cells,
        target_width,
        max_point_checks,
        max_mapping_cells,
    )?;
    let mut out = Proposal {
        search,
        candidate_loops: None,
        source_joins_proven: false,
        reason: "source-crossing-search-unqualified",
    };
    if !out.search.complete {
        return Ok(out);
    }
    let choose = |edge: usize| {
        out.search
            .points
            .iter()
            .filter(|p| p.loop_index == loop_index && p.boundary_index == edge)
            .filter_map(|p| p.report.as_ref().and_then(|r| r.point.as_ref()))
            .collect::<Vec<_>>()
    };
    let start = choose(start_edge);
    let end = choose(end_edge);
    if start.len() != 1 || end.len() != 1 {
        out.reason = "source-endpoint-root-ambiguous";
        return Ok(out);
    }
    let p0 = start[0].clone();
    let p1 = end[0].clone();
    let original = &wires[loop_index];
    let head = &original[start_edge].pcurve;
    let tail = &original[end_edge].pcurve;
    let contact_fragment = Fragment::new(
        surface,
        contact,
        Endpoint::Crossing {
            point: p0.clone(),
            role: Role::Contact,
        },
        Endpoint::Crossing {
            point: p1.clone(),
            role: Role::Contact,
        },
    )?;
    let head_fragment = Fragment::new(
        surface,
        head,
        Endpoint::Parameter(head.domain()[0]),
        Endpoint::Crossing {
            point: p0,
            role: Role::Boundary,
        },
    )?;
    let tail_fragment = Fragment::new(
        surface,
        tail,
        Endpoint::Crossing {
            point: p1,
            role: Role::Boundary,
        },
        Endpoint::Parameter(tail.domain()[1]),
    )?;
    let full = |b: &Boundary| {
        Fragment::new(
            surface,
            &b.pcurve,
            Endpoint::Parameter(b.pcurve.domain()[0]),
            Endpoint::Parameter(b.pcurve.domain()[1]),
        )
    };
    let mut loops = wires
        .iter()
        .map(|wire| wire.iter().map(full).collect::<Result<Vec<_>>>())
        .collect::<Result<Vec<_>>>()?;
    let mut replacement = vec![contact_fragment, tail_fragment];
    let mut i = (end_edge + 1) % original.len();
    while i != start_edge {
        replacement.push(full(&original[i])?);
        i = (i + 1) % original.len();
    }
    replacement.push(head_fragment);
    loops[loop_index] = replacement;
    let closed = loops.iter().all(|wire| {
        !wire.is_empty() && (0..wire.len()).all(|i| wire[i].joins(&wire[(i + 1) % wire.len()]))
    });
    if !closed {
        out.reason = "source-contour-joins-unproven";
        return Ok(out);
    }
    out.candidate_loops = Some(loops);
    out.source_joins_proven = true;
    out.reason = "source-contour-candidate-closed-region-unverified";
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn automatic_crossings_clip_source_expressions_without_rounded_curve_trims() {
        let s = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 1.], vec![0., 1., 1.]],
                vec![vec![1., 0., 1.], vec![1., 1., 1.]],
            ],
            weights: vec![vec![1., 1.], vec![1., 1.]],
            periodic_u: false,
            periodic_v: false,
        };
        let p = [vec![1., 0.], vec![1., 1.], vec![0., 1.], vec![0., 0.]];
        let wire = (0..4)
            .map(|i| {
                let pcurve =
                    Curve::from_polyline(vec![p[i].clone(), p[(i + 1) % 4].clone()]).unwrap();
                let mut curve = pcurve.clone();
                for p in &mut curve.control_points {
                    p.push(1.);
                }
                Boundary {
                    curve,
                    pcurve,
                    reversed: false,
                }
            })
            .collect::<Vec<_>>();
        let contact = Curve::from_polyline(vec![vec![-0.2, 0.3], vec![1.2, 0.3]]).unwrap();
        let limits = Limits {
            pairs: 10000,
            region_cells: 100000,
            domain_cells: 100000,
            agreement_cells: 100000,
        };
        let out = propose(
            &ToleranceContext::default_valid(),
            &s,
            &[wire.clone()],
            &contact,
            0,
            0,
            2,
            1e-8,
            limits,
            10000,
            [1e-7, 1e-7],
            16,
            16,
        )
        .unwrap();
        assert!(out.source_joins_proven);
        let loops = out.candidate_loops.unwrap();
        assert_eq!(loops[0].len(), 4);
        assert_eq!(loops[0][0].curve(), &contact);
        assert!(loops[0][0].reversed());
        assert_eq!(loops[0][1].curve(), &wire[2].pcurve);
        assert_eq!(loops[0][2].curve(), &wire[3].pcurve);
        assert_eq!(loops[0][3].curve(), &wire[0].pcurve);
        let stopped = propose(
            &ToleranceContext::default_valid(),
            &s,
            &[wire],
            &contact,
            0,
            0,
            2,
            1e-8,
            limits,
            1,
            [1e-7, 1e-7],
            16,
            16,
        )
        .unwrap();
        assert!(!stopped.source_joins_proven && stopped.candidate_loops.is_none());
    }
}
