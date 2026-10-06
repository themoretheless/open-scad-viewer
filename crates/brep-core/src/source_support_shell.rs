//! Rebuild source regions and shared incidence from authored support topology.
//! Closed flags and Body records in the input confer no geometric authority.
use crate::source_shell_incidence::{Address, Pair, Pole};
use std::collections::BTreeMap;
pub struct Limits {
    pub tolerance_uv: f64,
    pub faces: usize,
    pub uses: usize,
    /// Independently bounded original-region work per face.
    pub regions: crate::trimmed_face_recipe::Limits,
    pub exact_work: u64,
}
pub fn prepare(
    model: &crate::Model,
    limits: Limits,
) -> crate::Result<crate::source_shell_incidence::Report> {
    if !limits.tolerance_uv.is_finite()
        || limits.tolerance_uv <= 0.
        || !(1..=4096).contains(&limits.faces)
        || !(1..=100000).contains(&limits.uses)
        || !(1..=100_000_000).contains(&limits.exact_work)
    {
        return Err(crate::invalid(
            "Choose bounded source support work and positive UV tolerance",
        ));
    }
    if model.faces.len() > limits.faces
        || model.shells.len() != 1
        || model.shells[0].faces.len() != model.faces.len()
    {
        return Err(crate::invalid(
            "Source support assembly requires one complete connected shell",
        ));
    }
    let count = model.loops.iter().map(|w| w.coedges.len()).sum::<usize>();
    if count > limits.uses {
        return Err(crate::invalid("Source support use limit"));
    }
    model.validate()?;
    // Propose consistently oriented charts. Every proposed lift and shared
    // carrier is freshly checked below; reflected floats grant no authority.
    let oriented = crate::step_interchange_v3::canonical_face_senses(model)?;
    let model = &oriented;
    let mut regions = Vec::new();
    let mut pairs = Vec::new();
    let mut poles = Vec::new();
    let mut pending = BTreeMap::new();
    for (face, support) in model.faces.iter().enumerate() {
        let mut wires = Vec::new();
        for (wire, index) in std::iter::once(support.outer)
            .chain(support.holes.iter().copied())
            .enumerate()
        {
            let mut boundaries = Vec::new();
            for (edge, use_) in model.loops[index].coedges.iter().enumerate() {
                let address = Address { face, wire, edge };
                let original = &model.edges[use_.edge];
                let pcurve = use_.pcurve.clone();
                let reversed = use_.reversed;
                boundaries.push(crate::trimmed_face_recipe::Boundary {
                    curve: original.curve.clone(),
                    pcurve,
                    reversed,
                });
                if original.degenerate {
                    poles.push(Pole {
                        use_: address,
                        point: model.vertices[original.vertices[0]].point,
                    });
                } else if let Some((other, reversed)) = pending.remove(&use_.edge) {
                    pairs.push(Pair {
                        uses: [other, address],
                        world: original.curve.clone(),
                        world_reversed: [reversed, use_.reversed],
                        cutters: [None, None],
                    });
                } else {
                    pending.insert(use_.edge, (address, reversed));
                }
            }
            wires.push(boundaries);
        }
        let qualified = crate::source_contour_proposal::qualify_original_region(
            &cad_predicates::ToleranceContext::default_valid(),
            &support.surface,
            &wires,
            limits.tolerance_uv,
            crate::trimmed_face_recipe::Limits {
                pairs: limits.regions.pairs,
                region_cells: limits.regions.region_cells,
                domain_cells: limits.regions.domain_cells,
                agreement_cells: limits.regions.agreement_cells,
            },
        )?;
        let Some(region) = qualified.region else {
            return Err(crate::invalid(format!(
                "Source support face {face}: {}",
                qualified.audit.reason
            )));
        };
        regions.push(region);
    }
    if !pending.is_empty() {
        return Err(crate::invalid(
            "Source supports retain unpaired ordinary edges",
        ));
    }
    crate::source_shell_incidence::assemble_regions_with_poles(
        &regions,
        &pairs,
        &poles,
        limits.exact_work,
    )
    .map_err(|e| crate::Error::new(e.code, e.message))
}
#[cfg(test)]
mod tests {
    use super::*;
    fn limits() -> Limits {
        Limits {
            tolerance_uv: 1e-8,
            faces: 4096,
            uses: 100000,
            regions: crate::trimmed_face_recipe::Limits {
                pairs: 10000,
                region_cells: 10000,
                domain_cells: 10000,
                agreement_cells: 10000,
            },
            exact_work: 100_000_000,
        }
    }
    #[test]
    fn original_annular_projection_boundary_reversal_and_wall_rank_deficiency() {
        let model = crate::circular_blend::partial_annular_quarter(20.,5.,6.,1.25,1.,1e-7).unwrap();
        for face in [0,2] {
            let surface = &model.faces[face].surface;
            let report = nurbs_core::surface_projection_jacobian::certify(surface,[0,1],100_000_000).unwrap();
            let counts = report.signs.as_ref().map(|rows| {
                let mut counts=[0usize;3];
                for s in rows.iter().flatten() { counts[match s {
                    cad_predicates::Sign::Negative=>0,
                    cad_predicates::Sign::Zero=>1,
                    cad_predicates::Sign::Positive=>2,
                }]+=1; } counts
            });
            eprintln!("original annular face {face}: {} work={} exact={:?} signs={counts:?}",report.reason,report.exact_work,report.exact_reason);
            if face==0 {
                assert!(report.certificate.is_none());
                assert!(report.signs.is_some());
                assert_eq!(counts,Some([65,14,11]));
                assert_eq!(report.opposite_v_boundary_signs,Some([cad_predicates::Sign::Negative,cad_predicates::Sign::Positive]));
                eprintln!("transition grid {:?}",report.signs);
                assert!(nurbs_core::surface_projection_jacobian::certify(surface,[0,1],1).unwrap().certificate.is_none());
                assert!(report.exact_work > 0);
            } else {
                assert!(report.signs.is_some(),"{}",report.reason);
                assert!(report.signs.as_ref().unwrap().iter().flatten().all(|s| *s==cad_predicates::Sign::Zero));
                assert!(report.certificate.is_none());
            }
        }
    }
    #[test]
    fn annular_incidence_and_planar_contact_pass_while_cylinder_contact_refuses() {
        let model =
            crate::circular_blend::partial_annular_quarter(20., 5., 6., 1.25, 1., 1e-7).unwrap();
        let report = prepare(&model, limits());
        match report {
            Ok(report) => {
                eprintln!(
                    "annular source incidence: {} pair={:?}",
                    report.reason, report.uncertain_pair
                );
                assert!(report.shell.is_some(), "{}", report.reason);
                let shell = report.shell.unwrap();
                assert_eq!(shell.faces().len(), 27);
                assert_eq!(shell.poles().len(), 2);
                let contact =
                    crate::source_pole_planar_contact::certify(&shell, [0, 1], 100_000_000, 100000)
                        .unwrap();
                assert!(contact.certificate.is_some(), "{}", contact.reason);
                assert!(
                    crate::source_pole_planar_contact::certify(&shell, [0, 1], 1, 100000)
                        .unwrap()
                        .certificate
                        .is_none()
                );
                let geometry = crate::source_shell_geometry::qualify(
                    shell,
                    crate::source_shell_geometry::Limits {
                        tolerance_uv: 1e-8,
                        corners: 10000,
                        spans: 100000,
                        linear_cells: 100000,
                        exact_work: 100_000_000,
                        driver_cells: 100000,
                        pairs: crate::face_contacts::Limits {
                            pairs: 10000,
                            cells: 10000,
                            domain_cells: 100000,
                            cells_per_pair: 32,
                            domain_cells_per_pair: 512,
                        },
                    },
                )
                .unwrap();
                eprintln!(
                    "annular source geometry: {} face={:?} pair={:?}",
                    geometry.reason, geometry.uncertain_face, geometry.next_pair
                );
                assert!(geometry.geometry.is_none());
                assert_eq!(
                    geometry.reason,
                    "source-shell-different-face-contacts-unproven"
                );
                assert_eq!(geometry.next_pair, Some([0, 2]));
            }
            Err(e) => panic!("annular source support: {}: {}", e.code, e.message),
        }
    }
    #[test]
    fn source_support_claims_do_not_authorize_geometric_lifts() {
        let model =
            crate::circular_blend::partial_annular_quarter(20., 5., 6., 1.25, 1., 1e-7).unwrap();
        let snapshot = format!("{model:?}");
        let original = prepare(&model, limits()).unwrap().shell.unwrap();
        let mut without_claims = model.clone();
        without_claims.0.bodies.clear();
        without_claims.0.shells[0].closed = false;
        without_claims.rebuild_topology_ids();
        let fresh = prepare(&without_claims, limits()).unwrap().shell.unwrap();
        assert_eq!(fresh.definition().unwrap(), original.definition().unwrap());
        assert_eq!(format!("{model:?}"), snapshot);
        let mut damaged = model.clone();
        let wire = damaged.faces[1].outer;
        damaged.0.loops[wire].coedges[0].pcurve.control_points[2][0] += 0.01;
        assert!(prepare(&damaged, limits()).is_err());
        let mut low = limits();
        low.faces = 26;
        assert!(prepare(&model, low).is_err());
    }
}
