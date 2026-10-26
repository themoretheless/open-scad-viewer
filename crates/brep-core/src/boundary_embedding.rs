//! Joint evidence for a closed embedded boundary. Material roles, nesting and
//! outward orientation are deliberately separate requirements for solid volume.
pub mod regularity;
use crate::{Error, Model, Result, boundary_agreement, face_contacts, face_domain, self_intersection};

#[derive(Clone, Copy)]
pub struct Limits {
    pub exact_work: u64,
    pub trim_pairs: usize,
    pub trim_cells: usize,
    pub trim_domain_cells: usize,
    pub spans: usize,
    pub contacts: face_contacts::Limits,
}
pub struct Report {
    pub proven: bool,
    pub hull_contacts: Vec<crate::boundary_hull_contact::Certificate>,
    pub agreement: boundary_agreement::ExactReport,
    pub trim: face_domain::TrimAudit,
    pub intersections: self_intersection::Report,
    pub cap_contacts: Vec<(usize, crate::sweep_cap_contacts::Report)>,
}
/// Every component retains its unresolved entries. Budgets are explicit per
/// stage; callers must not interpret one successful stage as the whole result.
pub fn inspect(model: &Model, tolerance_uv: f64, limits: Limits) -> Result<Report> {
    inspect_impl(model, tolerance_uv, limits, 0, None, &[], false, true)
}
/// Optional independent shared whole-chart oblique injectivity cell budget.
pub fn inspect_with_linear(
    model: &Model,
    tolerance_uv: f64,
    limits: Limits,
    max_linear_cells: usize,
) -> Result<Report> {
    inspect_impl(
        model,
        tolerance_uv,
        limits,
        max_linear_cells,
        None,
        &[],
        false,
        false,
    )
}
/// Explicit cap selection; per-cap budgets are independent of joint audit budgets.
pub fn inspect_sweep(
    model: &Model,
    tolerance_uv: f64,
    limits: Limits,
    max_linear_cells: usize,
    caps: &[usize],
    cap_limits: crate::sweep_cap_contacts::Budgets,
) -> Result<Report> {
    inspect_impl(
        model,
        tolerance_uv,
        limits,
        max_linear_cells,
        if caps.is_empty() {
            None
        } else {
            Some((caps, cap_limits))
        },
        &[],
        true,
        false,
    )
}
pub fn inspect_sweep_with_projections(
    model: &Model,
    tolerance_uv: f64,
    limits: Limits,
    max_linear_cells: usize,
    caps: &[usize],
    cap_limits: crate::sweep_cap_contacts::Budgets,
    proposals: &[Option<[[i8; 3]; 2]>],
) -> Result<Report> {
    inspect_impl(
        model,
        tolerance_uv,
        limits,
        max_linear_cells,
        if caps.is_empty() {
            None
        } else {
            Some((caps, cap_limits))
        },
        proposals,
        true,
        false,
    )
}
fn inspect_impl(
    model: &Model,
    tolerance_uv: f64,
    limits: Limits,
    max_linear_cells: usize,
    caps: Option<(&[usize], crate::sweep_cap_contacts::Budgets)>,
    proposals: &[Option<[[i8; 3]; 2]>],
    sweep_hulls: bool,
    compatibility: bool,
) -> Result<Report> {
    // Includes connected shells, vertex fans, opposite edge uses and ownership.
    model.validate()?;
    if model.shells.is_empty() || model.shells.iter().any(|s| !s.closed || s.faces.is_empty()) {
        return Err(Error::new(
            "BREP_INVALID_INPUT",
            "Embedding audit requires nonempty closed shells",
        ));
    }
    let agreement = boundary_agreement::verify_exact(model, limits.exact_work)?;
    let trim = face_domain::audit_trim_regions(
        model,
        tolerance_uv,
        limits.trim_pairs,
        limits.trim_cells,
        limits.trim_domain_cells,
    )?;
    let faces = if compatibility {
        crate::face_injectivity::inspect(model, limits.spans)?
    } else {
        crate::face_injectivity::inspect_with_projections(
            model,
            limits.spans,
            max_linear_cells,
            proposals,
        )?
    };
    let winding = trim.faces.iter().all(|r| {
        r.as_ref()
            .is_some_and(|r| r.winding.first() == Some(&Some(1)))
    });
    let mut hull_contacts = Vec::new();
    let exact_domain = agreement.all_equal
        && agreement.all_joins_exact
        && trim.all_valid
        && winding
        && faces.all_faces_injective;
    if exact_domain && !sweep_hulls {
        // Bound certificate enumeration by the same lexicographic pair prefix.
        let mut visited = 0;
        'pairs: for a in 0..model.faces.len() {
            for b in a + 1..model.faces.len() {
                if visited == limits.contacts.pairs {
                    break 'pairs;
                }
                visited += 1;
                if let Some(c) = crate::boundary_hull_contact::certify(model, [a, b]) {
                    hull_contacts.push(c);
                }
            }
        }
    }
    let mut cap_contacts = Vec::new();
    let mut allowed_caps = Vec::new();
    if let Some((caps, budget)) = caps {
        if caps.is_empty() || caps.len() > 16 {
            return Err(Error::new(
                "BREP_SWEEP_CAP_CONTACT_INVALID",
                "Select 1..16 cap faces",
            ));
        }
        // Recompute even when another stage is unresolved so diagnostics remain
        // observable, but never authorize a pair without joint prerequisites.
        for &cap in caps {
            let report = crate::sweep_cap_contacts::inspect(model, cap, caps, budget)?;
            if exact_domain && report.cap_certified {
                for &[wall, edge] in &report.allowed_boundaries {
                    allowed_caps.push([cap, wall, edge]);
                }
            }
            cap_contacts.push((cap, report));
        }
    }
    let pairs = if sweep_hulls {
        face_contacts::inspect_with_sweep_certificates(
            model,
            tolerance_uv,
            limits.contacts,
            &hull_contacts,
            &allowed_caps,
            exact_domain,
        )?
    } else {
        face_contacts::inspect_with_certificates(
            model,
            tolerance_uv,
            limits.contacts,
            &hull_contacts,
            &allowed_caps,
            exact_domain,
        )?
    };
    if sweep_hulls {
        // Contacts are recomputed for the actual scalar traversal. Group gaps
        // can skip a large prefix without authorizing unvisited contacts.
        hull_contacts.extend(pairs.pairs.iter().filter_map(|pair| match &pair.boundary {
            Some(face_contacts::SharedBoundary::ExactHull(c)) => Some(c.clone()),
            _ => None,
        }));
    }
    let intersections = self_intersection::Report {
        absence_proven: faces.all_faces_injective && pairs.all_pairs_classified,
        faces,
        pairs,
    };
    let proven = agreement.all_equal
        && agreement.all_joins_exact
        && trim.all_valid
        && winding
        && intersections.absence_proven;
    Ok(Report {
        proven,
        hull_contacts,
        agreement,
        trim,
        intersections,
        cap_contacts,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn limits() -> Limits {
        Limits {
            exact_work: 1000000,
            trim_pairs: 1000,
            trim_cells: 10000,
            trim_domain_cells: 100000,
            spans: 100,
            contacts: face_contacts::Limits {
                pairs: 100,
                cells: 10000,
                domain_cells: 100000,
                cells_per_pair: 16,
                domain_cells_per_pair: 1000,
            },
        }
    }
    #[test]
    fn exact_quadrant_sweep_admits_contacts_only_after_joint_prerequisites() {
        let section = |z| {
            vec![
                [
                    [[0.5, 0.], [0.5, 0.5], [0., 0.5]],
                    [[0., 0.5], [-0.5, 0.5], [-0.5, 0.]],
                    [[-0.5, 0.], [-0.5, -0.5], [0., -0.5]],
                    [[0., -0.5], [0.5, -0.5], [0.5, 0.]],
                ]
                .into_iter()
                .map(|p| nurbs_core::curve::Curve {
                    degree: 2,
                    knots: vec![0., 0., 0., 1., 1., 1.],
                    control_points: p.into_iter().map(|xy| vec![xy[0], xy[1], z]).collect(),
                    weights: vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.],
                    periodic: false,
                })
                .collect(),
            ]
        };
        let model = crate::rational_section_loft(&[section(0.), section(5.)]).unwrap();
        let mut l = limits();
        l.contacts.pairs = 1;
        let report = inspect_with_linear(&model, 1e-8, l, 10000).unwrap();
        assert!(report.agreement.all_equal && report.agreement.all_joins_exact);
        assert!(report.trim.all_valid);
        assert!(report.intersections.faces.all_faces_injective);
        assert!(!report.hull_contacts.is_empty());
        assert!(report.intersections.pairs.pairs[0].boundary.is_some());
        assert!(report.intersections.pairs.next_pair.is_some());
        assert!(
            !report.proven,
            "one classified pair cannot certify the body"
        );
        let full = inspect_with_linear(&model, 1e-8, limits(), 10000).unwrap();
        assert!(full.intersections.pairs.next_pair.is_none());
        assert_eq!(full.intersections.pairs.pairs.len(), 15);
        assert!(
            full.proven,
            "exact curved-trim compatibility certificates complete this fixture"
        );
        assert!(
            full.intersections
                .pairs
                .pairs
                .iter()
                .filter(|p| p.reason == "pair-unresolved")
                .all(|p| p.faces[0] < 4 && p.faces[1] >= 4)
        );
        for pair in &full.intersections.pairs.pairs {
            println!(
                "retained pair {:?}: {} boundary={}",
                pair.faces,
                pair.reason,
                pair.boundary.is_some()
            );
        }
        println!("retained embedded boundary proven={}", full.proven);
        let cap_budget = crate::sweep_cap_contacts::Budgets {
            max_walls: 1024,
            max_exact_work: 1000000,
            max_chart_cells: 1000,
            max_trim_pairs: 100000,
            max_trim_cells: 100000,
            max_trim_domain_cells: 1000000,
        };
        let joined = inspect_sweep(&model, 1e-8, limits(), 10000, &[4, 5], cap_budget).unwrap();
        assert!(
            joined.proven,
            "joint cap contacts must complete this exact fixture"
        );
        assert_eq!(joined.cap_contacts.len(), 2);
        assert_eq!(
            joined
                .intersections
                .pairs
                .pairs
                .iter()
                .filter(|p| matches!(
                    p.boundary,
                    Some(face_contacts::SharedBoundary::SweepCap { .. })
                ))
                .count(),
            8
        );
        let short = inspect_sweep(
            &model,
            1e-8,
            limits(),
            10000,
            &[4, 5],
            crate::sweep_cap_contacts::Budgets {
                max_exact_work: 0,
                ..cap_budget
            },
        )
        .unwrap();
        assert!(
            short.proven,
            "independent complete face proofs remain admissible"
        );
        assert!(short.cap_contacts.iter().all(|(_, r)| !r.cap_certified));
        assert!(!short.intersections.pairs.pairs.iter().any(|p| matches!(
            p.boundary,
            Some(face_contacts::SharedBoundary::SweepCap { .. })
        )));
    }
    #[test]
    fn oblique_chart_proof_does_not_bypass_exact_boundary_prerequisites() {
        let section = |z| {
            vec![vec![
                nurbs_core::primitives::circle([0., 0., z], [0., 0., 1.], 0.5).unwrap(),
            ]]
        };
        let model = crate::rational_section_loft(&[section(0.), section(5.)]).unwrap();
        let mut l = limits();
        l.exact_work = 1;
        l.contacts.pairs = 1;
        let report = inspect_with_linear(&model, 1e-8, l, 10000).unwrap();
        assert!(report.intersections.faces.all_faces_injective);
        assert!(report.intersections.faces.linear_cells > 0);
        assert!(!report.agreement.all_equal);
        assert!(report.hull_contacts.is_empty());
        assert!(!report.proven);
        assert!(report.intersections.pairs.next_pair.is_some());
    }
    #[test]
    fn cube_requires_every_stage_and_preserves_input() {
        let m = crate::cuboid([0.; 3], [1.; 3]).unwrap();
        let before = format!("{m:?}");
        assert!(inspect(&m, 1e-8, limits()).unwrap().proven);
        for l in [
            Limits {
                exact_work: 1,
                ..limits()
            },
            Limits {
                trim_pairs: 1,
                ..limits()
            },
            Limits {
                trim_domain_cells: 1,
                ..limits()
            },
            Limits {
                spans: 1,
                ..limits()
            },
            Limits {
                contacts: face_contacts::Limits {
                    pairs: 1,
                    ..limits().contacts
                },
                ..limits()
            },
        ] {
            let r = inspect(&m, 1e-8, l).unwrap();
            assert!(!r.proven);
            assert_eq!(r.agreement.uses.len(), 24);
            assert_eq!(r.trim.faces.len(), 6);
            assert_eq!(r.intersections.faces.faces.len(), 6);
        }
        assert_eq!(format!("{m:?}"), before);
    }
    #[test]
    fn tolerance_accepted_gap_is_not_an_exact_embedded_boundary() {
        let mut m = crate::cuboid([0.; 3], [1.; 3]).unwrap();
        m.edges[0].curve.control_points[0][2] += 1e-12;
        m.validate().unwrap();
        let r = inspect(&m, 1e-8, limits()).unwrap();
        assert!(!r.agreement.all_equal);
        assert!(!r.proven);
    }

    #[test]
    fn exact_sphere_boundary_embedding_covers_poles_equator_edges_and_vertices() {
        let model = crate::analytic::sphere(3.).unwrap();
        let before = format!("{model:?}");
        let mut l = limits();
        l.spans = 1000;
        let r = inspect(&model, 1e-8, l).unwrap();
        assert!(r.agreement.all_equal && r.agreement.all_joins_exact && r.trim.all_valid);
        assert!(r.intersections.faces.all_faces_injective);
        for pair in [[0, 2], [1, 3], [4, 6], [5, 7]] {
            assert!(
                r.hull_contacts
                    .iter()
                    .any(|c| c.faces == pair && c.vertex.is_some())
            );
        }
        assert!(r.proven);
        assert!(r.intersections.absence_proven);
        assert!(r.intersections.pairs.all_pairs_classified);
        assert_eq!(
            r.hull_contacts
                .iter()
                .filter(|c| c.vertex.is_some())
                .count(),
            12
        );
        let rounded = crate::analytic::sphere(2.).unwrap();
        let r = inspect(&rounded, 1e-8, l).unwrap();
        assert!(!r.proven);
        assert!(!r.agreement.all_equal);
        assert!(r.hull_contacts.is_empty());
        assert_eq!(format!("{model:?}"), before);
    }
    #[test]
    fn trimmed_plane_hulls_require_complete_exact_boundary_prerequisites() {
        let model =
            crate::circular_blend::partial_annular_quarter(20., 5., 6., 1.25, 1., 1e-7).unwrap();
        let before = format!("{model:?}");
        let mut l = limits();
        l.spans = 4096;
        l.contacts.pairs = 400;
        l.contacts.cells = 150000;
        l.contacts.domain_cells = 1500000;
        l.contacts.cells_per_pair = 1024;
        l.contacts.domain_cells_per_pair = 100000;
        let r = inspect(&model, 1e-8, l).unwrap();
        assert!(r.agreement.all_equal && r.agreement.all_joins_exact && r.trim.all_valid);
        assert!(r.intersections.faces.all_faces_injective);
        assert!(
            r.hull_contacts
                .iter()
                .any(|c| c.faces == [0, 6] && c.vertex.is_some())
        );
        assert!(
            r.hull_contacts
                .iter()
                .any(|c| c.faces == [1, 23] && !c.edges.is_empty())
        );
        assert!(
            r.hull_contacts
                .iter()
                .any(|c| c.faces == [4, 9] && c.edges == vec![13])
        );
        assert!(
            r.hull_contacts
                .iter()
                .any(|c| c.faces == [1, 2] && c.vertex == Some(0))
        );
        assert!(
            r.intersections
                .pairs
                .pairs
                .iter()
                .any(|p| p.faces == [0, 1] && p.reason == "shared-boundary")
        );
        assert!(
            r.proven
                && r.intersections.absence_proven
                && r.intersections.pairs.all_pairs_classified
        );
        assert_eq!(
            r.hull_contacts
                .iter()
                .filter(|c| c.joined_proof.is_some())
                .count(),
            2
        );
        l.exact_work = 1;
        l.contacts.pairs = 1;
        let incomplete = inspect(&model, 1e-8, l).unwrap();
        assert!(!incomplete.agreement.all_equal && incomplete.hull_contacts.is_empty());
        assert!(!incomplete.proven);
        assert_eq!(format!("{model:?}"), before);
    }
}

pub(crate) struct Inspector {
    model: std::sync::Arc<Model>,
    tolerance_uv: f64,
    limits: Limits,
    sweep_hulls: bool,
    caps: Option<(Vec<usize>, crate::sweep_cap_contacts::Budgets)>,
    cap_next: usize,
    agreement: Option<boundary_agreement::ExactReport>,
    trim: Option<face_domain::TrimAudit>,
    face_work: Option<crate::face_injectivity::Inspector>,
    faces: Option<crate::face_injectivity::Report>,
    contact_work: Option<face_contacts::Inspector>,
    result: Option<Report>,
    hull_contacts: Vec<crate::boundary_hull_contact::Certificate>,
    cover: Option<crate::face_contact_groups::Cover>,
    cover_cells: usize,
    cap_contacts: Vec<(usize, crate::sweep_cap_contacts::Report)>,
    allowed_caps: Vec<[usize; 3]>,
    exact_domain: bool,
    winding: bool,
    prepared: bool,
    failed: bool,
}
impl Inspector {
    pub(crate) fn new(
        model: std::sync::Arc<Model>,
        tolerance_uv: f64,
        limits: Limits,
        max_linear_cells: usize,
        caps: Option<(Vec<usize>, crate::sweep_cap_contacts::Budgets)>,
        proposals: Vec<Option<[[i8; 3]; 2]>>,
        sweep_hulls: bool,
    ) -> Result<Self> {
        // Includes connected shells, vertex fans, opposite edge uses and ownership.
        model.validate()?;
        if model.shells.is_empty() || model.shells.iter().any(|s| !s.closed || s.faces.is_empty()) {
            return Err(Error::new(
                "BREP_INVALID_INPUT",
                "Embedding audit requires nonempty closed shells",
            ));
        }
        let agreement = boundary_agreement::verify_exact(&model, limits.exact_work)?;
        let trim = face_domain::audit_trim_regions(
            &model,
            tolerance_uv,
            limits.trim_pairs,
            limits.trim_cells,
            limits.trim_domain_cells,
        )?;
        let face_work = crate::face_injectivity::Inspector::new(
            model.clone(),
            limits.spans,
            max_linear_cells,
            proposals,
        )?;
        Ok(Self {
            model,
            tolerance_uv,
            limits,
            sweep_hulls,
            caps,
            cap_next: 0,
            agreement: Some(agreement),
            trim: Some(trim),
            face_work: Some(face_work),
            faces: None,
            contact_work: None,
            result: None,
            hull_contacts: vec![],
            cover: None,
            cover_cells: 0,
            cap_contacts: vec![],
            allowed_caps: vec![],
            exact_domain: false,
            winding: false,
            prepared: false,
            failed: false,
        })
    }
    pub(crate) fn finished(&self) -> bool {
        self.result.is_some()
    }
    pub(crate) fn advance(&mut self, max_units: usize) -> Result<usize> {
        if !(1..=100_000).contains(&max_units) {
            return Err(Error::new(
                "BREP_BOUNDARY_STEP",
                "Use a positive bounded boundary step",
            ));
        }
        if self.failed {
            return Err(Error::new(
                "BREP_BOUNDARY_FAILED",
                "Discard a failed boundary audit",
            ));
        }
        let result = self.advance_inner(max_units);
        self.failed = result.is_err();
        result
    }
    fn advance_inner(&mut self, max_units: usize) -> Result<usize> {
        if self.finished() {
            return Ok(0);
        }
        if let Some(work) = &mut self.face_work {
            let used = work.advance(max_units)?;
            if work.finished() {
                self.faces = Some(self.face_work.take().unwrap().finish()?);
            }
            return Ok(used);
        }
        if !self.prepared {
            let model = self.model.as_ref();
            let limits = self.limits;
            let sweep_hulls = self.sweep_hulls;
            let faces = self.faces.as_ref().unwrap();
            let winding = self.trim.as_ref().unwrap().faces.iter().all(|r| {
                r.as_ref()
                    .is_some_and(|r| r.winding.first() == Some(&Some(1)))
            });
            let mut hull_contacts = Vec::new();
            let exact_domain = self.agreement.as_ref().unwrap().all_equal
                && self.agreement.as_ref().unwrap().all_joins_exact
                && self.trim.as_ref().unwrap().all_valid
                && winding
                && faces.all_faces_injective;
            if exact_domain && !sweep_hulls {
                // Bound certificate enumeration by the same lexicographic pair prefix.
                let mut visited = 0;
                'pairs: for a in 0..model.faces.len() {
                    for b in a + 1..model.faces.len() {
                        if visited == limits.contacts.pairs {
                            break 'pairs;
                        }
                        visited += 1;
                        if let Some(c) = crate::boundary_hull_contact::certify(&self.model, [a, b])
                        {
                            hull_contacts.push(c);
                        }
                    }
                }
            }
            // One original-pole cover shared by caps and the subsequent pair audit.
            // This is native same-invocation reuse, never a supplied certificate.
            let (cover, cover_cells) = if sweep_hulls
                && model.faces.len() > 1
                && (1..=1_000_000).contains(&limits.contacts.cells)
            {
                crate::face_contact_groups::Cover::prepare(&self.model, limits.contacts.cells)
            } else {
                (None, 0)
            };
            let cap_contacts = Vec::new();
            let allowed_caps = Vec::new();
            self.winding = winding;
            self.exact_domain = exact_domain;
            self.hull_contacts = hull_contacts;
            self.cover = cover;
            self.cover_cells = cover_cells;
            self.cap_contacts = cap_contacts;
            self.allowed_caps = allowed_caps;
            if self
                .caps
                .as_ref()
                .is_some_and(|(caps, _)| caps.is_empty() || caps.len() > 16)
            {
                return Err(Error::new(
                    "BREP_SWEEP_CAP_CONTACT_INVALID",
                    "Select 1..16 cap faces",
                ));
            }
            self.prepared = true;
            return Ok(1);
        }
        if let Some((caps, budget)) = &self.caps {
            if self.cap_next < caps.len() {
                let cap = caps[self.cap_next];
                let report = crate::sweep_cap_contacts::inspect_with_separation(
                    &self.model,
                    cap,
                    caps,
                    *budget,
                    &mut |wall| {
                        if !self.exact_domain || self.cover_cells >= self.limits.contacts.cells {
                            return false;
                        }
                        let Some(cover) = &self.cover else {
                            return false;
                        };
                        self.cover_cells += 1;
                        cover.pair_separated(cap, wall) == Some(true)
                    },
                )?;
                if self.exact_domain && report.cap_certified {
                    for &[wall, edge] in &report.allowed_boundaries {
                        self.allowed_caps.push([cap, wall, edge]);
                    }
                }
                self.cap_contacts.push((cap, report));
                self.cap_next += 1;
                return Ok(1);
            }
        }
        if self.contact_work.is_none() {
            self.contact_work = Some(face_contacts::Inspector::new(
                self.model.clone(),
                self.tolerance_uv,
                self.limits.contacts,
                self.hull_contacts.clone(),
                std::mem::take(&mut self.allowed_caps),
                self.sweep_hulls,
                self.exact_domain,
                self.sweep_hulls
                    .then(|| (self.cover.take(), self.cover_cells)),
            )?);
            return Ok(1);
        }
        let work = self.contact_work.as_mut().unwrap();
        let used = work.advance(max_units)?;
        if work.finished() {
            let pairs = self.contact_work.take().unwrap().finish()?;
            if self.sweep_hulls {
                self.hull_contacts.extend(pairs.pairs.iter().filter_map(
                    |pair| match &pair.boundary {
                        Some(face_contacts::SharedBoundary::ExactHull(c)) => Some(c.clone()),
                        _ => None,
                    },
                ));
            }
            let faces = self.faces.take().unwrap();
            let intersections = self_intersection::Report {
                absence_proven: faces.all_faces_injective && pairs.all_pairs_classified,
                faces,
                pairs,
            };
            let agreement = self.agreement.take().unwrap();
            let trim = self.trim.take().unwrap();
            let proven = agreement.all_equal
                && agreement.all_joins_exact
                && trim.all_valid
                && self.winding
                && intersections.absence_proven;
            self.result = Some(Report {
                proven,
                hull_contacts: std::mem::take(&mut self.hull_contacts),
                agreement,
                trim,
                intersections,
                cap_contacts: std::mem::take(&mut self.cap_contacts),
            });
        }
        Ok(used)
    }
    pub(crate) fn finish(self) -> Result<Report> {
        if self.failed {
            return Err(Error::new(
                "BREP_BOUNDARY_FAILED",
                "Discard a failed boundary audit",
            ));
        }
        self.result.ok_or_else(|| {
            Error::new(
                "BREP_BOUNDARY_PENDING",
                "Complete every boundary phase before requesting a report",
            )
        })
    }
}

#[cfg(test)]
fn frozen_inspect_impl(
    model: &Model,
    tolerance_uv: f64,
    limits: Limits,
    max_linear_cells: usize,
    caps: Option<(&[usize], crate::sweep_cap_contacts::Budgets)>,
    proposals: &[Option<[[i8; 3]; 2]>],
    sweep_hulls: bool,
) -> Result<Report> {
    // Includes connected shells, vertex fans, opposite edge uses and ownership.
    model.validate()?;
    if model.shells.is_empty() || model.shells.iter().any(|s| !s.closed || s.faces.is_empty()) {
        return Err(Error::new(
            "BREP_INVALID_INPUT",
            "Embedding audit requires nonempty closed shells",
        ));
    }
    let agreement = boundary_agreement::verify_exact(model, limits.exact_work)?;
    let trim = face_domain::audit_trim_regions(
        model,
        tolerance_uv,
        limits.trim_pairs,
        limits.trim_cells,
        limits.trim_domain_cells,
    )?;
    let faces = crate::face_injectivity::inspect_with_projections(
        model,
        limits.spans,
        max_linear_cells,
        proposals,
    )?;
    let winding = trim.faces.iter().all(|r| {
        r.as_ref()
            .is_some_and(|r| r.winding.first() == Some(&Some(1)))
    });
    let mut hull_contacts = Vec::new();
    let exact_domain = agreement.all_equal
        && agreement.all_joins_exact
        && trim.all_valid
        && winding
        && faces.all_faces_injective;
    if exact_domain && !sweep_hulls {
        // Bound certificate enumeration by the same lexicographic pair prefix.
        let mut visited = 0;
        'pairs: for a in 0..model.faces.len() {
            for b in a + 1..model.faces.len() {
                if visited == limits.contacts.pairs {
                    break 'pairs;
                }
                visited += 1;
                if let Some(c) = crate::boundary_hull_contact::certify(model, [a, b]) {
                    hull_contacts.push(c);
                }
            }
        }
    }
    // One original-pole cover shared by caps and the subsequent pair audit.
    // This is native same-invocation reuse, never a supplied certificate.
    let (cover, mut cover_cells) =
        if sweep_hulls && model.faces.len() > 1 && (1..=1_000_000).contains(&limits.contacts.cells)
        {
            crate::face_contact_groups::Cover::prepare(model, limits.contacts.cells)
        } else {
            (None, 0)
        };
    let mut cap_contacts = Vec::new();
    let mut allowed_caps = Vec::new();
    if let Some((caps, budget)) = caps {
        if caps.is_empty() || caps.len() > 16 {
            return Err(Error::new(
                "BREP_SWEEP_CAP_CONTACT_INVALID",
                "Select 1..16 cap faces",
            ));
        }
        // Recompute even when another stage is unresolved so diagnostics remain
        // observable, but never authorize a pair without joint prerequisites.
        for &cap in caps {
            let report = crate::sweep_cap_contacts::inspect_with_separation(
                model,
                cap,
                caps,
                budget,
                &mut |wall| {
                    if !exact_domain || cover_cells >= limits.contacts.cells {
                        return false;
                    }
                    let Some(cover) = &cover else {
                        return false;
                    };
                    cover_cells += 1;
                    cover.pair_separated(cap, wall) == Some(true)
                },
            )?;
            if exact_domain && report.cap_certified {
                for &[wall, edge] in &report.allowed_boundaries {
                    allowed_caps.push([cap, wall, edge]);
                }
            }
            cap_contacts.push((cap, report));
        }
    }
    let pairs = if sweep_hulls {
        face_contacts::inspect_with_prepared_sweep_certificates(
            model,
            tolerance_uv,
            limits.contacts,
            &hull_contacts,
            &allowed_caps,
            exact_domain,
            (cover, cover_cells),
        )?
    } else {
        face_contacts::inspect_with_certificates(
            model,
            tolerance_uv,
            limits.contacts,
            &hull_contacts,
            &allowed_caps,
            exact_domain,
        )?
    };
    if sweep_hulls {
        // Contacts are recomputed for the actual scalar traversal. Group gaps
        // can skip a large prefix without authorizing unvisited contacts.
        hull_contacts.extend(pairs.pairs.iter().filter_map(|pair| match &pair.boundary {
            Some(face_contacts::SharedBoundary::ExactHull(c)) => Some(c.clone()),
            _ => None,
        }));
    }
    let intersections = self_intersection::Report {
        absence_proven: faces.all_faces_injective && pairs.all_pairs_classified,
        faces,
        pairs,
    };
    let proven = agreement.all_equal
        && agreement.all_joins_exact
        && trim.all_valid
        && winding
        && intersections.absence_proven;
    Ok(Report {
        proven,
        hull_contacts,
        agreement,
        trim,
        intersections,
        cap_contacts,
    })
}

#[cfg(test)]
mod incremental_tests {
    use super::*;
    fn limits() -> Limits {
        Limits {
            exact_work: 1000000,
            trim_pairs: 1000,
            trim_cells: 10000,
            trim_domain_cells: 100000,
            spans: 100,
            contacts: face_contacts::Limits {
                pairs: 100,
                cells: 10000,
                domain_cells: 100000,
                cells_per_pair: 16,
                domain_cells_per_pair: 1000,
            },
        }
    }
    fn report_identity(r: &Report) -> String {
        format!(
            "{:?}",
            (
                r.proven,
                &r.hull_contacts,
                (
                    r.agreement.all_equal,
                    &r.agreement.joins,
                    r.agreement.all_joins_exact,
                    r.agreement.work,
                    r.agreement
                        .uses
                        .iter()
                        .map(|u| (
                            u.face,
                            u.wire,
                            u.coedge,
                            u.edge,
                            u.decision.as_ref().map(|d| (&d.outcome, d.work_used))
                        ))
                        .collect::<Vec<_>>()
                ),
                (
                    r.trim
                        .faces
                        .iter()
                        .map(|face| face.as_ref().map(|f| (
                            f.valid,
                            f.reason,
                            &f.winding,
                            f.pairs,
                            f.cells,
                            f.domain_cells,
                            f.problem_loops,
                            f.loops
                                .iter()
                                .map(|l| (
                                    l.proven_simple,
                                    l.exact_joins,
                                    &l.injective,
                                    l.pairs
                                        .iter()
                                        .map(|p| (p.curves, p.proven))
                                        .collect::<Vec<_>>(),
                                    l.total_pairs,
                                    l.cells
                                ))
                                .collect::<Vec<_>>()
                        )))
                        .collect::<Vec<_>>(),
                    r.trim.all_valid,
                    r.trim.pairs,
                    r.trim.cells,
                    r.trim.domain_cells
                ),
                &r.intersections,
                &r.cap_contacts
            )
        )
    }
    #[test]
    fn resumable_boundary_preserves_joint_prerequisites_and_all_stage_reports() {
        let cube = crate::cuboid([0.; 3], [1.; 3]).unwrap();
        let mut gap = cube.clone();
        gap.edges[0].curve.control_points[0][2] += 1e-12;
        let cap_budget = crate::sweep_cap_contacts::Budgets {
            max_walls: 1024,
            max_exact_work: 1000000,
            max_chart_cells: 1000,
            max_trim_pairs: 100000,
            max_trim_cells: 100000,
            max_trim_domain_cells: 1000000,
        };
        for model in [cube, gap] {
            for sweep in [false, true] {
                for caps in [None, Some((vec![0, 1], cap_budget))] {
                    if !sweep && caps.is_some() {
                        continue;
                    }
                    for limits in [
                        limits(),
                        Limits {
                            spans: 1,
                            ..limits()
                        },
                        Limits {
                            contacts: face_contacts::Limits {
                                cells: 1,
                                ..limits().contacts
                            },
                            ..limits()
                        },
                    ] {
                        let expected = frozen_inspect_impl(
                            &model,
                            1e-8,
                            limits,
                            10000,
                            caps.as_ref().map(|(c, b)| (c.as_slice(), *b)),
                            &[],
                            sweep,
                        )
                        .unwrap();
                        for batch in [1, 2, 64] {
                            let mut work = Inspector::new(
                                std::sync::Arc::new(model.clone()),
                                1e-8,
                                limits,
                                10000,
                                caps.clone(),
                                vec![],
                                sweep,
                            )
                            .unwrap();
                            let mut calls = 0;
                            while !work.finished() {
                                assert!(work.advance(batch).unwrap() <= batch);
                                calls += 1;
                                assert!(calls < 1000);
                            }
                            assert_eq!(work.advance(batch).unwrap(), 0);
                            assert_eq!(
                                report_identity(&work.finish().unwrap()),
                                report_identity(&expected)
                            );
                        }
                    }
                }
            }
        }
    }
    #[test]
    fn resumable_boundary_refuses_unfinished_result() {
        let mut work = Inspector::new(
            std::sync::Arc::new(crate::cuboid([0.; 3], [1.; 3]).unwrap()),
            1e-8,
            limits(),
            10000,
            None,
            vec![],
            true,
        )
        .unwrap();
        assert!(work.advance(0).is_err());
        assert!(!work.finished());
        assert!(work.finish().is_err());
    }
    #[test]
    fn exact_quadrant_sweep_admits_contacts_only_after_joint_prerequisites() {
        let section = |z| {
            vec![
                [
                    [[0.5, 0.], [0.5, 0.5], [0., 0.5]],
                    [[0., 0.5], [-0.5, 0.5], [-0.5, 0.]],
                    [[-0.5, 0.], [-0.5, -0.5], [0., -0.5]],
                    [[0., -0.5], [0.5, -0.5], [0.5, 0.]],
                ]
                .into_iter()
                .map(|p| nurbs_core::curve::Curve {
                    degree: 2,
                    knots: vec![0., 0., 0., 1., 1., 1.],
                    control_points: p.into_iter().map(|xy| vec![xy[0], xy[1], z]).collect(),
                    weights: vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.],
                    periodic: false,
                })
                .collect(),
            ]
        };
        let model = crate::rational_section_loft(&[section(0.), section(5.)]).unwrap();
        let mut l = limits();
        l.contacts.pairs = 1;
        let report = inspect_with_linear(&model, 1e-8, l, 10000).unwrap();
        assert!(report.agreement.all_equal && report.agreement.all_joins_exact);
        assert!(report.trim.all_valid);
        assert!(report.intersections.faces.all_faces_injective);
        assert!(!report.hull_contacts.is_empty());
        assert!(report.intersections.pairs.pairs[0].boundary.is_some());
        assert!(report.intersections.pairs.next_pair.is_some());
        assert!(
            !report.proven,
            "one classified pair cannot certify the body"
        );
        let full = inspect_with_linear(&model, 1e-8, limits(), 10000).unwrap();
        assert!(full.intersections.pairs.next_pair.is_none());
        assert_eq!(full.intersections.pairs.pairs.len(), 15);
        // Exact hull contacts now cover these caps without an optional sweep
        // certificate. Keep the earlier one-pair budget refusal above: a
        // successful contact is still insufficient without complete coverage.
        assert!(full.proven);
        assert!(
            full.intersections
                .pairs
                .pairs
                .iter()
                .all(|p| p.reason != "pair-unresolved")
        );
        for pair in &full.intersections.pairs.pairs {
            println!(
                "retained pair {:?}: {} boundary={}",
                pair.faces,
                pair.reason,
                pair.boundary.is_some()
            );
        }
        println!("retained embedded boundary proven={}", full.proven);
        let cap_budget = crate::sweep_cap_contacts::Budgets {
            max_walls: 1024,
            max_exact_work: 1000000,
            max_chart_cells: 1000,
            max_trim_pairs: 100000,
            max_trim_cells: 100000,
            max_trim_domain_cells: 1000000,
        };
        let joined = inspect_sweep(&model, 1e-8, limits(), 10000, &[4, 5], cap_budget).unwrap();
        assert!(
            joined.proven,
            "joint cap contacts must complete this exact fixture"
        );
        assert_eq!(joined.cap_contacts.len(), 2);
        assert_eq!(
            joined
                .intersections
                .pairs
                .pairs
                .iter()
                .filter(|p| matches!(
                    p.boundary,
                    Some(face_contacts::SharedBoundary::SweepCap { .. })
                ))
                .count(),
            8
        );
        let short = inspect_sweep(
            &model,
            1e-8,
            limits(),
            10000,
            &[4, 5],
            crate::sweep_cap_contacts::Budgets {
                max_exact_work: 0,
                ..cap_budget
            },
        )
        .unwrap();
        // Exhausting the optional cap prover cannot invalidate independent
        // original-face hull proofs; it must never manufacture a cap receipt.
        if short.proven {
            assert!(full.proven);
            assert!(short.intersections.pairs.next_pair.is_none());
        }
        assert!(short.cap_contacts.iter().all(|(_, r)| !r.cap_certified));
        assert!(!short.intersections.pairs.pairs.iter().any(|p| matches!(
            p.boundary,
            Some(face_contacts::SharedBoundary::SweepCap { .. })
        )));
    }
    #[test]
    fn oblique_chart_proof_does_not_bypass_exact_boundary_prerequisites() {
        let section = |z| {
            vec![vec![
                nurbs_core::primitives::circle([0., 0., z], [0., 0., 1.], 0.5).unwrap(),
            ]]
        };
        let model = crate::rational_section_loft(&[section(0.), section(5.)]).unwrap();
        let mut l = limits();
        l.exact_work = 1;
        l.contacts.pairs = 1;
        let report = inspect_with_linear(&model, 1e-8, l, 10000).unwrap();
        assert!(report.intersections.faces.all_faces_injective);
        assert!(report.intersections.faces.linear_cells > 0);
        assert!(!report.agreement.all_equal);
        assert!(report.hull_contacts.is_empty());
        assert!(!report.proven);
        assert!(report.intersections.pairs.next_pair.is_some());
    }
    #[test]
    fn cap_cover_queries_share_the_existing_contact_budget() {
        let model=crate::cuboid([0.;3],[1.;3]).unwrap();
        let budget=crate::sweep_cap_contacts::Budgets {max_walls:1024,max_exact_work:1000000,
            max_chart_cells:1000,max_trim_pairs:100000,max_trim_cells:100000,max_trim_domain_cells:1000000};
        let full=inspect_sweep(&model,1e-8,limits(),10000,&[0,1],budget).unwrap();
        assert!(full.proven);
        let (_,preparation)=crate::face_contact_groups::Cover::prepare(&model,10000);
        assert!(full.intersections.pairs.group_cells>=preparation);
        for cells in [1,preparation-1,preparation,preparation+1] {
            let restricted=inspect_sweep(&model,1e-8,Limits {contacts:face_contacts::Limits {cells,..limits().contacts},..limits()},10000,&[0,1],budget).unwrap();
            assert!(!restricted.proven);
            assert!(restricted.intersections.pairs.cells<=cells);
            assert!(restricted.intersections.pairs.group_cells<=cells);
        }
        let mut gap=model.clone();
        gap.edges[0].curve.control_points[0][2]+=1e-12;
        let refused=inspect_sweep(&gap,1e-8,limits(),10000,&[0,1],budget).unwrap();
        assert!(!refused.proven);
        assert!(!refused.agreement.all_equal);
    }
    #[test]
    fn cube_requires_every_stage_and_preserves_input() {
        let m = crate::cuboid([0.;3], [1.;3]).unwrap();
        let before = format!("{m:?}");
        assert!(inspect(&m,1e-8,limits()).unwrap().proven);
        for l in [Limits { exact_work: 1, ..limits() },
            Limits { trim_pairs: 1, ..limits() }, Limits { trim_domain_cells: 1, ..limits() },
            Limits { spans: 1, ..limits() },
            Limits { contacts: face_contacts::Limits { pairs: 1, ..limits().contacts }, ..limits() }] {
            let r = inspect(&m,1e-8,l).unwrap();
            assert!(!r.proven);
            assert_eq!(r.agreement.uses.len(),24);
            assert_eq!(r.trim.faces.len(),6);
            assert_eq!(r.intersections.faces.faces.len(),6);
        }
        assert_eq!(format!("{m:?}"),before);
    }
    #[test]
    fn tolerance_accepted_gap_is_not_an_exact_embedded_boundary() {
        let mut m = crate::cuboid([0.;3], [1.;3]).unwrap();
        m.edges[0].curve.control_points[0][2] += 1e-12;
        m.validate().unwrap();
        let r = inspect(&m,1e-8,limits()).unwrap();
        assert!(!r.agreement.all_equal);
        assert!(!r.proven);
    }
}
