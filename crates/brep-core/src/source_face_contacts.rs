//! Different-face contact search over immutable qualified source UV regions.
//! Shared topology does not prune searches or authorize absence of extra contact.
use crate::{source_contour_proposal::SourceRegion, source_contour_winding};
use nurbs_core::{
    Error, Result,
    surface_contact_search::{self, DomainClassification, DomainClassifier},
};
struct Domain<'a> {
    region: &'a SourceRegion,
    tolerance_uv: f64,
}
impl DomainClassifier for Domain<'_> {
    fn classify_domain(
        &self,
        rectangle: [[f64; 2]; 2],
        max_cells: usize,
    ) -> Result<DomainClassification> {
        let r = source_contour_winding::classify(
            self.region.loops(),
            rectangle,
            self.tolerance_uv,
            max_cells,
        )?;
        Ok(DomainClassification {
            location: r.location,
            cells: r.cells,
        })
    }
}
pub fn search(
    regions: [&SourceRegion; 2],
    tolerance_uv: f64,
    max_cells: usize,
    max_domain_cells: usize,
) -> Result<surface_contact_search::Report> {
    if !tolerance_uv.is_finite() || tolerance_uv <= 0. {
        return Err(Error::new(
            "BREP_SOURCE_CONTACT",
            "Choose a positive original UV tolerance",
        ));
    }
    let domains = regions.map(|region| Domain {
        region,
        tolerance_uv,
    });
    surface_contact_search::search_domains(
        regions[0].loops()[0][0].surface(),
        regions[1].loops()[0][0].surface(),
        [&domains[0], &domains[1]],
        max_cells,
        max_domain_cells,
    )
}
pub struct PairReport {
    pub faces: [usize; 2],
    pub result: Option<surface_contact_search::Report>,
    pub allowed: Option<crate::source_allowed_contact::Certificate>,
    pub fiber: Option<crate::source_fiber_contact::Certificate>,
    pub interior_fiber: Option<crate::source_interior_contact::Certificate>,
    pub paired_fiber: Option<crate::source_paired_fiber_contact::Certificate>,
    pub vertex_contact: Option<crate::source_vertex_contact::Certificate>,
    pub ruled_projection: Option<crate::source_ruled_projection_contact::Certificate>,
    pub pole_planar: Option<crate::source_pole_planar_contact::Certificate>,
    pub pole_paired: Option<crate::source_pole_paired_contact::Certificate>,
    pub disjoint_hull: Option<crate::source_hull_separation::Certificate>,
}
pub struct ShellReport {
    pub pairs: Vec<PairReport>,
    pub total_pairs: usize,
    pub next_pair: Option<[usize; 2]>,
    pub cells: usize,
    pub domain_cells: usize,
    pub all_pairs_absence_proven: bool,
    pub all_pairs_qualified: bool,
    pub exact_work: u64,
    pub spans: usize,
    pub driver_cells: usize,
    pub linear_cells: usize,
}
/// Adjacent pairs are included: topology sharing does not exclude extra contact.
pub fn inspect_shell(
    shell: &crate::source_shell_incidence::Shell,
    tolerance_uv: f64,
    limits: crate::face_contacts::Limits,
) -> Result<ShellReport> {
    inspect_impl(shell, tolerance_uv, limits, None)
}
/// Native recomputation of allowed contacts; no caller certificates are accepted.
pub fn inspect_shell_with_allowed(
    shell: &crate::source_shell_incidence::Shell,
    tolerance_uv: f64,
    limits: crate::face_contacts::Limits,
    max_exact_work: u64,
    max_spans: usize,
) -> Result<ShellReport> {
    if !(1..=100_000_000).contains(&max_exact_work) || !(1..=100000).contains(&max_spans) {
        return Err(Error::new(
            "BREP_SOURCE_CONTACT",
            "Bound exact allowed-contact work",
        ));
    }
    inspect_impl(
        shell,
        tolerance_uv,
        limits,
        Some((max_exact_work, max_spans, 0, 0)),
    )
}
/// Recompute natural and supported interior fiber ownership before contact search.
/// Shared budgets cover both face orientations and all fallback proofs.
pub fn inspect_shell_with_boundary_fibers(
    shell: &crate::source_shell_incidence::Shell,
    tolerance_uv: f64,
    limits: crate::face_contacts::Limits,
    max_exact_work: u64,
    max_spans: usize,
    max_driver_cells: usize,
) -> Result<ShellReport> {
    if !(1..=100_000_000).contains(&max_exact_work)
        || !(1..=100000).contains(&max_spans)
        || !(1..=100000).contains(&max_driver_cells)
    {
        return Err(Error::new(
            "BREP_SOURCE_CONTACT",
            "Bound source fiber qualification work",
        ));
    }
    inspect_impl(
        shell,
        tolerance_uv,
        limits,
        Some((max_exact_work, max_spans, max_driver_cells, 0)),
    )
}
/// Source fiber ownership with a separate budget for oblique chart proofs.
pub fn inspect_shell_with_boundary_fibers_and_chart_work(
    shell: &crate::source_shell_incidence::Shell,
    tolerance_uv: f64,
    limits: crate::face_contacts::Limits,
    max_exact_work: u64,
    max_spans: usize,
    max_driver_cells: usize,
    max_linear_cells: usize,
) -> Result<ShellReport> {
    if !(1..=100_000_000).contains(&max_exact_work)
        || !(1..=100000).contains(&max_spans)
        || !(1..=100000).contains(&max_driver_cells)
        || max_linear_cells > 100000
    {
        return Err(Error::new(
            "BREP_SOURCE_CONTACT",
            "Bound source fiber and chart work",
        ));
    }
    inspect_impl(
        shell,
        tolerance_uv,
        limits,
        Some((
            max_exact_work,
            max_spans,
            max_driver_cells,
            max_linear_cells,
        )),
    )
}
fn inspect_impl(
    shell: &crate::source_shell_incidence::Shell,
    tolerance_uv: f64,
    limits: crate::face_contacts::Limits,
    allowed_budget: Option<(u64, usize, usize, usize)>,
) -> Result<ShellReport> {
    if !(1..=100000).contains(&limits.pairs)
        || !(1..=1000000).contains(&limits.cells)
        || !(1..=8000000).contains(&limits.domain_cells)
        || !(1..=100000).contains(&limits.cells_per_pair)
        || !(1..=1000000).contains(&limits.domain_cells_per_pair)
        || !tolerance_uv.is_finite()
        || tolerance_uv <= 0.
    {
        return Err(Error::new(
            "BREP_SOURCE_CONTACT",
            "Choose bounded source pair and domain work",
        ));
    }
    let regions = shell.regions().ok_or_else(|| {
        Error::new(
            "BREP_SOURCE_CONTACT",
            "Source pair search requires qualified material regions",
        )
    })?;
    let n = regions.len();
    let mut out = ShellReport {
        pairs: Vec::new(),
        total_pairs: n * (n - 1) / 2,
        next_pair: None,
        cells: 0,
        domain_cells: 0,
        all_pairs_absence_proven: true,
        all_pairs_qualified: true,
        exact_work: 0,
        spans: 0,
        driver_cells: 0,
        linear_cells: 0,
    };
    for a in 0..n {
        for b in a + 1..n {
            if out.pairs.len() == limits.pairs
                || out.cells == limits.cells
                || out.domain_cells == limits.domain_cells
            {
                out.next_pair = Some([a, b]);
                out.all_pairs_absence_proven = false;
                out.all_pairs_qualified = false;
                return Ok(out);
            }
            if let Some((work, spans, driver, linear)) = allowed_budget {
                if out.exact_work < work {
                    let proof = crate::source_hull_separation::certify_shell(
                        shell, [a,b],
                        work - out.exact_work,
                    )?;
                    out.exact_work += proof.exact_work;
                    if let Some(certificate) = proof.certificate {
                        out.pairs.push(PairReport {
                            faces: [a, b],
                            result: None,
                            allowed: None,
                            fiber: None,
                            interior_fiber: None,
                            paired_fiber: None,
                            vertex_contact: None,
                            pole_paired: None,
                            ruled_projection: None, pole_planar: None,
                            disjoint_hull: Some(certificate),
                        });
                        continue;
                    }
                }

                if out.exact_work < work && out.spans < spans {
                    let proof = crate::source_allowed_contact::certify(
                        shell,
                        [a, b],
                        work - out.exact_work,
                        spans - out.spans,
                    )?;
                    out.exact_work += proof.exact_work;
                    out.spans += proof.spans;
                    if let Some(certificate) = proof.certificate {
                        out.all_pairs_absence_proven = false;
                        out.pairs.push(PairReport {
                            faces: [a, b],
                            result: None,
                            allowed: Some(certificate),
                            fiber: None,
                            pole_paired: None,
                            ruled_projection: None, pole_planar: None,
                            disjoint_hull: None,
                            vertex_contact: None,
                            paired_fiber: None,
                            interior_fiber: None,
                        });
                        continue;
                    }
                }

                // Recompute the natural plane image before expensive chart guesses.
                if out.exact_work < work && out.driver_cells < driver {
                    let proof = crate::source_pole_planar_contact::certify(
                        shell, [a, b], work - out.exact_work, driver - out.driver_cells,
                    )?;
                    out.exact_work += proof.exact_work;
                    out.driver_cells += proof.driver_cells;
                    if let Some(certificate) = proof.certificate {
                        out.all_pairs_absence_proven = false;
                        out.pairs.push(PairReport {
                            faces: [a, b], result: None, allowed: None, fiber: None,
                            interior_fiber: None, paired_fiber: None, vertex_contact: None,
                            pole_paired: None, ruled_projection: None, pole_planar: Some(certificate), disjoint_hull: None,
                        });
                        continue;
                    }
                }
                if out.exact_work < work && out.driver_cells < driver {
                    let proof = crate::source_ruled_projection_contact::certify(
                        shell,[a,b],tolerance_uv,work-out.exact_work,driver-out.driver_cells,
                    )?;
                    out.exact_work+=proof.exact_work;out.driver_cells+=proof.driver_cells;
                    if let Some(certificate)=proof.certificate {
                        out.all_pairs_absence_proven=false;
                        out.pairs.push(PairReport {
                            faces:[a,b],result:None,allowed:None,fiber:None,
                            interior_fiber:None,paired_fiber:None,vertex_contact:None,
                            pole_paired:None,pole_planar:None,disjoint_hull:None,
                            ruled_projection:Some(certificate),
                        });
                        continue;
                    }
                }
                if out.exact_work < work && out.driver_cells < driver {
                    let proof = crate::source_pole_paired_contact::certify(
                        shell,
                        [a, b],
                        work - out.exact_work,
                        driver - out.driver_cells,
                    )?;
                    out.exact_work += proof.exact_work;
                    out.driver_cells += proof.driver_cells;
                    if let Some(certificate) = proof.certificate {
                        out.all_pairs_absence_proven = false;
                        out.pairs.push(PairReport {
                            faces: [a, b],
                            result: None,
                            allowed: None,
                            fiber: None,
                            interior_fiber: None,
                            paired_fiber: None,
                            vertex_contact: None,
                            disjoint_hull: None,
                            pole_paired: Some(certificate),
                            ruled_projection: None, pole_planar: None,
                        });
                        continue;
                    }
                }
                if out.exact_work < work {
                    let proof = crate::source_vertex_contact::certify(
                        shell,
                        [a, b],
                        work - out.exact_work,
                    )?;
                    out.exact_work += proof.exact_work;
                    if let Some(certificate) = proof.certificate {
                        out.all_pairs_absence_proven = false;
                        out.pairs.push(PairReport {
                            faces: [a, b],
                            result: None,
                            allowed: None,
                            fiber: None,
                            interior_fiber: None,
                            paired_fiber: None,
                            pole_paired: None,
                            ruled_projection: None, pole_planar: None,
                            disjoint_hull: None,
                            vertex_contact: Some(certificate),
                        });
                        continue;
                    }
                }
                if driver > 0 && out.exact_work < work && out.driver_cells < driver {
                    let proof = crate::source_paired_fiber_contact::certify(
                        shell,
                        [a, b],
                        work - out.exact_work,
                        driver - out.driver_cells,
                    )?;
                    out.exact_work += proof.exact_work;
                    out.driver_cells += proof.driver_cells;
                    if let Some(certificate) = proof.certificate {
                        out.all_pairs_absence_proven = false;
                        out.pairs.push(PairReport {
                            faces: [a, b],
                            result: None,
                            allowed: None,
                            fiber: None,
                            interior_fiber: None,
                            pole_paired: None,
                            ruled_projection: None, pole_planar: None,
                            disjoint_hull: None,
                            vertex_contact: None,
                            paired_fiber: Some(certificate),
                        });
                        continue;
                    }
                }
                if driver > 0 {
                    for faces in [[a, b], [b, a]] {
                        if out.exact_work == work
                            || out.spans == spans
                            || out.driver_cells == driver
                        {
                            break;
                        }
                        let proof = crate::source_fiber_contact::certify_with_linear_chart(
                            shell,
                            faces,
                            work - out.exact_work,
                            spans - out.spans,
                            driver - out.driver_cells,
                            linear - out.linear_cells,
                        )?;
                        out.exact_work += proof.exact_work;
                        out.spans += proof.spans;
                        out.driver_cells += proof.driver_cells;
                        out.linear_cells += proof.linear_cells;
                        if let Some(certificate) = proof.certificate {
                            out.all_pairs_absence_proven = false;
                            out.pairs.push(PairReport {
                                faces: [a, b],
                                result: None,
                                allowed: None,
                                fiber: Some(certificate),
                                pole_paired: None,
                                ruled_projection: None, pole_planar: None,
                                disjoint_hull: None,
                                vertex_contact: None,
                                paired_fiber: None,
                                interior_fiber: None,
                            });
                            break;
                        }
                    }
                    if out.pairs.last().is_some_and(|p| p.faces == [a, b]) {
                        continue;
                    }
                }
                if driver > 0 {
                    for faces in [[a, b], [b, a]] {
                        if out.exact_work == work
                            || out.spans == spans
                            || out.driver_cells == driver
                        {
                            break;
                        }
                        let proof = crate::source_interior_contact::certify_with_linear_chart(
                            shell,
                            faces,
                            work - out.exact_work,
                            spans - out.spans,
                            driver - out.driver_cells,
                            linear - out.linear_cells,
                        )?;
                        out.exact_work += proof.exact_work;
                        out.spans += proof.spans;
                        out.driver_cells += proof.driver_cells;
                        out.linear_cells += proof.linear_cells;
                        if let Some(certificate) = proof.certificate {
                            out.all_pairs_absence_proven = false;
                            out.pairs.push(PairReport {
                                faces: [a, b],
                                result: None,
                                allowed: None,
                                fiber: None,
                                pole_paired: None,
                                ruled_projection: None, pole_planar: None,
                                disjoint_hull: None,
                                vertex_contact: None,
                                paired_fiber: None,
                                interior_fiber: Some(certificate),
                            });
                            break;
                        }
                    }
                    if out.pairs.last().is_some_and(|p| p.faces == [a, b]) {
                        continue;
                    }
                }

            }
            let result = search(
                [&regions[a], &regions[b]],
                tolerance_uv,
                limits.cells_per_pair.min(limits.cells - out.cells),
                limits
                    .domain_cells_per_pair
                    .min(limits.domain_cells - out.domain_cells),
            )?;
            out.cells += result.cells;
            out.domain_cells += result.domain_cells;
            out.all_pairs_absence_proven &= result.absence_proven;
            out.all_pairs_qualified &= result.absence_proven;
            out.pairs.push(PairReport {
                faces: [a, b],
                pole_paired: None,
                ruled_projection: None, pole_planar: None,
                result: Some(result),
                allowed: None,
                fiber: None,
                disjoint_hull: None,
                vertex_contact: None,
                paired_fiber: None,
                interior_fiber: None,
            });
        }
    }
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        source_contour_proposal::{qualify_linear_region, qualify_original_region},
        trimmed_face_recipe::{Boundary, Limits},
    };
    use nurbs_core::{curve::Curve, surface::Surface};
    fn surface(vertical: bool) -> Surface {
        Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: (0..2)
                .map(|u| {
                    (0..2)
                        .map(|v| {
                            if vertical {
                                vec![0.5, u as f64, v as f64 - 0.5]
                            } else {
                                vec![u as f64, v as f64, 0.]
                            }
                        })
                        .collect()
                })
                .collect(),
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        }
    }
    fn wire(s: &Surface) -> Vec<Boundary> {
        let points = [[0., 0.], [1., 0.], [1., 1.], [0., 1.]];
        (0..4)
            .map(|i| {
                let a = points[i];
                let b = points[(i + 1) % 4];
                Boundary {
                    curve: Curve::from_polyline(vec![
                        s.evaluate(a[0], a[1]).unwrap().point.to_vec(),
                        s.evaluate(b[0], b[1]).unwrap().point.to_vec(),
                    ])
                    .unwrap(),
                    pcurve: Curve::from_polyline(vec![a.to_vec(), b.to_vec()]).unwrap(),
                    reversed: false,
                }
            })
            .collect()
    }
    #[test]
    fn original_interior_contact_is_removed_only_after_root_valued_region_clipping() {
        let a = surface(false);
        let b = surface(true);
        let context = cad_predicates::ToleranceContext::default_valid();
        let limits = Limits {
            pairs: 1000,
            region_cells: 10000,
            domain_cells: 10000,
            agreement_cells: 10000,
        };
        let wa = wire(&a);
        let wb = wire(&b);
        let ra = qualify_original_region(&context, &a, &[wa.clone()], 1e-8, limits)
            .unwrap()
            .region
            .unwrap();
        let rb = qualify_original_region(&context, &b, &[wb], 1e-8, limits)
            .unwrap()
            .region
            .unwrap();
        let crossing = search([&ra, &rb], 1e-8, 1000, 100000).unwrap();
        assert!(crossing.contact.is_some());
        assert!(!crossing.absence_proven);
        let contact = Curve::from_polyline(vec![vec![0.75, 1.2], vec![0.75, -0.2]]).unwrap();
        let clipped = qualify_linear_region(
            &context,
            &a,
            &[wa],
            &contact,
            0,
            2,
            0,
            1e-8,
            limits,
            10000,
            [1e-8; 2],
            32,
            10000,
            1,
            10000,
            10000,
            10000,
        )
        .unwrap();
        assert!(
            clipped.region.is_some(),
            "{} / {}",
            clipped.reason,
            clipped.interior.reason
        );
        let clipped = clipped.region.unwrap();
        let excluded = search([&clipped, &rb], 1e-8, 10000, 1000000).unwrap();
        assert!(
            excluded.absence_proven,
            "cells {} domain {} unknown {}",
            excluded.cells,
            excluded.domain_cells,
            excluded.unresolved.len()
        );
        assert!(excluded.contact.is_none());
        let stopped = search([&clipped, &rb], 1e-8, 1, 1).unwrap();
        assert!(!stopped.absence_proven && stopped.contact.is_none());
        assert!(!stopped.unresolved.is_empty());
    }
}
