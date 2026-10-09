//! Resume only the verified pending frontier; never replay excluded products.
use super::*;
fn refine<F>(
    report: &mut SupportPairReport,
    cells: usize,
    spans: usize,
    domains: usize,
    mut inspect: F,
) -> Result<()>
where
    F: FnMut(SupportProduct, usize, usize, usize) -> Result<SupportPairReport>,
{
    let mut pending = Vec::new();
    let mut separated = Vec::new();
    let mut outside = Vec::new();
    let (mut used_cells, mut used_spans, mut used_domains) = (0, 0, 0);
    for (index, &product) in report.pending.iter().enumerate() {
        let remaining = report.pending.len() - index;
        let cell_budget = (cells - used_cells) / remaining;
        let span_budget = (spans - used_spans) / remaining;
        let domain_budget = (domains - used_domains) / remaining;
        if cell_budget == 0 || span_budget == 0 {
            pending.push(product);
            continue;
        }
        let step = inspect(product, cell_budget, span_budget, domain_budget)?;
        if step.cells > cell_budget || step.spans > span_budget || step.domain_cells > domain_budget
        {
            return Err(Error::new(
                "BREP_PROFILE_CONTACTS",
                "Refinement exceeded shared work",
            ));
        }
        used_cells += step.cells;
        used_spans += step.spans;
        used_domains += step.domain_cells;
        pending.extend(step.pending);
        separated.extend(step.separated);
        outside.extend(step.outside);
    }
    report.pending = pending;
    report.separated.extend(separated);
    report.outside.extend(outside);
    report.cells += used_cells;
    report.spans += used_spans;
    report.domain_cells += used_domains;
    Ok(())
}
fn validate_resume(
    owners: &[ShellFace],
    out: &ShellContactAudit<'_>,
    limits: ContactAuditLimits,
    additional_cells_per_pair: usize,
) -> Result<()> {
    if !std::ptr::eq(out.owners, owners)
        || additional_cells_per_pair > 100000
        || limits.cells > 100000
        || limits.cells_per_pair > 100000
        || limits.spans > 100000
        || limits.domain_cells > 100000
        || limits.domain_cells_per_pair > 100000
        || limits.controls_per_enclosure > 1000000
        || limits.exact_work > 100000000
        || limits.driver_cells > 100000
        || !limits.tolerance_uv.is_finite()
        || limits.tolerance_uv <= 0.
        || out.cells > limits.cells
        || out.spans > limits.spans
        || out.domain_cells > limits.domain_cells
        || out.exact_work > limits.exact_work
        || out.driver_cells > limits.driver_cells
    {
        return Err(Error::new(
            "BREP_PROFILE_CONTACTS",
            "Require owned audit and cumulative bounded work",
        ));
    }
    Ok(())
}
impl ExpressionShell<'_, '_, '_, '_, '_, '_, '_, '_> {
    /// Fresh complete audit plus bounded refinement of unqualified original
    /// pairs. The immutable shell produces the frontier internally; callers
    /// cannot inject serialized diagnostics as geometric authority.
    pub fn inspect_contacts_with_profile_region_refined(
        &self,
        limits: ContactAuditLimits,
        region: &ProfileProjectedRegion<'_, '_, '_, '_, '_>,
        additional_cells_per_pair: usize,
    ) -> Result<ShellContactAudit<'_>> {
        self.refine_profile_contacts(limits, region, additional_cells_per_pair, false)
    }
    /// Refine every still-unqualified pair using the actual retained/profile
    /// and transition expressions. Qualified contact loci remain unchanged.
    pub fn inspect_all_contacts_with_profile_region_refined(
        &self,
        limits: ContactAuditLimits,
        region: &ProfileProjectedRegion<'_, '_, '_, '_, '_>,
        additional_cells_per_pair: usize,
    ) -> Result<ShellContactAudit<'_>> {
        self.refine_profile_contacts(limits, region, additional_cells_per_pair, true)
    }
    fn refine_profile_contacts(
        &self,
        limits: ContactAuditLimits,
        region: &ProfileProjectedRegion<'_, '_, '_, '_, '_>,
        additional_cells_per_pair: usize,
        include_changed: bool,
    ) -> Result<ShellContactAudit<'_>> {
        if additional_cells_per_pair > 100000 {
            return Err(Error::new(
                "BREP_PROFILE_CONTACTS",
                "Bound additional pair refinement",
            ));
        }
        let mut out = self.inspect_contacts_with_profile_region(limits, region)?;
        self.resume_contact_audit_impl(
            &mut out,
            limits,
            additional_cells_per_pair,
            include_changed,
            false,
        )?;
        Ok(out)
    }
    /// Continue an audit belonging to this immutable shell. Limits include all
    /// previous work; diagnostics cannot be used to manufacture this authority.
    pub fn resume_contact_audit<'x>(
        &'x self,
        audit: &mut ShellContactAudit<'x>,
        limits: ContactAuditLimits,
        additional_cells_per_pair: usize,
    ) -> Result<()> {
        self.resume_contact_audit_impl(audit, limits, additional_cells_per_pair, true, false)
    }
    /// Spend remaining support-query work only on pending transition pairs.
    pub fn resume_transition_contact_audit<'x>(
        &'x self,
        audit: &mut ShellContactAudit<'x>,
        limits: ContactAuditLimits,
        additional_cells_per_pair: usize,
    ) -> Result<()> {
        self.resume_contact_audit_impl(audit, limits, additional_cells_per_pair, true, true)
    }
    /// Requalify pending unchanged-source contacts with fresh linear-chart
    /// injectivity. Every computation is charged to the cumulative audit.
    pub fn qualify_remaining_original_contacts<'x>(
        &'x self,
        audit: &mut ShellContactAudit<'x>,
        limits: ContactAuditLimits,
        max_linear_cells: usize,
        linear_cells_per_pair: usize,
    ) -> Result<()> {
        if max_linear_cells > 100000
            || linear_cells_per_pair > 100000
            || audit.linear_cells > max_linear_cells
        {
            return Err(Error::new(
                "BREP_PROFILE_CONTACTS",
                "Bound cumulative linear-chart work",
            ));
        }
        self.resume_contact_audit(audit, limits, 0)?;
        if audit.linear_cells == max_linear_cells || linear_cells_per_pair == 0 {
            return Ok(());
        }
        let candidate = |p: &AuditedContactPair<'_>| {
            matches!(p.proof, PairContactProof::Pending)
                && p.support
                    .faces
                    .iter()
                    .all(|&i| matches!(self.faces[i].kind(), ShellFaceKind::Original(_)))
        };
        let mut remaining = audit.pairs.iter().filter(|p| candidate(p)).count();
        for pair in &mut audit.pairs {
            if !candidate(pair) {
                continue;
            }
            let linear =
                ((max_linear_cells - audit.linear_cells) / remaining).min(linear_cells_per_pair);
            let work = (limits.exact_work - audit.exact_work) / remaining as u64;
            let spans = (limits.spans - audit.spans) / remaining;
            let drivers = (limits.driver_cells - audit.driver_cells) / remaining;
            remaining -= 1;
            if linear == 0 || work == 0 || spans == 0 || drivers == 0 {
                continue;
            }
            let report = self.qualify_original_additional_contact_with_linear_chart(
                pair.support.faces,
                work,
                spans,
                drivers,
                linear,
            )?;
            audit.exact_work += report.exact_work;
            audit.spans += report.spans;
            audit.driver_cells += report.driver_cells;
            audit.linear_cells += report.linear_cells;
            pair.reason = report.reason;
            if let Some(certificate) = report.certificate {
                pair.proof = PairContactProof::VertexOrInterior(certificate);
            }
        }
        Ok(())
    }
    /// Fresh strict separators, with no inheritance from Body diagnostics.
    pub fn qualify_remaining_original_hulls<'x>(
        &'x self,
        audit: &mut ShellContactAudit<'x>,
        limits: ContactAuditLimits,
    ) -> Result<()> {
        self.resume_contact_audit(audit, limits, 0)?;
        let candidate = |p: &AuditedContactPair<'_>| {
            matches!(p.proof, PairContactProof::Pending)
                && p.support
                    .faces
                    .iter()
                    .all(|&i| matches!(self.faces[i].kind(), ShellFaceKind::Original(_)))
        };
        let mut remaining = audit.pairs.iter().filter(|p| candidate(p)).count();
        for pair in &mut audit.pairs {
            if !candidate(pair) {
                continue;
            }
            let budget = (limits.exact_work - audit.exact_work) / remaining as u64;
            remaining -= 1;
            if budget == 0 {
                continue;
            }
            let report = self.qualify_original_hull_contact(pair.support.faces, budget)?;
            audit.exact_work += report.exact_work;
            pair.reason = report.reason;
            if let Some(c) = report.certificate {
                pair.proof = PairContactProof::Hull(c);
            }
        }
        Ok(())
    }
    pub fn qualify_remaining_original_poles<'x>(
        &'x self,
        audit: &mut ShellContactAudit<'x>,
        limits: ContactAuditLimits,
    ) -> Result<()> {
        self.resume_contact_audit(audit, limits, 0)?;
        let candidate = |p: &AuditedContactPair<'_>| {
            matches!(p.proof, PairContactProof::Pending)
                && p.support
                    .faces
                    .iter()
                    .all(|&i| matches!(self.faces[i].kind(), ShellFaceKind::Original(_)))
        };
        let mut remaining = audit.pairs.iter().filter(|p| candidate(p)).count();
        for pair in &mut audit.pairs {
            if !candidate(pair) {
                continue;
            }
            let work = (limits.exact_work - audit.exact_work) / remaining as u64;
            let driver = (limits.driver_cells - audit.driver_cells) / remaining;
            remaining -= 1;
            if work == 0 || driver == 0 {
                continue;
            }
            let report = self.qualify_original_pole_contact(pair.support.faces, work, driver)?;
            audit.exact_work += report.exact_work;
            audit.driver_cells += report.driver_cells;
            pair.reason = report.reason;
            if let Some(c) = report.certificate {
                pair.proof = PairContactProof::Pole(c);
            }
        }
        Ok(())
    }
    pub fn qualify_remaining_profile_hulls<'x>(
        &'x self,
        audit: &mut ShellContactAudit<'x>,
        limits: ContactAuditLimits,
    ) -> Result<()> {
        self.resume_contact_audit(audit, limits, 0)?;
        let candidate = |p: &AuditedContactPair<'_>| {
            matches!(p.proof, PairContactProof::Pending)
                && matches!(
                    p.support.faces.map(|i| self.faces[i].kind()),
                    [ShellFaceKind::Original(_), ShellFaceKind::Retained(_)]
                        | [ShellFaceKind::Retained(_), ShellFaceKind::Original(_)]
                )
        };
        let mut remaining = audit.pairs.iter().filter(|p| candidate(p)).count();
        for pair in &mut audit.pairs {
            if !candidate(pair) {
                continue;
            }
            let work = (limits.exact_work - audit.exact_work) / remaining as u64;
            remaining -= 1;
            if work == 0 {
                continue;
            }
            let report = self.qualify_profile_hull_contact(pair.support.faces, work)?;
            audit.exact_work += report.exact_work;
            pair.reason = report.reason;
            if let Some(c) = report.certificate {
                pair.proof = PairContactProof::ProfileHull(c);
            }
        }
        Ok(())
    }
    pub fn qualify_remaining_original_boundaries<'x>(
        &'x self,
        audit: &mut ShellContactAudit<'x>,
        limits: ContactAuditLimits,
    ) -> Result<()> {
        self.resume_contact_audit(audit, limits, 0)?;
        let candidate = |p: &AuditedContactPair<'_>| {
            matches!(p.proof, PairContactProof::Pending)
                && p.support
                    .faces
                    .iter()
                    .all(|&i| matches!(self.faces[i].kind(), ShellFaceKind::Original(_)))
        };
        let mut remaining = audit.pairs.iter().filter(|p| candidate(p)).count();
        for pair in &mut audit.pairs {
            if !candidate(pair) {
                continue;
            }
            let work = (limits.exact_work - audit.exact_work) / remaining as u64;
            let driver = (limits.driver_cells - audit.driver_cells) / remaining;
            remaining -= 1;
            if work == 0 || driver == 0 {
                continue;
            }
            let report = self.qualify_original_boundary_contact(
                pair.support.faces,
                limits.tolerance_uv,
                work,
                driver,
            )?;
            audit.exact_work += report.exact_work;
            audit.driver_cells += report.driver_cells;
            pair.reason = report.reason;
            if let Some(c) = report.certificate {
                pair.proof = PairContactProof::Boundary(c);
            }
        }
        Ok(())
    }
    pub fn qualify_remaining_transition_coordinates<'x>(
        &'x self,
        audit: &mut ShellContactAudit<'x>,
        limits: ContactAuditLimits,
        cells_per_pair: usize,
    ) -> Result<()> {
        self.resume_contact_audit(audit, limits, 0)?;
        if cells_per_pair > 100000 {
            return Err(Error::new(
                "BREP_PROFILE_CONTACTS",
                "Bound coordinate pair refinement",
            ));
        }
        let candidate = |p: &AuditedContactPair<'_>| {
            matches!(p.proof, PairContactProof::Pending)
                && matches!(
                    p.support.faces.map(|i| self.faces[i].kind()),
                    [ShellFaceKind::Original(_), ShellFaceKind::Transition]
                        | [ShellFaceKind::Transition, ShellFaceKind::Original(_)]
                )
        };
        let mut remaining = audit.pairs.iter().filter(|p| candidate(p)).count();
        for pair in &mut audit.pairs {
            if !candidate(pair) {
                continue;
            }
            let cells = ((limits.cells - audit.cells) / remaining).min(cells_per_pair);
            let spans = (limits.spans - audit.spans) / remaining;
            remaining -= 1;
            if cells == 0 || spans == 0 {
                continue;
            }
            let report = self.qualify_transition_coordinate_contact(
                pair.support.faces,
                cells,
                limits.controls_per_enclosure,
                spans,
            )?;
            audit.cells += report.cells;
            audit.spans += report.spans;
            pair.reason = report.reason;
            if let Some(c) = report.certificate {
                pair.proof = PairContactProof::TransitionCoordinate(c);
            }
        }
        Ok(())
    }
    pub fn qualify_remaining_profile_vertices<'x>(
        &'x self,
        audit: &mut ShellContactAudit<'x>,
        limits: ContactAuditLimits,
    ) -> Result<()> {
        self.resume_contact_audit(audit, limits, 0)?;
        let candidate = |p: &AuditedContactPair<'_>| {
            matches!(p.proof, PairContactProof::Pending)
                && matches!(
                    p.support.faces.map(|i| self.faces[i].kind()),
                    [ShellFaceKind::Original(_), ShellFaceKind::Retained(_)]
                        | [ShellFaceKind::Retained(_), ShellFaceKind::Original(_)]
                )
        };
        let mut remaining = audit.pairs.iter().filter(|p| candidate(p)).count();
        for pair in &mut audit.pairs {
            if !candidate(pair) {
                continue;
            }
            let work = (limits.exact_work - audit.exact_work) / remaining as u64;
            remaining -= 1;
            if work == 0 {
                continue;
            }
            let report = self.qualify_profile_vertex_contact(pair.support.faces, work)?;
            audit.exact_work += report.exact_work;
            pair.reason = report.reason;
            if let Some(c) = report.certificate {
                pair.proof = PairContactProof::ProfileVertex(c);
            }
        }
        Ok(())
    }
    pub fn qualify_remaining_transition_xy_bounds<'x>(
        &'x self,
        audit: &mut ShellContactAudit<'x>,
        limits: ContactAuditLimits,
        region: &ProfileProjectedRegion<'_, '_, '_, '_, '_>,
    ) -> Result<()> {
        self.resume_contact_audit(audit, limits, 0)?;
        let candidate = |p: &AuditedContactPair<'_>| {
            matches!(p.proof, PairContactProof::Pending)
                && matches!(
                    p.support.faces.map(|i| self.faces[i].kind()),
                    [ShellFaceKind::Original(_), ShellFaceKind::Transition]
                        | [ShellFaceKind::Transition, ShellFaceKind::Original(_)]
                )
        };
        let mut remaining = audit.pairs.iter().filter(|p| candidate(p)).count();
        for pair in &mut audit.pairs {
            if !candidate(pair) {
                continue;
            }
            let spans = (limits.spans - audit.spans) / remaining;
            remaining -= 1;
            if spans == 0 {
                continue;
            }
            let report =
                self.qualify_transition_xy_bounds_contact(pair.support.faces, region, spans)?;
            audit.spans += report.spans;
            pair.reason = report.reason;
            if let Some(c) = report.certificate {
                pair.proof = PairContactProof::TransitionXyBounds(c);
            }
        }
        Ok(())
    }
    pub fn qualify_remaining_transition_xy_material_bounds<'x>(
        &'x self,
        audit: &mut ShellContactAudit<'x>,
        limits: ContactAuditLimits,
        region: &ProfileProjectedRegion<'_, '_, '_, '_, '_>,
    ) -> Result<()> {
        self.resume_contact_audit(audit, limits, 0)?;
        let candidate = |p: &AuditedContactPair<'_>| {
            matches!(p.proof, PairContactProof::Pending)
                && matches!(
                    p.support.faces.map(|i| self.faces[i].kind()),
                    [ShellFaceKind::Original(_), ShellFaceKind::Transition]
                        | [ShellFaceKind::Transition, ShellFaceKind::Original(_)]
                )
        };
        let mut remaining = audit.pairs.iter().filter(|p| candidate(p)).count();
        for pair in &mut audit.pairs {
            if !candidate(pair) {
                continue;
            }
            let spans = (limits.spans - audit.spans) / remaining;
            let work = (limits.exact_work - audit.exact_work) / remaining as u64;
            remaining -= 1;
            if spans == 0 || work == 0 {
                continue;
            }
            let report = self.qualify_transition_xy_material_bounds_contact(
                pair.support.faces,
                region,
                spans,
                work,
            )?;
            audit.spans += report.spans;
            audit.exact_work += report.exact_work;
            pair.reason = report.reason;
            if let Some(c) = report.certificate {
                pair.proof = PairContactProof::TransitionXyBounds(c);
            }
        }
        Ok(())
    }
    pub fn qualify_remaining_transition_xy_hulls<'x>(
        &'x self,
        audit: &mut ShellContactAudit<'x>,
        limits: ContactAuditLimits,
        region: &ProfileProjectedRegion<'_, '_, '_, '_, '_>,
    ) -> Result<()> {
        self.resume_contact_audit(audit, limits, 0)?;
        let candidate = |p: &AuditedContactPair<'_>| {
            matches!(p.proof, PairContactProof::Pending)
                && matches!(
                    p.support.faces.map(|i| self.faces[i].kind()),
                    [ShellFaceKind::Original(_), ShellFaceKind::Transition]
                        | [ShellFaceKind::Transition, ShellFaceKind::Original(_)]
                )
        };
        let mut remaining = audit.pairs.iter().filter(|p| candidate(p)).count();
        for pair in &mut audit.pairs {
            if !candidate(pair) {
                continue;
            }
            let work = (limits.exact_work - audit.exact_work) / remaining as u64;
            remaining -= 1;
            if work == 0 {
                continue;
            }
            let report =
                self.qualify_transition_xy_hull_contact(pair.support.faces, region, work)?;
            audit.exact_work += report.exact_work;
            pair.reason = report.reason;
            if let Some(c) = report.certificate {
                pair.proof = PairContactProof::TransitionXyHull(c);
            }
        }
        Ok(())
    }
    fn resume_contact_audit_impl<'x>(
        &'x self,
        out: &mut ShellContactAudit<'x>,
        limits: ContactAuditLimits,
        additional_cells_per_pair: usize,
        include_changed: bool,
        only_transition: bool,
    ) -> Result<()> {
        validate_resume(
            self.faces.as_slice(),
            out,
            limits,
            additional_cells_per_pair,
        )?;
        let candidate =
            |pair: &AuditedContactPair<'_>| {
                matches!(pair.proof, PairContactProof::Pending)
                    && if only_transition {
                        matches!(
                            pair.support.faces.map(|i| self.faces[i].kind()),
                            [ShellFaceKind::Original(_), ShellFaceKind::Transition]
                                | [ShellFaceKind::Transition, ShellFaceKind::Original(_)]
                        )
                    } else {
                        include_changed
                            || pair.support.faces.iter().all(|&i| {
                                matches!(self.faces[i].kind(), ShellFaceKind::Original(_))
                            })
                    }
            };
        let mut remaining = out.pairs.iter().filter(|p| candidate(p)).count();
        for pair in &mut out.pairs {
            if !candidate(pair) {
                continue;
            }
            let cells = ((limits.cells - out.cells) / remaining).min(additional_cells_per_pair);
            let spans = (limits.spans - out.spans) / remaining;
            let domains = ((limits.domain_cells - out.domain_cells) / remaining)
                .min(limits.domain_cells_per_pair);
            remaining -= 1;
            let old = (
                pair.support.inspection.cells,
                pair.support.inspection.spans,
                pair.support.inspection.domain_cells,
            );
            refine(
                &mut pair.support.inspection,
                cells,
                spans,
                domains,
                |product, c, s, d| {
                    self.inspect_trimmed_pair(
                        pair.support.faces,
                        product,
                        limits.tolerance_uv,
                        c,
                        limits.controls_per_enclosure,
                        s,
                        d,
                    )
                },
            )?;
            out.cells += pair.support.inspection.cells - old.0;
            out.spans += pair.support.inspection.spans - old.1;
            out.domain_cells += pair.support.inspection.domain_cells - old.2;
            if pair.support.inspection.complete_excluded() {
                pair.proof = PairContactProof::Excluded;
                pair.reason = "expression-contact-refined-products-excluded";
            }
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn continuation_rejects_foreign_owners_and_insufficient_cumulative_limits() {
        let faces = [ShellFace {
            kind: ShellFaceKind::Original(0),
            loops: vec![],
        }];
        let foreign = [ShellFace {
            kind: ShellFaceKind::Original(0),
            loops: vec![],
        }];
        let audit = ShellContactAudit {
            owners: &faces,
            pairs: vec![],
            cells: 3,
            spans: 5,
            domain_cells: 7,
            exact_work: 11,
            driver_cells: 13,
            linear_cells: 17,
        };
        let limits = ContactAuditLimits {
            cells: 3,
            cells_per_pair: 0,
            controls_per_enclosure: 0,
            spans: 5,
            domain_cells: 7,
            domain_cells_per_pair: 0,
            exact_work: 11,
            driver_cells: 13,
            tolerance_uv: 1e-8,
        };
        let before = audit.diagnostics();
        assert!(validate_resume(&faces, &audit, limits, 0).is_ok());
        assert!(validate_resume(&foreign, &audit, limits, 0).is_err());
        for smaller in [
            ContactAuditLimits { cells: 2, ..limits },
            ContactAuditLimits { spans: 4, ..limits },
            ContactAuditLimits {
                domain_cells: 6,
                ..limits
            },
            ContactAuditLimits {
                exact_work: 10,
                ..limits
            },
            ContactAuditLimits {
                driver_cells: 12,
                ..limits
            },
        ] {
            assert!(validate_resume(&faces, &audit, smaller, 0).is_err());
        }
        for invalid in [
            ContactAuditLimits {
                cells: 100001,
                ..limits
            },
            ContactAuditLimits {
                controls_per_enclosure: 1000001,
                ..limits
            },
            ContactAuditLimits {
                tolerance_uv: f64::NAN,
                ..limits
            },
            ContactAuditLimits {
                tolerance_uv: 0.,
                ..limits
            },
        ] {
            assert!(validate_resume(&faces, &audit, invalid, 0).is_err());
        }
        assert!(validate_resume(&faces, &audit, limits, 100001).is_err());
        assert_eq!(audit.diagnostics(), before);
    }
    fn volume(p: SupportProduct) -> f64 {
        p.iter().flatten().map(|r| r[1] - r[0]).product()
    }
    #[test]
    fn resumed_support_queries_keep_a_real_plane_intersection() {
        let inspect = |domain, cells, spans, domains| {
            super::super::super::contacts::inspect_regions(
                domain,
                cells,
                spans,
                domains,
                |side, r, budget| {
                    if budget == 0 {
                        return Ok((None, 0));
                    }
                    Ok((
                        Some(if side == 0 {
                            [r[0], r[1], [0.; 2]]
                        } else {
                            [r[0], [0.5; 2], r[1]]
                        }),
                        1,
                    ))
                },
                |_, _, _| Ok((false, 0)),
            )
        };
        let mut report = inspect([[[0., 1.]; 2]; 2], 7, 32, 0).unwrap();
        let witness = [[0.25, 0.5], [0.25, 0.]];
        for _ in 0..4 {
            assert!(!report.complete_excluded());
            assert!(report.pending.iter().any(|p| (0..2).all(|side| {
                (0..2).all(|axis| {
                    p[side][axis][0] <= witness[side][axis]
                        && witness[side][axis] <= p[side][axis][1]
                })
            })));
            assert_eq!(
                report
                    .pending
                    .iter()
                    .chain(&report.separated)
                    .chain(&report.outside)
                    .copied()
                    .map(volume)
                    .sum::<f64>(),
                1.
            );
            refine(&mut report, 64, 256, 0, inspect).unwrap();
        }
        assert!(!report.complete_excluded());
    }
    #[test]
    fn pending_refinement_preserves_cover_and_failed_work_is_transactional() {
        let whole = [[[0., 1.]; 2]; 2];
        let mut left = whole;
        left[0][0][1] = 0.5;
        let mut right = whole;
        right[0][0][0] = 0.5;
        let mut report = SupportPairReport {
            separated: vec![left],
            outside: vec![],
            pending: vec![right],
            cells: 1,
            spans: 1,
            domain_cells: 0,
        };
        refine(&mut report, 0, 1, 1, |_, _, _, _| {
            panic!("no budget must not inspect")
        })
        .unwrap();
        assert_eq!(report.pending, vec![right]);
        let failed = refine(&mut report, 2, 2, 2, |_, _, _, _| {
            Err(Error::new("TEST", "cancelled"))
        });
        assert!(failed.is_err());
        assert_eq!(report.pending, vec![right]);
        assert_eq!(report.cells, 1);
        let failed = refine(&mut report, 1, 1, 1, |p, _, _, _| {
            Ok(SupportPairReport {
                separated: vec![p],
                outside: vec![],
                pending: vec![],
                cells: 2,
                spans: 1,
                domain_cells: 0,
            })
        });
        assert!(failed.is_err());
        assert_eq!(report.pending, vec![right]);
        assert_eq!(report.separated, vec![left]);
        assert_eq!((report.cells, report.spans), (1, 1));
        refine(&mut report, 3, 3, 3, |p, c, s, d| {
            assert_eq!(p, right);
            assert_eq!((c, s, d), (3, 3, 3));
            let mut a = p;
            let mut b = p;
            a[1][1][1] = 0.5;
            b[1][1][0] = 0.5;
            Ok(SupportPairReport {
                separated: vec![a],
                outside: vec![],
                pending: vec![b],
                cells: 1,
                spans: 1,
                domain_cells: 0,
            })
        })
        .unwrap();
        assert_eq!(report.cells, 2);
        assert_eq!(report.spans, 2);
        assert_eq!(
            report
                .pending
                .iter()
                .chain(&report.separated)
                .copied()
                .map(volume)
                .sum::<f64>(),
            1.
        );
        assert!(!report.complete_excluded());
        refine(&mut report, 1, 1, 1, |p, _, _, _| {
            Ok(SupportPairReport {
                separated: vec![p],
                outside: vec![],
                pending: vec![],
                cells: 1,
                spans: 1,
                domain_cells: 0,
            })
        })
        .unwrap();
        assert!(report.complete_excluded());
        assert_eq!(
            report.separated.iter().copied().map(volume).sum::<f64>(),
            1.
        );
    }
}
