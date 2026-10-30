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
            let report = crate::sweep_cap_contacts::inspect_with_separation(
                model,
                cap,
                caps,
                budget,
                Some(faces.faces[cap].absence_proven()),
                exact_domain,
                &mut |_| false,
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
    let caps_proven = cap_contacts
        .iter()
        .all(|(_, r)| r.cap_certified && r.all_cap_wall_contacts_certified);
    let proven = agreement.all_equal
        && agreement.all_joins_exact
        && trim.all_valid
        && winding
        && intersections.absence_proven
        && caps_proven;
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
#[path = "boundary_embedding/tests/mod.rs"]
mod tests;

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
                    Some(self.faces.as_ref().unwrap().faces[cap].absence_proven()),
                    self.exact_domain,
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
            let caps_proven = self
                .cap_contacts
                .iter()
                .all(|(_, r)| r.cap_certified && r.all_cap_wall_contacts_certified);
            let proven = agreement.all_equal
                && agreement.all_joins_exact
                && trim.all_valid
                && self.winding
                && intersections.absence_proven
                && caps_proven;
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
                Some(faces.faces[cap].absence_proven()),
                exact_domain,
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
    let caps_proven = cap_contacts
        .iter()
        .all(|(_, r)| r.cap_certified && r.all_cap_wall_contacts_certified);
    let proven = agreement.all_equal
        && agreement.all_joins_exact
        && trim.all_valid
        && winding
        && intersections.absence_proven
        && caps_proven;
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
#[path="boundary_embedding/tests/incremental_tests.rs"]
mod incremental_tests;
