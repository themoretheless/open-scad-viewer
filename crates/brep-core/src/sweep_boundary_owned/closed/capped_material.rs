//! Original open traversal with two complete, materially bound filled caps.
//! Normalized-map correspondence is mandatory: independent image clocks do
//! not identify the boundary of the interpolated reference carrier.
use super::*;
use nurbs_core::progressive_sweep::{
    OriginalClassifiedVolume, OriginalClassifiedVolumeAttempt, OriginalClassifiedVolumeLimits,
    OriginalClassifiedVolumeReport, OriginalProfileClass, OriginalSmoothVolumeLimits,
};

#[derive(Clone, Copy)]
pub struct OriginalClassifiedCappedMaterialLimits {
    /// One ledger for both source bindings, common UV region, and volume proof.
    pub volume: OriginalClassifiedVolumeLimits,
    /// One aggregate material audit budget, shared by both caps and all holes.
    pub material: MaterialLimits,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct OriginalClassifiedCappedMaterialWork {
    pub cells: usize,
    pub exact_work: u64,
    pub products: usize,
    pub material_pairs: usize,
    pub material_cells: usize,
    pub material_domain_cells: usize,
}
pub struct OriginalClassifiedCappedMaterialRefusal {
    pub materials: Vec<MaterialReport>,
    pub volume: Option<Box<OriginalClassifiedVolumeReport>>,
    pub work: OriginalClassifiedCappedMaterialWork,
    pub reason: Option<&'static str>,
}
/// Immutable original material domain times an interval, globally embedded by
/// the exact source map. This owner supplies no retained Solid/error guarantee.
/// Its two cap carriers are reference sources, before physical endpoint pose.
pub struct OriginalClassifiedCappedMaterial<'s, 'a> {
    caps: [&'s LoftCap; 2],
    bindings: [Vec<MaterialSpanBinding>; 2],
    materials: [MaterialReport; 2],
    volume: Box<OriginalClassifiedVolume<'s, 'a>>,
    work: OriginalClassifiedCappedMaterialWork,
}
impl<'s, 'a> OriginalClassifiedCappedMaterial<'s, 'a> {
    pub fn caps(&self) -> [&'s LoftCap; 2] {
        self.caps
    }
    pub fn bindings(&self) -> [&[MaterialSpanBinding]; 2] {
        [&self.bindings[0], &self.bindings[1]]
    }
    pub fn material_reports(&self) -> &[MaterialReport; 2] {
        &self.materials
    }
    pub fn volume(&self) -> &OriginalClassifiedVolume<'s, 'a> {
        &self.volume
    }
    pub fn work(&self) -> OriginalClassifiedCappedMaterialWork {
        self.work
    }
    pub fn boundary_classes(&self) -> &[OriginalProfileClass] {
        self.volume.classes()
    }
    /// No cap/wall geometric tangency is inferred from volume-chart Ck.
    pub fn cap_wall_continuity(&self) -> &'static str {
        "C0"
    }
}
pub enum OriginalClassifiedCappedMaterialAttempt<'s, 'a> {
    Certified(Box<OriginalClassifiedCappedMaterial<'s, 'a>>),
    Refused(Box<OriginalClassifiedCappedMaterialRefusal>),
}
fn refused<'s, 'a>(
    materials: Vec<MaterialReport>,
    volume: Option<Box<OriginalClassifiedVolumeReport>>,
    work: OriginalClassifiedCappedMaterialWork,
    reason: Option<&'static str>,
) -> OriginalClassifiedCappedMaterialAttempt<'s, 'a> {
    OriginalClassifiedCappedMaterialAttempt::Refused(Box::new(
        OriginalClassifiedCappedMaterialRefusal {
            materials,
            volume,
            work,
            reason,
        },
    ))
}
fn charge(work: &mut OriginalClassifiedCappedMaterialWork, report: &ClosedReport) {
    work.cells += report.source_cells;
    work.exact_work += report.ownership_exact_work;
    work.products += report.source_products;
    if let Some(trim) = &report.material.trim {
        work.material_pairs += trim.pairs;
        work.material_cells += trim.cells;
        work.material_domain_cells += trim.domain_cells;
    }
}
/// Fresh native proof of the complete original capped material volume.
/// Both cap traces equal the same original profiles under normalized clocks.
/// Their normalized UV regions coincide, so on every boundary point C0=C1
/// and the interpolated carrier is exactly the source wall for all t.
/// The independent global filled embedding transports the valid material
/// topology, including every hole. No caller report authorizes any stage.
pub fn certify_original_classified_capped_material<'s, 'a>(
    sweep: &'s MultiSweep<'a>,
    loop_sizes: &[usize],
    caps: [&'s LoftCap; 2],
    classes: &[OriginalProfileClass],
    projection: [[f64; 3]; 3],
    tolerance_uv: f64,
    limits: OriginalClassifiedCappedMaterialLimits,
) -> Result<OriginalClassifiedCappedMaterialAttempt<'s, 'a>> {
    let l = limits.volume.charts;
    if !matches!(l.order, 1 | 2)
        || l.cells > 1000000
        || l.exact_work > 1000000
        || l.products > 1000000
        || limits.volume.steps > 1000000
        || classes.is_empty()
        || classes.len() > 1000000
    {
        return Err(fail("Invalid original classified capped material limits"));
    }
    let mut work = OriginalClassifiedCappedMaterialWork::default();
    let mut reports = Vec::new();
    let mut bindings = Vec::new();
    for cap in caps {
        let prepared = prepare_material_with_binding(
            sweep,
            loop_sizes,
            cap,
            tolerance_uv,
            OriginalClosedLimits {
                source_cells: (l.cells - work.cells).min(100000),
                source_products: l.products - work.products,
                source_exact_work: l.exact_work - work.exact_work,
                material: MaterialLimits {
                    pairs: limits.material.pairs - work.material_pairs,
                    cells: limits.material.cells - work.material_cells,
                    domain_cells: limits.material.domain_cells - work.material_domain_cells,
                },
                ideal: IdealLimits {
                    cells: 0,
                    exact_work: 0,
                    projection,
                },
                wall_faces: 0,
            },
            MaterialBinding::NormalizedMap,
            MaterialTraversal::Open,
        )?;
        let source = match prepared {
            PreparedMaterialAttempt::Ready(p) => *p,
            PreparedMaterialAttempt::Refused(r) => {
                charge(&mut work, &r);
                let reason = r.reason;
                reports.push(r.material);
                return Ok(refused(reports, None, work, reason));
            }
        };
        charge(&mut work, &source.report);
        // Positive rational hull of EVERY original decomposed UV trace stays
        // in the carrier rectangle. The valid bounded material lies there too.
        let domain = [
            [
                cap.surface.knots_u[cap.surface.degree_u],
                cap.surface.knots_u[cap.surface.control_points.len()],
            ],
            [
                cap.surface.knots_v[cap.surface.degree_v],
                cap.surface.knots_v[cap.surface.control_points[0].len()],
            ],
        ];
        reports.push(source.report.material);
        for p in source
            .material
            .trims
            .iter()
            .flatten()
            .flat_map(|c| &c.control_points)
        {
            if work.cells == l.cells {
                return Ok(refused(
                    reports,
                    None,
                    work,
                    Some("original-capped-material-domain-work-limit"),
                ));
            }
            work.cells += 1;
            if (0..2).any(|k| p[k] < domain[k][0] || p[k] > domain[k][1]) {
                return Ok(refused(
                    reports,
                    None,
                    work,
                    Some("original-capped-material-domain-unproved"),
                ));
            }
        }
        bindings.push(source.bindings);
    }
    if !super::super::common_trim_region::certify(caps, &mut work.exact_work, l.exact_work)? {
        return Ok(refused(
            reports,
            None,
            work,
            Some("original-capped-material-common-region-unproved"),
        ));
    }
    let volume = sweep.certify_original_classified_volume_atlas(
        [&caps[0].surface, &caps[1].surface],
        loop_sizes,
        classes,
        projection,
        OriginalClassifiedVolumeLimits {
            charts: OriginalSmoothVolumeLimits {
                cells: l.cells - work.cells,
                exact_work: l.exact_work - work.exact_work,
                products: l.products - work.products,
                ..l
            },
            steps: limits.volume.steps,
        },
    )?;
    let volume = match volume {
        OriginalClassifiedVolumeAttempt::Certified(v) => v,
        OriginalClassifiedVolumeAttempt::Refused(v) => {
            work.cells += v.cells;
            work.exact_work += v.exact_work;
            work.products += v.products;
            let reason = v.reason;
            return Ok(refused(reports, Some(v), work, reason));
        }
    };
    work.cells += volume.report().cells;
    work.exact_work += volume.report().exact_work;
    work.products += volume.report().products;
    // Both stages completed; construction is private and cannot accept reports.
    let [first, second] = <[MaterialReport; 2]>::try_from(reports)
        .map_err(|_| fail("Incomplete capped material ownership"))?;
    let [start, end] = <[Vec<MaterialSpanBinding>; 2]>::try_from(bindings)
        .map_err(|_| fail("Incomplete capped boundary binding"))?;
    Ok(OriginalClassifiedCappedMaterialAttempt::Certified(
        Box::new(OriginalClassifiedCappedMaterial {
            caps,
            bindings: [start, end],
            materials: [first, second],
            volume,
            work,
        }),
    ))
}
#[cfg(test)]
mod tests;
