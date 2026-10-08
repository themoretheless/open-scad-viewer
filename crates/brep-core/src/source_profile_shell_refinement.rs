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
        if additional_cells_per_pair > 100000 {
            return Err(Error::new(
                "BREP_PROFILE_CONTACTS",
                "Bound additional pair refinement",
            ));
        }
        let mut out = self.inspect_contacts_with_profile_region(limits, region)?;
        let candidate = |pair: &AuditedContactPair<'_>| {
            matches!(pair.proof, PairContactProof::Pending)
                && pair
                    .support
                    .faces
                    .iter()
                    .all(|&i| matches!(self.faces[i].kind(), ShellFaceKind::Original(_)))
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
        Ok(out)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
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
