//! Bounded enumeration of different face pairs. Adjacent faces are not skipped:
//! sharing topology does not prove the absence of extra interior intersections.
#[path = "boundary_stitched_contact.rs"]
mod boundary_stitched_contact;
#[path = "source_volume_contact.rs"]
mod source_volume_contact;
use crate::{Error, Model, Result, face_domain::FaceDomain};
use nurbs_core::surface_contact_search::{self, Report as Search};
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub pairs: usize,
    pub cells: usize,
    pub domain_cells: usize,
    pub cells_per_pair: usize,
    pub domain_cells_per_pair: usize,
}
#[derive(Clone, Debug)]
pub enum SharedBoundary {
    SweepCap { cap: usize, wall: usize, edge: usize },
    ExactHull(crate::boundary_hull_contact::Certificate),
    PlanarFace(crate::shared_boundary::Certificate),
    OppositeSides(crate::shared_boundary::OppositeSidesCertificate),
    JoinedSourceCharts(boundary_stitched_contact::Certificate),
    SourceVolumeBoundary(source_volume_contact::Certificate),
}
#[derive(Clone, Debug)]
pub struct Pair {
    pub faces: [usize; 2],
    pub result: Option<Search>,
    pub boundary: Option<SharedBoundary>,
    /// Disjointness was proven exactly by control-hull separation, so no
    /// trimmed search report exists for this pair.
    pub hull_disjoint: bool,
    /// Exact control-hull separation work spent on this pair. Attributed to
    /// the pair so per-pair work adds up to the report totals.
    pub hull_cells: usize,
    pub source_chart_cells:usize,
    pub volume_separation:Option<source_volume_contact::Certificate>,
    pub reason: &'static str,
}
#[derive(Clone, Debug)]
pub struct Report {
    pub pairs: Vec<Pair>,
    pub total_pairs: usize,
    /// First unvisited pair in lexicographic order; the entire suffix is pending.
    pub next_pair: Option<[usize; 2]>,
    pub cells: usize,
    pub domain_cells: usize,
    /// All distinct face pairs were proven disjoint. This says nothing about
    /// self-intersection inside one face or geometric validity of a volume.
    pub all_pairs_disjoint: bool,
    pub all_pairs_classified: bool,
}
pub fn inspect(model: &Model, tolerance_uv: f64, limits: Limits) -> Result<Report> {
    inspect_with_hulls(model, tolerance_uv, limits, &[])
}
/// Hull certificates are admitted only by the exact boundary embedding audit.
pub(crate) fn inspect_with_hulls(model: &Model, tolerance_uv: f64, limits: Limits,
    hulls: &[crate::boundary_hull_contact::Certificate]) -> Result<Report> {
    inspect_with_certificates(model,tolerance_uv,limits,hulls,&[],false)
}
/// Cap tuples originate only from native recomputation in boundary embedding.
pub(crate) fn inspect_with_certificates(model: &Model, tolerance_uv: f64, limits: Limits,
    hulls: &[crate::boundary_hull_contact::Certificate], caps: &[[usize;3]], exact_domain: bool) -> Result<Report> {
    model.validate_boundary_diagnostic_inputs()?;
    if !(1..=100_000).contains(&limits.pairs)
        || !(1..=1_000_000).contains(&limits.cells)
        || !(1..=8_000_000).contains(&limits.domain_cells)
        || !(1..=100_000).contains(&limits.cells_per_pair)
        || !(1..=1_000_000).contains(&limits.domain_cells_per_pair)
        || !tolerance_uv.is_finite()
        || tolerance_uv <= 0.
    {
        return Err(Error::new(
            "BREP_CONTACT_BUDGET",
            "Use positive bounded pair, geometry and UV budgets and finite positive UV tolerance",
        ));
    }
    // Prepare each chart once, rather than validate/copy the model per pair.
    let domains = (0..model.faces.len())
        .map(|f| FaceDomain::new(model, f, tolerance_uv))
        .collect::<Result<Vec<_>>>()?;
    let source_volume_context=exact_domain.then(||source_volume_contact::Prepared::new(model));
    let n = model.faces.len();
    let total_pairs = n
        .checked_mul(n.saturating_sub(1))
        .ok_or_else(|| Error::new("BREP_CONTACT_BUDGET", "Too many face pairs"))?
        / 2;
    let mut out = Report {
        pairs: Vec::new(),
        total_pairs,
        next_pair: None,
        cells: 0,
        domain_cells: 0,
        all_pairs_disjoint: true,
        all_pairs_classified: true,
    };
    for a in 0..n {
        for b in a + 1..n {
            if out.pairs.len() == limits.pairs
                || out.cells == limits.cells
                || out.domain_cells == limits.domain_cells
            {
                out.next_pair = Some([a, b]);
                out.all_pairs_disjoint = false;
                out.all_pairs_classified = false;
                return Ok(out);
            }
            let sa = &model.faces[a].surface;
            let sb = &model.faces[b].surface;
            let mut boundary = if let Some(c)=caps.iter().find(|c|
                [c[0].min(c[1]),c[0].max(c[1])]==[a,b]) {
                Some(SharedBoundary::SweepCap {cap:c[0],wall:c[1],edge:c[2]})
            } else if let Some(c) = hulls.iter().find(|c|c.faces==[a,b]) {
                Some(SharedBoundary::ExactHull(c.clone()))
            } else if let Some(c) = crate::shared_boundary::certify(model, [a, b]) {
                Some(SharedBoundary::PlanarFace(c))
            } else {
                crate::shared_boundary::certify_opposite(model, [a, b])?
                    .map(SharedBoundary::OppositeSides)
            };
            let periodic = sa.periodic_u || sa.periodic_v || sb.periodic_u || sb.periodic_v;
            let mut source_chart_cells=0;
            let mut volume_separation=None;
            // Quintic polygon corner charts need a source-volume proof: their
            // piecewise profile weights need not have one global chart map.
            // Try the exact four-wall cell before spending the pair budget on
            // a stitched surface. Other chart families keep their ordering.
            let volume_first=sa.degree_u==1 && sa.degree_v==5 && sb.degree_u==1 && sb.degree_v==5;
            if volume_first && boundary.is_none()&&!periodic&&exact_domain {
                let separated=source_volume_context.as_ref().unwrap().certify([a,b],
                    (limits.cells-out.cells).min(limits.cells_per_pair))?;
                source_chart_cells=separated.cells;out.cells+=separated.cells;
                if let Some(c)=separated.certificate {
                    if c.shared_edge.is_some() {boundary=Some(SharedBoundary::SourceVolumeBoundary(c));}
                    else {volume_separation=Some(c);}
                }
            }
            if boundary.is_none()&&volume_separation.is_none()&&!periodic&&exact_domain&&source_chart_cells<limits.cells_per_pair {
                let joined=boundary_stitched_contact::certify(model,[a,b],
                    (limits.cells-out.cells).min(limits.cells_per_pair-source_chart_cells))?;
                source_chart_cells+=joined.cells;out.cells+=joined.cells;
                boundary=joined.certificate.map(SharedBoundary::JoinedSourceCharts);
            }
            if !volume_first&&boundary.is_none()&&!periodic&&exact_domain&&source_chart_cells<limits.cells_per_pair {
                let separated=source_volume_context.as_ref().unwrap().certify([a,b],
                    (limits.cells-out.cells).min(limits.cells_per_pair-source_chart_cells))?;
                source_chart_cells+=separated.cells;out.cells+=separated.cells;
                if let Some(c)=separated.certificate {
                    if c.shared_edge.is_some() {boundary=Some(SharedBoundary::SourceVolumeBoundary(c));}
                    else {volume_separation=Some(c);}
                }
            }
            let pair_cells=limits.cells_per_pair-source_chart_cells;
            // Exact control-hull separation also applies to closed bodies
            // after the joint boundary, trim and injectivity prerequisites.
            // The plain face-contact API retains its trim-search accounting.
            let (hull_disjoint,hull_cells)=if boundary.is_none() && volume_separation.is_none() && !periodic && exact_domain {
                crate::control_hull_separation::inspect(sa,sb,
                    (limits.cells-out.cells).min(pair_cells/2).min(64))
            } else { (false,0) };
            out.cells+=hull_cells;
            let result = if boundary.is_some() || hull_disjoint || volume_separation.is_some()
                || periodic
                || out.cells == limits.cells
                || hull_cells == pair_cells
            {
                None
            } else {
                Some(surface_contact_search::search_trimmed(
                    sa,
                    sb,
                    [&domains[a].region, &domains[b].region],
                    (limits.cells - out.cells).min(pair_cells-hull_cells),
                    (limits.domain_cells - out.domain_cells).min(limits.domain_cells_per_pair),
                )?)
            };
            let reason = match &result {
                None if hull_disjoint => "pair-disjoint",
                None if volume_separation.is_some() => "pair-disjoint",
                None if boundary.is_some() => "shared-boundary",
                None if sa.periodic_u || sa.periodic_v || sb.periodic_u || sb.periodic_v => "periodic-trim-not-supported",
                None => "pair-unresolved",
                Some(r) if r.absence_proven => "pair-disjoint",
                Some(r) if r.contact.is_some() => "interior-contact",
                Some(_) => "pair-unresolved",
            };
            out.all_pairs_classified &=
                boundary.is_some() || hull_disjoint || volume_separation.is_some() || result.as_ref().is_some_and(|r| r.absence_proven);
            out.all_pairs_disjoint &= hull_disjoint || volume_separation.is_some() || result.as_ref().is_some_and(|r| r.absence_proven);
            if let Some(r) = &result {
                out.cells += r.cells;
                out.domain_cells += r.domain_cells;
            }
            out.pairs.push(Pair {
                faces: [a, b],
                result,
                boundary,
                hull_disjoint,
                hull_cells,
                source_chart_cells,
                volume_separation,
                reason,
            });
        }
    }
    Ok(out)
}
#[cfg(test)]
mod tests {
    #[test]
    fn partial_annular_separate_pairs_require_complete_search_coverage() {
        let model=crate::circular_blend::partial_annular_quarter(20.,5.,6.,1.25,1.,1e-7).unwrap();
        let before=format!("{model:?}");
        for [a,b] in [[6,15],[6,23],[9,18],[9,26],[15,23],[18,26]] {
            let da=crate::face_domain::FaceDomain::new(&model,a,1e-8).unwrap();
            let db=crate::face_domain::FaceDomain::new(&model,b,1e-8).unwrap();
            let run=|budget|nurbs_core::surface_contact_search::search_trimmed(
                &model.faces[a].surface,&model.faces[b].surface,[&da.region,&db.region],budget,1_000_000).unwrap();
            let partial=run(256);
            assert!(!partial.absence_proven&&!partial.unresolved.is_empty());
            let complete=run(1024);
            assert!(complete.absence_proven&&complete.unresolved.is_empty(),"faces={a}/{b}");
            assert!(complete.contact.is_none());
        }
        assert_eq!(format!("{model:?}"),before);
    }
    use super::*;
    fn limits() -> Limits {
        Limits {
            pairs: 100,
            cells: 10000,
            domain_cells: 100000,
            cells_per_pair: 16,
            domain_cells_per_pair: 1000,
        }
    }
    #[test]
    fn cube_visits_all_pairs_without_excusing_adjacent_contacts() {
        let m = crate::cuboid([0.; 3], [1.; 3]).unwrap();
        let r = inspect(&m, 1e-8, limits()).unwrap();
        assert_eq!(r.total_pairs, 15);
        assert_eq!(r.pairs.len(), 15);
        assert!(r.next_pair.is_none());
        assert!(!r.all_pairs_disjoint);
        assert!(
            r.pairs
                .iter()
                .all(|p| p.result.as_ref().is_none_or(|r| r.contact.is_none()))
        );
        assert!(r.pairs.iter().any(|p| p.reason == "pair-disjoint"));
        assert_eq!(
            r.pairs
                .iter()
                .filter(|p| p.reason == "shared-boundary")
                .count(),
            12
        );
        assert!(r.all_pairs_classified);
    }
    #[test]
    fn limits_preserve_exact_unvisited_suffix() {
        let mut m = crate::cuboid([0.; 3], [1.; 3]).unwrap();
        // Force actual UV work: an intact cube now needs only shared-edge
        // certificates and spatial exclusions, consuming no UV cells.
        for row in &mut m.faces[0].surface.control_points {
            for p in row {
                for x in p {
                    *x += 0.01;
                }
            }
        }
        for l in [
            Limits {
                pairs: 1,
                ..limits()
            },
            Limits {
                cells: 1,
                ..limits()
            },
            Limits {
                domain_cells: 1,
                ..limits()
            },
        ] {
            let r = inspect(&m, 1e-8, l).unwrap();
            assert!(!r.all_pairs_disjoint);
            assert!(r.next_pair.is_some());
            assert!(
                r.cells <= l.cells && r.domain_cells <= l.domain_cells && r.pairs.len() <= l.pairs
            );
            let expected: Vec<_> = (0..6)
                .flat_map(|a| (a + 1..6).map(move |b| [a, b]))
                .collect();
            assert_eq!(r.next_pair, Some(expected[r.pairs.len()]));
            assert_eq!(
                r.pairs.iter().map(|p| p.faces).collect::<Vec<_>>(),
                expected[..r.pairs.len()]
            );
        }
    }
    #[test]
    fn a_defect_does_not_stop_later_pairs_from_being_reported() {
        let mut m = crate::cuboid([0.; 3], [1.; 3]).unwrap();
        for (f, face) in m.faces.iter_mut().enumerate() {
            face.surface.control_points = (0..2)
                .map(|u| {
                    (0..2)
                        .map(|v| {
                            if f == 1 {
                                vec![u as f64, 0.5, v as f64 - 0.5]
                            } else {
                                vec![u as f64, v as f64, f as f64 * 10.]
                            }
                        })
                        .collect()
                })
                .collect();
        }
        let r = inspect(&m, 1e-8, limits()).unwrap();
        assert_eq!(r.pairs.len(), 15);
        assert!(r.next_pair.is_none());
        assert_eq!(r.pairs[0].faces, [0, 1]);
        assert_eq!(r.pairs[0].reason, "interior-contact");
        assert!(r.pairs[1..].iter().all(|p| p.reason == "pair-disjoint"));
        assert!(!r.all_pairs_disjoint);
    }
}
