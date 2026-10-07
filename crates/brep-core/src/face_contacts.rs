//! Bounded enumeration of different face pairs. Adjacent faces are not skipped:
//! sharing topology does not prove the absence of extra interior intersections.
use crate::{Error, Model, Result, face_domain::FaceDomain};
use nurbs_core::surface_contact_search::{self, Report as Search};
pub use crate::face_contact_groups::Certificate as DisjointGroup;
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// Maximum individually classified pairs. Sweep range certificates use
    /// the shared geometry-cell budget instead and are reported separately.
    pub pairs: usize,
    pub cells: usize,
    pub domain_cells: usize,
    pub cells_per_pair: usize,
    pub domain_cells_per_pair: usize,
}
#[derive(Clone, Debug)]
pub enum SharedBoundary {
    SweepCap {
        cap: usize,
        wall: usize,
        edge: usize,
    },
    ExactHull(crate::boundary_hull_contact::Certificate),
    PlanarFace(crate::shared_boundary::Certificate),
    OppositeSides(crate::shared_boundary::OppositeSidesCertificate),
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
    pub reason: &'static str,
}
#[derive(Clone, Debug)]
pub struct Report {
    pub pairs: Vec<Pair>,
    pub disjoint_groups: Vec<DisjointGroup>,
    pub grouped_pairs: usize,
    /// Includes preparation and unsuccessful group attempts.
    pub group_cells: usize,
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
pub(crate) fn inspect_with_hulls(
    model: &Model,
    tolerance_uv: f64,
    limits: Limits,
    hulls: &[crate::boundary_hull_contact::Certificate],
) -> Result<Report> {
    inspect_with_certificates(model, tolerance_uv, limits, hulls, &[])
}
/// Cap tuples originate only from native recomputation in boundary embedding.
pub(crate) fn inspect_with_certificates(
    model: &Model,
    tolerance_uv: f64,
    limits: Limits,
    hulls: &[crate::boundary_hull_contact::Certificate],
    caps: &[[usize; 3]],
) -> Result<Report> {
    inspect_impl(model, tolerance_uv, limits, hulls, caps, !caps.is_empty(),false)
}
/// Sweep-only exact hull separation, also for closed bodies without caps.
/// Every plane is proved on actual positive rational control coefficients.
pub(crate) fn inspect_with_sweep_certificates(
    model: &Model,
    tolerance_uv: f64,
    limits: Limits,
    hulls: &[crate::boundary_hull_contact::Certificate],
    caps: &[[usize; 3]],
    joint_exact_domain: bool,
) -> Result<Report> {
    inspect_impl(model, tolerance_uv, limits, hulls, caps, true,joint_exact_domain)
}
fn inspect_impl(
    model: &Model,
    tolerance_uv: f64,
    limits: Limits,
    hulls: &[crate::boundary_hull_contact::Certificate],
    caps: &[[usize; 3]],
    sweep_hulls: bool,
    joint_exact_domain: bool,
) -> Result<Report> {
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
    let n = model.faces.len();
    let total_pairs = n
        .checked_mul(n.saturating_sub(1))
        .ok_or_else(|| Error::new("BREP_CONTACT_BUDGET", "Too many face pairs"))?
        / 2;
    let mut out = Report {
        pairs: Vec::new(),
        disjoint_groups: Vec::new(),
        grouped_pairs: 0,
        group_cells: 0,
        total_pairs,
        next_pair: None,
        cells: 0,
        domain_cells: 0,
        all_pairs_disjoint: true,
        all_pairs_classified: true,
    };
    let cover = if sweep_hulls && n > 1 {
        let (cover,cells)=crate::face_contact_groups::Cover::prepare(model,limits.cells);
        out.cells+=cells;out.group_cells+=cells;
        cover
    } else {None};
    for a in 0..n {
        let mut pending=cover.as_ref().map(|c|vec![c.root]).unwrap_or_default();
        let mut scalar_next=a+1;
        loop {
            let b=if let Some(cover)=&cover {
                let Some(index)=pending.pop() else {break;};
                let node=&cover.nodes[index];
                if node.range[1]<=a+1 {continue;}
                if node.range[0]>a && node.children.is_some() {
                    if out.cells==limits.cells {
                        out.next_pair=Some([a,node.range[0]]);
                        out.all_pairs_disjoint=false;out.all_pairs_classified=false;
                        return Ok(out);
                    }
                    out.cells+=1;out.group_cells+=1;
                    if let Some(certificate)=cover.certify(a,index) {
                        out.grouped_pairs+=certificate.range[1]-certificate.range[0];
                        out.disjoint_groups.push(certificate);
                        continue;
                    }
                }
                if let Some([left,right])=node.children {
                    pending.push(right);pending.push(left);continue;
                }
                node.range[0]
            } else {
                if scalar_next==n {break;}
                let b=scalar_next;scalar_next+=1;b
            };
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
            let boundary = if let Some(c) = caps
                .iter()
                .find(|c| [c[0].min(c[1]), c[0].max(c[1])] == [a, b])
            {
                Some(SharedBoundary::SweepCap {
                    cap: c[0],
                    wall: c[1],
                    edge: c[2],
                })
            } else if let Some(c) = hulls.iter().find(|c| c.faces == [a, b]) {
                Some(SharedBoundary::ExactHull(c.clone()))
            } else if let Some(c) = joint_exact_domain.then(||crate::boundary_hull_contact::certify(model,[a,b])).flatten() {
                // Only the joint embedding audit authorizes this recomputation.
                // The same individual-pair limit bounds all such attempts,
                // including pairs beyond the old lexicographic prefix.
                Some(SharedBoundary::ExactHull(c))
            } else if let Some(c) = crate::shared_boundary::certify(model, [a, b]) {
                Some(SharedBoundary::PlanarFace(c))
            } else {
                crate::shared_boundary::certify_opposite(model, [a, b])?
                    .map(SharedBoundary::OppositeSides)
            };
            let periodic = sa.periodic_u || sa.periodic_v || sb.periodic_u || sb.periodic_v;
            // Exact control-hull separation is the fast path only for sweep
            // certification, including closed no-cap bodies: the plain face-contact
            // API keeps the frozen trim-search work accounting byte-for-byte.
            let (hull_disjoint, hull_cells) = if boundary.is_none() && !periodic && sweep_hulls {
                crate::control_hull_separation::inspect(
                    sa,
                    sb,
                    (limits.cells - out.cells)
                        .min(limits.cells_per_pair / 2)
                        .min(64),
                )
            } else {
                (false, 0)
            };
            out.cells += hull_cells;
            let result = if boundary.is_some()
                || hull_disjoint
                || periodic
                || out.cells == limits.cells
                || hull_cells == limits.cells_per_pair
            {
                None
            } else {
                let search = if sweep_hulls {
                    surface_contact_search::search_trimmed_with_control_hulls
                } else {
                    surface_contact_search::search_trimmed
                };
                Some(search(
                    sa,
                    sb,
                    [&domains[a].region, &domains[b].region],
                    (limits.cells - out.cells).min(limits.cells_per_pair - hull_cells),
                    (limits.domain_cells - out.domain_cells).min(limits.domain_cells_per_pair),
                )?)
            };
            let reason = match &result {
                None if hull_disjoint => "pair-disjoint",
                None if boundary.is_some() => "shared-boundary",
                None if sa.periodic_u || sa.periodic_v || sb.periodic_u || sb.periodic_v => {
                    "periodic-trim-not-supported"
                }
                None => "pair-unresolved",
                Some(r) if r.absence_proven => "pair-disjoint",
                Some(r) if r.contact.is_some() => "interior-contact",
                Some(_) => "pair-unresolved",
            };
            out.all_pairs_classified &= boundary.is_some()
                || hull_disjoint
                || result.as_ref().is_some_and(|r| r.absence_proven);
            out.all_pairs_disjoint &=
                hull_disjoint || result.as_ref().is_some_and(|r| r.absence_proven);
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
                reason,
            });
        }
    }
    debug_assert_eq!(out.pairs.len()+out.grouped_pairs,out.total_pairs);
    Ok(out)
}
#[cfg(test)]
mod tests {
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
    fn sweep_groups_partition_all_pairs_and_preserve_bounded_pending_suffix() {
        let mut model=crate::cuboid([0.;3],[1.;3]).unwrap();
        let original=model.faces[0].surface.clone();
        for (i,face) in model.faces.iter_mut().enumerate() {
            face.surface=original.clone();
            for row in &mut face.surface.control_points {for p in row {p[0]+=10.*i as f64;}}
        }
        for limits in [limits(),Limits {pairs:1,..limits()},Limits {cells:1,..limits()}] {
            let report=inspect_with_sweep_certificates(&model,1e-8,limits,&[],&[],false).unwrap();
            assert!(report.cells<=limits.cells && report.pairs.len()<=limits.pairs);
            let mut covered=report.pairs.iter().map(|p|p.faces).collect::<Vec<_>>();
            for c in &report.disjoint_groups {
                assert!(c.range[0]>c.face && c.range[1]<=model.faces.len());
                for b in c.range[0]..c.range[1] {covered.push([c.face,b]);}
            }
            assert_eq!(covered.len(),report.pairs.len()+report.grouped_pairs);
            covered.sort_unstable();
            let expected=(0..6).flat_map(|a|(a+1..6).map(move |b|[a,b])).collect::<Vec<_>>();
            assert_eq!(covered,expected[..covered.len()]);
            assert_eq!(report.cells,report.group_cells+report.pairs.iter().map(|p|p.hull_cells+p.result.as_ref().map_or(0,|r|r.cells)).sum::<usize>());
            if let Some(next)=report.next_pair {
                assert_eq!(next,expected[covered.len()]);assert!(!report.all_pairs_classified);
            } else {
                assert_eq!(covered.len(),15);assert!(report.all_pairs_classified);
                assert!(report.grouped_pairs>0);
            }
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
