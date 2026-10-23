//! Exact contacts between a planar source region and a curved boundary/pole image.
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
    fiber: source_pole_plane_image::Certificate,
    edges: Vec<SharedEdge>,
}
impl Certificate {
    pub fn faces(&self) -> [usize; 2] {
        self.faces
    }
    pub fn regions(&self) -> &[SourceRegion; 2] {
        &self.regions
    }
    pub fn fiber(&self) -> &source_pole_plane_image::Certificate {
        &self.fiber
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
/// This owns only cross-face contacts; shell chart injectivity is a separate gate.
pub fn certify(
    shell: &Shell,
    faces: [usize; 2],
    max_work: u64,
    max_driver: usize,
) -> Result<Report> {
    let regions = shell
        .regions()
        .ok_or_else(|| Error::new("BREP_SOURCE_POLE_PLANAR", "Original regions required"))?;
    if faces[0] == faces[1]
        || faces.iter().any(|&f| f >= regions.len())
        || !(1..=100_000_000).contains(&max_work)
        || !(1..=100000).contains(&max_driver)
    {
        return Err(Error::new(
            "BREP_SOURCE_POLE_PLANAR",
            "Choose distinct faces and bounded work",
        ));
    }
    let mut out = Report {
        certificate: None,
        exact_work: 0,
        driver_cells: 0,
        reason: "source-pole-planar-contact-unproven",
    };
    let shared = shell
        .uses()
        .iter()
        .enumerate()
        .filter(|(_, a)| a.iter().all(|a| faces.contains(&a.face)))
        .collect::<Vec<_>>();
    if shared.is_empty() {
        return Ok(out);
    }
    for planar in 0..2 {
        if out.exact_work == max_work {
            return Ok(out);
        }
        let Some(plane) = crate::source_allowed_contact::plane(
            regions[faces[planar]].loops()[0][0].surface(),
            &mut out.exact_work,
            max_work,
        )?
        else {
            continue;
        };
        let curved = faces[1 - planar];
        let addresses = shared
            .iter()
            .map(|(_, uses)| *uses.iter().find(|a| a.face == curved).unwrap())
            .collect::<Vec<_>>();
        for axis in 0..2 {
            for upper in [false, true] {
                if out.exact_work == max_work {
                    return Ok(out);
                }
                let proof = source_pole_plane_image::certify(
                    regions[curved].loops()[0][0].surface(),
                    plane,
                    axis,
                    upper,
                    max_work - out.exact_work,
                )?;
                out.exact_work += proof.exact_work;
                let Some(fiber) = proof.certificate else {
                    continue;
                };
                let mut owned = true;
                for (_, point) in fiber.poles() {
                    owned &= shell.poles().iter().any(|(a, p)| {
                        if a.face != curved || p.point() != *point {
                            return false;
                        }
                        let ids = shell.vertices()[curved][a.wire][a.edge];
                        ids[0] == ids[1]
                            && addresses.iter().any(|edge| {
                                (0..2).any(|end| {
                                    shell.vertices()[curved][edge.wire][edge.edge][end] == ids[0]
                                        && crate::source_vertex_contact::corner(
                                            &regions[curved].loops()[edge.wire][edge.edge],
                                            end,
                                        ) == Some(*point)
                                })
                            })
                    });
                }
                for (wire, fragments) in regions[curved].loops().iter().enumerate() {
                    for (edge, fragment) in fragments.iter().enumerate() {
                        if out.driver_cells == max_driver {
                            return Ok(out);
                        }
                        let proof = source_fiber_boundary::inspect_natural(
                            fragment,
                            fiber.surface(),
                            fiber.boundary(),
                            max_driver - out.driver_cells,
                        )?;
                        out.driver_cells += proof.driver_cells;
                        let address = Address {
                            face: curved,
                            wire,
                            edge,
                        };
                        let vertex_owned = |end| {
                            let id = shell.vertices()[curved][wire][edge][end];
                            addresses
                                .iter()
                                .any(|a| shell.vertices()[curved][a.wire][a.edge].contains(&id))
                        };
                        owned &= match proof.locus {
                            Locus::EntireFragment => addresses.contains(&address),
                            Locus::Endpoints(ends) => (0..2).all(|i| !ends[i] || vertex_owned(i)),
                            Locus::Away => true,
                            _ => false,
                        };
                    }
                }
                if owned {
                    out.certificate = Some(Certificate {
                        faces,
                        regions: [regions[faces[0]].clone(), regions[faces[1]].clone()],
                        fiber,
                        edges: shared
                            .iter()
                            .map(|(i, _)| shell.edges()[*i].clone())
                            .collect(),
                    });
                    out.reason = "source-pole-planar-contact-qualified";
                    return Ok(out);
                }
            }
        }
    }
    Ok(out)
}
