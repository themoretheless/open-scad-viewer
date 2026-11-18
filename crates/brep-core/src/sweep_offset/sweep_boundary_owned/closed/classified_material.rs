//! Original closed material boundary: fresh trim/trace binding + typed volume.
//! No retained mesh, sampled image or caller report supplies proof authority.
use super::*;
mod bindings;
pub use bindings::{MaterialSpanBinding, MaterialSpanBindingKind};
use nurbs_core::progressive_sweep::{
    OriginalClassifiedVolume, OriginalClassifiedVolumeAttempt, OriginalClassifiedVolumeLimits,
    OriginalClassifiedVolumeReport, OriginalProfileClass, OriginalSmoothVolumeLimits,
};
#[derive(Clone, Copy)]
pub struct OriginalClassifiedClosedMaterialLimits {
    /// Source extraction/binding and the complete classified volume share this
    /// single cells/exact/products budget. Steps count native profile visits.
    pub volume: OriginalClassifiedVolumeLimits,
    /// Independent bounded full UV material audit, including every hole.
    pub material: MaterialLimits,
}
#[derive(Clone, Copy, Debug)]
pub struct OriginalClassifiedClosedMaterialWork {
    pub cells: usize,
    pub exact_work: u64,
    pub products: usize,
}
pub struct OriginalClassifiedClosedMaterialRefusal {
    pub material: MaterialReport,
    pub volume: Option<Box<OriginalClassifiedVolumeReport>>,
    pub work: OriginalClassifiedClosedMaterialWork,
    pub reason: Option<&'static str>,
}
/// Exact original material domain embedded by one immutable closed source map.
/// Each declared profile join now has material-boundary authority; C0 entries
/// remain explicit sharp joins. This type issues no retained Solid guarantee.
pub struct OriginalClassifiedClosedMaterial<'s, 'a> {
    material: &'s LoftCap,
    bindings: Vec<MaterialSpanBinding>,
    line_image_spans: usize,
    projective_image_spans: usize,
    volume: Box<OriginalClassifiedVolume<'s, 'a>>,
    material_report: MaterialReport,
    work: OriginalClassifiedClosedMaterialWork,
}
impl<'s, 'a> OriginalClassifiedClosedMaterial<'s, 'a> {
    /// Complete, ordered ownership of every decomposed boundary span. Records
    /// index the immutable original material and sweep borrowed by this owner.
    /// Loop zero is the outer contour; remaining loops are material holes.
    pub fn bindings(&self) -> &[MaterialSpanBinding] {
        &self.bindings
    }
    /// Exact oriented line-image bindings, with no normalized-map authority.
    pub fn line_image_spans(&self) -> usize {
        self.line_image_spans
    }
    /// Full coefficient identity under a positive derived projective clock.
    /// This does not grant normalized map correspondence.
    pub fn projective_image_spans(&self) -> usize {
        self.projective_image_spans
    }
    pub fn material(&self) -> &'s LoftCap {
        self.material
    }
    pub fn volume(&self) -> &OriginalClassifiedVolume<'s, 'a> {
        &self.volume
    }
    pub fn material_report(&self) -> &MaterialReport {
        &self.material_report
    }
    pub fn work(&self) -> OriginalClassifiedClosedMaterialWork {
        self.work
    }
    /// Complete regular original material wall joins, including every hole.
    pub fn boundary_classes(&self) -> &[OriginalProfileClass] {
        self.volume.classes()
    }
    /// Ck across the closed traversal seam, away from declared sharp joins.
    pub fn closed_seam_order(&self) -> usize {
        self.volume.report().charts.order
    }
}
pub enum OriginalClassifiedClosedMaterialAttempt<'s, 'a> {
    Certified(Box<OriginalClassifiedClosedMaterial<'s, 'a>>),
    Refused(Box<OriginalClassifiedClosedMaterialRefusal>),
}
/// Fresh native binding of ALL original profiles to ALL original UV contours,
/// valid complete outer/hole material region, global filled embedding and exact
/// declared G1/G2 jets. Global embedding transports the valid material topology;
/// common smooth motion transports profile jets. Reports cannot rebind sources.
pub fn certify_original_classified_closed_material<'s, 'a>(
    sweep: &'s MultiSweep<'a>,
    loop_sizes: &[usize],
    material: &'s LoftCap,
    classes: &[OriginalProfileClass],
    projection: [[f64; 3]; 3],
    tolerance_uv: f64,
    limits: OriginalClassifiedClosedMaterialLimits,
) -> Result<OriginalClassifiedClosedMaterialAttempt<'s, 'a>> {
    let l = limits.volume.charts;
    if !matches!(l.order, 1 | 2)
        || l.cells > 1000000
        || l.exact_work > 1000000
        || l.products > 1000000
        || limits.volume.steps > 1000000
        || classes.is_empty()
        || classes.len() > 1000000
    {
        return Err(fail("Invalid original classified closed material limits"));
    }
    let prepared = prepare_classified_closed_material(
        sweep,
        loop_sizes,
        material,
        tolerance_uv,
        OriginalClosedLimits {
            source_cells: l.cells.min(100000),
            source_products: l.products,
            source_exact_work: l.exact_work,
            material: limits.material,
            ideal: IdealLimits {
                cells: 0,
                exact_work: 0,
                projection,
            },
            wall_faces: 0,
        },
    )?;
    let source = match prepared {
        PreparedMaterialAttempt::Ready(p) => *p,
        PreparedMaterialAttempt::Refused(r) => {
            return Ok(OriginalClassifiedClosedMaterialAttempt::Refused(Box::new(
                OriginalClassifiedClosedMaterialRefusal {
                    work: OriginalClassifiedClosedMaterialWork {
                        cells: r.source_cells,
                        exact_work: r.ownership_exact_work,
                        products: r.source_products,
                    },
                    material: r.material,
                    volume: None,
                    reason: r.reason,
                },
            )));
        }
    };
    let PreparedMaterial {
        bindings,
        report: source,
        line_image_spans,
        projective_image_spans,
        material: bound,
        ..
    } = source;
    let mut prefix = OriginalClassifiedClosedMaterialWork {
        cells: source.source_cells,
        exact_work: source.ownership_exact_work,
        products: source.source_products,
    };
    // Positive rational Bezier hulls keep the complete UV boundary inside the
    // original atlas rectangle. A simple bounded region then lies inside that
    // convex rectangle too. Image binding does not prove this containment.
    let domain = [
        [
            material.surface.knots_u[material.surface.degree_u],
            material.surface.knots_u[material.surface.control_points.len()],
        ],
        [
            material.surface.knots_v[material.surface.degree_v],
            material.surface.knots_v[material.surface.control_points[0].len()],
        ],
    ];
    for p in bound.trims.iter().flatten().flat_map(|c| &c.control_points) {
        let exhausted = prefix.cells == l.cells;
        if !exhausted {
            prefix.cells += 1;
        }
        if exhausted || (0..2).any(|k| p[k] < domain[k][0] || p[k] > domain[k][1]) {
            return Ok(OriginalClassifiedClosedMaterialAttempt::Refused(Box::new(
                OriginalClassifiedClosedMaterialRefusal {
                    material: source.material,
                    volume: None,
                    work: prefix,
                    reason: Some(if exhausted {
                        "original-material-carrier-domain-work-limit"
                    } else {
                        "original-material-carrier-domain-unproved"
                    }),
                },
            )));
        }
    }
    let volume = sweep.certify_original_classified_volume_atlas(
        [&material.surface, &material.surface],
        loop_sizes,
        classes,
        projection,
        OriginalClassifiedVolumeLimits {
            charts: OriginalSmoothVolumeLimits {
                cells: l.cells - prefix.cells,
                exact_work: l.exact_work - prefix.exact_work,
                products: l.products - prefix.products,
                ..l
            },
            steps: limits.volume.steps,
        },
    )?;
    let volume = match volume {
        OriginalClassifiedVolumeAttempt::Certified(v) => v,
        OriginalClassifiedVolumeAttempt::Refused(v) => {
            let work = OriginalClassifiedClosedMaterialWork {
                cells: prefix.cells + v.cells,
                exact_work: prefix.exact_work + v.exact_work,
                products: prefix.products + v.products,
            };
            let reason = v.reason;
            return Ok(OriginalClassifiedClosedMaterialAttempt::Refused(Box::new(
                OriginalClassifiedClosedMaterialRefusal {
                    material: source.material,
                    volume: Some(v),
                    work,
                    reason,
                },
            )));
        }
    };
    let work = OriginalClassifiedClosedMaterialWork {
        cells: prefix.cells + volume.report().cells,
        exact_work: prefix.exact_work + volume.report().exact_work,
        products: prefix.products + volume.report().products,
    };
    Ok(OriginalClassifiedClosedMaterialAttempt::Certified(
        Box::new(OriginalClassifiedClosedMaterial {
            bindings,
            material,
            line_image_spans,
            projective_image_spans,
            volume,
            material_report: source.material,
            work,
        }),
    ))
}
#[cfg(test)]
mod tests;
