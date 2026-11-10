//! Original loop carriers are refined only for proof, under exact original
//! parameterwise identity. Tolerance agreement alone cannot transfer topology.
use crate::{Error, Result};
use nurbs_core::{curve::Curve, curve_partition_agreement, periodic_trim};
fn refused(s: &str) -> Error {
    Error::new("BREP_CLOSED_UV_REFUSED", s)
}
pub(super) fn prepare(
    source: &Curve,
    tolerance: f64,
    max_cells: usize,
    max_work: usize,
) -> Result<(Vec<Curve>, usize, usize)> {
    source.validate()?;
    if max_cells == 0 || max_work == 0 {
        return Err(refused("Closed original UV proof work exhausted"));
    }
    if let Some(curves) = curve_partition_agreement::exact_bezier_span_partition(source)? {
        if curves.len() >= 2 {
            let work = curves
                .iter()
                .map(|c| {
                    source.control_points.len()
                        + source.knots.len()
                        + c.control_points.len()
                        + c.knots.len()
                })
                .sum::<usize>();
            if curves.len() > max_cells || work > max_work {
                return Err(refused("Exact original UV span work exhausted"));
            }
            let cells = curves.len();
            return Ok((curves, cells, work));
        }
    }
    let d = source.domain();
    let mut cuts = source
        .knots
        .iter()
        .copied()
        .filter(|u| *u > d[0] && *u < d[1])
        .collect::<Vec<_>>();
    cuts.dedup();
    if cuts.is_empty() {
        let t = d[0] * 0.5 + d[1] * 0.5;
        if t <= d[0] || t >= d[1] {
            return Err(refused(
                "Closed UV interval cannot be represented by interior cuts",
            ));
        }
        cuts.push(t);
    }
    if cuts.len() > 255 {
        return Err(refused(
            "Closed UV proof partition exceeds bounded loop complexity",
        ));
    }
    let proposal = curve_partition_agreement::partition(source, &cuts, tolerance, max_cells)?;
    if !proposal.geometry_preserved
        || !proposal.endpoint_joins_exact
        || proposal.periodic_closure_exact == Some(false)
    {
        return Err(refused(&format!(
            "Original closed UV proposal remains unproven: geometry={}, joins={}, closure={:?}, cells={}",
            proposal.geometry_preserved,
            proposal.endpoint_joins_exact,
            proposal.periodic_closure_exact,
            proposal.visited
        )));
    }
    let cells = proposal.visited;
    let partition = periodic_trim::Report {
        pieces: proposal
            .pieces
            .into_iter()
            .map(|p| periodic_trim::Piece {
                source_domain: p.curve.domain(),
                curve: p.curve,
                translation: [0.; 2],
                tile: [0; 2],
            })
            .collect(),
        candidate_hulls_contained: false,
    };
    let mut work = 0usize;
    let mut structural = true;
    for piece in &partition.pieces {
        let cost = source.control_points.len()
            + source.knots.len()
            + piece.curve.control_points.len()
            + piece.curve.knots.len();
        if cost > max_work.saturating_sub(work) {
            return Err(refused(
                "Original closed UV structural proof work exhausted",
            ));
        }
        work += cost;
        let literal=curve_partition_agreement::verify_exact_span_restriction(source, &piece.curve)?;
        if !literal {
            let remaining=max_work.saturating_sub(work).min(cad_predicates::MAX_WORK as usize);
            if remaining==0 {return Err(refused("Original rational restriction proof work exhausted"));}
            let exact=curve_partition_agreement::verify_exact_bspline_restriction(source,&piece.curve,remaining as u64)?;
            if let Some(exact)=exact {
                work=work.checked_add(exact.work_used as usize).ok_or_else(||refused("Original rational proof work overflow"))?;
                structural &= exact.outcome==cad_predicates::BezierIdentity::Equal;
            }else{structural=false;}
        }
    }
    let curves = if structural {
        partition
            .pieces
            .iter()
            .map(|p| p.curve.clone())
            .collect::<Vec<_>>()
    } else {
        let exact = periodic_trim::verify_exact_partition_identity(
            source,
            &partition,
            (max_work - work) as u64,
        )?
        .ok_or_else(|| refused("Original closed UV restriction identity remains unproven"))?;
        work += exact.work_used() as usize;
        exact
            .partition()
            .pieces
            .iter()
            .map(|p| p.curve.clone())
            .collect::<Vec<_>>()
    };
    if nurbs_core::trim_domain::exact_loop_joins(&curves)? != Some(true) {
        return Err(refused(
            "Original closed UV partition is not exactly closed",
        ));
    }
    Ok((curves, cells, work))
}

pub(super) fn literal_multispan(source: &Curve) -> bool {
    let p = source.degree;
    let spans = (p..source.control_points.len())
        .filter(|i| source.knots[*i] < source.knots[*i + 1])
        .collect::<Vec<_>>();
    spans.len() > 1
        && spans.iter().all(|i| {
            [source.knots[*i], source.knots[*i + 1]]
                .iter()
                .all(|v| source.knots.iter().filter(|k| *k == v).count() >= p)
        })
}

#[cfg(test)]
mod rational_restriction_tests {
    use super::*;
    #[test]
    fn rational_closed_bezier_preparation_reproves_both_original_half_intervals() {
        let source=Curve {degree:3,knots:vec![0.,0.,0.,0.,1.,1.,1.,1.],
            control_points:vec![vec![0.,0.],vec![10.,0.],vec![10.,10.],vec![0.,0.]],
            weights:vec![1.,3.,3.,1.],periodic:false};
        let before=source.clone();
        let (curves,cells,work)=prepare(&source,1e-8,10000,1_000_000).unwrap();
        assert_eq!(curves.len(),2);assert!(cells>0&&work>0&&work<=1_000_000);
        for c in &curves {
            assert!(!curve_partition_agreement::verify_exact_span_restriction(&source,c).unwrap());
            let r=curve_partition_agreement::verify_exact_bezier_restriction(&source,c,c.domain(),false,100000).unwrap().unwrap();
            assert_eq!(r.outcome,cad_predicates::BezierIdentity::Equal);
        }
        assert!(prepare(&source,1e-8,10000,1).is_err());
        assert_eq!(source,before);
    }
}
