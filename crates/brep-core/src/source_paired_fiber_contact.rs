//! Two curved original charts separated except for owned natural boundary fibers.
use crate::{
    source_contour_proposal::SourceRegion,
    source_fiber_boundary::{self, Locus},
    source_plane_fiber,
    source_shared_edge::SharedEdge,
    source_shell_incidence::{Address, Shell},
};
use nurbs_core::{Error, Result};
pub struct Certificate {
    faces: [usize; 2],
    regions: [SourceRegion; 2],
    fibers: [source_plane_fiber::Certificate; 2],
    edges: Vec<SharedEdge>,
}
impl Certificate {
    pub fn faces(&self) -> [usize; 2] {
        self.faces
    }
    pub fn regions(&self) -> &[SourceRegion; 2] {
        &self.regions
    }
    pub fn fibers(&self) -> &[source_plane_fiber::Certificate; 2] {
        &self.fibers
    }
    pub fn edges(&self) -> &[SharedEdge] {
        &self.edges
    }
}
pub struct Report {
    pub certificate: Option<Certificate>,
    pub exact_work: u64,
    pub driver_cells: usize,
    pub reason: &'static str,
}
pub fn certify(
    shell: &Shell,
    faces: [usize; 2],
    max_work: u64,
    max_driver_cells: usize,
) -> Result<Report> {
    let regions = shell
        .regions()
        .ok_or_else(|| Error::new("BREP_SOURCE_CONTACT", "Qualified original regions required"))?;
    if faces[0] == faces[1]
        || faces.iter().any(|&f| f >= regions.len())
        || !(1..=100_000_000).contains(&max_work)
        || !(1..=100000).contains(&max_driver_cells)
    {
        return Err(Error::new(
            "BREP_SOURCE_CONTACT",
            "Choose distinct source faces and bounded paired-fiber work",
        ));
    }
    let mut out = Report {
        certificate: None,
        exact_work: 0,
        driver_cells: 0,
        reason: "source-paired-fiber-plane-unproven",
    };
    let shared = shell
        .uses()
        .iter()
        .enumerate()
        .filter(|(_, a)| a.iter().all(|a| faces.contains(&a.face)))
        .collect::<Vec<_>>();
    for (index, _) in &shared {
        let c = shell.edges()[*index].world();
        let mut planes = Vec::new();
        if c.control_points.len() >= 3 {
            planes.push(std::array::from_fn(|i| {
                std::array::from_fn(|k| c.control_points[i][k])
            }));
        } else if c.control_points.len() == 2 {
            // Pole points are source-owned proposals, not a geometric plane
            // certificate. Both original chart preimages are still rechecked.
            let mut seen = std::collections::BTreeSet::new();
            for (_, pole) in shell.poles() {
                let point = pole.point();
                if seen.insert(point.map(f64::to_bits)) {
                    planes.push([
                        c.control_points[0].as_slice().try_into().unwrap(),
                        c.control_points[1].as_slice().try_into().unwrap(),
                        point,
                    ]);
                }
                if planes.len() == 64 {
                    break;
                }
            }
        }
        for plane in planes {
            let mut candidates: [Vec<source_plane_fiber::Certificate>; 2] =
                [Vec::new(), Vec::new()];
            for (slot, &face) in faces.iter().enumerate() {
                let s = regions[face].loops()[0][0].surface();
                for axis in 0..2 {
                    for upper in [false, true] {
                        if out.exact_work == max_work {
                            out.reason = "source-paired-fiber-work-limit";
                            return Ok(out);
                        }
                        let r = source_plane_fiber::certify(
                            s,
                            plane,
                            axis,
                            upper,
                            max_work - out.exact_work,
                        )?;
                        out.exact_work += r.exact_work;
                        if let Some(proof) = r.certificate {
                            candidates[slot].push(proof);
                        }
                    }
                }
            }
            for a in &candidates[0] {
                for b in &candidates[1] {
                    if a.side() == b.side() {
                        continue;
                    }
                    let mut owned = true;
                    for (slot, fiber) in [a, b].iter().enumerate() {
                        let face = faces[slot];
                        let addresses = shared
                            .iter()
                            .map(|(_, uses)| *uses.iter().find(|u| u.face == face).unwrap())
                            .collect::<Vec<_>>();
                        for (wire, fragments) in regions[face].loops().iter().enumerate() {
                            for (edge, fragment) in fragments.iter().enumerate() {
                                if out.driver_cells == max_driver_cells {
                                    out.reason = "source-paired-fiber-driver-work-limit";
                                    return Ok(out);
                                }
                                let r = source_fiber_boundary::inspect(
                                    fragment,
                                    fiber,
                                    max_driver_cells - out.driver_cells,
                                )?;
                                out.driver_cells += r.driver_cells;
                                let address = Address { face, wire, edge };
                                let vertex_owned = |end| {
                                    let vertex = (edge + end) % fragments.len();
                                    addresses.iter().any(|a| {
                                        a.wire == wire
                                            && (a.edge == vertex
                                                || (a.edge + 1) % fragments.len() == vertex)
                                    })
                                };
                                owned &= match r.locus {
                                    Locus::EntireFragment => addresses.contains(&address),
                                    Locus::Endpoints(ends) => {
                                        (0..2).all(|i| !ends[i] || vertex_owned(i))
                                    }
                                    Locus::Away => true,
                                    _ => false,
                                };
                            }
                        }
                    }
                    if owned {
                        out.certificate = Some(Certificate {
                            faces,
                            regions: [regions[faces[0]].clone(), regions[faces[1]].clone()],
                            fibers: [a.clone(), b.clone()],
                            edges: shared
                                .iter()
                                .map(|(i, _)| shell.edges()[*i].clone())
                                .collect(),
                        });
                        out.reason = "source-paired-fiber-contact-qualified";
                        return Ok(out);
                    }
                }
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trimmed_face_recipe::{Boundary, Limits};
    fn limits() -> Limits {
        Limits {
            pairs: 10000,
            region_cells: 10000,
            domain_cells: 10000,
            agreement_cells: 10000,
        }
    }
    #[test]
    fn straight_neighbor_rails_use_owned_pole_plane_proposals_and_exact_fibers() {
        for radii in [[0.5, 1.25], [1.25, 0.5], [1., 1.]] {
            for sweep in [std::f64::consts::TAU, -std::f64::consts::TAU] {
                let spans = crate::linear_canal::construct(
                    [[10., -7., 5.], [13., -3., 17.]],
                    radii,
                    [1., 0., 0.],
                    sweep,
                )
                .unwrap();
                let shell = crate::linear_canal::to_capped_source_shell(
                    &spans,
                    1e-7,
                    1e-8,
                    limits(),
                    100_000_000,
                )
                .unwrap()
                .shell
                .unwrap();
                let mut rails = 0;
                for (i, uses) in shell.uses().iter().enumerate() {
                    if shell.edges()[i].world().control_points.len() != 2 {
                        continue;
                    }
                    rails += 1;
                    for faces in [[uses[0].face, uses[1].face], [uses[1].face, uses[0].face]] {
                        let r = certify(&shell, faces, 1000000, 10000).unwrap();
                        let c = r.certificate.expect(r.reason);
                        assert_ne!(c.fibers()[0].side(), c.fibers()[1].side());
                        assert_eq!(c.edges().len(), 1);
                        assert!(
                            certify(&shell, faces, 1, 10000)
                                .unwrap()
                                .certificate
                                .is_none()
                        );
                    }
                }
                assert_eq!(rails, 4);
            }
        }
    }
    #[test]
    fn sphere_join_is_owned_in_both_orders_and_work_exhaustion_refuses_it() {
        for radii in [[0.5, 1.25], [1.25, 0.5], [0., 1.], [1., 0.]] {
            for sweep in [std::f64::consts::TAU, -std::f64::consts::TAU] {
                let spans = crate::linear_canal::construct(
                    [[10., -7., 5.], [13., -3., 17.]],
                    radii,
                    [1., 0., 0.],
                    sweep,
                )
                .unwrap();
                let shell = crate::linear_canal::to_capped_source_shell(
                    &spans,
                    1e-7,
                    1e-8,
                    limits(),
                    100_000_000,
                )
                .unwrap()
                .shell
                .unwrap();
                let other = shell
                    .uses()
                    .iter()
                    .enumerate()
                    .find_map(|(i, uses)| {
                        (uses.iter().any(|a| a.face == 0)
                            && shell.edges()[i].world().control_points.len() == 3)
                            .then(|| uses.iter().find(|a| a.face != 0).unwrap().face)
                    })
                    .unwrap();
                for faces in [[0, other], [other, 0]] {
                    let r = certify(&shell, faces, 1000000, 10000).unwrap();
                    let c = r.certificate.expect(r.reason);
                    assert_eq!(c.faces(), faces);
                    assert_ne!(c.fibers()[0].side(), c.fibers()[1].side());
                    assert!(!c.edges().is_empty());
                    assert!(r.exact_work > 0 && r.exact_work <= 1000000);
                }
                assert!(
                    certify(&shell, [0, other], 1, 10000)
                        .unwrap()
                        .certificate
                        .is_none()
                );
                assert!(certify(&shell, [0, 0], 1000000, 10000).is_err());
            }
        }
    }
    #[test]
    fn coincident_curved_faces_cannot_use_shared_edges_as_contact_authority() {
        let spans =
            crate::linear_canal::construct([[0.; 3], [0., 0., 8.]], [0.5, 1.], [1., 0., 0.], 0.7)
                .unwrap();
        let s = spans[0].surface();
        let boundaries = spans[0].boundaries().unwrap().map(|b| Boundary {
            curve: b.curve,
            pcurve: b.pcurve,
            reversed: false,
        });
        let mut reflected = s.clone();
        reflected.control_points.reverse();
        reflected.weights.reverse();
        let order = [0, 3, 2, 1];
        let reversed = (0..4)
            .map(|edge| Boundary {
                curve: boundaries[order[edge]].curve.clone(),
                pcurve: boundaries[edge].pcurve.clone(),
                reversed: true,
            })
            .collect::<Vec<_>>();
        let mut regions = Vec::new();
        for (surface, wire) in [(s, boundaries.to_vec()), (&reflected, reversed)] {
            let r = crate::source_contour_proposal::qualify_original_region(
                &cad_predicates::ToleranceContext::default_valid(),
                surface,
                &[wire],
                1e-8,
                limits(),
            )
            .unwrap();
            regions.push(r.region.expect(r.reason));
        }
        let pairs = (0..4)
            .map(|edge| crate::source_shell_incidence::Pair {
                uses: [
                    Address {
                        face: 0,
                        wire: 0,
                        edge,
                    },
                    Address {
                        face: 1,
                        wire: 0,
                        edge: order[edge],
                    },
                ],
                world: boundaries[edge].curve.clone(),
                world_reversed: [false, true],
                cutters: [None, None],
            })
            .collect::<Vec<_>>();
        let shell = crate::source_shell_incidence::assemble_regions(&regions, &pairs, 1000000)
            .unwrap()
            .shell
            .unwrap();
        assert!(
            crate::source_vertex_contact::certify(&shell, [0, 1], 1000000)
                .unwrap()
                .certificate
                .is_none()
        );
        assert!(
            certify(&shell, [0, 1], 1000000, 10000)
                .unwrap()
                .certificate
                .is_none()
        );
    }
}
