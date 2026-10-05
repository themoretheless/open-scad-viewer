//! Two curved original charts separated except for owned natural boundary fibers.
use crate::{
    source_contour_proposal::SourceRegion,
    source_fiber_boundary::{self, Locus},
    source_pole_plane_image,
    source_shared_edge::SharedEdge,
    source_shell_incidence::{Address, Shell},
};
use nurbs_core::{Error, Result};
pub struct Certificate {
    faces: [usize; 2],
    regions: [SourceRegion; 2],
    fibers: [source_pole_plane_image::Certificate; 2],
    edges: Vec<SharedEdge>,
}
impl Certificate {
    pub fn faces(&self) -> [usize; 2] {
        self.faces
    }
    pub fn regions(&self) -> &[SourceRegion; 2] {
        &self.regions
    }
    pub fn fibers(&self) -> &[source_pole_plane_image::Certificate; 2] {
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
        reason: "source-pole-paired-plane-unproven",
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
            let mut candidates: [Vec<source_pole_plane_image::Certificate>; 2] =
                [Vec::new(), Vec::new()];
            for (slot, &face) in faces.iter().enumerate() {
                let s = regions[face].loops()[0][0].surface();
                for axis in 0..2 {
                    for upper in [false, true] {
                        if out.exact_work == max_work {
                            out.reason = "source-pole-paired-work-limit";
                            return Ok(out);
                        }
                        let r = source_pole_plane_image::certify(
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
                        // Every extra plane-image pole needs a privately certified
                        // collapsed use, the same global vertex as a shared edge,
                        // and an exact original shared-edge corner at that point.
                        for (_, point) in fiber.poles() {
                            owned &= shell.poles().iter().any(|(pole_address, proof)| {
                                if pole_address.face != face || proof.point() != *point {
                                    return false;
                                }
                                let ids =
                                    shell.vertices()[face][pole_address.wire][pole_address.edge];
                                ids[0] == ids[1]
                                    && addresses.iter().any(|a| {
                                        (0..2).any(|end| {
                                            shell.vertices()[face][a.wire][a.edge][end] == ids[0]
                                                && crate::source_vertex_contact::corner(
                                                    &regions[face].loops()[a.wire][a.edge],
                                                    end,
                                                ) == Some(*point)
                                        })
                                    })
                            });
                        }
                        for (wire, fragments) in regions[face].loops().iter().enumerate() {
                            for (edge, fragment) in fragments.iter().enumerate() {
                                if out.driver_cells == max_driver_cells {
                                    out.reason = "source-pole-paired-driver-work-limit";
                                    return Ok(out);
                                }
                                let r = source_fiber_boundary::inspect_natural(
                                    fragment,
                                    fiber.surface(),
                                    fiber.boundary(),
                                    max_driver_cells - out.driver_cells,
                                )?;
                                out.driver_cells += r.driver_cells;
                                let address = Address { face, wire, edge };
                                let vertex_owned = |end| {
                                    let id = shell.vertices()[face][wire][edge][end];
                                    addresses.iter().any(|a| {
                                        shell.vertices()[face][a.wire][a.edge].contains(&id)
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
                        out.reason = "source-pole-paired-contact-qualified";
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
    #[test]
    fn owned_cap_meridians_admit_both_orders_but_exhausted_work_refuses() {
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
                    crate::trimmed_face_recipe::Limits {
                        pairs: 10000,
                        region_cells: 10000,
                        domain_cells: 10000,
                        agreement_cells: 10000,
                    },
                    100_000_000,
                )
                .unwrap()
                .shell
                .unwrap();
                let mut count = 0;
                for uses in shell.uses() {
                    if !uses
                        .iter()
                        .all(|a| shell.poles().iter().any(|(p, _)| p.face == a.face))
                    {
                        continue;
                    }
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
                    count += 1;
                }
                assert_eq!(count, 8);
            }
        }
    }
}
